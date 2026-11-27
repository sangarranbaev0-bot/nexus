import { useState, type ReactNode } from 'react';
import { useSession } from './session';
import { useEmojis } from './emojis';
import { useTodos } from './todos';

/**
 * Безопасный рендер Discord-подобного Markdown: без HTML и dangerouslySetInnerHTML.
 * Поддерживает **жирный**, *курсив*, __подчёркнутый__, ~~зачёркнутый~~,
 * ||спойлер||, `код`, блоки кода, цитаты, списки, заголовки, <t:unix:R>
 * и чеклисты: - [ ] задача / - [x] выполнено.
 */
export function Markdown({ text, messageId }: { text: string; messageId?: number }) {
  const me = useSession((s) => s.user?.username);
  const byServer = useEmojis((s) => s.byServer);
  const lookup = (name: string) => {
    for (const list of Object.values(byServer)) {
      const hit = list?.find((e) => e.name === name);
      if (hit) return hit.url;
    }
    return undefined;
  };
  return <div className="space-y-1 break-words whitespace-pre-wrap">{blocks(text, me, lookup, messageId)}</div>;
}

type EmojiLookup = (name: string) => string | undefined;

const STOCK_EMOJI: Record<string, string> = {
  // reactions
  thumbsup: '👍', thumbup: '👍', '+1': '👍',
  thumbsdown: '👎', thumbdown: '👎', '-1': '👎',
  OK_hand: '👌', peace: '✌️', v: '✌️',
  praying_hands: '🙏', clap: '👏', wave: '👋',
  muscle: '💪', flex: '💪',
  hands: '👐', open_hands: '👐',
  point_up: '☝️', point_down: '👇', point_left: '👈', point_right: '👉',
  crossed_fingers: '🤞', love_you_gesture: '🤟',
  metal: '🤘', sign_of_the_horns: '🤘',
  writing_hand: '✍️', selfie: '🤳',
  nail_polish: '💅', raising_hand: '🙋',
  person_raising_hand: '🙋', bow: '🙇',
  face_palm: '🤦', shrug: '🤷',
  // faces
  grinning: '😀', smiley: '😃', smile: '😄',
  grin: '😁', laughing: '😆', sweat_smile: '😅',
  roasted: '🤣', joyful: '😂', wink: '😉',
  blown_kiss: '😘', kissing_heart: '😘',
  kissing: '😗', kissing_smiling_eyes: '😙',
  kissing_face: '😚', relaxing: '😊',
  blush: '🙂', subtle_smile: '🙂',
  upside_down: '🙃', melted: '🫠',
  yum: '😋', stuck_out_tongue: '😛',
  stuck_out_tongue_winking_eye: '😜',
  zany_face: '🤪', money_mouth: '🤑',
  hugging: '🤗', hand_over_mouth: '🤭',
  shushing: '🤫', thinking: '🤔',
  raised_eyebrows: '🤨', sunglasses: '😎',
  star_struck: '🤩', partying: '🥳',
  woozy: '🥴', hot_face: '🥵', cold_face: '🥶',
  nauseated: '🤢', vomiting: '🤮', sneezing: '🤧',
  dizzy_face: '😵', exploding_head: '🤯',
  flushed: '😳', disappointed: '😞', worried: '😟',
  angry: '😠', rage: '😡', imp: '👿',
  skull: '💀', ghost: '👻', alien: '👽',
  robot: '🤖', poi: '🤡', crown: '👑',
  woomh: '🧐', smiling_imp: '😈',
   // emotions
   heart: '❤️', orange_heart: '🧡', yellow_heart: '💛',
   green_heart: '💚', blue_heart: '💙', purple_heart: '💜',
   black_heart: '🖤', white_heart: '🤍', broken_heart: '💔',
   two_hearts: '💕', revolving_hearts: '💞',
   beating_heart: '💓', heartbeat: '💓',
   heartpulse: '💗', sparkling_heart: '💖',
   cupid: '💘', gift_heart: '💝', love_letter: '💌',
   zzz: '💤', dash: '💨', sweat: '😓',
   sweat_drops: '💦', breath: '🌬️',
   fire: '🔥', sparkles: '✨', star: '⭐',
   stars: '🌟', shiny_spoon: '🥄', gem: '💎',
   guitar: '🎸', drum: '🥁',
  headphones: '🎧', microphone: '🎤', camera: '📷',
  video_camera: '📹', movie_camera: '🎥',
  computer: '💻', keyboard: '⌨️', phone: '📱',
  telephone: '☎️', bell: '🔔', clock: '🕐',
  lock: '🔒', unlock: '🔓', key: '🔑',
  hammer: '🔨', wrench: '🔧', screwdriver: '🪛',
  box: '📦', package: '📦', gift: '🎁',
  envelope: '✉️', mail: '📧', inbox: '📥',
  book: '📖', books: '📚', notebook: '📓',
  pencil: '✏️', pen: '🖊️', paintbrush: '🖌️',
  crayon: '🖍️', memo: '📝', briefcase: '💼',
  file_folder: '📁', open_file_folder: '📂',
  calendar: '📅', date: '📆', trash: '🗑️',
  locked: '🔒', unlocked: '🔓', pencil2: '✏️',
  mag: '🔍', mag_right: '🔎', globe_with_meridians: '🌐',
  bulb: '💡', anchor: '⚓', helicopter: '🚁',
  airplane: '✈️', rocket: '🚀', flying_saucer: '🛸',
  bike: '🚲', bus: '🚌', car: '🚗', taxi: '🚕',
  train: '🚆', ship: '🚢', sailboat: '⛵',
  // food
  pizza: '🍕', burger: '🍔', fries: '🍟',
  hotdog: '🌭', sandwich: '🥪', taco: '🌮',
  burrito: '🌯', salad: '🥗', popcorn: '🍿',
  bread: '🍞', croissant: '🥐', baguette: '🥖',
  pancake: '🥞', cheese: '🧀', egg: '🥚',
  bacon: '🥓', lettuce: '🥬', tomato: '🍅',
  avocado: '🥑', cucumber: '🥒', carrot: '🥕',
  corn: '🌽', chili: '🌶️', broccoli: '🥦',
  mushroom: '🍄', peanut: '🥜', honey_pot: '🍯',
  butter: '🧈', cake: '🍰', birthday: '🎂',
  cupcake: '🧁', pie: '🥧', chocolate_bar: '🍫',
  candy: '🍬', lollipop: '🍭', cookie: '🍪',
  milk: '🥛', baby_bottle: '🍼', coffee: '☕',
  tea: '🍵', sake: '🍶', beer: '🍺', beers: '🍻',
  champagne: '🍾', wine_glass: '🍷', cocktail: '🍸',
  tropical: '🍹', drink: '🍹', tumbler_glass: '🥃',
  chopsticks: '🥢', fork_and_knife: '🍴', plate_with_cutlery: '🍽️',
  sushi: '🍣', bento: '🍱', ramen: '🍜',
  curry: '🍛', rice: '🍚', rice_ball: '🍙',
  rice_cracker: '🍘', fish_cake: '🍥', dango: '🍡',
  // nature
  snowflake: '❄️', cloud: '☁️', rainbow: '🌈',
  sunny: '☀️', partly_sunny: '⛅', thunder_cloud_rain: '🌩️',
  moon: '🌙', new_moon: '🌑', full_moon: '🌕',
  galaxy: '🌌', volcano: '🌋', milky_way: '🌌',
  bridge_at_night: '🌉', hotsprings: '♨️',
  tent: '⛺', camping: '🏕️', beach_umbrella: '🏖️',
  motorway: '🛣️', railway_track: '🛤️',
  sunrise: '🌅', sunrise_over_mountains: '🌄',
   star2: '🌟', shells: '🐚',
  bamboo: '🎍', leaves: '🍃', herb: '🌿',
  four_leaf_clover: '🍀', seedling: '🌱',
  palm_tree: '🌴', cactus: '🌵', evergreen_tree: '🌲',
  deciduous_tree: '🌳', tree: '🌳', maple_leaf: '🍁',
  fallen_leaf: '🍂', flower: '🌸', tulip: '🌷',
  rose: '🌹', sunflower: '🌻', hibiscus: '🌺',
  cherry_blossom: '🌸', bouqet: '💐', chrysanthemum: '🌼',
  // animals
  cat: '🐱', dog: '🐶', mouse: '🐭', hamster: '🐹',
  rabbit: '🐰', fox: '🦊', bear: '🐻',
  panda: '🐼', koala: '🐨', tiger: '🐯',
  lion: '🦁', cow: '🐮', pig: '🐷',
  frog: '🐸', monkey_face: '🐒', chicken: '🐔',
  penguin: '🐧', bird: '🐦', baby_chick: '🐤',
  hatching_chick: '🐣', hatched: '🐥', duck: '🦆',
  eagle: '🦅', owl: '🦉', bat: '🦇', wolf: '🐺',
  boar: '🐗', horse: '🐴', unicorn: '🦄',
  hedgehog: '🦔', sheep: '🐑', goat: '🐐',
  llama: '🦙', giraffe: '🦒', zebra: '🦓',
  elephant: '🐘', rhino: '🦏', giraffe_face: '🦒',
  moose: '🫎', donkey: '🫏', hippo: '🦛',
  monkey: '🐵', gorilla: '🦍', orangutan: '🦧',
  snake: '🐍', lizard: '🦎', turtle: '🐢',
  tortoise: '🐢', bug: '🐛', butterfly: '🦋',
  snail: '🐌', beetle: '🪲', ant: '🐜',
  bee: '🐝', lady_beetle: '🐞', crickets: '🦗',
  mosquito: '🦟', microbe: '🦠', lobster: '🦞',
  mussel: '🦪', shrimp: '🦐', squid: '🦑',
  octopus: '🐙', shell: '🐚', coral: '🪸',
  // sports
  soccer: '⚽', basketball: '🏀', football: '🏈',
  baseball: '⚾', tennis: '🎾', volleyball: '🏐',
  rugby_football: '🏉', ping_pong: '🏓', badminton: '🏸',
  golf: '⛳', hockey: '🏒', field_hockey: '🏑',
  lacrosse: '🥍', cricket: '🏏', boxing: '🥊',
  martial_arts: '🥋', ice_hockey: '🏒', skiing: '⛷️',
  snowboarder: '🏂', weight_lifting: '🏋️',
  bicyclist: '🚴', mountain_bicyclist: '🚵',
  horse_racing: '🏇', surf: '🏄', swimming: '🏊',
  water_polo: '🤽', rowing: '🚣', dart: '🎯',
  bowling: '🎳', golf_course: '⛳', sled: '🛷',
  curling_stone: '🥌',
};

function blocks(text: string, me?: string, emoji?: EmojiLookup, messageId?: number): ReactNode[] {
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
      const re = unordered ? /^\s*[-*]\s+/ : /^\s*\d+\.\s+/;
      // Проверяем, является ли строка чеклистом: - [ ] задача / - [x] задача
      const taskMatch = /^\s*[-*]\s+\[[ xX]\]\s+(.*)$/.exec(line);
      if (taskMatch !== null && messageId !== undefined) {
        // Группа чеклистов: собираем подряд идущие задачи
        const tasks: { checked: boolean; content: string }[] = [];
        while (i < lines.length && /^\s*[-*]\s+\[[ xX]\]\s+/.test(lines[i])) {
          const m = /^\s*[-*]\s+\[([ xX])\]\s+(.*)$/.exec(lines[i]);
          if (m) tasks.push({ checked: m[1] !== ' ', content: m[2] });
          i++;
        }
        const items = tasks.map((t, n) => (
          <TaskItem key={n} messageId={messageId} idx={n} checked={t.checked} text={t.content} />
        ));
        out.push(<ul key={key++} className="list-none pl-1 space-y-0.5">{items}</ul>);
        continue;
      }
      const items: string[] = [];
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

function TaskItem({ messageId, idx, checked: initialChecked, text }: { messageId: number; idx: number; checked: boolean; text: string }) {
  const toggle = useTodos((s) => s.toggle);
  const done = useTodos((s) => s.done[`${messageId}-${idx}`] ?? initialChecked);
  const onClick = () => toggle(messageId, idx);
  return (
    <li className="flex items-start gap-2 group/task">
      <button
        type="button"
        onClick={onClick}
        aria-checked={done}
        role="checkbox"
        className={`mt-0.5 h-4 w-4 shrink-0 rounded border flex items-center justify-center transition-colors ${
          done ? 'bg-indigo-500 border-indigo-500' : 'border-zinc-500 hover:border-zinc-300'
        }`}
      >
        {done && (
          <svg viewBox="0 0 12 12" fill="none" className="h-3 w-3 stroke-white stroke-[2.5]">
            <path d="M2 6l3 3 5-5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </button>
      <span className={`text-sm leading-relaxed ${done ? 'text-zinc-500 line-through' : 'text-zinc-200'}`}>
        {inline(text)}
      </span>
    </li>
  );
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
      // Стоковые эмодзи: неизвестный кастомный шорткод → Unicode символ
      if (custom) {
        const stock = STOCK_EMOJI[custom[1]];
        if (stock) {
          flush();
          nodes.push(<span key={key++}>{stock}</span>);
          i += custom[0].length;
          continue;
        }
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
