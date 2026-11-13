import { create } from 'zustand';
import { api } from './api';
import type { Channel } from './generated/Channel';
import type { ChannelKind } from './generated/ChannelKind';
import type { CreateInviteRequest } from './generated/CreateInviteRequest';
import type { Invite } from './generated/Invite';
import type { Member } from './generated/Member';
import type { Role } from './generated/Role';
import type { Server } from './generated/Server';

interface GuildsState {
  /** `null` — ещё не загружено. */
  servers: Server[] | null;
  channels: Record<number, Channel[] | undefined>;
}

export const useGuilds = create<GuildsState>(() => ({ servers: null, channels: {} }));

const rank: Record<Role, number> = { member: 0, admin: 1, owner: 2 };
export const canManage = (server: Server | undefined) => !!server && rank[server.myRole] >= rank.admin;

export async function loadServers() {
  useGuilds.setState({ servers: await api<Server[]>('/api/servers') });
}

export async function loadChannels(serverId: number) {
  const list = await api<Channel[]>(`/api/servers/${serverId}/channels`);
  useGuilds.setState((s) => ({ channels: { ...s.channels, [serverId]: list } }));
  return list;
}

const addServer = (server: Server) =>
  useGuilds.setState((s) => ({
    servers: [...(s.servers ?? []).filter((x) => x.id !== server.id), server],
  }));

export async function createServer(name: string) {
  const server = await api<Server>('/api/servers', { method: 'POST', body: { name } });
  addServer(server);
  return server;
}

/** Принимает код или ссылку целиком: берём последний сегмент. */
export async function joinInvite(codeOrLink: string) {
  const code = codeOrLink.trim().split('/').pop() ?? '';
  const server = await api<Server>(`/api/invites/${encodeURIComponent(code)}/join`, { method: 'POST' });
  addServer(server);
  return server;
}

export async function createChannel(serverId: number, name: string, kind: ChannelKind = 'text') {
  const channel = await api<Channel>(`/api/servers/${serverId}/channels`, {
    method: 'POST',
    body: { name, kind },
  });
  await loadChannels(serverId);
  return channel;
}

export async function deleteChannel(channel: Channel) {
  await api(`/api/channels/${channel.id}`, { method: 'DELETE' });
  await loadChannels(channel.serverId);
}

export const createInvite = (serverId: number, req: CreateInviteRequest = {}) =>
  api<Invite>(`/api/servers/${serverId}/invites`, { method: 'POST', body: req });

export const listMembers = (serverId: number) => api<Member[]>(`/api/servers/${serverId}/members`);

export const setMemberRole = (serverId: number, userId: number, role: Role) =>
  api(`/api/servers/${serverId}/members/${userId}`, { method: 'PATCH', body: { role } });

/** Найти сервер канала среди загруженных списков каналов. */
export function serverOfChannel(channelId: number): number | undefined {
  for (const [serverId, list] of Object.entries(useGuilds.getState().channels)) {
    if (list?.some((c) => c.id === channelId)) return Number(serverId);
  }
  return undefined;
}

// Канал из сервера, список каналов которого не загружен: перечитываем список серверов,
// но не чаще раза в секунду.
let reloadTimer: number | undefined;
function reloadServersSoon() {
  window.clearTimeout(reloadTimer);
  reloadTimer = window.setTimeout(() => void loadServers().catch(() => {}), 1000);
}

const UNREAD_CAP = 100;

/** Новое чужое сообщение в непросматриваемом канале. */
export function bumpUnread(channelId: number) {
  const serverId = serverOfChannel(channelId);
  if (serverId === undefined) return reloadServersSoon();
  useGuilds.setState((s) => ({
    channels: {
      ...s.channels,
      [serverId]: s.channels[serverId]?.map((c) =>
        c.id === channelId ? { ...c, unreadCount: Math.min(UNREAD_CAP, c.unreadCount + 1) } : c,
      ),
    },
    servers: s.servers?.map((x) => (x.id === serverId ? { ...x, unreadCount: x.unreadCount + 1 } : x)) ?? null,
  }));
}

/** Канал прочитан (здесь или в другой вкладке). */
export function clearUnread(serverId: number, channelId: number) {
  const channel = useGuilds.getState().channels[serverId]?.find((c) => c.id === channelId);
  if (!channel) return reloadServersSoon();
  if (channel.unreadCount === 0) return;
  useGuilds.setState((s) => ({
    channels: {
      ...s.channels,
      [serverId]: s.channels[serverId]?.map((c) => (c.id === channelId ? { ...c, unreadCount: 0 } : c)),
    },
    servers:
      s.servers?.map((x) =>
        x.id === serverId ? { ...x, unreadCount: Math.max(0, x.unreadCount - channel.unreadCount) } : x,
      ) ?? null,
  }));
}

export const formatBadge = (n: number) => (n >= 100 ? '99+' : String(n));
