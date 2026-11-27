import { useEffect, useState, type FormEvent } from "react";
import { useNavigate } from "react-router";
import type { Channel } from "./generated/Channel";
import type { Member } from "./generated/Member";
import type { Overwrite } from "./generated/Overwrite";
import type { Server } from "./generated/Server";
import {
  deleteChannel,
  deleteOverwrite,
  listMembers,
  listOverwrites,
  loadCategories,
  loadRoles,
  updateChannel,
  upsertOverwrite,
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
  PERMISSION_GROUPS,
  colorToCss,
  hasPerm,
} from "./permissions";

type Tab = "overview" | "permissions";

export function ChannelSettingsDialog({
  server,
  channel,
  onClose,
}: {
  server: Server;
  channel: Channel;
  onClose: () => void;
}) {
  const [tab, setTab] = useState<Tab>("overview");
  return (
    <Modal
      title={`${channel.kind === "voice" ? "🔊" : "#"} ${channel.name}`}
      onClose={onClose}
    >
      <div role="tablist" className="flex gap-1 border-b border-nc-line-strong pb-2">
        {(
          [
            ["overview", "Обзор"],
            ["permissions", "Права"],
          ] as const
        ).map(([key, label]) => (
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
            {label}
          </button>
        ))}
      </div>
      {tab === "overview" ? (
        <OverviewTab server={server} channel={channel} onClose={onClose} />
      ) : (
        <PermissionsTab server={server} channel={channel} />
      )}
    </Modal>
  );
}

function OverviewTab({
  server,
  channel,
  onClose,
}: {
  server: Server;
  channel: Channel;
  onClose: () => void;
}) {
  const navigate = useNavigate();
  const categories = useGuilds((s) => s.categories[server.id]) ?? [];
  const [name, setName] = useState(channel.name);
  const [categoryId, setCategoryId] = useState(channel.categoryId ?? 0);
  const [topic, setTopic] = useState(channel.topic ?? '');
  const [slowmodeSeconds, setSlowmodeSeconds] = useState(channel.slowmodeSeconds ?? 0);
  const [nsfw, setNsfw] = useState(channel.nsfw ?? false);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    void loadCategories(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  const save = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setPending(true);
    setError(null);
    try {
      const trimmed = name.trim();
      await updateChannel(channel.id, {
        name: trimmed === channel.name ? null : trimmed,
        categoryId,
        position: null,
        topic: topic === (channel.topic ?? '') ? undefined : topic,
        slowmodeSeconds: slowmodeSeconds === (channel.slowmodeSeconds ?? 0) ? undefined : slowmodeSeconds,
        nsfw: nsfw === (channel.nsfw ?? false) ? undefined : nsfw,
      });
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const remove = async () => {
    if (!confirm(`Удалить канал ${channel.name}? Это необратимо.`)) return;
    setPending(true);
    setError(null);
    try {
      await deleteChannel(channel);
      navigate(`/channels/${server.id}`);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  return (
    <form onSubmit={save} className="space-y-4">
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Название канала
        </span>
        <input
          required
          maxLength={32}
          value={name}
          onChange={(e) => setName(e.target.value)}
          className={inputClass}
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Категория
        </span>
        <select
          value={categoryId}
          onChange={(e) => setCategoryId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Без категории</option>
          {[...categories]
            .sort((a, b) => a.position - b.position || a.id - b.id)
            .map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Тема
        </span>
        <input
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          className={inputClass}
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Медленный режим (сек.)
        </span>
        <input
          type="number"
          value={slowmodeSeconds}
          onChange={(e) => setSlowmodeSeconds(Number(e.target.value) || 0)}
          className={inputClass}
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          NSFW
        </span>
        <input
          type="checkbox"
          checked={nsfw}
          onChange={(e) => setNsfw(e.target.checked)}
          className={inputClass}
        />
      </label>
      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={remove}
          disabled={pending}
          className="rounded px-3 py-2 text-sm text-red-400 hover:bg-red-500/10 disabled:opacity-60"
        >
          Удалить канал
        </button>
        <div className="flex gap-2">
          <button type="button" className={ghostButton} onClick={onClose}>
            Отмена
          </button>
          <button disabled={pending} className={primaryButton}>
            Сохранить
          </button>
        </div>
      </div>
    </form>
  );
}

type Target = { kind: "role" | "user"; id: number };

function PermissionsTab({
  server,
  channel,
}: {
  server: Server;
  channel: Channel;
}) {
  const roles = useGuilds((s) => s.roles[server.id]);
  const [members, setMembers] = useState<Member[]>([]);
  const [overwrites, setOverwrites] = useState<Overwrite[] | null>(null);
  const [target, setTarget] = useState<Target | null>(null);
  const [allow, setAllow] = useState(0);
  const [deny, setDeny] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const reload = async () => {
    setOverwrites(await listOverwrites(channel.id));
  };

  useEffect(() => {
    void (async () => {
      try {
        const [list, memberList] = await Promise.all([
          listOverwrites(channel.id),
          listMembers(server.id),
          loadRoles(server.id),
          loadCategories(server.id),
        ]);
        setOverwrites(list);
        setMembers(memberList);
      } catch (e) {
        setError(errorMessage(e));
      }
    })();
  }, [channel.id, server.id]);

  useEffect(() => {
    if (!target && roles) {
      const everyone = roles.find((r) => r.isDefault);
      if (everyone) setTarget({ kind: "role", id: everyone.id });
    }
  }, [roles, target]);

  const current = target
    ? overwrites?.find((o) =>
        target.kind === "role" ? o.roleId === target.id : o.userId === target.id,
      )
    : undefined;

  useEffect(() => {
    setAllow(current?.allow ?? 0);
    setDeny(current?.deny ?? 0);
  }, [current]);

  const setBit = (bit: number, mode: "allow" | "deny" | "inherit") => {
    if (mode === "allow") {
      setAllow((a) => a | bit);
      setDeny((d) => d & ~bit);
    } else if (mode === "deny") {
      setDeny((d) => d | bit);
      setAllow((a) => a & ~bit);
    } else {
      setAllow((a) => a & ~bit);
      setDeny((d) => d & ~bit);
    }
  };

  const save = async () => {
    if (!target) return;
    setPending(true);
    setError(null);
    try {
      if (allow === 0 && deny === 0) {
        if (current) await deleteOverwrite(channel.id, current.id);
      } else if (target.kind === "role") {
        await upsertOverwrite(channel.id, {
          roleId: target.id,
          allow,
          deny,
        });
      } else {
        await upsertOverwrite(channel.id, {
          userId: target.id,
          allow,
          deny,
        });
      }
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const reset = async () => {
    if (!current) return;
    setPending(true);
    setError(null);
    try {
      await deleteOverwrite(channel.id, current.id);
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const memberName = (id: number) =>
    members.find((m) => m.userId === id)?.username ?? `#${id}`;
  const userOverwrites = (overwrites ?? []).filter((o) => o.userId !== null);

  const targetLabel =
    target?.kind === "role"
      ? (roles?.find((r) => r.id === target.id)?.name ?? "роль")
      : target
        ? memberName(target.id)
        : "";

  return (
    <div className="space-y-4">
      <section>
        <h3 className="mb-1 text-xs font-semibold uppercase text-nc-muted">
          Роли
        </h3>
        <div className="space-y-0.5">
          {[...(roles ?? [])]
            .sort((a, b) => b.position - a.position || a.id - b.id)
            .map((role) => {
              const active = target?.kind === "role" && target.id === role.id;
              const ow = overwrites?.find((o) => o.roleId === role.id);
              return (
                <button
                  key={role.id}
                  onClick={() => setTarget({ kind: "role", id: role.id })}
                  className={`flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm ${
                    active
                      ? "bg-nc-hover text-white"
                      : "text-nc-text-2 hover:bg-nc-hover/50"
                  }`}
                >
                  <span
                    className="size-3 shrink-0 rounded-full bg-nc-strong"
                    style={{ backgroundColor: colorToCss(role.color) }}
                    aria-hidden
                  />
                  <span className="flex-1 truncate">{role.name}</span>
                  {role.isDefault && (
                    <span className="text-xs text-nc-faint">по умолчанию</span>
                  )}
                  {ow && (
                    <span className="text-xs text-nc-accent-text">изменено</span>
                  )}
                </button>
              );
            })}
        </div>
      </section>

      <section>
        <h3 className="mb-1 text-xs font-semibold uppercase text-nc-muted">
          Участники
        </h3>
        {userOverwrites.length > 0 && (
          <div className="mb-1 space-y-0.5">
            {userOverwrites.map((o) => {
              const id = o.userId!;
              const active = target?.kind === "user" && target.id === id;
              return (
                <button
                  key={o.id}
                  onClick={() => setTarget({ kind: "user", id })}
                  className={`flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm ${
                    active
                      ? "bg-nc-hover text-white"
                      : "text-nc-text-2 hover:bg-nc-hover/50"
                  }`}
                >
                  <span className="flex-1 truncate">{memberName(id)}</span>
                  <span className="text-xs text-nc-accent-text">изменено</span>
                </button>
              );
            })}
          </div>
        )}
        <select
          value=""
          onChange={(e) => {
            if (e.target.value)
              setTarget({ kind: "user", id: Number(e.target.value) });
          }}
          aria-label="Добавить участника"
          className={inputClass}
        >
          <option value="">Добавить участника…</option>
          {members
            .filter((m) => !userOverwrites.some((o) => o.userId === m.userId))
            .map((m) => (
              <option key={m.userId} value={m.userId}>
                {m.username}
              </option>
            ))}
        </select>
      </section>

      {target && (
        <section className="border-t border-nc-line-strong pt-3">
          <div className="mb-2 flex items-baseline justify-between">
            <h3 className="font-semibold">{targetLabel}</h3>
            <span className="text-xs text-nc-faint">
              {current ? "переопределение" : "наследует права"}
            </span>
          </div>
          <div className="max-h-[45vh] space-y-2 overflow-y-auto pr-1">
            {PERMISSION_GROUPS.map((group) => (
              <fieldset key={group.title}>
                <legend className="mb-1 text-xs font-semibold uppercase text-nc-muted">
                  {group.title}
                </legend>
                <div className="space-y-0.5">
                  {group.items.map((item) => {
                    const value = hasPerm(allow, item.bit)
                      ? "allow"
                      : hasPerm(deny, item.bit)
                        ? "deny"
                        : "inherit";
                    return (
                      <div
                        key={item.bit}
                        className="flex items-center gap-2 rounded px-1 py-0.5 text-sm hover:bg-nc-hover/30"
                      >
                        <span className="flex-1 truncate" title={item.hint}>
                          {item.label}
                        </span>
                        <div
                          role="group"
                          aria-label={item.label}
                          className="flex shrink-0 overflow-hidden rounded border border-nc-line-max"
                        >
                          {(
                            [
                              ["allow", "✓", "Разрешить", "bg-emerald-600"],
                              ["inherit", "—", "Наследовать", "bg-nc-active"],
                              ["deny", "✕", "Запретить", "bg-red-600"],
                            ] as const
                          ).map(([mode, glyph, title, activeClass]) => (
                            <button
                              key={mode}
                              type="button"
                              title={title}
                              aria-label={title}
                              aria-pressed={value === mode}
                              onClick={() => setBit(item.bit, mode)}
                              className={`px-2 ${
                                value === mode
                                  ? `${activeClass} text-white`
                                  : "text-nc-muted hover:bg-nc-hover"
                              }`}
                            >
                              {glyph}
                            </button>
                          ))}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </fieldset>
            ))}
          </div>
          {error && (
            <p role="alert" className="mt-2 text-sm text-red-400">
              {error}
            </p>
          )}
          <div className="mt-3 flex items-center justify-between">
            <button
              type="button"
              onClick={reset}
              disabled={pending || !current}
              className="rounded px-3 py-2 text-sm text-nc-text-2 hover:underline disabled:opacity-40"
            >
              Сбросить
            </button>
            <button
              type="button"
              onClick={save}
              disabled={pending}
              className={primaryButton}
            >
              Сохранить
            </button>
          </div>
        </section>
      )}
      {error && !target && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}