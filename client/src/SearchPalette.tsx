import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { Markdown } from './Markdown';
import { inputClass, primaryButton } from './Modal';
import { searchServerMessages, type ServerSearchParams, type ServerSearchResult } from './guilds';
import { useGuilds } from './guilds';
import { dmTitle, useDms } from './dms';
import { StatusDot } from './Dialogs';
import { Search, ArrowRight } from 'lucide-react';

/** Результат глобального поиска: сообщение из любого канала + метаданные. */
export interface GlobalHit {
  messageId: number;
  serverId: number;
  channelId: number;
  channelName: string;
  serverName: string;
  message: { authorUsername: string; content: string; createdAt: number };
}

interface Props {
  onClose: () => void;
}

export function SearchPalette({ onClose }: Props) {
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<GlobalHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [highlightedIdx, setHighlightedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const servers = useGuilds((s) => s.servers ?? []);
  const dms = useDms((s) => s.dms ?? []);
  const channels = useGuilds((s) => s.channels ?? {});

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Выполняем поиск по всем серверам при вводе (задержка 300 мс).
  useEffect(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      setResults([]);
      setError(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    const timer = setTimeout(async () => {
      try {
        // Параллельно ищем по каждому серверу, объединяем результаты.
        const tasks = servers.map((srv) =>
          searchServerMessages(srv.id, { q, limit: 10 })
            .then((res) =>
              res.results.map((r: ServerSearchResult) => ({
                messageId: r.messageId,
                serverId: srv.id,
                channelId: r.channelId,
                channelName: r.channelName,
                serverName: srv.name,
                message: {
                  authorUsername: r.message.authorUsername,
                  content: r.message.content,
                  createdAt: r.message.createdAt,
                },
              })),
            )
            .catch(() => [] as GlobalHit[]),
        );
        const all = (await Promise.all(tasks)).flat();
        if (!cancelled) {
          setResults(all.slice(0, 50));
          setHighlightedIdx(0);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, servers]);

  const jumpTo = (hit: GlobalHit) => {
    navigate(`/channels/${hit.serverId}/${hit.channelId}#msg-${hit.messageId}`);
    onClose();
  };

  // Навигация стрелками: Enter/ArrowDown — выбрать, ArrowUp — всплыть, Escape — закрыть.
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') { onClose(); return; }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setHighlightedIdx((i) => Math.min(i + 1, results.length - 1));
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      setHighlightedIdx((i) => Math.max(i - 1, 0));
    }
    if (e.key === 'Enter' && results[highlightedIdx]) {
      e.preventDefault();
      jumpTo(results[highlightedIdx]);
    }
  };

  const q = query.trim().toLowerCase();

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh] px-4">
      {/* Затемнение фона: ловит клик за пределами палитры. */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-2xl rounded-xl border border-nc-line-strong bg-nc-elevated shadow-2xl">
        {/* Поле поиска с иконкой */}
        <div className="flex items-center gap-2 border-b border-nc-line-strong px-4 py-3">
          <Search size={18} className="shrink-0 text-nc-muted" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Поиск по всем серверам…  (Esc — закрыть)"
            className="flex-1 bg-transparent text-sm text-nc-text outline-none placeholder:text-nc-faint"
          />
          <button onClick={onClose} className={primaryButton}>Esc</button>
        </div>

        {/* Содержимое */}
        <div className="max-h-[50vh] overflow-y-auto py-2">
          {!q && !loading && (
            <>
              {/* Быстрая навигация по серверам */}
              {servers.length > 0 && (
                <div className="px-4 py-2">
                  <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-nc-faint">Серверы</p>
                  {servers.map((srv) => (
                    <button
                      key={srv.id}
                      onClick={() => { navigate(`/channels/${srv.id}`); onClose(); }}
                      className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-nc-hover"
                    >
                      <span className="size-7 grid place-items-center rounded-full bg-nc-hover text-xs font-bold text-nc-accent-text">
                        {srv.name.charAt(0).toUpperCase()}
                      </span>
                      <span className="truncate font-medium text-nc-text-2">{srv.name}</span>
                      {srv.unreadCount > 0 && (
                        <span className="ml-auto rounded-full bg-red-500 px-1.5 text-xs font-bold text-white">{srv.unreadCount}</span>
                      )}
                      <ArrowRight size={14} className="ml-auto shrink-0 text-nc-faint" />
                    </button>
                  ))}
                </div>
              )}
              {/* ДМ */}
              {dms.length > 0 && (
                <div className="px-4 py-2">
                  <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-nc-faint">Личные сообщения</p>
                  {dms.slice(0, 10).map((dm) => (
                    <button
                      key={dm.id}
                      onClick={() => { navigate(`/channels/@me/${dm.id}`); onClose(); }}
                      className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-nc-hover"
                    >
                      <StatusDot userId={dm.userId} fallback={dm.status} />
                      <span className="flex-1 truncate text-nc-text-2">{dmTitle(dm)}</span>
                      {dm.unreadCount > 0 && (
                        <span className="rounded-full bg-red-500 px-1.5 text-xs font-bold text-white">{dm.unreadCount}</span>
                      )}
                    </button>
                  ))}
                </div>
              )}
              <p className="px-4 py-3 text-xs text-nc-faint">Введите текст для поиска сообщений по всем серверам.</p>
            </>
          )}

          {error && (
            <p role="alert" className="px-4 py-2 text-sm text-red-400">{error}</p>
          )}
          {loading && (
            <p className="px-4 py-3 text-sm text-nc-muted">Поиск…</p>
          )}

          {/* Результаты поиска */}
          {!loading && !error && q && results.length === 0 && (
            <p className="px-4 py-3 text-sm text-nc-muted">Нет результатов по запросу «{query}».</p>
          )}
          {!loading && !error && results.length > 0 && (
            <ul className="pb-2">
              {results.map((hit, idx) => (
                <li key={hit.messageId}>
                  <button
                    type="button"
                    onClick={() => jumpTo(hit)}
                    onMouseEnter={() => setHighlightedIdx(idx)}
                    className={`flex w-full flex-col gap-0.5 px-4 py-2 text-left ${
                      idx === highlightedIdx ? 'bg-nc-hover/50' : ''
                    } hover:bg-nc-hover`}
                  >
                    <div className="flex items-center gap-2 text-xs text-nc-faint">
                      <span className="font-medium text-nc-text-2">{hit.message.authorUsername}</span>
                      <span>·</span>
                      <span className="text-nc-accent-text">#{hit.channelName}</span>
                      <span>·</span>
                      <span className="text-nc-faint">{hit.serverName}</span>
                      <span className="ml-auto shrink-0 text-nc-faint">
                        {new Date(hit.message.createdAt).toLocaleString('ru-RU', { hour: '2-digit', minute: '2-digit' })}
                      </span>
                    </div>
                    <div className="truncate text-sm text-nc-text-2">
                      <Markdown text={hit.message.content} />
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
