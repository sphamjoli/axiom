//! The model templates the web interface teaches with, in
//! `fixtures/models/templates/`. The interface imports the same files, so
//! this test pins what each one teaches: the valid ones parse to the form
//! the guide describes, and the broken one fails where the guide says.

mod common;

use axiom_model::CheckedModel;
use common::{TestError, fixture, rejection};

fn template(name: &str) -> Result<String, TestError> {
    fixture(&format!("templates/{name}"))
}

/// `(file, states, exact)`: a chain with that many states, and whether it
/// has an exact matrix (no parameters).
const CHAINS: [(&str, usize, bool); 5] = [
    ("links.axm", 2, true),
    ("slow-repair.axm", 2, true),
    ("flip.axm", 2, true),
    ("server.axm", 3, true),
    ("links-param.axm", 2, false),
];

#[test]
fn chain_templates_parse_as_described() -> Result<(), TestError> {
    for (name, states, exact) in CHAINS {
        let CheckedModel::Chain(chain) = axiom_model::parse(&template(name)?)? else {
            return Err(TestError::Field("expected a chain"));
        };
        if chain.states.len() != states || chain.matrix.is_some() != exact {
            return Err(TestError::Field("chain template has the wrong form"));
        }
    }
    Ok(())
}

#[test]
fn retry_template_parses_with_an_exact_success() -> Result<(), TestError> {
    let CheckedModel::Retry(retry) = axiom_model::parse(&template("retry.axm")?)? else {
        return Err(TestError::Field("expected a retry"));
    };
    if retry.probability.is_none() {
        return Err(TestError::Field("retry template has parameters"));
    }
    Ok(())
}

/// The guide shows this error, code and position, as the one to learn from.
#[test]
fn broken_template_fails_at_the_row_that_sums_to_eleven_tenths() -> Result<(), TestError> {
    let error = rejection(&template("broken.axm")?)?;
    let at = error.position();
    if error.code() != "row-not-normalised" || (at.line, at.column) != (4, 3) {
        return Err(TestError::Field("broken template fails elsewhere"));
    }
    Ok(())
}
