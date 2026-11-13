import { useEffect, useState } from "react";
import { Link, NavLink, Navigate, useNavigate, useParams } from "react-router";
import { ChatView } from "./ChatView";
import {
  AddServerDialog,
  CreateChannelDialog,
  InviteDialog,
  MembersDialog,
  NewDmDialog,
  StatusDot,
} from "./Dialogs";
import { clearDmUnread, loadDms, useDms } from "./dms";
import { joinVoice, useVoice } from "./voice";
import { VoicePanel, VoiceRoster, VoiceStage } from "./VoiceView";
import type { DmChannel } from "./generated/DmChannel";
import type { Channel } from "./generated/Channel";
import type { Server } from "./generated/Server";
import {
  canManage,
  clearUnread,
  deleteChannel,
  formatBadge,
  loadCategories,
  loadChannels,
  loadServers,
  useGuilds,
} from "./guilds";
import { errorMessage } from "./Modal";
import { ServerSettingsDialog } from "./Settings";
import { presenceView } from "./presence";
import { logout, useSession } from "./session";
import { connectSocket, useSocket } from "./socket";

type Dialog =
  | "addServer"
  | "createChannel"
  | "invite"
  | "members"
  | "newDm"
  | "settings"
  | null;

export function AppLayout() {
  const params = useParams();
  const isHome = params.serverId === "@me";
  const serverId = isHome ? undefined : Number(params.serverId) || undefined;
  const channelId = Number(params.channelId) || undefined;
  const servers = useGuilds((s) => s.servers);
  const channels = useGuilds((s) =>
    serverId ? s.channels[serverId] : undefined,
  );
  const dms = useDms((s) => s.dms);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Телефон: серверы и каналы выезжают поверх чата; на md+ всегда на месте.
  const [navOpen, setNavOpen] = useState(false);
  useEffect(() => setNavOpen(false), [params.serverId, params.channelId]);
  useEffect(() => {
    if (!navOpen) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setNavOpen(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navOpen]);

  useEffect(() => connectSocket(), []);
  useEffect(() => {
    loadServers().catch((e) => setLoadError(errorMessage(e)));
    loadDms().catch((e) => setLoadError(errorMessage(e)));
  }, []);
  useEffect(() => {
    if (serverId && servers?.some((s) => s.id === serverId)) {
      loadChannels(serverId).catch((e) => setLoadError(errorMessage(e)));
      loadCategories(serverId).catch((e) => setLoadError(errorMessage(e)));
    }
  }, [serverId, servers]);

  const server = servers?.find((s) => s.id === serverId);
  const channel = channels?.find((c) => c.id === channelId);
  const dm = isHome ? dms?.find((d) => d.id === channelId) : undefined;
  const voiceLabel = useVoiceLabel();

  // Неполный или устаревший адрес ведём на первый доступный сервер и канал, без серверов — в ЛС.
  if (!isHome && servers) {
    if (!server)
      return (
        <Navigate
          to={
            servers.length > 0 ? `/channels/${servers[0].id}` : "/channels/@me"
          }
          replace
        />
      );
    if (channels && !channel && channels.length > 0) {
      return (
        <Navigate to={`/channels/${server.id}/${channels[0].id}`} replace />
      );
    }
  }
  if (isHome && channelId && dms && !dm)
    return <Navigate to="/channels/@me" replace />;

  return (
    <div className="flex h-dvh text-zinc-100">
      {navOpen && (
        <button
          aria-label="Закрыть меню"
          onClick={() => setNavOpen(false)}
          className="fixed inset-0 z-10 bg-black/60 md:hidden"
        />
      )}
      <div
        className={`fixed inset-y-0 left-0 z-20 flex transition-transform md:static md:translate-x-0 ${
          // invisible: закрытое меню не ловит Tab и не читается скринридером
          navOpen ? "translate-x-0" : "invisible -translate-x-full md:visible"
        }`}
      >
        <ServerRail
          servers={servers ?? []}
          activeId={serverId}
          homeActive={isHome}
          dmUnread={(dms ?? []).reduce((n, d) => n + d.unreadCount, 0)}
          onAdd={() => setDialog("addServer")}
        />

        <aside className="flex w-60 flex-col bg-zinc-900">
          {isHome ? (
            <DmList dms={dms ?? []} onNew={() => setDialog("newDm")} />
          ) : server ? (
            <ChannelList
              server={server}
              channels={channels ?? []}
              onDialog={setDialog}
            />
          ) : (
            <div className="flex-1" />
          )}
          <VoicePanel label={voiceLabel} />
          <ConnectionStatus />
        </aside>
      </div>

      <main className="flex min-w-0 flex-1 flex-col bg-zinc-800">
        {loadError && (
          <p role="alert" className="bg-red-900/60 px-4 py-2 text-sm">
            {loadError}
          </p>
        )}
        <header className="flex h-12 items-center gap-2 border-b border-zinc-900 px-4 font-semibold">
          <button
            onClick={() => setNavOpen(true)}
            aria-label="Открыть меню"
            aria-expanded={navOpen}
            className="-ml-2 rounded px-2 text-xl text-zinc-400 hover:text-zinc-100 md:hidden"
          >
            ☰
          </button>
          {channel && (
            <>
              <span className="text-zinc-500">
                {channel.kind === "voice" ? "🔊" : "#"}
              </span>
              {channel.name}
            </>
          )}
          {dm && (
            <>
              <span className="text-zinc-500">@</span>
              {dm.username}
              <StatusDot userId={dm.userId} fallback={dm.status} />
              <DmCallButton dm={dm} />
            </>
          )}
        </header>
        {server && channel?.kind === "voice" ? (
          <VoiceStage
            key={channel.id}
            channelId={channel.id}
            serverId={server.id}
            label={`🔊 ${channel.name}`}
          />
        ) : server && channel ? (
          <ChatView
            key={channel.id}
            channel={{ id: channel.id, label: `#${channel.name}` }}
            canModerate={canManage(server)}
            onRead={() => clearUnread(server.id, channel.id)}
          />
        ) : dm ? (
          <>
            <DmCall dm={dm} />
            <ChatView
              key={dm.id}
              channel={{ id: dm.id, label: `@${dm.username}` }}
              canModerate={false}
              onRead={() => clearDmUnread(dm.id)}
            />
          </>
        ) : isHome ? (
          <HomePlaceholder
            noServers={servers?.length === 0}
            onNewDm={() => setDialog("newDm")}
            onAddServer={() => setDialog("addServer")}
          />
        ) : (
          <section className="grid flex-1 place-items-center text-zinc-400">
            {server && channels?.length === 0 && "На сервере пока нет каналов."}
          </section>
        )}
      </main>

      {dialog === "addServer" && (
        <AddServerDialog onClose={() => setDialog(null)} />
      )}
      {dialog === "newDm" && <NewDmDialog onClose={() => setDialog(null)} />}
      {server && dialog === "createChannel" && (
        <CreateChannelDialog server={server} onClose={() => setDialog(null)} />
      )}
      {server && dialog === "invite" && (
        <InviteDialog server={server} onClose={() => setDialog(null)} />
      )}
      {server && dialog === "members" && (
        <MembersDialog server={server} onClose={() => setDialog(null)} />
      )}
      {server && dialog === "settings" && (
        <ServerSettingsDialog server={server} onClose={() => setDialog(null)} />
      )}
    </div>
  );
}

function HomePlaceholder({
  noServers,
  onNewDm,
  onAddServer,
}: {
  noServers: boolean;
  onNewDm: () => void;
  onAddServer: () => void;
}) {
  return (
    <div className="grid flex-1 place-items-center p-8 text-center">
      <div className="space-y-4">
        <p className="text-lg text-zinc-300">
          Выберите диалог слева или напишите кому-нибудь.
        </p>
        <div className="flex justify-center gap-2">
          <button
            onClick={onNewDm}
            className="rounded bg-indigo-500 px-4 py-2 font-medium hover:bg-indigo-600"
          >
            Написать
          </button>
          {noServers && (
            <button
              onClick={onAddServer}
              className="rounded bg-zinc-700 px-4 py-2 font-medium hover:bg-zinc-600"
            >
              Создать или вступить на сервер
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function DmList({ dms, onNew }: { dms: DmChannel[]; onNew: () => void }) {
  return (
    <>
      <header className="flex h-12 items-center border-b border-zinc-950 px-3">
        <h2 className="flex-1 truncate font-semibold">Личные сообщения</h2>
        <button
          onClick={onNew}
          title="Написать сообщение"
          aria-label="Написать сообщение"
          className="rounded px-1.5 text-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
        >
          +
        </button>
      </header>
      <ul className="flex-1 space-y-0.5 overflow-y-auto p-2">
        {dms.length === 0 && (
          <li className="px-2 py-1.5 text-sm text-zinc-500">
            Диалогов пока нет.
          </li>
        )}
        {dms.map((d) => (
          <li key={d.id}>
            <NavLink
              to={`/channels/@me/${d.id}`}
              aria-label={
                d.unreadCount
                  ? `${d.username}, непрочитанных: ${formatBadge(d.unreadCount)}`
                  : undefined
              }
              className={({ isActive }) =>
                `flex items-center gap-2 rounded px-2 py-1.5 ${
                  isActive
                    ? "bg-zinc-700/60 text-white"
                    : d.unreadCount > 0
                      ? "font-semibold text-white hover:bg-zinc-800"
                      : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
                }`
              }
            >
              <StatusDot userId={d.userId} fallback={d.status} />
              <span className="flex-1 truncate">{d.username}</span>
              <CallMarker channelId={d.id} />
              {d.unreadCount > 0 && (
                <span
                  aria-hidden
                  className="rounded-full bg-red-500 px-1.5 text-xs font-bold text-white"
                >
                  {formatBadge(d.unreadCount)}
                </span>
              )}
            </NavLink>
          </li>
        ))}
      </ul>
    </>
  );
}

function initials(name: string) {
  return name
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? "")
    .join("");
}

function ServerRail({
  servers,
  activeId,
  homeActive,
  dmUnread,
  onAdd,
}: {
  servers: Server[];
  activeId?: number;
  homeActive: boolean;
  dmUnread: number;
  onAdd: () => void;
}) {
  const pill = "grid size-12 place-items-center font-semibold transition-all";
  return (
    <nav
      aria-label="Серверы"
      className="flex w-[72px] flex-col items-center gap-2 overflow-y-auto bg-zinc-950 py-3"
    >
      <div className="relative">
        <Link
          to="/channels/@me"
          title="Личные сообщения"
          aria-label={
            dmUnread
              ? `Личные сообщения, непрочитанных: ${formatBadge(dmUnread)}`
              : "Личные сообщения"
          }
          aria-current={homeActive ? "page" : undefined}
          className={`${pill} text-xl ${
            homeActive
              ? "rounded-2xl bg-indigo-500"
              : "rounded-3xl bg-zinc-800 hover:rounded-2xl hover:bg-indigo-500"
          }`}
        >
          💬
        </Link>
        {dmUnread > 0 && (
          <span
            aria-hidden
            className="absolute -right-1 -bottom-1 rounded-full border-4 border-zinc-950 bg-red-500 px-1.5 text-xs font-bold"
          >
            {formatBadge(dmUnread)}
          </span>
        )}
      </div>
      <hr className="w-8 border-zinc-800" />
      {servers.map((s) => (
        <div key={s.id} className="relative">
          <Link
            to={`/channels/${s.id}`}
            title={s.name}
            aria-label={
              s.unreadCount
                ? `${s.name}, непрочитанных: ${formatBadge(s.unreadCount)}`
                : s.name
            }
            aria-current={s.id === activeId ? "page" : undefined}
            className={`${pill} ${
              s.id === activeId
                ? "rounded-2xl bg-indigo-500"
                : "rounded-3xl bg-zinc-800 hover:rounded-2xl hover:bg-indigo-500"
            }`}
          >
            {initials(s.name)}
          </Link>
          {s.unreadCount > 0 && (
            <span
              aria-hidden
              className="absolute -right-1 -bottom-1 rounded-full border-4 border-zinc-950 bg-red-500 px-1.5 text-xs font-bold"
            >
              {formatBadge(s.unreadCount)}
            </span>
          )}
        </div>
      ))}
      <button
        onClick={onAdd}
        title="Добавить сервер"
        aria-label="Добавить сервер"
        className={`${pill} rounded-3xl bg-zinc-800 text-2xl text-emerald-500 hover:rounded-2xl hover:bg-emerald-600 hover:text-white`}
      >
        +
      </button>
    </nav>
  );
}

function ChannelList({
  server,
  channels,
  onDialog,
}: {
  server: Server;
  channels: Channel[];
  onDialog: (d: Dialog) => void;
}) {
  const navigate = useNavigate();
  const manage = canManage(server);
  const categories = useGuilds((s) => s.categories[server.id]) ?? [];
  const iconButton =
    "rounded px-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100";

  const remove = async (c: Channel) => {
    if (!confirm(`Удалить канал #${c.name}? Это необратимо.`)) return;
    try {
      await deleteChannel(c);
      navigate(`/channels/${server.id}`);
    } catch (e) {
      alert(errorMessage(e));
    }
  };

  const sortedCategories = [...categories].sort(
    (a, b) => a.position - b.position || a.id - b.id,
  );
  const byCategory = new Map<number | null, Channel[]>();
  byCategory.set(null, []);
  for (const category of sortedCategories) byCategory.set(category.id, []);
  for (const channel of [...channels].sort(
    (a, b) => a.position - b.position || a.id - b.id,
  )) {
    const key =
      channel.categoryId !== null && byCategory.has(channel.categoryId)
        ? channel.categoryId
        : null;
    byCategory.get(key)!.push(channel);
  }

  return (
    <>
      <header className="flex h-12 items-center gap-1 border-b border-zinc-950 px-3">
        <h2 className="flex-1 truncate font-semibold">{server.name}</h2>
        {manage && (
          <button
            className={iconButton}
            title="Настройки сервера"
            aria-label="Настройки сервера"
            onClick={() => onDialog("settings")}
          >
            ⚙️
          </button>
        )}
        <button
          className={iconButton}
          title="Участники"
          aria-label="Участники"
          onClick={() => onDialog("members")}
        >
          👥
        </button>
        <button
          className={iconButton}
          title="Пригласить"
          aria-label="Пригласить"
          onClick={() => onDialog("invite")}
        >
          ✉️
        </button>
      </header>
      <div className="flex-1 overflow-y-auto px-2 pb-3">
        {channels.length === 0 && (
          <p className="px-2 py-3 text-sm text-zinc-500">Каналов пока нет.</p>
        )}
        <div className="space-y-0.5">
          {(byCategory.get(null) ?? []).map((channel) => (
            <ChannelRow
              key={channel.id}
              serverId={server.id}
              channel={channel}
              manage={manage}
              onDelete={remove}
            />
          ))}
        </div>
        {sortedCategories.map((category) => (
          <section key={category.id} className="mt-3">
            <h3 className="truncate px-2 pt-2 pb-1 text-xs font-semibold uppercase text-zinc-400">
              {category.name}
            </h3>
            <div className="space-y-0.5">
              {(byCategory.get(category.id) ?? []).map((channel) => (
                <ChannelRow
                  key={channel.id}
                  serverId={server.id}
                  channel={channel}
                  manage={manage}
                  onDelete={remove}
                />
              ))}
            </div>
          </section>
        ))}
        {manage && (
          <button
            onClick={() => onDialog("createChannel")}
            className="mt-3 w-full rounded px-2 py-1.5 text-left text-sm text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
          >
            + Создать канал
          </button>
        )}
      </div>
    </>
  );
}

function ChannelRow({
  serverId,
  channel,
  manage,
  onDelete,
}: {
  serverId: number;
  channel: Channel;
  manage: boolean;
  onDelete: (c: Channel) => void;
}) {
  return (
    <div className="group relative">
      <NavLink
        to={`/channels/${serverId}/${channel.id}`}
        onClick={() => {
          if (
            channel.kind === "voice" &&
            useVoice.getState().current?.channelId !== channel.id
          )
            void joinVoice(channel.id, serverId);
        }}
        aria-label={
          channel.unreadCount
            ? `#${channel.name}, непрочитанных: ${formatBadge(channel.unreadCount)}`
            : undefined
        }
        className={({ isActive }) =>
          `flex items-center rounded px-2 py-1.5 pr-7 ${
            isActive
              ? "bg-zinc-700/60 text-white"
              : channel.unreadCount > 0
                ? "font-semibold text-white hover:bg-zinc-800"
                : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
          }`
        }
      >
        <span className="mr-1.5 text-zinc-500">
          {channel.kind === "voice" ? "🔊" : "#"}
        </span>
        <span className="flex-1 truncate">{channel.name}</span>
        {channel.unreadCount > 0 && (
          <span
            aria-hidden
            className={`ml-1 rounded-full bg-red-500 px-1.5 text-xs font-bold text-white ${manage ? "group-hover:invisible" : ""}`}
          >
            {formatBadge(channel.unreadCount)}
          </span>
        )}
      </NavLink>
      {manage && (
        <button
          onClick={() => onDelete(channel)}
          title={`Удалить #${channel.name}`}
          aria-label={`Удалить канал ${channel.name}`}
          className="absolute top-1/2 right-1 -translate-y-1/2 rounded px-1.5 text-zinc-400 hover:text-red-400 md:opacity-0 md:group-hover:opacity-100 md:focus:opacity-100"
        >
          ×
        </button>
      )}
      {channel.kind === "voice" && <VoiceRoster channelId={channel.id} />}
    </div>
  );
}

/** Подпись текущего звонка для панели: «🔊 голосовой / Сервер» или «@имя». */
function useVoiceLabel() {
  const current = useVoice((s) => s.current);
  const channels = useGuilds((s) => s.channels);
  const servers = useGuilds((s) => s.servers);
  const dms = useDms((s) => s.dms);
  if (!current) return "";
  if (current.serverId === null)
    return `@${dms?.find((d) => d.id === current.channelId)?.username ?? "звонок"}`;
  const ch = channels[current.serverId]?.find(
    (c) => c.id === current.channelId,
  );
  const sv = servers?.find((x) => x.id === current.serverId);
  return `🔊 ${ch?.name ?? ""} / ${sv?.name ?? ""}`;
}

/** 📞 у диалога, где идёт звонок без меня. */
function CallMarker({ channelId }: { channelId: number }) {
  const ringing = useVoice(
    (s) =>
      Object.keys(s.participants[channelId] ?? {}).length > 0 &&
      s.current?.channelId !== channelId,
  );
  return ringing ? (
    <span
      title="Идёт звонок"
      aria-label="идёт звонок"
      className="animate-pulse"
    >
      📞
    </span>
  ) : null;
}

function DmCallButton({ dm }: { dm: DmChannel }) {
  const inThisCall = useVoice((s) => s.current?.channelId === dm.id);
  if (inThisCall) return null;
  return (
    <button
      onClick={() => void joinVoice(dm.id, null)}
      title="Позвонить"
      aria-label={`Позвонить ${dm.username}`}
      className="ml-auto rounded px-2 py-1 text-lg hover:bg-zinc-700"
    >
      📞
    </button>
  );
}

/** Звонок в диалоге: сцена над перепиской, пока в звонке кто-то есть. */
function DmCall({ dm }: { dm: DmChannel }) {
  const active = useVoice(
    (s) => Object.keys(s.participants[dm.id] ?? {}).length > 0,
  );
  if (!active) return null;
  return (
    <VoiceStage
      channelId={dm.id}
      serverId={null}
      label={`@${dm.username}`}
      compact
    />
  );
}

const connectionView = {
  connecting: { dot: "bg-amber-500", label: "Подключение…" },
  closed: { dot: "bg-red-500", label: "Нет соединения" },
} as const;

function ConnectionStatus() {
  const { status, latencyMs, idle } = useSocket();
  const user = useSession((s) => s.user);
  const view =
    status === "open"
      ? presenceView[idle ? "idle" : "online"]
      : connectionView[status];
  return (
    <footer className="flex items-center gap-2 bg-zinc-950/60 px-3 py-2 text-sm">
      <span
        className={`size-2.5 shrink-0 rounded-full ${view.dot}`}
        aria-hidden
      />
      <div className="min-w-0 flex-1" role="status">
        <div className="truncate font-medium">{user?.username}</div>
        <div className="text-xs text-zinc-400">
          {view.label}
          {latencyMs !== null && ` · ${latencyMs} мс`}
        </div>
      </div>
      <button
        onClick={() => void logout()}
        className="rounded px-2 py-1 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
      >
        Выйти
      </button>
    </footer>
  );
}
