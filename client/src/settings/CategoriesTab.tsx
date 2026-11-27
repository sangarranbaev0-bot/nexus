import { useEffect, useState, type FormEvent } from "react";
import type { Server } from "../generated/Server";
import { createCategory, deleteCategory, loadCategories, loadChannels, updateCategory, updateChannel, useGuilds } from "../guilds";
import { errorMessage, inputClass, primaryButton } from "../Modal";

export function CategoriesTab({ server }: { server: Server }) {
  const categories = useGuilds((s) => s.categories[server.id]) ?? [];
  const channels = useGuilds((s) => s.channels[server.id]) ?? [];
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<{ id: number; name: string } | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    void loadCategories(server.id).catch((e) => setError(errorMessage(e)));
    void loadChannels(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  const create = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!name.trim()) return;
    setPending(true);
    setError(null);
    try {
      await createCategory(server.id, name);
      setName("");
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const saveName = async () => {
    if (!editing) return;
    setPending(true);
    setError(null);
    try {
      await updateCategory(server.id, editing.id, editing.name, null);
      setEditing(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const move = async (categoryId: number, direction: -1 | 1) => {
    const sorted = [...categories].sort(
      (a, b) => a.position - b.position || a.id - b.id,
    );
    const index = sorted.findIndex((c) => c.id === categoryId);
    const other = sorted[index + direction];
    if (!other) return;
    const current = sorted[index];
    setPending(true);
    setError(null);
    try {
      await updateCategory(server.id, current.id, null, other.position);
      await updateCategory(server.id, other.id, null, current.position);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const remove = async (categoryId: number, label: string) => {
    if (!confirm(`Удалить категорию «${label}»? Каналы останутся.`)) return;
    setPending(true);
    setError(null);
    try {
      await deleteCategory(server.id, categoryId);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="space-y-4">
      <form onSubmit={create} className="flex gap-2">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          maxLength={100}
          placeholder="Новая категория"
          className={inputClass}
          aria-label="Название категории"
        />
        <button disabled={pending} className={primaryButton}>
          Добавить
        </button>
      </form>

      <ul className="divide-y divide-nc-line-strong">
        {categories.length === 0 && (
          <li className="py-2 text-sm text-nc-faint">Категорий пока нет.</li>
        )}
        {[...categories]
          .sort((a, b) => a.position - b.position || a.id - b.id)
          .map((category, index, list) => (
            <li key={category.id} className="flex items-center gap-1 py-2">
              {editing?.id === category.id ? (
                <>
                  <input
                    autoFocus
                    value={editing.name}
                    onChange={(e) =>
                      setEditing({ id: category.id, name: e.target.value })
                    }
                    className={inputClass}
                  />
                  <button onClick={saveName} className={primaryButton}>
                    ОК
                  </button>
                </>
              ) : (
                <>
                  <span className="flex-1 truncate">{category.name}</span>
                  <button
                    onClick={() => move(category.id, -1)}
                    disabled={pending || index === 0}
                    aria-label="Выше"
                    className="rounded px-2 text-nc-muted hover:bg-nc-hover disabled:opacity-30"
                  >
                    ↑
                  </button>
                  <button
                    onClick={() => move(category.id, 1)}
                    disabled={pending || index === list.length - 1}
                    aria-label="Ниже"
                    className="rounded px-2 text-nc-muted hover:bg-nc-hover disabled:opacity-30"
                  >
                    ↓
                  </button>
                  <button
                    onClick={() =>
                      setEditing({ id: category.id, name: category.name })
                    }
                    className="text-sm text-nc-accent-text hover:underline"
                  >
                    Изменить
                  </button>
                  <button
                    onClick={() => remove(category.id, category.name)}
                    disabled={pending}
                    aria-label={`Удалить категорию ${category.name}`}
                    className="rounded px-2 text-nc-muted hover:text-red-400"
                  >
                    ×
                  </button>
                </>
              )}
            </li>
          ))}
      </ul>

      <div>
        <h3 className="mb-2 text-xs font-semibold uppercase text-nc-muted">
          Каналы
        </h3>
        <div className="space-y-1">
          {[...channels]
            .sort((a, b) => a.position - b.position || a.id - b.id)
            .map((channel) => (
              <label
                key={channel.id}
                className="flex items-center gap-2 text-sm"
              >
                <span className="flex-1 truncate text-nc-text-2">
                  {channel.kind === "voice" ? "🔊" : "#"} {channel.name}
                </span>
                <select
                  value={channel.categoryId ?? 0}
                  onChange={(e) =>
                    void updateChannel(channel.id, {
                      name: null,
                      categoryId: Number(e.target.value),
                      position: null,
                    }).catch((err) => setError(errorMessage(err)))
                  }
                  className="rounded bg-nc-surface px-2 py-1 outline-none focus:ring-2 focus:ring-nc-accent"
                  aria-label={`Категория канала ${channel.name}`}
                >
                  <option value={0}>Без категории</option>
                  {categories.map((category) => (
                    <option key={category.id} value={category.id}>
                      {category.name}
                    </option>
                  ))}
                </select>
              </label>
            ))}
        </div>
      </div>

      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}

export const actionLabel: Record<string, string> = {
  "role.create": "Создана роль",
  "role.update": "Изменена роль",
  "role.delete": "Удалена роль",
  "member.roles": "Изменены роли участника",
  "category.create": "Создана категория",
  "category.update": "Изменена категория",
  "category.delete": "Удалена категория",
  "channel.update": "Изменён канал",
  "channel.overwrite": "Изменены права канала",
  "channel.overwrite.delete": "Сброшены права канала",
  "member.ban": "Забанен участник",
  "member.unban": "Разбанен участник",
  "member.kick": "Кикнут участник",
  "member.timeout": "Временная блокировка участника",
  "member.timeout.clear": "Снята временная блокировка",
};
