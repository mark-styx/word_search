import { Sparkles } from "lucide-react";

import { Button, Modal, Select } from "@/components/ui";
import type { Difficulty, WordBank } from "@/types";

type CreateGameModalProps = {
  open: boolean;
  createError: string;
  readyWordBanks: WordBank[];
  readyWordBankOptions: { label: string; options: { value: string; label: string }[] }[];
  selectedBank: WordBank | null;
  newDiff: Difficulty;
  creating: boolean;
  difficultyOptions: { value: string; label: string }[];
  onClose: () => void;
  onSelectBank: (key: string) => void;
  onSelectDifficulty: (difficulty: Difficulty) => void;
  onCreate: () => void;
  onOpenGenerate: () => void;
  formatError: (message: string) => string;
};

export default function CreateGameModal({
  open,
  createError,
  readyWordBanks,
  readyWordBankOptions,
  selectedBank,
  newDiff,
  creating,
  difficultyOptions,
  onClose,
  onSelectBank,
  onSelectDifficulty,
  onCreate,
  onOpenGenerate,
  formatError,
}: CreateGameModalProps) {
  return (
    <Modal open={open} onClose={onClose} title="Create Game">
      <div className="flex flex-col gap-4">
        {createError && (
          <div className="rounded-lg bg-error/10 px-3 py-2 text-sm text-error">
            {createError}
          </div>
        )}

        <Select
          label="Word Bank"
          options={[]}
          groups={readyWordBankOptions}
          value={selectedBank?.key ?? ""}
          onChange={(event) => onSelectBank(event.target.value)}
        />

        {selectedBank && (
          <div className="rounded-xl border border-border bg-bg-secondary/60 px-4 py-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-text">Consumption</span>
              <span className="text-sm text-text-secondary">
                {selectedBank.coverage_percent}% used
              </span>
            </div>
            <div className="mt-2 h-2 overflow-hidden rounded-full bg-border">
              <div
                className="h-full rounded-full bg-primary transition-all"
                style={{ width: `${Math.max(selectedBank.coverage_percent, 4)}%` }}
              />
            </div>
            <p className="mt-2 text-xs text-text-secondary">
              {selectedBank.used_words_count}/{selectedBank.total_words} unique words have appeared in puzzles.
              {" "}
              {selectedBank.total_usage_count} total placements so far.
              {selectedBank.extension_state === "extending" ? " Fresh words are being fetched now." : ""}
              {selectedBank.extension_state === "disabled" && selectedBank.can_extend === false
                ? " This theme has stopped expanding automatically."
                : ""}
            </p>

            {selectedBank.last_extension_error && selectedBank.extension_state === "failed" && (
              <p className="mt-2 text-sm text-error">{formatError(selectedBank.last_extension_error)}</p>
            )}
          </div>
        )}

        <div className="rounded-xl border border-border bg-bg-secondary/40 px-4 py-3 text-sm text-text-secondary">
          Need a new topic? Generate a family-friendly bank from any word or short phrase.
          <div className="mt-3">
            <Button variant="ghost" size="sm" onClick={onOpenGenerate}>
              <Sparkles size={16} /> Generate New Word Bank
            </Button>
          </div>
        </div>

        <Select
          label="Difficulty"
          options={difficultyOptions}
          value={newDiff}
          onChange={(event) => onSelectDifficulty(event.target.value as Difficulty)}
        />

        <div className="mt-2 flex gap-2">
          <Button onClick={onCreate} loading={creating} className="flex-1" disabled={readyWordBanks.length === 0}>
            Create
          </Button>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </Modal>
  );
}
