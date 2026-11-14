import { useEffect, useState, type FormEvent } from 'react';
import { Modal, errorMessage, inputClass, primaryButton } from './Modal';
import {
  addNotifyWord,
  loadNotifyWords,
  removeNotifyWord,
  useNotify,
} from './notify';

const MAX_WORDS = 20;

/** Управление словами-триггерами: уведомление о чужом сообщении с этим словом. */
export function NotifyWordsDialog({ onClose }: { onClose: () => void }) {
  const words = useNotify((s) => s.words);
  const [value, setValue] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    if (words === null) void loadNotifyWords().catch((e) => setError(errorMessage(e)));
  }, [words]);

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const word = value.trim();
    if (word.length < 2) {
      setError('Слово: от 2 до 32 символов');
      return;
    }
    setPending(true);
    setError(null);
    try {
      await addNotifyWord(word, caseSensitive);
      setValue('');
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  const remove = (word: string) => {
    setError(null);
    void removeNotifyWord(word).catch((e) => setError(errorMessage(e)));
  };

  const full = (words?.length ?? 0) >= MAX_WORDS;

  return (
    <Modal title="Слова-триггеры" onClose={onClose}>
      <p className="text-sm text-zinc-400">
        Присылать уведомление, когда чужое сообщение содержит это слово.
      </p>
      <form onSubmit={submit} className="flex flex-wrap items-center gap-2">
        <input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          maxLength={32}
          placeholder="Слово"
          aria-label="Новое слово"
          className={`${inputClass} flex-1`}
        />
        <label className="flex items-center gap-1.5 text-sm text-zinc-300">
          <input
            type="checkbox"
            checked={caseSensitive}
            onChange={(e) => setCaseSensitive(e.target.checked)}
          />
          Учитывать регистр
        </label>
        <button disabled={pending || full} className={primaryButton}>
          Добавить
        </button>
      </form>
      {full && <p className="text-sm text-amber-400">Достигнут лимит {MAX_WORDS} слов.</p>}

      {error && (
        <p role="alert" className="text-sm text-red-400">
          {error}
        </p>
      )}

      {words === null ? (
        <p className="text-sm text-zinc-400">Загрузка…</p>
      ) : words.length === 0 ? (
        <p className="text-sm text-zinc-500">Слов пока нет.</p>
      ) : (
        <ul className="divide-y divide-zinc-700">
          {words.map((w) => (
            <li key={w.word} className="flex items-center gap-2 py-2">
              <span className="flex-1 truncate">{w.word}</span>
              {w.caseSensitive && <span className="text-xs text-zinc-500">Aa</span>}
              <button
                type="button"
                onClick={() => remove(w.word)}
                aria-label={`Удалить слово ${w.word}`}
                className="rounded px-2 text-zinc-400 hover:text-red-400"
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}
