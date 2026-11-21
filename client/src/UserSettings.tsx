import { useEffect, useState, type FormEvent } from 'react';
import { api } from './api';
import { useSession, signOutLocal } from './session';
import { setAccessToken } from './api';
import { useAccounts, type Account } from './auth';
import { Modal, errorMessage, ghostButton, inputClass, primaryButton } from './Modal';
import type { User } from './generated/User';
import type { Note } from './generated/Note';
import type { Session } from './generated/Session';
import type { TwoFactorSetup } from './generated/TwoFactorSetup';
import type { UpdateProfileRequest } from './generated/UpdateProfileRequest';
import type { ExportResponse } from './generated/ExportResponse';
import type { TwoFactorCode } from './generated/TwoFactorCode';

type Tab = 'profile' | 'notes' | 'sessions' | 'security' | 'view' | 'accounts';
type Theme = 'auto' | 'light' | 'dark';
type Fs = 14 | 16 | 18;
const themes: { value: Theme; label: string }[] = [
  { value: 'auto', label: 'Системная' },
  { value: 'light', label: 'Светлая' },
  { value: 'dark', label: 'Тёмная' },
];
const fontSizes: { value: Fs; label: string }[] = [
  { value: 14, label: '14' },
  { value: 16, label: '16' },
  { value: 18, label: '18' },
];

export function UserSettingsDialog({ onClose }: { onClose: () => void }) {
  const user = useSession((s) => s.user)!;
  const [tab, setTab] = useState<Tab>('profile');
  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem('nexus-theme') as Theme) || 'auto');
  const [fs, setFs] = useState<Fs>(() => Number(localStorage.getItem('nexus-fs') || '16') as Fs);
  const [compact, setCompactState] = useState(localStorage.getItem('nexus-compact') === '1');
  const tabs: [Tab, string][] = [
    ['profile', 'Профиль'],
    ['notes', 'Заметки'],
    ['sessions', 'Устройства'],
    ['accounts', 'Аккаунты'],
    ['security', 'Безопасность'],
    ['view', 'Вид'],
  ];
  const applyView = (nextTheme: Theme, nextFs: Fs, nextCompact: boolean) => {
    localStorage.setItem('nexus-theme', nextTheme);
    localStorage.setItem('nexus-fs', String(nextFs));
    localStorage.setItem('nexus-compact', nextCompact ? '1' : '0');
    const root = document.documentElement;
    const dark = nextTheme === 'dark' || (nextTheme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    root.classList.toggle('nexus-dark', dark);
    root.classList.toggle('nexus-light', !dark);
    root.classList.toggle('nexus-fs-' + nextFs, true);
    root.classList.remove('nexus-fs-14', 'nexus-fs-16', 'nexus-fs-18');
    root.classList.add('nexus-fs-' + nextFs);
    root.classList.toggle('nexus-compact', nextCompact);
    root.style.setProperty('--nexus-fs', `${nextFs}px`);
  };
  useEffect(() => {
    applyView(theme, fs, compact);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [theme, fs, compact]);
  const setView = (key: Tab) => setTab(key);
  const setCompact = (v: boolean) => { setCompactState(v); applyView(theme, fs, v); };
  return (
    <Modal title="Настройки пользователя" onClose={onClose} wide>
      <div role="tablist" className="mb-3 flex gap-1 border-b border-zinc-700 pb-2">
        {tabs.map(([key, label]) => (
          <button
            key={key}
            role="tab"
            aria-selected={tab === key}
            onClick={() => setTab(key)}
            className={`rounded px-3 py-1.5 text-sm ${tab === key ? 'bg-zinc-700 text-white' : 'text-zinc-400 hover:bg-zinc-800'}`}
          >
            {label}
          </button>
        ))}
      </div>
      <div className="max-h-[60vh] overflow-y-auto pr-1">
        {tab === 'profile' && <ProfileTab user={user} />}
        {tab === 'notes' && <NotesTab />}
        {tab === 'sessions' && <SessionsTab />}
        {tab === 'accounts' && <AccountsTab />}
        {tab === 'security' && <SecurityTab user={user} />}
        {tab === 'view' && (
          <ViewTab
            theme={theme}
            fs={fs}
            compact={compact}
            onTheme={setTheme}
            onFs={setFs}
            onCompact={setCompact}
          />
        )}
      </div>
    </Modal>
  );
}

function ProfileTab({ user }: { user: User }) {
  const setUser = useSession((s) => s.user);
  const [form, setForm] = useState({
    about: user.about, bannerUrl: user.bannerUrl, pronouns: user.pronouns,
    accentColor: user.accentColor ?? 0, statusText: user.statusText, statusEmoji: user.statusEmoji,
    activityName: user.activityName, activityType: user.activityType, notifyMode: user.notifyMode,
    dnd: user.dnd, expires: user.statusExpiresAt ? new Date(user.statusExpiresAt).toISOString().slice(0, 16) : '',
    badge: user.badge ?? '',
  });
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const save = async (e: FormEvent) => {
    e.preventDefault();
    const body: UpdateProfileRequest = {
      about: form.about, bannerUrl: form.bannerUrl, pronouns: form.pronouns,
      accentColor: form.accentColor || null, statusText: form.statusText, statusEmoji: form.statusEmoji,
      activityName: form.activityName, activityType: form.activityType, notifyMode: form.notifyMode,
      dnd: form.dnd,
      statusExpiresAt: form.expires ? new Date(form.expires).getTime() : null,
      badge: form.badge,
    };
    try {
      const next = await api<User>('/api/me/profile', { method: 'PATCH', body });
      useSession.setState({ user: next });
      setError(null);
      setSaved(true);
      void setUser;
    } catch (err) { setError(errorMessage(err)); }
  };
  return (
    <form onSubmit={save} className="space-y-3">
      <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">О себе</span>
        <textarea value={form.about} onChange={(e) => setForm({ ...form, about: e.target.value })} rows={3} maxLength={2000} className={inputClass} /></label>
      <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Ссылка на баннер</span>
        <input value={form.bannerUrl} onChange={(e) => setForm({ ...form, bannerUrl: e.target.value })} className={inputClass} /></label>
      <div className="grid grid-cols-2 gap-2">
        <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Местоимения</span>
          <input value={form.pronouns} onChange={(e) => setForm({ ...form, pronouns: e.target.value })} className={inputClass} placeholder="он/его" /></label>
        <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Цвет профиля</span>
          <input type="color" value={`#${(form.accentColor || 0).toString(16).padStart(6, '0')}`}
            onChange={(e) => setForm({ ...form, accentColor: parseInt(e.target.value.slice(1), 16) })}
            className="h-10 w-full rounded bg-zinc-900" /></label>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Статус</span>
          <input value={form.statusText} onChange={(e) => setForm({ ...form, statusText: e.target.value })} className={inputClass} placeholder="Играю..." /></label>
        <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Эмодзи статуса</span>
          <input value={form.statusEmoji} onChange={(e) => setForm({ ...form, statusEmoji: e.target.value })} className={inputClass} placeholder="🎮" /></label>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Активность (Discord)</span>
          <input value={form.activityName} onChange={(e) => setForm({ ...form, activityName: e.target.value })} className={inputClass} placeholder="Играю в ..." /></label>
        <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Тип активности</span>
          <select value={form.activityType} onChange={(e) => setForm({ ...form, activityType: e.target.value })} className={inputClass}>
            <option value="playing">Играю (Playing)</option>
            <option value="streaming">Стримлю (Streaming)</option>
            <option value="listening">Слушаю (Listening)</option>
            <option value="watching">Смотрю (Watching)</option>
            <option value="custom">Свой (Custom)</option>
            <option value="competing">Соревнуюсь (Competing)</option>
          </select></label>
      </div>
      <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Режим уведомлений</span>
        <select value={form.notifyMode} onChange={(e) => setForm({ ...form, notifyMode: e.target.value })} className={inputClass}>
          <option value="all">Все уведомления</option>
          <option value="mentions">Только @упоминания</option>
          <option value="none">Не беспокоить (offline)</option>
        </select></label>
      <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Действует до</span>
        <input type="datetime-local" value={form.expires} onChange={(e) => setForm({ ...form, expires: e.target.value })} className={inputClass} /></label>
      <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.dnd} onChange={(e) => setForm({ ...form, dnd: e.target.checked })} /> Не беспокоить</label>
      <label className="block"><span className="mb-1 block text-xs uppercase text-zinc-400">Бейдж</span>
        <input value={form.badge} onChange={(e) => setForm({ ...form, badge: e.target.value })} maxLength={16} className={inputClass} placeholder="напр. Разработчик, VIP, Основатель" /></label>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      {saved && <p className="text-sm text-emerald-400">Сохранено</p>}
      <button className={primaryButton}>Сохранить</button>
    </form>
  );
}

function NotesTab() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [target, setTarget] = useState('');
  const [text, setText] = useState('');
  const [nickname, setNickname] = useState('');
  const [error, setError] = useState<string | null>(null);
  const reload = () => api<Note[]>('/api/me/notes').then(setNotes, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, []);
  const save = async (e: FormEvent) => {
    e.preventDefault();
    const id = Number(target);
    if (!id) return;
    try {
      await api(`/api/me/notes/${id}`, { method: 'PUT', body: { note: text, nickname } });
      setTarget(''); setText(''); setNickname(''); await reload();
    } catch (err) { setError(errorMessage(err)); }
  };
  return (
    <div className="space-y-3">
      <form onSubmit={save} className="space-y-2">
        <input value={target} onChange={(e) => setTarget(e.target.value)} className={inputClass} placeholder="ID пользователя" required />
        <input value={nickname} onChange={(e) => setNickname(e.target.value)} className={inputClass} placeholder="Ник" />
        <textarea value={text} onChange={(e) => setText(e.target.value)} rows={3} className={inputClass} placeholder="Заметка" />
        <button className={primaryButton}>Сохранить заметку</button>
      </form>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-zinc-700">
        {notes.map((n) => <li key={n.targetId} className="py-2 text-sm">
          <span className="font-medium">{n.nickname || `#${n.targetId}`}</span>
          <span className="ml-2 text-zinc-400">{n.note}</span>
          <button onClick={() => void api(`/api/me/notes/${n.targetId}`, { method: 'DELETE' }).then(reload)} className="ml-2 text-xs text-red-400">×</button>
        </li>)}
      </ul>
    </div>
  );
}

function ViewTab({
  theme, fs, compact, onTheme, onFs, onCompact,
}: {
  theme: Theme;
  fs: Fs;
  compact: boolean;
  onTheme: (v: Theme) => void;
  onFs: (v: Fs) => void;
  onCompact: (v: boolean) => void;
}) {
  const radio = (checked: boolean, onChange: () => void, label: string) => (
    <label className={`cursor-pointer rounded px-3 py-2 text-center text-sm ${checked ? 'bg-zinc-700 text-white' : 'bg-zinc-900 text-zinc-300 hover:bg-zinc-800'}`} aria-label={label}>
      <input type="radio" name="nexus-theme" checked={checked} onChange={onChange} className="hidden" />{label}
    </label>
  );
  return (
    <div className="space-y-4">
      <div>
        <h3 className="mb-2 text-xs font-semibold uppercase text-zinc-400">Тема</h3>
        <div className="grid gap-2 sm:grid-cols-3">
          {themes.map((t) => radio(theme === t.value, () => onTheme(t.value), t.label))}
        </div>
      </div>
      <div>
        <h3 className="mb-2 text-xs font-semibold uppercase text-zinc-400">Размер шрифта</h3>
        <div className="grid gap-2 sm:grid-cols-3">
          {fontSizes.map((f) => radio(fs === f.value, () => onFs(f.value), `${f.value}px`))}
        </div>
      </div>
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={compact} onChange={(e) => onCompact(e.target.checked)} /> Компактный режим
      </label>
      <p className="text-xs text-zinc-500">Системная тема поддерживается автоматически.</p>
    </div>
  );
}

function SessionsTab() {
  const [items, setItems] = useState<Session[]>([]);
  const [error, setError] = useState<string | null>(null);
  const reload = () => api<Session[]>('/api/me/sessions').then(setItems, (e) => setError(errorMessage(e)));
  useEffect(() => { void reload(); }, []);
  return (
    <div className="space-y-3">
      <button onClick={() => void api('/api/me/sessions/logout-others', { method: 'POST' }).then(reload, (e) => setError(errorMessage(e)))} className={ghostButton}>
        Выйти на других устройствах
      </button>
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
      <ul className="divide-y divide-zinc-700">
        {items.map((s) => <li key={s.id} className="flex items-center gap-2 py-2 text-sm">
          <div className="flex-1">
            <div>{s.deviceLabel || 'Сессия'} {s.current && <span className="text-xs text-emerald-400">• текущая</span>}</div>
            <div className="text-xs text-zinc-500">{new Date(s.createdAt).toLocaleString('ru-RU')} · {s.ip || '—'}</div>
          </div>
          {!s.current && !s.revoked && (
            <button onClick={() => void api(`/api/me/sessions/${s.id}`, { method: 'DELETE' }).then(reload)} className="text-xs text-red-400">Выйти</button>
          )}
        </li>)}
      </ul>
    </div>
  );
}

function AccountsTab() {
  const accounts = useAccounts();
  const user = useSession((s) => s.user);
  const switchAccount = (account: Account) => {
    setAccessToken(account.accessToken);
    useSession.setState({ status: 'authed', user: account.user });
  };
  const remove = (account: Account) => {
    switchAccount(account);
    if (account.user.id === user?.id) signOutLocal();
  };
  return (
    <ul className="divide-y divide-zinc-700">
      {accounts.map((account) => (
        <li key={account.user.id} className="flex items-center justify-between py-2 text-sm">
          <span className="truncate">
            <span className="font-medium">{account.user.username}</span>
            {account.user.id === user?.id && <span className="text-xs text-emerald-400"> • текущий</span>}
          </span>
          <div className="flex gap-2">
            <button onClick={() => switchAccount(account)} className="text-xs text-indigo-400 hover:underline">Открыть</button>
            <button onClick={() => remove(account)} className="text-xs text-red-400 hover:underline">Удалить</button>
          </div>
        </li>
      ))}
      {accounts.length === 0 && <li className="py-2 text-sm text-zinc-500">Аккаунтов пока нет.</li>}
    </ul>
  );
}

function SecurityTab({ user }: { user: User }) {
  const [setup, setSetup] = useState<TwoFactorSetup | null>(null);
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState('');
  const start = () => api<TwoFactorSetup>('/api/me/2fa/setup', { method: 'POST' })
    .then((s) => { setSetup(s); setInfo('Сохраните коды восстановления и отсканируйте секрет в приложении.'); }, (e) => setError(errorMessage(e)));
  const enable = async (e: FormEvent) => {
    e.preventDefault();
    try {
      await api('/api/me/2fa/enable', { method: 'POST', body: { code } satisfies TwoFactorCode });
      useSession.setState({ user: { ...user, twoFactorEnabled: true } });
      setInfo('2FA включена'); setSetup(null);
    } catch (err) { setError(errorMessage(err)); }
  };
  const disable = async (e: FormEvent) => {
    e.preventDefault();
    try {
      await api('/api/me/2fa/disable', { method: 'POST', body: { code } satisfies TwoFactorCode });
      useSession.setState({ user: { ...user, twoFactorEnabled: false } });
      setInfo('2FA отключена');
    } catch (err) { setError(errorMessage(err)); }
  };
  const exportData = async () => {
    try {
      const res = await api<ExportResponse>('/api/me/export');
      const blob = new Blob([res.data], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = 'nexuschat-export.json'; a.click();
      URL.revokeObjectURL(url);
    } catch (err) { setError(errorMessage(err)); }
  };
  return (
    <div className="space-y-3">
      <p className="text-sm">Двухфакторная аутентификация: {user.twoFactorEnabled ? 'включена' : 'отключена'}</p>
      {!setup && <button onClick={start} className={primaryButton}>{user.twoFactorEnabled ? 'Сменить секрет' : 'Настроить 2FA'}</button>}
      {setup && (
        <div className="space-y-2 rounded bg-zinc-900 p-3">
          <p className="text-sm">Секрет: <code className="break-all">{setup.secret}</code></p>
          <p className="text-xs text-zinc-400">{setup.uri}</p>
          <div className="grid grid-cols-2 gap-1 font-mono text-xs">
            {setup.recoveryCodes.map((c) => <span key={c}>{c}</span>)}
          </div>
          <form onSubmit={enable} className="flex gap-2">
            <input value={code} onChange={(e) => setCode(e.target.value)} className={inputClass} placeholder="Код из приложения" />
            <button className={primaryButton}>Включить</button>
          </form>
        </div>
      )}
      {user.twoFactorEnabled && !setup && (
        <form onSubmit={disable} className="flex gap-2">
          <input value={code} onChange={(e) => setCode(e.target.value)} className={inputClass} placeholder="Код для отключения" />
          <button className={ghostButton}>Отключить 2FA</button>
        </form>
      )}
      <button onClick={() => void exportData()} className={ghostButton}>Выгрузить мои данные</button>
      {info && <p className="text-sm text-emerald-400">{info}</p>}
      {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
    </div>
  );
}
