// Биты прав должны совпадать с server/src/permissions.rs.
export const P = {
  VIEW_CHANNEL: 1 << 0,
  SEND_MESSAGES: 1 << 1,
  ATTACH_FILES: 1 << 2,
  ADD_REACTIONS: 1 << 3,
  MENTION_EVERYONE: 1 << 4,
  READ_MESSAGE_HISTORY: 1 << 5,
  MANAGE_MESSAGES: 1 << 6,
  MANAGE_CHANNELS: 1 << 7,
  MANAGE_ROLES: 1 << 8,
  KICK_MEMBERS: 1 << 9,
  BAN_MEMBERS: 1 << 10,
  TIMEOUT_MEMBERS: 1 << 11,
  CREATE_INVITE: 1 << 12,
  CHANGE_NICKNAME: 1 << 13,
  MANAGE_NICKNAMES: 1 << 14,
  CREATE_THREADS: 1 << 15,
  MANAGE_THREADS: 1 << 16,
  VIEW_AUDIT_LOG: 1 << 17,
  CONNECT_VOICE: 1 << 18,
  SPEAK: 1 << 19,
  MUTE_MEMBERS: 1 << 20,
  DEAFEN_MEMBERS: 1 << 21,
  MOVE_MEMBERS: 1 << 22,
  PRIORITY_SPEAKER: 1 << 23,
  STREAM: 1 << 24,
  ADMINISTRATOR: 1 << 25,
  PIN_MESSAGES: 1 << 26,
} as const;

export const ALL_PERMISSIONS =
  (1 << 27) - 1;

export interface PermissionItem {
  bit: number;
  label: string;
  hint?: string;
}
export interface PermissionGroup {
  title: string;
  items: PermissionItem[];
}

export const PERMISSION_GROUPS: PermissionGroup[] = [
  {
    title: 'Общие',
    items: [
      { bit: P.VIEW_CHANNEL, label: 'Просмотр каналов' },
      { bit: P.MANAGE_CHANNELS, label: 'Управление каналами' },
      { bit: P.MANAGE_ROLES, label: 'Управление ролями' },
      { bit: P.CREATE_INVITE, label: 'Создавать приглашения' },
      { bit: P.VIEW_AUDIT_LOG, label: 'Журнал аудита' },
    ],
  },
  {
    title: 'Участники',
    items: [
      { bit: P.KICK_MEMBERS, label: 'Выгонять участников' },
      { bit: P.BAN_MEMBERS, label: 'Банить участников' },
      { bit: P.TIMEOUT_MEMBERS, label: 'Выдавать тайм-аут' },
      { bit: P.MANAGE_NICKNAMES, label: 'Управлять никами' },
      { bit: P.CHANGE_NICKNAME, label: 'Менять свой ник' },
    ],
  },
  {
    title: 'Текст',
    items: [
      { bit: P.SEND_MESSAGES, label: 'Отправлять сообщения' },
      { bit: P.ATTACH_FILES, label: 'Прикреплять файлы' },
      { bit: P.ADD_REACTIONS, label: 'Ставить реакции' },
      { bit: P.MENTION_EVERYONE, label: 'Упоминать @everyone' },
      { bit: P.READ_MESSAGE_HISTORY, label: 'Читать историю' },
      { bit: P.MANAGE_MESSAGES, label: 'Удалять чужие сообщения' },
      { bit: P.PIN_MESSAGES, label: 'Закреплять сообщения' },
      { bit: P.CREATE_THREADS, label: 'Создавать ветки' },
      { bit: P.MANAGE_THREADS, label: 'Управлять ветками' },
    ],
  },
  {
    title: 'Голос',
    items: [
      { bit: P.CONNECT_VOICE, label: 'Входить в голос' },
      { bit: P.SPEAK, label: 'Говорить' },
      { bit: P.STREAM, label: 'Видео и трансляция' },
      { bit: P.MUTE_MEMBERS, label: 'Мьютить участников' },
      { bit: P.DEAFEN_MEMBERS, label: 'Заглушать участников' },
      { bit: P.MOVE_MEMBERS, label: 'Перемещать участников' },
      { bit: P.PRIORITY_SPEAKER, label: 'Приоритетный режим' },
    ],
  },
  {
    title: 'Особые',
    items: [
      {
        bit: P.ADMINISTRATOR,
        label: 'Администратор',
        hint: 'Все права и обход переопределений каналов',
      },
    ],
  },
];

export const hasPerm = (bits: number, bit: number) => (bits & bit) === bit;

export const togglePerm = (bits: number, bit: number, on: boolean) =>
  on ? bits | bit : bits & ~bit;

export const colorToCss = (color: number | null) =>
  color === null ? undefined : `#${color.toString(16).padStart(6, '0')}`;