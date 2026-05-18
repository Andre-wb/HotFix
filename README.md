# HotFix - Programming Debugging Challenge Platform

HotFix is a web-based platform where users can solve programming debugging challenges. It provides a sandboxed environment for running code, user authentication with 2FA via email, and an AI-powered problem generation system (currently under development).

## Features

- **User Authentication**: Register/Login with email verification
- **Two-Factor Authentication (2FA)**: Email-based verification codes
- **Programming Challenges**: Solve Rust debugging problems
- **Code Execution**: Secure sandboxed execution using Docker
- **Progress Tracking**: Track solved problems and user statistics
- **AI Problem Generation**: (WIP - coming soon)

## Prerequisites

- **Rust** (latest stable)
- **PostgreSQL** (14 or higher)
- **Docker** (for code execution sandbox)
- **SMTP server** (for email verification - can be disabled in development)

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/Andre-wb/HotFix
cd hotfix
```

### 2. Install Rust (if not installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Install and Setup PostgreSQL

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

**macOS:**
```bash
brew install postgresql
brew services start postgresql
```

**Create database:**
```bash
sudo -u postgres psql
```

```sql
CREATE DATABASE hotfix;
CREATE USER hotfix_user WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE hotfix TO hotfix_user;
\q
```

### 4. Install Docker

**Ubuntu/Debian:**
```bash
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Log out and back in, or run: newgrp docker
```

**macOS:**
Download from [Docker Desktop](https://www.docker.com/products/docker-desktop/)

### 5. Configure Environment

Create a `.env` file in the project root:

```env
# Required - at least 32 characters
ENCRYPTION_KEY=your_32_character_encryption_key_here_123
USERNAME_SECRET=your_username_secret_here
SESSION_SECRET=your_session_secret_here

# Admin token for problem generation (optional)
ADMIN_TOKEN=your_admin_token_here

# Database
DATABASE_URL=postgres://hotfix_user:your_password@localhost/hotfix
DATABASE_TEST_URL=postgres://hotfix_user:your_password@localhost/hotfix_test

# Environment: "development" disables actual email sending
APP_ENVIRONMENT=development
LOG_LEVEL=debug

# SMTP (only needed if APP_ENVIRONMENT=production)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your_email@gmail.com
SMTP_PASSWORD=your_app_password
SMTP_FROM=noreply@yourdomain.com
```

> **Note**: For development, set `APP_ENVIRONMENT=development` to disable actual email sending. Verification codes will be printed to console.

### 6. Run Database Migrations

The application automatically runs migrations on startup, but you can also run them manually:

```bash
# Install sqlx-cli (if not installed)
cargo install sqlx-cli

# Run migrations
sqlx migrate run
```

Or simply run the app - migrations will run automatically.

### 7. Build and Run

```bash
cargo run
```

The server will start at `http://127.0.0.1:8000`

## Available Problems

The platform comes with 6 pre-loaded problems:

| Problem | Difficulty | Description |
|---------|------------|-------------|
| Sum of Array | Easy | Calculate sum of N numbers |
| Find Maximum | Easy | Find maximum value in array |
| Even Numbers Count | Medium | Count even numbers in array |
| Reverse Array | Medium | Reverse array order |
| Fibonacci Number | Hard | Calculate N-th Fibonacci number |
| Prime Check | Hard | Check if number is prime |

## Usage Guide

### 1. Register an Account

Visit `http://127.0.0.1:8000/register` and create an account.

### 2. Verify Email

- In development mode: Check console for verification code
- In production: Check your email inbox
- Enter the 6-digit code at `/2fa_confirm`

### 3. Solve Problems

- Browse problems at `/problems`
- Click on any problem to view description and starter code
- Write your solution in the editor
- Submit to run against test cases

### 4. Track Progress

- View your profile at `/profile` to see:
    - Total problems solved
    - Your rank
    - Topic-based statistics

## Manual Database Setup (Alternative)

If migrations don't run automatically, run these SQL files in order:

```bash
# Connect to your database
psql -U hotfix_user -d hotfix

# Run migrations (in this order)
\i migrations/20260508112303_initial_schema.sql
\i migrations/20270509120000_upgrade_problems.sql
\i migrations/20270509120001_create_submissions.sql
\i migrations/20270509120002_create_problems.sql
```

Or using psql directly:

```bash
cat migrations/*.sql | psql -U hotfix_user -d hotfix
```

## Troubleshooting

### Docker permission denied

```bash
sudo usermod -aG docker $USER
# Log out and back in
```

### Database connection failed

Check PostgreSQL is running:
```bash
sudo systemctl status postgresql  # Linux
brew services list                 # macOS
```

### Port 8000 already in use

```bash
# Change port in main.rs (line 54):
# Change from "127.0.0.1:8000" to "127.0.0.1:8001"
```

### SQLx offline mode (if build fails)

```bash
# Set environment variable
export SQLX_OFFLINE=true
cargo build
```

## Project Structure

```
hotfix/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration management
│   ├── db.rs             # Database operations
│   ├── routes.rs         # HTTP handlers
│   ├── schemas.rs        # Data structures
│   ├── auth.rs           # 2FA and verification
│   ├── email.rs          # Email sending
│   ├── ai.rs             # AI problem generation (WIP)
│   ├── sandbox.rs        # Docker-based code execution
│   ├── problem_validation.rs # Problem validation
│   └── middleware.rs     # Request middleware
├── migrations/           # SQL migration files
├── templates/            # HTML templates
├── static/               # Static assets
└── Cargo.toml           # Dependencies
```

## Development Notes

### AI Problem Generation (Currently Not Working)

The AI problem generation system is under development. It requires:
- Ollama running locally with `qwen2.5-coder:7b` model
- Proper API configuration

For now, use the pre-loaded problems.

### Running Tests

```bash
cargo test
```

### Building for Production

```bash
cargo build --release
```

The binary will be at `target/release/hotfix`

## Environment Variables Reference

| Variable | Required | Description |
|----------|----------|-------------|
| ENCRYPTION_KEY | Yes | 32+ char encryption key |
| USERNAME_SECRET | Yes | Secret for username hashing |
| SESSION_SECRET | Yes | Secret for session management |
| DATABASE_URL | Yes | PostgreSQL connection string |
| APP_ENVIRONMENT | No | "development" or "production" |
| LOG_LEVEL | No | Log level (debug, info, warn, error) |
| ADMIN_TOKEN | No | Token for admin endpoints |
| SMTP_* | Only in production | Email configuration |

## License

Apache 2.0 License

## Support

For issues or questions, please open an issue on GitHub.