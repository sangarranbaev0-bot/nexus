import { useEffect, useState } from "react";
import type { Server } from "../generated/Server";
import { listBans, banMember, unbanMember, kickMember, timeoutMember, unTimeoutMember } from "../guilds";
import type { BanInfo } from "../generated/BanInfo";
import { errorMessage, ghostButton, inputClass, primaryButton } from "../Modal";

export function ModerationTab({ server }: { server: Server }) {
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
