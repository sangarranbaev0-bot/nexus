import { create } from 'zustand';
import { api } from './api';
import type { AuditEntry } from './generated/AuditEntry';
import type { Category } from './generated/Category';
import type { Channel } from './generated/Channel';
import type { ChannelKind } from './generated/ChannelKind';
import type { CreateInviteRequest } from './generated/CreateInviteRequest';
import type { CreateRoleRequest } from './generated/CreateRoleRequest';
import type { Invite } from './generated/Invite';
import type { Member } from './generated/Member';
import type { Overwrite } from './generated/Overwrite';
import type { Role } from './generated/Role';
import type { RoleInfo } from './generated/RoleInfo';
import type { Server } from './generated/Server';
import type { UpdateChannelRequest } from './generated/UpdateChannelRequest';
import type { UpdateRoleRequest } from './generated/UpdateRoleRequest';
import type { UpsertOverwriteRequest } from './generated/UpsertOverwriteRequest';

import { P, hasPerm } from './permissions';

interface GuildsState {
  /** `null` — ещё не загружено. */
  servers: Server[] | null;
  channels: Record<number, Channel[] | undefined>;
  categories: Record<number, Category[] | undefined>;
  roles: Record<number, RoleInfo[] | undefined>;
}

export const useGuilds = create<GuildsState>(() => ({
  servers: null,
  channels: {},
  categories: {},
  roles: {},
}));

const rank: Record<Role, number> = { member: 0, admin: 1, owner: 2 };
export const canManage = (server: Server | undefined) => !!server && rank[server.myRole] >= rank.admin;
export const can = (server: Server | undefined, permission: number) =>
  !!server && hasPerm(server.myPermissions, permission);

export async function loadServers() {
  useGuilds.setState({ servers: await api<Server[]>('/api/servers') });
}

export async function loadChannels(serverId: number) {
  const list = await api<Channel[]>(`/api/servers/${serverId}/channels`);
  useGuilds.setState((s) => ({ channels: { ...s.channels, [serverId]: list } }));
  return list;
}

export async function loadCategories(serverId: number) {
  const list = await api<Category[]>(`/api/servers/${serverId}/categories`);
  useGuilds.setState((s) => ({ categories: { ...s.categories, [serverId]: list } }));
  return list;
}

export async function loadRoles(serverId: number) {
  const list = await api<RoleInfo[]>(`/api/servers/${serverId}/roles`);
  useGuilds.setState((s) => ({ roles: { ...s.roles, [serverId]: list } }));
  return list;
}

export async function createRole(serverId: number, req: CreateRoleRequest) {
  const role = await api<RoleInfo>(`/api/servers/${serverId}/roles`, { method: 'POST', body: req });
  await loadRoles(serverId);
  return role;
}

export async function updateRole(serverId: number, roleId: number, req: UpdateRoleRequest) {
  const role = await api<RoleInfo>(`/api/servers/${serverId}/roles/${roleId}`, {
    method: 'PATCH',
    body: req,
  });
  await loadRoles(serverId);
  return role;
}

export async function deleteRole(serverId: number, roleId: number) {
  await api(`/api/servers/${serverId}/roles/${roleId}`, { method: 'DELETE' });
  await loadRoles(serverId);
}

export async function assignRoles(serverId: number, userId: number, roleIds: number[]) {
  await api(`/api/servers/${serverId}/members/${userId}/roles`, {
    method: 'PUT',
    body: { roleIds },
  });
}

export async function createCategory(serverId: number, name: string) {
  const category = await api<Category>(`/api/servers/${serverId}/categories`, {
    method: 'POST',
    body: { name },
  });
  await loadCategories(serverId);
  return category;
}

export async function updateCategory(serverId: number, categoryId: number, name: string | null, position: number | null) {
  await api(`/api/servers/${serverId}/categories/${categoryId}`, {
    method: 'PATCH',
    body: { name, position },
  });
  await loadCategories(serverId);
}

export async function deleteCategory(serverId: number, categoryId: number) {
  await api(`/api/servers/${serverId}/categories/${categoryId}`, { method: 'DELETE' });
  await loadCategories(serverId);
  await loadChannels(serverId);
}

export async function updateChannel(channelId: number, req: UpdateChannelRequest) {
  await api(`/api/channels/${channelId}`, { method: 'PATCH', body: req });
  const serverId = serverOfChannel(channelId);
  if (serverId !== undefined) await loadChannels(serverId);
}

export const listAudit = (serverId: number) =>
  api<AuditEntry[]>(`/api/servers/${serverId}/audit`);

export const listOverwrites = (channelId: number) =>
  api<Overwrite[]>(`/api/channels/${channelId}/overwrites`);

export const upsertOverwrite = (channelId: number, req: UpsertOverwriteRequest) =>
  api(`/api/channels/${channelId}/overwrites`, { method: 'PUT', body: req });

export const deleteOverwrite = (channelId: number, overwriteId: number) =>
  api(`/api/channels/${channelId}/overwrites/${overwriteId}`, { method: 'DELETE' });

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
