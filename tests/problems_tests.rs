/// Integration tests for all problems in the database
///
/// Run with: cargo test --test problems_tests -- --nocapture

use hotfix::{db, sandbox::SandBox};

fn unescape_code(code: &str) -> String {
    code.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

#[tokio::test]
async fn test_all_problems() {
    dotenvy::dotenv().ok();
    let _ = hotfix::config::Config::init();

    let pool = db::init_pool().await;
    let sandbox = SandBox::new();

    // Get all problems from database
    let problems = db::get_problems(&pool).await.expect("Failed to fetch problems");

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                     PROBLEM VALIDATION SUITE                         ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");
    println!("║ Total problems to test: {:30}           ║", problems.len());
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    let mut correct_passed = 0;
    let mut correct_failed = 0;
    let mut incorrect_passed = 0;
    let mut incorrect_failed = 0;
    let mut problems_with_issues = Vec::new();

    for (idx, problem) in problems.iter().enumerate() {
        println!("\n┌────────────────────────────────────────────────────────────────────┐");
        println!("│ Problem {}: {:<52} │", idx + 1, problem.name);
        println!("├────────────────────────────────────────────────────────────────────┤");
        println!("│ Difficulty: {:10} │ Language: {:10} │ Time limit: {}s │",
                 problem.difficulty.to_string().to_uppercase(),
                 problem.language.to_uppercase(),
                 problem.time_limit_seconds
        );
        println!("└────────────────────────────────────────────────────────────────────┘");

        // Parse tests
        let tests: Vec<hotfix::schemas::TestCase> = match serde_json::from_value(problem.tests.clone()) {
            Ok(t) => t,
            Err(e) => {
                println!("  ❌ Failed to parse tests: {}", e);
                problems_with_issues.push((problem.name.clone(), "Invalid test format".to_string()));
                continue;
            }
        };

        // Декодируем код из БД
        let correct_code = unescape_code(&problem.correct_version);
        let incorrect_code = unescape_code(&problem.incorrect_version);

        let total_tests = tests.len();
        println!("\n  📋 Tests: {} test case(s)", total_tests);

        // Test correct version
        println!("\n  🟢 CORRECT VERSION:");
        let correct_results = test_version(
            &sandbox,
            &problem.language,
            &correct_code,
            &tests,
            problem.time_limit_seconds as u64,
        ).await;

        let correct_score = format!("{}/{}", correct_results.passed, total_tests);
        if correct_results.passed == total_tests {
            println!("    ✅ PASSED: {} tests passed", correct_score);
            correct_passed += 1;
        } else {
            println!("    ❌ FAILED: {} tests passed (expected {}/{})", correct_score, total_tests, total_tests);
            correct_failed += 1;
            problems_with_issues.push((problem.name.clone(), format!("Correct version: {}/{}", correct_results.passed, total_tests)));
        }

        // Print details for failed correct version tests
        for result in &correct_results.details {
            if !result.passed {
                println!("\n    ┌─ Test case failed ───────────────────────────────────────");
                println!("    │ Input:    {:?}", result.input);
                println!("    │ Expected: {:?}", result.expected);
                println!("    │ Got:      {:?}", result.actual);
                println!("    └───────────────────────────────────────────────────────────");
            }
        }

        // Test incorrect version
        println!("\n  🔴 INCORRECT VERSION:");
        let incorrect_results = test_version(
            &sandbox,
            &problem.language,
            &incorrect_code,
            &tests,
            problem.time_limit_seconds as u64,
        ).await;

        let incorrect_score = format!("{}/{}", incorrect_results.passed, total_tests);

        // For incorrect version, we expect 0 tests passed
        if incorrect_results.passed == 0 {
            println!("    ✅ GOOD: {} tests failed (all tests fail as expected)", incorrect_score);
            incorrect_passed += 1;
        } else {
            println!("    ❌ BAD: {} tests passed (expected 0/{})", incorrect_score, total_tests);
            incorrect_failed += 1;
            problems_with_issues.push((problem.name.clone(), format!("Incorrect version passed {}/{} tests", incorrect_results.passed, total_tests)));

            // Print which tests passed unexpectedly
            for result in &incorrect_results.details {
                if result.passed {
                    println!("\n    ⚠️  Test case passed unexpectedly:");
                    println!("       Input:    {:?}", result.input);
                    println!("       Expected: {:?}", result.expected);
                    println!("       Got:      {:?}", result.actual);
                }
            }
        }

        println!("\n  ────────────────────────────────────────────────────────────────────");
    }

    // Summary
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                           SUMMARY                                     ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");
    println!("║                                                                       ║");
    println!("║  CORRECT VERSION (should pass ALL tests):                            ║");
    println!("║    ✅ Passed: {:2}/{:2} problems                                        ║", correct_passed, problems.len());
    println!("║    ❌ Failed: {:2}/{:2} problems                                        ║", correct_failed, problems.len());
    println!("║                                                                       ║");
    println!("║  INCORRECT VERSION (should pass 0 tests):                            ║");
    println!("║    ✅ Good (0 passed): {:2}/{:2} problems                               ║", incorrect_passed, problems.len());
    println!("║    ❌ Bad (passed some): {:2}/{:2} problems                             ║", incorrect_failed, problems.len());
    println!("║                                                                       ║");

    if !problems_with_issues.is_empty() {
        println!("╠══════════════════════════════════════════════════════════════════════╣");
        println!("║                         PROBLEMS WITH ISSUES                         ║");
        println!("╠══════════════════════════════════════════════════════════════════════╣");
        for (name, issue) in &problems_with_issues {
            println!("║  • {:<60} ║", truncate(name, 60));
            println!("║    └─ {:<66} ║", truncate(issue, 66));
        }
    }

    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    // Assert all problems pass validation
    assert_eq!(correct_failed, 0, "Some problems failed correct version validation");
    assert_eq!(incorrect_failed, 0, "Some problems have incorrect version that passes tests");
}

#[derive(Debug)]
struct TestVersionResult {
    passed: usize,
    details: Vec<TestCaseDetail>,
}

#[derive(Debug)]
struct TestCaseDetail {
    input: String,
    expected: String,
    actual: String,
    passed: bool,
}

async fn test_version(
    sandbox: &SandBox,
    language: &str,
    code: &str,
    tests: &[hotfix::schemas::TestCase],
    time_limit_secs: u64,
) -> TestVersionResult {
    let mut passed = 0;
    let mut details = Vec::new();

    for (i, test) in tests.iter().enumerate() {
        match sandbox
            .run(language, code, &test.input, time_limit_secs)
            .await
        {
            Ok(output) => {
                let trimmed_output = output.trim().to_string();
                let trimmed_expected = test.expected_output.trim();
                let test_passed = trimmed_output == trimmed_expected;

                if test_passed {
                    passed += 1;
                }

                details.push(TestCaseDetail {
                    input: test.input.clone(),
                    expected: test.expected_output.clone(),
                    actual: output,
                    passed: test_passed,
                });

                // Print progress for debugging
                if test_passed {
                    println!("      Test {}: ✅ PASSED", i + 1);
                } else {
                    println!("      Test {}: ❌ FAILED", i + 1);
                    println!("         Expected: {:?}", trimmed_expected);
                    println!("         Got:      {:?}", trimmed_output);
                }
            }
            Err(e) => {
                details.push(TestCaseDetail {
                    input: test.input.clone(),
                    expected: test.expected_output.clone(),
                    actual: e.clone(),
                    passed: false,
                });
                println!("      Test {}: 💥 ERROR: {}", i + 1, e);
            }
        }
    }

    TestVersionResult { passed, details }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Test all problems and generate a JSON report
#[tokio::test]
async fn test_all_problems_with_report() {
    dotenvy::dotenv().ok();
    let _ = hotfix::config::Config::init();

    let pool = db::init_pool().await;
    let sandbox = SandBox::new();
    let problems = db::get_problems(&pool).await.expect("Failed to fetch problems");

    let mut report = Vec::new();

    for problem in problems {
        let tests: Vec<hotfix::schemas::TestCase> = serde_json::from_value(problem.tests.clone()).unwrap_or_default();

        let correct_code = unescape_code(&problem.correct_version);
        let incorrect_code = unescape_code(&problem.incorrect_version);

        let correct_result = test_version(&sandbox, &problem.language, &correct_code, &tests, problem.time_limit_seconds as u64).await;
        let incorrect_result = test_version(&sandbox, &problem.language, &incorrect_code, &tests, problem.time_limit_seconds as u64).await;

        report.push(serde_json::json!({
            "id": problem.id,
            "name": problem.name,
            "difficulty": problem.difficulty,
            "correct": {
                "passed": correct_result.passed,
                "total": tests.len(),
                "valid": correct_result.passed == tests.len()
            },
            "incorrect": {
                "passed": incorrect_result.passed,
                "total": tests.len(),
                "valid": incorrect_result.passed == 0
            }
        }));
    }

    let report_json = serde_json::to_string_pretty(&report).unwrap();
    std::fs::write("problem_validation_report.json", report_json).unwrap();
    println!("Report saved to problem_validation_report.json");
}