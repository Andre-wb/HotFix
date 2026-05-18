CREATE TABLE IF NOT EXISTS users (
                                     id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rank VARCHAR(20) NOT NULL DEFAULT 'beginner',
    problems_solved INTEGER NOT NULL DEFAULT 0,
    tags JSONB NOT NULL DEFAULT '{}'::jsonb,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    last_login_at TIMESTAMPTZ
    );

CREATE TABLE IF NOT EXISTS problems (
                                        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    topics TEXT[] NOT NULL DEFAULT '{}',
    language VARCHAR(20) NOT NULL,
    difficulty VARCHAR(10) NOT NULL,
    correct_version TEXT NOT NULL,
    incorrect_version TEXT NOT NULL,
    tests JSONB NOT NULL,
    time_limit_seconds INTEGER NOT NULL,
    description TEXT NOT NULL,
    solved_count INTEGER NOT NULL DEFAULT 0
    );

DO $$ BEGIN
CREATE TYPE difficulty_enum AS ENUM ('easy', 'medium', 'hard');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS email_verification_codes (
                                                        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_last_login_at ON users(last_login_at);
CREATE INDEX IF NOT EXISTS idx_email_codes_user_id ON email_verification_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_email_codes_used ON email_verification_codes(used);