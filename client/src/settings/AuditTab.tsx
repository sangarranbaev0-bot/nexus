import { useEffect, useState } from "react";
import type { AuditEntry } from "../generated/AuditEntry";
import type { Server } from "../generated/Server";
import { listAudit } from "../guilds";
import { errorMessage } from "../Modal";
import { actionLabel } from "./CategoriesTab";

export function AuditTab({ server }: { server: Server }) {
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
