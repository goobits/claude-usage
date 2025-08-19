// Script to compare multiple dates against expected results
// Run with: cargo run --example compare_dates

use anyhow::Result;
use std::process::Command;

#[derive(Debug)]
struct DateResult {
    date: String,
    expected_cost: f64,
    expected_sessions: usize,
    actual_cost: f64,
    actual_sessions: usize,
    matches: bool,
}

fn parse_daily_output(output: &str, date: &str) -> (f64, usize) {
    // Look for the date line in the output
    for line in output.lines() {
        if line.contains(date) && line.contains("$") {
            // Parse: "📅 2025-07-08 — $0.06 (1 sessions)"
            if let Some(cost_start) = line.find("$") {
                let rest = &line[cost_start + 1..];
                if let Some(space_pos) = rest.find(" ") {
                    if let Ok(cost) = rest[..space_pos].parse::<f64>() {
                        // Extract session count
                        if let Some(paren_start) = rest.find("(") {
                            if let Some(sessions_end) = rest.find(" session") {
                                let sessions_str = &rest[paren_start + 1..sessions_end];
                                if let Ok(sessions) = sessions_str.parse::<usize>() {
                                    return (cost, sessions);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (0.0, 0)
}

fn main() -> Result<()> {
    println!("🔍 Comparing Claude Usage dates");
    println!("{}", "=".repeat(80));

    // Expected results from user
    let expected_results = vec![
        ("2025-07-10", 0.00, 0),
        ("2025-07-09", 0.00, 0),
        ("2025-07-08", 0.06, 1),
        ("2025-07-07", 0.00, 0),
        ("2025-07-06", 0.00, 0),
        ("2025-07-05", 0.00, 0),
        ("2025-07-04", 0.00, 0),
    ];

    let mut all_results = Vec::new();

    // Build the tool first
    println!("Building claude-usage tool...");
    let build_output = Command::new("cargo")
        .args(&["build", "--release"])
        .output()?;

    if !build_output.status.success() {
        println!(
            "Failed to build: {}",
            String::from_utf8_lossy(&build_output.stderr)
        );
        return Ok(());
    }

    // Run claude-usage for each date
    for (date, expected_cost, expected_sessions) in expected_results {
        print!("Checking {}... ", date);

        let output = Command::new("cargo")
            .args(&[
                "run",
                "--release",
                "--",
                "daily",
                "--since",
                date,
                "--until",
                date,
            ])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let (actual_cost, actual_sessions) = parse_daily_output(&output_str, date);

        let matches =
            (expected_cost - actual_cost).abs() < 0.01 && expected_sessions == actual_sessions;

        all_results.push(DateResult {
            date: date.to_string(),
            expected_cost,
            expected_sessions,
            actual_cost,
            actual_sessions,
            matches,
        });

        println!("{}", if matches { "✅" } else { "❌" });
    }

    // Display comparison table
    println!("\n📊 Comparison Results:");
    println!("{}", "=".repeat(80));
    println!(
        "{:<12} | {:>10} | {:>10} | {:>10} | {:>10} | {}",
        "Date", "Expected $", "Actual $", "Exp Sess", "Act Sess", "Match"
    );
    println!("{}", "-".repeat(80));

    let mut matches = 0;
    let mut total_expected_cost = 0.0;
    let mut total_actual_cost = 0.0;

    for result in &all_results {
        if result.matches {
            matches += 1;
        }

        total_expected_cost += result.expected_cost;
        total_actual_cost += result.actual_cost;

        println!(
            "{:<12} | ${:>9.2} | ${:>9.2} | {:>10} | {:>10} | {}",
            result.date,
            result.expected_cost,
            result.actual_cost,
            result.expected_sessions,
            result.actual_sessions,
            if result.matches {
                "✅ Match"
            } else {
                "❌ Differ"
            }
        );
    }

    println!("{}", "-".repeat(80));
    println!(
        "{:<12} | ${:>9.2} | ${:>9.2} |",
        "Totals:", total_expected_cost, total_actual_cost
    );
    println!(
        "\n📈 Summary: {}/{} dates match expected results",
        matches,
        all_results.len()
    );

    if matches < all_results.len() {
        println!("\n⚠️  Discrepancies found!");
        println!("\nTo investigate specific dates, run:");
        for result in &all_results {
            if !result.matches {
                println!(
                    "   cargo run --example check_date  # (edit to set target_date = \"{}\")",
                    result.date
                );
            }
        }
    }

    Ok(())
}
