/// Schemas for database tables

use askama::Template;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
#[allow(unused)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub rank: String,
    pub problems_solved: i32, // for tracking how many problems user solved
    pub tags: Value, // for tracking topics of problems which user solved and how many problems user solved in 1 exact topic
    pub last_login_at: Option<DateTime<Utc>>,
    pub email_verified: bool,
}

#[derive(Debug, FromRow)]
pub struct Problem {
    pub id: Uuid,
    pub name: String,
    pub topics: Vec<String>,
    pub language: String,
    pub difficulty: Difficulty,
    pub correct_version: String,
    pub incorrect_version: String,
    pub tests: Value,
    pub time_limit_seconds: i32,
    pub description: String,
    pub solved_count: i32,
}

#[derive(Template)]
#[template(path = "problem.html")]
pub struct ProblemTemplate {
    pub problem: Problem,
    pub start_time: i64,
}

#[derive(Template)]
#[template(path = "problems.html")]
pub struct ProblemsTemplate {
    pub problems: Vec<Problem>,
}

#[derive(Deserialize)]
pub struct SubmitForm {
    pub code: String,
    pub start_time: i64,
}

#[derive(Template)]
#[template(path = "results.html")]
pub struct ResultsTemplate {
    pub passed: i32,
    pub total: i32,
    pub all_passed: bool,
    pub results: Vec<TestResult>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub input: String,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "difficulty_enum", rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Easy => write!(f, "easy"),
            Difficulty::Medium => write!(f, "medium"),
            Difficulty::Hard => write!(f, "hard"),
        }
    }
}

pub struct AiService {
    pub client: Client,
    pub url: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    pub response: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
}

#[derive(Debug, Deserialize)]
pub struct GeneratedProblem {
    pub name: String,
    pub topics: Vec<String>,
    pub description: String,
    pub incorrect_version: String,
    pub correct_version: String,
    pub tests: Vec<TestCase>,
    pub time_limit_seconds: i32,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub encryption_key: &'static str,
    pub username_secret: &'static str,
    pub session_secret: &'static str,
    pub database_url: &'static str,
    pub app_environment: &'static str,
    pub log_level: &'static str,
    pub smtp_host: &'static str,
    pub smtp_port: u16,
    pub smtp_username: &'static str,
    pub smtp_password: &'static str,
    pub smtp_from: &'static str,
}

#[derive(Debug, FromRow)]
pub struct EmailVerificationCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
}

pub enum CodeVerificationResult {
    Valid,
    Invalid,
    Expired,
    TooManyAttempts,
    AlreadyUsed,
}

#[derive(Deserialize, Debug)]
pub struct TwoFaForm {
    pub confirm_code: String,
}

#[derive(Template)]
#[template(path = "2fa.html")]
pub struct TwoFaTemplate {
    pub flash_message: Option<String>,
    pub confirm_code: Option<String>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    pub flash_message: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub confirm_password: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub flash_message: Option<String>,
    pub identifier: Option<String>,
    pub password: Option<String>,
}

#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileTemplate;

#[derive(Deserialize, Debug)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginForm {
    pub identifier: String,
    pub password: String,
}