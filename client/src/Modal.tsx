import { useEffect, useRef, type ReactNode } from 'react';

/** Нативный <dialog>: фокус-ловушка, Esc и затемнение фона из коробки. */
export function Modal({
  title,
  onClose,
  children,
  wide = false,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    ref.current?.showModal();
  }, []);

  return (
    <dialog
      ref={ref}
      onClose={onClose}
      onClick={(e) => e.target === ref.current && onClose()}
      aria-labelledby="modal-title"
      className={`m-auto rounded-lg bg-nc-elevated p-0 text-nc-text backdrop:bg-black/70 ${
        wide ? 'w-[min(72rem,calc(100vw-2rem))]' : 'w-[min(28rem,calc(100vw-2rem))]'
      }`}
    >
      <div className="space-y-4 p-6">
        <h2 id="modal-title" className="text-xl font-semibold">
          {title}
        </h2>
        {children}
      </div>
    </dialog>
  );
}

/** Ошибка API для показа пользователю. */
export function errorMessage(err: unknown, fallback = 'Что-то пошло не так') {
  return err instanceof Error && err.message ? err.message : fallback;
}

export const inputClass =
  'w-full rounded bg-nc-surface px-3 py-2 outline-none focus:ring-2 focus:ring-nc-accent';
export const primaryButton =
  'rounded bg-nc-accent-strong px-4 py-2 font-medium hover:bg-nc-accent-hover disabled:opacity-60';
export const ghostButton = 'rounded px-4 py-2 text-nc-text-2 hover:underline';
