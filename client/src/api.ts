import type { ApiError } from './generated/ApiError';
import type { Attachment } from './generated/Attachment';
import type { AuthResponse } from './generated/AuthResponse';
import { signOutLocal } from './session';

// Access-токен живёт только в памяти вкладки; refresh лежит в httpOnly cookie.
let accessToken: string | null = null;

export const getAccessToken = () => accessToken;
export const setAccessToken = (token: string | null) => {
  accessToken = token;
};

export class HttpError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

// Сообщения сервера (src/*.rs) по-русски; неизвестные показываются как есть.
const ru: Record<string, string> = {
  'invalid email or password': 'Неверный email или пароль',
  'email already registered': 'Этот email уже зарегистрирован',
  'username already taken': 'Это имя пользователя уже занято',
  'invalid email': 'Некорректный email',
  'username must be 2-32 characters': 'Имя пользователя: от 2 до 32 символов',
  'password must be 8-72 bytes': 'Пароль: от 8 символов (не длиннее 72 байт)',
  'server name must be 2-100 characters': 'Название сервера: от 2 до 100 символов',
  "channel name must be 1-32 letters, digits, '-' or '_'":
    'Название канала: до 32 символов, только буквы, цифры, «-» и «_»',
  'invite not found': 'Приглашение не найдено',
  'invite expired': 'Приглашение истекло или исчерпано',
  'server not found': 'Сервер не найден',
  'channel not found': 'Канал не найден',
  forbidden: 'Недостаточно прав',
  'file is larger than 10 MB': 'Файл больше 10 МБ',
  'unsupported file type': 'Такой тип файла нельзя отправить: только png, jpg, gif, webp, pdf и zip',
  'invalid attachments': 'Вложение не найдено или уже отправлено',
  'too many attachments': 'Не больше 10 файлов в одном сообщении',
  'user not found': 'Пользователь не найден',
  'cannot message yourself': 'Нельзя написать самому себе',
  'message must be 1-2000 characters': 'Сообщение: от 1 до 2000 символов',
  'message not found': 'Сообщение не найдено',
  'username is required': 'Укажите имя пользователя',
  'cannot friend yourself': 'Нельзя добавить самого себя',
  'already friends': 'Вы уже друзья',
  'request already sent': 'Заявка уже отправлена',
  'incoming request exists': 'Этот пользователь уже отправил вам заявку',
  'cannot remove yourself': 'Нельзя удалить самого себя',
  'cannot block yourself': 'Нельзя заблокировать самого себя',
  blocked: 'Действие недоступно: пользователь в блокировке',
  'a group needs 3-10 people': 'В группе должно быть от 3 до 10 человек',
  'group name is too long': 'Название группы слишком длинное',
};

async function parse<T>(res: Response): Promise<T> {
  if (res.ok) return (res.status === 204 ? undefined : await res.json()) as T;
  const body = (await res.json().catch(() => null)) as ApiError | null;
  const message = body?.error ?? res.statusText;
  throw new HttpError(res.status, ru[message] ?? message);
}

let refreshing: Promise<AuthResponse> | null = null;

/** Обновляет пару токенов по cookie. Параллельные вызовы делят один запрос. */
export function refreshSession(): Promise<AuthResponse> {
  refreshing ??= fetch('/api/auth/refresh', { method: 'POST' })
    .then((res) => parse<AuthResponse>(res))
    .then((auth) => {
      accessToken = auth.accessToken;
      return auth;
    })
    .finally(() => {
      refreshing = null;
    });
  return refreshing;
}

/** JSON-запрос к API. При 401 один раз обновляет токен и повторяет запрос. */
export async function api<T>(
  path: string,
  init: { method?: string; body?: unknown; form?: FormData } = {},
): Promise<T> {
  const send = () =>
    fetch(path, {
      method: init.method ?? 'GET',
      headers: {
        ...(init.body !== undefined && { 'Content-Type': 'application/json' }),
        ...(accessToken && { Authorization: `Bearer ${accessToken}` }),
      },
      body: init.form ?? (init.body === undefined ? undefined : JSON.stringify(init.body)),
    });

  let res = await send();
  if (res.status === 401 && accessToken && !path.startsWith('/api/auth/')) {
    try {
      await refreshSession();
    } catch (e) {
      signOutLocal(); // refresh тоже истёк: сессия закончилась
      throw e;
    }
    res = await send();
  }
  return parse<T>(res);
}

/**
 * Загружает файл в канал. Через XHR, потому что fetch не сообщает прогресс отправки.
 * При 401 один раз обновляет токен и повторяет, как `api`.
 */
export async function uploadFile(
  channelId: number,
  file: File,
  onProgress: (fraction: number) => void,
): Promise<Attachment> {
  const attempt = () =>
    new Promise<{ status: number; body: unknown }>((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open('POST', `/api/channels/${channelId}/attachments`);
      if (accessToken) xhr.setRequestHeader('Authorization', `Bearer ${accessToken}`);
      xhr.upload.onprogress = (e) => e.lengthComputable && onProgress(e.loaded / e.total);
      xhr.onload = () => {
        let body: unknown = null;
        try {
          body = JSON.parse(xhr.responseText);
        } catch {
          // тело не JSON (например, ошибка прокси) — ниже сработает текст по статусу
        }
        resolve({ status: xhr.status, body });
      };
      xhr.onerror = () => reject(new Error('Не удалось загрузить файл: нет соединения'));
      const form = new FormData();
      form.append('file', file);
      xhr.send(form);
    });

  let res = await attempt();
  if (res.status === 401 && accessToken) {
    await refreshSession();
    res = await attempt();
  }
  if (res.status >= 200 && res.status < 300) return res.body as Attachment;
  const message =
    (res.body as ApiError | null)?.error ?? (res.status === 413 ? 'file is larger than 10 MB' : 'Не удалось загрузить файл');
  throw new HttpError(res.status, ru[message] ?? message);
}
