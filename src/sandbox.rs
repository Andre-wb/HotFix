/// SandBox for problems testing and checking

use std::time::Duration;
use tokio::{process::Command, time::timeout};
use uuid::Uuid;

pub struct SandBox;

impl SandBox {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(
        &self,
        language: &str,
        code: &str,
        input: &str,
        time_limit_secs: u64,
    ) -> Result<String, String> {
        let run_id = Uuid::new_v4();
        let temp_dir = std::env::temp_dir().join(format!("hotfix_{}", run_id));
        std::fs::create_dir_all(&temp_dir).map_err(|error| format!("Failed to create temp directory: {error}"))?;

        let (filename, image, compile_cmd, run_cmd) = match language.to_lowercase().as_str() {
            "rust" => ("main.rs", "rust:1.75-slim", Some("rustc main.rs"), "./main"),
            "python" | "py" => ("main.py", "python:3.11-slim", None, "python main.py"),
            "javascript" | "js" => ("main.js", "node:20-slim", None, "node main.js"),
            _ => return Err(format!("Unsupported language: {}", language)),
        };

        let code_path = temp_dir.join(filename);
        let input_path = temp_dir.join("input.txt");
        std::fs::write(&code_path, code).map_err(|error| error.to_string())?;
        std::fs::write(&input_path, input).map_err(|error| error.to_string())?;

        let execute_cmd = match compile_cmd {
            Some(cc) => format!("{cc} && {run_cmd} < input.txt"),
            None => format!("{run_cmd} < input.txt"),
        };

        let mut cmd = Command::new("docker");
        cmd.args([
            "run", "--rm",
            "--network", "none",
            "--memory", "128m",
            "--cpus", "0.5",
            "--pids-limit", "50",
            "-v", &format!("{}:/workspace:ro", temp_dir.display()),
            "-w", "/workspace",
            image,
            "sh", "-c", &execute_cmd,
        ]);

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = match timeout(Duration::from_secs(time_limit_secs), cmd.output()).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("Execution failed: {}", e));
            }
            Err(_) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err("Time limit exceeded".to_string());
            }
        };

        let _ = std::fs::remove_dir_all(&temp_dir);

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Runtime error: {}", stderr.chars().take(500).collect::<String>()))
        }
    }
}