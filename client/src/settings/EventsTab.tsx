import { useEffect, useState, type FormEvent } from "react";
import type { ServerEvent } from "../generated/ServerEvent";
import type { Server } from "../generated/Server";
import { listEvents, createEvent, deleteEvent, toggleInterest } from "../guilds";
import { errorMessage, inputClass, primaryButton } from "../Modal";

export function EventsTab({ server }: { server: Server }) {
  const [items, setItems] = useState<ServerEvent[]>([]);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [time, setTime] = useState("");
  const [error, setError] = useState<string | null>(null);
  const reload = () => listEvents(server.id).then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    const starts = new Date(time).getTime();
    if (!name || !Number.isFinite(starts)) return;
    try {
      await createEvent(server.id, { name, description, startsAt: starts, reminder: true });
      setName(""); setDescription(""); setTime(""); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };

  return (
    <div className="space-y-4">
      <form onSubmit={add} className="space-y-2">
        <input value={name} onChange={(e) => setName(e.target.value)} className={inputClass} placeholder="Название события" required />
        <input value={description} onChange={(e) => setDescription(e.target.value)} className={inputClass} placeholder="Описание" />
        <input type="datetime-local" value={time} onChange={(e) => setTime(e.target.value)} className={inputClass} required />
        <button className={primaryButton}>Создать событие</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-nc-line-strong">
        {items.map((ev) => (
          <li key={ev.id} className="flex items-center gap-2 py-2 text-sm">
            <div className="flex-1">
              <div className="font-medium">{ev.name}</div>
              <div className="text-xs text-nc-muted">{new Date(ev.startsAt).toLocaleString("ru-RU")} · {ev.interestedCount} интересно</div>
            </div>
            <button onClick={() => void toggleInterest(server.id, ev.id).then(reload)} className="text-sm text-nc-accent-text">
              {ev.interested ? "Не интересно" : "Интересно"}
            </button>
            <button onClick={() => void deleteEvent(server.id, ev.id).then(reload)} className="text-xs text-red-400">Удалить</button>
          </li>
        ))}
      </ul>
    </div>
  );
}
