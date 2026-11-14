import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';
import type { Attachment } from './generated/Attachment';
import type { Message } from './generated/Message';
import { uploadFile } from './api';
import {
  activeView,
  deleteMessage,
  editMessage,
  isViewing,
  loadInitial,
  loadOlder,
  loadPins,
  markRead,
  sendMessage,
  setPinned,
  toggleReaction,
  useMessages,
} from './messages';
import { Modal, errorMessage } from './Modal';
import { useSession } from './session';
import { sendTyping } from './socket';

const MAX_LEN = 2000;
const TYPING_EVERY_MS = 3000;
const NEAR_BOTTOM_PX = 120;
const LOAD_OLDER_PX = 200;

/** Что показываем: id канала и подпись вида «#общий» или «@bob». */
export interface ChatTarget {
  id: number;
  label: string;
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

  useEffect(() => {
    setError(null);
    loadInitial(channel.id).catch((e) => setError(errorMessage(e)));
  }, [channel.id, log === undefined]);

  const [dropped, setDropped] = useState<{ files: File[] } | null>(null);
  const [dragging, setDragging] = useState(false);
  const [replyTo, setReplyTo] = useState<Message | null>(null);

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
      <MessageList
        channel={channel}
        canModerate={canModerate}
        canPin={canPin}
        onRead={onRead}
        items={log?.items}
        hasMore={!!log?.hasMore}
        onError={setError}
        onReply={setReplyTo}
      />
      <button
        type="button"
        onClick={() => setShowPins(true)}
        title="Закреплённые сообщения"
        className="absolute top-1 right-4 z-10 rounded bg-zinc-900/80 px-2 py-1 text-xs text-zinc-300 backdrop-blur hover:bg-zinc-800"
      >
        📌 Закреплённые
      </button>
      {showPins && (
        <PinsDialog channelId={channel.id} onClose={() => setShowPins(false)} onError={setError} />
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
  onError,
  onReply,
}: {
  channel: ChatTarget;
  canModerate: boolean;
  canPin: boolean;
  onRead: () => void;
  items: Message[] | undefined;
  hasMore: boolean;
  onError: (e: string) => void;
  onReply: (m: Message) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const nearBottom = useRef(true);
  const prev = useRef<{ first?: number; last?: number; height: number }>({ height: 0 });
  const latest = useRef(items);
  latest.current = items;

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
    const el = ref.current;
    if (!el || !items) return;
    const first = items[0]?.id;
    const last = items.at(-1)?.id;
    const p = prev.current;
    if (p.first !== undefined && first !== p.first && last === p.last) {
      el.scrollTop += el.scrollHeight - p.height;
    } else if (last !== p.last && (nearBottom.current || p.last === undefined)) {
      el.scrollTop = el.scrollHeight;
    }
    prev.current = { first, last, height: el.scrollHeight };
    ackIfViewing();
  }, [items]);

  // Картинки получают высоту только после загрузки: если были внизу, остаёмся внизу.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onLoad = (e: Event) => {
      if (e.target instanceof HTMLImageElement && nearBottom.current) el.scrollTop = el.scrollHeight;
    };
    el.addEventListener('load', onLoad, true); // load не всплывает, ловим на погружении
    return () => el.removeEventListener('load', onLoad, true);
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

  const onScroll = () => {
    const el = ref.current!;
    nearBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < NEAR_BOTTOM_PX;
    prev.current.height = el.scrollHeight;
    ackIfViewing();
    if (el.scrollTop < LOAD_OLDER_PX && hasMore) loadOlder(channel.id).catch((e) => onError(errorMessage(e)));
  };

  return (
    <div ref={ref} onScroll={onScroll} className="min-h-0 flex-1 overflow-y-auto py-4" role="log" aria-label={`Сообщения ${channel.label}`}>
      {items && !hasMore && (
        <div className="px-4 pb-4">
          <h3 className="text-2xl font-bold">Добро пожаловать в {channel.label}!</h3>
          <p className="text-zinc-400">
            {channel.label.startsWith("@") ? "Это начало вашей переписки." : "Это начало канала."}
          </p>
        </div>
      )}
      {items === undefined && <p className="px-4 text-zinc-400">Загрузка…</p>}
      {items?.map((m) => (
        <MessageItem
          key={m.id}
          message={m}
          canModerate={canModerate}
          canPin={canPin}
          onError={onError}
          onReply={onReply}
        />
      ))}
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
    <article id={`msg-${m.id}`} tabIndex={0} className="group relative px-4 py-1 hover:bg-zinc-900/30 focus:bg-zinc-900/30">
      <header className="flex items-baseline gap-2">
        <span className="font-medium text-indigo-300">{m.authorUsername}</span>
        <time className="text-xs text-zinc-500" dateTime={new Date(m.createdAt).toISOString()}>
          {formatTime(m.createdAt)}
        </time>
        {m.pinned && (
          <span title="Закреплено" className="text-xs text-amber-300">
            📌
          </span>
        )}
      </header>
      {m.replyTo && <ReplyQuote reply={m.replyTo} />}
      {editing ? (
        <EditBox message={m} onDone={() => setEditing(false)} onError={onError} />
      ) : (
        (m.content || m.editedAt !== null) && (
          <p className="break-words whitespace-pre-wrap text-zinc-100">
            {m.content}
            {m.editedAt !== null && <span className="ml-1 text-xs text-zinc-500">(изменено)</span>}
          </p>
        )
      )}
      <Attachments files={m.attachments} />
      <ReactionBar message={m} me={me?.id} onError={onError} />
      {!editing && (
        <div className="absolute -top-3 right-4 hidden gap-1 rounded bg-zinc-900 p-0.5 shadow group-focus-within:flex group-hover:flex">
          <button onClick={() => onReply(m)} className="rounded px-2 py-0.5 text-sm text-zinc-300 hover:bg-zinc-700">
            Ответить
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
          {(mine || canModerate) && (
            <button onClick={remove} className="rounded px-2 py-0.5 text-sm text-red-400 hover:bg-zinc-700">
              Удалить
            </button>
          )}
        </div>
      )}
    </article>
  );
}

function PinsDialog({
  channelId,
  onClose,
  onError,
}: {
  channelId: number;
  onClose: () => void;
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

function jumpToMessage(id: number) {
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

const QUICK_REACTIONS = ['👍', '😂', '❤️', '🔥', '👀', '✅'];

function ReactionBar({ message, me, onError }: { message: Message; me?: number; onError: (e: string) => void }) {
  const [picker, setPicker] = useState(false);
  const [custom, setCustom] = useState('');
  const mine = (emoji: string) =>
    me !== undefined && message.reactions.some((r) => r.emoji === emoji && r.userIds.includes(me));

  const toggle = async (emoji: string) => {
    if (!emoji) return;
    try {
      await toggleReaction(message.id, emoji, mine(emoji));
      setCustom('');
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
        <div className="flex flex-wrap items-center gap-1 rounded bg-zinc-800 p-1 shadow">
          {QUICK_REACTIONS.map((e) => (
            <button
              key={e}
              type="button"
              onClick={() => toggle(e)}
              className="rounded px-1 text-lg hover:bg-zinc-700"
            >
              {e}
            </button>
          ))}
          <input
            value={custom}
            maxLength={16}
            placeholder="эмодзи"
            aria-label="Своя реакция"
            onChange={(e) => setCustom(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && custom.trim()) {
                e.preventDefault();
                void toggle(custom.trim());
              }
            }}
            className="w-24 rounded bg-zinc-700 px-2 py-0.5 text-sm outline-none placeholder:text-zinc-500"
          />
        </div>
      )}
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
        rows={Math.min(8, value.split('\n').length)}
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
  const lastTyping = useRef(0);
  const fileInput = useRef<HTMLInputElement>(null);

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
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void submit();
    }
  };

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
        <div className="flex items-end">
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
            rows={Math.min(8, value.split('\n').length)}
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
  if (files.length === 0) return null;
  return (
    <div className="mt-1 flex flex-wrap items-start gap-2">
      {files.map((f) =>
        f.contentType.startsWith('image/') ? (
          <a key={f.id} href={f.url} target="_blank" rel="noreferrer" title={f.filename}>
            <img src={f.url} alt={f.filename} loading="lazy" className="max-h-80 max-w-full rounded sm:max-w-sm" />
          </a>
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
