import { create } from 'zustand';
import { api } from './api';
import type { DmChannel } from './generated/DmChannel';
import { rememberStatuses } from './presence';

/** `null` — ещё не загружено. Свежие диалоги сверху. */
export const useDms = create<{ dms: DmChannel[] | null }>(() => ({ dms: null }));

export async function loadDms() {
  const dms = await api<DmChannel[]>('/api/dms');
  rememberStatuses(dms.map((d) => ({ userId: d.userId, username: d.username, role: 'member', roleIds: [], status: d.status })));
  useDms.setState({ dms });
}

/** Название диалога: у группы — имя или список участников, у ЛС — собеседник. */
export const dmTitle = (d: DmChannel) =>
  d.isGroup ? d.name || d.members.map((m) => m.username).join(', ') : d.username;

export async function createGroup(usernames: string[], name: string) {
  const dm = await api<DmChannel>('/api/dms/group', { method: 'POST', body: { usernames, name } });
  useDms.setState((s) => ({ dms: [dm, ...(s.dms ?? []).filter((d) => d.id !== dm.id)] }));
  return dm;
}

export async function openDm(username: string) {
  const dm = await api<DmChannel>('/api/dms', { method: 'POST', body: { username } });
  useDms.setState((s) => ({ dms: [dm, ...(s.dms ?? []).filter((d) => d.id !== dm.id)] }));
  return dm;
}

export const isDm = (channelId: number) => !!useDms.getState().dms?.some((d) => d.id === channelId);

let reloadTimer: number | undefined;
/** Сообщение в диалоге, которого ещё нет в списке (кто-то написал впервые). */
export function reloadDmsSoon() {
  window.clearTimeout(reloadTimer);
  reloadTimer = window.setTimeout(() => void loadDms().catch(() => {}), 500);
}

/** Новое сообщение в диалоге: поднимаем его наверх, чужое ещё и считаем непрочитанным. */
export function bumpDm(channelId: number, unread: boolean) {
  useDms.setState((s) => {
    const dm = s.dms?.find((d) => d.id === channelId);
    if (!dm) return {};
    const updated = { ...dm, unreadCount: unread ? Math.min(100, dm.unreadCount + 1) : dm.unreadCount };
    return { dms: [updated, ...s.dms!.filter((d) => d.id !== channelId)] };
  });
}

export function clearDmUnread(channelId: number) {
  useDms.setState((s) => ({
    dms: s.dms?.map((d) => (d.id === channelId && d.unreadCount ? { ...d, unreadCount: 0 } : d)) ?? null,
  }));
}
