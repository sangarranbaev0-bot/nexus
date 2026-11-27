import { useEffect, useRef, useState } from 'react';
import type { Message } from './generated/Message';
import { inputClass, primaryButton } from './Modal';
import { Markdown } from './Markdown';
import { searchServerMessages, type ServerSearchParams, type ServerSearchResult } from './guilds';

interface Props {
  serverId: number;
  onClose: () => void;
}

export function SearchDialog({ serverId, onClose }: Props) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<ServerSearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      setError(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    const timer = setTimeout(async () => {
      try {
        const params: ServerSearchParams = { q: query.trim(), limit: 50 };
        const res = await searchServerMessages(serverId, params);
        if (!cancelled) {
          setResults(res.results);
          setError(null);
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
  }, [query, serverId]);

  const jumpToMessage = async (messageId: number) => {
    // Находим канал результата и переходим к сообщению
    const hit = results.find((r) => r.messageId === messageId);
    if (!hit) return;
    window.location.href = `/channels/${serverId}/${hit.channelId}#msg-${messageId}`;
  };

  return (
    <div className="m-auto rounded-lg bg-nc-elevated p-0 text-nc-text backdrop:bg-black/70 w-[min(72rem,calc(100vw-2rem))] w-full max-w-2xl flex flex-col max-h-[80vh]">
      <h2 className="text-lg font-semibold mb-4">Поиск по серверу</h2>
      <div className="flex gap-2 mb-4">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') onClose();
            if (e.key === 'Enter' && results[0]) jumpToMessage(results[0].messageId);
          }}
          placeholder="Поиск: from:user has:link ключевое слово"
          className={inputClass}
        />
        <button onClick={onClose} className={primaryButton}>Закрыть</button>
      </div>

      {error && <p role="alert" className="text-red-400 mb-2">{error}</p>}
      {loading && <p className="text-nc-muted mb-2">Поиск…</p>}
      {!loading && !error && query && results.length === 0 && (
        <p className="text-nc-muted mb-2">Нет сообщений по запросу.</p>
      )}

      <ul className="flex-1 overflow-y-auto space-y-1 pr-2">
        {results.map((r) => (
          <li key={r.messageId}>
            <button
              type="button"
              onClick={() => jumpToMessage(r.messageId)}
              className="w-full rounded p-2 text-left hover:bg-nc-hover transition-colors"
            >
              <div className="flex items-center gap-2 text-xs text-nc-faint">
                <span className="font-medium text-nc-text-2">{r.message.authorUsername}</span>
                <span>#{r.channelName}</span>
                <span>{new Date(r.message.createdAt).toLocaleString('ru-RU')}</span>
              </div>
              <div className="mt-1 truncate text-sm text-nc-text-2">
                <Markdown text={r.message.content} />
              </div>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
