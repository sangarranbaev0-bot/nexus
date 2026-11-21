import { create } from 'zustand';
import type { User } from './generated/User';

export interface Account {
  accessToken: string;
  user: User;
}

const readAccounts = (): Account[] => {
  try {
    const raw = JSON.parse(localStorage.getItem('nexus-accounts') ?? '[]');
    return Array.isArray(raw) ? raw : [];
  } catch {
    return [];
  }
};

export const useAccounts = create<Account[]>(() => readAccounts());

export const appendAccount = (account: Account) => {
  const accounts = useAccounts.getState();
  if (accounts.some((a) => a.user.id === account.user.id)) return;
  const next = [...accounts, account];
  localStorage.setItem('nexus-accounts', JSON.stringify(next));
  useAccounts.setState(next);
};
