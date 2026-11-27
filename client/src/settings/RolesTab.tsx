import { useEffect, useState, type FormEvent } from "react";
import type { RoleInfo } from "../generated/RoleInfo";
import type { Server } from "../generated/Server";
import { createRole, deleteRole, loadRoles, updateRole, useGuilds } from "../guilds";
import { errorMessage, ghostButton, inputClass, primaryButton } from "../Modal";
import { PERMISSION_GROUPS, colorToCss, hasPerm, togglePerm } from "../permissions";

export function RolesTab({ server }: { server: Server }) {
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
        <p className="text-sm text-nc-muted">Загрузка…</p>
      ) : (
        <ul className="divide-y divide-nc-line-strong">
          {roles.map((role) => (
            <li key={role.id} className="flex items-center gap-2 py-2">
              <span
                className="size-3 shrink-0 rounded-full bg-nc-strong"
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
                <span className="text-xs text-nc-faint">по умолчанию</span>
              ) : (
                <button
                  onClick={() => remove(role)}
                  disabled={pending}
                  className="text-sm text-nc-muted hover:text-red-400"
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

export function RoleEditor({
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
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
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
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
              Цвет
            </span>
            <input
              type="color"
              value={color || "#000000"}
              onChange={(e) => setColor(e.target.value)}
              className="h-10 w-16 cursor-pointer rounded bg-nc-surface"
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
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
              Позиция
            </span>
            <input
              type="number"
              min={0}
              max={1000}
              value={position}
              onChange={(e) => setPosition(Number(e.target.value))}
              className="w-24 rounded bg-nc-surface px-3 py-2 outline-none focus:ring-2 focus:ring-nc-accent"
            />
          </label>
        </div>
      )}

      <div className="space-y-2">
        {PERMISSION_GROUPS.map((group) => (
          <fieldset key={group.title}>
            <legend className="mb-1 text-xs font-semibold uppercase text-nc-muted">
              {group.title}
            </legend>
            <div className="grid grid-cols-1 gap-1 sm:grid-cols-2">
              {group.items.map((item) => (
                <label
                  key={item.bit}
                  className="flex items-start gap-2 rounded px-1 py-0.5 text-sm hover:bg-nc-hover/40"
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
