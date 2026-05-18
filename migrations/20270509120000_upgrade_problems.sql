ALTER TABLE problems ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'problems'
          AND column_name = 'difficulty'
          AND data_type = 'character varying'
    ) THEN
ALTER TABLE problems
ALTER COLUMN difficulty TYPE difficulty_enum
            USING difficulty::difficulty_enum;
END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_problems_difficulty ON problems(difficulty);
CREATE INDEX IF NOT EXISTS idx_problems_language ON problems(language);