import { create } from 'zustand';
import { api } from './api';
import { hasPerm } from './permissions';
import type { AuditEntry } from './generated/AuditEntry';
import type { CreateReactionRoleRequest } from './generated/CreateReactionRoleRequest';
import type { CreateOnboardingQuestionRequest } from './generated/CreateOnboardingQuestionRequest';
import type { CreateServerEventRequest } from './generated/CreateServerEventRequest';
import type { CreateThreadTagRequest } from './generated/CreateThreadTagRequest';
import type { CreateFromTemplateRequest } from './generated/CreateFromTemplateRequest';
import type { Message } from './generated/Message';
import type { OnboardingQuestion } from './generated/OnboardingQuestion';
import type { PruneResponse } from './generated/PruneResponse';
import type { ServerEvent } from './generated/ServerEvent';
import type { ServerTemplate } from './generated/ServerTemplate';
import type { ThreadTag } from './generated/ThreadTag';
import type { MemberSearchResponse } from './generated/MemberSearchResponse';
import type { ReactionRole } from './generated/ReactionRole';
import type { Category } from './generated/Category';
import type { Channel } from './generated/Channel';
import type { ChannelKind } from './generated/ChannelKind';
import type { CreateInviteRequest } from './generated/CreateInviteRequest';
import type { CreateRoleRequest } from './generated/CreateRoleRequest';
import type { Invite } from './generated/Invite';
import type { Member } from './generated/Member';
import type { Overwrite } from './generated/Overwrite';
import type { Role } from './generated/Role';
import type { Ticket } from './generated/Ticket';
import type { RoleInfo } from './generated/RoleInfo';
import type { Server } from './generated/Server';
import type { UpdateChannelRequest } from './generated/UpdateChannelRequest';
import type { UpdateRoleRequest } from './generated/UpdateRoleRequest';
import type { UpsertOverwriteRequest } from './generated/UpsertOverwriteRequest';
import type { BanInfo } from './generated/BanInfo';
import type { BanRequest } from './generated/BanRequest';
import type { EmbedPreview } from './generated/EmbedPreview';
import type { StickerPack } from './generated/StickerPack';
import type { Sticker } from './generated/Sticker';
import type { CreateStickerPackRequest } from './generated/CreateStickerPackRequest';
import type { CreateStickerRequest } from './generated/CreateStickerRequest';
import type { AutoModRule } from './generated/AutoModRule';
import type { CreateAutoModRuleRequest } from './generated/CreateAutoModRuleRequest';

interface GuildsState {
  /** `null` — ещё не загружено. */
  servers: Server[] | null;
  channels: Record<number, Channel[] | undefined>;
  categories: Record<number, Category[] | undefined>;
  roles: Record<number, RoleInfo[] | undefined>;
  members: Record<number, Member[]>;
}

export const useGuilds = create<GuildsState>(() => ({
  servers: null,
  channels: {},
  categories: {},
  roles: {},
  members: {},
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

export const listMembers = (serverId: number) =>
  api<Member[]>(`/api/servers/${serverId}/members`);

export async function loadMembers(serverId: number) {
  const members = await listMembers(serverId);
  useGuilds.setState((s) => ({ members: { ...s.members, [serverId]: members } }));
}

/** Owner меняет роль участника (admin/member). */
export function setMemberRole(serverId: number, userId: number, role: string) {
  return api(`/api/servers/${serverId}/members/${userId}`, {
    method: 'PATCH',
    body: { role },
  });
}

/** Волна всем участникам сервера. */
export function wave(serverId: number) {
  return api(`/api/servers/${serverId}/wave`, { method: 'POST' });
}

export const listBans = (serverId: number) => api<BanInfo[]>(`/api/servers/${serverId}/bans`);

export function banMember(serverId: number, userId: number, body: BanRequest) {
  return api(`/api/servers/${serverId}/members/${userId}/ban`, {
    method: 'POST',
    body,
  });
}

export function unbanMember(serverId: number, userId: number) {
  return api(`/api/servers/${serverId}/members/${userId}/ban`, { method: 'DELETE' });
}

export function kickMember(serverId: number, userId: number, body: BanRequest) {
  return api(`/api/servers/${serverId}/members/${userId}/kick`, {
    method: 'POST',
    body,
  });
}

export function timeoutMember(serverId: number, userId: number, seconds: number) {
  return api(`/api/servers/${serverId}/members/${userId}/timeout`, {
    method: 'PATCH',
    body: { seconds },
  });
}

export function unTimeoutMember(serverId: number, userId: number) {
  return api(`/api/servers/${serverId}/members/${userId}/timeout`, {
    method: 'DELETE',
  });
}

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

/** Обновляет системный/AFK-каналы; список серверов перечитываем, чтобы подтянуть изменения. */
export async function setMemberProfile(serverId: number, body: { nickname: string; avatarUrl: string }) {
  await api(`/api/servers/${serverId}/members/me`, { method: 'PUT', body });
}

export async function listOnboarding(serverId: number) {
  return api<OnboardingQuestion[]>(`/api/servers/${serverId}/onboarding`);
}
export async function createOnboarding(serverId: number, body: CreateOnboardingQuestionRequest) {
  await api(`/api/servers/${serverId}/onboarding`, { method: 'POST', body });
}
export async function deleteOnboarding(serverId: number, questionId: number) {
  await api(`/api/servers/${serverId}/onboarding/${questionId}`, { method: 'DELETE' });
}
export async function answerOnboarding(serverId: number, questionId: number) {
  await api(`/api/servers/${serverId}/onboarding/${questionId}/answer`, { method: 'POST' });
}
export async function listEvents(serverId: number) {
  return api<ServerEvent[]>(`/api/servers/${serverId}/events`);
}
export async function createEvent(serverId: number, body: CreateServerEventRequest) {
  await api(`/api/servers/${serverId}/events`, { method: 'POST', body });
}
export async function deleteEvent(serverId: number, eventId: number) {
  await api(`/api/servers/${serverId}/events/${eventId}`, { method: 'DELETE' });
}
export async function toggleInterest(serverId: number, eventId: number) {
  await api(`/api/servers/${serverId}/events/${eventId}/interest`, { method: 'POST' });
}
export async function listTags(serverId: number) {
  return api<ThreadTag[]>(`/api/servers/${serverId}/tags`);
}
export async function createTag(serverId: number, body: CreateThreadTagRequest) {
  await api(`/api/servers/${serverId}/tags`, { method: 'POST', body });
}
export async function deleteTag(serverId: number, tagId: number) {
  await api(`/api/servers/${serverId}/tags/${tagId}`, { method: 'DELETE' });
}
export async function getTemplate(serverId: number) {
  return api<ServerTemplate>(`/api/servers/${serverId}/template`);
}
export async function createFromTemplate(template: ServerTemplate) {
  return api<Server>(`/api/servers/template`, { method: 'POST', body: { template } });
}
export async function pruneMembers(serverId: number, days: number) {
  return api<PruneResponse>(`/api/servers/${serverId}/prune?days=${days}`, { method: 'POST' });
}
export async function listThreads(channelId: number) {
  return api<Message[]>(`/api/channels/${channelId}/threads`);
}
export async function createThread(channelId: number, content: string) {
  return api<Message>(`/api/channels/${channelId}/threads`, { method: 'POST', body: { content } });
}
export async function updateThread(messageId: number, body: { archived?: boolean; locked?: boolean }) {
  await api(`/api/messages/${messageId}/thread`, { method: 'PATCH', body });
}
export async function loadReplies(rootId: number) {
  return api<Message[]>(`/api/messages/${rootId}/replies`);
}

export async function listReactionRoles(serverId: number) {
  return api<ReactionRole[]>(`/api/servers/${serverId}/reaction-roles`);
}

export async function createReactionRole(serverId: number, messageId: number, body: CreateReactionRoleRequest) {
  await api(`/api/servers/${serverId}/messages/${messageId}/reaction-roles`, { method: 'POST', body });
}

export async function removeReactionRole(serverId: number, messageId: number, emoji: string, roleId: number) {
  await api(
    `/api/servers/${serverId}/reaction-roles/${messageId}/${encodeURIComponent(emoji)}/${roleId}`,
    { method: 'DELETE' },
  );
}

export async function searchMembers(serverId: number, q: string) {
  return api<MemberSearchResponse>(
    `/api/servers/${serverId}/members/search?q=${encodeURIComponent(q)}&limit=50`,
  );
}

export async function listTickets(serverId: number) {
  return api<Ticket[]>(`/api/servers/${serverId}/tickets`);
}
export async function createTicket(serverId: number, title: string) {
  return api<Ticket>(`/api/servers/${serverId}/tickets`, { method: 'POST', body: { title } });
}
export async function updateTicket(serverId: number, ticketId: number, body: { status?: string; title?: string }) {
  return api<Ticket>(`/api/servers/${serverId}/tickets/${ticketId}`, { method: 'PATCH', body });
}

export async function updateServer(serverId: number, body: import('./generated/UpdateServerRequest').UpdateServerRequest) {
  await api(`/api/servers/${serverId}`, { method: 'PATCH', body });
  const servers = await api<Server[]>('/api/servers');
  useGuilds.setState({ servers });
}

/** Принимает код или ссылку целиком: берём последний сегмент. */
export async function joinInvite(codeOrLink: string) {
  const code = codeOrLink.trim().split('/').pop() ?? '';
  const server = await api<Server>(`/api/invites/${encodeURIComponent(code)}/join`, { method: 'POST' });
  addServer(server);
  return server;
}

export async function createChannel(
  serverId: number,
  name: string,
  kind: ChannelKind = 'text',
  isForum = false,
) {
  const channel = await api<Channel>(`/api/servers/${serverId}/channels`, {
    method: 'POST',
    body: { name, kind, is_forum: isForum },
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

export const listInvites = (serverId: number) => api<Invite[]>(`/api/servers/${serverId}/invites`);

export const deleteInvite = (code: string) =>
  api<void>(`/api/invites/${encodeURIComponent(code)}`, { method: 'DELETE' });

// --- Server/channel lookup helpers ---

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

export async function loadEmbed(messageId: number) {
  return api<EmbedPreview | null>(`/api/messages/${messageId}/embed`);
}

export const formatBadge = (n: number) => (n >= 100 ? '99+' : String(n));

export async function listStickerPacks(serverId: number) {
  return api<StickerPack[]>(`/api/servers/${serverId}/stickers/packages`);
}

export async function createStickerPack(serverId: number, req: CreateStickerPackRequest) {
  return api<StickerPack>(`/api/servers/${serverId}/stickers/packages`, { method: 'POST', body: JSON.stringify(req) });
}

export async function getStickerPack(serverId: number, packId: number) {
  return api<StickerPack>(`/api/servers/${serverId}/stickers/packages/${packId}`);
}

export async function deleteStickerPack(serverId: number, packId: number) {
  return api<void>(`/api/servers/${serverId}/stickers/packages/${packId}`, { method: 'DELETE' });
}

export async function createSticker(serverId: number, packId: number, req: CreateStickerRequest) {
  return api<Sticker>(`/api/servers/${serverId}/stickers/packages/${packId}/stickers`, { method: 'POST', body: JSON.stringify(req) });
}

export async function deleteSticker(serverId: number, stickerId: number) {
  return api<void>(`/api/servers/${serverId}/stickers/${stickerId}`, { method: 'DELETE' });
}

export async function listChannelStickers(channelId: number) {
  return api<Sticker[]>(`/api/channels/${channelId}/stickers`);
}

export async function listPackStickers(serverId: number, packId: number) {
  return api<Sticker[]>(`/api/servers/${serverId}/stickers/packages/${packId}/stickers`);
}

export interface ServerSearchResult {
  messageId: number;
  channelId: number;
  channelName: string;
  message: Message;
}

export interface ServerSearchParams {
  q: string;
  channelId?: number;
  limit?: number;
}

export async function searchServerMessages(serverId: number, params: ServerSearchParams) {
  const qs = new URLSearchParams({ q: params.q });
  if (params.channelId) qs.set('channelId', String(params.channelId));
  if (params.limit) qs.set('limit', String(params.limit));
  return api<{ results: ServerSearchResult[]; hasMore: boolean }>(
    `/api/servers/${serverId}/messages/search?${qs}`,
  );
}

export const listAutoModRules = (serverId: number) =>
  api<AutoModRule[]>(`/api/servers/${serverId}/automod`);

export const createAutoModRule = (serverId: number, req: CreateAutoModRuleRequest) =>
  api<AutoModRule>(`/api/servers/${serverId}/automod`, { method: 'POST', body: req });

export const deleteAutoModRule = (serverId: number, ruleId: number) =>
  api<void>(`/api/servers/${serverId}/automod/${ruleId}`, { method: 'DELETE' });

export const getServerBoosts = (serverId: number) =>
  api<{ level: number; count: number; boosters: Array<{ user_id: number; username: string; boosted_at: number }> }>(
    `/api/servers/${serverId}/boosts`,
  );

export const toggleServerBoost = (serverId: number) =>
  api<{ level: number; count: number; boosters: Array<{ user_id: number; username: string; boosted_at: number }> }>(
    `/api/servers/${serverId}/boosts`,
    { method: 'POST' },
  );