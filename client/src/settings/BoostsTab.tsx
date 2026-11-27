import { useEffect, useState } from "react";
import type { Server } from "../generated/Server";
import { getServerBoosts, toggleServerBoost } from "../guilds";
import { errorMessage } from "../Modal";
import { BOOST_NAMES, BOOST_THRESHOLDS } from "./AutoModTab";

export function BoostsTab({ server }: { server: Server }) {
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
