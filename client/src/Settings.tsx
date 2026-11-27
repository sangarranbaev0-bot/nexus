import { useState } from "react";
import type { Server } from "./generated/Server";
import { Modal } from "./Modal";
import { NotifyControls } from "./NotifyControls";
import { P, hasPerm } from "./permissions";
import { GeneralTab } from "./settings/GeneralTab";
import { RolesTab } from "./settings/RolesTab";
import { CategoriesTab } from "./settings/CategoriesTab";
import { TicketsTab } from "./settings/TicketsTab";
import { AuditTab } from "./settings/AuditTab";
import { EmojisTab } from "./settings/EmojisTab";
import { ReactionRolesTab } from "./settings/ReactionRolesTab";
import { OnboardingTab } from "./settings/OnboardingTab";
import { EventsTab } from "./settings/EventsTab";
import { ForumTab } from "./settings/ForumTab";
import { ModerationTab } from "./settings/ModerationTab";
import { TemplatesTab } from "./settings/TemplatesTab";
import { AutoModTab } from "./settings/AutoModTab";
import { BoostsTab } from "./settings/BoostsTab";

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
