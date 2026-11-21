//! The model fixtures in `fixtures/models/`. Each valid model has a sibling
//! `.expected.json` of what the model reader must produce, and each file in
//! `fixtures/models/reject/` has one naming the fault and its position.

mod common;

use std::fs;

use common::{TestError, array, chain, expected, field, fixture, fraction, matrix, models_dir};
use num_rational::BigRational;
use num_traits::Zero;
use serde_json::Value;

fn expected_matrix(value: &Value) -> Result<Vec<Vec<BigRational>>, TestError> {
    array(value, "matrix")?
        .iter()
        .map(|row| {
            row.as_array()
                .ok_or(TestError::Field("matrix"))?
                .iter()
                .map(|entry| fraction(entry, "matrix"))
                .collect()
        })
        .collect()
}

fn expected_states(value: &Value) -> Result<Vec<String>, TestError> {
    array(value, "states")?
        .iter()
        .map(|state| {
            state
                .as_str()
                .map(str::to_owned)
                .ok_or(TestError::Field("states"))
        })
        .collect()
}

#[test]
fn links_parses_to_its_expected_states_and_matrix() -> Result<(), TestError> {
    let expected = expected("links.expected.json")?;
    let links = chain(&fixture("links.axm")?)?;
    assert_eq!(links.states, expected_states(&expected)?);
    assert_eq!(matrix(&links)?, expected_matrix(&expected)?);
    Ok(())
}

#[test]
fn periodic_parses_to_its_expected_states_and_matrix() -> Result<(), TestError> {
    let expected = expected("periodic.expected.json")?;
    let flip = chain(&fixture("periodic.axm")?)?;
    assert_eq!(flip.states, expected_states(&expected)?);
    assert_eq!(matrix(&flip)?, expected_matrix(&expected)?);
    Ok(())
}

#[test]
fn retry_parses_to_its_expected_success_probability() -> Result<(), TestError> {
    let expected = expected("retry.expected.json")?;
    let request = common::retry(&fixture("retry.axm")?)?;
    let success = request
        .probability
        .as_ref()
        .ok_or(TestError::WrongForm)?
        .value();
    assert_eq!(success, &fraction(field(&expected, "success")?, "success")?);
    assert!(request.success.is_constant());
    Ok(())
}

#[test]
fn parametric_links_matches_its_expected_results() -> Result<(), TestError> {
    let expected = expected("links-param.expected.json")?;
    let links = chain(&fixture("links-param.axm")?)?;
    assert!(links.matrix.is_none());
    assert_eq!(links.states, expected_states(&expected)?);

    let parameters = array(&expected, "parameters")?;
    assert_eq!(links.parameters.len(), parameters.len());
    for (parameter, wanted) in links.parameters.iter().zip(parameters) {
        assert_eq!(
            Some(parameter.name.as_str()),
            field(wanted, "name")?.as_str()
        );
        assert_eq!(
            parameter.lower.value,
            fraction(field(wanted, "lower")?, "lower")?
        );
        assert_eq!(
            parameter.upper.value,
            fraction(field(wanted, "upper")?, "upper")?
        );
        assert_eq!(
            Some(parameter.lower.closed),
            field(wanted, "lower_closed")?.as_bool()
        );
        assert_eq!(
            Some(parameter.upper.closed),
            field(wanted, "upper_closed")?.as_bool()
        );
    }

    let rows = array(&expected, "rows")?;
    assert_eq!(links.rows.len(), rows.len());
    for (row, wanted_row) in links.rows.iter().zip(rows) {
        let wanted_row = wanted_row.as_array().ok_or(TestError::Field("rows"))?;
        assert_eq!(row.len(), wanted_row.len());
        for (weight, wanted) in row.iter().zip(wanted_row) {
            assert_eq!(
                weight.constant(),
                &fraction(field(wanted, "constant")?, "constant")?
            );
            let coefficients = field(wanted, "coefficients")?;
            for (parameter, coefficient) in links.parameters.iter().zip(weight.coefficients()) {
                let wanted = match coefficients.get(&parameter.name) {
                    Some(value) => fraction(value, "coefficients")?,
                    None => BigRational::zero(),
                };
                assert_eq!(coefficient, &wanted);
            }
        }
    }
    Ok(())
}

#[test]
fn every_rejection_fixture_is_rejected_at_its_position() -> Result<(), TestError> {
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for entry in fs::read_dir(models_dir().join("reject"))? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("axm") {
            continue;
        }
        let source = fs::read_to_string(&path)?;
        let wanted: Value =
            serde_json::from_str(&fs::read_to_string(path.with_extension("expected.json"))?)?;
        let error = common::rejection(&source)?;
        let position = error.position();
        let found = (
            Some(error.code()),
            u64::try_from(position.line).ok(),
            u64::try_from(position.column).ok(),
        );
        let wanted = (
            field(&wanted, "error")?.as_str(),
            field(&wanted, "line")?.as_u64(),
            field(&wanted, "column")?.as_u64(),
        );
        if found != wanted {
            mismatches.push(format!("{}: {found:?}", path.display()));
        }
        checked += 1;
    }
    assert_eq!(mismatches, Vec::<String>::new());
    assert_eq!(checked, 29, "every fixture is checked");
    Ok(())
}

#[test]
fn every_valid_fixture_has_expected_results() -> Result<(), TestError> {
    let mut checked = 0;
    for entry in fs::read_dir(models_dir())? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("axm") {
            continue;
        }
        axiom_model::parse(&fs::read_to_string(&path)?)?;
        assert!(path.with_extension("expected.json").is_file());
        checked += 1;
    }
    assert_eq!(checked, 4, "every fixture is checked");
    Ok(())
}
