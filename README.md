# HotFix 
## Programming Debugging Challenge Platform

HotFix is a web-based platform where users can solve programming debugging challenges. It provides a sandboxed environment for running code, user authentication with 2FA via email, and an AI-powered problem generation system (currently under development).

## Features

- **User Authentication**: Register/Login with email verification
- **Two-Factor Authentication (2FA)**: Email-based verification codes
- **Programming Challenges**: Solve Rust debugging problems
- **Safe Execution**: Secure sandboxed execution using Docker
- **Progress Tracking**: Track solved problems and user statistics

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

### 2. Install Rust

**Linux / macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Windows:**
Download and run from [https://rustup.rs](https://rustup.rs)

### 3. Install and Setup PostgreSQL

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

#### macOS
```bash
brew install postgresql
brew services start postgresql
```

#### Windows
Download from [https://www.postgresql.org/download/windows/](https://www.postgresql.org/download/windows/)

#### Create Database and User (All Platforms)

```bash
# Linux/macOS
sudo -u postgres psql

# Windows (as Administrator)
psql -U postgres
```

Then run the following SQL commands:

```sql
CREATE DATABASE hotfix;
CREATE USER hotfix_user WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE hotfix TO hotfix_user;

-- IMPORTANT: Grant schema privileges
\c hotfix
GRANT ALL ON SCHEMA public TO hotfix_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO hotfix_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO hotfix_user;

\q
```

> **⚠️ Note:** The `GRANT ALL ON SCHEMA public` command is **required** for the application to create and access tables properly.

### 4. Install Docker

#### Linux
```bash
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Log out and back in, or run: newgrp docker
```

#### macOS
Download from [Docker Desktop](https://www.docker.com/products/docker-desktop/)

#### Windows
Download from [Docker Desktop for Windows](https://www.docker.com/products/docker-desktop/)

> **Important:** you need to ensure that docker engine is running

## Verify Docker is Running

After installation, verify that Docker is properly installed and the engine is running:

### macOS:

- Open Docker Desktop application
- Wait for the status to show "Engine running" (green indicator in menu bar)
- Verify in terminal:

```bash
docker ps
docker version
```
### Windows:

- Option 1 (Docker Desktop): 
Open Docker Desktop → wait for green indicator → "Engine running"
- Option 2 (WSL2 only):

```bash
# In WSL2 terminal
sudo service docker status
sudo service docker start
```
### Verify in terminal (Command Prompt, PowerShell, or WSL2):

```bash
docker ps
docker version
```
### Linux (native Docker Engine):

Docker runs as a system service. Check status:
- Should show "active (running)"
```bash
sudo systemctl status docker
```


### Start if not running:
```bash
sudo systemctl start docker
```

### Enable to start on boot:
```bash
sudo systemctl enable docker
```
Verify with:

```bash
docker ps
docker version
```
Test Docker with Hello World (All Platforms)

```bash
docker run hello-world
```
You should see a welcome message confirming Docker is working correctly.

## Common Docker Issues and Solutions

- Docker permission denied (Linux):

```bash
sudo usermod -aG docker $USER
# Log out and back in, OR run:
newgrp docker
```
- Docker daemon not running (Linux):

```bash
sudo systemctl start docker
sudo systemctl enable docker  # auto-start on boot
```
### Docker Desktop won't start (macOS/Windows):

Ensure virtualization is enabled in BIOS
Check system resources (minimum 4GB RAM, 64-bit CPU)
Restart Docker Desktop
Check logs: ~/.docker/desktop/log.log (macOS) or Windows Event Viewer
Port conflicts with Docker (when running sandbox):

```bash
# Check if Docker daemon is using conflicting ports
docker ps
docker system df
```
WSL2 issues (Windows):

```bash
# In PowerShell (Administrator)
wsl --update
wsl --set-default-version 2
# Restart Docker Desktop
```

### Quick Status Check
Run this command to get a comprehensive Docker status:

```bash
docker info
```
If this command succeeds and shows system information (not "Cannot connect to daemon"), your Docker engine is running correctly and ready for the HotFix sandbox execution.

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
# Linux/macOS
psql -U hotfix_user -d hotfix

# Windows
psql -U hotfix_user -d hotfix
```

```sql
-- Run in this order
\i migrations/20260508112303_initial_schema.sql
\i migrations/20270509120000_upgrade_problems.sql
\i migrations/20270509120001_create_submissions.sql
\i migrations/20270509120002_create_problems.sql

-- Ensure schema permissions
GRANT ALL ON SCHEMA public TO hotfix_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO hotfix_user;
```

Or using command line:

**Linux/macOS:**
```bash
cat migrations/*.sql | psql -U hotfix_user -d hotfix
```

**Windows (PowerShell):**
```powershell
Get-Content migrations/*.sql | psql -U hotfix_user -d hotfix
```

## Troubleshooting

### Docker permission denied (Linux)

```bash
sudo usermod -aG docker $USER
# Log out and back in
```

### Database connection failed

**Linux:**
```bash
sudo systemctl status postgresql
```

**macOS:**
```bash
brew services list
```

**Windows:**
Check Services menu or run as Administrator:
```bash
net start postgresql-x64-14
```

### Public schema permission denied

If you see errors like `permission denied for schema public`, run:

```sql
\c hotfix
GRANT ALL ON SCHEMA public TO hotfix_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO hotfix_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO hotfix_user;
```

### Port 8000 already in use

```bash
# Change port in main.rs (line 54):
# Change from "127.0.0.1:8000" to "127.0.0.1:8001"
```

### SQLx offline mode (if build fails)

```bash
# Set environment variable
export SQLX_OFFLINE=true  # Linux/macOS
set SQLX_OFFLINE=true     # Windows Command Prompt
$env:SQLX_OFFLINE=true    # Windows PowerShell

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

The binary will be at `target/release/hotfix` (or `target\release\hotfix.exe` on Windows)

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