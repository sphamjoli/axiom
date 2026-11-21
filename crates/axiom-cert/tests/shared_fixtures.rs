//! The certificate fixtures shared with the Lean checker. Every file in
//! `fixtures/certificates/reject/` breaks exactly one rule of the version 1
//! to 4 formats and must be rejected for that rule: `reject-reasons.tsv`
//! names, for each file, the engine's error variant and the Lean checker's
//! reason (`check` when the reader accepts it and the claim check rejects
//! it). Every file in `fixtures/certificates/accept/` is valid and must be
//! accepted. The Lean checker's tests read the same files, so the two readers
//! agree on what is valid and why.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use axiom_cert::Certificate;
use common::TestError;

/// The shared certificate fixture directory.
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/certificates")
}

/// The engine error variant expected for each rejection fixture.
fn expected_variants() -> Result<BTreeMap<String, String>, TestError> {
    let table = fs::read_to_string(fixtures().join("reject-reasons.tsv"))?;
    table
        .lines()
        .map(
            |line| match line.split('\t').collect::<Vec<_>>().as_slice() {
                [name, variant, _lean] => Ok(((*name).to_owned(), (*variant).to_owned())),
                _ => Err(TestError::Fixture),
            },
        )
        .collect()
}

/// The name of the error's variant, such as `FalseClaim`.
fn variant(error: &impl std::fmt::Debug) -> String {
    format!("{error:?}")
        .split([' ', '(', '{'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn every_rejection_fixture_is_rejected_for_its_rule() -> Result<(), TestError> {
    let expected = expected_variants()?;
    let mut names = Vec::new();
    for entry in fs::read_dir(fixtures().join("reject"))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let found = match Certificate::from_json(&fs::read_to_string(&path)?) {
            Ok(_) => "accepted".to_owned(),
            Err(error) => variant(&error),
        };
        assert_eq!(Some(&found), expected.get(&name), "{name}");
        names.push(name);
    }
    names.sort();
    assert_eq!(
        names,
        expected.keys().cloned().collect::<Vec<_>>(),
        "every fixture has exactly one expected reason"
    );
    Ok(())
}

#[test]
fn every_acceptance_fixture_is_accepted() -> Result<(), TestError> {
    let mut checked = 0;
    for entry in fs::read_dir(fixtures().join("accept"))? {
        let path = entry?.path();
        Certificate::from_json(&fs::read_to_string(&path)?)?;
        checked += 1;
    }
    assert_eq!(checked, 18, "every fixture is checked");
    Ok(())
}
