//! Every file in `fixtures/certificates/reject/` breaks exactly one rule of the
//! version 1 format and must be rejected. The Lean checker is tested against
//! the same files, so the two readers agree on what is invalid.

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
    assert_eq!(checked, 17, "every fixture is checked");
    Ok(())
}
