import { useEffect, useState, type FormEvent } from "react";
import type { StickerPack } from "../generated/StickerPack";
import type { Sticker } from "../generated/Sticker";
import type { Server } from "../generated/Server";
import { createStickerPack, deleteSticker, deleteStickerPack, listPackStickers, listStickerPacks } from "../guilds";
import { errorMessage, inputClass, primaryButton } from "../Modal";

export function StickersTab({ server }: { server: Server }) {
  type PackWithStickers = StickerPack & { stickers: Sticker[] };
  const [packs, setPacks] = useState<PackWithStickers[]>();
  const [selectedPackId, setSelectedPackId] = useState<number | null>(null);
  const [packError, setPackError] = useState<string | null>(null);
  const [packPending, setPackPending] = useState(false);
  const [newPackName, setNewPackName] = useState('');
  const [newPackDesc, setNewPackDesc] = useState('');
  const [stickerError, setStickerError] = useState<string | null>(null);
  const [stickerPending, setStickerPending] = useState(false);
  const [newStickerName, setNewStickerName] = useState('');
  const [newStickerEmoji, setNewStickerEmoji] = useState('');
  const [newStickerTags, setNewStickerTags] = useState('');
  const [newStickerFile, setNewStickerFile] = useState<File | null>(null);

  useEffect(() => {
    void loadPacks(server.id).catch((e) => setPackError(errorMessage(e)));
  }, [server.id]);

  const loadPacks = async (serverId: number) => {
    const packs = await listStickerPacks(serverId);
    // Only load basic pack info; stickers loaded on selection via useEffect below
    const packsWithEmptyStickers = packs.map(pack => ({ ...pack, stickers: [] as Sticker[] }));
    setPacks(packsWithEmptyStickers);
    if (packs.length > 0) setSelectedPackId(packs[0].id);
  };

  // Load stickers for the currently selected pack when it changes
  useEffect(() => {
    if (!selectedPackId || !packs) return;
    const currentPack = packs.find(p => p.id === selectedPackId);
    if (currentPack?.stickers !== undefined && currentPack.stickers.length > 0) return;
    void loadPacks(server.id).then(() => {
      // Already loaded above; just ensure we have stickers for this pack
    }).catch(() => {});
    void listPackStickers(server.id, selectedPackId)
      .then(stickers => {
        setPacks(prev => prev?.map(p =>
          p.id === selectedPackId ? { ...p, stickers } : p
        ));
      })
      .catch((e) => setPackError(errorMessage(e)));
  }, [selectedPackId, server.id]);

  const createPack = async () => {
    if (!newPackName.trim()) {
      setPackError("Введите название пака");
      return;
    }
    setPackPending(true);
    setPackError(null);
    try {
      const pack = await createStickerPack(server.id, {
        name: newPackName.trim(),
        description: newPackDesc.trim() || null,
      });
      const packWithStickers = { ...pack, stickers: [] } as PackWithStickers;
      setPacks(prev => {
        const updated = [...(prev ?? []), packWithStickers];
        if (!selectedPackId) setSelectedPackId(packWithStickers.id);
        return updated;
      });
      setNewPackName('');
      setNewPackDesc('');
    } catch (err) {
      setPackError(errorMessage(err));
    } finally {
      setPackPending(false);
    }
  };

  const deletePack = async (packId: number) => {
    if (!confirm(`Удалить пак "${packs?.find(p => p.id === packId)?.name}"?`)) return;
    try {
      await deleteStickerPack(server.id, packId);
      setPacks(prev => prev?.filter(p => p.id !== packId));
      if (selectedPackId === packId) {
        setSelectedPackId(packs?.[0].id || null);
      }
    } catch (err) {
      setPackError(errorMessage(err));
    }
  };

  const uploadSticker = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!selectedPackId) return;
    if (!newStickerName.trim()) {
      setStickerError("Введите название стикера");
      return;
    }
    if (!newStickerEmoji.trim()) {
      setStickerError("Введите emoji/короткое имя");
      return;
    }
    if (!newStickerFile) {
      setStickerError("Выберите файл");
      return;
    }
    if (newStickerFile.size > 10 * 1024 * 1024) {
      setStickerError("Файл слишком большой (макс. 10 МБ)");
      return;
    }
    if (!['image/png', 'image/webp'].includes(newStickerFile.type)) {
      setStickerError("Поддерживаются только PNG и WebP");
      return;
    }

    setStickerPending(true);
    setStickerError(null);
    try {
      const formData = new FormData();
      formData.append('name', newStickerName.trim());
      formData.append('emoji', newStickerEmoji.trim());
      formData.append('tags', newStickerTags.trim());
      formData.append('file', newStickerFile);

      const resp = await fetch(`/api/servers/${server.id}/stickers/packages/${selectedPackId}/stickers`, {
        method: 'POST',
        credentials: 'include',
        body: formData,
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const sticker = await resp.json();
      
      setPacks(prev => prev?.map(p => 
        p.id === selectedPackId ? { ...p, stickers: [...(p.stickers ?? []), sticker] } : p
      ));
      setNewStickerName('');
      setNewStickerEmoji('');
      setNewStickerTags('');
      setNewStickerFile(null);
    } catch (err) {
      setStickerError(errorMessage(err));
    } finally {
      setStickerPending(false);
    }
  };

  const deleteSticker = async (packId: number, stickerId: number) => {
    if (!confirm("Удалить стикер?")) return;
    try {
      await deleteSticker(server.id, stickerId);
      setPacks(prev => prev?.map(p => 
        p.id === packId ? { ...p, stickers: (p.stickers ?? []).filter(s => s.id !== stickerId) } : p
      ));
    } catch (err) {
      setStickerError(errorMessage(err));
    }
  };

  return (
    <div className="space-y-4">
      {/* Pack management */}
      <div className="space-y-2">
        <h3 className="mb-1 text-xs font-semibold uppercase text-nc-muted">
          Стикерпаки
        </h3>
        <form onSubmit={createPack} className="space-y-3">
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
              Название
            </span>
            <input
              name="name"
              value={newPackName}
              onChange={(e) => setNewPackName(e.target.value)}
              required
              minLength={2}
              maxLength={32}
              className={inputClass}
              placeholder="Emoji Pack"
            />
          </label>
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
              Описание
            </span>
            <input
              name="description"
              value={newPackDesc}
              onChange={(e) => setNewPackDesc(e.target.value)}
              maxLength={200}
              className={inputClass}
              placeholder="Краткое описание"
            />
          </label>
          <button type="submit" disabled={packPending} className={primaryButton}>
            {packPending ? "Создание…" : "Создать пак"}
          </button>
        </form>
        {packError && (
          <p role="alert" className="text-sm text-red-400">
            {packError}
          </p>
        )}
      </div>

      {/* Pack list */}
      <div className="space-y-2">
        {packs === undefined ? (
          <p className="text-nc-muted">Загрузка…</p>
        ) : packs.length === 0 ? (
          <p className="text-nc-muted">Пока нет стикерпаков.</p>
        ) : (
          <ul className="grid grid-cols-1 gap-2">
            {packs.map(pack => {
              const active = selectedPackId === pack.id;
              return (
                <button
                  key={pack.id}
                  onClick={() => setSelectedPackId(pack.id)}
                  className={`flex items-center gap-2 rounded px-2 py-1.5 text-left text-sm ${
                    active ? "bg-nc-hover text-white" : "text-nc-text-2 hover:bg-nc-hover/50"
                  }`}
                >
                  <div className="flex-1">{pack.name}</div>
                  <div className="text-xs text-fg/60">{pack.description || " — "}</div>
                  <div className="text-xs text-fg/60">{pack.stickers?.length ?? 0} стикеров</div>
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); deletePack(pack.id); }}
                    className="text-xs text-red-400 hover:underline"
                    aria-label={`Удалить пак ${pack.name}`}
                  >
                    Удалить
                  </button>
                </button>
              );
            })}
          </ul>
        )}
      </div>

      {/* Sticker management for selected pack */}
      {selectedPackId != null && packs?.find(p => p.id === selectedPackId) && (
        <div className="space-y-4">
          <div className="space-y-2">
            <h3 className="mb-1 text-xs font-semibold uppercase text-nc-muted">
              Стикеры в паке
            </h3>
            <form onSubmit={uploadSticker} className="space-y-3">
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  Имя стикера
                </span>
                <input
                  name="name"
                  value={newStickerName}
                  onChange={(e) => setNewStickerName(e.target.value)}
                  required
                  minLength={2}
                  maxLength={32}
                  className={inputClass}
                  placeholder="party_blob"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  Emoji/короткое имя (1-2 символа)
                </span>
                <input
                  name="emoji"
                  value={newStickerEmoji}
                  onChange={(e) => setNewStickerEmoji(e.target.value)}
                  maxLength={4}
                  className={inputClass}
                  placeholder="🎉"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  Теги (через запятую)
                </span>
                <input
                  name="tags"
                  value={newStickerTags}
                  onChange={(e) => setNewStickerTags(e.target.value)}
                  maxLength={100}
                  className={inputClass}
                  placeholder="party, celebration, blob"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold uppercase text-nc-muted">
                  PNG или WebP (до 10 МБ)
                </span>
                <input
                  name="file"
                  type="file"
                  accept="image/png,image/webp"
                  required
                  onChange={(e) => {
                    const file = e.target?.files?.[0] || null;
                    setNewStickerFile(file);
                  }}
                  className="block w-full text-sm text-nc-text-2"
                />
              </label>
              <button type="submit" disabled={stickerPending} className={primaryButton}>
                {stickerPending ? "Загрузка…" : "Добавить стикер"}
              </button>
            </form>
            {stickerError && (
              <p role="alert" className="text-sm text-red-400">
                {stickerError}
              </p>
            )}
          </div>

          {packs.find(p => p.id === selectedPackId)?.stickers === undefined ? (
            <p className="text-nc-muted">Загрузка стикеров…</p>
          ) : (packs.find(p => p.id === selectedPackId)?.stickers?.length ?? 0) === 0 ? (
            <p className="text-nc-muted">Пока нет стикеров в этом паке.</p>
          ) : (
            <ul className="grid grid-cols-2 gap-2 sm:grid-cols-3">
              {(packs.find(p => p.id === selectedPackId)?.stickers ?? []).map(sticker => (
                <li key={sticker.id} className="flex items-center gap-2 rounded bg-nc-surface p-2">
                  <img src={`/storage/${sticker.storageKey}`} alt={sticker.name} className="h-6 w-6 object-contain" />
                  <span className="min-w-0 flex-1 truncate text-sm">:{sticker.emoji}:</span>
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); deleteSticker(selectedPackId, sticker.id); }}
                    className="text-xs text-red-400 hover:underline"
                    aria-label={`Удалить стикер ${sticker.name}`}
                  >
                    Удалить
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
