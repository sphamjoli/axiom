//! The static checks that turn a syntax tree into a checked model.

use std::collections::HashMap;

use axiom_core::{Probability, TransitionMatrix, TransitionRow};
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::ast::{ChainDecl, File, ModelDecl, Name, ParamDecl, RetryDecl, Row, Sign, Term, Weight};
use crate::error::{ModelError, Position};
use crate::model::{AffineWeight, Bound, Chain, CheckedModel, Parameter, Retry};

/// Runs every static check on `file` and returns the checked model.
///
/// The checks are, in order: each parameter interval is non-empty and each
/// name is declared once; every row names a declared state, has no repeated
/// next state, and appears once; every weight stays between zero and one for
/// all parameter values in range; every row sums to exactly one for all
/// parameter values; every state has a row; and every parameter is used.
///
/// # Errors
///
/// Returns the first fault found, with its position.
pub fn check(file: &File) -> Result<CheckedModel, ModelError> {
    let mut scope = Scope::default();
    let mut parameters = Vec::with_capacity(file.params.len());
    for param in &file.params {
        check_interval(param)?;
        scope.declare(&param.name, Kind::Parameter(parameters.len()))?;
        parameters.push(Parameter {
            name: param.name.text.clone(),
            lower: Bound {
                value: param.lower.value.value.clone(),
                closed: param.lower.closed,
            },
            upper: Bound {
                value: param.upper.value.value.clone(),
                closed: param.upper.closed,
            },
        });
    }
    let mut checker = Checker {
        scope,
        parameters,
        used: vec![false; file.params.len()],
    };
    let model = match &file.model {
        ModelDecl::Chain(chain) => CheckedModel::Chain(checker.chain(chain)?),
        ModelDecl::Retry(retry) => CheckedModel::Retry(checker.retry(retry)?),
    };
    if let Some(param) = file
        .params
        .iter()
        .zip(&checker.used)
        .find_map(|(param, used)| (!used).then_some(param))
    {
        return Err(ModelError::UnusedParameter { at: param.name.at });
    }
    Ok(model)
}

fn check_interval(param: &ParamDecl) -> Result<(), ModelError> {
    let lower = &param.lower.value.value;
    let upper = &param.upper.value.value;
    let empty = lower > upper || (lower == upper && !(param.lower.closed && param.upper.closed));
    if empty {
        return Err(ModelError::EmptyInterval {
            at: param.interval_at,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Parameter(usize),
    State(usize),
}

#[derive(Debug, Default)]
struct Scope {
    names: HashMap<String, (Kind, Position)>,
}

impl Scope {
    fn declare(&mut self, name: &Name, kind: Kind) -> Result<(), ModelError> {
        if let Some((_, first)) = self.names.get(&name.text) {
            return Err(ModelError::RepeatedName {
                at: name.at,
                first: *first,
            });
        }
        self.names.insert(name.text.clone(), (kind, name.at));
        Ok(())
    }

    fn state(&self, name: &Name) -> Result<usize, ModelError> {
        match self.names.get(&name.text) {
            Some((Kind::State(index), _)) => Ok(*index),
            Some((Kind::Parameter(_), _)) => Err(ModelError::ParameterAsState { at: name.at }),
            None => Err(ModelError::UndeclaredName { at: name.at }),
        }
    }

    fn parameter(&self, name: &Name) -> Result<usize, ModelError> {
        match self.names.get(&name.text) {
            Some((Kind::Parameter(index), _)) => Ok(*index),
            Some((Kind::State(_), _)) => Err(ModelError::StateAsParameter { at: name.at }),
            None => Err(ModelError::UndeclaredName { at: name.at }),
        }
    }
}

struct Checker {
    scope: Scope,
    parameters: Vec<Parameter>,
    used: Vec<bool>,
}

impl Checker {
    fn chain(&mut self, chain: &ChainDecl) -> Result<Chain, ModelError> {
        for (index, state) in chain.states.iter().enumerate() {
            self.scope.declare(state, Kind::State(index))?;
        }
        let count = chain.states.len();
        let mut rows: Vec<Option<(Position, Vec<AffineWeight>)>> = vec![None; count];
        for row in &chain.rows {
            let source = self.scope.state(&row.source)?;
            let weights = self.row(row, count)?;
            match rows.get_mut(source) {
                Some(Some((first, _))) => {
                    return Err(ModelError::RepeatedRow {
                        at: row.source.at,
                        first: *first,
                    });
                }
                Some(slot) => *slot = Some((row.source.at, weights)),
                None => return Err(ModelError::UndeclaredName { at: row.source.at }),
            }
        }
        let mut dense = Vec::with_capacity(count);
        for (state, row) in chain.states.iter().zip(rows) {
            match row {
                Some((_, weights)) => dense.push(weights),
                None => return Err(ModelError::MissingRow { at: state.at }),
            }
        }
        let matrix = if self.parameters.is_empty() {
            Some(constant_matrix(&dense, chain)?)
        } else {
            None
        };
        Ok(Chain {
            name: chain.name.text.clone(),
            parameters: self.parameters.clone(),
            states: chain
                .states
                .iter()
                .map(|state| state.text.clone())
                .collect(),
            rows: dense,
            matrix,
        })
    }

    fn row(&mut self, row: &Row, count: usize) -> Result<Vec<AffineWeight>, ModelError> {
        let parameters = self.parameters.len();
        let mut weights = vec![AffineWeight::zero(parameters); count];
        let mut seen: Vec<Option<Position>> = vec![None; count];
        let mut sum = AffineWeight::zero(parameters);
        for entry in &row.entries {
            let target = self.scope.state(&entry.target)?;
            if let Some(Some(first)) = seen.get(target) {
                return Err(ModelError::RepeatedTarget {
                    at: entry.target.at,
                    first: *first,
                });
            }
            if let Some(slot) = seen.get_mut(target) {
                *slot = Some(entry.target.at);
            }
            let weight = self.weight(&entry.weight)?;
            sum.add(&weight);
            if let Some(slot) = weights.get_mut(target) {
                *slot = weight;
            }
        }
        if !sum.constant().is_one() || !sum.is_constant() {
            return Err(ModelError::RowNotNormalised { at: row.at });
        }
        Ok(weights)
    }

    fn retry(&mut self, retry: &RetryDecl) -> Result<Retry, ModelError> {
        let success = self.weight(&retry.success)?;
        let probability = if self.parameters.is_empty() {
            Some(Probability::try_new(success.constant()).map_err(|_| {
                ModelError::WeightAboveOne {
                    at: retry.success.at,
                }
            })?)
        } else {
            None
        };
        Ok(Retry {
            name: retry.name.text.clone(),
            parameters: self.parameters.clone(),
            success,
            probability,
        })
    }

    /// Builds the affine form of `weight` and checks that it lies between
    /// zero and one for every parameter value in range.
    ///
    /// An affine function on a box of intervals reaches its extremes at the
    /// corners, and each parameter contributes independently, so the least
    /// value is the constant plus, for each parameter, the smaller of its
    /// term at the two ends. Open ends are treated as closed: the extremes
    /// over an open interval are approached but not exceeded, so the
    /// non-strict bounds `0 <= w <= 1` hold on the open interval exactly when
    /// they hold on its closure.
    fn weight(&mut self, weight: &Weight) -> Result<AffineWeight, ModelError> {
        let mut affine = AffineWeight::zero(self.parameters.len());
        for (sign, term) in &weight.terms {
            let (factor, parameter) = match term {
                Term::Constant(value) => (value.value.clone(), None),
                Term::Scaled(value, name) => (value.value.clone(), Some(name)),
                Term::Parameter(name) => (BigRational::one(), Some(name)),
            };
            let factor = match sign {
                Sign::Plus => factor,
                Sign::Minus => -factor,
            };
            match parameter {
                None => affine.add_constant(&factor),
                Some(name) => {
                    let index = self.scope.parameter(name)?;
                    if let Some(used) = self.used.get_mut(index) {
                        *used = true;
                    }
                    affine.add_coefficient(index, &factor);
                }
            }
        }
        let mut least = affine.constant().clone();
        let mut greatest = affine.constant().clone();
        for (coefficient, parameter) in affine.coefficients().iter().zip(&self.parameters) {
            let at_lower = coefficient * &parameter.lower.value;
            let at_upper = coefficient * &parameter.upper.value;
            if at_lower <= at_upper {
                least += at_lower;
                greatest += at_upper;
            } else {
                least += at_upper;
                greatest += at_lower;
            }
        }
        if least < BigRational::zero() {
            return Err(ModelError::WeightBelowZero { at: weight.at });
        }
        if greatest > BigRational::one() {
            return Err(ModelError::WeightAboveOne { at: weight.at });
        }
        Ok(affine)
    }
}

/// The exact matrix of a chain without parameters. Every weight was already
/// checked to be a probability and every row to sum to one, so the mapped
/// errors only restate those checks.
fn constant_matrix(
    rows: &[Vec<AffineWeight>],
    chain: &ChainDecl,
) -> Result<TransitionMatrix, ModelError> {
    let mut built = Vec::with_capacity(rows.len());
    for (row, state) in rows.iter().zip(&chain.states) {
        let probabilities = row
            .iter()
            .map(|weight| {
                Probability::try_new(weight.constant())
                    .map_err(|_| ModelError::WeightAboveOne { at: state.at })
            })
            .collect::<Result<Vec<_>, _>>()?;
        built.push(
            TransitionRow::try_new(probabilities)
                .map_err(|_| ModelError::RowNotNormalised { at: state.at })?,
        );
    }
    TransitionMatrix::try_new(built).map_err(|_| ModelError::NoStates {
        at: chain.states_at,
    })
}
