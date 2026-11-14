import { create } from 'zustand';
import { api } from './api';
import type { Notification } from './generated/Notification';
import type { NotifyLevel } from './generated/NotifyLevel';
import type { NotifySetting } from './generated/NotifySetting';
import type { NotifyWord } from './generated/NotifyWord';
import { isDm } from './dms';
import { serverOfChannel } from './guilds';

/** Ключ настройки в сторе: `s:<id>` для сервера, `c:<id>` для канала. */
const serverKey = (id: number) => `s:${id}`;
const channelKey = (id: number) => `c:${id}`;

interface NotifyState {
  /** `null` — ещё не загружено. */
  settings: Record<string, NotifySetting> | null;
  /** Центр уведомлений: свежие сверху, `null` — не загружено. */
  center: Notification[] | null;
  /** Слова-триггеры текущего пользователя. */
  words: NotifyWord[] | null;
}

export const useNotify = create<NotifyState>(() => ({ settings: null, center: null, words: null }));

export async function loadNotifySettings() {
  const list = await api<NotifySetting[]>('/api/me/notify-settings');
  const settings: Record<string, NotifySetting> = {};
  for (const item of list) {
    if (item.serverId !== null) settings[serverKey(item.serverId)] = item;
    else if (item.channelId !== null) settings[channelKey(item.channelId)] = item;
  }
  useNotify.setState({ settings });
}

/** Уровень для сервера; по умолчанию — «все». */
export function serverLevel(serverId: number): NotifyLevel {
  return useNotify.getState().settings?.[serverKey(serverId)]?.level ?? 'all';
}

/** Глушит ли сервер @everyone. */
export function serverSuppressEveryone(serverId: number): boolean {
  return useNotify.getState().settings?.[serverKey(serverId)]?.suppressEveryone ?? false;
}

/** Уровень для канала с учётом наследования от сервера. */
export function channelLevel(channelId: number): NotifyLevel {
  const state = useNotify.getState();
  const own = state.settings?.[channelKey(channelId)];
  if (own) return own.level;
  const serverId = serverOfChannel(channelId);
  return serverId !== undefined ? serverLevel(serverId) : 'all';
}

/** Включён ли режим «показывать все» с учётом mute. */
export function isMuted(setting: NotifySetting | undefined): boolean {
  return !!setting?.mutedUntil && setting.mutedUntil > Date.now();
}

function applySetting(setting: NotifySetting) {
  useNotify.setState((s) => ({
    settings: {
      ...s.settings,
      [setting.serverId !== null ? serverKey(setting.serverId) : channelKey(setting.channelId!)]:
        setting,
    },
  }));
}

export async function setServerNotify(
  serverId: number,
  level: NotifyLevel,
  mutedUntil: number | null,
  suppressEveryone: boolean,
) {
  const body: NotifySetting = {
    serverId,
    channelId: null,
    level,
    mutedUntil,
    suppressEveryone,
  };
  await api('/api/me/notify-settings', { method: 'PUT', body });
  applySetting(body);
}

export async function setChannelNotify(
  channelId: number,
  level: NotifyLevel,
  mutedUntil: number | null,
  suppressEveryone: boolean,
) {
  const body: NotifySetting = {
    serverId: null,
    channelId,
    level,
    mutedUntil,
    suppressEveryone,
  };
  await api('/api/me/notify-settings', { method: 'PUT', body });
  applySetting(body);
}

/** Молчать ли при новом сообщении в канале (учитывает уровень и mute). */
export function shouldNotify(channelId: number, mentionsMe: boolean): boolean {
  const state = useNotify.getState().settings;
  if (!state) return true;
  if (isMuted(state[channelKey(channelId)])) return false;
  if (!isDm(channelId)) {
    const serverId = serverOfChannel(channelId);
    if (serverId !== undefined && isMuted(state[serverKey(serverId)])) return false;
  }
  const level = channelLevel(channelId);
  if (level === 'none') return false;
  if (level === 'mentions' && !mentionsMe) return false;
  return true;
}

// ---- Центр уведомлений ----------------------------------------------------

export async function loadNotifications() {
  const list = await api<Notification[]>('/api/me/notifications');
  useNotify.setState({ center: list });
}

/** Новое уведомление из сокета: ставим сверху. */
export function pushNotification(n: Notification) {
  useNotify.setState((s) => ({
    center: [n, ...(s.center ?? []).filter((x) => x.id !== n.id)],
  }));
}

export const unreadCount = (list: Notification[] | null) =>
  (list ?? []).filter((n) => !n.read).length;

export async function markNotificationRead(id?: number) {
  await api('/api/me/notifications/read', { method: 'POST', body: { id: id ?? null } });
  useNotify.setState((s) => ({
    center: id
      ? (s.center?.map((n) => (n.id === id ? { ...n, read: true } : n)) ?? null)
      : (s.center?.map((n) => ({ ...n, read: true })) ?? null),
  }));
}

// ---- Слова-триггеры -------------------------------------------------------

export async function loadNotifyWords() {
  useNotify.setState({ words: await api<NotifyWord[]>('/api/me/notification-words') });
}

export async function addNotifyWord(word: string, caseSensitive: boolean) {
  await api('/api/me/notification-words', {
    method: 'POST',
    body: { word, caseSensitive },
  });
  await loadNotifyWords();
}

export async function removeNotifyWord(word: string) {
  await api(`/api/me/notification-words/${encodeURIComponent(word)}`, { method: 'DELETE' });
  useNotify.setState((s) => ({ words: s.words?.filter((w) => w.word !== word) ?? null }));
}
