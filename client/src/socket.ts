import { create } from 'zustand';
import { getAccessToken, refreshSession } from './api';
import type { ClientMessage } from './generated/ClientMessage';
import type { ServerMessage } from './generated/ServerMessage';
import { bumpDm, clearDmUnread, isDm, loadDms, reloadDmsSoon } from './dms';
import type { Message } from './generated/Message';
import {
  bumpUnread,
  clearUnread,
  loadCategories,
  loadChannels,
  loadRoles,
  serverOfChannel,
} from './guilds';
import { bumpFriends } from './friends';
import { loadEmojis } from './emojis';
import { pushNotification } from './notify';
import { applyCreated, applyDeleted, applyTyping, applyUpdated, isViewing, markLastRead, resetMessages } from './messages';
import { applyPresence, watchIdle } from './presence';
import { signOutLocal, useSession } from './session';
import { onChannelsReloaded, onHello, onVoiceLeft, onVoiceSignal, onVoiceState } from './voice';

type Status = 'connecting' | 'open' | 'closed';

interface SocketState {
  status: Status;
  latencyMs: number | null;
  /** Эта вкладка без активности дольше 5 минут. */
  idle: boolean;
}

export const useSocket = create<SocketState>(() => ({ status: 'connecting', latencyMs: null, idle: false }));

const PING_EVERY_MS = 5000;
const RECONNECT_BASE_MS = 2000;
const RECONNECT_MAX_MS = 30_000;
const RECONNECT_BACKOFF = 1.5;
const RECONNECT_FAIL_LIMIT = 8; // после ~45 с подряд — остановиться, чтобы не спамить логи

/** Адрес WebSocket-сервера. При сборке под GitHub Pages укажите VITE_WS_URL, иначе берётся относительный путь от origin. */
const WS_BASE = (() => {
  const env = import.meta.env.VITE_WS_URL;
  if (env) return env;
  // Подключаемся к тому же хосту, откуда загружена страница (GitHub Pages + сервер на одном узле).
  return `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}`;
})();

function send(ws: WebSocket, msg: ClientMessage) {
  ws.send(JSON.stringify(msg));
}

let current: WebSocket | undefined;

function onMessageCreated(m: Message) {
  applyCreated(m);
  const unread = m.authorId !== useSession.getState().user?.id && !isViewing(m.channelId);
  if (isDm(m.channelId)) bumpDm(m.channelId, unread);
  else if (serverOfChannel(m.channelId) === undefined) reloadDmsSoon(); // новый диалог или незагруженный сервер
  if (unread && !isDm(m.channelId)) bumpUnread(m.channelId);
  // Звуковое оповещение: только если пришло от другого пользователя и мы сейчас в этом канале.
  if (!isDm(m.channelId) && m.authorId !== useSession.getState().user?.id && isViewing(m.channelId)) {
    playNotificationSound();
  }
}

/** Краткий звук уведомления через Web Audio API; громкость регулируется в браузере. */
function playNotificationSound() {
  try {
    const ctx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(880, ctx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(440, ctx.currentTime + 0.12);
    gain.gain.setValueAtTime(0.15, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.2);
    osc.connect(gain).connect(ctx.destination);
    osc.start(ctx.currentTime);
    osc.stop(ctx.currentTime + 0.2);
  } catch {
    // аудио-контекст недоступен — молчим
  }
}

/** Отправить сообщение серверу, если соединение открыто (иначе молча теряется). */
export function sendWs(msg: ClientMessage) {
  if (current?.readyState === WebSocket.OPEN) send(current, msg);
}

/** «Печатает…»: Composer вызывает не чаще раза в 3 с, сервер тоже режет частоту. */
export const sendTyping = (channelId: number) => sendWs({ type: 'typing', channelId });

/** Открывает соединение и переподключается при обрыве. Возвращает функцию отключения. */
export function connectSocket(): () => void {
  let ws: WebSocket | undefined;
  let pingTimer: number | undefined;
  let reconnectTimer: number | undefined;
  let rejected = false;
  let stopped = false;
  let connectedBefore = false;
  let failedConsecutive = 0;
  let reconnectDelayMs = RECONNECT_BASE_MS;

  const open = async () => {
    useSocket.setState({ status: 'connecting' });
    if (rejected) {
      // Сервер отверг токен: обновляем его перед переподключением.
      try {
        await refreshSession();
        rejected = false;
      } catch {
        signOutLocal();
        return;
      }
    }
    if (stopped) return;

    const socket = new WebSocket(`${WS_BASE}/ws`);
    ws = socket;
    current = socket;

    socket.onopen = () => {
      failedConsecutive = 0;
      reconnectDelayMs = RECONNECT_BASE_MS;
      send(socket, { type: 'auth', token: getAccessToken() ?? '' });
    };

    socket.onmessage = (event) => {
      const msg = JSON.parse(event.data) as ServerMessage;
      switch (msg.type) {
        case 'hello':
          useSocket.setState({ status: 'open' });
          onHello(msg.connId);
          // Пока сокета не было, события терялись: перечитываем открытые ленты.
          if (connectedBefore) resetMessages();
          connectedBefore = true;
          if (useSocket.getState().idle) send(socket, { type: 'setIdle', idle: true });
          send(socket, { type: 'ping', sentAt: Date.now() });
          pingTimer = window.setInterval(
            () => send(socket, { type: 'ping', sentAt: Date.now() }),
            PING_EVERY_MS,
          );
          break;
        case 'pong':
          useSocket.setState({ latencyMs: Date.now() - msg.sentAt });
          break;
        case 'error':
          if (msg.message === 'unauthorized') rejected = true;
          else console.warn('server error:', msg.message);
          break;
        case 'messageCreated':
          onMessageCreated(msg.message);
          break;
        case 'messageUpdated':
          applyUpdated(msg.message);
          break;
        case 'messageDeleted':
          applyDeleted(msg.channelId, msg.messageId);
          break;
        case 'typing':
          applyTyping(msg.channelId, msg.userId, msg.username);
          break;
        case 'channelsChanged':
          void loadCategories(msg.serverId).catch(() => {});
          void loadChannels(msg.serverId)
            .then((list) => onChannelsReloaded(msg.serverId, list.map((c) => c.id)))
            .catch(() => {});
          break;
        case 'rolesChanged':
          void loadRoles(msg.serverId).catch(() => {});
          void loadChannels(msg.serverId).catch(() => {});
          void loadCategories(msg.serverId).catch(() => {});
          break;
        case 'emojisChanged':
           void loadEmojis(msg.serverId).catch(() => {});
          break;
        case 'voiceState':
          onVoiceState(msg.participant);
          break;
        case 'voiceLeft':
          onVoiceLeft(msg.channelId, msg.userId);
          break;
        case 'voiceSignal':
          void onVoiceSignal(msg.fromUserId, msg.data);
          break;
        case 'presenceChanged':
          applyPresence(msg.userId, msg.status);
          break;
        case 'channelRead':
          if (msg.serverId === null) clearDmUnread(msg.channelId);
          else clearUnread(msg.serverId, msg.channelId);
          markLastRead(msg.channelId);
          break;
        case 'friendsChanged':
          bumpFriends();
          void loadDms().catch(() => {});
          break;
        case 'dmsChanged':
          void loadDms().catch(() => {});
          break;
        case 'notificationCreated':
          pushNotification(msg.notification);
          break;
      }
    };

    socket.onclose = () => {
      window.clearInterval(pingTimer);
      useSocket.setState({ status: 'closed', latencyMs: null });
      if (stopped) return;
      failedConsecutive += 1;
      if (failedConsecutive >= RECONNECT_FAIL_LIMIT) {
        // Слишком много неудач подряд — прекращаем попытки, пользователь перезагрузит страницу сам.
        console.warn('[ws] connection failed repeatedly; stopping reconnects');
        return;
      }
      reconnectTimer = window.setTimeout(open, reconnectDelayMs);
      reconnectDelayMs = Math.min(reconnectDelayMs * RECONNECT_BACKOFF, RECONNECT_MAX_MS);
    };
  };

  const stopIdle = watchIdle((idle) => {
    useSocket.setState({ idle });
    if (current?.readyState === WebSocket.OPEN) send(current, { type: 'setIdle', idle });
  });

  void open();
  return () => {
    stopIdle();
    stopped = true;
    current = undefined;
    window.clearTimeout(reconnectTimer);
    ws?.close();
  };
}
