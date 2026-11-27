import { useEffect, useState } from 'react';
import { X } from 'lucide-react';

const STORAGE_KEY = 'nexus-keyboard-hint-shown';

interface Props {
  onClose: () => void;
}

export function KeyboardHint({ onClose }: Props) {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    // Показываем только один раз — если ещё не видели подсказку.
    if (!localStorage.getItem(STORAGE_KEY)) {
      setTimeout(() => setVisible(true), 600);
    }
  }, []);

  const done = () => {
    localStorage.setItem(STORAGE_KEY, '1');
    setVisible(false);
    onClose();
  };

  if (!visible) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center px-4">
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={done} />
      <div className="relative w-full max-w-lg rounded-xl border border-nc-line-strong bg-nc-elevated shadow-2xl p-6">
        {/* Закрыть */}
        <button
          onClick={done}
          title="Закрыть подсказку"
          aria-label="Закрыть подсказку"
          className="absolute right-3 top-3 rounded p-1 text-nc-muted hover:bg-nc-hover hover:text-nc-text"
        >
          <X size={16} />
        </button>

        <h2 className="mb-4 text-lg font-semibold text-nc-text">Горячие клавиши</h2>
        <ul className="space-y-2 text-sm">
          {[
            ['Ctrl+1–9', 'Переключиться на сервер'],
            ['Ctrl+N', 'Создать канал'],
            ['Ctrl+G', 'Поиск по серверу'],
            ['Ctrl+K', 'Глобальный поиск'],
            ['Alt+Enter', 'Новая строка в сообщении'],
            ['Esc', 'Закрыть диалог / палитру'],
          ].map(([key, desc]) => (
            <li key={key} className="flex items-center gap-3">
              <kbd className="rounded border border-nc-line-max bg-nc-surface px-2 py-0.5 text-xs font-mono text-nc-accent-text min-w-[80px] text-center">
                {key}
              </kbd>
              <span className="text-nc-text-2">{desc}</span>
            </li>
          ))}
        </ul>
        <p className="mt-4 text-xs text-nc-faint">Подсказка появится снова, если вы очистите кэш браузера.</p>
        <button onClick={done} className="mt-4 w-full rounded-lg bg-nc-accent-hover px-4 py-2 text-sm font-medium text-white hover:bg-nc-accent-strong">
          Понятно
        </button>
      </div>
    </div>
  );
}
