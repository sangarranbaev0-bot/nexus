import { useState } from "react";
import type { ServerTemplate } from "../generated/ServerTemplate";
import type { Server } from "../generated/Server";
import { loadServers, getTemplate, createFromTemplate } from "../guilds";
import { errorMessage, ghostButton, inputClass, primaryButton } from "../Modal";

export function TemplatesTab({ server }: { server: Server }) {
  const [template, setTemplate] = useState<ServerTemplate | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const exportTemplate = async () => {
    try {
      const t = await getTemplate(server.id);
      setTemplate(t);
      setText(JSON.stringify(t, null, 2));
    } catch (e) { setError(errorMessage(e)); }
  };
  const copy = () => void navigator.clipboard?.writeText(text);
  const importTemplate = async () => {
    try {
      const t = JSON.parse(text) as ServerTemplate;
      const created = await createFromTemplate(t);
      await loadServers();
      setError(null);
      window.location.href = `/channels/${created.id}`;
    } catch (e) { setError(errorMessage(e)); }
  };

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <button onClick={() => void exportTemplate()} className={primaryButton}>Экспортировать текущий сервер</button>
        {template && <button onClick={copy} className={ghostButton}>Копировать JSON</button>}
      </div>
      <textarea value={text} onChange={(e) => setText(e.target.value)} rows={12} className={inputClass} placeholder="Вставьте JSON шаблона, чтобы создать новый сервер" />
      <button onClick={() => void importTemplate()} className={primaryButton}>Создать сервер по шаблону</button>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
    </div>
  );
}
