import { create } from 'zustand';
import type { Member } from './generated/Member';
import type { Presence } from './generated/Presence';

/** Последний известный статус каждого пользователя. */
export const usePresence = create<Record<number, Presence>>(() => ({}));

export const rememberStatuses = (members: Member[]) =>
  usePresence.setState(Object.fromEntries(members.map((m) => [m.userId, m.status])));

export const applyPresence = (userId: number, status: Presence) => usePresence.setState({ [userId]: status });

export const presenceView: Record<Presence, { dot: string; label: string }> = {
  online: { dot: 'bg-emerald-500', label: 'В сети' },
  idle: { dot: 'bg-amber-400', label: 'Неактивен' },
  offline: { dot: 'bg-nc-strong', label: 'Не в сети' },
};

const IDLE_AFTER_MS = 5 * 60 * 1000;
const CHECK_EVERY_MS = 15 * 1000;
const ACTIVITY = ['mousemove', 'mousedown', 'keydown', 'touchstart', 'wheel'] as const;

/** Следит за бездействием вкладки и сообщает о смене состояния. Возвращает отписку. */
export function watchIdle(onChange: (idle: boolean) => void): () => void {
  let last = Date.now();
  let idle = false;
  const update = (next: boolean) => {
    if (next !== idle) onChange((idle = next));
  };
  const onActivity = () => {
    last = Date.now();
    update(false);
  };
  ACTIVITY.forEach((e) => window.addEventListener(e, onActivity, { passive: true }));
  const timer = window.setInterval(() => update(Date.now() - last > IDLE_AFTER_MS), CHECK_EVERY_MS);
  return () => {
    ACTIVITY.forEach((e) => window.removeEventListener(e, onActivity));
    window.clearInterval(timer);
  };
}
