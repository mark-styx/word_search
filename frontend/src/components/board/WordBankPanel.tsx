import { scoreWord } from "@/lib/scoring";
import type { PlacedWord, PlayerState, Difficulty } from "@/types";

type WordBankPanelProps = {
  words: PlacedWord[];
  foundWords: Set<string>;
  myFoundWords: Set<string>;
  foundWordOwners: Map<string, string>;
  playersById: Map<string, PlayerState>;
  currentUserId: string | null;
  difficulty: Difficulty;
  resolvePlayerColor: (playerId: string) => string;
};

export default function WordBankPanel({
  words,
  foundWords,
  myFoundWords,
  foundWordOwners,
  playersById,
  currentUserId,
  difficulty,
  resolvePlayerColor,
}: WordBankPanelProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between gap-2 px-0.5">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.16em] text-text-secondary">Word Bank</h3>
        <span className="text-[11px] font-medium text-text-secondary">{words.length} words</span>
      </div>
      <div className="grid max-h-[12.5rem] grid-cols-3 gap-1 overflow-y-auto pr-0.5 sm:max-h-[14rem] lg:grid-cols-2">
        {words.map((placedWord) => {
          const found = foundWords.has(placedWord.word);
          const foundByMe = myFoundWords.has(placedWord.word);
          const ownerId = foundWordOwners.get(placedWord.word);
          const wordScore = scoreWord(placedWord.word, difficulty);
          const ownerColor = ownerId ? resolvePlayerColor(ownerId) : null;
          const ownerName = ownerId ? playersById.get(ownerId)?.username ?? "Player" : null;

          return (
            <span
              key={`${placedWord.word}:${placedWord.start.join(",")}:${placedWord.direction}`}
              className={`flex min-h-0 items-center justify-between gap-1 rounded-md px-2 py-1 text-[11px] leading-tight ${
                found ? "line-through" : "bg-bg-secondary/80 text-text-secondary"
              }`}
              title={found && ownerName ? `Found by ${ownerName}` : undefined}
              style={found && ownerColor
                ? {
                    background: `${ownerColor}${foundByMe ? "2e" : "1f"}`,
                    color: ownerColor,
                  }
                : undefined}
            >
              <span className="min-w-0 truncate font-medium">{placedWord.word}</span>
              <span className="shrink-0 text-[10px] font-semibold uppercase tracking-[0.08em] opacity-70">
                +{wordScore}
              </span>
            </span>
          );
        })}
      </div>
    </div>
  );
}
