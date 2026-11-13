import { useEffect, useRef, type ReactNode } from 'react';

/** Нативный <dialog>: фокус-ловушка, Esc и затемнение фона из коробки. */
export function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
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
      className="m-auto w-[min(28rem,calc(100vw-2rem))] rounded-lg bg-zinc-800 p-0 text-zinc-100 backdrop:bg-black/70"
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
  'w-full rounded bg-zinc-900 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500';
export const primaryButton =
  'rounded bg-indigo-500 px-4 py-2 font-medium hover:bg-indigo-600 disabled:opacity-60';
export const ghostButton = 'rounded px-4 py-2 text-zinc-300 hover:underline';
