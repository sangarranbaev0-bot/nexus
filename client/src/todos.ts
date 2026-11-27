import { create } from 'zustand';

interface TodosState {
  done: Record<string, boolean>;
  toggle(msgId: number, idx: number): void;
}

export const useTodos = create<TodosState>((set) => ({
  done: {},
  toggle: (msgId, idx) =>
    set((s) => {
      const key = `${msgId}-${idx}`;
      return { done: { ...s.done, [key]: !s.done[key] } };
    }),
}));
