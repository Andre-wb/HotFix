/// AI interact module

use reqwest::Client;
use serde_json::json;
use crate::schemas::{AiService, GeneratedProblem, OllamaResponse, };



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

STRICT REQUIREMENTS:
1. incorrect_version must contain realistic bugs (logic errors, off-by-one, missing edge cases)
2. incorrect_version must NOT pass all tests
3. correct_version must pass ALL tests
4. Use stdin/stdout for input/output
5. Include exactly 3 test cases

Return ONLY valid JSON in this exact structure(for example):
{{
  "name": "Short descriptive title",
  "topics": ["arrays", "sorting"],
  "description": "Detailed explanation of what the correct version of program should do",
  "incorrect_version": "fn main() {{ ... }}",
  "correct_version": "fn main() {{ ... }}",
  "tests": [
    {{"input": "3\n1 2 3", "expected_output": "6"}},
    {{"input": "2\n5 5", "expected_output": "10"}},
    {{"input": "1\n42", "expected_output": "42"}}
  ],
  "time_limit_seconds": 300
}}"#);

        let body = json!({
            "model": self.model,
            "system": "You are an expert competitive programming coach. You generate debugging challenges. Always respond with valid JSON only. No markdown, no explanations outside JSON.",
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        let res = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama connection failed: {}. Is Ollama running?", e))?;

        let data: OllamaResponse = res.json().await.map_err(|e| e.to_string())?;

        let problem: GeneratedProblem = serde_json::from_str(&data.response)
            .map_err(|e| format!("AI returned invalid JSON: {}. Raw: {}", e, data.response))?;

        Ok(problem)
    }
}