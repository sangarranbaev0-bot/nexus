import { Fragment, useEffect, useLayoutEffect, useRef, useState, type FormEvent, type KeyboardEvent } from 'react';
import { Virtuoso } from 'react-virtuoso';
import type { Attachment } from './generated/Attachment';
import type { Message } from './generated/Message';
import type { MessageEdit } from './generated/MessageEdit';
import type { SearchResponse } from './generated/SearchResponse';
import type { SearchResult } from './generated/SearchResult';
import type { Sticker } from './generated/Sticker';
import { api, uploadFile } from './api';
import { ContextMenu } from './ContextMenu';
import {
  activeView,
  deleteMessage,
  editMessage,
  forwardMessage,
  isViewing,
  loadInitial,
  loadOlder,
  loadPins,
  loadEdits,
  markLastRead,
  markRead,
  sendMessage,
  setPinned,
  toggleReaction,
  useMessages,
} from './messages';
import { Modal, errorMessage, ghostButton, inputClass, primaryButton } from './Modal';
import { Markdown } from './Markdown';
import { EmojiPicker } from './EmojiPicker';
import { StickerPicker } from './StickerPicker';
import { useSession } from './session';
import { sendTyping } from './socket';
import { createThread, listThreads, loadReplies, loadEmbed, updateThread, useGuilds } from './guilds';
import { dmTitle, useDms } from './dms';

const MAX_LEN = 2000;
const TYPING_EVERY_MS = 3000;
const NEAR_BOTTOM_PX = 120;
const LOAD_OLDER_PX = 200;

/** Что показываем: id канала и подпись вида «#общий» или «@bob». */
export interface ChatTarget {
  id: number;
  label: string;
  /** Непрочитанных на момент открытия: по нему рисуем линию «НОВОЕ». */
  unread?: number;
  /** Сервер id: позволяет показывать поиск в ЛС тоже. */
  serverId?: number;
  /** Форум-канал: вместо ленты показываем ветки. */
  isForum?: boolean;
}

/** Один результат поиска: сообщение + представитель канала. */
export interface SearchHit {
  message: Message;
  channelLabel: string;
}

export function ChatView({
  channel,
  canModerate,
  canPin,
  onRead,
}: {
  channel: ChatTarget;
  canModerate: boolean;
  canPin: boolean;
  /** Пользователь увидел последнее сообщение: сбросить счётчик непрочитанных. */
  onRead: () => void;
}) {
  const log = useMessages((s) => s.logs[channel.id]);
  const [error, setError] = useState<string | null>(null);
  const [showPins, setShowPins] = useState(false);
  const [showUnreadOnly, setShowUnreadOnly] = useState(false);
  const [searchVisible, setSearchVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<SearchHit[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [hasMoreSearch, setHasMoreSearch] = useState(false);

  useEffect(() => {
    setError(null);
    loadInitial(channel.id).catch((e) => setError(errorMessage(e)));
  }, [channel.id, log === undefined]);

  const [dropped, setDropped] = useState<{ files: File[] } | null>(null);
  const [dragging, setDragging] = useState(false);
  const [replyTo, setReplyTo] = useState<Message | null>(null);
  const initialUnread = useRef(channel.unread ?? 0).current;
  // При открытии канала отмечаем «с этого момента прочитано», чтобы фильтр «непрочитанные» работал.
  useEffect(() => { markLastRead(channel.id); }, []);
  const lastReadAt = useMessages((s) => s.lastReadAt[channel.id] ?? null);
  const channels = useGuilds((s) => s.channels);

  // Search effect
  useEffect(() => {
    if (!searchVisible || !searchQuery.trim()) {
      setSearchResults([]);
      setHasMoreSearch(false);
      setSearchError(null);
      setSearchLoading(false);
      return;
    }
    let cancelled = false;
    setSearchLoading(true);
    setSearchError(null);
    
    const timer = setTimeout(async () => {
      try {
        const res = await api<SearchResponse>(
          `/api/channels/${channel.id}/messages/search?q=${encodeURIComponent(searchQuery)}&limit=50`,
        );
        if (!cancelled) {
          setSearchResults(res.results.map((r: SearchResult) => ({
            message: r.message,
            channelLabel: r.channelName,
          })));
          setHasMoreSearch(res.hasMore);
        }
      } catch (e) {
        if (!cancelled) {
          setSearchError(errorMessage(e));
        }
      } finally {
        if (!cancelled) {
          setSearchLoading(false);
        }
      }
    }, 300); // debounce

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [searchVisible, searchQuery, channel.id]);

  return (
    <div
      className={`relative flex min-h-0 flex-1 flex-col ${dragging ? 'ring-2 ring-indigo-500 ring-inset' : ''}`}
      onDragOver={(e) => {
        if (!e.dataTransfer.types.includes('Files')) return;
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={(e) => e.currentTarget === e.target && setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        if (e.dataTransfer.files.length) setDropped({ files: Array.from(e.dataTransfer.files) });
      }}
    >
      {dragging && (
        <div className="pointer-events-none absolute inset-0 z-10 grid place-items-center bg-zinc-900/70 text-lg">
          Отпустите, чтобы прикрепить к {channel.label}
        </div>
      )}
      {channel.isForum ? (
        <ForumThreads channel={channel} onError={setError} />
      ) : (
        <MessageList
          channel={channel}
          canModerate={canModerate}
          canPin={canPin}
          onRead={onRead}
          items={log?.items}
          hasMore={!!log?.hasMore}
          unread={initialUnread}
          showUnreadOnly={showUnreadOnly}
          lastReadAt={lastReadAt}
          onError={setError}
          onReply={setReplyTo}
        />
      )}
      <SearchBar
        visible={searchVisible}
        query={searchQuery}
        onToggle={() => {
          setSearchVisible((v) => !v);
          setSearchQuery('');
          setSearchResults([]);
          setSearchError(null);
        }}
        onQueryChange={setSearchQuery}
        loading={searchLoading}
        error={searchError}
        results={searchResults}
        hasMore={hasMoreSearch}
        onJump={(id) => requestAnimationFrame(() => jumpToMessage(id))}
      />
      <button
        type="button"
        onClick={() => setSearchVisible((v) => !v)}
        title="Поиск в канале (Ctrl+F)"
        className="absolute top-2 right-14 z-10 rounded bg-zinc-900/80 px-2 py-1 text-xs text-zinc-300 backdrop-blur hover:bg-zinc-800"
        aria-label="Поиск"
      >
        🔍
      </button>
        <button
          type="button"
          onClick={() => setShowPins(true)}
          title="Закреплённые сообщения"
          className="absolute top-1 right-4 z-10 rounded bg-zinc-900/80 px-2 py-1 text-xs text-zinc-300 backdrop-blur hover:bg-zinc-800"
        >
          📌 Закреплённые
        </button>
        <button
          type="button"
          onClick={() => setShowUnreadOnly((v) => !v)}
          title={showUnreadOnly ? 'Показать все сообщения' : 'Показать только непрочитанные'}
          className={`absolute top-1 right-28 z-10 rounded bg-zinc-900/80 px-2 py-1 text-xs backdrop-blur ${showUnreadOnly ? 'bg-red-500/20 text-red-300 hover:bg-red-500/30' : 'text-zinc-300 hover:bg-zinc-800'}`}
        >
          {showUnreadOnly ? '🔥 Все' : '🔥 Непрочитанные'}
        </button>
      {showPins && (
        <PinsDialog
          channelId={channel.id}
          onClose={() => setShowPins(false)}
          onJump={(id) => {
            setShowPins(false);
            requestAnimationFrame(() => jumpToMessage(id));
          }}
          onError={setError}
        />
      )}
      {error && (
        <p role="alert" className="px-4 text-sm text-red-400">
          {error}
        </p>
      )}
      <Composer channel={channel} dropped={dropped} replyTo={replyTo} onCancelReply={() => setReplyTo(null)} />
      <TypingIndicator channelId={channel.id} />
    </div>
  );
}

function MessageList({
   channel,
     canModerate,
     canPin,
     onRead,
     items,
     hasMore,
     unread,
     onError,
     onReply,
     showUnreadOnly,
     lastReadAt,
   }: {
     channel: ChatTarget;
     canModerate: boolean;
     canPin: boolean;
     onRead: () => void;
     items: Message[] | undefined;
     hasMore: boolean;
     unread: number;
     onError: (e: string) => void;
     onReply: (m: Message) => void;
     showUnreadOnly: boolean;
     lastReadAt: number | null;
   }) {
   const nearBottom = useRef(true);
   const prev = useRef<{ first?: number; last?: number; height: number }>({ height: 0 });
   const latest = useRef(items);
   latest.current = items;
   // Фильтруем только непрочитанные, если включён соответствующий режим.
   const visibleItems =
     showUnreadOnly && lastReadAt !== null && items ? items.filter((m) => m.createdAt > lastReadAt) : items;

   // Видно последнее сообщение — канал прочитан.
   const ackIfViewing = () => {
     activeView.atBottom = nearBottom.current;
     const last = latest.current?.at(-1);
     if (last && isViewing(channel.id)) {
       markRead(channel.id, last.id);
       onRead();
     }
   };

    // Сохраняем позицию при подгрузке старых и прилипаем к низу при новых.
    useLayoutEffect(() => {
      // Note: With Virtuoso, scroll position is managed by the library
      // but we still need to track first/last for loadOlder/auto-scroll
      if (!items) return;
      const first = items[0]?.id;
      const last = items.at(-1)?.id;
      const p = prev.current;
      if (p.first !== undefined && first !== p.first && last === p.last) {
        // Skip - we rely on Virtuoso's auto-scroll
      } else if (last !== p.last && (nearBottom.current || p.last === undefined)) {
        nearBottom.current = false;
      }
      prev.current = { first, last, height: 0 };
      ackIfViewing();
    }, [items]);

    // Картинки получают высоту только после загрузки: если были внизу, остаёмся внизу.
    useEffect(() => {
      const onLoad = (e: Event) => {
        if (e.target instanceof HTMLImageElement && nearBottom.current) {
          // Virtuoso auto-scrolls on size changes, so we don't need manual scroll
        }
      };
      // This event listener won't fire in the same way with Virtuoso
      // but we keep it for compatibility
      return;
    }, []);

    useEffect(() => {
      nearBottom.current = true;
      prev.current = { height: 0 };
      activeView.channelId = channel.id;
      activeView.atBottom = true;
      document.addEventListener('visibilitychange', ackIfViewing);
      return () => {
        document.removeEventListener('visibilitychange', ackIfViewing);
        if (activeView.channelId === channel.id) activeView.channelId = undefined;
      };
    }, [channel.id]);

    // Calculate firstUnread index for rendering the "NEW" divider
    const firstUnread = unread > 0 && unread < (visibleItems || []).length ? (visibleItems || []).length - unread : -1;

    return (
      <div className="min-h-0 flex-1 overflow-y-auto py-4" role="log" aria-label={`Сообщения ${channel.label}`}>
        {visibleItems && !hasMore && (
          <div className="px-4 pb-4">
            <h3 className="text-2xl font-bold">Добро пожаловать в {channel.label}!</h3>
            <p className="text-zinc-400">
              {channel.label.startsWith("@") ? "Это начало вашей переписки." : "Это начало канала."}
            </p>
          </div>
        )}
        {visibleItems === undefined && <p className="px-4 text-zinc-400">Загрузка…</p>}
        {showUnreadOnly && lastReadAt !== null && visibleItems && visibleItems.length === 0 && items && items.some((m) => m.createdAt > lastReadAt) === false && (
          <p className="px-4 text-sm text-zinc-500">Нет непрочитанных сообщений после последнего чтения.</p>
        )}
         <Virtuoso
           className="w-full"
           style={{ height: '100%' }}
           data={visibleItems || []}
           itemContent={(index: number, message: Message, context: any) => {
             const arr = visibleItems || [];
             const prevMsg = index > 0 ? arr[index - 1] : null;
             const prevDate = prevMsg ? new Date(prevMsg.createdAt).toDateString() : '';
             const currDate = new Date(message.createdAt).toDateString();
             const dayChanged = currDate !== prevDate;
             return (
               <Fragment key={message.id}>
                 {dayChanged && (
                   <div className="my-3 flex items-center gap-3 px-4 text-xs font-semibold text-zinc-500">
                     <span className="h-px flex-1 bg-zinc-700" />
                     <time dateTime={new Date(message.createdAt).toISOString()}>
                       {new Intl.DateTimeFormat('ru', { weekday: 'long', day: 'numeric', month: 'long' }).format(new Date(message.createdAt))}
                     </time>
                     <span className="h-px flex-1 bg-zinc-700" />
                   </div>
                 )}
                 {index === firstUnread && (
                   <div
                     data-new-divider
                     role="separator"
                     aria-label="Новые сообщения"
                     className="my-2 flex items-center gap-3 px-4 text-xs font-semibold text-red-400"
                   >
                     <span className="h-px flex-1 bg-red-500/60" />
                     НОВОЕ
                     <span className="h-px flex-1 bg-red-500/60" />
                   </div>
                 )}
                 <MessageItem
                   message={message}
                   canModerate={canModerate}
                   canPin={canPin}
                   onError={onError}
                   onReply={onReply}
                 />
               </Fragment>
             );
           }}
           followOutput={(isAtBottom: boolean) => {
             if (isAtBottom) {
               // Auto-scroll to bottom when new messages arrive and user is at bottom
               nearBottom.current = true;
               return 'auto';
             }
             // Don't auto-scroll if user scrolled up
             return false;
           }}
           atBottomStateChange={(atBottom: boolean) => {
             nearBottom.current = atBottom;
             if (atBottom) {
               // User scrolled to bottom - mark as read
               ackIfViewing();
             }
           }}
           endReached={(lastIndex: number) => {
             // Load older messages when reaching the end (top of the list)
             if (hasMore) {
               loadOlder(channel.id).catch((e) => onError(errorMessage(e)));
             }
           }}
           alignToBottom={true}
         />
       </div>
    );
  }

const timeFmt = new Intl.DateTimeFormat('ru', { hour: '2-digit', minute: '2-digit' });
const dateFmt = new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });

function formatTime(ms: number) {
  const d = new Date(ms);
  return d.toDateString() === new Date().toDateString() ? `сегодня в ${timeFmt.format(d)}` : dateFmt.format(d);
}

function MessageItem({
  message: m,
  canModerate,
  canPin,
  onError,
  onReply,
}: {
  message: Message;
  canModerate: boolean;
  canPin: boolean;
  onError: (e: string) => void;
  onReply: (m: Message) => void;
}) {
  const me = useSession((s) => s.user);
  const [editing, setEditing] = useState(false);
  const [history, setHistory] = useState<MessageEdit[] | null>(null);
  const [forwarding, setForwarding] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number } | null>(null);
  const mine = m.authorId === me?.id;

  const remove = async () => {
    if (!confirm('Удалить сообщение?')) return;
    try {
      await deleteMessage(m);
    } catch (e) {
      onError(errorMessage(e));
    }
  };

  const togglePin = async () => {
    try {
      await setPinned(m.id, !m.pinned);
    } catch (e) {
      onError(errorMessage(e));
    }
  };

  return (
    <article
      id={`msg-${m.id}`}
      tabIndex={0}
      onContextMenu={(e) => {
        e.preventDefault();
        setCtx({ x: e.clientX, y: e.clientY });
      }}
      className="group relative px-4 py-1 hover:bg-zinc-900/30 focus:bg-zinc-900/30"
    >
      {m.systemKind && <SystemMessage message={m} />}
      {!m.systemKind && (
        <>
      <header className="flex items-baseline gap-2">
        <span className="font-medium text-indigo-300">{m.authorUsername}{m.authorBadge ? ` · ${m.authorBadge}` : ''}</span>
        <time className="text-xs text-zinc-500" dateTime={new Date(m.createdAt).toISOString()}>
          {formatTime(m.createdAt)}
        </time>
        {m.pinned && (
          <span title="Закреплено" className="text-xs text-amber-300">
            📌
          </span>
        )}
        {m.editedAt !== null && (
          <span className="text-xs text-zinc-500" title={formatTime(m.editedAt)}>
            (изменено)
          </span>
        )}
      </header>
      {m.replyTo && <ReplyQuote reply={m.replyTo} />}
      {m.forwardedFrom && <ForwardQuote from={m.forwardedFrom} />}
      {editing ? (
        <EditBox message={m} onDone={() => setEditing(false)} onError={onError} />
      ) : (
        (m.content || m.editedAt !== null) && (
          <div className="break-words whitespace-pre-wrap text-zinc-100">
            <Markdown text={m.content} />
          </div>
        )
      )}
      <Attachments files={m.attachments} />
      {m.hasEmbed && <EmbedBlock messageId={m.id} />}
      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          onClose={() => setCtx(null)}
          items={[
            { label: 'Ответить', onClick: () => { onReply(m); setCtx(null); } },
            { label: 'Переслать', onClick: () => { setForwarding(true); setCtx(null); } },
            { label: 'ID', onClick: () => { void navigator.clipboard?.writeText(String(m.id)); setCtx(null); } },
            { label: 'Ссылка', onClick: () => { void navigator.clipboard?.writeText(window.location.origin + '/nexus/channels#' + m.id); setCtx(null); } },
            ...(canPin ? [{ label: m.pinned ? 'Открепить' : 'Закрепить', onClick: () => { void togglePin(); setCtx(null); } }] : []),
            ...(mine ? [{ label: 'Изменить', onClick: () => { setEditing(true); setCtx(null); } }] : []),
            ...(m.editedAt !== null ? [{ label: 'История', onClick: () => { void loadEdits(m.id).then(setHistory, (e) => onError(errorMessage(e))); setCtx(null); } }] : []),
            ...((mine || canModerate) ? [{ label: 'Удалить', danger: true, onClick: () => { void remove(); setCtx(null); } }] : []),
          ]}
        />
      )}
      <ReactionBar message={m} me={me?.id} onError={onError} />
      {!editing && (
        <div className="absolute -top-3 right-4 hidden gap-1 rounded bg-zinc-900 p-0.5 shadow group-focus-within:flex group-hover:flex">
          <button onClick={() => onReply(m)} className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700">
            Ответить
          </button>
          <button onClick={() => setForwarding(true)} className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700">
            Переслать
          </button>
          <button
            onClick={() => void navigator.clipboard?.writeText(String(m.id))}
            className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700"
            title="Копировать ID сообщения"
          >
            ID
          </button>
          {canPin && (
            <button onClick={togglePin} className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700">
              {m.pinned ? 'Открепить' : 'Закрепить'}
            </button>
          )}
          {mine && (
            <button onClick={() => setEditing(true)} className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700">
              Изменить
            </button>
          )}
          {m.editedAt !== null && (
            <button
              onClick={() => void loadEdits(m.id).then(setHistory, (e) => onError(errorMessage(e)))}
              className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700"
            >
              История
            </button>
          )}
          {(mine || canModerate) && (
            <button onClick={remove} className="rounded px-2 py-0.5 text-sm text-red-400 hover:bg-zinc-700">
              Удалить
            </button>
          )}
        </div>
      )}
        </>
      )}
      {history && (
        <Modal title="История правок" onClose={() => setHistory(null)}>
          {history.length === 0 ? (
            <p className="text-zinc-400">Предыдущих версий нет.</p>
          ) : (
            <ol className="max-h-[60vh] space-y-2 overflow-y-auto">
              {history.map((e) => (
                <li key={e.id} className="rounded bg-zinc-900 p-2 text-sm">
                  <time className="mb-1 block text-xs text-zinc-500" dateTime={new Date(e.editedAt).toISOString()}>
                    {formatTime(e.editedAt)}
                  </time>
                  <div className="whitespace-pre-wrap break-words">
                    <Markdown text={e.content} />
                  </div>
                </li>
              ))}
            </ol>
          )}
        </Modal>
      )}
      {forwarding && (
        <ForwardDialog
          message={m}
          onClose={() => setForwarding(false)}
          onError={onError}
        />
      )}
    </article>
  );
}

const SYSTEM_TEXT: Record<string, string> = {
  welcome: 'приветствует всех на сервере',
  memberJoin: 'присоединился к серверу',
  channelCreate: 'создал канал',
  roleUpdate: 'изменил роль участника',
  wave: 'машет всем 👋',
};

/** Компактная строка системного события в системном канале. */
function SystemMessage({ message: m }: { message: Message }) {
  const text = SYSTEM_TEXT[m.systemKind ?? ''] ?? 'системное событие';
  const detail = m.systemDetail ? ` ${m.systemDetail}` : '';
  return (
    <div className="flex items-center gap-1.5 text-sm text-zinc-400">
      <span aria-hidden>•</span>
      <span className="font-medium text-zinc-300">{m.authorUsername}</span>
      <span>
        {text}
        {detail}
      </span>
      <time className="text-xs text-zinc-600" dateTime={new Date(m.createdAt).toISOString()}>
        {formatTime(m.createdAt)}
      </time>
    </div>
  );
}

function PinsDialog({
  channelId,
  onClose,
  onJump,
  onError,
}: {
  channelId: number;
  onClose: () => void;
  onJump: (id: number) => void;
  onError: (e: string) => void;
}) {
  const [items, setItems] = useState<Message[] | null>(null);

  useEffect(() => {
    loadPins(channelId).then(setItems, (e) => {
      onError(errorMessage(e));
      onClose();
    });
    // Загружаем один раз при открытии.
  }, [channelId]);

  const unpin = async (id: number) => {
    try {
      await setPinned(id, false);
      setItems((list) => list?.filter((m) => m.id !== id) ?? null);
    } catch (e) {
      onError(errorMessage(e));
    }
  };

  return (
    <Modal title={`Закреплённые${items ? ` (${items.length})` : ''}`} onClose={onClose}>
      {items === null ? (
        <p className="text-zinc-400">Загрузка…</p>
      ) : items.length === 0 ? (
        <p className="text-zinc-400">Пока ничего не закреплено.</p>
      ) : (
        <ul className="max-h-[60vh] space-y-2 overflow-y-auto">
          {items.map((m) => (
            <li key={m.id} className="rounded bg-zinc-900 p-3 text-sm">
              <div className="flex items-baseline gap-2 text-xs text-zinc-400">
                <span className="font-medium text-indigo-300">{m.authorUsername}</span>
                <time dateTime={new Date(m.createdAt).toISOString()}>{formatTime(m.createdAt)}</time>
                <button onClick={() => unpin(m.id)} className="ml-auto text-red-400 hover:underline">
                  Открепить
                </button>
                <button onClick={() => onJump(m.id)} className="text-indigo-300 hover:underline">
                  Перейти
                </button>
              </div>
              <button
                type="button"
                onClick={() => {
                  onClose();
                  jumpToMessage(m.id);
                }}
                className="mt-1 block w-full truncate text-left text-zinc-200 hover:underline"
              >
                {m.content || '📎 вложение'}
              </button>
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}

export function jumpToMessage(id: number) {
  const el = document.getElementById(`msg-${id}`);
  if (!el) return;
  el.scrollIntoView({ behavior: 'smooth', block: 'center' });
  el.animate(
    [{ backgroundColor: 'rgba(99,102,241,0.28)' }, { backgroundColor: 'transparent' }],
    { duration: 1200 },
  );
}

function ReplyQuote({ reply }: { reply: NonNullable<Message['replyTo']> }) {
  return (
    <button
      type="button"
      onClick={() => jumpToMessage(reply.id)}
      title="Перейти к сообщению"
      className="mb-0.5 flex max-w-full items-center gap-1 border-l-2 border-zinc-600 pl-2 text-left text-xs text-zinc-400 hover:text-zinc-200"
    >
      <span className="shrink-0 text-indigo-300">↩ {reply.authorUsername}</span>
      <span className="truncate">{reply.content || '📎 вложение'}</span>
    </button>
  );
}

function ForwardQuote({ from }: { from: NonNullable<Message['forwardedFrom']> }) {
  return (
    <div className="mb-1 rounded border-l-2 border-indigo-500/70 bg-indigo-500/5 py-1 pl-2 pr-2 text-sm">
      <div className="text-xs font-semibold text-indigo-300">
        ↪ Переслано от {from.authorUsername}
      </div>
      <div className="max-h-40 overflow-y-auto whitespace-pre-wrap break-words text-zinc-300">
        {from.content ? <Markdown text={from.content} /> : '📎 вложение'}
      </div>
    </div>
  );
}

function EmbedBlock({ messageId }: { messageId: number }) {
  const [embed, setEmbed] = useState<{ url: string; title: string | null; description: string | null; imageUrl: string | null; siteName: string | null } | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    loadEmbed(messageId).then((data) => {
      setEmbed(data ?? null);
    }).catch((e) => setErr(errorMessage(e)));
  }, [messageId]);
  if (err) return null;
  if (!embed) return <div className="mt-1 text-xs text-zinc-500 italic">Загрузка превью...</div>;
  return (
    <a
      href={embed.url}
      target="_blank"
      rel="noopener noreferrer"
      className="mt-1 flex max-w-md gap-3 rounded border border-zinc-700 bg-zinc-800/60 p-3 hover:border-indigo-500/50 hover:bg-zinc-800"
    >
      {embed.imageUrl && (
        <img src={embed.imageUrl} alt="" loading="lazy" className="h-16 w-16 shrink-0 rounded object-cover" />
      )}
      <div className="min-w-0">
        {embed.siteName && (
          <div className="text-[10px] uppercase tracking-wide text-zinc-500">{embed.siteName}</div>
        )}
        {embed.title && (
          <div className="truncate text-sm font-semibold text-indigo-300">{embed.title}</div>
        )}
        {embed.description && (
          <div className="line-clamp-2 text-xs text-zinc-400">{embed.description}</div>
        )}
        <div className="mt-1 truncate text-[10px] text-zinc-500">{embed.url}</div>
      </div>
    </a>
  );
}

/** Выбор канала для пересылки: серверные текст-каналы без текущего, затем ЛС и группы. */
function ForwardDialog({
  message,
  onClose,
  onError,
}: {
  message: Message;
  onClose: () => void;
  onError: (e: string) => void;
}) {
  const servers = useGuilds((s) => s.servers);
  const allChannels = useGuilds((s) => s.channels);
  const dms = useDms((s) => s.dms);
  const [pending, setPending] = useState<number | null>(null);

  const groups: { serverId: number; title: string; channels: { id: number; name: string }[] }[] =
    (servers ?? [])
      .map((srv) => ({
        serverId: srv.id,
        title: srv.name,
        channels: (allChannels[srv.id] ?? [])
          .filter((c) => c.kind === 'text' && c.id !== message.channelId)
          .map((c) => ({ id: c.id, name: c.name })),
      }))
      .filter((g) => g.channels.length > 0);

  const onPick = async (channelId: number) => {
    setPending(channelId);
    try {
      await forwardMessage(message.id, channelId);
      onClose();
    } catch (e) {
      onError(errorMessage(e));
      setPending(null);
    }
  };

  return (
    <Modal title="Переслать сообщение" onClose={onClose}>
      <div className="max-h-[60vh] space-y-3 overflow-y-auto">
        {groups.length === 0 && (!dms || dms.length === 0) && (
          <p className="text-zinc-400">Нет других каналов для пересылки.</p>
        )}
        {groups.map((g) => (
          <section key={g.serverId}>
            <h3 className="mb-1 text-xs font-semibold uppercase text-zinc-400">{g.title}</h3>
            <ul className="space-y-1">
              {g.channels.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    disabled={pending !== null}
                    onClick={() => void onPick(c.id)}
                    className={`w-full rounded px-2 py-1 text-left hover:bg-zinc-700 disabled:opacity-60 ${
                      pending === c.id ? ghostButton : ''
                    }`}
                  >
                    #{c.name}
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ))}
        {dms && dms.length > 0 && (
          <section>
            <h3 className="mb-1 text-xs font-semibold uppercase text-zinc-400">Личные сообщения</h3>
            <ul className="space-y-1">
              {dms
                .filter((d) => d.id !== message.channelId)
                .map((d) => (
                  <li key={d.id}>
                    <button
                      type="button"
                      disabled={pending !== null}
                      onClick={() => void onPick(d.id)}
                      className={`w-full rounded px-2 py-1 text-left hover:bg-zinc-700 disabled:opacity-60 ${
                        pending === d.id ? inputClass : ''
                      }`}
                    >
                      @{dmTitle(d)}
                    </button>
                  </li>
                ))}
            </ul>
          </section>
        )}
      </div>
    </Modal>
  );
}

function ReactionBar({ message, me, onError }: { message: Message; me?: number; onError: (e: string) => void }) {
  const [picker, setPicker] = useState(false);
  const mine = (emoji: string) =>
    me !== undefined && message.reactions.some((r) => r.emoji === emoji && r.userIds.includes(me));

  const toggle = async (emoji: string) => {
    if (!emoji) return;
    try {
      await toggleReaction(message.id, emoji, mine(emoji));
    } catch (e) {
      onError(errorMessage(e));
    }
  };

  return (
    <div className="mt-1 flex flex-wrap items-center gap-1">
      {message.reactions.map((r) => {
        const active = me !== undefined && r.userIds.includes(me);
        return (
          <button
            key={r.emoji}
            type="button"
            onClick={() => toggle(r.emoji)}
            title={active ? 'Снять реакцию' : 'Поставить реакцию'}
            className={`flex items-center gap-1 rounded px-1.5 py-0.5 text-sm ${
              active ? 'bg-indigo-600/40 text-indigo-100' : 'bg-zinc-700/60 text-zinc-200'
            } hover:bg-zinc-600`}
          >
            <span>{r.emoji}</span>
            <span className="text-xs text-zinc-300">{r.userIds.length}</span>
          </button>
        );
      })}
      <div className="relative">
        <button
          type="button"
          onClick={() => setPicker((v) => !v)}
          title="Добавить реакцию"
          aria-label="Добавить реакцию"
          className="rounded px-1.5 py-0.5 text-sm text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
        >
          🙂+
        </button>
        {picker && (
          <EmojiPicker
            className="absolute bottom-full left-0 z-20 mb-1"
            onPick={toggle}
            onClose={() => setPicker(false)}
          />
        )}
      </div>
    </div>
  );
}

function EditBox({ message, onDone, onError }: { message: Message; onDone: () => void; onError: (e: string) => void }) {
  const [value, setValue] = useState(message.content);

  const save = async () => {
    const text = value.trim();
    if (!text || text === message.content) return onDone();
    try {
      await editMessage(message.id, text);
      onDone();
    } catch (e) {
      onError(errorMessage(e));
    }
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Escape') onDone();
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void save();
    }
  };

  return (
    <div className="mt-1">
       <textarea
         autoFocus
         value={value}
         maxLength={MAX_LEN}
         onChange={(e) => setValue(e.target.value)}
         onKeyDown={onKeyDown}
         aria-label="Изменить сообщение"
         className="w-full resize-none rounded bg-zinc-700 px-3 py-2 outline-none"
       />
      <p className="text-xs text-zinc-400">Esc — отмена, Enter — сохранить</p>
    </div>
  );
}

const MAX_FILE_BYTES = 10 * 1024 * 1024;
const MAX_FILES = 10;
const ALLOWED_EXT = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'pdf', 'zip'];

interface Pending {
  key: string;
  file: File;
  progress: number;
  preview?: string;
  attachment?: Attachment;
  error?: string;
}

function Composer({
  channel,
  dropped,
  replyTo,
  onCancelReply,
}: {
  channel: ChatTarget;
  dropped: { files: File[] } | null;
  replyTo: Message | null;
  onCancelReply: () => void;
}) {
const [value, setValue] = useState('');
const [error, setError] = useState<string | null>(null);
const [pending, setPending] = useState<Pending[]>([]);
const [emojiOpen, setEmojiOpen] = useState(false);
const [stickerOpen, setStickerOpen] = useState(false);
  const lastTyping = useRef(0);
  const fileInput = useRef<HTMLInputElement>(null);
  const textInput = useRef<HTMLTextAreaElement>(null);

  const insertEmoji = (emoji: string) => {
    const el = textInput.current;
    const start = el?.selectionStart ?? value.length;
    const end = el?.selectionEnd ?? start;
    setValue(value.slice(0, start) + emoji + value.slice(end));
    setEmojiOpen(false);
    if (Date.now() - lastTyping.current > TYPING_EVERY_MS) {
      lastTyping.current = Date.now();
      sendTyping(channel.id);
    }
    requestAnimationFrame(() => {
      el?.focus();
      if (el) el.selectionStart = el.selectionEnd = start + emoji.length;
    });
  };

    const insertSticker = (sticker: Sticker) => {
    // Add sticker as an attachment in the message
    setPending((prev) => [...prev, { key: `sticker-${Date.now()}`, file: null as any, progress: 100, preview: `/storage/${sticker.storageKey}` }]);
    setStickerOpen(false);
  };

  const update = (key: string, patch: Partial<Pending>) =>
    setPending((list) => list.map((p) => (p.key === key ? { ...p, ...patch } : p)));

  const addFiles = (files: File[]) => {
    setError(null);
    const room = MAX_FILES - pending.length;
    if (files.length > room) setError(ru.tooMany);
    for (const file of files.slice(0, Math.max(0, room))) {
      const key = `${file.name}-${file.size}-${Math.random()}`;
      const ext = file.name.split('.').pop()?.toLowerCase() ?? '';
      // Быстрая проверка в браузере; окончательно тип по содержимому проверяет сервер.
      const precheck =
        file.size > MAX_FILE_BYTES ? ru.tooBig : !ALLOWED_EXT.includes(ext) ? ru.badType : undefined;
      const preview = !precheck && file.type.startsWith('image/') ? URL.createObjectURL(file) : undefined;
      setPending((list) => [...list, { key, file, progress: 0, preview, error: precheck }]);
      if (precheck) continue;
      uploadFile(channel.id, file, (progress) => update(key, { progress })).then(
        (attachment) => update(key, { attachment, progress: 1 }),
        (e) => update(key, { error: errorMessage(e) }),
      );
    }
  };

  useEffect(() => {
    if (dropped) addFiles(dropped.files);
    // Реагируем только на новый сброс файлов, а не на смену addFiles.
  }, [dropped]);

  const remove = (key: string) =>
    setPending((list) => {
      const item = list.find((p) => p.key === key);
      if (item?.preview) URL.revokeObjectURL(item.preview);
      return list.filter((p) => p.key !== key);
    });

  const uploading = pending.some((p) => !p.attachment && !p.error);

  const submit = async () => {
    const text = value.trim();
    const ready = pending.filter((p) => p.attachment);
    if (uploading || (!text && ready.length === 0)) return;
    setError(null);
    try {
      await sendMessage(channel.id, text, ready.map((p) => p.attachment!.id), replyTo?.id);
      setValue('');
      pending.forEach((p) => p.preview && URL.revokeObjectURL(p.preview));
      setPending([]);
      onCancelReply();
    } catch (e) {
      setError(errorMessage(e)); // текст и файлы остаются в поле
    }
  };

    const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Escape' && replyTo) {
        e.preventDefault();
        onCancelReply();
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        void submit();
      }
    };

    // Auto-grow textarea based on scroll height
    useLayoutEffect(() => {
      if (textInput.current) {
        textInput.current.style.height = 'auto';
        textInput.current.style.height = Math.min(textInput.current.scrollHeight, 8 * 20) + 'px'; // max 8 rows
      }
    }, [value]);

  const onChange = (text: string) => {
    setValue(text);
    const now = Date.now();
    if (text && now - lastTyping.current > TYPING_EVERY_MS) {
      lastTyping.current = now;
      sendTyping(channel.id);
    }
  };

  return (
    <div className="px-4 pt-2">
      <div className="rounded-lg bg-zinc-700">
        {replyTo && (
          <div className="flex items-center gap-2 border-b border-zinc-600 px-3 py-1.5 text-xs">
            <span className="shrink-0 text-indigo-300">Ответ {replyTo.authorUsername}</span>
            <span className="min-w-0 flex-1 truncate text-zinc-400">
              {replyTo.content || '📎 вложение'}
            </span>
            <button
              type="button"
              onClick={onCancelReply}
              aria-label="Отменить ответ"
              className="text-zinc-400 hover:text-zinc-100"
            >
              ×
            </button>
          </div>
        )}
        {pending.length > 0 && (
          <ul className="flex flex-wrap gap-2 p-3" aria-label="Вложения к сообщению">
            {pending.map((p) => (
              <li key={p.key} className="relative w-36 rounded bg-zinc-800 p-2 text-xs">
                {p.preview ? (
                  <img src={p.preview} alt="" className="mb-1 h-20 w-full rounded object-cover" />
                ) : (
                  <div className="mb-1 grid h-20 place-items-center text-3xl">{fileIcon(p.file.name)}</div>
                )}
                <div className="truncate" title={p.file.name}>
                  {p.file.name}
                </div>
                {p.error ? (
                  <div role="alert" className="text-red-400">
                    {p.error}
                  </div>
                ) : p.attachment ? (
                  <div className="text-zinc-400">{formatSize(p.file.size)}</div>
                ) : (
                  <progress value={p.progress} max={1} className="w-full" aria-label={`Загрузка ${p.file.name}`} />
                )}
                <button
                  type="button"
                  onClick={() => remove(p.key)}
                  aria-label={`Убрать ${p.file.name}`}
                  className="absolute -top-2 -right-2 grid size-6 place-items-center rounded-full bg-zinc-900 text-zinc-300 hover:text-red-400"
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
        <div className="relative flex items-end">
          <div className="relative">
            <button
              type="button"
              onClick={() => setEmojiOpen((v) => !v)}
              title="Эмодзи"
              aria-label="Эмодзи"
              className="px-2 py-2.5 text-lg text-zinc-400 hover:text-zinc-100"
            >
              🙂
            </button>
            {emojiOpen && (
              <EmojiPicker
                className="absolute bottom-full left-0 z-20 mb-2"
                onPick={insertEmoji}
                onClose={() => setEmojiOpen(false)}
              />
            )}
          </div>
          <button
            type="button"
            onClick={() => setStickerOpen((v) => !v)}
            title="Стикер"
            aria-label="Стикер"
            className="px-2 py-2.5 text-lg text-zinc-400 hover:text-zinc-100"
          >
            📄
          </button>
          {stickerOpen && (
            <div className="absolute bottom-full left-0 z-20 mb-2">
              <StickerPicker
                channelId={channel.id}
                onPick={insertSticker}
                onClose={() => setStickerOpen(false)}
              />
            </div>
          )}
          <button
            type="button"
            onClick={() => fileInput.current?.click()}
            title="Прикрепить файл"
            aria-label="Прикрепить файл"
            className="px-3 py-2.5 text-lg text-zinc-400 hover:text-zinc-100"
          >
            📎
          </button>
          <input
            ref={fileInput}
            type="file"
            multiple
            hidden
            accept={ALLOWED_EXT.map((e) => '.' + e).join(',')}
            onChange={(e) => {
              addFiles(Array.from(e.target.files ?? []));
              e.target.value = '';
            }}
          />
           <textarea
             ref={textInput}
             value={value}
             maxLength={MAX_LEN}
             onChange={(e) => onChange(e.target.value)}
             onKeyDown={onKeyDown}
             onPaste={(e) => {
               const files = Array.from(e.clipboardData.files);
               if (files.length) {
                 e.preventDefault();
                 addFiles(files);
               }
             }}
             placeholder={`Написать ${channel.label}`}
             aria-label={`Сообщение ${channel.label}`}
             className="block w-full resize-none bg-transparent py-2.5 pr-4 outline-none placeholder:text-zinc-400"
           />
        </div>
      </div>
      {uploading && <p className="pt-1 text-xs text-zinc-400">Файлы загружаются, отправка станет доступна после загрузки…</p>}
      {error && (
        <p role="alert" className="pt-1 text-sm text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}

const ru = {
  tooBig: 'Файл больше 10 МБ',
  badType: 'Можно только png, jpg, gif, webp, pdf и zip',
  tooMany: 'Не больше 10 файлов в одном сообщении',
};

function fileIcon(name: string) {
  return name.toLowerCase().endsWith('.pdf') ? '📄' : name.toLowerCase().endsWith('.zip') ? '🗜️' : '🖼️';
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} Б`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} КБ`;
  return `${(bytes / 1024 / 1024).toFixed(1)} МБ`;
}

function Attachments({ files }: { files: Attachment[] }) {
  const [open, setOpen] = useState<Attachment | null>(null);
  if (files.length === 0) return null;
  return (
    <div className="mt-1 flex flex-wrap items-start gap-2">
      {files.map((f) =>
        f.contentType.startsWith('image/') ? (
          <button
            key={f.id}
            type="button"
            onClick={() => setOpen(f)}
            title={f.filename}
            className="cursor-zoom-in rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
          >
            <img src={f.url} alt={f.filename} loading="lazy" className="max-h-80 max-w-full rounded sm:max-w-sm" />
          </button>
        ) : (
          <a
            key={f.id}
            href={f.url}
            download={f.filename}
            className="flex w-72 max-w-full items-center gap-3 rounded border border-zinc-700 bg-zinc-900 p-3 hover:border-zinc-500"
          >
            <span className="text-3xl" aria-hidden>
              {fileIcon(f.filename)}
            </span>
            <span className="min-w-0">
              <span className="block truncate text-indigo-300">{f.filename}</span>
              <span className="text-xs text-zinc-400">{formatSize(f.sizeBytes)}</span>
            </span>
          </a>
        ),
      )}
      {open && (
        <Modal title={open.filename} onClose={() => setOpen(null)} wide>
          <img
            src={open.url}
            alt={open.filename}
            className="mx-auto max-h-[70vh] max-w-full rounded object-contain"
          />
          <div className="flex items-center justify-between gap-4 text-sm text-zinc-400">
            <span>{formatSize(open.sizeBytes)}</span>
            <a href={open.url} download={open.filename} className="text-indigo-300 hover:underline">
              Скачать
            </a>
          </div>
        </Modal>
      )}
    </div>
  );
}

function TypingIndicator({ channelId }: { channelId: number }) {
  const typing = useMessages((s) => s.typing[channelId]);
  const me = useSession((s) => s.user);
  const [now, setNow] = useState(Date.now());

  // Записи «печатает» истекают по времени: перерисовываемся раз в секунду, пока они есть.
  useEffect(() => {
    if (!typing) return;
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, [typing]);

  const names = Object.entries(typing ?? {})
    .filter(([id, t]) => Number(id) !== me?.id && t.until > now)
    .map(([, t]) => t.username);

  const text =
    names.length === 0
      ? ''
      : names.length === 1
        ? `${names[0]} печатает…`
        : names.length <= 3
          ? `${names.join(', ')} печатают…`
          : 'Несколько человек печатают…';

  return (
    <p aria-live="polite" className="h-6 truncate px-4 text-xs text-zinc-400">
      {text}
    </p>
  );
}

interface SearchBarProps {
  visible: boolean;
  query: string;
  onToggle: () => void;
  onQueryChange: (q: string) => void;
  loading: boolean;
  error: string | null;
  results: SearchHit[];
  hasMore: boolean;
  onJump: (id: number) => void;
}

function SearchBar({
  visible,
  query,
  onToggle,
  onQueryChange,
  loading,
  error,
  results,
  hasMore,
  onJump,
}: SearchBarProps) {
  if (!visible) return null;

  return (
    <div className="absolute inset-0 z-5 flex flex-col bg-zinc-900/95 backdrop-blur">
      <div className="flex items-center gap-2 px-4 py-2">
        <input
          autoFocus
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') onToggle();
            if (e.key === 'Enter' && results[0]) onJump(results[0].message.id);
          }}
          placeholder="Поиск: from:user has:link ключевое слово"
          className="w-full rounded bg-zinc-800 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-500"
        />
        <button
          type="button"
          onClick={onToggle}
          className="rounded px-3 py-2 text-sm text-zinc-400 hover:bg-zinc-700"
          aria-label="Закрыть поиск"
        >
          Закрыть
        </button>
      </div>
      {error && (
        <p role="alert" className="px-4 py-2 text-sm text-red-400">
          {error}
        </p>
      )}
      {loading && (
        <p className="px-4 py-2 text-sm text-zinc-400">Поиск…</p>
      )}
      {!loading && !error && results.length === 0 && query && (
        <p className="px-4 py-2 text-sm text-zinc-400">Нет сообщений по запросу.</p>
      )}
      {results.length > 0 && (
        <div className="flex-1 overflow-y-auto px-2 pb-4">
          <ul className="space-y-1">
            {results.map((r) => (
              <li key={r.message.id}>
                <button
                  type="button"
                  onClick={() => onJump(r.message.id)}
                  className="w-full rounded px-3 py-2 text-left hover:bg-zinc-800"
                >
                  <div className="flex items-center gap-2 text-xs text-zinc-500">
                    <span className="font-medium text-zinc-300">{r.message.authorUsername}</span>
                    <span>#{r.channelLabel}</span>
                    <span>{new Date(r.message.createdAt).toLocaleString('ru-RU')}</span>
                  </div>
                  <div className="mt-0.5 truncate text-sm text-zinc-300">
                    <Markdown text={r.message.content} />
                  </div>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}


function ForumThreads({ channel, onError }: { channel: ChatTarget; onError: (e: string) => void }) {
  const [items, setItems] = useState<Message[]>([]);
  const [text, setText] = useState("");
  const [open, setOpen] = useState<number | null>(null);
  const [replies, setReplies] = useState<Map<number, Message[]>>(new Map());
  const reload = () => listThreads(channel.id).then(setItems, (e) => onError(errorMessage(e)));

  useEffect(() => { void reload(); }, [channel.id]);

  const toggleThread = async (id: number) => {
    const next = open === id ? null : id;
    setOpen(next);
    if (next !== null && !replies.has(id)) {
      try {
        const list = await loadReplies(id);
        setReplies((m) => new Map(m).set(id, list));
      } catch (err) { onError(errorMessage(err)); }
    }
  };

  const create = async (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim()) return;
    try {
      await createThread(channel.id, text);
      setText("");
      await reload();
    } catch (err) { onError(errorMessage(err)); }
  };

  const change = async (id: number, key: "archived" | "locked", value: boolean) => {
    try {
      await updateThread(id, { [key]: value });
      await reload();
    } catch (err) { onError(errorMessage(err)); }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex-1 overflow-y-auto px-4 py-2">
        {items.length === 0 && <p className="text-zinc-400">Пока нет веток. Создайте первую.</p>}
        <ul className="space-y-2">
          {items.map((m) => (
            <li key={m.id} className="rounded border border-zinc-700 bg-zinc-900 p-3">
              <div className="flex items-center gap-2 text-xs text-zinc-500">
                <span className="font-medium text-zinc-300">{m.authorUsername}</span>
                <span>{new Date(m.createdAt).toLocaleString("ru-RU")}</span>
                {m.threadArchived && <span className="rounded bg-zinc-700 px-1">архив</span>}
                {m.threadLocked && <span className="rounded bg-zinc-700 px-1">закрыта</span>}
                <span className="text-zinc-600">· {replies.get(m.id)?.length ?? '…'} отв.</span>
              </div>
              <button onClick={() => void toggleThread(m.id)} className="mt-1 block w-full text-left text-sm">
                <Markdown text={m.content} />
              </button>
              {open === m.id && (
                <>
                  <div className="mt-2 border-t border-zinc-700 pt-2">
                    {replies.has(m.id) && replies.get(m.id)!.length === 0
                      ? <p className="text-xs text-zinc-500">Пока нет ответов.</p>
                      : replies.get(m.id)?.map((r) => (
                        <div key={r.id} className="mb-1 rounded bg-zinc-800 px-2 py-1 text-xs">
                          <span className="font-medium text-zinc-300">{r.authorUsername}</span>
                          {' — '}<Markdown text={r.content} />
                        </div>
                      ))}
                  </div>
                  <div className="mt-2 flex gap-2 border-t border-zinc-700 pt-2 text-xs">
                    <button onClick={() => void change(m.id, "archived", !m.threadArchived)} className="text-indigo-400">
                      {m.threadArchived ? "Разархивировать" : "В архив"}
                    </button>
                    <button onClick={() => void change(m.id, "locked", !m.threadLocked)} className="text-indigo-400">
                      {m.threadLocked ? "Открыть" : "Закрыть"}
                    </button>
                  </div>
                </>
              )}
            </li>
          ))}
        </ul>
      </div>
      <form onSubmit={create} className="flex gap-2 border-t border-zinc-700 p-3">
        <input value={text} onChange={(e) => setText(e.target.value)} placeholder={`Новая ветка в ${channel.label}`} className={inputClass} />
        <button className={primaryButton} disabled={!text.trim()}>Создать</button>
      </form>
    </div>
  );
}