-- 008_builtin_word_bank_usage.sql
CREATE TABLE IF NOT EXISTS builtin_word_bank_state (
    theme_key TEXT PRIMARY KEY,
    can_extend INTEGER NOT NULL DEFAULT 1,
    extension_state TEXT NOT NULL DEFAULT 'idle',
    last_extended_at TEXT,
    last_extension_error TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS builtin_word_bank_generated_words (
    theme_key TEXT NOT NULL,
    word TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (theme_key, word)
);

CREATE TABLE IF NOT EXISTS builtin_word_bank_word_usage (
    theme_key TEXT NOT NULL,
    word TEXT NOT NULL,
    times_used INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    PRIMARY KEY (theme_key, word)
);

CREATE INDEX IF NOT EXISTS idx_builtin_word_bank_generated_words_theme
    ON builtin_word_bank_generated_words(theme_key, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_builtin_word_bank_word_usage_theme
    ON builtin_word_bank_word_usage(theme_key, times_used DESC, last_used_at DESC);
