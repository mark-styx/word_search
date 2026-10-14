-- 006_custom_word_banks.sql
CREATE TABLE IF NOT EXISTS custom_word_banks (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    prompt TEXT NOT NULL,
    label TEXT,
    words_json TEXT,
    total_words INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'generating',
    can_extend INTEGER NOT NULL DEFAULT 1,
    extension_state TEXT NOT NULL DEFAULT 'idle',
    error_message TEXT,
    last_extended_at TEXT,
    last_extension_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS custom_word_bank_word_usage (
    word_bank_id TEXT NOT NULL REFERENCES custom_word_banks(id),
    word TEXT NOT NULL,
    times_used INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    PRIMARY KEY (word_bank_id, word)
);

CREATE INDEX IF NOT EXISTS idx_custom_word_banks_status_created_at
    ON custom_word_banks(status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_custom_word_banks_user_created_at
    ON custom_word_banks(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_custom_word_bank_word_usage_word_bank
    ON custom_word_bank_word_usage(word_bank_id, times_used DESC, last_used_at DESC);
