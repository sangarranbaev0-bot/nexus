import { useMemo, useRef, useState } from 'react';
import { useEmojis } from './emojis';

/** Категории-паки стандартных эмодзи. */
type Pack = 'smileys' | 'animals' | 'hearts' | 'hands' | 'symbols' | 'food' | 'travel' | 'objects';

const PACKS: { id: Pack; label: string; icon: string }[] = [
  { id: 'smileys', label: 'Смайлики', icon: '😀' },
  { id: 'animals', label: 'Животные', icon: '🐱' },
  { id: 'hearts', label: 'Сердца', icon: '❤️' },
  { id: 'hands', label: 'Жесты', icon: '👍' },
  { id: 'food', label: 'Еда', icon: '🍕' },
  { id: 'travel', label: 'Природа', icon: '🌈' },
  { id: 'objects', label: 'Предметы', icon: '🚀' },
  { id: 'symbols', label: 'Символы', icon: '✅' },
];

/** Эмодзи с русскими названиями и английскими ключами для поиска. */
const EMOJI: { e: string; name: string; keys: string; pack: Pack }[] = [
  // Смайлики
  { e: '😀', name: 'улыбка', keys: 'smile grin happy', pack: 'smileys' },
  { e: '😃', name: 'радость', keys: 'smiley happy', pack: 'smileys' },
  { e: '😄', name: 'смех', keys: 'smile laugh', pack: 'smileys' },
  { e: '😁', name: 'ухмылка', keys: 'grin', pack: 'smileys' },
  { e: '😆', name: 'хохот', keys: 'laugh', pack: 'smileys' },
  { e: '😅', name: 'пот', keys: 'sweat smile', pack: 'smileys' },
  { e: '😂', name: 'слёзы смеха', keys: 'joy laugh tears', pack: 'smileys' },
  { e: '🤣', name: 'катаюсь', keys: 'rofl laugh', pack: 'smileys' },
  { e: '😊', name: 'мило', keys: 'blush smile', pack: 'smileys' },
  { e: '😇', name: 'ангел', keys: 'angel innocent', pack: 'smileys' },
  { e: '🙂', name: 'улыбка', keys: 'slight smile', pack: 'smileys' },
  { e: '🙃', name: 'перевёрнутая', keys: 'upside down', pack: 'smileys' },
  { e: '😉', name: 'подмигивание', keys: 'wink', pack: 'smileys' },
  { e: '😌', name: 'спокойствие', keys: 'relieved', pack: 'smileys' },
  { e: '😍', name: 'влюблён', keys: 'heart eyes love', pack: 'smileys' },
  { e: '🥰', name: 'обожаю', keys: 'love hearts', pack: 'smileys' },
  { e: '😘', name: 'поцелуй', keys: 'kiss', pack: 'smileys' },
  { e: '😗', name: 'чмок', keys: 'kiss', pack: 'smileys' },
  { e: '😙', name: 'поцелуй', keys: 'kiss', pack: 'smileys' },
  { e: '😚', name: 'поцелуй', keys: 'kiss', pack: 'smileys' },
  { e: '😋', name: 'вкусно', keys: 'yum tasty', pack: 'smileys' },
  { e: '😛', name: 'язык', keys: 'tongue', pack: 'smileys' },
  { e: '😜', name: 'подмигиваю', keys: 'wink tongue', pack: 'smileys' },
  { e: '🤪', name: 'безумие', keys: 'zany crazy', pack: 'smileys' },
  { e: '😝', name: 'язык', keys: 'tongue', pack: 'smileys' },
  { e: '🤑', name: 'деньги', keys: 'money', pack: 'smileys' },
  { e: '🤗', name: 'обнимаю', keys: 'hug', pack: 'smileys' },
  { e: '🤭', name: 'ой', keys: 'hand mouth giggle', pack: 'smileys' },
  { e: '🤫', name: 'тихо', keys: 'shush quiet', pack: 'smileys' },
  { e: '🤔', name: 'думаю', keys: 'think', pack: 'smileys' },
  { e: '🤐', name: 'молчу', keys: 'zipper mouth', pack: 'smileys' },
  { e: '🤨', name: 'бровь', keys: 'raised eyebrow', pack: 'smileys' },
  { e: '😐', name: 'нейтрально', keys: 'neutral', pack: 'smileys' },
  { e: '😑', name: 'без эмоций', keys: 'expressionless', pack: 'smileys' },
  { e: '😶', name: 'без слов', keys: 'no mouth', pack: 'smileys' },
  { e: '😏', name: 'ухмылка', keys: 'smirk', pack: 'smileys' },
  { e: '😒', name: 'недоволен', keys: 'unamused', pack: 'smileys' },
  { e: '🙄', name: 'закатываю глаза', keys: 'eye roll', pack: 'smileys' },
  { e: '😬', name: 'неловко', keys: 'grimace', pack: 'smileys' },
  { e: '🤥', name: 'вру', keys: 'lying', pack: 'smileys' },
  { e: '😴', name: 'сплю', keys: 'sleep', pack: 'smileys' },
  { e: '🤤', name: 'слюни', keys: 'drool', pack: 'smileys' },
  { e: '😪', name: 'сонный', keys: 'sleepy', pack: 'smileys' },
  { e: '😵', name: 'голова кругом', keys: 'dizzy', pack: 'smileys' },
  { e: '🤯', name: 'взрыв мозга', keys: 'exploding head', pack: 'smileys' },
  { e: '🤠', name: 'ковбой', keys: 'cowboy', pack: 'smileys' },
  { e: '🥳', name: 'праздник', keys: 'party', pack: 'smileys' },
  { e: '😎', name: 'крутой', keys: 'cool sunglasses', pack: 'smileys' },
  { e: '🤓', name: 'ботан', keys: 'nerd', pack: 'smileys' },
  { e: '🧐', name: 'монокль', keys: 'monocle', pack: 'smileys' },
  { e: '😕', name: 'смущён', keys: 'confused', pack: 'smileys' },
  { e: '😟', name: 'беспокоюсь', keys: 'worried', pack: 'smileys' },
  { e: '🙁', name: 'грусть', keys: 'sad frown', pack: 'smileys' },
  { e: '😮', name: 'удивление', keys: 'open mouth wow', pack: 'smileys' },
  { e: '😯', name: 'тихое удивление', keys: 'hushed', pack: 'smileys' },
  { e: '😲', name: 'шок', keys: 'astonished', pack: 'smileys' },
  { e: '😳', name: 'румянец', keys: 'flushed', pack: 'smileys' },
  { e: '🥺', name: 'умоляю', keys: 'pleading puppy', pack: 'smileys' },
  { e: '😢', name: 'слеза', keys: 'cry sad', pack: 'smileys' },
  { e: '😭', name: 'плачу', keys: 'sob cry', pack: 'smileys' },
  { e: '😱', name: 'ужас', keys: 'scream fear', pack: 'smileys' },
  { e: '😖', name: 'мучение', keys: 'confounded', pack: 'smileys' },
  { e: '😣', name: 'терплю', keys: 'persevere', pack: 'smileys' },
  { e: '😞', name: 'разочарован', keys: 'disappointed', pack: 'smileys' },
  { e: '😓', name: 'холодный пот', keys: 'sweat', pack: 'smileys' },
  { e: '😩', name: 'устал', keys: 'weary', pack: 'smileys' },
  { e: '😫', name: 'измучен', keys: 'tired', pack: 'smileys' },
  { e: '🥱', name: 'зеваю', keys: 'yawn', pack: 'smileys' },
  { e: '😤', name: 'пыхчу', keys: 'triumph huff', pack: 'smileys' },
  { e: '😡', name: 'злость', keys: 'rage angry', pack: 'smileys' },
  { e: '😠', name: 'сердит', keys: 'angry', pack: 'smileys' },
  { e: '🤬', name: 'ругаюсь', keys: 'cursing', pack: 'smileys' },
  { e: '😈', name: 'чертёнок', keys: 'devil', pack: 'smileys' },
  { e: '👿', name: 'злой черт', keys: 'imp devil', pack: 'smileys' },
  { e: '💀', name: 'череп', keys: 'skull dead', pack: 'smileys' },
  { e: '☠️', name: 'опасность', keys: 'skull crossbones', pack: 'smileys' },
  { e: '💩', name: 'какашка', keys: 'poop', pack: 'smileys' },
  { e: '🤡', name: 'клоун', keys: 'clown', pack: 'smileys' },
  { e: '👻', name: 'призрак', keys: 'ghost', pack: 'smileys' },
  { e: '👽', name: 'пришелец', keys: 'alien', pack: 'smileys' },
  { e: '🤖', name: 'робот', keys: 'robot', pack: 'smileys' },
  // Животные
  { e: '😺', name: 'кот', keys: 'cat smile', pack: 'animals' },
  { e: '😸', name: 'кот смеётся', keys: 'cat grin', pack: 'animals' },
  { e: '😹', name: 'кот плачет', keys: 'cat joy tears', pack: 'animals' },
  { e: '😻', name: 'кот влюблён', keys: 'cat heart', pack: 'animals' },
  { e: '🙈', name: 'обезьяна', keys: 'see no evil monkey', pack: 'animals' },
  { e: '🙉', name: 'обезьяна', keys: 'hear no evil monkey', pack: 'animals' },
  { e: '🙊', name: 'обезьяна', keys: 'speak no evil monkey', pack: 'animals' },
  // Сердца
  { e: '❤️', name: 'сердце', keys: 'heart love red', pack: 'hearts' },
  { e: '🧡', name: 'оранжевое сердце', keys: 'orange heart', pack: 'hearts' },
  { e: '💛', name: 'жёлтое сердце', keys: 'yellow heart', pack: 'hearts' },
  { e: '💚', name: 'зелёное сердце', keys: 'green heart', pack: 'hearts' },
  { e: '💙', name: 'синее сердце', keys: 'blue heart', pack: 'hearts' },
  { e: '💜', name: 'фиолетовое сердце', keys: 'purple heart', pack: 'hearts' },
  { e: '🖤', name: 'чёрное сердце', keys: 'black heart', pack: 'hearts' },
  { e: '🤍', name: 'белое сердце', keys: 'white heart', pack: 'hearts' },
  { e: '💔', name: 'разбитое сердце', keys: 'broken heart', pack: 'hearts' },
  { e: '💕', name: 'сердечки', keys: 'two hearts', pack: 'hearts' },
  { e: '💞', name: 'сердца', keys: 'revolving hearts', pack: 'hearts' },
  { e: '💓', name: 'бьётся', keys: 'beating heart', pack: 'hearts' },
  { e: '💗', name: 'растёт', keys: 'growing heart', pack: 'hearts' },
  { e: '💖', name: 'блеск', keys: 'sparkling heart', pack: 'hearts' },
  { e: '💘', name: 'стрела', keys: 'cupid arrow', pack: 'hearts' },
  { e: '💝', name: 'подарок', keys: 'gift heart', pack: 'hearts' },
  // Жесты
  { e: '👍', name: 'палец вверх', keys: 'thumbs up like ok', pack: 'hands' },
  { e: '👎', name: 'палец вниз', keys: 'thumbs down dislike', pack: 'hands' },
  { e: '👌', name: 'ок', keys: 'ok hand', pack: 'hands' },
  { e: '🤌', name: 'щипок', keys: 'pinched fingers', pack: 'hands' },
  { e: '✌️', name: 'виктория', keys: 'victory peace', pack: 'hands' },
  { e: '🤞', name: 'удача', keys: 'crossed fingers luck', pack: 'hands' },
  { e: '🤟', name: 'люблю', keys: 'love you gesture', pack: 'hands' },
  { e: '🤘', name: 'рок', keys: 'rock horns', pack: 'hands' },
  { e: '🤙', name: 'позвони', keys: 'call me', pack: 'hands' },
  { e: '👈', name: 'налево', keys: 'point left', pack: 'hands' },
  { e: '👉', name: 'направо', keys: 'point right', pack: 'hands' },
  { e: '👆', name: 'вверх', keys: 'point up', pack: 'hands' },
  { e: '👇', name: 'вниз', keys: 'point down', pack: 'hands' },
  { e: '☝️', name: 'указательный', keys: 'index up', pack: 'hands' },
  { e: '✋', name: 'рука', keys: 'raised hand stop', pack: 'hands' },
  { e: '🤚', name: 'ладонь', keys: 'back hand', pack: 'hands' },
  { e: '🖐️', name: 'пять', keys: 'hand fingers', pack: 'hands' },
  { e: '🖖', name: 'вулкан', keys: 'spock', pack: 'hands' },
  { e: '👋', name: 'привет', keys: 'wave hello bye', pack: 'hands' },
  { e: '🤝', name: 'рукопожатие', keys: 'handshake', pack: 'hands' },
  { e: '🙏', name: 'спасибо', keys: 'pray thanks please', pack: 'hands' },
  { e: '✍️', name: 'пишу', keys: 'writing', pack: 'hands' },
  { e: '💪', name: 'сила', keys: 'muscle strong', pack: 'hands' },
  { e: '👏', name: 'аплодисменты', keys: 'clap', pack: 'hands' },
  { e: '🙌', name: 'ура', keys: 'raise hands hooray', pack: 'hands' },
  { e: '🙋', name: 'я', keys: 'raising hand', pack: 'hands' },
  { e: '🤷', name: 'пожимаю плечами', keys: 'shrug', pack: 'hands' },
  { e: '🤦', name: 'фейспалм', keys: 'facepalm', pack: 'hands' },
  // Еда
  { e: '☕', name: 'кофе', keys: 'coffee', pack: 'food' },
  { e: '🍕', name: 'пицца', keys: 'pizza', pack: 'food' },
  { e: '🍺', name: 'пиво', keys: 'beer', pack: 'food' },
  { e: '🍻', name: 'чокаемся', keys: 'beers cheers', pack: 'food' },
  // Природа
  { e: '🌙', name: 'луна', keys: 'moon night', pack: 'travel' },
  { e: '☀️', name: 'солнце', keys: 'sun', pack: 'travel' },
  { e: '🌈', name: 'радуга', keys: 'rainbow', pack: 'travel' },
  { e: '❄️', name: 'снег', keys: 'snowflake', pack: 'travel' },
  { e: '💧', name: 'капля', keys: 'droplet water', pack: 'travel' },
  { e: '🌊', name: 'волна', keys: 'wave water', pack: 'travel' },
  // Предметы
  { e: '🚀', name: 'ракета', keys: 'rocket', pack: 'objects' },
  { e: '💻', name: 'ноутбук', keys: 'laptop computer', pack: 'objects' },
  { e: '⚙️', name: 'настройки', keys: 'gear settings', pack: 'objects' },
  { e: '🛠️', name: 'инструменты', keys: 'tools', pack: 'objects' },
  { e: '🐛', name: 'баг', keys: 'bug', pack: 'objects' },
  { e: '🧪', name: 'тест', keys: 'test tube', pack: 'objects' },
  { e: '📦', name: 'коробка', keys: 'package box', pack: 'objects' },
  { e: '🗑️', name: 'удалить', keys: 'trash delete', pack: 'objects' },
  { e: '🔒', name: 'замок', keys: 'lock', pack: 'objects' },
  { e: '🔓', name: 'открыто', keys: 'unlock', pack: 'objects' },
  { e: '🔑', name: 'ключ', keys: 'key', pack: 'objects' },
  { e: '🛡️', name: 'щит', keys: 'shield', pack: 'objects' },
  { e: '⚔️', name: 'мечи', keys: 'swords', pack: 'objects' },
  { e: '🏳️', name: 'флаг', keys: 'white flag', pack: 'objects' },
  { e: '🏴', name: 'чёрный флаг', keys: 'black flag', pack: 'objects' },
  { e: '🎮', name: 'игра', keys: 'game controller', pack: 'objects' },
  { e: '🎵', name: 'музыка', keys: 'music note', pack: 'objects' },
  { e: '🎧', name: 'наушники', keys: 'headphones', pack: 'objects' },
  // Символы
  { e: '🔥', name: 'огонь', keys: 'fire hot', pack: 'symbols' },
  { e: '💯', name: 'сто', keys: 'hundred', pack: 'symbols' },
  { e: '✨', name: 'искры', keys: 'sparkles', pack: 'symbols' },
  { e: '⭐', name: 'звезда', keys: 'star', pack: 'symbols' },
  { e: '🌟', name: 'сияние', keys: 'glowing star', pack: 'symbols' },
  { e: '⚡', name: 'молния', keys: 'zap lightning', pack: 'symbols' },
  { e: '💥', name: 'взрыв', keys: 'boom explosion', pack: 'symbols' },
  { e: '💫', name: 'головокружение', keys: 'dizzy star', pack: 'symbols' },
  { e: '🎉', name: 'тада', keys: 'party tada', pack: 'symbols' },
  { e: '🎊', name: 'конфетти', keys: 'confetti', pack: 'symbols' },
  { e: '🎁', name: 'подарок', keys: 'gift present', pack: 'symbols' },
  { e: '🏆', name: 'кубок', keys: 'trophy win', pack: 'symbols' },
  { e: '🥇', name: 'первое место', keys: 'gold medal', pack: 'symbols' },
  { e: '🎯', name: 'в цель', keys: 'target bullseye', pack: 'symbols' },
  { e: '✅', name: 'готово', keys: 'check done', pack: 'symbols' },
  { e: '☑️', name: 'галочка', keys: 'checkbox check', pack: 'symbols' },
  { e: '❌', name: 'нет', keys: 'cross no', pack: 'symbols' },
  { e: '⚠️', name: 'внимание', keys: 'warning', pack: 'symbols' },
  { e: '❓', name: 'вопрос', keys: 'question', pack: 'symbols' },
  { e: '❗', name: 'восклицание', keys: 'exclamation', pack: 'symbols' },
  { e: '💤', name: 'сон', keys: 'zzz sleep', pack: 'symbols' },
  { e: '💬', name: 'сообщение', keys: 'speech bubble', pack: 'symbols' },
  { e: '📌', name: 'закреп', keys: 'pin', pack: 'symbols' },
  { e: '📎', name: 'скрепка', keys: 'paperclip attach', pack: 'symbols' },
  { e: '🔗', name: 'ссылка', keys: 'link', pack: 'symbols' },
  { e: '📝', name: 'заметка', keys: 'memo note', pack: 'symbols' },
  { e: '📅', name: 'календарь', keys: 'calendar', pack: 'symbols' },
  { e: '⏰', name: 'будильник', keys: 'alarm clock', pack: 'symbols' },
  { e: '⏳', name: 'время', keys: 'hourglass', pack: 'symbols' },
  // Умные дополнения (остальные)
  { e: '🧠', name: 'мозг', keys: 'brain', pack: 'smileys' },
  { e: '👀', name: 'глаза', keys: 'eyes look', pack: 'smileys' },
  { e: '👁️', name: 'глаз', keys: 'eye', pack: 'smileys' },
  { e: '👅', name: 'язык', keys: 'tongue', pack: 'smileys' },
  { e: '👄', name: 'губы', keys: 'lips', pack: 'smileys' },
];

const RECENT_KEY = 'nexus.emoji.recent';
const RECENT_MAX = 24;

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    return raw ? (JSON.parse(raw) as string[]).slice(0, RECENT_MAX) : [];
  } catch {
    return [];
  }
}

export function EmojiPicker({
  onPick,
  onClose,
  className = '',
}: {
  onPick: (emoji: string) => void;
  onClose?: () => void;
  className?: string;
}) {
  const [query, setQuery] = useState('');
  const [pack, setPack] = useState<Pack | 'recent'>('recent');
  const [recent, setRecent] = useState<string[]>(loadRecent);
  const input = useRef<HTMLInputElement>(null);
  const byServer = useEmojis((s) => s.byServer);

  const list = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q) {
      return EMOJI.filter((m) => m.name.includes(q) || m.keys.includes(q) || m.e === q);
    }
    if (pack === 'recent') {
      return recent.length > 0 ? EMOJI.filter((m) => recent.includes(m.e)) : EMOJI;
    }
    return EMOJI.filter((m) => m.pack === pack);
  }, [query, recent, pack]);

  // Свои эмодзи серверов: ищем по имени, вставляем как `:name:`.
  const custom = useMemo(() => {
    const q = query.trim().toLowerCase();
    const all = Object.values(byServer).flatMap((l) => l ?? []);
    const seen = new Set<string>();
    return all.filter((e) => {
      if (seen.has(e.name)) return false;
      if (q && !e.name.includes(q)) return false;
      seen.add(e.name);
      return true;
    });
  }, [byServer, query]);

  const pick = (emoji: string) => {
    const next = [emoji, ...recent.filter((e) => e !== emoji)].slice(0, RECENT_MAX);
    setRecent(next);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(next));
    } catch {
      // Приватный режим: недавние просто не сохранятся.
    }
    onPick(emoji);
    onClose?.();
  };

  return (
    <div className={`w-72 rounded-lg border border-zinc-700 bg-zinc-800 p-2 shadow-xl ${className}`}>
      <input
        ref={input}
        autoFocus
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') onClose?.();
          if (e.key === 'Enter' && list[0]) {
            e.preventDefault();
            pick(list[0].e);
          }
        }}
        placeholder="Поиск эмодзи"
        aria-label="Поиск эмодзи"
        className="mb-2 w-full rounded bg-zinc-900 px-2 py-1 text-sm outline-none placeholder:text-zinc-500"
      />
      {custom.length > 0 && (
        <>
          <p className="mb-1 px-1 text-xs text-zinc-500">Свои эмодзи сервера</p>
          <div className="mb-2 grid max-h-24 grid-cols-8 gap-1 overflow-y-auto">
            {custom.map((e) => (
              <button
                key={`custom-${e.id}`}
                type="button"
                onClick={() => pick(`:${e.name}:`)}
                title={`:${e.name}:`}
                className="grid h-7 w-7 place-items-center rounded hover:bg-zinc-700"
              >
                <img src={e.url} alt={`:${e.name}:`} className="h-5 w-5" />
              </button>
            ))}
          </div>
        </>
      )}
      {!query && (
        <div className="mb-2 flex flex-wrap gap-1">
          <button
            type="button"
            onClick={() => setPack('recent')}
            title="Недавние"
            aria-label="Недавние"
            className={`grid size-7 place-items-center rounded text-base hover:bg-zinc-700 ${
              pack === 'recent' ? 'bg-zinc-700' : ''
            }`}
          >
            🕘
          </button>
          {PACKS.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={() => setPack(p.id)}
              title={p.label}
              aria-label={p.label}
              className={`grid size-7 place-items-center rounded text-base hover:bg-zinc-700 ${
                pack === p.id ? 'bg-zinc-700' : ''
              }`}
            >
              {p.icon}
            </button>
          ))}
        </div>
      )}
      {list.length === 0 ? (
        <p className="px-1 py-2 text-sm text-zinc-400">Ничего не найдено.</p>
      ) : (
        <div className="grid max-h-56 grid-cols-8 gap-1 overflow-y-auto">
          {list.map((m) => (
            <button
              key={m.e + m.name}
              type="button"
              title={m.name}
              onClick={() => pick(m.e)}
              className="grid size-8 place-items-center rounded text-lg hover:bg-zinc-700"
            >
              {m.e}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
