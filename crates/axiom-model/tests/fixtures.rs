//! The model fixtures in `fixtures/models/`. Each valid model has a sibling
//! `.expected.json` of hand-computed exact results, and each file in
//! `fixtures/models/reject/` has one naming the fault and its position.

mod common;

use std::fs;

use axiom_core::{Probability, TwoStateChain};
use common::{TestError, array, chain, expected, field, fixture, fraction, matrix, models_dir};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
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

fn two_state(rows: &[Vec<BigRational>]) -> Result<TwoStateChain, TestError> {
    let entry = |row: usize, column: usize| {
        rows.get(row)
            .and_then(|row| row.get(column))
            .ok_or(TestError::Field("matrix"))
    };
    Ok(TwoStateChain::new(
        Probability::try_new(entry(0, 1)?)?,
        Probability::try_new(entry(1, 0)?)?,
    ))
}

#[test]
fn links_matches_its_expected_results() -> Result<(), TestError> {
    let expected = expected("links.expected.json")?;
    let links = chain(&fixture("links.axm")?)?;
    assert_eq!(links.states, expected_states(&expected)?);
    let rows = matrix(&links)?;
    assert_eq!(rows, expected_matrix(&expected)?);

    let update = two_state(&rows)?.second_state_update();
    assert_eq!(
        update.factor(),
        &fraction(field(&expected, "lambda")?, "lambda")?
    );
    assert_eq!(
        update.offset(),
        &fraction(field(&expected, "offset")?, "offset")?
    );
    let steady = update.offset() / (BigRational::one() - update.factor());
    assert_eq!(
        steady,
        fraction(field(&expected, "steady_down")?, "steady_down")?
    );
    assert_eq!(
        Some(update.factor().abs() < BigRational::one()),
        field(&expected, "convergent")?.as_bool()
    );

    let iterates = array(&expected, "iterates_from_down_one")?;
    for (value, wanted) in update.iterate(BigRational::one()).zip(iterates) {
        assert_eq!(value, fraction(wanted, "iterates_from_down_one")?);
    }

    let tolerance = fraction(field(&expected, "tolerance")?, "tolerance")?;
    let steps = update
        .iterate(BigRational::one())
        .take(1000)
        .position(|value| (value - &steady).abs() <= tolerance);
    let wanted = field(&expected, "steps_to_tolerance_from_down_one")?.as_u64();
    assert_eq!(steps.and_then(|steps| u64::try_from(steps).ok()), wanted);
    Ok(())
}

#[test]
fn periodic_parses_but_does_not_converge() -> Result<(), TestError> {
    let expected = expected("periodic.expected.json")?;
    let flip = chain(&fixture("periodic.axm")?)?;
    assert_eq!(flip.states, expected_states(&expected)?);
    let rows = matrix(&flip)?;
    assert_eq!(rows, expected_matrix(&expected)?);
    let update = two_state(&rows)?.second_state_update();
    assert_eq!(
        update.factor(),
        &fraction(field(&expected, "lambda")?, "lambda")?
    );
    assert_eq!(
        update.offset(),
        &fraction(field(&expected, "offset")?, "offset")?
    );
    assert_eq!(
        Some(update.factor().abs() < BigRational::one()),
        field(&expected, "convergent")?.as_bool()
    );
    Ok(())
}

#[test]
fn retry_matches_its_expected_results() -> Result<(), TestError> {
    let expected = expected("retry.expected.json")?;
    let request = common::retry(&fixture("retry.axm")?)?;
    let success = request
        .probability
        .as_ref()
        .ok_or(TestError::WrongForm)?
        .value()
        .clone();
    assert_eq!(success, fraction(field(&expected, "success")?, "success")?);
    assert!(request.success.is_constant());

    let succeeds = if success.is_zero() {
        BigRational::zero()
    } else {
        BigRational::one()
    };
    assert_eq!(
        succeeds,
        fraction(
            field(&expected, "success_probability")?,
            "success_probability"
        )?
    );
    let attempts = BigRational::one() / &success;
    assert_eq!(
        attempts,
        fraction(field(&expected, "expected_attempts")?, "expected_attempts")?
    );
    let count = field(&expected, "failure_attempts")?
        .as_u64()
        .ok_or(TestError::Field("failure_attempts"))?;
    let fails = BigRational::one() - &success;
    let mut failure = BigRational::one();
    for _ in 0..count {
        failure *= &fails;
    }
    assert_eq!(
        failure,
        fraction(
            field(&expected, "failure_probability")?,
            "failure_probability"
        )?
    );
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
