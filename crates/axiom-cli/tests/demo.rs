//! The end-to-end demo: `scripts/demo.sh` prints the pass/reject table and
//! exits non-zero when any outcome differs from the expected one. Skipped
//! when `lake` is not on PATH.

mod common;

use std::process::Command;

use common::{TestError, ensure, lake_available, repo_root};

#[test]
fn lean_demo_accepts_true_claims_and_rejects_negatives_and_mutants() -> Result<(), TestError> {
    if !lake_available("lean_demo_accepts_true_claims_and_rejects_negatives_and_mutants")? {
        return Ok(());
    }
    let output = Command::new(repo_root().join("scripts/demo.sh"))
        .env("AXIOM_BIN", env!("CARGO_BIN_EXE_axiom"))
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    ensure(
        output.status.success(),
        format!(
            "demo failed\n{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        ),
    )?;
    for row in [
        "alice links.axm | 4 | ConvergenceBound | proven | accepted | -",
        "alice links.axm | 5 | Estimate | estimated | accepted | -",
        "alice retry.axm | 0 | Termination | proven | accepted | -",
        "mutant links 12 steps | 4 | ConvergenceBound | proven | rejected | false-claim",
        "mutant links 100 samples | 5 | Estimate | estimated | rejected | false-claim",
        "mutant retry 3 attempts | 0 | Termination | proven | rejected | false-claim",
        "negative convergence-bound-on-retry.json | - | - | - | rejected | checker-not-chain",
        "negative not-normalised.axm | - | - | - | rejected | row-not-normalised",
    ] {
        ensure(stdout.contains(row), format!("missing row: {row}"))?;
    }
    Ok(())
}
