import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { jumpToMessage } from './ChatView';
import { errorMessage } from './Modal';
import type { Notification } from './generated/Notification';
import {
  loadNotifications,
  markNotificationRead,
  unreadCount,
  useNotify,
} from './notify';

const kindLabel: Record<Notification['kind'], string> = {
  mention: 'Упомянул вас',
  reply: 'Ответил вам',
  word: 'Слово-триггер',
  everyone: '@everyone',
};

type Filter = 'all' | 'unread' | 'mentions';

/** Колокольчик со счётчиком непрочитанных и выпадающим списком. */
export function NotificationCenter() {
  const navigate = useNavigate();
  const center = useNotify((s) => s.center);
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState<Filter>('all');
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void loadNotifications().catch((e) => setError(errorMessage(e)));
  }, []);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  const unread = unreadCount(center);
  const items = (center ?? []).filter((n) => {
    if (filter === 'unread') return !n.read;
    if (filter === 'mentions') return n.kind === 'mention' || n.kind === 'reply' || n.kind === 'everyone';
    return true;
  });

  const openItem = (n: Notification) => {
    setOpen(false);
    void markNotificationRead(n.id).catch(() => {});
    // Переходим в канал; если сообщение уже загружено, подсветим его.
    navigate(n.serverId !== null ? `/channels/${n.serverId}/${n.channelId}` : `/channels/@me/${n.channelId}`);
    requestAnimationFrame(() => setTimeout(() => jumpToMessage(n.messageId), 120));
  };

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={unread ? `Уведомления, непрочитанных: ${unread}` : 'Уведомления'}
        title="Уведомления"
        className="relative rounded px-2 py-1 text-lg text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
      >
        🔔
        {unread > 0 && (
          <span
            aria-hidden
            className="absolute -top-0.5 -right-0.5 rounded-full bg-red-500 px-1.5 text-xs font-bold text-white"
          >
            {unread > 99 ? '99+' : unread}
          </span>
        )}
      </button>

      {open && (
        <div
          role="menu"
          className="absolute right-0 z-30 mt-1 w-96 max-w-[calc(100vw-2rem)] rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl"
        >
          <div className="flex items-center gap-1 border-b border-zinc-800 p-2">
            {(
              [
                ['all', 'Все'],
                ['unread', 'Непрочитанное'],
                ['mentions', 'Упоминания'],
              ] as const
            ).map(([key, label]) => (
              <button
                key={key}
                onClick={() => setFilter(key)}
                className={`rounded px-2 py-1 text-sm ${
                  filter === key
                    ? 'bg-zinc-700 text-white'
                    : 'text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200'
                }`}
              >
                {label}
              </button>
            ))}
            <button
              onClick={() => void markNotificationRead().catch((e) => setError(errorMessage(e)))}
              disabled={unread === 0}
              className="ml-auto rounded px-2 py-1 text-sm text-zinc-400 hover:text-zinc-100 disabled:opacity-40"
            >
              Прочитать всё
            </button>
          </div>

          {error && (
            <p role="alert" className="px-3 py-2 text-sm text-red-400">
              {error}
            </p>
          )}

          <ul className="max-h-96 overflow-y-auto">
            {items.length === 0 && (
              <li className="px-3 py-6 text-center text-sm text-zinc-500">Уведомлений нет.</li>
            )}
            {items.map((n) => (
              <li key={n.id}>
                <button
                  onClick={() => openItem(n)}
                  className={`flex w-full flex-col gap-0.5 px-3 py-2 text-left hover:bg-zinc-800 ${
                    n.read ? 'opacity-60' : ''
                  }`}
                >
                  <span className="flex items-center gap-2 text-sm">
                    {!n.read && <span className="size-2 shrink-0 rounded-full bg-indigo-400" />}
                    <span className="font-medium text-zinc-100">{n.actorUsername}</span>
                    <span className="text-xs text-zinc-400">{kindLabel[n.kind]}</span>
                    <span className="ml-auto shrink-0 text-xs text-zinc-500">
                      {new Date(n.createdAt).toLocaleString('ru-RU', {
                        day: 'numeric',
                        month: 'short',
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </span>
                  </span>
                  <span className="truncate text-sm text-zinc-300">{n.preview || '📎 вложение'}</span>
                  {n.location && <span className="truncate text-xs text-zinc-500">{n.location}</span>}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
