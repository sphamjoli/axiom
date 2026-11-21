//! The certificate fixtures shared with the Lean checker. Every file in
//! `fixtures/certificates/reject/` breaks exactly one rule of the version 1 or 2
//! format and must be rejected; every file in `fixtures/certificates/accept/`
//! is valid and must be accepted. The Lean checker's tests read the same files,
//! so the two readers agree on what is valid.

use std::fs;
use std::io;
use std::path::PathBuf;

use axiom_cert::Certificate;

#[test]
fn every_rejection_fixture_is_rejected() -> io::Result<()> {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/certificates/reject");
    let mut checked = 0;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let text = fs::read_to_string(&path)?;
        assert!(
            Certificate::from_json(&text).is_err(),
            "accepted {}",
            path.display()
        );
        checked += 1;
    }
    assert_eq!(checked, 49, "every fixture is checked");
    Ok(())
}

#[test]
fn every_acceptance_fixture_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/certificates/accept");
    let mut checked = 0;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        Certificate::from_json(&fs::read_to_string(&path)?)?;
        checked += 1;
    }
    assert_eq!(checked, 7, "every fixture is checked");
    Ok(())
}
