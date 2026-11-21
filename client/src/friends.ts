import { create } from 'zustand';
import { api } from './api';
import type { BlockedUser } from './generated/BlockedUser';
import type { Friend } from './generated/Friend';
import type { FriendRequest } from './generated/FriendRequest';
import type { FriendRequests } from './generated/FriendRequests';
import { rememberStatuses } from './presence';

interface FriendsState {
  friends: Friend[] | null;
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
  blocks: BlockedUser[] | null;
  /** Счётчик изменений из сокета: страница перечитывает данные. */
  revision: number;
}

export const useFriends = create<FriendsState>(() => ({
  friends: null,
  incoming: [],
  outgoing: [],
  blocks: null,
  revision: 0,
}));

export async function loadFriends() {
  const friends = await api<Friend[]>('/api/friends');
  rememberStatuses(friends.map((f) => ({ userId: f.userId, username: f.username, role: 'member', roleIds: [], status: f.status, nickname: '', avatarUrl: '', communicationDisabledUntil: null })));
  useFriends.setState({ friends });
}

export async function loadRequests() {
  const reqs = await api<FriendRequests>('/api/friends/requests');
  useFriends.setState({ incoming: reqs.incoming, outgoing: reqs.outgoing });
}

export async function loadBlocks() {
  useFriends.setState({ blocks: await api<BlockedUser[]>('/api/blocks') });
}

export async function loadAllFriends() {
  await Promise.all([loadFriends(), loadRequests(), loadBlocks()]);
}

export async function sendFriendRequest(username: string) {
  await api('/api/friends/requests', { method: 'POST', body: { username } });
  await Promise.all([loadRequests(), loadFriends()]);
}

export async function acceptFriendRequest(id: number) {
  await api(`/api/friends/requests/${id}/accept`, { method: 'POST' });
  await Promise.all([loadRequests(), loadFriends()]);
}

export async function cancelFriendRequest(id: number) {
  await api(`/api/friends/requests/${id}`, { method: 'DELETE' });
  await loadRequests();
}

export async function removeFriend(userId: number) {
  await api(`/api/friends/${userId}`, { method: 'DELETE' });
  await loadFriends();
}

export async function blockUser(userId: number) {
  await api(`/api/blocks/${userId}`, { method: 'PUT' });
  await Promise.all([loadFriends(), loadRequests(), loadBlocks()]);
}

export async function unblockUser(userId: number) {
  await api(`/api/blocks/${userId}`, { method: 'DELETE' });
  await loadBlocks();
}

export function resetFriends() {
  useFriends.setState({ friends: null, incoming: [], outgoing: [], blocks: null, revision: 0 });
}

/** Сокет сообщил об изменении: страница, если открыта, перечитает данные. */
export function bumpFriends() {
  useFriends.setState((s) => ({ revision: s.revision + 1 }));
}
