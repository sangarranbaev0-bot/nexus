import { useState, type ReactNode } from 'react';
import { useSession } from './session';
import { useEmojis } from './emojis';

/**
 * Безопасный рендер Discord-подобного Markdown: без HTML и dangerouslySetInnerHTML.
 * Поддерживает **жирный**, *курсив*, __подчёркнутый__, ~~зачёркнутый~~,
 * ||спойлер||, `код`, блоки кода, цитаты, списки, заголовки и <t:unix:R>.
 */
export function Markdown({ text }: { text: string }) {
  const me = useSession((s) => s.user?.username);
  const byServer = useEmojis((s) => s.byServer);
  const lookup = (name: string) => {
    for (const list of Object.values(byServer)) {
      const hit = list?.find((e) => e.name === name);
      if (hit) return hit.url;
    }
    return undefined;
  };
  return <div className="space-y-1 break-words whitespace-pre-wrap">{blocks(text, me, lookup)}</div>;
}

type EmojiLookup = (name: string) => string | undefined;

function blocks(text: string, me?: string, emoji?: EmojiLookup): ReactNode[] {
  const lines = text.split('\n');
  const out: ReactNode[] = [];
  let i = 0;
  let key = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (line.trim() === '') {
      i++;
      continue;
    }

    const fence = /^```(.*)$/.exec(line);
    if (fence) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) code.push(lines[i++]);
      if (i < lines.length) i++;
      out.push(
        <pre key={key++} className="overflow-x-auto rounded bg-zinc-900 p-2 text-sm">
          <code>{code.join('\n')}</code>
        </pre>,
      );
      continue;
    }

    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const cls =
        level === 1
          ? 'text-2xl font-bold'
          : level === 2
            ? 'text-xl font-bold'
            : level === 3
              ? 'text-lg font-semibold'
              : 'font-semibold';
      out.push(
        <p key={key++} className={cls}>
          {inline(heading[2], me, emoji)}
        </p>,
      );
      i++;
      continue;
    }

    if (line.startsWith('>')) {
      const quote: string[] = [];
      while (i < lines.length && lines[i].startsWith('>')) {
        quote.push(lines[i].replace(/^>\s?/, ''));
        i++;
      }
      out.push(
        <blockquote key={key++} className="border-l-2 border-zinc-600 pl-2 text-zinc-300">
          {inline(quote.join('\n'), me, emoji)}
        </blockquote>,
      );
      continue;
    }

    const unordered = /^\s*[-*]\s+/.test(line);
    const ordered = /^\s*\d+\.\s+/.test(line);
    if (unordered || ordered) {
      const items: string[] = [];
      const re = unordered ? /^\s*[-*]\s+/ : /^\s*\d+\.\s+/;
      while (i < lines.length && re.test(lines[i])) {
        items.push(lines[i].replace(re, ''));
        i++;
      }
      const children = items.map((item, n) => <li key={n}>{inline(item, me, emoji)}</li>);
      out.push(
        unordered ? (
          <ul key={key++} className="list-disc pl-5">
            {children}
          </ul>
        ) : (
          <ol key={key++} className="list-decimal pl-5">
            {children}
          </ol>
        ),
      );
      continue;
    }

    const paragraph: string[] = [];
    while (i < lines.length && lines[i].trim() !== '' && !isBlockStart(lines[i])) {
      paragraph.push(lines[i++]);
    }
    out.push(
      <p key={key++} className="whitespace-pre-wrap">
        {inline(paragraph.join('\n'), me, emoji)}
      </p>,
    );
  }

  return out;
}

function isBlockStart(line: string) {
  return (
    line.startsWith('```') ||
    line.startsWith('>') ||
    /^(#{1,6})\s+/.test(line) ||
    /^\s*[-*]\s+/.test(line) ||
    /^\s*\d+\.\s+/.test(line)
  );
}

const PAIRS: { open: string; close: string; tag: 'strong' | 'u' | 'del' | 'em' }[] = [
  { open: '**', close: '**', tag: 'strong' },
  { open: '__', close: '__', tag: 'u' },
  { open: '~~', close: '~~', tag: 'del' },
  { open: '*', close: '*', tag: 'em' },
];

function inline(text: string, me?: string, emoji?: EmojiLookup): ReactNode[] {
  const nodes: ReactNode[] = [];
  let buf = '';
  let i = 0;
  let key = 0;
  const flush = () => {
    if (buf) {
      nodes.push(buf);
      buf = '';
    }
  };

  while (i < text.length) {
    const rest = text.slice(i);

    if (text[i] === '`') {
      const end = text.indexOf('`', i + 1);
      if (end > i + 1) {
        flush();
        nodes.push(
          <code key={key++} className="rounded bg-zinc-900 px-1 text-sm">
            {text.slice(i + 1, end)}
          </code>,
        );
        i = end + 1;
        continue;
      }
    }

    if (text.startsWith('||', i)) {
      const end = text.indexOf('||', i + 2);
      if (end > i + 1) {
        flush();
        nodes.push(<Spoiler key={key++}>{inline(text.slice(i + 2, end), me, emoji)}</Spoiler>);
        i = end + 2;
        continue;
      }
    }

const ts = /^<t:(\d+)(?::([tTdDfFR]))?>/.exec(rest);
    if (ts) {
      flush();
      nodes.push(<Timestamp key={key++} ms={Number(ts[1])} style={ts[2]} />);
      i += ts[0].length;
      continue;
    }

    // Упоминания не внутри слова/email: перед @ или # должен быть пробел/начало.
    const before = i === 0 ? '' : text[i - 1];
    if ((before === '' || /\s|[([{>]/.test(before)) && /^https?:\/\/\S+/.test(rest)) {
      const url = /^https?:\/\/[^\s<]+/.exec(rest)!;
      flush();
      nodes.push(
        <a key={key++} href={url[0]} target="_blank" rel="noopener noreferrer" className="text-sky-400 hover:underline">
          {url[0]}
        </a>,
      );
      i += url[0].length;
      continue;
    }
    if ((text[i] === '@' || text[i] === '#') && (before === '' || /\s|[([{>]/.test(before))) {
      const mention = /^([@#])([\p{L}\p{N}_.-]{1,32})/u.exec(rest);
      if (mention) {
        flush();
        const name = mention[2];
        if (mention[1] === '@') {
          const special = name === 'everyone' || name === 'here';
          const self = me?.toLowerCase() === name.toLowerCase();
          nodes.push(
            <span
              key={key++}
              className={
                special
                  ? 'rounded bg-amber-500/25 px-0.5 text-amber-200'
                  : self
                    ? 'rounded bg-indigo-500 px-0.5 font-medium text-white'
                    : 'rounded bg-indigo-500/30 px-0.5 text-indigo-200'
              }
            >
              @{name}
            </span>,
          );
        } else {
          nodes.push(
            <span key={key++} className="rounded bg-indigo-500/20 px-0.5 text-indigo-300">
              #{name}
            </span>,
          );
        }
        i += mention[0].length;
        continue;
      }
    }

    // Свои эмодзи сервера: :name: превращается в картинку, если такое имя загружено.
    if (emoji && text[i] === ':' && (before === '' || /\s|[([{>]/.test(before))) {
      const custom = /^:([a-z0-9_]{2,32}):/.exec(rest);
      const url = custom && emoji(custom[1]);
      if (custom && url) {
        flush();
        nodes.push(
          <img
            key={key++}
            src={url}
            alt={custom[0]}
            title={custom[0]}
            className="inline-block h-5 w-5 align-text-bottom"
          />,
        );
        i += custom[0].length;
        continue;
      }
    }

    let matched = false;
    for (const pair of PAIRS) {
      if (!text.startsWith(pair.open, i)) continue;
      const end = text.indexOf(pair.close, i + pair.open.length);
      if (end <= i + pair.open.length) continue;
      flush();
      const inner = inline(text.slice(i + pair.open.length, end), me, emoji);
      const Tag = pair.tag;
      nodes.push(<Tag key={key++}>{inner}</Tag>);
      i = end + pair.close.length;
      matched = true;
      break;
    }
    if (matched) continue;

    buf += text[i];
    i++;
  }

  flush();
  return nodes;
}

function Spoiler({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <span
      role="button"
      tabIndex={0}
      onClick={(e) => {
        e.stopPropagation();
        setOpen(true);
      }}
      onKeyDown={(e) => e.key === 'Enter' && setOpen(true)}
      className={
        open
          ? 'rounded bg-zinc-700/60 px-0.5'
          : 'cursor-pointer rounded bg-zinc-600 px-0.5 text-transparent select-none hover:bg-zinc-500'
      }
    >
      {children}
    </span>
  );
}

const timeFmt: Record<string, Intl.DateTimeFormat> = {
  t: new Intl.DateTimeFormat('ru', { hour: '2-digit', minute: '2-digit' }),
  T: new Intl.DateTimeFormat('ru', { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
  d: new Intl.DateTimeFormat('ru', { day: '2-digit', month: '2-digit', year: 'numeric' }),
  D: new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'long', year: 'numeric' }),
  f: new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }),
  F: new Intl.DateTimeFormat('ru', { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' }),
};

const relative = new Intl.RelativeTimeFormat('ru', { numeric: 'auto' });

function relativeText(ms: number) {
  const diff = Math.round((ms - Date.now()) / 1000);
  const abs = Math.abs(diff);
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31_536_000],
    ['month', 2_592_000],
    ['day', 86_400],
    ['hour', 3_600],
    ['minute', 60],
  ];
  for (const [unit, size] of units) {
    if (abs >= size) return relative.format(Math.round(diff / size), unit);
  }
  return relative.format(diff, 'second');
}

function Timestamp({ ms, style }: { ms: number; style?: string }) {
  const date = new Date(ms);
  const text = style === 'R' ? relativeText(ms) : (timeFmt[style ?? 'f']?.format(date) ?? date.toLocaleString('ru'));
  return (
    <time
      dateTime={date.toISOString()}
      title={date.toLocaleString('ru')}
      className="rounded bg-zinc-700/60 px-1 text-xs text-zinc-200"
    >
      {text}
    </time>
  );
}
