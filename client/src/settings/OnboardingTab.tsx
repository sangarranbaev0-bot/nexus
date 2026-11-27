import { useEffect, useState, type FormEvent } from "react";
import type { OnboardingQuestion } from "../generated/OnboardingQuestion";
import type { Server } from "../generated/Server";
import { listOnboarding, createOnboarding, deleteOnboarding, loadRoles, useGuilds } from "../guilds";
import { errorMessage, inputClass, primaryButton } from "../Modal";

export function OnboardingTab({ server }: { server: Server }) {
  const roles = useGuilds((s) => s.roles[server.id]) ?? [];
  const [items, setItems] = useState<OnboardingQuestion[]>([]);
  const [question, setQuestion] = useState("");
  const [roleId, setRoleId] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const reload = () => listOnboarding(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void loadRoles(server.id); void reload(); }, [server.id]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (!question || !roleId) return;
    try {
      await createOnboarding(server.id, { question, roleId });
      setQuestion(""); setRoleId(0); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-nc-muted">Участник отвечает на вопрос «Да» и получает роль. Ответы учитываются один раз.</p>
      <form onSubmit={add} className="space-y-2">
        <input value={question} onChange={(e) => setQuestion(e.target.value)} className={inputClass} placeholder="Вы прочитали правила?" required />
        <div className="flex gap-2">
          <select value={roleId} onChange={(e) => setRoleId(Number(e.target.value))} className={inputClass} required>
            <option value={0}>Роль</option>
            {roles.filter((r) => !r.isDefault).map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
          </select>
          <button className={primaryButton}>Добавить</button>
        </div>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-nc-line-strong">
        {items.map((q) => (
          <li key={q.id} className="flex items-center gap-2 py-2 text-sm">
            <span className="flex-1">{q.question}</span>
            <span className="text-nc-muted">{roles.find((r) => r.id === q.roleId)?.name}</span>
            <button onClick={() => void deleteOnboarding(server.id, q.id).then(reload)} className="text-xs text-red-400">Удалить</button>
          </li>
        ))}
      </ul>
    </div>
  );
}
