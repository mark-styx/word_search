import { AnimatePresence, motion } from "framer-motion";
import { Send } from "lucide-react";
import { Button, Card } from "@/components/ui";

type ChatPanelProps = {
  messages: { from: string; text: string }[];
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
};

export default function ChatPanel({ messages, value, onChange, onSend }: ChatPanelProps) {
  return (
    <Card className="flex min-h-0 flex-1 flex-col gap-2">
      <h3 className="text-sm font-semibold uppercase text-text-secondary">Chat</h3>
      <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
        <AnimatePresence>
          {messages.map((message, index) => (
            <motion.div
              key={`${message.from}:${message.text}:${index}`}
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0 }}
              className="text-sm"
            >
              <span className="font-medium text-primary">{message.from}: </span>
              <span className="text-text-secondary">{message.text}</span>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
      <div className="flex gap-2">
        <input
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => event.key === "Enter" && onSend()}
          placeholder="Type a message..."
          className="h-11 flex-1 rounded-lg border border-border bg-bg-secondary px-3 text-sm text-text focus:outline-none focus:ring-2 focus:ring-primary/60"
        />
        <Button
          onClick={onSend}
          type="button"
          size="sm"
          className="px-0"
          aria-label="Send chat message"
        >
          <Send size={16} />
        </Button>
      </div>
    </Card>
  );
}
