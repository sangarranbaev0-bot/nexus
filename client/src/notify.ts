import { create } from 'zustand';
import { api } from './api';
import type { NotifyLevel } from './generated/NotifyLevel';
import type { NotifySetting } from './generated/NotifySetting';
import { isDm } from './dms';
import { serverOfChannel } from './guilds';

/** Ключ настройки в сторе: `s:<id>` для сервера, `c:<id>` для канала. */
const serverKey = (id: number) => `s:${id}`;
const channelKey = (id: number) => `c:${id}`;

interface NotifyState {
  /** `null` — ещё не загружено. */
  settings: Record<string, NotifySetting> | null;
}

export const useNotify = create<NotifyState>(() => ({ settings: null }));

export async function loadNotifySettings() {
  const list = await api<NotifySetting[]>('/api/me/notifications');
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

export async function setServerNotify(
  serverId: number,
  level: NotifyLevel,
  mutedUntil: number | null,
) {
  const body: NotifySetting = { serverId, channelId: null, level, mutedUntil };
  await api('/api/me/notifications', { method: 'PUT', body });
  useNotify.setState((s) => ({ settings: { ...s.settings, [serverKey(serverId)]: body } }));
}

export async function setChannelNotify(
  channelId: number,
  level: NotifyLevel,
  mutedUntil: number | null,
) {
  const body: NotifySetting = { serverId: null, channelId, level, mutedUntil };
  await api('/api/me/notifications', { method: 'PUT', body });
  useNotify.setState((s) => ({ settings: { ...s.settings, [channelKey(channelId)]: body } }));
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
