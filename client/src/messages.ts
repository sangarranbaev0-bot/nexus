import { create } from 'zustand';
import { api } from './api';
import type { Message } from './generated/Message';
import type { MessageEdit } from './generated/MessageEdit';

const PAGE = 50;
export const TYPING_TTL_MS = 6000;

interface ChannelLog {
  items: Message[];
  hasMore: boolean;
  loading: boolean;
}

interface MessagesState {
  /** Лента только тех каналов, которые открывали; остальные события игнорируем. */
  logs: Record<number, ChannelLog | undefined>;
  /** channelId → userId → кто печатает и до какого момента показывать. */
  typing: Record<number, Record<number, { username: string; until: number }> | undefined>;
}

export const useMessages = create<MessagesState>(() => ({ logs: {}, typing: {} }));

const setLog = (channelId: number, update: (log: ChannelLog) => ChannelLog) =>
  useMessages.setState((s) => {
    const log = s.logs[channelId];
    return log ? { logs: { ...s.logs, [channelId]: update(log) } } : {};
  });

const page = (channelId: number, before?: number) =>
  api<Message[]>(`/api/channels/${channelId}/messages?limit=${PAGE}${before ? `&before=${before}` : ''}`);

export async function loadInitial(channelId: number) {
  if (useMessages.getState().logs[channelId]) return;
  useMessages.setState((s) => ({
    logs: { ...s.logs, [channelId]: { items: [], hasMore: false, loading: true } },
  }));
  try {
    const items = await page(channelId);
    setLog(channelId, () => ({ items, hasMore: items.length === PAGE, loading: false }));
  } catch (e) {
    // Убираем заготовку, чтобы при следующем открытии канала попробовать снова.
    useMessages.setState((s) => ({ logs: { ...s.logs, [channelId]: undefined } }));
    throw e;
  }
}

export async function loadOlder(channelId: number) {
  const log = useMessages.getState().logs[channelId];
  if (!log || log.loading || !log.hasMore || log.items.length === 0) return;
  setLog(channelId, (l) => ({ ...l, loading: true }));
  try {
    const older = await page(channelId, log.items[0].id);
    setLog(channelId, (l) => ({
      items: [...older, ...l.items],
      hasMore: older.length === PAGE,
      loading: false,
    }));
  } catch (e) {
    setLog(channelId, (l) => ({ ...l, loading: false }));
    throw e;
  }
}

export function applyCreated(m: Message) {
  // Своё сообщение приходит дважды (ответ REST и событие): не дублируем.
  setLog(m.channelId, (l) =>
    l.items.some((x) => x.id === m.id) ? l : { ...l, items: [...l.items, m] },
  );
  clearTyping(m.channelId, m.authorId);
}

export function applyUpdated(m: Message) {
  setLog(m.channelId, (l) => ({ ...l, items: l.items.map((x) => (x.id === m.id ? m : x)) }));
}

export function applyDeleted(channelId: number, messageId: number) {
  setLog(channelId, (l) => ({ ...l, items: l.items.filter((x) => x.id !== messageId) }));
}

export function applyTyping(channelId: number, userId: number, username: string) {
  useMessages.setState((s) => ({
    typing: {
      ...s.typing,
      [channelId]: { ...s.typing[channelId], [userId]: { username, until: Date.now() + TYPING_TTL_MS } },
    },
  }));
}

function clearTyping(channelId: number, userId: number) {
  useMessages.setState((s) => {
    if (!s.typing[channelId]?.[userId]) return {};
    const { [userId]: _, ...rest } = s.typing[channelId]!;
    return { typing: { ...s.typing, [channelId]: rest } };
  });
}

/** После переподключения сокета ленты могли устареть: перечитываем их заново. */
export function resetMessages() {
  useMessages.setState({ logs: {}, typing: {} });
}

export async function sendMessage(
  channelId: number,
  content: string,
  attachmentIds: number[] = [],
  replyToId?: number,
) {
  applyCreated(
    await api<Message>(`/api/channels/${channelId}/messages`, {
      method: 'POST',
      body: { content, attachmentIds, replyToId },
    }),
  );
}

export async function editMessage(id: number, content: string) {
  applyUpdated(await api<Message>(`/api/messages/${id}`, { method: 'PATCH', body: { content } }));
}

export async function deleteMessage(m: Message) {
  await api(`/api/messages/${m.id}`, { method: 'DELETE' });
  applyDeleted(m.channelId, m.id);
}

/** Поставить или снять реакцию. `mine` — стоит ли она уже у текущего пользователя. */
export async function toggleReaction(id: number, emoji: string, mine: boolean) {
  applyUpdated(
    await api<Message>(`/api/messages/${id}/reactions/${encodeURIComponent(emoji)}`, {
      method: mine ? 'DELETE' : 'PUT',
    }),
  );
}

export async function setPinned(id: number, pinned: boolean) {
  applyUpdated(await api<Message>(`/api/messages/${id}/pin`, { method: pinned ? 'PUT' : 'DELETE' }));
}

export async function loadPins(channelId: number) {
  return api<Message[]>(`/api/channels/${channelId}/pins`);
}

/** Предыдущие версии сообщения, свежие сверху. */
export async function loadEdits(messageId: number) {
  return api<MessageEdit[]>(`/api/messages/${messageId}/edits`);
}

/** Пересылка в другой канал: копия со снимком автора и текста. */
export async function forwardMessage(messageId: number, channelId: number) {
  return api<Message>(`/api/messages/${messageId}/forward`, {
    method: 'POST',
    body: { channelId },
  });
}

// --- Прочтение -------------------------------------------------------------

/** Что сейчас на экране: канал и прокручен ли он до низа. Ставит ChatView. */
export const activeView: { channelId?: number; atBottom: boolean } = { atBottom: true };

/** Видит ли пользователь новое сообщение в этом канале прямо сейчас. */
export const isViewing = (channelId: number) =>
  activeView.channelId === channelId && activeView.atBottom && document.visibilityState === 'visible';

const acked: Record<number, number> = {};
const ackTimers: Record<number, number> = {};

/** Отмечает канал прочитанным до `messageId`. Запросы склеиваются за 500 мс. */
export function markRead(channelId: number, messageId: number) {
  if ((acked[channelId] ?? 0) >= messageId) return;
  acked[channelId] = messageId;
  window.clearTimeout(ackTimers[channelId]);
  ackTimers[channelId] = window.setTimeout(() => {
    api(`/api/channels/${channelId}/read`, { method: 'POST', body: { messageId } }).catch(() => {
      delete acked[channelId]; // повторим при следующем случае
    });
  }, 500);
}
