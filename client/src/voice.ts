// Голос и видео: WebRTC «каждый с каждым». Сервер только знает, кто где, и пересылает сигналы.
import { create } from "zustand";
import { api } from "./api";
import type { VoiceFlags } from "./generated/VoiceFlags";
import type { VoiceParticipant } from "./generated/VoiceParticipant";
import { useSession } from "./session";
import { sendWs } from "./socket";

export type MicMode = "voice" | "ptt";

interface Settings {
  micMode: MicMode;
  /** KeyboardEvent.code; по умолчанию клавиша ` (Ё), не зависит от раскладки. */
  pttKey: string;
  /** Громкость участников, 0–2 (до 200%). */
  volumes: Record<number, number>;
}

export interface RemoteMedia {
  audio?: MediaStream;
  camera?: MediaStream;
  screen?: MediaStream;
}

interface VoiceState {
  /** channelId → userId → участник. Все голоса, которые видит пользователь. */
  participants: Record<number, Record<number, VoiceParticipant> | undefined>;
  /** Звонок, в котором я сейчас; serverId `null` — ЛС. */
  current: { channelId: number; serverId: number | null } | null;
  flags: VoiceFlags;
  /** Передаётся ли сейчас мой микрофон (говорю / зажата кнопка). */
  transmitting: boolean;
  speaking: Record<number, boolean>;
  remote: Record<number, RemoteMedia | undefined>;
  localCamera: MediaStream | null;
  localScreen: MediaStream | null;
  settings: Settings;
  error: string | null;
}

const SETTINGS_KEY = "nexus.voice.settings";
const defaultSettings: Settings = {
  micMode: "voice",
  pttKey: "Backquote",
  volumes: {},
};

function loadSettings(): Settings {
  try {
    return {
      ...defaultSettings,
      ...JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "{}"),
    };
  } catch {
    return defaultSettings;
  }
}

const noFlags: VoiceFlags = {
  muted: false,
  deafened: false,
  camera: false,
  screen: false,
};

export const useVoice = create<VoiceState>(() => ({
  participants: {},
  current: null,
  flags: noFlags,
  transmitting: false,
  speaking: {},
  remote: {},
  localCamera: null,
  localScreen: null,
  settings: loadSettings(),
  error: null,
}));

export function updateSettings(patch: Partial<Settings>) {
  const settings = { ...useVoice.getState().settings, ...patch };
  useVoice.setState({ settings });
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // нет хранилища (приватный режим) — настройки живут до перезагрузки
  }
  applyVolumes();
  updateGate();
}

// ---------------------------------------------------------------------------
// Кто где: снимок при загрузке и события сервера
// ---------------------------------------------------------------------------

let myConnId: number | null = null;
const myId = () => useSession.getState().user?.id;

export async function loadVoice() {
  const list = await api<VoiceParticipant[]>("/api/voice");
  const participants: VoiceState["participants"] = {};
  for (const p of list) (participants[p.channelId] ??= {})[p.userId] = p;
  useVoice.setState({ participants });
}

/** Новое подключение к серверу. Прежний голос сервер уже снял: входим заново. */
export function onHello(connId: number) {
  myConnId = connId;
  void loadVoice().catch(() => {});
  const cur = useVoice.getState().current;
  if (cur) {
    teardown();
    void joinVoice(cur.channelId, cur.serverId);
  }
}

export function onVoiceState(p: VoiceParticipant) {
  useVoice.setState((s) => {
    const participants = { ...s.participants };
    // Человек в голосе только в одном канале: убираем его из прежнего.
    for (const [cid, users] of Object.entries(participants)) {
      if (users?.[p.userId] && Number(cid) !== p.channelId) {
        const { [p.userId]: _, ...rest } = users;
        participants[Number(cid)] = rest;
      }
    }
    participants[p.channelId] = { ...participants[p.channelId], [p.userId]: p };
    return { participants };
  });

  const cur = useVoice.getState().current;
  if (p.userId === myId()) {
    if (p.connectionId !== myConnId) {
      teardown(); // вошёл в голос из другой вкладки
    } else if (cur?.channelId === p.channelId) {
      for (const other of Object.values(
        useVoice.getState().participants[p.channelId] ?? {},
      )) {
        if (other.userId !== myId()) ensurePeer(other.userId);
      }
    }
    return;
  }
  if (cur?.channelId === p.channelId) ensurePeer(p.userId);
  else closePeer(p.userId); // ушёл из моего звонка в другой
}

export function onVoiceLeft(channelId: number, userId: number) {
  useVoice.setState((s) => {
    const users = s.participants[channelId];
    if (!users?.[userId]) return {};
    const { [userId]: _, ...rest } = users;
    return { participants: { ...s.participants, [channelId]: rest } };
  });
  if (useVoice.getState().current?.channelId !== channelId) return;
  if (userId === myId()) teardown();
  else closePeer(userId);
}

/** Каналы сервера изменились: если моего голосового канала больше нет — выходим. */
export function onChannelsReloaded(serverId: number, channelIds: number[]) {
  const cur = useVoice.getState().current;
  if (cur?.serverId === serverId && !channelIds.includes(cur.channelId))
    leaveVoice();
}

// ---------------------------------------------------------------------------
// Вход, выход, флаги
// ---------------------------------------------------------------------------

let mic: MediaStream | null = null;
let audioCtx: AudioContext | null = null;

export async function joinVoice(channelId: number, serverId: number | null) {
  if (useVoice.getState().current) teardown(true);
  useVoice.setState({ error: null });
  try {
    mic = await navigator.mediaDevices.getUserMedia({
      audio: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
      },
    });
  } catch {
    useVoice.setState({
      error: "Нет доступа к микрофону. Разрешите его в настройках браузера.",
    });
    return;
  }
  // Создаём по нажатию пользователя: иначе браузер не даст воспроизводить звук.
  audioCtx ??= new AudioContext();
  void audioCtx.resume();
  useVoice.setState({ current: { channelId, serverId }, flags: noFlags });
  startLocalMeter();
  updateGate();
  sendWs({ type: "voiceJoin", channelId });
}

export function leaveVoice() {
  if (!useVoice.getState().current) return;
  sendWs({ type: "voiceLeave" });
  teardown(true);
}

/** Закрыть всё локально. `stopDevices=false` — когда сервер нас перенёс в другую вкладку. */
function teardown(stopDevices = true) {
  for (const id of [...peers.keys()]) closePeer(id);
  stopLocalMeter();
  const s = useVoice.getState();
  if (stopDevices) {
    mic?.getTracks().forEach((t) => t.stop());
    s.localCamera?.getTracks().forEach((t) => t.stop());
    s.localScreen?.getTracks().forEach((t) => t.stop());
    mic = null;
  }
  useVoice.setState({
    current: null,
    flags: noFlags,
    transmitting: false,
    localCamera: null,
    localScreen: null,
    remote: {},
    speaking: {},
  });
}

function setFlags(patch: Partial<VoiceFlags>) {
  const flags = { ...useVoice.getState().flags, ...patch };
  useVoice.setState({ flags });
  sendWs({ type: "voiceSetFlags", flags });
  updateGate();
  applyVolumes();
}

export const toggleMute = () => {
  const f = useVoice.getState().flags;
  // Включить микрофон при выключенном звуке — значит вернуть и звук, как в Discord.
  setFlags(f.muted ? { muted: false, deafened: false } : { muted: true });
};

export const toggleDeafen = () => {
  const f = useVoice.getState().flags;
  setFlags(
    f.deafened
      ? { deafened: false, muted: false }
      : { deafened: true, muted: true },
  );
};

export async function toggleCamera() {
  const s = useVoice.getState();
  if (s.localCamera) {
    s.localCamera.getTracks().forEach((t) => t.stop());
    useVoice.setState({ localCamera: null });
    for (const p of peers.values()) removeSenders(p, "camera");
    setFlags({ camera: false });
    return;
  }
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      video: { width: 1280, height: 720 },
    });
    useVoice.setState({ localCamera: stream });
    for (const p of peers.values())
      if (p.tracksAdded) addStream(p, "camera", stream);
    setFlags({ camera: true });
  } catch {
    useVoice.setState({ error: "Нет доступа к камере." });
  }
}

export async function toggleScreen() {
  const s = useVoice.getState();
  if (s.localScreen) return stopScreen();
  try {
    const stream = await navigator.mediaDevices.getDisplayMedia({
      video: true,
      audio: true,
    });
    // Пользователь остановил показ кнопкой браузера.
    stream.getVideoTracks()[0]?.addEventListener("ended", stopScreen);
    useVoice.setState({ localScreen: stream });
    for (const p of peers.values())
      if (p.tracksAdded) addStream(p, "screen", stream);
    setFlags({ screen: true });
  } catch {
    // отменил выбор окна — это не ошибка
  }
}

function stopScreen() {
  const stream = useVoice.getState().localScreen;
  if (!stream) return;
  stream.getTracks().forEach((t) => t.stop());
  useVoice.setState({ localScreen: null });
  for (const p of peers.values()) removeSenders(p, "screen");
  setFlags({ screen: false });
}

export function setVolume(userId: number, volume: number) {
  const s = useVoice.getState().settings;
  updateSettings({ volumes: { ...s.volumes, [userId]: volume } });
}

// ---------------------------------------------------------------------------
// Микрофон: по голосу или по кнопке
// ---------------------------------------------------------------------------

/** Порог громкости для режима «по голосу» (RMS ≈ −34 дБ). */
const VOICE_THRESHOLD = 0.02;
/** Не обрезаем концы слов: держим микрофон открытым ещё столько после тишины. */
const VOICE_HANGOVER_MS = 300;
const METER_EVERY_MS = 50;

let pttDown = false;
let voiceOpenUntil = 0;
let meterTimer: number | undefined;
let localAnalyser: AnalyserNode | null = null;
let analysisTrack: MediaStreamTrack | null = null;

/** Включает или глушит отправку микрофона по текущему режиму и флагам. */
function updateGate() {
  const s = useVoice.getState();
  const track = mic?.getAudioTracks()[0];
  if (!track) return;
  const open =
    !!s.current &&
    !s.flags.muted &&
    !s.flags.deafened &&
    (s.settings.micMode === "ptt" ? pttDown : Date.now() < voiceOpenUntil);
  track.enabled = open;
  if (s.transmitting !== open) {
    useVoice.setState({
      transmitting: open,
      speaking: { ...s.speaking, [myId() ?? 0]: open },
    });
  }
}

function rms(analyser: AnalyserNode, buf: Float32Array<ArrayBuffer>) {
  analyser.getFloatTimeDomainData(buf);
  let sum = 0;
  for (const v of buf) sum += v * v;
  return Math.sqrt(sum / buf.length);
}

function startLocalMeter() {
  if (!audioCtx || !mic) return;
  // Слушаем копию дорожки: отправляемая выключается гейтом, а слышать себя надо всегда.
  analysisTrack = mic.getAudioTracks()[0].clone();
  localAnalyser = audioCtx.createAnalyser();
  localAnalyser.fftSize = 512;
  audioCtx
    .createMediaStreamSource(new MediaStream([analysisTrack]))
    .connect(localAnalyser);
  const buf = new Float32Array(localAnalyser.fftSize);
  // setInterval, а не requestAnimationFrame: тот замирает во фоновой вкладке.
  meterTimer = window.setInterval(() => {
    if (localAnalyser && rms(localAnalyser, buf) > VOICE_THRESHOLD) {
      voiceOpenUntil = Date.now() + VOICE_HANGOVER_MS;
    }
    updateGate();
    meterRemotes();
  }, METER_EVERY_MS);
}

function stopLocalMeter() {
  window.clearInterval(meterTimer);
  analysisTrack?.stop();
  analysisTrack = null;
  localAnalyser = null;
}

function isTyping(e: KeyboardEvent) {
  const el = e.target as HTMLElement | null;
  return (
    !!el &&
    (el.tagName === "INPUT" ||
      el.tagName === "TEXTAREA" ||
      el.isContentEditable)
  );
}

// shortcut: кнопка работает, только пока окно NexusChat в фокусе; глобальная клавиша для
// приложения Windows — через плагин global-shortcut Tauri, когда понадобится.
window.addEventListener("keydown", (e) => {
  if (e.code !== useVoice.getState().settings.pttKey || e.repeat || isTyping(e))
    return;
  pttDown = true;
  updateGate();
});
window.addEventListener("keyup", (e) => {
  if (e.code !== useVoice.getState().settings.pttKey) return;
  pttDown = false;
  updateGate();
});
window.addEventListener("blur", () => {
  pttDown = false; // отпустили клавишу в другом окне — не держим микрофон открытым
  updateGate();
});

// ---------------------------------------------------------------------------
// Соединения с участниками (perfect negotiation из спецификации WebRTC)
// ---------------------------------------------------------------------------

type StreamKind = "mic" | "camera" | "screen";

interface Peer {
  userId: number;
  pc: RTCPeerConnection;
  polite: boolean;
  makingOffer: boolean;
  ignoreOffer: boolean;
  /** id потока собеседника → что это (передаётся вместе с описанием сессии). */
  remoteKinds: Record<string, StreamKind>;
  senders: Partial<Record<StreamKind, RTCRtpSender[]>>;
  tracksAdded?: boolean;
  /** Сигналы обрабатываем строго по очереди: иначе ICE обгоняет описание сессии. */
  queue: Promise<void>;
  gain?: GainNode;
  analyser?: AnalyserNode;
  audioEl?: HTMLAudioElement;
}

const peers = new Map<number, Peer>();
const ICE_SERVERS: RTCIceServer[] = [{ urls: "stun:stun.l.google.com:19302" }];
// shortcut: без TURN-сервера звонок не соединится, если оба за строгим NAT (часть мобильных
// сетей и корпоративных); добавить coturn в docker-compose, когда появятся такие жалобы.

function myStreams(): Record<string, StreamKind> {
  const s = useVoice.getState();
  const map: Record<string, StreamKind> = {};
  if (mic) map[mic.id] = "mic";
  if (s.localCamera) map[s.localCamera.id] = "camera";
  if (s.localScreen) map[s.localScreen.id] = "screen";
  return map;
}

function signal(to: number, data: object) {
  sendWs({ type: "voiceSignal", toUserId: to, data });
}

function ensurePeer(userId: number): Peer {
  const existing = peers.get(userId);
  if (existing) return existing;

  const pc = new RTCPeerConnection({ iceServers: ICE_SERVERS });
  const peer: Peer = {
    userId,
    pc,
    // Вежливый уступает при встречных предложениях; роли разные у двух сторон.
    polite: (myId() ?? 0) > userId,
    makingOffer: false,
    ignoreOffer: false,
    remoteKinds: {},
    senders: {},
    queue: Promise.resolve(),
  };
  peers.set(userId, peer);

  pc.onnegotiationneeded = async () => {
    try {
      peer.makingOffer = true;
      await pc.setLocalDescription();
      signal(userId, {
        description: pc.localDescription,
        streams: myStreams(),
      });
    } catch (e) {
      console.warn("negotiation failed", e);
    } finally {
      peer.makingOffer = false;
    }
  };
  pc.onicecandidate = (e) =>
    e.candidate && signal(userId, { candidate: e.candidate });
  pc.ontrack = (e) => onRemoteTrack(peer, e);
  pc.onconnectionstatechange = () => {
    if (pc.connectionState === "failed") pc.restartIce();
  };

  // Первое предложение делает только невежливая сторона: вежливая добавит свои дорожки
  // в ответ на него. Так при входе нет встречных предложений, на которых Chrome теряет ICE.
  if (!peer.polite) addLocalStreams(peer);
  return peer;
}

function addLocalStreams(peer: Peer) {
  peer.tracksAdded = true;
  const s = useVoice.getState();
  if (mic) addStream(peer, "mic", mic);
  if (s.localCamera) addStream(peer, "camera", s.localCamera);
  if (s.localScreen) addStream(peer, "screen", s.localScreen);
}

function addStream(peer: Peer, kind: StreamKind, stream: MediaStream) {
  peer.senders[kind] = stream
    .getTracks()
    .map((t) => peer.pc.addTrack(t, stream));
}

function removeSenders(peer: Peer, kind: StreamKind) {
  for (const sender of peer.senders[kind] ?? []) peer.pc.removeTrack(sender);
  delete peer.senders[kind];
}

function closePeer(userId: number) {
  const peer = peers.get(userId);
  if (!peer) return;
  peers.delete(userId);
  peer.pc.close();
  peer.gain?.disconnect();
  peer.audioEl?.remove();
  useVoice.setState((s) => {
    const { [userId]: _r, ...remote } = s.remote;
    const { [userId]: _s, ...speaking } = s.speaking;
    return { remote, speaking };
  });
}

export async function onVoiceSignal(from: number, data: unknown) {
  // Сервер пропускает сигнал, только если мы оба в одном канале. Его событие о входе
  // собеседника может прийти позже сигнала — не ждём его, иначе предложение потеряется.
  if (!useVoice.getState().current) return;
  const peer = ensurePeer(from);
  peer.queue = peer.queue.then(() => handleSignal(peer, data));
}

async function handleSignal(peer: Peer, data: unknown) {
  const { pc } = peer;
  const from = peer.userId;
  const msg = data as {
    description?: RTCSessionDescriptionInit;
    candidate?: RTCIceCandidateInit;
    streams?: Record<string, StreamKind>;
  };
  try {
    if (msg.description) {
      peer.remoteKinds = { ...peer.remoteKinds, ...msg.streams };
      const collision =
        msg.description.type === "offer" &&
        (peer.makingOffer || pc.signalingState !== "stable");
      peer.ignoreOffer = !peer.polite && collision;
      if (peer.ignoreOffer) return;
      await pc.setRemoteDescription(msg.description);
      if (msg.description.type === "offer") {
        if (!peer.tracksAdded) addLocalStreams(peer);
        await pc.setLocalDescription();
        signal(from, {
          description: pc.localDescription,
          streams: myStreams(),
        });
      }
    } else if (msg.candidate) {
      try {
        await pc.addIceCandidate(msg.candidate);
      } catch (e) {
        if (!peer.ignoreOffer) throw e;
      }
    }
  } catch (e) {
    console.warn("voice signal failed", e);
  }
}

function onRemoteTrack(peer: Peer, e: RTCTrackEvent) {
  const stream = e.streams[0];
  if (!stream) return;
  const kind =
    peer.remoteKinds[stream.id] ??
    (e.track.kind === "audio" ? "mic" : "camera");
  const key = kind === "mic" ? "audio" : kind;
  useVoice.setState((s) => ({
    remote: {
      ...s.remote,
      [peer.userId]: { ...s.remote[peer.userId], [key]: stream },
    },
  }));
  if (kind === "mic" || (kind === "screen" && e.track.kind === "audio"))
    playAudio(peer, stream);
}

/** Звук собеседника через усилитель: громкость 0–200% и «выключить звук» у себя. */
function playAudio(peer: Peer, stream: MediaStream) {
  if (!audioCtx) return;
  // Chrome не отдаёт звук WebRTC в WebAudio без элемента <audio>, поэтому заглушённый элемент тоже.
  const el = new Audio();
  el.srcObject = stream;
  el.muted = true;
  void el.play().catch(() => {});
  peer.audioEl = el;
  const source = audioCtx.createMediaStreamSource(stream);
  peer.gain ??= audioCtx.createGain();
  peer.analyser ??= audioCtx.createAnalyser();
  peer.analyser.fftSize = 512;
  source.connect(peer.analyser);
  source.connect(peer.gain).connect(audioCtx.destination);
  applyVolumes();
}

function applyVolumes() {
  const s = useVoice.getState();
  for (const p of peers.values()) {
    if (p.gain)
      p.gain.gain.value = s.flags.deafened
        ? 0
        : (s.settings.volumes[p.userId] ?? 1);
  }
}

function meterRemotes() {
  const s = useVoice.getState();
  let changed = false;
  const speaking = { ...s.speaking };
  for (const p of peers.values()) {
    if (!p.analyser) continue;
    const buf = new Float32Array(p.analyser.fftSize);
    const now = rms(p.analyser, buf) > VOICE_THRESHOLD;
    if (speaking[p.userId] !== now) {
      speaking[p.userId] = now;
      changed = true;
    }
  }
  if (changed) useVoice.setState({ speaking });
}
