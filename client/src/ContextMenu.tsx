import { useEffect } from "react";

export interface CtxItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
}

export function ContextMenu({
  x,
  y,
  items,
  onClose,
}: {
  x: number;
  y: number;
  items: CtxItem[];
  onClose: () => void;
}) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div
      role="menu"
      className="fixed z-20 min-w-[12rem] rounded-lg border border-zinc-800 bg-zinc-950/95 p-1 shadow-2xl backdrop-blur"
      style={{ left: x, top: y }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          role="menuitem"
          onClick={() => {
            onClose();
            item.onClick();
          }}
          className={`w-full rounded px-2 py-1.5 text-left text-sm hover:bg-zinc-800 ${
            item.danger ? "text-red-400" : "text-zinc-200"
          }`}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
