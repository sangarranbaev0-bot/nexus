import { useEffect, useState } from 'react';
import { errorMessage } from './Modal';
import type { NotifyLevel } from './generated/NotifyLevel';
import type { NotifySetting } from './generated/NotifySetting';
import {
  channelLevel,
  loadNotifySettings,
  serverLevel,
  setChannelNotify,
  setServerNotify,
  useNotify,
} from './notify';

const serverKey = (id: number) => `s:${id}`;
const channelKey = (id: number) => `c:${id}`;

const LEVELS: { value: NotifyLevel; label: string; hint: string }[] = [
  { value: 'all', label: 'Все сообщения', hint: 'Уведомлять о каждом новом сообщении' },
  { value: 'mentions', label: 'Только @упоминания', hint: 'Только когда вас упомянули или ответили' },
  { value: 'none', label: 'Ничего', hint: 'Никаких уведомлений' },
];

const MUTE_MINUTES = [15, 60, 180, 480, 1440, 10080];

function formatUntil(ms: number) {
  return new Date(ms).toLocaleString('ru-RU', {
    day: 'numeric',
    month: 'long',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** Выбор уровня уведомлений и временного mute для сервера или канала. */
export function NotifyControls({
  target,
  inheritHint,
}: {
  target: { serverId: number } | { channelId: number };
  inheritHint?: string;
}) {
  const settings = useNotify((s) => s.settings);
  const [level, setLevel] = useState<NotifyLevel>('all');
  const [mutedUntil, setMutedUntil] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const serverId = 'serverId' in target ? target.serverId : undefined;
  const channelId = 'channelId' in target ? target.channelId : undefined;
  const key = serverId !== undefined ? serverKey(serverId) : channelKey(channelId!);
  const current: NotifySetting | undefined = settings?.[key];

  useEffect(() => {
    if (settings === null) void loadNotifySettings().catch((e) => setError(errorMessage(e)));
  }, [settings]);

  useEffect(() => {
    if (current) {
      setLevel(current.level);
      setMutedUntil(current.mutedUntil && current.mutedUntil > Date.now() ? current.mutedUntil : null);
    } else if (settings) {
      setLevel(channelId !== undefined ? channelLevel(channelId) : serverLevel(serverId!));
      setMutedUntil(null);
    }
  }, [current, settings, serverId, channelId]);

  const save = async (nextLevel: NotifyLevel, nextMute: number | null) => {
    setPending(true);
    setError(null);
    try {
      if (serverId !== undefined) await setServerNotify(serverId, nextLevel, nextMute);
      else await setChannelNotify(channelId!, nextLevel, nextMute);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setPending(false);
    }
  };

  const mute = (minutes: number | null) => {
    const until = minutes === null ? null : Date.now() + minutes * 60_000;
    setMutedUntil(until);
    void save(level, until);
  };

  return (
    <div className="space-y-3">
      {inheritHint && <p className="text-xs text-zinc-500">{inheritHint}</p>}
      <fieldset className="space-y-1">
        <legend className="mb-1 text-xs font-semibold uppercase text-zinc-400">
          Уведомления
        </legend>
        {LEVELS.map((opt) => (
          <label
            key={opt.value}
            title={opt.hint}
            className="flex items-start gap-2 rounded px-1 py-1 text-sm hover:bg-zinc-700/40"
          >
            <input
              type="radio"
              name={`notify-${key}`}
              checked={level === opt.value}
              disabled={pending}
              onChange={() => {
                setLevel(opt.value);
                void save(opt.value, mutedUntil);
              }}
              className="mt-1"
            />
            <span>{opt.label}</span>
          </label>
        ))}
      </fieldset>

      <div>
        <span className="mb-1 block text-xs font-semibold uppercase text-zinc-400">
          Не беспокоить до
        </span>
        <div className="flex flex-wrap items-center gap-1.5">
          {mutedUntil ? (
            <>
              <span className="text-sm text-amber-400">{formatUntil(mutedUntil)}</span>
              <button
                type="button"
                onClick={() => mute(null)}
                disabled={pending}
                className="rounded px-2 py-1 text-sm text-zinc-300 hover:bg-zinc-700 disabled:opacity-60"
              >
                Снять
              </button>
            </>
          ) : (
            MUTE_MINUTES.map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => mute(m)}
                disabled={pending}
                className="rounded bg-zinc-700 px-2 py-1 text-xs text-zinc-200 hover:bg-zinc-600 disabled:opacity-60"
              >
                {m < 60 ? `${m} мин` : m < 1440 ? `${m / 60} ч` : m < 10080 ? `${m / 1440} дн` : '7 дн'}
              </button>
            ))
          )}
        </div>
      </div>

      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}
