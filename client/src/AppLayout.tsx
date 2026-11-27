import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { Link, NavLink, Navigate, useNavigate, useParams } from "react-router";
import { ChatView } from "./ChatView";
import { ChannelSettingsDialog } from "./ChannelSettings";
import {
  AddServerDialog,
  CreateChannelDialog,
  InviteDialog,
  InviteListDialog,
  MembersDialog,
  NewDmDialog,
  OnboardingDialog,
  EventsDialog,
  StatusDot,
} from "./Dialogs";
import { Menu, BellOff, LinkIcon, Search, MessageCircle, Settings, Users, ClipboardList, CalendarDays, Mail } from "lucide-react";
import { clearDmUnread, dmTitle, loadDms, useDms } from "./dms";
import { FriendsPage } from "./FriendsPage";
import { GroupActions, GroupHeader } from "./Groups";
import { joinVoice, useVoice } from "./voice";
import { VoicePanel, VoiceRoster, VoiceStage } from "./VoiceView";
import type { DmChannel } from "./generated/DmChannel";
import type { Channel } from "./generated/Channel";
import type { Server } from "./generated/Server";
import {
  can,
  clearUnread,
  deleteChannel,
  formatBadge,
  loadCategories,
  loadChannels,
  loadServers,
  updateServer,
  useGuilds,
} from "./guilds";
import { ContextMenu } from "./ContextMenu";
import { P } from "./permissions";
import { Modal, errorMessage } from "./Modal";
import { NotificationCenter } from "./NotificationCenter";
import { NotifyControls } from "./NotifyControls";
import { UserSettingsDialog } from "./UserSettings";
import { NotifyWordsDialog } from "./NotifyWords";
import { loadNotifications, loadNotifySettings } from "./notify";
import { ServerSettingsDialog } from "./Settings";
import { SearchDialog } from "./SearchDialog";
import { presenceView } from "./presence";
import { logout, useSession } from "./session";
import { useAccounts, type Account } from "./auth";
import { setAccessToken } from "./api";
import { connectSocket, useSocket } from "./socket";
import { loadAllEmojis } from "./emojis";

type Dialog =
  | "addServer"
  | "createChannel"
  | "invite"
  | "members"
  | "newDm"
  | "settings"
  | "userSettings"
  | "onboarding"
  | "events"
  | "notify"
  | "inviteList"
  | "search"
  | null;

export function AppLayout() {
  const params = useParams();
  const isHome = params.serverId === "@me";
  const serverId = isHome ? undefined : Number(params.serverId) || undefined;
  const channelId = Number(params.channelId) || undefined;
  const servers = useGuilds((s) => s.servers);
  const navigate = useNavigate();
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
    loadNotifySettings().catch(() => {});
    loadNotifications().catch(() => {});
  }, []);
  useEffect(() => {
    if (servers) loadAllEmojis(servers.map((s) => s.id)).catch(() => {});
  }, [servers]);
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

  useEffect(() => {
    const unread = servers?.reduce((sum, s) => sum + s.unreadCount, 0) ?? 0;
    const total = unread + (dms?.reduce((sum, d) => sum + d.unreadCount, 0) ?? 0);
    document.title = total ? `(${total}) NexusChat` : "NexusChat";
  }, [servers, dms]);

   useEffect(() => {
     const onKey = (e: KeyboardEvent) => {
       if (!e.ctrlKey || e.altKey) return;
       if (e.key >= "1" && e.key <= "9") {
         const target = (servers ?? [])[Number(e.key) - 1];
         if (target) {
           e.preventDefault();
           navigate(`/channels/${target.id}`);
         }
       } else if (e.key.toLowerCase() === "n" && serverId && server) {
         e.preventDefault();
         setDialog("createChannel");
       } else if (e.key.toLowerCase() === "g" && serverId && server) {
         e.preventDefault();
         setDialog("search");
       }
     };
     window.addEventListener("keydown", onKey);
     return () => window.removeEventListener("keydown", onKey);
   }, [servers, serverId, server, navigate]);

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
          <ConnectionStatus onSettings={() => setDialog("userSettings")} />
        </aside>
      </div>

      <main className="flex min-w-0 flex-1 flex-col bg-[var(--nc-bg-primary)]">
        {loadError && (
          <p role="alert" className="bg-red-900/60 px-4 py-2 text-sm">
            {loadError}
          </p>
        )}
        <header className="nc-header">
          <button
            onClick={() => setNavOpen(true)}
            aria-label="Открыть меню"
            aria-expanded={navOpen}
            className="nc-ib md:hidden"
          >
            <Menu size={18} />
          </button>
          {channel && (
            <>
              <span className="nc-header-icon">
                {channel.kind === "stage" ? "🎤" : channel.kind === "voice" ? "🔊" : "#"}
              </span>
              {channel.name}
            </>
          )}
          {dm && dm.isGroup && (
            <>
              <GroupHeader dm={dm} />
              <DmCallButton dm={dm} />
              <GroupActions dm={dm} />
            </>
          )}
          {dm && !dm.isGroup && (
            <>
         <span className="text-[var(--nc-text-muted)]">@</span>
              {dm.username}
              <StatusDot userId={dm.userId} fallback={dm.status} />
              <DmCallButton dm={dm} />
            </>
          )}
          {isHome && !dm && <span>Друзья</span>}
          <div className="ml-auto flex items-center gap-1">
            <NotificationCenter />
            {server && (
              <>
                <button
                  onClick={() => setDialog("notify")}
                  title="Настройки уведомлений"
                  aria-label="Настройки уведомлений канала"
                  className="nc-ib"
                >
                  <BellOff size={16} />
                </button>
                <button
                  onClick={() => setDialog("inviteList")}
                  title="Список приглашений"
                  aria-label="Приглашения"
                  className="nc-ib"
                >
                  <LinkIcon size={16} />
                </button>
                <button
                  onClick={() => setDialog("search")}
                  title="Поиск по серверу (Ctrl+G)"
                  aria-label="Глобальный поиск"
                  className="nc-ib"
                >
                  <Search size={16} />
                </button>
              </>
            )}
          </div>
        </header>
        {server?.bannerUrl && (
          <img
            src={server.bannerUrl}
            alt=""
            className="h-24 w-full object-cover opacity-80"
          />
        )}
        <div className={`flex min-h-0 flex-1 ${server && channel ? 'nc-anim-fade-in' : ''}`}>
        {server && channel?.kind === "voice" ? (
          <div className="flex min-h-0 flex-1">
            <VoiceStage
              key={channel.id}
              channelId={channel.id}
              serverId={server.id}
              label={`🔊 ${channel.name}`}
            />
            {(() => {
              const chatChannel = channels?.find((c) => c.id === server.systemChannelId && c.kind === 'text')
                ?? channels?.find((c) => c.kind === 'text');
              if (!chatChannel) return null;
              return (
                <aside className="hidden w-80 shrink-0 border-l border-[var(--nc-border-light)] lg:block" aria-label="Чат голосового канала">
                  <ChatView
                    key={chatChannel.id}
                    channel={{ id: chatChannel.id, label: `#${chatChannel.name}` }}
                    canModerate={can(server, P.MANAGE_MESSAGES)}
                    canPin={can(server, P.PIN_MESSAGES)}
                    onRead={() => clearUnread(server.id, chatChannel.id)}
                  />
                </aside>
              );
            })()}
          </div>
        ) : server && channel ? (
          <ChatView
            key={channel.id}
                    channel={{ id: channel.id, label: `#${channel.name}`, unread: channel.unreadCount, isForum: channel.isForum }}
            canModerate={can(server, P.MANAGE_MESSAGES)}
            canPin={can(server, P.PIN_MESSAGES)}
            onRead={() => clearUnread(server.id, channel.id)}
          />
        ) : dm ? (
          <>
            <DmCall dm={dm} />
            <ChatView
              key={dm.id}
              channel={{ id: dm.id, label: dmTitle(dm), unread: dm.unreadCount }}
              canModerate={false}
              canPin={true}
              onRead={() => clearDmUnread(dm.id)}
            />
          </>
        ) : isHome ? (
          <FriendsPage />
        ) : (
          <section className="grid flex-1 place-items-center text-zinc-400">
            {server && channels?.length === 0 && "На сервере пока нет каналов."}
          </section>
        )}
        </div>
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
        {server && dialog === "inviteList" && (
          <InviteListDialog server={server} onClose={() => setDialog(null)} />
        )}
        {server && dialog === "onboarding" && (
          <OnboardingDialog server={server} onClose={() => setDialog(null)} />
        )}
        {server && dialog === "events" && (
          <EventsDialog server={server} onClose={() => setDialog(null)} />
        )}
      {dialog === "userSettings" && <UserSettingsDialog onClose={() => setDialog(null)} />}
      {server && dialog === "settings" && (
        <ServerSettingsDialog server={server} onClose={() => setDialog(null)} />
      )}
      {server && dialog === "notify" && (
        <Modal
          title={`Уведомления · ${channel ? `#${channel.name}` : server.name}`}
          onClose={() => setDialog(null)}
        >
          <NotifyControls
            target={channel ? { channelId: channel.id } : { serverId: server.id }}
            inheritHint={
              channel
                ? `Для этого канала. Если не менять — действует настройка сервера «${server.name}».`
                : undefined
            }
          />
        </Modal>
      )}
      {server && dialog === "search" && (
        <SearchDialog serverId={server.id} onClose={() => setDialog(null)} />
      )}
    </div>
  );
}

function DmList({ dms, onNew }: { dms: DmChannel[]; onNew: () => void }) {
  return (
    <>
      <header className="nc-header">
        <h2 className="flex-1 truncate font-semibold">Личные сообщения</h2>
        <button
          onClick={onNew}
          title="Написать сообщение"
          aria-label="Написать сообщение"
          className="nc-ib"
        >
          +
        </button>
      </header>
      <ul className="flex-1 overflow-y-auto p-2 space-y-0.5">
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
                  ? `${dmTitle(d)}, непрочитанных: ${formatBadge(d.unreadCount)}`
                  : undefined
              }
              className={({ isActive }) =>
                `nc-ch flex items-center gap-2 ${
                  isActive
                    ? "text-white"
                    : d.unreadCount > 0
                      ? "font-semibold text-white"
                      : "text-[var(--nc-text-muted)]"
                }`
              }
            >
              <StatusDot userId={d.userId} fallback={d.status} />
              <span className="flex-1 truncate">{dmTitle(d)}</span>
              <CallMarker channelId={d.id} />
              {d.unreadCount > 0 && (
                <span
                  aria-hidden
                  className="nc-badge"
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
  const [folders, setFolders] = useState<Record<string, number[]>>(() => {
    try { return JSON.parse(localStorage.getItem('nexus-folders') ?? '{}'); } catch { return {}; }
  });
  const drag = useRef<number | null>(null);
  const [ctx, setCtx] = useState<
    { x: number; y: number; kind: "server" | "folder"; id?: number; name?: string } | null
  >(null);
  const persist = (next: Record<string, number[]>) => {
    setFolders(next);
    localStorage.setItem('nexus-folders', JSON.stringify(next));
  };
  const dropInto = (folder: string) => {
    const id = drag.current;
    if (!id) return;
    const next = { ...folders };
    for (const key of Object.keys(next)) next[key] = next[key].filter((x) => x !== id);
    if (!next[folder]) next[folder] = [];
    if (!next[folder].includes(id)) next[folder].push(id);
    persist(next);
    drag.current = null;
  };
  const unassign = () => {
    const id = drag.current;
    if (!id) return;
    const next = { ...folders };
    for (const key of Object.keys(next)) next[key] = next[key].filter((x) => x !== id);
    persist(next);
    drag.current = null;
  };
  const assigned = new Set(Object.values(folders).flat());
  const createFolder = () => {
    const name = prompt('Название папки');
    if (name && !folders[name]) persist({ ...folders, [name]: [] });
  };
  const renameServer = async (id: number) => {
    const name = prompt('Название сервера');
    if (!name) return;
    try { await updateServer(id, { name }); }
    catch (e) { alert(errorMessage(e)); }
  };
  const renameFolder = () => {
    if (!ctx?.name) return;
    const next = { ...folders };
    next[ctx.name] = next[ctx.name] ?? [];
    const name = prompt('Название папки', ctx.name);
    if (name && name !== ctx.name) {
      if (next[name]) { alert('Папка с таким названием уже есть'); return; }
      next[name] = next[ctx.name];
      delete next[ctx.name];
      persist(next);
    }
  };
  const deleteFolder = () => {
    if (!ctx?.name) return;
    const next = { ...folders };
    for (const id of next[ctx.name] ?? []) {
      for (const key of Object.keys(next)) next[key] = next[key].filter((x) => x !== id);
    }
    delete next[ctx.name];
    persist(next);
  };
  const serverPill = (s: Server) => (
    <div
      key={s.id}
      className="relative"
      draggable
      onContextMenu={(e) => { e.preventDefault(); setCtx({ x: e.clientX, y: e.clientY, kind: 'server', id: s.id }); }}
      onDragStart={() => { drag.current = s.id; }}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        e.preventDefault();
        const from = drag.current;
        if (!from || from === s.id) return;
        const next = { ...folders };
        for (const key of Object.keys(next)) {
          const i = next[key].indexOf(from);
          const j = next[key].indexOf(s.id);
          if (i >= 0 && j >= 0) [next[key][i], next[key][j]] = [next[key][j], next[key][i]];
        }
        persist(next);
        drag.current = null;
      }}
    >
      <Link
        to={`/channels/${s.id}`}
        title={s.name}
        aria-label={s.unreadCount ? `${s.name}, непрочитанных: ${formatBadge(s.unreadCount)}` : s.name}
        aria-current={s.id === activeId ? "page" : undefined}
        className={`nc-srv ${s.id === activeId ? 'nc-srv--active' : ''}`}
      >
        {initials(s.name)}
      </Link>
      {s.unreadCount > 0 && (
        <span aria-hidden className="nc-badge">
          {formatBadge(s.unreadCount)}
        </span>
      )}
    </div>
  );

  return (
    <nav
      aria-label="Серверы"
      className="flex w-[72px] flex-col items-center gap-2 overflow-y-auto bg-[var(--nc-bg-primary)] border-r border-[var(--nc-border-light)] py-3"
    >
      <div className="relative">
        <Link
          to="/channels/@me"
          title="Личные сообщения"
          aria-label={dmUnread ? `Личные сообщения, непрочитанных: ${formatBadge(dmUnread)}` : "Личные сообщения"}
          aria-current={homeActive ? "page" : undefined}
          className={`nc-srv ${homeActive ? 'nc-srv--active' : ''}`}
        >
          <MessageCircle size={22} />
        </Link>
        {dmUnread > 0 && (
          <span aria-hidden className="nc-badge">
            {formatBadge(dmUnread)}
          </span>
        )}
      </div>
      <div className="w-8 h-[2px] bg-[var(--nc-bg-hover)] rounded" />
      {Object.entries(folders).map(([name, ids]) => (
        <div
          key={name}
          title={name}
          onContextMenu={(e) => { e.preventDefault(); setCtx({ x: e.clientX, y: e.clientY, kind: 'folder', name }); }}
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => { e.preventDefault(); dropInto(name); }}
          className="nc-srv nc-srv--folder"
        >
          📁
        </div>
      ))}
      <button onClick={createFolder} title="Создать папку" aria-label="Создать папку" className={`${pill} rounded-3xl bg-zinc-900 text-xl text-amber-500`}>📁+</button>
      <div
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => { e.preventDefault(); unassign(); }}
        title="Перетащите сюда, чтобы убрать из папки"
        className="h-2 w-10 rounded bg-zinc-900/40"
      />
      {servers.filter((s) => !assigned.has(s.id)).map(serverPill)}
      <button
        onClick={onAdd}
        title="Добавить сервер"
        aria-label="Добавить сервер"
        className="nc-srv nc-srv--add"
      >
        +
      </button>
      {ctx && ctx.kind === "server" && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          onClose={() => setCtx(null)}
          items={[
            { label: "Переименовать", onClick: () => { renameServer(ctx.id!); setCtx(null); } },
          ]}
        />
      )}
      {ctx && ctx.kind === "folder" && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          onClose={() => setCtx(null)}
          items={[
            { label: "Переименовать", onClick: () => { renameFolder(); setCtx(null); } },
            { label: "Удалить папку", danger: true, onClick: () => { deleteFolder(); setCtx(null); } },
          ]}
        />
      )}
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
  const manage = can(server, P.MANAGE_CHANNELS);
  const canSettings =
    can(server, P.MANAGE_ROLES) || can(server, P.VIEW_AUDIT_LOG) || manage;
  const categories = useGuilds((s) => s.categories[server.id]) ?? [];
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [search, setSearch] = useState('');
  const [ctx, setCtx] = useState<{ x: number; y: number; channel: Channel } | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
  const iconButton = "nc-ib";

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
  const q = search.trim().toLowerCase();
  const visibleChannels = channels.filter(
    (c) => !q || c.name.toLowerCase().includes(q),
  );
  const byCategory = new Map<number | null, Channel[]>();
  byCategory.set(null, []);
  for (const category of sortedCategories) byCategory.set(category.id, []);
  for (const channel of [...visibleChannels].sort(
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
       <header className="nc-header">
         <h2 className="flex-1 truncate font-semibold">{server.name}</h2>
         {canSettings && (
           <button
             className="nc-ib"
             title="Настройки сервера"
             aria-label="Настройки сервера"
             onClick={() => onDialog("settings")}
           >
             <Settings size={16} />
           </button>
         )}
         <button
           className="nc-ib"
           title="Участники"
           aria-label="Участники"
           onClick={() => onDialog("members")}
         >
           <Users size={16} />
         </button>
         <button
           className="nc-ib"
           title="Онбординг"
           aria-label="Онбординг"
           onClick={() => onDialog("onboarding")}
         >
           <ClipboardList size={16} />
         </button>
         <button
           className="nc-ib"
           title="События"
           aria-label="События"
           onClick={() => onDialog("events")}
         >
           <CalendarDays size={16} />
         </button>
         <button
           className="nc-ib"
           title="Пригласить"
           aria-label="Пригласить"
           onClick={() => onDialog("invite")}
         >
           <Mail size={16} />
         </button>
       </header>
       {channels.length > 0 && (
         <input
           ref={searchRef}
           value={search}
           onChange={(e) => setSearch(e.target.value)}
           placeholder="Поиск каналов"
           aria-label="Поиск каналов"
           className="mb-1 w-full rounded bg-[var(--nc-bg-tertiary)] px-2 py-1.5 text-sm outline-none placeholder:text-[var(--nc-text-muted)]"
         />
       )}
       <div className="flex-1 overflow-y-auto px-2 pb-3">
         {channels.length === 0 && (
           <p className="px-2 py-3 text-sm text-[var(--nc-text-muted)]">Каналов пока нет.</p>
         )}
         <div className="space-y-0.5">
           {(byCategory.get(null) ?? []).map((channel) => (
             <ChannelRow
               key={channel.id}
               serverId={server.id}
               channel={channel}
               manage={manage}
               onDelete={remove}
               onSettings={setEditingChannel}
               onContextMenu={(e) => setCtx({ x: e.clientX, y: e.clientY, channel })}
             />
           ))}
         </div>
         {sortedCategories.map((category) => (
           <section key={category.id} className="mt-3">
             <h3 className="nc-section-label">
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
                   onSettings={setEditingChannel}
                   onContextMenu={(e) => setCtx({ x: e.clientX, y: e.clientY, channel })}
                 />
               ))}
             </div>
           </section>
         ))}
         {manage && (
           <button
             onClick={() => onDialog("createChannel")}
             className="mt-3 w-full rounded px-2 py-1.5 text-left text-sm text-[var(--nc-text-muted)] hover:bg-[var(--nc-bg-hover)] hover:text-[var(--nc-text-primary)]"
           >
             + Создать канал
           </button>
         )}
       </div>
      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          onClose={() => setCtx(null)}
          items={[
            { label: "Переименовать", onClick: () => { setEditingChannel(ctx.channel); setCtx(null); } },
            { label: "Удалить", danger: true, onClick: () => { remove(ctx.channel); setCtx(null); } },
          ]}
        />
      )}
      {editingChannel && (
        <ChannelSettingsDialog
          server={server}
          channel={editingChannel}
          onClose={() => setEditingChannel(null)}
        />
      )}
    </>
  );
}

function ChannelRow({
  serverId,
  channel,
  manage,
  onDelete,
  onSettings,
  onContextMenu,
}: {
  serverId: number;
  channel: Channel;
  manage: boolean;
  onDelete: (c: Channel) => void;
  onSettings: (c: Channel) => void;
  onContextMenu?: (e: ReactMouseEvent<HTMLElement>) => void;
}) {
    return (
     <div className="group relative hover-lift">
      <NavLink
        to={`/channels/${serverId}/${channel.id}`}
        onContextMenu={onContextMenu}
        onClick={() => {
          if (
            (channel.kind === "voice" || channel.kind === "stage") &&
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
          `flex items-center rounded px-2 py-1.5 pr-12 ${
            isActive
              ? "bg-zinc-700/60 text-white"
              : channel.unreadCount > 0
                ? "font-semibold text-white hover:bg-zinc-900"
                : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200"
          }`
        }
      >
        <span className="mr-1.5 text-zinc-500">
                {channel.kind === "stage" ? "🎤" : channel.kind === "voice" ? "🔊" : "#"}
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
        <>
          <button
            onClick={() => onSettings(channel)}
            title={`Настройки канала ${channel.name}`}
            aria-label={`Настройки канала ${channel.name}`}
            className="absolute top-1/2 right-7 -translate-y-1/2 rounded px-1.5 text-zinc-400 hover:text-zinc-100 md:opacity-0 md:group-hover:opacity-100 md:focus:opacity-100"
          >
            ⚙
          </button>
          <button
            onClick={() => onDelete(channel)}
            title={`Удалить #${channel.name}`}
            aria-label={`Удалить канал ${channel.name}`}
            className="absolute top-1/2 right-1 -translate-y-1/2 rounded px-1.5 text-zinc-400 hover:text-red-400 md:opacity-0 md:group-hover:opacity-100 md:focus:opacity-100"
          >
            ×
          </button>
        </>
      )}
      {(channel.kind === "voice" || channel.kind === "stage") && <VoiceRoster channelId={channel.id} />}
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
      aria-label={`Позвонить ${dmTitle(dm)}`}
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
      label={dmTitle(dm)}
      compact
    />
  );
}

const connectionView = {
  connecting: { dot: "bg-amber-500", label: "Подключение…" },
  closed: { dot: "bg-red-500", label: "Нет соединения" },
} as const;

const ACTIVITY_ICONS: Record<string, string> = {
  playing: "🎮", streaming: "🎥", listening: "🎧", watching: "👁️", custom: "", competing: "🏆",
};

function ConnectionStatus({ onSettings }: { onSettings: () => void }) {
  const { status, latencyMs, idle } = useSocket();
  const user = useSession((s) => s.user);
  const accounts = useAccounts((a) => a);
  const other = accounts.find((a) => a.user.id !== user?.id) ?? accounts[0];
  const [wordsOpen, setWordsOpen] = useState(false);
  const view =
    status === "open"
      ? presenceView[idle ? "idle" : "online"]
      : connectionView[status];
  const switchAccount = (account: Account) => {
    setAccessToken(account.accessToken);
    useSession.setState({ status: "authed", user: account.user });
  };
  return (
    <footer className="flex items-center gap-2 bg-[var(--nc-bg-secondary)] px-3 py-2 text-sm">
      <span
        className={`size-2.5 shrink-0 rounded-full ${view.dot}`}
        aria-hidden
      />
      <div className="min-w-0 flex-1" role="status">
        <div className="truncate font-medium">{user?.username}</div>
        <div className="text-xs text-[var(--nc-text-muted)]">
          {view.label}
          {latencyMs !== null && ` · ${latencyMs} мс`}
        </div>
        {user?.statusText && (
          <div className="text-xs text-[var(--nc-text-muted)]">{user.statusEmoji} {user.statusText}</div>
        )}
        {user?.activityName && (
          <div className="text-xs text-[var(--nc-text-muted)]" title={user.activityType}>
            {ACTIVITY_ICONS[user.activityType] ?? ""} {user.activityName}
          </div>
        )}
      </div>
      {other && (
        <button
          onClick={() => switchAccount(other)}
          title="Аккаунты"
          aria-label="Аккаунты"
          className="nc-ib"
        >
          {other.user.username}
        </button>
      )}
      <button
        onClick={onSettings}
        title="Настройки пользователя"
        aria-label="Настройки пользователя"
        className="nc-ib"
      >
        <Settings size={16} />
      </button>
      <button
        onClick={() => setWordsOpen(true)}
        title="Слова-триггеры"
        aria-label="Слова-триггеры"
        className="nc-ib"
      >
        Aa
      </button>
      <button
        onClick={() => void logout()}
        className="nc-ib"
      >
        Выйти
      </button>
      {wordsOpen && <NotifyWordsDialog onClose={() => setWordsOpen(false)} />}
    </footer>
  );
}
