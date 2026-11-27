import { useEffect, useState, type FormEvent } from "react";
import type { Server } from "../generated/Server";
import { loadChannels, updateServer, useGuilds } from "../guilds";
import { errorMessage, ghostButton, inputClass, primaryButton } from "../Modal";
import { P, hasPerm } from "../permissions";

export function GeneralTab({ server, onAudit }: { server: Server; onAudit: () => void }) {
  const channels = useGuilds((s) => s.channels[server.id]) ?? [];
  const [systemId, setSystemId] = useState(server.systemChannelId ?? 0);
  const [afkId, setAfkId] = useState(server.afkChannelId ?? 0);
  const [announcementId, setAnnouncementId] = useState(server.announcementChannelId ?? 0);
  const [description, setDescription] = useState(server.description);
  const [rules, setRules] = useState(server.rules);
  const [bannerUrl, setBannerUrl] = useState(server.bannerUrl);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    void loadChannels(server.id).catch((e) => setError(errorMessage(e)));
  }, [server.id]);

  const save = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setPending(true);
    setError(null);
    try {
      await updateServer(server.id, {
        systemChannelId: systemId || null,
        afkChannelId: afkId || null,
        announcementChannelId: announcementId || null,
        description,
        rules,
        bannerUrl,
      });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const text = channels.filter((c) => c.kind === "text");
  const voice = channels.filter((c) => c.kind === "voice");
  const canEdit = server.myRole !== "member";

  return (
    <form onSubmit={save} className="space-y-4">
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Системный канал (приветствия и события)
        </span>
        <select
          value={systemId}
          disabled={!canEdit}
          onChange={(e) => setSystemId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Не задан</option>
          {text.map((c) => <option key={c.id} value={c.id}>#{c.name}</option>)}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          AFK-канал (после 5 минут бездействия)
        </span>
        <select
          value={afkId}
          disabled={!canEdit}
          onChange={(e) => setAfkId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Не задан</option>
          {voice.map((c) => <option key={c.id} value={c.id}>🔊 {c.name}</option>)}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Канал объявлений
        </span>
        <select
          value={announcementId}
          disabled={!canEdit}
          onChange={(e) => setAnnouncementId(Number(e.target.value))}
          className={inputClass}
        >
          <option value={0}>Не задан</option>
          {text.map((c) => <option key={c.id} value={c.id}>#{c.name}</option>)}
        </select>
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Описание сервера
        </span>
        <textarea
          value={description}
          disabled={!canEdit}
          maxLength={2000}
          rows={3}
          onChange={(e) => setDescription(e.target.value)}
          className={inputClass}
          placeholder="Коротко о вашем сервере"
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Правила (Markdown)
        </span>
        <textarea
          value={rules}
          disabled={!canEdit}
          maxLength={8000}
          rows={6}
          onChange={(e) => setRules(e.target.value)}
          className={inputClass}
          placeholder="1. Будьте вежливы\n2. Не спамьте"
        />
      </label>
      <label className="block">
        <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
          Ссылка на баннер
        </span>
        <input
          value={bannerUrl}
          disabled={!canEdit}
          maxLength={512}
          onChange={(e) => setBannerUrl(e.target.value)}
          className={inputClass}
          placeholder="https://…"
        />
      </label>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      {hasPerm(server.myPermissions, P.VIEW_AUDIT_LOG) && (
        <button type="button" onClick={onAudit} className={ghostButton}>
          Журнал аудита
        </button>
      )}
      {canEdit && <button disabled={pending} className={primaryButton}>Сохранить</button>}
    </form>
  );
}
