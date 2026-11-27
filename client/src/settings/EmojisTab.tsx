import { useEffect, useState, type FormEvent } from "react";
import type { Server } from "../generated/Server";
import { errorMessage, inputClass, primaryButton } from "../Modal";
import { addEmoji, loadEmojis, removeEmoji, useEmojis } from "../emojis";

export function EmojisTab({ server }: { server: Server }) {
  const list = useEmojis((s) => s.byServer[server.id]);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    loadEmojis(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  const upload = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const el = e.currentTarget;
    const data = new FormData(el);
    const name = String(data.get("name") ?? "");
    const file = data.get("file");
    if (!(file instanceof File) || file.size === 0) {
      setError("Выберите PNG-файл.");
      return;
    }
    setPending(true);
    setError(null);
    try {
      await addEmoji(server.id, name, file);
      el.reset();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const remove = async (id: number) => {
    if (!confirm("Удалить эмодзи?")) return;
    try {
      await removeEmoji(server.id, id);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <div className="space-y-4">
      <form onSubmit={upload} className="space-y-3">
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
            Имя (a-z, 0-9, _)
          </span>
          <input name="name" required minLength={2} maxLength={32} className={inputClass} placeholder="party_blob" />
        </label>
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
            PNG до 256 КБ
          </span>
          <input name="file" type="file" accept="image/png" required className="block w-full text-sm text-nc-text-2" />
        </label>
        <button type="submit" disabled={pending} className={primaryButton}>
          {pending ? "Загрузка…" : "Загрузить эмодзи"}
        </button>
      </form>
      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}
      {list === undefined ? (
        <p className="text-nc-muted">Загрузка…</p>
      ) : list.length === 0 ? (
        <p className="text-nc-muted">Пока нет своих эмодзи.</p>
      ) : (
        <ul className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          {list.map((e) => (
            <li key={e.id} className="flex items-center gap-2 rounded bg-nc-surface p-2">
              <img src={e.url} alt={`:${e.name}:`} className="h-6 w-6" />
              <span className="min-w-0 flex-1 truncate text-sm">:{e.name}:</span>
              <button
                type="button"
                onClick={() => void remove(e.id)}
                className="text-xs text-red-400 hover:underline"
                aria-label={`Удалить эмодзи ${e.name}`}
              >
                Удалить
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
