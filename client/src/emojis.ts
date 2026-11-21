import { create } from 'zustand';
import { api } from './api';
import type { Emoji } from './generated/Emoji';

/** Свои эмодзи по серверам: `null` — ещё не загружено. */
export const useEmojis = create<{ byServer: Record<number, Emoji[] | undefined> }>(() => ({
  byServer: {},
}));

export async function loadEmojis(serverId: number) {
  const list = await api<Emoji[]>(`/api/servers/${serverId}/emojis`);
  useEmojis.setState((s) => ({ byServer: { ...s.byServer, [serverId]: list } }));
  return list;
}

export async function loadAllEmojis(serverIds: number[]) {
  const pending = serverIds.filter((id) => useEmojis.getState().byServer[id] === undefined);
  await Promise.all(pending.map((id) => loadEmojis(id).catch(() => {})));
}

export async function addEmoji(serverId: number, name: string, file: File) {
  const form = new FormData();
  form.append('name', name);
  form.append('file', file);
  const emoji = await api<Emoji>(`/api/servers/${serverId}/emojis`, { method: 'POST', form });
  await loadEmojis(serverId).catch(() => {});
  return emoji;
}

export async function removeEmoji(serverId: number, emojiId: number) {
  await api(`/api/servers/${serverId}/emojis/${emojiId}`, { method: 'DELETE' });
  await loadEmojis(serverId).catch(() => {});
}

/** Все известные свои эмодзи: имя → ссылка. При совпадении имён побеждает первый сервер. */
export function emojiUrl(name: string): string | undefined {
  for (const list of Object.values(useEmojis.getState().byServer)) {
    const hit = list?.find((e) => e.name === name);
    if (hit) return hit.url;
  }
  return undefined;
}

export const customEmojis = () =>
  Object.values(useEmojis.getState().byServer).flatMap((list) => list ?? []);
