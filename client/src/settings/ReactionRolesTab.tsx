import { useEffect, useState, type FormEvent } from "react";
import type { ReactionRole } from "../generated/ReactionRole";
import type { Server } from "../generated/Server";
import { listReactionRoles, createReactionRole, removeReactionRole, loadRoles, useGuilds } from "../guilds";
import { errorMessage, inputClass, primaryButton } from "../Modal";

/** Настройка выдачи ролей за реакции на сообщения. */
export function ReactionRolesTab({ server }: { server: Server }) {
  const roles = useGuilds((s) => s.roles[server.id]) ?? [];
  const [items, setItems] = useState<ReactionRole[]>([]);
  const [messageId, setMessageId] = useState('');
  const [emoji, setEmoji] = useState('');
  const [roleId, setRoleId] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const reload = async () => {
    try {
      setItems(await listReactionRoles(server.id));
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  useEffect(() => {
    void loadRoles(server.id).catch((e) => setError(errorMessage(e)));
    void reload();
  }, [server.id]);

  const create = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const id = Number(messageId);
    if (!id || !emoji || !roleId) return;
    setPending(true);
    setError(null);
    try {
      await createReactionRole(server.id, id, { emoji, roleId });
      setMessageId('');
      setEmoji('');
      setRoleId(0);
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const remove = async (item: ReactionRole) => {
    try {
      await removeReactionRole(server.id, item.messageId, item.emoji, item.roleId);
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const roleName = (id: number) => roles.find((r) => r.id === id)?.name ?? `#${id}`;

  return (
    <div className="space-y-4">
      <form onSubmit={create} className="space-y-3">
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
            ID сообщения (правый клик → Копировать ID)
          </span>
          <input value={messageId} onChange={(e) => setMessageId(e.target.value)} className={inputClass} placeholder="123456" required />
        </label>
        <div className="grid grid-cols-2 gap-2">
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">Эмодзи</span>
            <input value={emoji} onChange={(e) => setEmoji(e.target.value)} className={inputClass} placeholder="🎉 или :name:" required />
          </label>
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">Роль</span>
            <select value={roleId} onChange={(e) => setRoleId(Number(e.target.value))} className={inputClass} required>
              <option value={0}>Выберите роль</option>
              {roles.filter((r) => !r.isDefault).map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
            </select>
          </label>
        </div>
        <button disabled={pending} className={primaryButton}>Добавить</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      {items.length === 0 ? (
        <p className="text-nc-muted">Пока нет ролей за реакции.</p>
      ) : (
        <ul className="divide-y divide-nc-line-strong">
          {items.map((item) => (
            <li key={`${item.messageId}-${item.emoji}-${item.roleId}`} className="flex items-center gap-2 py-2 text-sm">
              <span className="text-lg">{item.emoji}</span>
              <span className="flex-1 truncate">сообщение #{item.messageId} → {roleName(item.roleId)}</span>
              <button type="button" onClick={() => void remove(item)} className="text-xs text-red-400 hover:underline">
                Удалить
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
