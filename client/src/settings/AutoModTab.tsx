import { useEffect, useState, type FormEvent } from "react";
import type { Server } from "../generated/Server";
import { listAutoModRules, createAutoModRule, deleteAutoModRule } from "../guilds";
import type { AutoModRule } from "../generated/AutoModRule";
import { errorMessage, inputClass, primaryButton } from "../Modal";

export function AutoModTab({ server }: { server: Server }) {
  const [rules, setRules] = useState<AutoModRule[]>([]);
  const [pattern, setPattern] = useState("");
  const [action, setAction] = useState("delete");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const reload = () => listAutoModRules(server.id).then(setRules, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (!pattern.trim() || pattern.trim().length < 2) return;
    try {
      await createAutoModRule(server.id, { pattern: pattern.trim(), action, note: note.trim() || null });
      setPattern(""); setAction("delete"); setNote(""); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };

  const remove = async (id: number) => {
    try { await deleteAutoModRule(server.id, id); await reload(); }
    catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-nc-muted">
        Автомодерация удаляет или предупреждает сообщения, содержащие указанные паттерны.
        Паттерн проверяется по RegEx (падение на Contains при ошибке). Требует разрешение «Управление ролями».
      </p>
      <form onSubmit={add} className="flex flex-col gap-2">
        <input value={pattern} onChange={(e) => setPattern(e.target.value)} className={inputClass} placeholder="Паттерн (2–100 символов)" minLength={2} maxLength={100} />
        <select value={action} onChange={(e) => setAction(e.target.value)} className={inputClass}>
          <option value="delete">Удалить</option>
          <option value="warn">Предупредить</option>
        </select>
        <input value={note} onChange={(e) => setNote(e.target.value)} className={inputClass} placeholder="Комментарий (необязательно)" />
        <button className={primaryButton}>Добавить правило</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-nc-line-strong">
        {rules.map((r) => (
          <li key={r.id} className="flex items-center gap-3 py-2 text-sm">
            <code className="bg-nc-elevated px-2 py-0.5 rounded font-mono text-xs flex-1 truncate">{r.pattern}</code>
            <span className={`text-xs px-2 py-0.5 rounded ${r.action === "delete" ? "bg-red-900/40 text-red-300" : "bg-yellow-900/40 text-yellow-300"}`}>{r.action === "delete" ? "удаление" : "предупреждение"}</span>
            {r.note && <span className="text-xs text-nc-faint max-w-[200px] truncate">{r.note}</span>}
            <button onClick={() => void remove(r.id)} className="text-xs text-red-400 hover:underline">Удалить</button>
          </li>
        ))}
        {rules.length === 0 && <li className="text-sm text-nc-faint py-2">Нет правил.</li>}
      </ul>
    </div>
  );
}

export const BOOST_THRESHOLDS = [1, 3, 7];
export const BOOST_NAMES = ["Нет бустов", "Уровень 1", "Уровень 2", "Уровень 3"];
