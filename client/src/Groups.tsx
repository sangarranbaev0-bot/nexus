import { useNavigate } from 'react-router';
import { api } from './api';
import { dmTitle, loadDms } from './dms';
import type { DmChannel } from './generated/DmChannel';
import { errorMessage } from './Modal';

/** Заголовок группового диалога: название и участники. */
export function GroupHeader({ dm }: { dm: DmChannel }) {
  return (
    <>
      <span aria-hidden>👥</span>
      <span className="truncate">{dmTitle(dm)}</span>
      <span className="hidden truncate text-sm font-normal text-nc-muted md:inline">
        — {dm.members.map((m) => m.username).join(', ')}
      </span>
    </>
  );
}

/** Действия с группой: добавить участника, переименовать, выйти. */
export function GroupActions({ dm }: { dm: DmChannel }) {
  const navigate = useNavigate();
  const run = (action: () => Promise<unknown>) =>
    action()
      .then(() => loadDms())
      .catch((e) => alert(errorMessage(e)));
  const btn = 'rounded px-2 py-1 text-sm text-nc-muted hover:bg-nc-hover hover:text-nc-text';
  return (
    <>
      <button
        className={btn}
        onClick={() => {
          const username = prompt('Кого добавить в группу?');
          if (username?.trim())
            run(() =>
              api(`/api/dms/${dm.id}/members`, {
                method: 'POST',
                body: { username: username.trim() },
              }),
            );
        }}
      >
        + Участник
      </button>
      <button
        className={btn}
        onClick={() => {
          const name = prompt('Название группы (пусто — по участникам)', dm.name);
          if (name !== null)
            run(() => api(`/api/dms/${dm.id}`, { method: 'PATCH', body: { name } }));
        }}
      >
        Переименовать
      </button>
      <button
        className={`${btn} hover:text-red-400`}
        onClick={() => {
          if (!confirm(`Выйти из группы «${dmTitle(dm)}»?`)) return;
          run(() => api(`/api/dms/${dm.id}/members/me`, { method: 'DELETE' })).then(() =>
            navigate('/channels/@me'),
          );
        }}
      >
        Выйти
      </button>
    </>
  );
}
