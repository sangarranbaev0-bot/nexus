import { useEffect, useState, type FormEvent } from "react";
import type { AuditEntry } from "./generated/AuditEntry";
import type { RoleInfo } from "./generated/RoleInfo";
import type { Server } from "./generated/Server";
import {
  createCategory,
  createRole,
  deleteCategory,
  deleteRole,
  listAudit,
  loadCategories,
  loadChannels,
  loadRoles,
  updateCategory,
  updateChannel,
  updateRole,
  useGuilds,
} from "./guilds";
import {
  Modal,
  errorMessage,
  ghostButton,
  inputClass,
  primaryButton,
} from "./Modal";
import {
  P,
  PERMISSION_GROUPS,
  colorToCss,
  hasPerm,
  togglePerm,
} from "./permissions";

type Tab = "roles" | "categories" | "audit";

const tabLabel: Record<Tab, string> = {
  roles: "Роли",
  categories: "Категории",
  audit: "Аудит",
};

export function ServerSettingsDialog({
  server,
  onClose,
}: {
  server: Server;
  onClose: () => void;
}) {
  const [tab, setTab] = useState<Tab>(
    hasPerm(server.myPermissions, P.MANAGE_ROLES)
      ? "roles"
      : hasPerm(server.myPermissions, P.MANAGE_CHANNELS)
        ? "categories"
        : "audit",
  );
  const tabs: Tab[] = [];
  if (hasPerm(server.myPermissions, P.MANAGE_ROLES)) tabs.push("roles");
  if (hasPerm(server.myPermissions, P.MANAGE_CHANNELS))
    tabs.push("categories");
  if (hasPerm(server.myPermissions, P.VIEW_AUDIT_LOG)) tabs.push("audit");
  return (
    <Modal title={`Настройки «${server.name}»`} onClose={onClose}>
      <div role="tablist" className="flex gap-1 border-b border-zinc-700 pb-2">
        {tabs.map((key) => (
          <button
            key={key}
            role="tab"
            aria-selected={tab === key}
            onClick={() => setTab(key)}
            className={`rounded px-3 py-1.5 text-sm ${
              tab === key
                ? "bg-zinc-700 text-white"
                : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
            }`}
          >
            {tabLabel[key]}
          </button>
        ))}
      </div>
      <div className="max-h-[60vh] overflow-y-auto pr-1">
        {tab === "roles" && <RolesTab server={server} />}
        {tab === "categories" && <CategoriesTab server={server} />}
        {tab === "audit" && <AuditTab server={server} />}
      </div>
    </Modal>
  );
}

function RolesTab({ server }: { server: Server }) {
  const roles = useGuilds((s) => s.roles[server.id]);
  const [editing, setEditing] = useState<
    { mode: "new" } | { mode: "edit"; role: RoleInfo } | null
  >(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    void loadRoles(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  if (editing)
    return (
      <RoleEditor
        server={server}
        role={editing.mode === "edit" ? editing.role : null}
        onDone={() => setEditing(null)}
      />
    );

  const remove = async (role: RoleInfo) => {
    if (!confirm(`Удалить роль «${role.name}»?`)) return;
    setPending(true);
    setError(null);
    try {
      await deleteRole(server.id, role.id);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="space-y-3">
      <button
        onClick={() => setEditing({ mode: "new" })}
        className={primaryButton}
      >
        Создать роль
      </button>
      {roles === undefined ? (
        <p className="text-sm text-zinc-400">Загрузка…</p>
      ) : (
        <ul className="divide-y divide-zinc-700">
          {roles.map((role) => (
            <li key={role.id} className="flex items-center gap-2 py-2">
              <span
                className="size-3 shrink-0 rounded-full bg-zinc-500"
                style={{ backgroundColor: colorToCss(role.color) }}
                aria-hidden
              />
              <button
                onClick={() => setEditing({ mode: "edit", role })}
                className="flex-1 truncate text-left hover:underline"
                style={{ color: colorToCss(role.color) }}
              >
                {role.name}
              </button>
              {role.isDefault ? (
                <span className="text-xs text-zinc-500">по умолчанию</span>
              ) : (
                <button
                  onClick={() => remove(role)}
                  disabled={pending}
                  className="text-sm text-zinc-400 hover:text-red-400"
                  aria-label={`Удалить роль ${role.name}`}
                >
                  ×
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}

function RoleEditor({
  server,
  role,
  onDone,
}: {
  server: Server;
  role: RoleInfo | null;
  onDone: () => void;
}) {
  const isNew = role === null;
  const isDefault = role?.isDefault ?? false;
  const [name, setName] = useState(role?.name ?? "");
  const [color, setColor] = useState(
    role?.color != null
      ? `#${role.color.toString(16).padStart(6, "0")}`
      : "",
  );
  const [permissions, setPermissions] = useState(role?.permissions ?? 0);
  const [hoist, setHoist] = useState(role?.hoist ?? false);
  const [mentionable, setMentionable] = useState(role?.mentionable ?? false);
  const [position, setPosition] = useState(role?.position ?? 0);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const save = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setPending(true);
    setError(null);
    const colorValue = color ? parseInt(color.slice(1), 16) : null;
    try {
      if (isNew) {
        await createRole(server.id, {
          name,
          color: colorValue,
          permissions,
          hoist,
          mentionable,
        });
      } else {
        await updateRole(server.id, role.id, {
          name: isDefault ? null : name,
          color: isDefault ? null : (colorValue ?? -1),
          permissions,
          hoist: isDefault ? null : hoist,
          mentionable: isDefault ? null : mentionable,
          position: isDefault ? null : position,
        });
      }
      onDone();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  return (
    <form onSubmit={save} className="space-y-4">
      <h3 className="font-semibold">
        {isNew ? "Новая роль" : `Роль «${role.name}»`}
      </h3>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
          Название
        </span>
        <input
          required
          maxLength={100}
          disabled={isDefault}
          value={name}
          onChange={(e) => setName(e.target.value)}
          className={`${inputClass} disabled:opacity-60`}
        />
      </label>

      {!isDefault && (
        <div className="flex items-end gap-2">
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
              Цвет
            </span>
            <input
              type="color"
              value={color || "#000000"}
              onChange={(e) => setColor(e.target.value)}
              className="h-10 w-16 cursor-pointer rounded bg-zinc-900"
            />
          </label>
          <button
            type="button"
            onClick={() => setColor("")}
            className={ghostButton}
          >
            Без цвета
          </button>
          <label className="ml-auto block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
              Позиция
            </span>
            <input
              type="number"
              min={0}
              max={1000}
              value={position}
              onChange={(e) => setPosition(Number(e.target.value))}
              className="w-24 rounded bg-zinc-900 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500"
            />
          </label>
        </div>
      )}

      <div className="space-y-2">
        {PERMISSION_GROUPS.map((group) => (
          <fieldset key={group.title}>
            <legend className="mb-1 text-xs font-semibold uppercase text-zinc-400">
              {group.title}
            </legend>
            <div className="grid grid-cols-1 gap-1 sm:grid-cols-2">
              {group.items.map((item) => (
                <label
                  key={item.bit}
                  className="flex items-start gap-2 rounded px-1 py-0.5 text-sm hover:bg-zinc-700/40"
                  title={item.hint}
                >
                  <input
                    type="checkbox"
                    checked={hasPerm(permissions, item.bit)}
                    onChange={(e) =>
                      setPermissions((bits) =>
                        togglePerm(bits, item.bit, e.target.checked),
                      )
                    }
                    className="mt-0.5"
                  />
                  <span>{item.label}</span>
                </label>
              ))}
            </div>
          </fieldset>
        ))}
      </div>

      {!isDefault && (
        <div className="flex flex-wrap gap-4 text-sm">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={hoist}
              onChange={(e) => setHoist(e.target.checked)}
            />
            Показывать отдельно в списке участников
          </label>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={mentionable}
              onChange={(e) => setMentionable(e.target.checked)}
            />
            Упоминать роль
          </label>
        </div>
      )}

      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-2">
        <button type="button" className={ghostButton} onClick={onDone}>
          Отмена
        </button>
        <button disabled={pending} className={primaryButton}>
          Сохранить
        </button>
      </div>
    </form>
  );
}

function CategoriesTab({ server }: { server: Server }) {
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

      <ul className="divide-y divide-zinc-700">
        {categories.length === 0 && (
          <li className="py-2 text-sm text-zinc-500">Категорий пока нет.</li>
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
                    className="rounded px-2 text-zinc-400 hover:bg-zinc-700 disabled:opacity-30"
                  >
                    ↑
                  </button>
                  <button
                    onClick={() => move(category.id, 1)}
                    disabled={pending || index === list.length - 1}
                    aria-label="Ниже"
                    className="rounded px-2 text-zinc-400 hover:bg-zinc-700 disabled:opacity-30"
                  >
                    ↓
                  </button>
                  <button
                    onClick={() =>
                      setEditing({ id: category.id, name: category.name })
                    }
                    className="text-sm text-indigo-400 hover:underline"
                  >
                    Изменить
                  </button>
                  <button
                    onClick={() => remove(category.id, category.name)}
                    disabled={pending}
                    aria-label={`Удалить категорию ${category.name}`}
                    className="rounded px-2 text-zinc-400 hover:text-red-400"
                  >
                    ×
                  </button>
                </>
              )}
            </li>
          ))}
      </ul>

      <div>
        <h3 className="mb-2 text-xs font-semibold uppercase text-zinc-400">
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
                <span className="flex-1 truncate text-zinc-300">
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
                  className="rounded bg-zinc-900 px-2 py-1 outline-none focus:ring-2 focus:ring-indigo-500"
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

const actionLabel: Record<string, string> = {
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
};

function AuditTab({ server }: { server: Server }) {
  const [entries, setEntries] = useState<AuditEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listAudit(server.id).then(setEntries, (e) => setError(errorMessage(e)));
  }, [server.id]);

  if (error)
    return (
      <p role="alert" className="text-sm text-red-400">
        {error}
      </p>
    );
  if (entries === null)
    return <p className="text-sm text-zinc-400">Загрузка…</p>;
  if (entries.length === 0)
    return <p className="text-sm text-zinc-500">Записей пока нет.</p>;

  return (
    <ul className="divide-y divide-zinc-700 text-sm">
      {entries.map((entry) => (
        <li key={entry.id} className="py-2">
          <div className="flex items-baseline justify-between gap-2">
            <span className="font-medium">
              {actionLabel[entry.action] ?? entry.action}
              {entry.target && (
                <span className="font-normal text-zinc-400">
                  {" "}
                  · {entry.target}
                </span>
              )}
            </span>
            <time className="shrink-0 text-xs text-zinc-500">
              {new Date(entry.createdAt).toLocaleString("ru-RU")}
            </time>
          </div>
          <div className="text-xs text-zinc-500">
            {entry.actorUsername}
          </div>
        </li>
      ))}
    </ul>
  );
}