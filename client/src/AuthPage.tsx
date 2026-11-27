import { useState, type FormEvent } from 'react';
import { Link, Navigate } from 'react-router';
import { login, loginWithCode, register, useSession } from './session';

export function AuthPage({ mode }: { mode: 'login' | 'register' }) {
  const status = useSession((s) => s.status);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [show2fa, setShow2fa] = useState(false);
  const [lastCreds, setLastCreds] = useState<{ email: string; password: string } | null>(null);

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
      if (isRegister) {
        await register({ email, password, username: String(form.get('username')) });
      } else if (show2fa || code) {
        await loginWithCode({ email, password, code, recoveryCode: '' });
      } else {
        await login({ email, password });
        setLastCreds({ email, password });
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Что-то пошло не так, попробуйте ещё раз';
      if (msg.includes('2fa required') && !show2fa) {
        setShow2fa(true);
        setLastCreds({ email, password });
        setError('');
      } else {
        setError(msg);
      }
    } finally {
      setPending(false);
    }
  };

  return (
    <main className="grid min-h-screen place-items-center bg-[#0d0d0d] p-4 text-[#e5e7eb]">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-sm space-y-4 rounded-xl border border-[#242424] bg-[#121212] p-8 shadow-[0_24px_64px_rgba(0,0,0,.6)]"
      >
        <img src={`${import.meta.env.BASE_URL}favicon.svg`} alt="" width={64} height={64} className="mx-auto" />
        <h1 className="text-center text-2xl font-semibold">
          {isRegister ? 'Создать аккаунт' : 'С возвращением!'}
        </h1>

        <Field label="Email" name="email" type="email" autoComplete="email" defaultValue={lastCreds?.email ?? ''} />
        {isRegister && (
          <Field label="Имя пользователя" name="username" minLength={2} maxLength={32} autoComplete="username" />
        )}
        <Field
          label="Пароль"
          name="password"
          type="password"
          minLength={isRegister ? 8 : undefined}
          autoComplete={isRegister ? 'new-password' : 'current-password'}
          defaultValue={lastCreds?.password ?? ''}
        />

        {show2fa && (
          <label className="block">
            <span className="mb-1.5 block text-xs font-semibold uppercase text-[#9ca3af]">Код 2FA</span>
            <input
              name="code"
              autoComplete="one-time-code"
              placeholder="Введите код из приложения"
              required={show2fa}
              className="w-full rounded-md border border-[#242424] bg-[#1e1e1e] px-3 py-2 outline-none transition-colors duration-150 focus:ring-2 focus:ring-nc-accent/50"
            />
          </label>
        )}

        {error && (
          <p role="alert" className="text-sm text-red-400">
            {error}
          </p>
        )}

        <button
          disabled={pending}
          className="w-full rounded-lg bg-[#4f46e5] py-2.5 font-semibold text-white transition-colors duration-150 hover:bg-[#4338ca] disabled:opacity-60"
        >
          {isRegister ? 'Продолжить' : 'Вход'}
        </button>

        <p className="text-sm text-[#9ca3af]">
          {isRegister ? 'Уже есть аккаунт? ' : 'Нужна учётная запись? '}
          <Link to={isRegister ? '/login' : '/register'} className="text-[#a5b4fc] hover:underline">
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
      <span className="mb-1.5 block text-xs font-semibold uppercase text-[#9ca3af]">{label}</span>
      <input required {...input} className="w-full rounded-md border border-[#242424] bg-[#1e1e1e] px-3 py-2 outline-none transition-colors duration-150 focus:ring-2 focus:ring-nc-accent/50" />
    </label>
  );
}
