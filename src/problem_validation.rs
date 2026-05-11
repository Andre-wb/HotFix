/// Helper for problem approving

use crate::sandbox::SandBox as Sandbox;
use crate::schemas::GeneratedProblem;

pub async fn validate_problem(
    sandbox: &Sandbox,
    problem: &GeneratedProblem,
    language: &str,
) -> Result<(), String> {
    eprintln!("\n=== Starting problem validation ===");
    eprintln!("Problem: {}", problem.name);
    eprintln!("Language: {}", language);
    eprintln!("Number of tests: {}", problem.tests.len());
    eprintln!("Time limit: {} seconds", problem.time_limit_seconds);

    // Validate correct version
    eprintln!("\n--- Testing CORRECT version ---");
    for (i, test) in problem.tests.iter().enumerate() {
        eprintln!("Test {}: input = {:?}", i, test.input);
        eprintln!("Expected output = {:?}", test.expected_output);

        let output = sandbox
            .run(language, &problem.correct_version, &test.input, problem.time_limit_seconds as u64)
            .await
            .map_err(|e| {
                eprintln!("❌ Correct version FAILED test {}: {}", i, e);
                format!("Correct version failed test {}: {}", i, e)
            })?;

        eprintln!("Got output = {:?}", output);

        if output.trim() != test.expected_output.trim() {
            eprintln!("❌ Output mismatch on test {}", i);
            eprintln!("   Expected: {:?}", test.expected_output.trim());
            eprintln!("   Got:      {:?}", output.trim());
            return Err(format!(
                "Correct version wrong output on test {}. Expected '{}', got '{}'",
                i, test.expected_output, output
            ));
        }
        eprintln!("✓ Test {} passed", i);
    }
    eprintln!("✓ Correct version passed all tests");

    // Validate incorrect version
    eprintln!("\n--- Testing INCORRECT version ---");
    let mut fails = false;
    for (i, test) in problem.tests.iter().enumerate() {
        eprintln!("Test {}: input = {:?}", i, test.input);

        match sandbox.run(language, &problem.incorrect_version, &test.input, problem.time_limit_seconds as u64).await {
            Ok(output) => {
                eprintln!("Got output = {:?}", output);
                if output.trim() != test.expected_output.trim() {
                    eprintln!("✓ Incorrect version FAILED test {} (output mismatch)", i);
                    fails = true;
                    break;
                } else {
                    eprintln!("⚠ Incorrect version PASSED test {} (output matched)", i);
                }
            }
            Err(e) => {
                eprintln!("✓ Incorrect version FAILED test {} (error: {})", i, e);
                fails = true;
                break;
            }
        }
    }

    if !fails {
        eprintln!("❌ VALIDATION FAILED: Incorrect version passed all tests");
        eprintln!("   AI didn't create proper bugs in the incorrect version");
        return Err("Incorrect version passed all tests — AI didn't create proper bugs".to_string());
    }

    eprintln!("✅ VALIDATION PASSED: Problem is valid!");
    eprintln!("=== Validation complete ===\n");
    Ok(())
}