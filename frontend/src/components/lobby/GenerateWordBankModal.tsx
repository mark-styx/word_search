import { Button, Input, Modal } from "@/components/ui";

type GenerateWordBankModalProps = {
  open: boolean;
  error: string;
  prompt: string;
  generating: boolean;
  onClose: () => void;
  onChangePrompt: (value: string) => void;
  onGenerate: () => void;
};

export default function GenerateWordBankModal({
  open,
  error,
  prompt,
  generating,
  onClose,
  onChangePrompt,
  onGenerate,
}: GenerateWordBankModalProps) {
  return (
    <Modal open={open} onClose={onClose} title="Generate Word Bank">
      <div className="flex flex-col gap-4">
        {error && (
          <div className="rounded-lg bg-error/10 px-3 py-2 text-sm text-error">
            {error}
          </div>
        )}

        <Input
          label="What should the bank be about?"
          placeholder="Examples: backyard camping, dinosaurs, baking cookies"
          value={prompt}
          onChange={(event) => onChangePrompt(event.target.value)}
          maxLength={120}
        />

        <p className="text-sm text-text-secondary">
          The generator will turn this into a family-friendly word bank. If the request is not family-friendly,
          it will ignore that part and make a wholesome bank instead.
        </p>

        <div className="mt-2 flex gap-2">
          <Button onClick={onGenerate} loading={generating} className="flex-1">
            Generate
          </Button>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </Modal>
  );
}
