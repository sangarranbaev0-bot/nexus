import { useEffect, useState, type FormEvent } from "react";
import { useNavigate } from "react-router";
import type { Invite } from "./generated/Invite";
import type { Member } from "./generated/Member";
import type { Server } from "./generated/Server";
import {
  createChannel,
  createInvite,
  createServer,
  joinInvite,
  listMembers,
  setMemberRole,
} from "./guilds";
import { openDm } from "./dms";
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
  const [kind, setKind] = useState<"text" | "voice">("text");
  const { onSubmit, error, pending } = useSubmit(async (value) => {
    const channel = await createChannel(server.id, value, kind);
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
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
            Название канала
          </span>
          <div className="flex items-center rounded bg-zinc-900 focus-within:ring-2 focus-within:ring-indigo-500">
            <span className="pl-3 text-zinc-500">
              {kind === "voice" ? "🔊" : "#"}
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
  const [error, setError] = useState<string | null>(null);
  const isOwner = server.myRole === "owner";

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
  }, [server.id]);

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
      <ul className="max-h-80 divide-y divide-zinc-700 overflow-y-auto">
        {members?.map((m) => (
          <li key={m.userId} className="flex items-center gap-3 py-2">
            <StatusDot userId={m.userId} fallback={m.status} />
            <span className="flex-1 truncate">
              {m.username}
              {m.userId === me?.id && (
                <span className="text-zinc-500"> (вы)</span>
              )}
            </span>
            <span className="text-sm text-zinc-400">{roleLabel[m.role]}</span>
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
  const { onSubmit, error, pending } = useSubmit(async (value) => {
    const dm = await openDm(value);
    onClose();
    navigate(`/channels/@me/${dm.id}`);
  });

  return (
    <Modal title="Написать сообщение" onClose={onClose}>
      <form onSubmit={onSubmit} className="space-y-4">
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">
            Имя пользователя
          </span>
          <input
            name="value"
            required
            autoFocus
            maxLength={32}
            placeholder="например, bob"
            className={inputClass}
          />
        </label>
        <ErrorText error={error} />
        <div className="flex justify-end gap-2">
          <button type="button" className={ghostButton} onClick={onClose}>
            Отмена
          </button>
          <button disabled={pending} className={primaryButton}>
            Открыть диалог
          </button>
        </div>
      </form>
    </Modal>
  );
}
