use reqwest::Client;
use serde_json::json;
use crate::schemas::{AiService, GeneratedProblem, OllamaResponse};

impl AiService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            url: "http://localhost:11434".to_string(),
            model: "qwen2.5-coder:7b".to_string(),
        }
    }

    pub async fn generate_problem(
        &self,
        language: &str,
        difficulty: &str,
    ) -> Result<GeneratedProblem, String> {
        let prompt = format!(
            r#"Create a {difficulty} debugging challenge in {language}.

IMPORTANT: The input format is:
- First line: integer N (number of elements)
- Second line: N space-separated integers

EXAMPLE OF CORRECT RUST CODE THAT READS SPACE-SEPARATED NUMBERS:
```rust
use std::io;
use std::io::BufRead;

fn main() {{
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    // Read N
    let n: usize = lines.next().unwrap().unwrap().trim().parse().unwrap();

    // Read the line of numbers
    let nums_line = lines.next().unwrap().unwrap();
    let numbers: Vec<i32> = nums_line
        .split_whitespace()
        .map(|x| x.parse().unwrap())
        .collect();

    // Process numbers
    let sum: i32 = numbers.iter().sum();
    println!("{{}}", sum);
}}
STRICT RULES:

correct_version MUST compile and run correctly
incorrect_version MUST have bugs but still compile
Use stdin/stdout for I/O
Parse input correctly: first read N, then read the next line and split by whitespace
Use proper parsing with error handling
Return ONLY valid JSON (no markdown, no explanations):
EXAMPLE JSON OUTPUT:
{{
"name": "Sum of Array",
"topics": ["arrays", "loops"],
"description": "Calculate the sum of N numbers",
"correct_version": "use std::io;\nuse std::io::BufRead;\n\nfn main() {{\n let stdin = io::stdin();\n let mut lines = stdin.lock().lines();\n let n: usize = lines.next().unwrap().unwrap().trim().parse().unwrap();\n let nums_line = lines.next().unwrap().unwrap();\n let numbers: Vec<i32> = nums_line.split_whitespace().map(|x| x.parse().unwrap()).collect();\n let sum: i32 = numbers.iter().sum();\n println!("{{}}", sum);\n}}",
"incorrect_version": "use std::io;\n\nfn main() {{\n let mut input = String::new();\n io::stdin().read_line(&mut input).unwrap();\n let n: i32 = input.trim().parse().unwrap();\n let mut sum = 0;\n for _ in 0..n {{\n let mut num = String::new();\n io::stdin().read_line(&mut num).unwrap();\n sum += num.trim().parse::<i32>().unwrap();\n }}\n println!("{{}}", sum);\n}}",
"tests": [
{{"input": "3\n1 2 3", "expected_output": "6"}},
{{"input": "4\n10 20 30 40", "expected_output": "100"}}
],
"time_limit_seconds": 5
}}"#);

        let system_prompt = "You are a Rust programming expert. Generate ONLY valid JSON.
The code MUST correctly parse space-separated numbers from stdin.
Use BufRead to read lines and split_whitespace() for parsing.
Never use markdown in JSON values.
Ensure all code compiles with rustc.";

        let body = json!({
"model": self.model,
"system": system_prompt,
"prompt": prompt,
"stream": false,
"format": "json",
"options": {
"temperature": 0.1,
"num_predict": 2048
}
});

        let res = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama connection failed: {}", e))?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(format!("Ollama API error: {}", error_text));
        }

        let data: OllamaResponse = res.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;

        // Clean up the response
        let cleaned_response = data.response
            .replace("json", "") .replace("rust", "")
            .replace("```", "")
            .trim()
            .to_string();

        let mut problem: GeneratedProblem = serde_json::from_str(&cleaned_response)
            .map_err(|e| format!("AI returned invalid JSON: {}. Raw: {}", e, &cleaned_response[..cleaned_response.len().min(500)]))?;

        // Fix common Rust issues
        problem.correct_version = fix_rust_code(&problem.correct_version);
        problem.incorrect_version = fix_rust_code(&problem.incorrect_version);

        Ok(problem)
    }
}

fn fix_rust_code(code: &str) -> String {
    let mut fixed = code.to_string();

    // Add missing imports
    if fixed.contains("BufRead") && !fixed.contains("use std::io::BufRead;") {
        if fixed.contains("use std::io;") {
            fixed = fixed.replace("use std::io;", "use std::io;\nuse std::io::BufRead;");
        } else {
            fixed = fixed.replacen("fn main()", "use std::io;\nuse std::io::BufRead;\n\nfn main()", 1);
        }
    }

    // Fix common parsing issues
    fixed = fixed.replace(".read_line(&mut input);", ".read_line(&mut input).unwrap();");
    fixed = fixed.replace("split_whitespace()", "split_whitespace()");

    fixed
}