//! The syntax tree of a model file, as written, before the static checks.
//!
//! Every node keeps the [`Position`] where it starts, so the checker can
//! point at the exact line and column of a fault.

use num_rational::BigRational;

use crate::error::Position;

/// A name and where it appears, such as the state `up` at line 4, column 10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name itself.
    pub text: String,
    /// Where it appears.
    pub at: Position,
}

/// A non-negative rational literal, such as `9/10`, and where it starts.
///
/// The value is kept in lowest terms, so `2/4` and `1/2` are the same
/// literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    /// The value, in lowest terms.
    pub value: BigRational,
    /// Where the literal starts.
    pub at: Position,
}

/// One end of a parameter interval.
///
/// In `]0, 1]` the lower end `0` is open (excluded) and the upper end `1` is
/// closed (included).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// The value of the end.
    pub value: Rational,
    /// Whether the value itself belongs to the interval.
    pub closed: bool,
}

/// A parameter declaration, such as `param a in ]0, 1[`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDecl {
    /// The parameter's name.
    pub name: Name,
    /// The position of the interval's opening bracket.
    pub interval_at: Position,
    /// The lower end of the interval.
    pub lower: Endpoint,
    /// The upper end of the interval.
    pub upper: Endpoint,
}

/// Whether a term is added or subtracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// `+`, or the first term of a weight.
    Plus,
    /// `-`.
    Minus,
}

/// One term of a weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// A constant, such as `1`.
    Constant(Rational),
    /// A rational multiple of a parameter, such as `1/2*b`.
    Scaled(Rational, Name),
    /// A parameter on its own, such as `a`, meaning `1*a`.
    Parameter(Name),
}

/// A weight: signed terms added together, such as `1 - a`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weight {
    /// The terms, each with its sign, in the order written.
    pub terms: Vec<(Sign, Term)>,
    /// Where the weight starts.
    pub at: Position,
}

/// One entry of a row: a next state and the weight of moving to it, such as
/// `down 1/10`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The next state.
    pub target: Name,
    /// The probability of moving to it.
    pub weight: Weight,
}

/// A row, such as `from up: up 9/10, down 1/10`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The position of `from`.
    pub at: Position,
    /// The current state.
    pub source: Name,
    /// The listed next states, in the order written.
    pub entries: Vec<Entry>,
}

/// A chain block: `chain name { states ... rows }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainDecl {
    /// The chain's name.
    pub name: Name,
    /// The position of `states`.
    pub states_at: Position,
    /// The declared states, in order.
    pub states: Vec<Name>,
    /// The rows, in the order written.
    pub rows: Vec<Row>,
}

/// A retry block: `retry name { success weight }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryDecl {
    /// The retry's name.
    pub name: Name,
    /// The probability that one attempt succeeds.
    pub success: Weight,
}

/// The model a file defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelDecl {
    /// A finite-state Markov chain.
    Chain(ChainDecl),
    /// A retry recursion.
    Retry(RetryDecl),
}

/// A whole file: parameter declarations followed by one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The parameters, in the order declared.
    pub params: Vec<ParamDecl>,
    /// The model.
    pub model: ModelDecl,
}
