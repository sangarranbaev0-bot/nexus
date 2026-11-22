import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useNavigate } from "react-router";
import type { Invite } from "./generated/Invite";
import type { Member } from "./generated/Member";
import type { OnboardingQuestion } from "./generated/OnboardingQuestion";
import type { ServerEvent } from "./generated/ServerEvent";
import type { Server } from "./generated/Server";
import {
  assignRoles,
  can,
  createChannel,
  createInvite,
    createServer,
    joinInvite,
    listInvites,
    deleteInvite,
    listMembers,
  loadRoles,
  setMemberProfile,
  listOnboarding,
  answerOnboarding,
  listEvents,
  toggleInterest,
  setMemberRole,
  useGuilds,
  wave,
} from "./guilds";
import { Markdown } from "./Markdown";
import { P, colorToCss } from "./permissions";
import { openDm, createGroup } from "./dms";
import {
  Modal,
  errorMessage,
  ghostButton,
  inputClass,
  primaryButton,
} from "./Modal";
import { presenceView, rememberStatuses, usePresence } from "./presence";
import { useSession } from "./session";
import type { Presence } from "./generated/Presence";

/** Общая форма с одним полем: отправка, ожидание, ошибка. */
function useSubmit(action: (value: string) => Promise<void>) {
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const value = String(new FormData(e.currentTarget).get("value") ?? "");
    setPending(true);
    setError(null);
    try {
      await action(value);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };
  return { onSubmit, error, pending };
}

function ErrorText({ error }: { error: string | null }) {
  return error ? (
    <p role="alert" className="text-sm text-red-400">
      {error}
    </p>
  ) : null;
}

export function AddServerDialog({ onClose }: { onClose: () => void }) {
  const [mode, setMode] = useState<"create" | "join">("create");
  const navigate = useNavigate();
  const { onSubmit, error, pending } = useSubmit(async (value) => {
    const server =
      mode === "create" ? await createServer(value) : await joinInvite(value);
    onClose();
    navigate(`/channels/${server.id}`);
  });

  return (
    <Modal
      title={mode === "create" ? "Создать сервер" : "Вступить на сервер"}
      onClose={onClose}
    >
      <form onSubmit={onSubmit} className="space-y-4">
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
            {mode === "create" ? "Название сервера" : "Код приглашения"}
          </span>
          <input
            key={mode}
            name="value"
            required
            autoFocus
            minLength={mode === "create" ? 2 : 1}
            maxLength={100}
            placeholder={
              mode === "create" ? "Мой сервер" : "например, hTKzmak2"
            }
            className={inputClass}
          />
        </label>
        <ErrorText error={error} />
        <div className="flex items-center justify-between">
          <button
            type="button"
            className="text-sm text-indigo-400 hover:underline"
            onClick={() => setMode(mode === "create" ? "join" : "create")}
          >
            {mode === "create" ? "Есть приглашение?" : "Создать свой сервер"}
          </button>
          <button disabled={pending} className={primaryButton}>
            {mode === "create" ? "Создать" : "Вступить"}
          </button>
        </div>
      </form>
    </Modal>
  );
}

export function CreateChannelDialog({
  server,
  onClose,
}: {
  server: Server;
  onClose: () => void;
}) {
  const navigate = useNavigate();
  const [kind, setKind] = useState<"text" | "voice" | "stage">("text");
  const [isForum, setIsForum] = useState(false);
  const { onSubmit, error, pending } = useSubmit(async (value) => {
    const channel = await createChannel(server.id, value, kind, isForum);
    onClose();
    navigate(`/channels/${server.id}/${channel.id}`);
  });

  return (
    <Modal title="Создать канал" onClose={onClose}>
      <form onSubmit={onSubmit} className="space-y-4">
        <fieldset className="grid grid-cols-2 gap-2">
          <legend className="mb-1.5 text-xs font-semibold uppercase text-zinc-400">
            Тип канала
          </legend>
          {(
            [
              ["text", "# Текстовый"],
              ["voice", "🔊 Голосовой"],
              ["stage", "🎤 Стади"],
            ] as const
          ).map(([k, title]) => (
            <label
              key={k}
              className={`cursor-pointer rounded p-3 ${kind === k ? "bg-zinc-700" : "bg-zinc-900"}`}
            >
              <input
                type="radio"
                name="kind"
                checked={kind === k}
                onChange={() => setKind(k)}
                className="mr-2"
              />
              {title}
            </label>
          ))}
        </fieldset>
        {kind === "text" && (
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={isForum}
              onChange={(e) => setIsForum(e.target.checked)}
              className="rounded bg-zinc-800 border-zinc-600"
            />
            <span className="text-sm">Форум-канал (ветки)</span>
          </label>
        )}
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
            Название канала
          </span>
          <div className="flex items-center rounded bg-zinc-900 focus-within:ring-2 focus-within:ring-indigo-500">
            <span className="pl-3 text-zinc-500">
              {kind === "stage" ? "🎤" : kind === "voice" ? "🔊" : "#"}
            </span>
            <input
              name="value"
              required
              autoFocus
              maxLength={32}
              placeholder="новый-канал"
              className="w-full bg-transparent px-2 py-2 outline-none"
            />
          </div>
        </label>
        <ErrorText error={error} />
        <div className="flex justify-end gap-2">
          <button type="button" className={ghostButton} onClick={onClose}>
            Отмена
          </button>
          <button disabled={pending} className={primaryButton}>
            Создать
          </button>
        </div>
      </form>
    </Modal>
  );
}

export function InviteDialog({
  server,
  onClose,
}: {
  server: Server;
  onClose: () => void;
}) {
  const [invite, setInvite] = useState<Invite | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [showRules, setShowRules] = useState(false);

  useEffect(() => {
    createInvite(server.id).then(setInvite, (err) =>
      setError(errorMessage(err)),
    );
  }, [server.id]);

  const copy = async () => {
    if (!invite) return;
    await navigator.clipboard.writeText(invite.code);
    setCopied(true);
  };

  return (
    <Modal title={`Пригласить на «${server.name}»`} onClose={onClose}>
      {server.bannerUrl && (
        <img
          src={server.bannerUrl}
          alt=""
          className="mb-3 h-28 w-full rounded object-cover"
        />
      )}
      {server.description && (
        <div className="mb-3 text-sm text-zinc-300">
          <Markdown text={server.description} />
        </div>
      )}
      {server.rules && (
        <button
          onClick={() => setShowRules((v) => !v)}
          className="mb-3 text-sm text-indigo-400 hover:underline"
        >
          {showRules ? "Скрыть правила" : "Показать правила"}
        </button>
      )}
      {showRules && server.rules && (
        <div className="mb-3 max-h-60 overflow-y-auto rounded bg-zinc-900 p-3 text-sm">
          <Markdown text={server.rules} />
        </div>
      )}
      <p className="text-sm text-zinc-400">
        Отправьте другу этот код, чтобы он вступил на сервер.
      </p>
      <div className="flex gap-2">
        <input
          readOnly
          value={invite?.code ?? "…"}
          aria-label="Код приглашения"
          className={`${inputClass} font-mono`}
        />
        <button onClick={copy} disabled={!invite} className={primaryButton}>
          {copied ? "Скопировано" : "Копировать"}
        </button>
      </div>
      <ErrorText error={error} />
    </Modal>
  );
}

const roleLabel = {
  owner: "Владелец",
  admin: "Админ",
  member: "Участник",
} as const;

export function MembersDialog({
  server,
  onClose,
}: {
  server: Server;
  onClose: () => void;
}) {
  const me = useSession((s) => s.user);
  const [members, setMembers] = useState<Member[] | null>(null);
  const [memberQuery, setMemberQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const isOwner = server.myRole === "owner";
  const manageRoles = can(server, P.MANAGE_ROLES);
  const roles = useGuilds((s) => s.roles[server.id]);
  const [rolesFor, setRolesFor] = useState<number | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [savingRoles, setSavingRoles] = useState(false);
  const [editProfile, setEditProfile] = useState(false);
  const [nickname, setNickname] = useState("");
  const [avatarUrl, setAvatarUrl] = useState("");

  const reload = () =>
    listMembers(server.id).then(
      (list) => {
        rememberStatuses(list);
        setMembers(list);
      },
      (err) => setError(errorMessage(err)),
    );
  useEffect(() => {
    void reload();
    void loadRoles(server.id).catch(() => {});
  }, [server.id]);

  const visibleMembers = useMemo(() => {
    const q = memberQuery.trim().toLowerCase();
    if (!q) return members ?? [];
    const rolesById = new Map((roles ?? []).map((r) => [r.id, r.name.toLowerCase()]));
    return (members ?? []).filter(
      (m) =>
        m.username.toLowerCase().includes(q) ||
        m.roleIds.some((id) => rolesById.get(id)?.includes(q)),
    );
  }, [members, memberQuery, roles]);

  const openRoles = (m: Member) => {
    const assignable = new Set(
      (roles ?? []).filter((r) => !r.isDefault).map((r) => r.id),
    );
    setSelected(m.roleIds.filter((id) => assignable.has(id)));
    setRolesFor(m.userId);
  };

  const saveRoles = async (member: Member) => {
    setSavingRoles(true);
    setError(null);
    try {
      await assignRoles(server.id, member.userId, selected);
      setRolesFor(null);
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSavingRoles(false);
    }
  };

  const navigate = useNavigate();
  const message = async (m: Member) => {
    try {
      const dm = await openDm(m.username);
      onClose();
      navigate(`/channels/@me/${dm.id}`);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const waveHello = async () => {
    setError(null);
    try {
      await wave(server.id);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const toggle = async (m: Member) => {
    setError(null);
    try {
      await setMemberRole(
        server.id,
        m.userId,
        m.role === "admin" ? "member" : "admin",
      );
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Modal title="Участники" onClose={onClose}>
      <div className="mb-3 flex gap-2">
        <input
          value={memberQuery}
          onChange={(e) => setMemberQuery(e.target.value)}
          placeholder="Поиск по имени или роли"
          aria-label="Поиск участников"
          className="flex-1 rounded bg-zinc-900 px-3 py-1.5 text-sm outline-none placeholder:text-zinc-500"
        />
        <button onClick={() => {
          const meMember = members?.find((m) => m.userId === me?.id);
          setNickname(meMember?.nickname ?? "");
          setAvatarUrl(meMember?.avatarUrl ?? "");
          setEditProfile(true);
        }} className="text-sm text-indigo-400 hover:underline">
          Мой ник
        </button>
        <button onClick={waveHello} className="text-sm text-indigo-400 hover:underline">
          👋 Помахать
        </button>
      </div>
      {editProfile && (
        <div className="mb-3 space-y-2 rounded bg-zinc-900 p-3">
          <input value={nickname} onChange={(e) => setNickname(e.target.value)} className={inputClass} placeholder="Ник на сервере" />
          <input value={avatarUrl} onChange={(e) => setAvatarUrl(e.target.value)} className={inputClass} placeholder="Ссылка на аватар" />
          <div className="flex gap-2">
            <button onClick={() => void setMemberProfile(server.id, { nickname, avatarUrl }).then(() => { setEditProfile(false); return reload(); }, (e) => setError(errorMessage(e)))} className={primaryButton}>Сохранить</button>
            <button onClick={() => setEditProfile(false)} className={ghostButton}>Отмена</button>
          </div>
        </div>
      )}
      <ul className="max-h-80 divide-y divide-zinc-700 overflow-y-auto">
        {visibleMembers.map((m) => (
          <li key={m.userId} className="py-2">
            <div className="flex items-center gap-3">
              <StatusDot userId={m.userId} fallback={m.status} />
              <span className="flex-1 truncate">
                {m.username}
                {m.userId === me?.id && (
                  <span className="text-zinc-500"> (вы)</span>
                )}
              </span>
              <span className="text-sm text-zinc-400">
                {roleLabel[m.role]}
              </span>
              {manageRoles && (m.role !== "owner" || isOwner) && (
                <button
                  onClick={() => openRoles(m)}
                  className="text-sm text-indigo-400 hover:underline"
                >
                  Роли
                </button>
              )}
              {m.userId !== me?.id && (
                <button
                  onClick={() => message(m)}
                  className="text-sm text-indigo-400 hover:underline"
                >
                  Написать
                </button>
              )}
              {isOwner && m.role !== "owner" && (
                <button
                  onClick={() => toggle(m)}
                  className="text-sm text-indigo-400 hover:underline"
                >
                  {m.role === "admin" ? "Снять админа" : "Сделать админом"}
                </button>
              )}
            </div>
            {m.roleIds.length > 0 && (
              <div className="mt-1 flex flex-wrap gap-1 pl-6">
                {m.roleIds
                  .map((id) => roles?.find((r) => r.id === id))
                  .filter((r) => r !== undefined)
                  .map((r) => (
                    <span
                      key={r.id}
                      style={{ color: colorToCss(r.color), borderColor: colorToCss(r.color) }}
                      className="rounded border-[3px] bg-zinc-700/70 px-1.5 py-0.5 text-xs"
                    >
                      {r.name}
                    </span>
                  ))}
              </div>
            )}
            {rolesFor === m.userId && (
              <div className="mt-2 ml-6 space-y-1 rounded bg-zinc-900 p-2">
                {(roles ?? []).filter((r) => !r.isDefault).length === 0 && (
                  <p className="text-sm text-zinc-500">
                    Сначала создайте роль в настройках сервера.
                  </p>
                )}
                {(roles ?? [])
                  .filter((r) => !r.isDefault)
                  .sort((a, b) => b.position - a.position || a.id - b.id)
                  .map((r) => (
                    <label
                      key={r.id}
                      className="flex items-center gap-2 text-sm"
                    >
                      <input
                        type="checkbox"
                        checked={selected.includes(r.id)}
                        onChange={(e) =>
                          setSelected((sel) =>
                            e.target.checked
                              ? [...sel, r.id]
                              : sel.filter((id) => id !== r.id),
                          )
                        }
                      />
                      <span style={{ color: colorToCss(r.color) }}>
                        {r.name}
                      </span>
                    </label>
                  ))}
                <div className="flex justify-end gap-3 pt-1">
                  <button
                    onClick={() => setRolesFor(null)}
                    className="text-sm text-zinc-300 hover:underline"
                  >
                    Отмена
                  </button>
                  <button
                    onClick={() => void saveRoles(m)}
                    disabled={savingRoles}
                    className="text-sm text-indigo-400 hover:underline disabled:opacity-60"
                  >
                    Сохранить
                  </button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
      <ErrorText error={error} />
    </Modal>
  );
}

/** Точка статуса; живое значение из событий, иначе то, что пришло со списком. */
export function StatusDot({
  userId,
  fallback,
}: {
  userId: number;
  fallback: Presence;
}) {
  const live = usePresence((s) => s[userId]);
  const view = presenceView[live ?? fallback];
  return (
    <span
      title={view.label}
      className={`size-2.5 shrink-0 rounded-full ${view.dot}`}
    >
      <span className="sr-only">{view.label}</span>
    </span>
  );
}

export function NewDmDialog({ onClose }: { onClose: () => void }) {
  const navigate = useNavigate();
  const [group, setGroup] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const f = new FormData(e.currentTarget);
    const value = String(f.get("value") ?? "");
    setPending(true);
    setError(null);
    try {
      const dm = group
        ? await createGroup(
            value
              .split(/[,\n]/)
              .map((u) => u.trim())
              .filter(Boolean),
            String(f.get("name") ?? ""),
          )
        : await openDm(value);
      onClose();
      navigate(`/channels/@me/${dm.id}`);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  return (
    <Modal title={group ? "Создать группу" : "Написать сообщение"} onClose={onClose}>
      <form onSubmit={onSubmit} className="space-y-4">
        <div className="grid grid-cols-2 gap-2" role="radiogroup" aria-label="Тип диалога">
          {(
            [
              [false, "👤 Личный"],
              [true, "👥 Группа"],
            ] as const
          ).map(([g, title]) => (
            <button
              key={title}
              type="button"
              role="radio"
              aria-checked={group === g}
              onClick={() => setGroup(g)}
              className={`rounded p-2 ${group === g ? "bg-zinc-700" : "bg-zinc-900"}`}
            >
              {title}
            </button>
          ))}
        </div>
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
            {group ? "Имена участников через запятую (2-9)" : "Имя пользователя"}
          </span>
          <input
            key={String(group)}
            name="value"
            required
            autoFocus
            maxLength={group ? 400 : 32}
            placeholder={group ? "bob, alice, kim" : "например, bob"}
            className={inputClass}
          />
        </label>
        {group && (
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
              Название группы (необязательно)
            </span>
            <input name="name" maxLength={100} className={inputClass} />
          </label>
        )}
        <ErrorText error={error} />
        <div className="flex justify-end gap-2">
          <button type="button" className={ghostButton} onClick={onClose}>
            Отмена
          </button>
          <button disabled={pending} className={primaryButton}>
            {group ? "Создать группу" : "Открыть диалог"}
          </button>
        </div>
      </form>
    </Modal>
  );
}


export function OnboardingDialog({ server, onClose }: { server: Server; onClose: () => void }) {
  const [items, setItems] = useState<OnboardingQuestion[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const reload = () => listOnboarding(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); void loadRoles(server.id).catch(() => {}); }, [server.id]);
  const roles = useGuilds((s) => s.roles[server.id]) ?? [];

  const answer = async (id: number) => {
    try { await answerOnboarding(server.id, id); await reload(); }
    catch (e) { setError(errorMessage(e)); }
  };

  return (
    <Modal title={`Онбординг · ${server.name}`} onClose={onClose}>
      <p className="mb-3 text-sm text-zinc-400">Ответьте «Да», чтобы получить роль. Повторно answered вопросы не показываются.</p>
      {error && <ErrorText error={error} />}
      {items === null ? (
        <p className="text-zinc-400">Загрузка…</p>
      ) : items.length === 0 ? (
        <p className="text-zinc-400">Вопросов пока нет.</p>
      ) : (
        <ul className="space-y-2">
          {items.map((q) => (
            <li key={q.id} className="flex items-center gap-2 rounded bg-zinc-900 p-3 text-sm">
              <span className="flex-1">{q.question}</span>
              <span className="text-xs text-zinc-400">{roles.find((r) => r.id === q.roleId)?.name}</span>
              {q.answered ? (
                <span className="text-xs text-emerald-400">получено</span>
              ) : (
                <button onClick={() => void answer(q.id)} className="text-sm text-indigo-400 hover:underline">Да</button>
              )}
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}

export function EventsDialog({ server, onClose }: { server: Server; onClose: () => void }) {
  const [items, setItems] = useState<ServerEvent[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const reload = () => listEvents(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const interest = async (id: number) => {
    try { await toggleInterest(server.id, id); await reload(); }
    catch (e) { setError(errorMessage(e)); }
  };

  return (
    <Modal title={`События · ${server.name}`} onClose={onClose}>
      {error && <ErrorText error={error} />}
      {items === null ? (
        <p className="text-zinc-400">Загрузка…</p>
      ) : items.length === 0 ? (
        <p className="text-zinc-400">Ближайших событий нет.</p>
      ) : (
        <ul className="space-y-2">
          {items.map((ev) => (
            <li key={ev.id} className="rounded bg-zinc-900 p-3">
              <div className="flex items-center justify-between gap-3">
                <span className="font-medium">{ev.name}</span>
                <span className="text-xs text-zinc-400">{new Date(ev.startsAt).toLocaleString("ru-RU")}</span>
              </div>
              {ev.description && <p className="mt-1 whitespace-pre-wrap text-sm text-zinc-400">{ev.description}</p>}
              <div className="mt-2 flex items-center gap-3 text-sm">
                <button onClick={() => void interest(ev.id)} className="text-indigo-400 hover:underline">
                  {ev.interested ? "✭ Интересно" : "☆ Интересно"}
                </button>
                <span className="text-zinc-500">{ev.interestedCount}</span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}

export function InviteListDialog({
  server,
  onClose,
}: {
  server: Server;
  onClose: () => void;
}) {
  const [invites, setInvites] = useState<Invite[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const reload = () => listInvites(server.id).then(setInvites, (e) => setError(errorMessage(e)));

  useEffect(() => { void reload(); }, [server.id]);

  const remove = async (code: string) => {
    try {
      await deleteInvite(code);
      setInvites((list) => list?.filter((i) => i.code !== code) ?? null);
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const copyCode = async (code: string) => {
    await navigator.clipboard.writeText(code);
  };

  return (
    <Modal title={`Приглашения · ${server.name}`} onClose={onClose}>
      {error && <ErrorText error={error} />}
      {invites === null ? (
        <p className="text-zinc-400">Загрузка…</p>
      ) : invites.length === 0 ? (
        <p className="text-zinc-400">Нет активных приглашений.</p>
      ) : (
        <ul className="space-y-2">
          {invites.map((inv) => (
            <li key={inv.code} className="flex items-center justify-between rounded bg-zinc-900 p-3 text-sm">
              <div className="min-w-0 flex-1">
                <span className="font-mono font-medium text-indigo-300">{inv.code}</span>
                {' · '}
                <span className="text-zinc-400">
                  использований: {inv.uses}{inv.maxUses !== null ? ` / ${inv.maxUses}` : ''}
                </span>
                {inv.expiresAt !== null && (
                  <span className="ml-2 text-xs text-zinc-500">
                    · до {new Date(inv.expiresAt * 1000).toLocaleString('ru-RU')}
                  </span>
                )}
              </div>
              <div className="ml-3 flex shrink-0 gap-2">
                <button
                  onClick={() => void copyCode(inv.code)}
                  className="rounded px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-700"
                >
                  Копировать
                </button>
                <button
                  onClick={() => void remove(inv.code)}
                  className="rounded px-2 py-1 text-xs text-red-400 hover:bg-red-500/20"
                >
                  Удалить
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}