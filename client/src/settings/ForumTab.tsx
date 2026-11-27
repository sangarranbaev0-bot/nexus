import { useEffect, useState } from "react";
import type { ThreadTag } from "../generated/ThreadTag";
import type { Server } from "../generated/Server";
import { listTags, createTag, deleteTag, pruneMembers, loadChannels, updateChannel, useGuilds } from "../guilds";
import { errorMessage, ghostButton, inputClass, primaryButton } from "../Modal";

export function ForumTab({ server }: { server: Server }) {
  const channels = useGuilds((s) => s.channels[server.id]) ?? [];
  const [items, setItems] = useState<ThreadTag[]>([]);
  const [name, setName] = useState("");
  const [days, setDays] = useState(30);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState("");
  const reload = () => listTags(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const forum = channels.find((c) => c.isForum);
  const toggleForum = async (channelId: number, current: boolean) => {
    try {
      await updateChannel(channelId, { name: null, categoryId: 0, position: null, isForum: !current });
      await loadChannels(server.id);
    } catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-nc-muted">Форум-канал: каждый пост — отдельная ветка с архивацией и блокировкой.</p>
      <ul className="space-y-1">
        {channels.filter((c) => c.kind === "text").map((c) => (
          <li key={c.id} className="flex items-center justify-between rounded bg-nc-surface px-3 py-2 text-sm">
            <span>#{c.name}</span>
            <button onClick={() => void toggleForum(c.id, c.isForum)} className="text-sm text-nc-accent-text">
              {c.isForum ? "Отключить форум" : "Сделать форумом"}
            </button>
          </li>
        ))}
      </ul>
      <form onSubmit={async (e) => { e.preventDefault(); if (!name) return; try { await createTag(server.id, { name }); setName(""); await reload(); } catch (err) { setError(errorMessage(err)); } }} className="flex gap-2">
        <input value={name} onChange={(e) => setName(e.target.value)} className={inputClass} placeholder="Новый тег ветки" />
        <button className={primaryButton}>Добавить</button>
      </form>
      {!forum && <p className="text-xs text-nc-faint">Теги привязываются к первому форум-каналу. Создайте его выше.</p>}
      <ul className="flex flex-wrap gap-2">
        {items.map((tag) => (
          <li key={tag.id} className="flex items-center gap-1 rounded bg-nc-hover px-2 py-1 text-xs">
            {tag.name}
            <button onClick={() => void deleteTag(server.id, tag.id).then(reload)} className="text-red-400">×</button>
          </li>
        ))}
      </ul>
      <div className="flex items-end gap-2 border-t border-nc-line-strong pt-3">
        <label className="flex-1 text-xs text-nc-muted">Чистка неактивных (дней)
          <input type="number" min={1} max={365} value={days} onChange={(e) => setDays(Number(e.target.value))} className={inputClass} />
        </label>
        <button onClick={() => void pruneMembers(server.id, days).then((r) => setInfo(`Удалено: ${r.removed}`), (e) => setError(errorMessage(e)))} className={ghostButton}>
          Очистить
        </button>
      </div>
      {info && <p className="text-sm text-emerald-400">{info}</p>}
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
    </div>
  );
}
