import { useEffect, useState, type FormEvent } from "react";
import type { AuditEntry } from "./generated/AuditEntry";
import type { RoleInfo } from "./generated/RoleInfo";
import type { ReactionRole } from "./generated/ReactionRole";
import type { OnboardingQuestion } from "./generated/OnboardingQuestion";
import type { ServerEvent } from "./generated/ServerEvent";
import type { ThreadTag } from "./generated/ThreadTag";
import type { ServerTemplate } from "./generated/ServerTemplate";
import type { Ticket } from "./generated/Ticket";
import type { StickerPack } from "./generated/StickerPack";
import type { Sticker } from "./generated/Sticker";
import type { Server } from "./generated/Server";
import {
  createCategory,
  createRole,
  deleteCategory,
  deleteRole,
  listAudit,
  loadServers,
  listReactionRoles,
  createReactionRole,
  removeReactionRole,
  listOnboarding,
  createOnboarding,
  deleteOnboarding,
  listEvents,
  createEvent,
  deleteEvent,
  toggleInterest,
  listTags,
  createTag,
  deleteTag,
  getTemplate,
  createFromTemplate,
  listTickets,
  createTicket,
  updateTicket,
  pruneMembers,
  loadCategories,
  loadChannels,
  loadRoles,
  updateCategory,
  updateChannel,
  updateRole,
  updateServer,
  useGuilds,
  listBans,
  banMember,
  unbanMember,
  kickMember,
  timeoutMember,
  unTimeoutMember,
  createSticker,
  createStickerPack,
  deleteSticker,
  deleteStickerPack,
  listPackStickers,
  listStickerPacks,
  listAutoModRules,
  createAutoModRule,
  deleteAutoModRule,
  getServerBoosts,
  toggleServerBoost,
} from "./guilds";
import type { BanInfo } from "./generated/BanInfo";
import type { AutoModRule } from "./generated/AutoModRule";
import {
  Modal,
  errorMessage,
  ghostButton,
  inputClass,
  primaryButton,
} from "./Modal";
import { NotifyControls } from "./NotifyControls";
import { addEmoji, loadEmojis, removeEmoji, useEmojis } from "./emojis";
import {
  P,
  PERMISSION_GROUPS,
  colorToCss,
  hasPerm,
  togglePerm,
} from "./permissions";

type Tab =
  | "general"
  | "roles"
  | "categories"
  | "emojis"
  | "stickers"
  | "reactionRoles"
  | "onboarding"
  | "events"
  | "forum"
  | "templates"
  | "audit"
  | "notify"
  | "tickets"
  | "moderation"
  | "automod"
  | "boosts";

const tabLabel: Record<Tab, string> = {
  general: "Общее",
  roles: "Роли",
  categories: "Категории",
  emojis: "Эмодзи",
  stickers: "Стикеры",
  reactionRoles: "Роли за реакции",
  onboarding: "Онбординг",
  events: "События",
  forum: "Форум",
  templates: "Шаблоны",
  audit: "Аудит",
  notify: "Уведомления",
  tickets: "Тикеты",
  moderation: "Модерация",
  automod: "Автомод",
  boosts: "Бусты",
};

export function ServerSettingsDialog({
  server,
  onClose,
}: {
  server: Server;
  onClose: () => void;
}) {
  const [tab, setTab] = useState<Tab>("general");
  const tabs: Tab[] = ["general"];
  if (hasPerm(server.myPermissions, P.MANAGE_ROLES)) tabs.push("roles");
  if (hasPerm(server.myPermissions, P.MANAGE_ROLES)) tabs.push("reactionRoles");
  if (hasPerm(server.myPermissions, P.MANAGE_ROLES)) tabs.push("automod");
  if (hasPerm(server.myPermissions, P.BAN_MEMBERS) || hasPerm(server.myPermissions, P.KICK_MEMBERS))
    tabs.push("moderation");
  tabs.push("onboarding", "events", "forum", "templates");
  if (hasPerm(server.myPermissions, P.MANAGE_CHANNELS))
    tabs.push("categories");
  if (server.myRole !== "member") tabs.push("emojis");
  if (hasPerm(server.myPermissions, P.MANAGE_CHANNELS))
    tabs.push("stickers");
  if (hasPerm(server.myPermissions, P.VIEW_AUDIT_LOG)) tabs.push("audit");
  tabs.push("notify", "tickets", "boosts");
  return (
    <Modal title={`Настройки «${server.name}»`} onClose={onClose}>
      <div role="tablist" className="flex gap-1 border-b border-nc-line-strong pb-2">
        {tabs.map((key) => (
          <button
            key={key}
            role="tab"
            aria-selected={tab === key}
            onClick={() => setTab(key)}
            className={`rounded px-3 py-1.5 text-sm ${
              tab === key
                ? "bg-nc-hover text-white"
                : "text-nc-muted hover:bg-nc-elevated hover:text-nc-text"
            }`}
          >
            {tabLabel[key]}
          </button>
        ))}
      </div>
      <div className="max-h-[60vh] overflow-y-auto pr-1">
        {tab === "general" && <GeneralTab server={server} onAudit={() => setTab("audit")} />}
        {tab === "roles" && <RolesTab server={server} />}
        {tab === "categories" && <CategoriesTab server={server} />}
        {tab === "emojis" && <EmojisTab server={server} />}
        {tab === "reactionRoles" && <ReactionRolesTab server={server} />}
        {tab === "onboarding" && <OnboardingTab server={server} />}
        {tab === "events" && <EventsTab server={server} />}
        {tab === "forum" && <ForumTab server={server} />}
        {tab === "templates" && <TemplatesTab server={server} />}
        {tab === "audit" && <AuditTab server={server} />}
        {tab === "tickets" && <TicketsTab server={server} />}
        {tab === "boosts" && <BoostsTab server={server} />}
        {tab === "moderation" && <ModerationTab server={server} />}
        {tab === "automod" && <AutoModTab server={server} />}
        {tab === "notify" && (
          <NotifyControls
            target={{ serverId: server.id }}
            inheritHint="Каналы наследуют эту настройку, если для них не задано своё значение."
          />
        )}
      </div>
    </Modal>
  );
}

function GeneralTab({ server, onAudit }: { server: Server; onAudit: () => void }) {
  const channels = useGuilds((s) => s.channels[server.id]) ?? [];
  const [systemId, setSystemId] = useState(server.systemChannelId ?? 0);
  const [afkId, setAfkId] = useState(server.afkChannelId ?? 0);
  const [announcementId, setAnnouncementId] = useState(server.announcementChannelId ?? 0);
  const [description, setDescription] = useState(server.description);
  const [rules, setRules] = useState(server.rules);
  const [bannerUrl, setBannerUrl] = useState(server.bannerUrl);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    void loadChannels(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  const save = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setPending(true);
    setError(null);
    try {
      await updateServer(server.id, {
        systemChannelId: systemId || null,
        afkChannelId: afkId || null,
        announcementChannelId: announcementId || null,
        description,
        rules,
        bannerUrl,
      });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const text = channels.filter((c) => c.kind === "text");
  const voice = channels.filter((c) => c.kind === "voice");
  const canEdit = server.myRole !== "member";

  return (
    <form onSubmit={save} className="space-y-4">
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Системный канал (приветствия и события)
        </span>
        <select
          value={systemId}
          disabled={!canEdit}
          onChange={(e) => setSystemId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Не задан</option>
          {text.map((c) => <option key={c.id} value={c.id}>#{c.name}</option>)}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          AFK-канал (после 5 минут бездействия)
        </span>
        <select
          value={afkId}
          disabled={!canEdit}
          onChange={(e) => setAfkId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Не задан</option>
          {voice.map((c) => <option key={c.id} value={c.id}>🔊 {c.name}</option>)}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Канал объявлений
        </span>
        <select
          value={announcementId}
          disabled={!canEdit}
          onChange={(e) => setAnnouncementId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Не задан</option>
          {text.map((c) => <option key={c.id} value={c.id}>#{c.name}</option>)}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Описание сервера
        </span>
        <textarea
          value={description}
          disabled={!canEdit}
          maxLength={2000}
          rows={3}
          onChange={(e) => setDescription(e.target.value)}
          className={inputClass}
          placeholder="Коротко о вашем сервере"
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Правила (Markdown)
        </span>
        <textarea
          value={rules}
          disabled={!canEdit}
          maxLength={8000}
          rows={6}
          onChange={(e) => setRules(e.target.value)}
          className={inputClass}
          placeholder="1. Будьте вежливы\n2. Не спамьте"
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Ссылка на баннер
        </span>
        <input
          value={bannerUrl}
          disabled={!canEdit}
          maxLength={512}
          onChange={(e) => setBannerUrl(e.target.value)}
          className={inputClass}
          placeholder="https://…"
        />
      </label>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      {hasPerm(server.myPermissions, P.VIEW_AUDIT_LOG) && (
        <button type="button" onClick={onAudit} className={ghostButton}>
          Журнал аудита
        </button>
      )}
      {canEdit && <button disabled={pending} className={primaryButton}>Сохранить</button>}
    </form>
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
  "member.ban": "Забанен участник",
  "member.unban": "Разбанен участник",
  "member.kick": "Кикнут участник",
  "member.timeout": "Временная блокировка участника",
  "member.timeout.clear": "Снята временная блокировка",
};

function TicketsTab({ server }: { server: Server }) {
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const reload = () => listTickets(server.id).then(setTickets, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);
  const create = async (e: FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setPending(true);
    setError(null);
    try {
      await createTicket(server.id, title);
      setTitle("");
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };
  const setStatus = async (id: number, status: string) => {
    try { await updateTicket(server.id, id, { status }); await reload(); }
    catch (e) { setError(errorMessage(e)); }
  };
  return (
    <div className="space-y-3">
      <form onSubmit={create} className="flex gap-2">
        <input value={title} onChange={(e) => setTitle(e.target.value)} className={inputClass} placeholder="Новый тикет: проблема/запрос" required />
        <button disabled={pending} className={primaryButton}>Создать</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      {tickets === null ? (
        <p className="text-sm text-nc-muted">Загрузка…</p>
      ) : tickets.length === 0 ? (
        <p className="text-sm text-nc-faint">Тикетов пока нет.</p>
      ) : (
        <ul className="divide-y divide-nc-line-strong">
          {tickets.map((ticket) => (
            <li key={ticket.id} className="flex items-center justify-between py-2 text-sm">
              <div>
                <div className="font-medium">#{ticket.id} · {ticket.title}</div>
                <div className="text-xs text-nc-faint">{new Date(ticket.createdAt).toLocaleString("ru-RU")} · {ticket.status}</div>
              </div>
              <div className="flex gap-2">
                <button onClick={() => setStatus(ticket.id, ticket.status === "open" ? "closed" : "open")} className={ghostButton}>
                  {ticket.status === "open" ? "Закрыть" : "Открыть"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

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
    return <p className="text-sm text-nc-muted">Загрузка…</p>;
  if (entries.length === 0)
    return <p className="text-sm text-nc-faint">Записей пока нет.</p>;

  return (
    <ul className="divide-y divide-nc-line-strong text-sm">
      {entries.map((entry) => (
        <li key={entry.id} className="py-2">
          <div className="flex items-baseline justify-between gap-2">
            <span className="font-medium">
              {actionLabel[entry.action] ?? entry.action}
              {entry.target && (
                <span className="font-normal text-nc-muted">
                  {" "}
                  · {entry.target}
                </span>
              )}
            </span>
            <time className="shrink-0 text-xs text-nc-faint">
              {new Date(entry.createdAt).toLocaleString("ru-RU")}
            </time>
          </div>
          <div className="text-xs text-nc-faint">
            {entry.actorUsername}
          </div>
        </li>
      ))}
        </ul>
  );
}

function EmojisTab({ server }: { server: Server }) {
  const list = useEmojis((s) => s.byServer[server.id]);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    loadEmojis(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  const upload = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const el = e.currentTarget;
    const data = new FormData(el);
    const name = String(data.get("name") ?? "");
    const file = data.get("file");
    if (!(file instanceof File) || file.size === 0) {
      setError("Выберите PNG-файл.");
      return;
    }
    setPending(true);
    setError(null);
    try {
      await addEmoji(server.id, name, file);
      el.reset();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const remove = async (id: number) => {
    if (!confirm("Удалить эмодзи?")) return;
    try {
      await removeEmoji(server.id, id);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <div className="space-y-4">
      <form onSubmit={upload} className="space-y-3">
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
            Имя (a-z, 0-9, _)
          </span>
          <input name="name" required minLength={2} maxLength={32} className={inputClass} placeholder="party_blob" />
        </label>
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
            PNG до 256 КБ
          </span>
          <input name="file" type="file" accept="image/png" required className="block w-full text-sm text-nc-text-2" />
        </label>
        <button type="submit" disabled={pending} className={primaryButton}>
          {pending ? "Загрузка…" : "Загрузить эмодзи"}
        </button>
      </form>
      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
      {list === undefined ? (
        <p className="text-nc-muted">Загрузка…</p>
      ) : list.length === 0 ? (
        <p className="text-nc-muted">Пока нет своих эмодзи.</p>
      ) : (
        <ul className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          {list.map((e) => (
            <li key={e.id} className="flex items-center gap-2 rounded bg-nc-surface p-2">
              <img src={e.url} alt={`:${e.name}:`} className="h-6 w-6" />
              <span className="min-w-0 flex-1 truncate text-sm">:{e.name}:</span>
              <button
                type="button"
                onClick={() => void remove(e.id)}
                className="text-xs text-red-400 hover:underline"
                aria-label={`Удалить эмодзи ${e.name}`}
              >
                Удалить
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}


/** Настройка выдачи ролей за реакции на сообщения. */
function ReactionRolesTab({ server }: { server: Server }) {
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

function OnboardingTab({ server }: { server: Server }) {
  const roles = useGuilds((s) => s.roles[server.id]) ?? [];
  const [items, setItems] = useState<OnboardingQuestion[]>([]);
  const [question, setQuestion] = useState("");
  const [roleId, setRoleId] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const reload = () => listOnboarding(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void loadRoles(server.id); void reload(); }, [server.id]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (!question || !roleId) return;
    try {
      await createOnboarding(server.id, { question, roleId });
      setQuestion(""); setRoleId(0); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-nc-muted">Участник отвечает на вопрос «Да» и получает роль. Ответы учитываются один раз.</p>
      <form onSubmit={add} className="space-y-2">
        <input value={question} onChange={(e) => setQuestion(e.target.value)} className={inputClass} placeholder="Вы прочитали правила?" required />
        <div className="flex gap-2">
          <select value={roleId} onChange={(e) => setRoleId(Number(e.target.value))} className={inputClass} required>
            <option value={0}>Роль</option>
            {roles.filter((r) => !r.isDefault).map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
          </select>
          <button className={primaryButton}>Добавить</button>
        </div>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-nc-line-strong">
        {items.map((q) => (
          <li key={q.id} className="flex items-center gap-2 py-2 text-sm">
            <span className="flex-1">{q.question}</span>
            <span className="text-nc-muted">{roles.find((r) => r.id === q.roleId)?.name}</span>
            <button onClick={() => void deleteOnboarding(server.id, q.id).then(reload)} className="text-xs text-red-400">Удалить</button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function EventsTab({ server }: { server: Server }) {
  const [items, setItems] = useState<ServerEvent[]>([]);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [time, setTime] = useState("");
  const [error, setError] = useState<string | null>(null);
  const reload = () => listEvents(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    const starts = new Date(time).getTime();
    if (!name || !Number.isFinite(starts)) return;
    try {
      await createEvent(server.id, { name, description, startsAt: starts, reminder: true });
      setName(""); setDescription(""); setTime(""); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <form onSubmit={add} className="space-y-2">
        <input value={name} onChange={(e) => setName(e.target.value)} className={inputClass} placeholder="Название события" required />
        <input value={description} onChange={(e) => setDescription(e.target.value)} className={inputClass} placeholder="Описание" />
        <input type="datetime-local" value={time} onChange={(e) => setTime(e.target.value)} className={inputClass} required />
        <button className={primaryButton}>Создать событие</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-nc-line-strong">
        {items.map((ev) => (
          <li key={ev.id} className="flex items-center gap-2 py-2 text-sm">
            <div className="flex-1">
              <div className="font-medium">{ev.name}</div>
              <div className="text-xs text-nc-muted">{new Date(ev.startsAt).toLocaleString("ru-RU")} · {ev.interestedCount} интересно</div>
            </div>
            <button onClick={() => void toggleInterest(server.id, ev.id).then(reload)} className="text-sm text-nc-accent-text">
              {ev.interested ? "Не интересно" : "Интересно"}
            </button>
            <button onClick={() => void deleteEvent(server.id, ev.id).then(reload)} className="text-xs text-red-400">Удалить</button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function ForumTab({ server }: { server: Server }) {
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

function ModerationTab({ server }: { server: Server }) {
  const [bans, setBans] = useState<BanInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [targetUserId, setTargetUserId] = useState(0);
  const [targetReason, setTargetReason] = useState("");
  const [timeoutSeconds, setTimeoutSeconds] = useState(60);
  const reload = () => void listBans(server.id).then(setBans, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const doBan = async () => {
    if (!targetUserId || !targetReason.trim()) return;
    setPending(true);
    setError(null);
    try {
      await banMember(server.id, targetUserId, { reason: targetReason });
      setTargetReason("");
      await reload();
    } catch (err) { setError(errorMessage(err)); } finally { setPending(false); }
  };

  const doUnban = async (userId: number) => {
    if (!confirm(`Разбанить пользователя ID ${userId}?`)) return;
    setPending(true);
    setError(null);
    try {
      await unbanMember(server.id, userId);
      await reload();
    } catch (err) { setError(errorMessage(err)); } finally { setPending(false); }
  };

  const doKick = async () => {
    if (!targetUserId || !targetReason.trim()) return;
    setPending(true);
    setError(null);
    try {
      await kickMember(server.id, targetUserId, { reason: targetReason });
      setTargetReason("");
      await reload();
    } catch (err) { setError(errorMessage(err)); } finally { setPending(false); }
  };

  const doTimeout = async () => {
    if (!targetUserId) return;
    setPending(true);
    setError(null);
    try {
      await timeoutMember(server.id, targetUserId, timeoutSeconds);
      await reload();
    } catch (err) { setError(errorMessage(err)); } finally { setPending(false); }
  };

  const doClearTimeout = async (userId: number) => {
    setPending(true);
    setError(null);
    try {
      await unTimeoutMember(server.id, userId);
    } catch (err) { setError(errorMessage(err)); } finally { setPending(false); }
  };

  const durationOptions = [10, 30, 60, 300, 600, 1800, 3600, 21600, 86400];

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2 border-b border-nc-line-strong pb-2">
        <span className="text-sm font-semibold uppercase text-nc-muted">Управление участниками</span>
      </div>

      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}

      <div className="grid grid-cols-2 gap-2">
        <div>
          <label className="block text-sm text-nc-muted">ID пользователя</label>
          <input
            value={targetUserId}
            onChange={(e) => setTargetUserId(Number(e.target.value))}
            type="number"
            className={inputClass}
            placeholder="123456"
          />
        </div>
        <div>
          <label className="block text-sm text-nc-muted">Причина</label>
          <input
            value={targetReason}
            onChange={(e) => setTargetReason(e.target.value)}
            className={inputClass}
            placeholder="Причина действия"
          />
        </div>
      </div>

      <div className="flex gap-2 flex-wrap">
        <button disabled={pending} className={primaryButton} onClick={doBan}>
          {pending ? "Блокировка…" : "Забанить"}
        </button>
        <button disabled={pending} className={ghostButton} onClick={() => setTargetUserId(0)}>
          Разбан
        </button>
        <button disabled={pending} className={primaryButton} onClick={doKick}>
          {pending ? "Кик…" : "Кикнуть"}
        </button>
        <div className="flex items-center gap-2 ml-auto">
          <label className="text-sm text-nc-muted">Таймаут (сек):</label>
          <select
            value={timeoutSeconds}
            onChange={(e) => setTimeoutSeconds(Number(e.target.value))}
            className={inputClass}
          >
            {durationOptions.map((s) => (
              <option key={s} value={s}>{s >= 60 ? `${s / 60} мин` : `${s} сек`}</option>
            ))}
            <option value={0}>Снять</option>
          </select>
          <button disabled={pending} className={primaryButton} onClick={doTimeout}>
            Применить таймаут
          </button>
        </div>
      </div>

      <div>
        <div className="flex items-center gap-2 border-b border-nc-line-strong pb-2 mb-2">
          <span className="text-sm font-semibold uppercase text-nc-muted">Активные баны</span>
          <span className="text-xs text-nc-faint capitalize">{bans?.length ?? 0} шт.</span>
        </div>
        {bans === null ? (
          <p className="text-sm text-nc-muted">Загрузка…</p>
        ) : bans.length === 0 ? (
          <p className="text-sm text-nc-faint">Активных банов нет.</p>
        ) : (
          <ul className="divide-y divide-nc-line-strong text-sm">
            {bans.map((ban) => (
              <li key={ban.id} className="flex items-center gap-2 py-2">
                <span className="flex-1 truncate">:{ban.username}: — {ban.actorUsername} — {new Date(ban.createdAt).toLocaleString("ru-RU")}</span>
                <button
                  onClick={() => void doUnban(ban.userId)}
                  className="text-xs text-red-400 hover:underline"
                  aria-label={`Разбан @${ban.username}`}
                >
                  Разбан
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function TemplatesTab({ server }: { server: Server }) {
  const [template, setTemplate] = useState<ServerTemplate | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const exportTemplate = async () => {
    try {
      const t = await getTemplate(server.id);
      setTemplate(t);
      setText(JSON.stringify(t, null, 2));
    } catch (e) { setError(errorMessage(e)); }
  };
  const copy = () => void navigator.clipboard?.writeText(text);
  const importTemplate = async () => {
    try {
      const t = JSON.parse(text) as ServerTemplate;
      const created = await createFromTemplate(t);
      await loadServers();
      setError(null);
      window.location.href = `/channels/${created.id}`;
    } catch (e) { setError(errorMessage(e)); }
  };

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <button onClick={() => void exportTemplate()} className={primaryButton}>Экспортировать текущий сервер</button>
        {template && <button onClick={copy} className={ghostButton}>Копировать JSON</button>}
      </div>
      <textarea value={text} onChange={(e) => setText(e.target.value)} rows={12} className={inputClass} placeholder="Вставьте JSON шаблона, чтобы создать новый сервер" />
      <button onClick={() => void importTemplate()} className={primaryButton}>Создать сервер по шаблону</button>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

function StickersTab({ server }: { server: Server }) {
  type PackWithStickers = StickerPack & { stickers: Sticker[] };
  const [packs, setPacks] = useState<PackWithStickers[]>();
  const [selectedPackId, setSelectedPackId] = useState<number | null>(null);
  const [packError, setPackError] = useState<string | null>(null);
  const [packPending, setPackPending] = useState(false);
  const [newPackName, setNewPackName] = useState('');
  const [newPackDesc, setNewPackDesc] = useState('');
  const [stickerError, setStickerError] = useState<string | null>(null);
  const [stickerPending, setStickerPending] = useState(false);
  const [newStickerName, setNewStickerName] = useState('');
  const [newStickerEmoji, setNewStickerEmoji] = useState('');
  const [newStickerTags, setNewStickerTags] = useState('');
  const [newStickerFile, setNewStickerFile] = useState<File | null>(null);

  useEffect(() => {
    void loadPacks(server.id).catch((e) => setPackError(errorMessage(e)));
  }, [server.id]);

  const loadPacks = async (serverId: number) => {
    const packs = await listStickerPacks(serverId);
    // Only load basic pack info; stickers loaded on selection via useEffect below
    const packsWithEmptyStickers = packs.map(pack => ({ ...pack, stickers: [] as Sticker[] }));
    setPacks(packsWithEmptyStickers);
    if (packs.length > 0) setSelectedPackId(packs[0].id);
  };

  // Load stickers for the currently selected pack when it changes
  useEffect(() => {
    if (!selectedPackId || !packs) return;
    const currentPack = packs.find(p => p.id === selectedPackId);
    if (currentPack?.stickers !== undefined && currentPack.stickers.length > 0) return;
    void loadPacks(server.id).then(() => {
      // Already loaded above; just ensure we have stickers for this pack
    }).catch(() => {});
    void listPackStickers(server.id, selectedPackId)
      .then(stickers => {
        setPacks(prev => prev?.map(p =>
          p.id === selectedPackId ? { ...p, stickers } : p
        ));
      })
      .catch((e) => setPackError(errorMessage(e)));
  }, [selectedPackId, server.id]);

  const createPack = async () => {
    if (!newPackName.trim()) {
      setPackError("Введите название пака");
      return;
    }
    setPackPending(true);
    setPackError(null);
    try {
      const pack = await createStickerPack(server.id, {
        name: newPackName.trim(),
        description: newPackDesc.trim() || null,
      });
      const packWithStickers = { ...pack, stickers: [] } as PackWithStickers;
      setPacks(prev => {
        const updated = [...(prev ?? []), packWithStickers];
        if (!selectedPackId) setSelectedPackId(packWithStickers.id);
        return updated;
      });
      setNewPackName('');
      setNewPackDesc('');
    } catch (err) {
      setPackError(errorMessage(err));
    } finally {
      setPackPending(false);
    }
  };

  const deletePack = async (packId: number) => {
    if (!confirm(`Удалить пак "${packs?.find(p => p.id === packId)?.name}"?`)) return;
    try {
      await deleteStickerPack(server.id, packId);
      setPacks(prev => prev?.filter(p => p.id !== packId));
      if (selectedPackId === packId) {
        setSelectedPackId(packs?.[0].id || null);
      }
    } catch (err) {
      setPackError(errorMessage(err));
    }
  };

  const uploadSticker = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!selectedPackId) return;
    if (!newStickerName.trim()) {
      setStickerError("Введите название стикера");
      return;
    }
    if (!newStickerEmoji.trim()) {
      setStickerError("Введите emoji/короткое имя");
      return;
    }
    if (!newStickerFile) {
      setStickerError("Выберите файл");
      return;
    }
    if (newStickerFile.size > 10 * 1024 * 1024) {
      setStickerError("Файл слишком большой (макс. 10 МБ)");
      return;
    }
    if (!['image/png', 'image/webp'].includes(newStickerFile.type)) {
      setStickerError("Поддерживаются только PNG и WebP");
      return;
    }

    setStickerPending(true);
    setStickerError(null);
    try {
      const formData = new FormData();
      formData.append('name', newStickerName.trim());
      formData.append('emoji', newStickerEmoji.trim());
      formData.append('tags', newStickerTags.trim());
      formData.append('file', newStickerFile);

      const resp = await fetch(`/api/servers/${server.id}/stickers/packages/${selectedPackId}/stickers`, {
        method: 'POST',
        credentials: 'include',
        body: formData,
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const sticker = await resp.json();
      
      setPacks(prev => prev?.map(p => 
        p.id === selectedPackId ? { ...p, stickers: [...(p.stickers ?? []), sticker] } : p
      ));
      setNewStickerName('');
      setNewStickerEmoji('');
      setNewStickerTags('');
      setNewStickerFile(null);
    } catch (err) {
      setStickerError(errorMessage(err));
    } finally {
      setStickerPending(false);
    }
  };

  const deleteSticker = async (packId: number, stickerId: number) => {
    if (!confirm("Удалить стикер?")) return;
    try {
      await deleteSticker(server.id, stickerId);
      setPacks(prev => prev?.map(p => 
        p.id === packId ? { ...p, stickers: (p.stickers ?? []).filter(s => s.id !== stickerId) } : p
      ));
    } catch (err) {
      setStickerError(errorMessage(err));
    }
  };

  return (
    <div className="space-y-4">
      {/* Pack management */}
      <div className="space-y-2">
        <h3 className="mb-1 text-xs font-semibold uppercase text-nc-muted">
          Стикерпаки
        </h3>
        <form onSubmit={createPack} className="space-y-3">
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
              Название
            </span>
            <input
              name="name"
              value={newPackName}
              onChange={(e) => setNewPackName(e.target.value)}
              required
              minLength={2}
              maxLength={32}
              className={inputClass}
              placeholder="Emoji Pack"
            />
          </label>
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
              Описание
            </span>
            <input
              name="description"
              value={newPackDesc}
              onChange={(e) => setNewPackDesc(e.target.value)}
              maxLength={200}
              className={inputClass}
              placeholder="Краткое описание"
            />
          </label>
          <button type="submit" disabled={packPending} className={primaryButton}>
            {packPending ? "Создание…" : "Создать пак"}
          </button>
        </form>
        {packError && (
          <p role="alert" className="text-sm text-red-400">
            {packError}
          </p>
        )}
      </div>

      {/* Pack list */}
      <div className="space-y-2">
        {packs === undefined ? (
          <p className="text-nc-muted">Загрузка…</p>
        ) : packs.length === 0 ? (
          <p className="text-nc-muted">Пока нет стикерпаков.</p>
        ) : (
          <ul className="grid grid-cols-1 gap-2">
            {packs.map(pack => {
              const active = selectedPackId === pack.id;
              return (
                <button
                  key={pack.id}
                  onClick={() => setSelectedPackId(pack.id)}
                  className={`flex items-center gap-2 rounded px-2 py-1.5 text-left text-sm ${
                    active ? "bg-nc-hover text-white" : "text-nc-text-2 hover:bg-nc-hover/50"
                  }`}
                >
                  <div className="flex-1">{pack.name}</div>
                  <div className="text-xs text-fg/60">{pack.description || " — "}</div>
                  <div className="text-xs text-fg/60">{pack.stickers?.length ?? 0} стикеров</div>
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); deletePack(pack.id); }}
                    className="text-xs text-red-400 hover:underline"
                    aria-label={`Удалить пак ${pack.name}`}
                  >
                    Удалить
                  </button>
                </button>
              );
            })}
          </ul>
        )}
      </div>

      {/* Sticker management for selected pack */}
      {selectedPackId != null && packs?.find(p => p.id === selectedPackId) && (
        <div className="space-y-4">
          <div className="space-y-2">
            <h3 className="mb-1 text-xs font-semibold uppercase text-nc-muted">
              Стикеры в паке
            </h3>
            <form onSubmit={uploadSticker} className="space-y-3">
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  Имя стикера
                </span>
                <input
                  name="name"
                  value={newStickerName}
                  onChange={(e) => setNewStickerName(e.target.value)}
                  required
                  minLength={2}
                  maxLength={32}
                  className={inputClass}
                  placeholder="party_blob"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  Emoji/короткое имя (1-2 символа)
                </span>
                <input
                  name="emoji"
                  value={newStickerEmoji}
                  onChange={(e) => setNewStickerEmoji(e.target.value)}
                  maxLength={4}
                  className={inputClass}
                  placeholder="🎉"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  Теги (через запятую)
                </span>
                <input
                  name="tags"
                  value={newStickerTags}
                  onChange={(e) => setNewStickerTags(e.target.value)}
                  maxLength={100}
                  className={inputClass}
                  placeholder="party, celebration, blob"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  PNG или WebP (до 10 МБ)
                </span>
                <input
                  name="file"
                  type="file"
                  accept="image/png,image/webp"
                  required
                  onChange={(e) => {
                    const file = e.target?.files?.[0] || null;
                    setNewStickerFile(file);
                  }}
                  className="block w-full text-sm text-nc-text-2"
                />
              </label>
              <button type="submit" disabled={stickerPending} className={primaryButton}>
                {stickerPending ? "Загрузка…" : "Добавить стикер"}
              </button>
            </form>
            {stickerError && (
              <p role="alert" className="text-sm text-red-400">
                {stickerError}
              </p>
            )}
          </div>

          {packs.find(p => p.id === selectedPackId)?.stickers === undefined ? (
            <p className="text-nc-muted">Загрузка стикеров…</p>
          ) : (packs.find(p => p.id === selectedPackId)?.stickers?.length ?? 0) === 0 ? (
            <p className="text-nc-muted">Пока нет стикеров в этом паке.</p>
          ) : (
            <ul className="grid grid-cols-2 gap-2 sm:grid-cols-3">
              {(packs.find(p => p.id === selectedPackId)?.stickers ?? []).map(sticker => (
                <li key={sticker.id} className="flex items-center gap-2 rounded bg-nc-surface p-2">
                  <img src={`/storage/${sticker.storageKey}`} alt={sticker.name} className="h-6 w-6 object-contain" />
                  <span className="min-w-0 flex-1 truncate text-sm">:{sticker.emoji}:</span>
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); deleteSticker(selectedPackId, sticker.id); }}
                    className="text-xs text-red-400 hover:underline"
                    aria-label={`Удалить стикер ${sticker.name}`}
                  >
                    Удалить
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

function AutoModTab({ server }: { server: Server }) {
  const [rules, setRules] = useState<AutoModRule[]>([]);
  const [pattern, setPattern] = useState("");
  const [action, setAction] = useState("delete");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const reload = () => listAutoModRules(server.id).then(setRules, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (!pattern.trim() || pattern.trim().length < 2) return;
    try {
      await createAutoModRule(server.id, { pattern: pattern.trim(), action, note: note.trim() || null });
      setPattern(""); setAction("delete"); setNote(""); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };

  const remove = async (id: number) => {
    try { await deleteAutoModRule(server.id, id); await reload(); }
    catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-nc-muted">
        Автомодерация удаляет или предупреждает сообщения, содержащие указанные паттерны.
        Паттерн проверяется по RegEx (падение на Contains при ошибке). Требует разрешение «Управление ролями».
      </p>
      <form onSubmit={add} className="flex flex-col gap-2">
        <input value={pattern} onChange={(e) => setPattern(e.target.value)} className={inputClass} placeholder="Паттерн (2–100 символов)" minLength={2} maxLength={100} />
        <select value={action} onChange={(e) => setAction(e.target.value)} className={inputClass}>
          <option value="delete">Удалить</option>
          <option value="warn">Предупредить</option>
        </select>
        <input value={note} onChange={(e) => setNote(e.target.value)} className={inputClass} placeholder="Комментарий (необязательно)" />
        <button className={primaryButton}>Добавить правило</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-nc-line-strong">
        {rules.map((r) => (
          <li key={r.id} className="flex items-center gap-3 py-2 text-sm">
            <code className="bg-nc-elevated px-2 py-0.5 rounded font-mono text-xs flex-1 truncate">{r.pattern}</code>
            <span className={`text-xs px-2 py-0.5 rounded ${r.action === "delete" ? "bg-red-900/40 text-red-300" : "bg-yellow-900/40 text-yellow-300"}`}>{r.action === "delete" ? "удаление" : "предупреждение"}</span>
            {r.note && <span className="text-xs text-nc-faint max-w-[200px] truncate">{r.note}</span>}
            <button onClick={() => void remove(r.id)} className="text-xs text-red-400 hover:underline">Удалить</button>
          </li>
        ))}
        {rules.length === 0 && <li className="text-sm text-nc-faint py-2">Нет правил.</li>}
      </ul>
    </div>
  );
}

const BOOST_THRESHOLDS = [1, 3, 7];
const BOOST_NAMES = ["Нет бустов", "Уровень 1", "Уровень 2", "Уровень 3"];

function BoostsTab({ server }: { server: Server }) {
  const [info, setInfo] = useState<{ level: number; count: number; boosters: Array<{ user_id: number; username: string; boosted_at: number }> } | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [boosting, setBoosting] = useState(false);
  const reload = () => getServerBoosts(server.id).then(setInfo, (e) => setError(errorMessage(e))).finally(() => setLoading(false));
  useEffect(() => { void reload(); }, [server.id]);

  const doBoost = async () => {
    setBoosting(true);
    try { await toggleServerBoost(server.id); await reload(); }
    catch (err) { setError(errorMessage(err)); }
    finally { setBoosting(false); }
  };

  if (loading) return <p className="text-sm text-nc-muted py-4">Загрузка…</p>;
  if (error) return <p role="alert" className="text-sm text-red-400 py-4">{error}</p>;

  const hasBoosted = info!.count > 0;
  const nextThreshold = BOOST_THRESHOLDS[info!.level] ?? Infinity;
  const progress = info!.count / nextThreshold;

  return (
    <div className="space-y-5">
      <div className="bg-nc-elevated rounded-lg p-4 space-y-3">
        <div className="flex items-center gap-3">
          <span className="text-2xl">🚀</span>
          <div>
            <p className="font-semibold text-base">{BOOST_NAMES[Math.min(info!.level, BOOST_NAMES.length - 1)]}</p>
            <p className="text-xs text-nc-muted">{info!.count} буст{info!.count === 1 ? "" : info!.count < 5 ? "а" : "ов"}</p>
          </div>
        </div>
        <div>
          <div className="flex justify-between text-xs text-nc-muted mb-1">
            <span>Прогресс до следующего уровня</span>
            <span>{Math.min(Math.round(progress * 100), 100)}%</span>
          </div>
          <div className="h-2 bg-nc-hover rounded-full overflow-hidden">
            <div className="h-full bg-gradient-to-r from-violet-500 to-fuchsia-400 transition-all" style={{ width: `${Math.min(progress * 100, 100)}%` }} />
          </div>
          {nextThreshold !== Infinity && <p className="text-xs text-nc-faint mt-1">Следующий уровень: ещё {nextThreshold - info!.count} буст{nextThreshold - info!.count === 1 ? "" : nextThreshold - info!.count < 5 ? "а" : "ов"}</p>}
          {nextThreshold === Infinity && <p className="text-xs text-nc-faint mt-1">Максимальный уровень достигнут!</p>}
        </div>
        <button onClick={() => void doBoost()} disabled={boosting} className={`w-full py-2 rounded text-sm font-medium ${hasBoosted ? "bg-violet-600 hover:bg-violet-500 text-white" : "bg-nc-hover hover:bg-nc-active text-nc-text-2"}`}>
          {boosting ? "…" : hasBoosted ? "Убрать буст" : "Бустнуть сервер 🚀"}
        </button>
      </div>
      <div>
        <p className="text-xs text-nc-muted uppercase tracking-wide mb-2">Бустеры ({info!.boosters.length})</p>
        {info!.boosters.length === 0
          ? <p className="text-sm text-nc-faint">Пока нет бустеров.</p>
          : <ul className="space-y-1">
              {info!.boosters.map((b) => (
                <li key={b.user_id} className="flex items-center gap-2 text-sm py-1.5 px-2 rounded hover:bg-nc-elevated">
                  <span>🚀</span>
                  <span className="flex-1 truncate">{b.username}</span>
                  <span className="text-xs text-nc-faint">
                    {new Date(b.boosted_at * 1000).toLocaleDateString("ru-RU")}
                  </span>
                </li>
              ))}
            </ul>
        }
      </div>
      <div className="border-t border-nc-line-strong pt-4">
        <p className="text-xs text-nc-muted uppercase tracking-wide mb-2">Бонусы по уровням</p>
        <div className="grid grid-cols-2 gap-2 text-xs">
          {[
            ["0", "0 бустов", "Стандартные лимиты"],
            ["1", "1 буст", "+100 эмодзи, +50 слотов ролей"],
            ["2", "3 буста", "+250 эмодзи, +200 слотов ролей"],
            ["3", "7 бустов", "Макс. лимиты"],
          ].map(([lvl, label, perk]) => (
            <div key={lvl} className={`p-2 rounded ${parseInt(lvl) <= (info?.level ?? 0) ? "bg-violet-900/20 border border-violet-700/40" : "bg-nc-elevated opacity-50"}`}>
              <p className="font-medium text-nc-text-2">{label}</p>
              <p className="text-nc-muted">{perk}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}