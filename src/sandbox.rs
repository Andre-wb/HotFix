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

        let (filename, dockerfile_content, compile_cmd, run_cmd) = match language.to_lowercase().as_str() {
            "rust" => (
                "main.rs",
                r#"FROM rust:1.75-slim
WORKDIR /workspace
RUN apt-get update && apt-get install -y gcc
"#,
                Some("rustc main.rs"),
                "./main",
            ),
            "python" | "py" => (
                "main.py",
                r#"FROM python:3.11-slim
WORKDIR /workspace
"#,
                None,
                "python main.py",
            ),
            "javascript" | "js" => (
                "main.js",
                r#"FROM node:20-slim
WORKDIR /workspace
"#,
                None,
                "node main.js",
            ),
            _ => return Err(format!("Unsupported language: {}", language)),
        };

        // Write code and input files
        let code_path = temp_dir.join(filename);
        let input_path = temp_dir.join("input.txt");
        std::fs::write(&code_path, code).map_err(|e| e.to_string())?;
        std::fs::write(&input_path, input).map_err(|e| e.to_string())?;

        // Create Dockerfile
        let dockerfile_path = temp_dir.join("Dockerfile");
        std::fs::write(&dockerfile_path, dockerfile_content).map_err(|e| e.to_string())?;

        // Build docker image
        let image_name = format!("hotfix_sandbox_{}", run_id);
        let build_status = Command::new("docker")
            .args(["build", "-t", &image_name, temp_dir.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| format!("Failed to build docker image: {}", e))?;

        if !build_status.status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(format!(
                "Docker build failed: {}",
                String::from_utf8_lossy(&build_status.stderr)
            ));
        }

        // Prepare execute command
        let execute_cmd = match compile_cmd {
            Some(cc) => format!("{} && {} < input.txt", cc, run_cmd),
            None => format!("{} < input.txt", run_cmd),
        };

        // Run the container
        let mut cmd = Command::new("docker");
        cmd.args([
            "run", "--rm",
            "--network", "none",
            "--memory", "256m",
            "--cpus", "1.0",
            "--pids-limit", "50",
            "-v", &format!("{}:/workspace", temp_dir.display()),
            "-w", "/workspace",
            &image_name,
            "sh", "-c", &execute_cmd,
        ]);

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = match timeout(Duration::from_secs(time_limit_secs), cmd.output()).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                let _ = Command::new("docker").args(["rmi", &image_name]).output().await;
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("Execution failed: {}", e));
            }
            Err(_) => {
                let _ = Command::new("docker").args(["rmi", &image_name]).output().await;
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err("Time limit exceeded".to_string());
            }
        };

        // Cleanup
        let _ = Command::new("docker").args(["rmi", &image_name]).output().await;
        let _ = std::fs::remove_dir_all(&temp_dir);

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Runtime error: {}", stderr.chars().take(500).collect::<String>()))
        }
    }
}