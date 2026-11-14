import { useEffect, useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router';
import { errorMessage } from './Modal';
import { openDm } from './dms';
import {
  acceptFriendRequest,
  blockUser,
  cancelFriendRequest,
  loadAllFriends,
  removeFriend,
  sendFriendRequest,
  unblockUser,
  useFriends,
} from './friends';
import type { Friend } from './generated/Friend';
import type { FriendRequest } from './generated/FriendRequest';
import { presenceView, usePresence } from './presence';

type Tab = 'friends' | 'requests' | 'blocked';

export function FriendsPage() {
  const navigate = useNavigate();
  const { friends, incoming, outgoing, blocks, revision } = useFriends();
  const [tab, setTab] = useState<Tab>('friends');
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    setError(null);
    loadAllFriends().catch((e) => setError(errorMessage(e)));
  }, [revision]);

  const openChat = async (userId: number, username: string) => {
    try {
      const dm = await openDm(username);
      navigate(`/channels/@me/${dm.id}`);
      void userId;
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const action = (p: Promise<void>) => {
    setError(null);
    p.catch((e) => setError(errorMessage(e)));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-3 border-b border-zinc-900 px-4 py-2">
        <nav className="flex gap-3 text-sm font-medium" aria-label="Разделы друзей">
          <TabButton active={tab === 'friends'} onClick={() => setTab('friends')}>
            Все
          </TabButton>
          <TabButton active={tab === 'requests'} onClick={() => setTab('requests')}>
            Заявки{incoming.length > 0 && <span className="ml-1 rounded-full bg-red-500 px-1.5 text-xs text-white">{incoming.length}</span>}
          </TabButton>
          <TabButton active={tab === 'blocked'} onClick={() => setTab('blocked')}>
            Заблокированные
          </TabButton>
        </nav>
        <button
          type="button"
          onClick={() => setAdding((v) => !v)}
          className="ml-auto rounded bg-indigo-500 px-3 py-1.5 text-sm font-medium hover:bg-indigo-600"
        >
          Добавить друга
        </button>
      </div>

      {error && (
        <p role="alert" className="px-4 py-2 text-sm text-red-400">
          {error}
        </p>
      )}

      {adding && <AddFriendForm onDone={() => setAdding(false)} onError={setError} />}

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {tab === 'friends' && (
          <FriendList friends={friends} onOpen={openChat} onAction={action} />
        )}
        {tab === 'requests' && (
          <RequestsList
            incoming={incoming}
            outgoing={outgoing}
            onAction={action}
            onLoadError={setError}
          />
        )}
        {tab === 'blocked' && (
          <BlockedList blocks={blocks} onAction={action} />
        )}
      </div>
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded px-2 py-1 ${active ? 'bg-zinc-700 text-white' : 'text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200'}`}
    >
      {children}
    </button>
  );
}

function AddFriendForm({ onDone, onError }: { onDone: () => void; onError: (e: string) => void }) {
  const [value, setValue] = useState('');
  const [pending, setPending] = useState(false);

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const username = value.trim();
    if (!username) return;
    setPending(true);
    try {
      await sendFriendRequest(username);
      setValue('');
      onDone();
    } catch (err) {
      onError(errorMessage(err));
    } finally {
      setPending(false);
    }
  };

  return (
    <form onSubmit={submit} className="flex gap-2 border-b border-zinc-900 px-4 py-3">
      <input
        autoFocus
        value={value}
        onChange={(e) => setValue(e.target.value)}
        maxLength={32}
        placeholder="Имя пользователя"
        aria-label="Имя пользователя для заявки"
        className="w-72 rounded bg-zinc-900 px-3 py-1.5 text-sm outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <button
        type="submit"
        disabled={pending || !value.trim()}
        className="rounded bg-indigo-500 px-3 py-1.5 text-sm font-medium hover:bg-indigo-600 disabled:opacity-60"
      >
        Отправить заявку
      </button>
      <button type="button" onClick={onDone} className="rounded px-3 py-1.5 text-sm text-zinc-400 hover:text-zinc-100">
        Отмена
      </button>
    </form>
  );
}

function FriendList({
  friends,
  onOpen,
  onAction,
}: {
  friends: Friend[] | null;
  onOpen: (id: number, username: string) => void;
  onAction: (p: Promise<void>) => void;
}) {
  if (friends === null) return <p className="text-zinc-400">Загрузка…</p>;
  if (friends.length === 0)
    return <p className="text-zinc-400">Пока нет друзей. Добавьте кого-нибудь по имени.</p>;
  return (
    <ul className="space-y-1">
      {friends.map((f) => (
        <FriendRow key={f.userId} friend={f} onOpen={onOpen} onAction={onAction} />
      ))}
    </ul>
  );
}

function FriendRow({
  friend,
  onOpen,
  onAction,
}: {
  friend: Friend;
  onOpen: (id: number, username: string) => void;
  onAction: (p: Promise<void>) => void;
}) {
  const live = usePresence((s) => s[friend.userId]);
  const view = presenceView[live ?? friend.status];
  return (
    <li className="flex items-center gap-3 rounded px-3 py-2 hover:bg-zinc-900/50">
      <span title={view.label} className={`size-2.5 rounded-full ${view.dot}`} />
      <span className="flex-1 truncate">{friend.username}</span>
      <span className="text-xs text-zinc-500">{view.label}</span>
      <button
        type="button"
        onClick={() => void onOpen(friend.userId, friend.username)}
        className="rounded bg-zinc-700 px-2.5 py-1 text-sm hover:bg-zinc-600"
      >
        Написать
      </button>
      <button
        type="button"
        onClick={() => onAction(removeFriend(friend.userId))}
        className="rounded px-2.5 py-1 text-sm text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
      >
        Удалить
      </button>
      <button
        type="button"
        onClick={() => onAction(blockUser(friend.userId))}
        className="rounded px-2.5 py-1 text-sm text-red-400 hover:bg-zinc-700"
      >
        Заблокировать
      </button>
    </li>
  );
}

function RequestsList({
  incoming,
  outgoing,
  onAction,
  onLoadError,
}: {
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
  onAction: (p: Promise<void>) => void;
  onLoadError: (e: string) => void;
}) {
  return (
    <div className="space-y-6">
      <section>
        <h2 className="mb-2 text-sm font-semibold text-zinc-400">Входящие</h2>
        {incoming.length === 0 ? (
          <p className="text-sm text-zinc-500">Новых заявок нет.</p>
        ) : (
          <ul className="space-y-1">
            {incoming.map((r) => (
              <li key={r.id} className="flex items-center gap-3 rounded px-3 py-2 hover:bg-zinc-900/50">
                <span className="flex-1 truncate">{r.fromUsername}</span>
                <button
                  type="button"
                  onClick={() => onAction(acceptFriendRequest(r.id))}
                  className="rounded bg-indigo-500 px-2.5 py-1 text-sm font-medium hover:bg-indigo-600"
                >
                  Принять
                </button>
                <button
                  type="button"
                  onClick={() => onAction(cancelFriendRequest(r.id))}
                  className="rounded px-2.5 py-1 text-sm text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
                >
                  Отклонить
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h2 className="mb-2 text-sm font-semibold text-zinc-400">Исходящие</h2>
        {outgoing.length === 0 ? (
          <p className="text-sm text-zinc-500">Вы никого не добавляли.</p>
        ) : (
          <ul className="space-y-1">
            {outgoing.map((r) => (
              <li key={r.id} className="flex items-center gap-3 rounded px-3 py-2 hover:bg-zinc-900/50">
                <span className="flex-1 truncate">{r.toUsername}</span>
                <span className="text-xs text-zinc-500">Ожидает ответа</span>
                <button
                  type="button"
                  onClick={() => onAction(cancelFriendRequest(r.id))}
                  className="rounded px-2.5 py-1 text-sm text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
                >
                  Отменить
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
      <button
        type="button"
        onClick={() => loadAllFriends().catch((e) => onLoadError(errorMessage(e)))}
        className="text-sm text-zinc-400 hover:text-zinc-100"
      >
        Обновить
      </button>
    </div>
  );
}

function BlockedList({
  blocks,
  onAction,
}: {
  blocks: { userId: number; username: string }[] | null;
  onAction: (p: Promise<void>) => void;
}) {
  if (blocks === null) return <p className="text-zinc-400">Загрузка…</p>;
  if (blocks.length === 0) return <p className="text-zinc-400">Заблокированных нет.</p>;
  return (
    <ul className="space-y-1">
      {blocks.map((b) => (
        <li key={b.userId} className="flex items-center gap-3 rounded px-3 py-2 hover:bg-zinc-900/50">
          <span className="flex-1 truncate">{b.username}</span>
          <button
            type="button"
            onClick={() => onAction(unblockUser(b.userId))}
            className="rounded px-2.5 py-1 text-sm text-zinc-300 hover:bg-zinc-700"
          >
            Разблокировать
          </button>
        </li>
      ))}
    </ul>
  );
}
