import { useEffect, useRef, useState } from "react";
import type { VoiceParticipant } from "./generated/VoiceParticipant";
import { Modal, primaryButton } from "./Modal";
import { useSession } from "./session";
import {
  joinVoice,
  leaveVoice,
  setVolume,
  toggleCamera,
  toggleDeafen,
  toggleMute,
  toggleScreen,
  updateSettings,
  useVoice,
} from "./voice";

const EMPTY: Record<number, VoiceParticipant> = {};
const useRoom = (channelId: number) =>
  Object.values(useVoice((s) => s.participants[channelId] ?? EMPTY));

function Avatar({
  name,
  speaking,
  size = "size-6 text-xs",
}: {
  name: string;
  speaking: boolean;
  size?: string;
}) {
  return (
    <span
      aria-hidden
      className={`grid shrink-0 place-items-center rounded-full bg-indigo-500 font-semibold text-white ${size} ${
        speaking ? "ring-2 ring-emerald-400" : ""
      }`}
    >
      {name[0]?.toUpperCase()}
    </span>
  );
}

const flagIcons = (p: VoiceParticipant) =>
  `${p.screen ? " 🖥️" : ""}${p.camera ? " 📷" : ""}${p.deafened ? " 🔇" : p.muted ? " 🎙️✕" : ""}`;

/** Кто в голосовом канале — под каналом в списке слева. */
export function VoiceRoster({ channelId }: { channelId: number }) {
  const room = useRoom(channelId);
  const speaking = useVoice((s) => s.speaking);
  if (room.length === 0) return null;
  return (
    <ul className="mb-1 ml-6 space-y-0.5" aria-label="В голосовом канале">
      {room.map((p) => (
        <li
          key={p.userId}
          className="flex items-center gap-2 px-2 py-0.5 text-sm text-zinc-300"
        >
          <Avatar name={p.username} speaking={!!speaking[p.userId]} />
          <span className="truncate">{p.username}</span>
          <span
            className="ml-auto text-xs"
            aria-label={flagIcons(p) ? "состояние" : undefined}
          >
            {flagIcons(p)}
          </span>
        </li>
      ))}
    </ul>
  );
}

function Video({
  stream,
  mirror,
  label,
}: {
  stream: MediaStream;
  mirror?: boolean;
  label: string;
}) {
  const ref = useRef<HTMLVideoElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.srcObject = stream;
  }, [stream]);
  return (
    <video
      ref={ref}
      autoPlay
      playsInline
      muted // звук идёт отдельно, через усилитель громкости
      aria-label={label}
      className={`h-full w-full rounded-lg bg-black object-contain ${mirror ? "-scale-x-100" : ""}`}
    />
  );
}

/** Сцена звонка: демонстрации экрана крупно, затем камеры и аватары. */
export function VoiceStage({
  channelId,
  serverId,
  label,
  compact,
}: {
  channelId: number;
  serverId: number | null;
  label: string;
  compact?: boolean;
}) {
  const me = useSession((s) => s.user);
  const room = useRoom(channelId);
  const {
    current,
    remote,
    speaking,
    localCamera,
    localScreen,
    settings,
    error,
  } = useVoice();
  const inCall = current?.channelId === channelId;
  const [focus, setFocus] = useState<string | null>(null);

  const media = (p: VoiceParticipant) =>
    p.userId === me?.id
      ? { camera: localCamera ?? undefined, screen: localScreen ?? undefined }
      : (remote[p.userId] ?? {});

  type Tile = {
    key: string;
    p: VoiceParticipant;
    stream?: MediaStream;
    kind: "screen" | "camera" | "avatar";
  };
  const tiles: Tile[] = [];
  for (const p of room) {
    const m = inCall ? media(p) : {};
    if (p.screen && m.screen)
      tiles.push({ key: `s${p.userId}`, p, stream: m.screen, kind: "screen" });
    tiles.push(
      p.camera && m.camera
        ? { key: `c${p.userId}`, p, stream: m.camera, kind: "camera" }
        : { key: `a${p.userId}`, p, kind: "avatar" },
    );
  }
  const focused =
    tiles.find((t) => t.key === focus) ??
    tiles.find((t) => t.kind === "screen");
  const others = tiles.filter((t) => t !== focused);
  const someoneElse = room.some((p) => p.userId !== me?.id);

  return (
    <section
      aria-label={`Звонок ${label}`}
      className={`flex flex-col gap-3 bg-zinc-950 p-3 ${compact ? "max-h-[45dvh]" : "min-h-0 flex-1"}`}
    >
      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
      {focused && inCall && (
        <div className="min-h-0 flex-1">
          <TileView
            tile={focused}
            big
            me={me?.id}
            speaking={!!speaking[focused.p.userId]}
            volume={settings.volumes[focused.p.userId] ?? 1}
          />
        </div>
      )}
      <div
        className={`flex flex-wrap justify-center gap-3 ${focused && inCall ? "" : "flex-1 content-center"}`}
      >
        {(inCall ? others : tiles).map((t) => (
          <button
            key={t.key}
            onClick={() => setFocus(t.key === focus ? null : t.key)}
            className="w-44 sm:w-56"
            aria-label={`${t.p.username}: показать крупно`}
          >
            <TileView
              tile={t}
              me={me?.id}
              speaking={!!speaking[t.p.userId]}
              volume={settings.volumes[t.p.userId] ?? 1}
            />
          </button>
        ))}
      </div>
      {!inCall && (
        <div className="flex justify-center">
          <button
            onClick={() => void joinVoice(channelId, serverId)}
            className={primaryButton}
          >
            {serverId === null && someoneElse
              ? "📞 Ответить"
              : room.length
                ? "Присоединиться"
                : serverId === null
                  ? "📞 Позвонить"
                  : "Войти в голосовой канал"}
          </button>
        </div>
      )}
      {inCall && (
        <p className="sr-only">Вы в звонке. Управление — внизу слева.</p>
      )}
    </section>
  );

  function TileView({
    tile,
    big,
    me,
    speaking,
    volume,
  }: {
    tile: Tile;
    big?: boolean;
    me?: number;
    speaking: boolean;
    volume: number;
  }) {
    const { p } = tile;
    const isMe = p.userId === me;
    return (
      <div
        className={`relative grid aspect-video place-items-center overflow-hidden rounded-lg bg-zinc-800 ${
          big ? "h-full w-full" : ""
        } ${speaking && tile.kind !== "screen" ? "ring-2 ring-emerald-400" : ""}`}
      >
        {tile.stream ? (
          <Video
            stream={tile.stream}
            mirror={isMe && tile.kind === "camera"}
            label={`${p.username}: ${tile.kind === "screen" ? "экран" : "камера"}`}
          />
        ) : (
          <Avatar
            name={p.username}
            speaking={speaking}
            size="size-16 text-2xl"
          />
        )}
        <span className="absolute bottom-1 left-1 rounded bg-black/60 px-1.5 text-xs text-white">
          {p.username}
          {tile.kind === "screen" ? " — экран" : flagIcons(p)}
        </span>
        {!isMe && inCall && tile.kind !== "screen" && (
          <label
            className="absolute top-1 right-1 flex items-center gap-1 rounded bg-black/60 px-1.5 text-xs text-white"
            onClick={(e) => e.stopPropagation()}
          >
            🔊
            <input
              type="range"
              min={0}
              max={200}
              step={5}
              value={Math.round(volume * 100)}
              onChange={(e) =>
                setVolume(p.userId, Number(e.target.value) / 100)
              }
              aria-label={`Громкость ${p.username}`}
              className="w-16"
            />
            {Math.round(volume * 100)}%
          </label>
        )}
      </div>
    );
  }
}

/** Панель управления звонком внизу слева, как в Discord. */
export function VoicePanel({ label }: { label: string }) {
  const { current, flags, transmitting, settings, localCamera, localScreen } =
    useVoice();
  const [settingsOpen, setSettingsOpen] = useState(false);
  if (!current) return null;

  const btn = (on: boolean, danger = false) =>
    `grid size-9 place-items-center rounded text-lg ${
      on
        ? danger
          ? "bg-red-500/20 text-red-400"
          : "bg-zinc-700 text-white"
        : "text-zinc-300 hover:bg-zinc-800"
    }`;

  return (
    <div className="border-t border-zinc-800 bg-zinc-950/60 px-3 py-2">
      <div className="flex items-center gap-2 text-sm">
        <span
          className={`size-2.5 rounded-full ${transmitting ? "bg-emerald-400" : "bg-emerald-700"}`}
          aria-hidden
        />
        <div className="min-w-0 flex-1">
          <div className="font-medium text-emerald-400">Голос подключён</div>
          <div className="truncate text-xs text-zinc-400">
            {label} ·{" "}
            {settings.micMode === "ptt"
              ? `по кнопке ${keyName(settings.pttKey)}`
              : "по голосу"}
          </div>
        </div>
        <button
          onClick={leaveVoice}
          title="Отключиться"
          aria-label="Отключиться"
          className={btn(false) + " hover:text-red-400"}
        >
          📴
        </button>
      </div>
      <div
        className="mt-2 flex justify-between"
        role="toolbar"
        aria-label="Управление звонком"
      >
        <button
          onClick={toggleMute}
          aria-pressed={flags.muted}
          title={flags.muted ? "Включить микрофон" : "Выключить микрофон"}
          aria-label="Микрофон"
          className={btn(flags.muted, true)}
        >
          {flags.muted ? "🎙️✕" : "🎙️"}
        </button>
        <button
          onClick={toggleDeafen}
          aria-pressed={flags.deafened}
          title={flags.deafened ? "Включить звук" : "Выключить звук"}
          aria-label="Звук"
          className={btn(flags.deafened, true)}
        >
          {flags.deafened ? "🔇" : "🎧"}
        </button>
        <button
          onClick={() => void toggleCamera()}
          aria-pressed={!!localCamera}
          title="Камера"
          aria-label="Камера"
          className={btn(!!localCamera)}
        >
          📷
        </button>
        <button
          onClick={() => void toggleScreen()}
          aria-pressed={!!localScreen}
          title="Демонстрация экрана"
          aria-label="Демонстрация экрана"
          className={btn(!!localScreen)}
        >
          🖥️
        </button>
        <button
          onClick={() => setSettingsOpen(true)}
          title="Настройки голоса"
          aria-label="Настройки голоса"
          className={btn(false)}
        >
          ⚙️
        </button>
      </div>
      {settingsOpen && <VoiceSettings onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}

const keyName = (code: string) =>
  code === "Backquote"
    ? "`/Ё"
    : code
        .replace(/^Key/, "")
        .replace(/^Digit/, "")
        .replace(/(Left|Right)$/, " $1");

function VoiceSettings({ onClose }: { onClose: () => void }) {
  const settings = useVoice((s) => s.settings);
  const [capturing, setCapturing] = useState(false);

  useEffect(() => {
    if (!capturing) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code !== "Escape") updateSettings({ pttKey: e.code });
      setCapturing(false);
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () =>
      window.removeEventListener("keydown", onKey, { capture: true });
  }, [capturing]);

  return (
    <Modal title="Настройки голоса" onClose={onClose}>
      <fieldset className="space-y-2">
        <legend className="mb-1 text-xs font-semibold uppercase text-zinc-400">
          Режим микрофона
        </legend>
        {(
          [
            [
              "voice",
              "По голосу",
              "Микрофон включается сам, когда вы говорите",
            ],
            ["ptt", "По кнопке", "Микрофон работает, пока зажата клавиша"],
          ] as const
        ).map(([mode, title, hint]) => (
          <label
            key={mode}
            className="flex cursor-pointer items-start gap-3 rounded bg-zinc-900 p-3"
          >
            <input
              type="radio"
              name="micMode"
              checked={settings.micMode === mode}
              onChange={() => updateSettings({ micMode: mode })}
              className="mt-1"
            />
            <span>
              <span className="block font-medium">{title}</span>
              <span className="text-sm text-zinc-400">{hint}</span>
            </span>
          </label>
        ))}
      </fieldset>
      {settings.micMode === "ptt" && (
        <div className="flex items-center gap-3">
          <span className="text-sm text-zinc-300">Клавиша:</span>
          <kbd className="rounded bg-zinc-900 px-2 py-1 font-mono">
            {capturing ? "нажмите клавишу…" : keyName(settings.pttKey)}
          </kbd>
          <button
            onClick={() => setCapturing(true)}
            className="text-sm text-indigo-400 hover:underline"
          >
            Изменить
          </button>
        </div>
      )}
      <p className="text-xs text-zinc-500">
        Клавиша работает, пока окно NexusChat активно. В полях ввода она
        печатает текст, а не включает микрофон.
      </p>
      <div className="flex justify-end">
        <button onClick={onClose} className={primaryButton}>
          Готово
        </button>
      </div>
    </Modal>
  );
}
