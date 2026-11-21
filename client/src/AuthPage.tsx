import { useState, type FormEvent } from 'react';
import { Link, Navigate } from 'react-router';
import { login, loginWithCode, register, useSession } from './session';

export function AuthPage({ mode }: { mode: 'login' | 'register' }) {
  const status = useSession((s) => s.status);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  if (status === 'authed') return <Navigate to="/channels" replace />;

  const isRegister = mode === 'register';

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const form = new FormData(e.currentTarget);
    const email = String(form.get('email'));
    const password = String(form.get('password'));
    const code = String(form.get('code') ?? '');
    setPending(true);
    setError(null);
    try {
      if (isRegister) await register({ email, password, username: String(form.get('username')) });
      else if (code) await loginWithCode({ email, password, code, recoveryCode: '' });
      else await login({ email, password });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Что-то пошло не так, попробуйте ещё раз');
    } finally {
      setPending(false);
    }
  };

  return (
    <main className="grid min-h-screen place-items-center bg-indigo-950 p-4 text-zinc-100">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-sm space-y-4 rounded-lg bg-zinc-800 p-8 shadow-xl"
      >
        <h1 className="text-center text-2xl font-semibold">
          {isRegister ? 'Создать аккаунт' : 'С возвращением!'}
        </h1>

        <Field label="Email" name="email" type="email" autoComplete="email" />
        {isRegister && (
          <Field label="Имя пользователя" name="username" minLength={2} maxLength={32} autoComplete="username" />
        )}
        <Field
          label="Пароль"
          name="password"
          type="password"
          minLength={isRegister ? 8 : undefined}
          autoComplete={isRegister ? 'new-password' : 'current-password'}
        />

        {!isRegister && <Field label="Код 2FA (если включён)" name="code" autoComplete="one-time-code" />}

        {error && (
          <p role="alert" className="text-sm text-red-400">
            {error}
          </p>
        )}

        <button
          disabled={pending}
          className="w-full rounded bg-indigo-500 py-2.5 font-medium hover:bg-indigo-600 disabled:opacity-60"
        >
          {isRegister ? 'Продолжить' : 'Вход'}
        </button>

        <p className="text-sm text-zinc-400">
          {isRegister ? 'Уже есть аккаунт? ' : 'Нужна учётная запись? '}
          <Link to={isRegister ? '/login' : '/register'} className="text-indigo-400 hover:underline">
            {isRegister ? 'Войти' : 'Зарегистрироваться'}
          </Link>
        </p>
      </form>
    </main>
  );
}

function Field({ label, ...input }: { label: string } & React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-xs font-semibold uppercase text-zinc-400">{label}</span>
      <input required {...input} className="w-full rounded bg-zinc-900 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500" />
    </label>
  );
}
