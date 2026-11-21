import { create } from 'zustand';
import { appendAccount } from './auth';
import { api, refreshSession, setAccessToken } from './api';
import { useDms } from './dms';
import { resetFriends } from './friends';
import { useGuilds } from './guilds';
import type { AuthResponse } from './generated/AuthResponse';
import type { LoginRequest } from './generated/LoginRequest';
import type { RegisterRequest } from './generated/RegisterRequest';
import type { User } from './generated/User';

interface SessionState {
  status: 'loading' | 'authed' | 'anon';
  user: User | null;
}

export const useSession = create<SessionState>(() => ({ status: 'loading', user: null }));

const signIn = (auth: AuthResponse) => {
  setAccessToken(auth.accessToken);
  useSession.setState({ status: 'authed', user: auth.user });
  appendAccount({ accessToken: auth.accessToken, user: auth.user });
};

export const signOutLocal = () => {
  setAccessToken(null);
  useSession.setState({ status: 'anon', user: null });
  useGuilds.setState({ servers: null, channels: {} });
  useDms.setState({ dms: null });
  resetFriends();
};

/** При загрузке страницы пытаемся восстановить сессию по refresh-cookie. */
export async function restoreSession() {
  try {
    signIn(await refreshSession());
  } catch {
    signOutLocal();
  }
}

export async function loginWithCode(req: import('./generated/LoginWithCode').LoginWithCode) {
  const res = await fetch('/api/auth/login-2fa', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error((await res.json().catch(() => ({ error: 'Неверный код' }))).error);
  signIn(await res.json());
}

export async function login(req: LoginRequest) {
  signIn(await api<AuthResponse>('/api/auth/login', { method: 'POST', body: req }));
}

export async function register(req: RegisterRequest) {
  signIn(await api<AuthResponse>('/api/auth/register', { method: 'POST', body: req }));
}

export async function logout() {
  await api('/api/auth/logout', { method: 'POST' }).catch(() => {});
  signOutLocal();
}
