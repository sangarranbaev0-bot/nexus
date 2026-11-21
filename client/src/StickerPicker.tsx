import { useEffect, useState } from 'react';
import { useGuilds } from './guilds';
import type { StickerPack } from './generated/StickerPack';
import type { Sticker } from './generated/Sticker';

/** Пакет стикеров с загруженными стикерами. */
type PackWithStickers = StickerPack & { stickers: Sticker[] };

export function StickerPicker({
    channelId,
    onPick,
    onClose,
}: {
    channelId: number;
    onPick: (sticker: Sticker) => void;
    onClose: () => void;
}) {
    const [packs, setPacks] = useState<PackWithStickers[]>([]);
    const [selectedPackId, setSelectedPackId] = useState<number | null>(null);
    const [loading, setLoading] = useState(false);
    const { servers } = useGuilds.getState();

    useEffect(() => {
        if (!servers) return;
        // Find the server this channel belongs to
        const { channels } = useGuilds.getState();
        const serverId = Object.entries(channels).find(([_, chans]) => 
            chans?.some(c => c.id === channelId)
        )?.[0];
        if (!serverId) return;

        setLoading(true);
        listStickerPacks(Number(serverId))
            .then(packs => Promise.all(packs.map((p: StickerPack) => 
                listChannelStickers(channelId)
                    .then((stickers: Sticker[]) => ({ ...p, stickers }))
            )))
            .then(packs => {
                setPacks(packs);
                if (packs.length > 0) setSelectedPackId(packs[0].id);
            })
            .catch(console.error)
            .finally(() => setLoading(false));
    }, [channelId, servers]);

    if (loading) return <div className="p-4 text-center">Загрузка…</div>;

    return (
        <div className="max-h-[300px] w-[320px] bg-bg dark:bg-bg-dk rounded-xl border border-border shadow-lg divide-y divide-border">
            <div className="flex flex-col h-full">
                {/* Pack selector */}
                <div className="flex-1 overflow-y-auto p-2 space-y-1">
                    {packs.map(pack => (
                        <button
                            key={pack.id}
                            onClick={() => setSelectedPackId(pack.id)}
                            className={`flex items-center gap-2 px-2 py-1.5 rounded-md transition-colors ${
                                selectedPackId === pack.id ? 'bg-primary/10 text-primary' : 'hover:bg-accent/10 text-fg'
                            }`}
                        >
                            <div className="flex-1">{pack.name}</div>
                            <div className="text-xs text-fg/60">{pack.stickers.length} стикеров</div>
                        </button>
                    ))}
                </div>

                {/* Stickers grid */}
                {selectedPackId != null && packs.find(p => p.id === selectedPackId) && (
                    <div className="flex-1 overflow-y-auto p-2 grid grid-cols-4 gap-1">
                        {packs.find(p => p.id === selectedPackId)?.stickers.map(sticker => (
                            <button
                                key={sticker.id}
                                onClick={() => onPick(sticker)}
                                className="flex items-center justify-center p-1 bg-bg-dk rounded-md hover:bg-accent/10 transition-colors"
                                title={sticker.name}
                            >
                                <img
                                    src={`/storage/${sticker.storageKey}`}
                                    alt={sticker.name}
                                    className="w-8 h-8 object-contain"
                                    onError={(e) => (e.target as HTMLImageElement).style.display = 'none'}
                                />
                            </button>
                        ))}
                    </div>
                )}
            </div>

            {/* Close button */}
            <div className="flex items-center justify-center px-3 py-2 border-t divide-border">
                <button
                    onClick={onClose}
                    className="px-3 py-1 rounded-md text-sm font-medium text-zinc-400 hover:text-zinc-100 transition-colors"
                >
                    Закрыть
                </button>
            </div>
        </div>
    );
}

async function listStickerPacks(serverId: number) {
    return fetch(`/api/servers/${serverId}/stickers/packages`, { credentials: 'include' })
        .then(r => r.json());
}

async function listChannelStickers(channelId: number) {
    return fetch(`/api/channels/${channelId}/stickers`, { credentials: 'include' })
        .then(r => r.json());
}