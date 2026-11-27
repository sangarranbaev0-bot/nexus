import { useEffect, useState, type FormEvent } from "react";
import type { Ticket } from "../generated/Ticket";
import type { Server } from "../generated/Server";
import { listTickets, createTicket, updateTicket } from "../guilds";
import { errorMessage, ghostButton, inputClass, primaryButton } from "../Modal";

export function TicketsTab({ server }: { server: Server }) {
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const reload = () => listTickets(server.id).then(setTickets, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, [server.id]);
  const create = async (e: FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setPending(true);
    setError(null);
    try {
      await createTicket(server.id, title);
      setTitle("");
      await reload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };
  const setStatus = async (id: number, status: string) => {
    try { await updateTicket(server.id, id, { status }); await reload(); }
    catch (e) { setError(errorMessage(e)); }
  };
  return (
    <div className="space-y-3">
      <form onSubmit={create} className="flex gap-2">
        <input value={title} onChange={(e) => setTitle(e.target.value)} className={inputClass} placeholder="Новый тикет: проблема/запрос" required />
        <button disabled={pending} className={primaryButton}>Создать</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      {tickets === null ? (
        <p className="text-sm text-nc-muted">Загрузка…</p>
      ) : tickets.length === 0 ? (
        <p className="text-sm text-nc-faint">Тикетов пока нет.</p>
      ) : (
        <ul className="divide-y divide-nc-line-strong">
          {tickets.map((ticket) => (
            <li key={ticket.id} className="flex items-center justify-between py-2 text-sm">
              <div>
                <div className="font-medium">#{ticket.id} · {ticket.title}</div>
                <div className="text-xs text-nc-faint">{new Date(ticket.createdAt).toLocaleString("ru-RU")} · {ticket.status}</div>
              </div>
              <div className="flex gap-2">
                <button onClick={() => setStatus(ticket.id, ticket.status === "open" ? "closed" : "open")} className={ghostButton}>
                  {ticket.status === "open" ? "Закрыть" : "Открыть"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
