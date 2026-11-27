import { useEffect, useState } from "react";
import { useGuilds } from "../guilds";
import { usePresence } from "../presence";
import { useSession } from "../session";
import type { Member } from "../generated/Member";
import { StatusDot } from "../Dialogs";

export function MemberList({ serverId }: { serverId: number }) {
  const servers = useGuilds((s) => s.servers);
  const members = useGuilds((s) => s.members);
  const presence = usePresence((s) => s);
  const user = useSession((s) => s.user);

  const server = servers?.find((g) => g.id === serverId);
  const memberList = members[serverId] || [];
  const [loading, setLoading] = useState(!server);

  useEffect(() => {
    if (!serverId) return;
    setLoading(true);
    const load = async () => {
      try {
        const res = await fetch(`/api/servers/${serverId}/members`);
        if (!res.ok) throw new Error("Failed");
        const data: Member[] = await res.json();
        useGuilds.setState((s) => ({ members: { ...s.members, [serverId]: data } }));
      } catch (e) {
        console.error("Member load error:", e);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [serverId]);

  const sortedMembers = memberList
    .filter((m) => m.userId !== user?.id)
    .sort((a, b) => {
      const aPres = presence[a.userId] ?? "offline";
      const bPres = presence[b.userId] ?? "offline";
      const order: Record<string, number> = { online: 0, idle: 1, dnd: 2, offline: 3 };
      const sa = order[aPres] ?? 3;
      const sb = order[bPres] ?? 3;
      return sa - sb || a.username.localeCompare(b.username);
    });

  if (loading || !server) {
    return (
      <div className="p-4 text-center text-nc-faint">
        {server ? "Загрузка участников..." : "Сервер не выбран"}
      </div>
    );
  }

  return (
    <div className="flex min-w-60 flex-col border-l border-nc-line bg-nc-surface">
      <header className="flex h-12 items-center justify-between border-b border-nc-line bg-nc-base px-4 text-sm font-medium">
        <span>Участники ({memberList.length})</span>
        <button
          className="rounded px-2 py-1 text-xs text-nc-muted hover:bg-nc-hover"
          aria-label="Список участников"
        >
          ▶
        </button>
      </header>

      <div className="flex-1 space-y-1 overflow-y-auto px-2 py-2">
        {sortedMembers.length === 0 ? (
          <div className="py-4 text-center text-nc-faint">Нет участников</div>
        ) : (
          <>
            {sortedMembers.map((member) => {
              const mp = presence[member.userId] ?? "offline";
              return (
                <div
                  key={member.userId}
                  className="flex items-center gap-2 rounded px-2 py-1.5 hover:bg-nc-elevated"
                  title={member.username}
                >
                  <StatusDot userId={member.userId} fallback={mp} />
                  <span className="truncate text-xs text-nc-text-2">{member.username}</span>
                  <span className="ml-auto text-xs text-nc-faint">{mp !== "offline" ? mp : ""}</span>
                </div>
              );
            })}
          </>
        )}
      </div>

      <footer className="flex h-10 items-center border-t border-nc-line px-4 text-xs text-nc-faint">
        <span>{sortedMembers.filter((m) => presence[m.userId] === "online").length} онлайн</span>
      </footer>
    </div>
  );
}
