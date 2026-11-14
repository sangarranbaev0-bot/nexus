import { useMemo, useRef, useState } from 'react';

/** Частые эмодзи с русскими названиями и английскими ключами для поиска. */
const EMOJI: { e: string; name: string; keys: string }[] = [
  { e: '😀', name: 'улыбка', keys: 'smile grin happy' },
  { e: '😃', name: 'радость', keys: 'smiley happy' },
  { e: '😄', name: 'смех', keys: 'smile laugh' },
  { e: '😁', name: 'ухмылка', keys: 'grin' },
  { e: '😆', name: 'хохот', keys: 'laugh' },
  { e: '😅', name: 'пот', keys: 'sweat smile' },
  { e: '😂', name: 'слёзы смеха', keys: 'joy laugh tears' },
  { e: '🤣', name: 'катаюсь', keys: 'rofl laugh' },
  { e: '😊', name: 'мило', keys: 'blush smile' },
  { e: '😇', name: 'ангел', keys: 'angel innocent' },
  { e: '🙂', name: 'улыбка', keys: 'slight smile' },
  { e: '🙃', name: 'перевёрнутая', keys: 'upside down' },
  { e: '😉', name: 'подмигивание', keys: 'wink' },
  { e: '😌', name: 'спокойствие', keys: 'relieved' },
  { e: '😍', name: 'влюблён', keys: 'heart eyes love' },
  { e: '🥰', name: 'обожаю', keys: 'love hearts' },
  { e: '😘', name: 'поцелуй', keys: 'kiss' },
  { e: '😗', name: 'чмок', keys: 'kiss' },
  { e: '😙', name: 'поцелуй', keys: 'kiss' },
  { e: '😚', name: 'поцелуй', keys: 'kiss' },
  { e: '😋', name: 'вкусно', keys: 'yum tasty' },
  { e: '😛', name: 'язык', keys: 'tongue' },
  { e: '😜', name: 'подмигиваю', keys: 'wink tongue' },
  { e: '🤪', name: 'безумие', keys: 'zany crazy' },
  { e: '😝', name: 'язык', keys: 'tongue' },
  { e: '🤑', name: 'деньги', keys: 'money' },
  { e: '🤗', name: 'обнимаю', keys: 'hug' },
  { e: '🤭', name: 'ой', keys: 'hand mouth giggle' },
  { e: '🤫', name: 'тихо', keys: 'shush quiet' },
  { e: '🤔', name: 'думаю', keys: 'think' },
  { e: '🤐', name: 'молчу', keys: 'zipper mouth' },
  { e: '🤨', name: 'бровь', keys: 'raised eyebrow' },
  { e: '😐', name: 'нейтрально', keys: 'neutral' },
  { e: '😑', name: 'без эмоций', keys: 'expressionless' },
  { e: '😶', name: 'без слов', keys: 'no mouth' },
  { e: '😏', name: 'ухмылка', keys: 'smirk' },
  { e: '😒', name: 'недоволен', keys: 'unamused' },
  { e: '🙄', name: 'закатываю глаза', keys: 'eye roll' },
  { e: '😬', name: 'неловко', keys: 'grimace' },
  { e: '🤥', name: 'вру', keys: 'lying' },
  { e: '😴', name: 'сплю', keys: 'sleep' },
  { e: '🤤', name: 'слюни', keys: 'drool' },
  { e: '😪', name: 'сонный', keys: 'sleepy' },
  { e: '😵', name: 'голова кругом', keys: 'dizzy' },
  { e: '🤯', name: 'взрыв мозга', keys: 'exploding head' },
  { e: '🤠', name: 'ковбой', keys: 'cowboy' },
  { e: '🥳', name: 'праздник', keys: 'party' },
  { e: '😎', name: 'крутой', keys: 'cool sunglasses' },
  { e: '🤓', name: 'ботан', keys: 'nerd' },
  { e: '🧐', name: 'монокль', keys: 'monocle' },
  { e: '😕', name: 'смущён', keys: 'confused' },
  { e: '😟', name: 'беспокоюсь', keys: 'worried' },
  { e: '🙁', name: 'грусть', keys: 'sad frown' },
  { e: '😮', name: 'удивление', keys: 'open mouth wow' },
  { e: '😯', name: 'тихое удивление', keys: 'hushed' },
  { e: '😲', name: 'шок', keys: 'astonished' },
  { e: '😳', name: 'румянец', keys: 'flushed' },
  { e: '🥺', name: 'умоляю', keys: 'pleading puppy' },
  { e: '😢', name: 'слеза', keys: 'cry sad' },
  { e: '😭', name: 'плачу', keys: 'sob cry' },
  { e: '😱', name: 'ужас', keys: 'scream fear' },
  { e: '😖', name: 'мучение', keys: 'confounded' },
  { e: '😣', name: 'терплю', keys: 'persevere' },
  { e: '😞', name: 'разочарован', keys: 'disappointed' },
  { e: '😓', name: 'холодный пот', keys: 'sweat' },
  { e: '😩', name: 'устал', keys: 'weary' },
  { e: '😫', name: 'измучен', keys: 'tired' },
  { e: '🥱', name: 'зеваю', keys: 'yawn' },
  { e: '😤', name: 'пыхчу', keys: 'triumph huff' },
  { e: '😡', name: 'злость', keys: 'rage angry' },
  { e: '😠', name: 'сердит', keys: 'angry' },
  { e: '🤬', name: 'ругаюсь', keys: 'cursing' },
  { e: '😈', name: 'чертёнок', keys: 'devil' },
  { e: '👿', name: 'злой черт', keys: 'imp devil' },
  { e: '💀', name: 'череп', keys: 'skull dead' },
  { e: '☠️', name: 'опасность', keys: 'skull crossbones' },
  { e: '💩', name: 'какашка', keys: 'poop' },
  { e: '🤡', name: 'клоун', keys: 'clown' },
  { e: '👻', name: 'призрак', keys: 'ghost' },
  { e: '👽', name: 'пришелец', keys: 'alien' },
  { e: '🤖', name: 'робот', keys: 'robot' },
  { e: '😺', name: 'кот', keys: 'cat smile' },
  { e: '😸', name: 'кот смеётся', keys: 'cat grin' },
  { e: '😹', name: 'кот плачет', keys: 'cat joy tears' },
  { e: '😻', name: 'кот влюблён', keys: 'cat heart' },
  { e: '🙈', name: 'обезьяна', keys: 'see no evil monkey' },
  { e: '🙉', name: 'обезьяна', keys: 'hear no evil monkey' },
  { e: '🙊', name: 'обезьяна', keys: 'speak no evil monkey' },
  { e: '💀', name: 'смех до слёз', keys: 'dead laugh' },
  { e: '❤️', name: 'сердце', keys: 'heart love red' },
  { e: '🧡', name: 'оранжевое сердце', keys: 'orange heart' },
  { e: '💛', name: 'жёлтое сердце', keys: 'yellow heart' },
  { e: '💚', name: 'зелёное сердце', keys: 'green heart' },
  { e: '💙', name: 'синее сердце', keys: 'blue heart' },
  { e: '💜', name: 'фиолетовое сердце', keys: 'purple heart' },
  { e: '🖤', name: 'чёрное сердце', keys: 'black heart' },
  { e: '🤍', name: 'белое сердце', keys: 'white heart' },
  { e: '💔', name: 'разбитое сердце', keys: 'broken heart' },
  { e: '💕', name: 'сердечки', keys: 'two hearts' },
  { e: '💞', name: 'сердца', keys: 'revolving hearts' },
  { e: '💓', name: 'бьётся', keys: 'beating heart' },
  { e: '💗', name: 'растёт', keys: 'growing heart' },
  { e: '💖', name: 'блеск', keys: 'sparkling heart' },
  { e: '💘', name: 'стрела', keys: 'cupid arrow' },
  { e: '💝', name: 'подарок', keys: 'gift heart' },
  { e: '👍', name: 'палец вверх', keys: 'thumbs up like ok' },
  { e: '👎', name: 'палец вниз', keys: 'thumbs down dislike' },
  { e: '👌', name: 'ок', keys: 'ok hand' },
  { e: '🤌', name: 'щипок', keys: 'pinched fingers' },
  { e: '✌️', name: 'виктория', keys: 'victory peace' },
  { e: '🤞', name: 'удача', keys: 'crossed fingers luck' },
  { e: '🤟', name: 'люблю', keys: 'love you gesture' },
  { e: '🤘', name: 'рок', keys: 'rock horns' },
  { e: '🤙', name: 'позвони', keys: 'call me' },
  { e: '👈', name: 'налево', keys: 'point left' },
  { e: '👉', name: 'направо', keys: 'point right' },
  { e: '👆', name: 'вверх', keys: 'point up' },
  { e: '👇', name: 'вниз', keys: 'point down' },
  { e: '☝️', name: 'указательный', keys: 'index up' },
  { e: '✋', name: 'рука', keys: 'raised hand stop' },
  { e: '🤚', name: 'ладонь', keys: 'back hand' },
  { e: '🖐️', name: 'пять', keys: 'hand fingers' },
  { e: '🖖', name: 'вулкан', keys: 'spock' },
  { e: '👋', name: 'привет', keys: 'wave hello bye' },
  { e: '🤝', name: 'рукопожатие', keys: 'handshake' },
  { e: '🙏', name: 'спасибо', keys: 'pray thanks please' },
  { e: '✍️', name: 'пишу', keys: 'writing' },
  { e: '💪', name: 'сила', keys: 'muscle strong' },
  { e: '👏', name: 'аплодисменты', keys: 'clap' },
  { e: '🙌', name: 'ура', keys: 'raise hands hooray' },
  { e: '👏', name: 'хлопаю', keys: 'clap' },
  { e: '🙋', name: 'я', keys: 'raising hand' },
  { e: '🤷', name: 'пожимаю плечами', keys: 'shrug' },
  { e: '🤦', name: 'фейспалм', keys: 'facepalm' },
  { e: '🧠', name: 'мозг', keys: 'brain' },
  { e: '👀', name: 'глаза', keys: 'eyes look' },
  { e: '👁️', name: 'глаз', keys: 'eye' },
  { e: '👅', name: 'язык', keys: 'tongue' },
  { e: '👄', name: 'губы', keys: 'lips' },
  { e: '🔥', name: 'огонь', keys: 'fire hot' },
  { e: '💯', name: 'сто', keys: 'hundred' },
  { e: '✨', name: 'искры', keys: 'sparkles' },
  { e: '⭐', name: 'звезда', keys: 'star' },
  { e: '🌟', name: 'сияние', keys: 'glowing star' },
  { e: '⚡', name: 'молния', keys: 'zap lightning' },
  { e: '💥', name: 'взрыв', keys: 'boom explosion' },
  { e: '💫', name: 'головокружение', keys: 'dizzy star' },
  { e: '🎉', name: 'тада', keys: 'party tada' },
  { e: '🎊', name: 'конфетти', keys: 'confetti' },
  { e: '🎁', name: 'подарок', keys: 'gift present' },
  { e: '🏆', name: 'кубок', keys: 'trophy win' },
  { e: '🥇', name: 'первое место', keys: 'gold medal' },
  { e: '🎯', name: 'в цель', keys: 'target bullseye' },
  { e: '✅', name: 'готово', keys: 'check done' },
  { e: '☑️', name: 'галочка', keys: 'checkbox check' },
  { e: '❌', name: 'нет', keys: 'cross no' },
  { e: '⚠️', name: 'внимание', keys: 'warning' },
  { e: '❓', name: 'вопрос', keys: 'question' },
  { e: '❗', name: 'восклицание', keys: 'exclamation' },
  { e: '💤', name: 'сон', keys: 'zzz sleep' },
  { e: '💬', name: 'сообщение', keys: 'speech bubble' },
  { e: '📌', name: 'закреп', keys: 'pin' },
  { e: '📎', name: 'скрепка', keys: 'paperclip attach' },
  { e: '🔗', name: 'ссылка', keys: 'link' },
  { e: '📝', name: 'заметка', keys: 'memo note' },
  { e: '📅', name: 'календарь', keys: 'calendar' },
  { e: '⏰', name: 'будильник', keys: 'alarm clock' },
  { e: '⏳', name: 'время', keys: 'hourglass' },
  { e: '🎮', name: 'игра', keys: 'game controller' },
  { e: '🎵', name: 'музыка', keys: 'music note' },
  { e: '🎧', name: 'наушники', keys: 'headphones' },
  { e: '☕', name: 'кофе', keys: 'coffee' },
  { e: '🍕', name: 'пицца', keys: 'pizza' },
  { e: '🍺', name: 'пиво', keys: 'beer' },
  { e: '🍻', name: 'чокаемся', keys: 'beers cheers' },
  { e: '🌙', name: 'луна', keys: 'moon night' },
  { e: '☀️', name: 'солнце', keys: 'sun' },
  { e: '🌈', name: 'радуга', keys: 'rainbow' },
  { e: '❄️', name: 'снег', keys: 'snowflake' },
  { e: '💧', name: 'капля', keys: 'droplet water' },
  { e: '🌊', name: 'волна', keys: 'wave water' },
  { e: '🚀', name: 'ракета', keys: 'rocket' },
  { e: '💻', name: 'ноутбук', keys: 'laptop computer' },
  { e: '⚙️', name: 'настройки', keys: 'gear settings' },
  { e: '🛠️', name: 'инструменты', keys: 'tools' },
  { e: '🐛', name: 'баг', keys: 'bug' },
  { e: '🧪', name: 'тест', keys: 'test tube' },
  { e: '📦', name: 'коробка', keys: 'package box' },
  { e: '🗑️', name: 'удалить', keys: 'trash delete' },
  { e: '🔒', name: 'замок', keys: 'lock' },
  { e: '🔓', name: 'открыто', keys: 'unlock' },
  { e: '🔑', name: 'ключ', keys: 'key' },
  { e: '🛡️', name: 'щит', keys: 'shield' },
  { e: '⚔️', name: 'мечи', keys: 'swords' },
  { e: '🏳️', name: 'флаг', keys: 'white flag' },
  { e: '🏴', name: 'чёрный флаг', keys: 'black flag' },
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
  const [recent, setRecent] = useState<string[]>(loadRecent);
  const input = useRef<HTMLInputElement>(null);

  const list = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return recent.length > 0 ? EMOJI.filter((m) => recent.includes(m.e)) : EMOJI;
    return EMOJI.filter((m) => m.name.includes(q) || m.keys.includes(q) || m.e === q);
  }, [query, recent]);

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
      {!query && recent.length > 0 && (
        <p className="mb-1 px-1 text-xs text-zinc-500">Недавние</p>
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