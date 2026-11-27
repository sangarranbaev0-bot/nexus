import { useEffect, useState } from 'react';
import { errorMessage } from './Modal';
import type { NotifyLevel } from './generated/NotifyLevel';
import type { NotifySetting } from './generated/NotifySetting';
import { serverOfChannel } from './guilds';
import {
  channelLevel,
  loadNotifySettings,
  serverLevel,
  serverSuppressEveryone,
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

function muteLabel(m: number) {
  if (m < 60) return `${m} мин`;
  if (m < 1440) return `${m / 60} ч`;
  if (m < 10080) return `${m / 1440} дн`;
  return '7 дн';
}

/** Выбор уровня уведомлений, подавления @everyone и временного mute для сервера или канала. */
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
  const [suppress, setSuppress] = useState(false);
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
      setSuppress(current.suppressEveryone);
    } else if (settings) {
      setLevel(channelId !== undefined ? channelLevel(channelId) : serverLevel(serverId!));
      setMutedUntil(null);
      const inheritedServer = channelId !== undefined ? serverOfChannel(channelId) : undefined;
      setSuppress(
        inheritedServer !== undefined ? serverSuppressEveryone(inheritedServer) : false,
      );
    }
  }, [current, settings, serverId, channelId]);

  const save = async (nextLevel: NotifyLevel, nextMute: number | null, nextSuppress: boolean) => {
    setPending(true);
    setError(null);
    try {
      if (serverId !== undefined)
        await setServerNotify(serverId, nextLevel, nextMute, nextSuppress);
      else await setChannelNotify(channelId!, nextLevel, nextMute, nextSuppress);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setPending(false);
    }
  };

  const mute = (minutes: number | null) => {
    const until = minutes === null ? null : Date.now() + minutes * 60_000;
    setMutedUntil(until);
    void save(level, until, suppress);
  };

  return (
    <div className="space-y-3">
      {inheritHint && <p className="text-xs text-nc-faint">{inheritHint}</p>}
      <fieldset className="space-y-1">
        <legend className="mb-1 text-xs font-semibold uppercase text-nc-muted">
          Уведомления
        </legend>
        {LEVELS.map((opt) => (
          <label
            key={opt.value}
            title={opt.hint}
            className="flex items-start gap-2 rounded px-1 py-1 text-sm hover:bg-nc-hover/40"
          >
            <input
              type="radio"
              name={`notify-${key}`}
              checked={level === opt.value}
              disabled={pending}
              onChange={() => {
                setLevel(opt.value);
                void save(opt.value, mutedUntil, suppress);
              }}
              className="mt-1"
            />
            <span>{opt.label}</span>
          </label>
        ))}
      </fieldset>

      <label className="flex items-start gap-2 rounded px-1 py-1 text-sm hover:bg-nc-hover/40">
        <input
          type="checkbox"
          checked={suppress}
          disabled={pending}
          onChange={(e) => {
            setSuppress(e.target.checked);
            void save(level, mutedUntil, e.target.checked);
          }}
          className="mt-1"
        />
        <span title="Не получать уведомления от @everyone и @here даже при праве автора">
          Глушить @everyone и @here
        </span>
      </label>

      <div>
        <span className="mb-1 block text-xs font-semibold uppercase text-nc-muted">
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
                className="rounded px-2 py-1 text-sm text-nc-text-2 hover:bg-nc-hover disabled:opacity-60"
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
                className="rounded bg-nc-hover px-2 py-1 text-xs text-nc-text-2 hover:bg-nc-active disabled:opacity-60"
              >
                {muteLabel(m)}
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
