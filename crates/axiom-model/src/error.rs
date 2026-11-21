use std::fmt;

use thiserror::Error;

/// A place in a model file: a one-based line and a one-based column.
///
/// Columns count characters, so in the line `  states up` the keyword
/// `states` starts at column 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// The line, starting at 1.
    pub line: usize,
    /// The column, starting at 1.
    pub column: usize,
}

impl Position {
    /// The start of the file, line 1, column 1.
    pub const START: Self = Self { line: 1, column: 1 };
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}, column {}", self.line, self.column)
    }
}

/// A grammar item the parser was looking for when it found something else.
///
/// It names the grammar, never the text of the file, so an error message
/// such as "expected a name" reveals nothing about the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Expected {
    /// A state or parameter name, such as `up`.
    Name,
    /// A non-negative rational, such as `3` or `9/10`.
    Rational,
    /// A weight term: a rational, a name, or `rational * name`.
    Term,
    /// A model, introduced by `chain` or `retry`.
    Model,
    /// An interval bracket, `[` or `]`.
    Bracket,
    /// The keyword `in`.
    In,
    /// The keyword `states`.
    States,
    /// The keyword `success`.
    Success,
    /// A row, introduced by `from`, or the closing `}`.
    RowOrClose,
    /// A comma.
    Comma,
    /// A colon.
    Colon,
    /// An opening brace, `{`.
    OpenBrace,
    /// A closing brace, `}`.
    CloseBrace,
}

impl fmt::Display for Expected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Name => "a name",
            Self::Rational => "a rational",
            Self::Term => "a weight term",
            Self::Model => "`chain` or `retry`",
            Self::Bracket => "`[` or `]`",
            Self::In => "`in`",
            Self::States => "`states`",
            Self::Success => "`success`",
            Self::RowOrClose => "`from` or `}`",
            Self::Comma => "`,`",
            Self::Colon => "`:`",
            Self::OpenBrace => "`{`",
            Self::CloseBrace => "`}`",
        })
    }
}

/// The reason a model file was rejected, with the position of the fault.
///
/// Messages state positions, counts and limits only. They never quote the
/// file, so a rejection can be logged or shown without leaking its contents.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ModelError {
    /// The file is larger than the size limit.
    #[error("{at}: file has {bytes} bytes, more than the limit of {limit}")]
    InputTooLarge {
        /// Always the start of the file.
        at: Position,
        /// The size of the file in bytes.
        bytes: usize,
        /// The largest accepted size in bytes.
        limit: usize,
    },
    /// A character that no token starts with, such as `^` or a non-ASCII
    /// letter.
    #[error("{at}: unexpected character")]
    UnexpectedCharacter {
        /// The position of the character.
        at: Position,
    },
    /// A number with more digits than the limit.
    #[error("{at}: number has {digits} digits, more than the limit of {limit}")]
    NumberTooLong {
        /// The position of the number.
        at: Position,
        /// The number of digits.
        digits: usize,
        /// The largest accepted number of digits.
        limit: usize,
    },
    /// A fraction whose denominator is zero, such as `1/0`.
    #[error("{at}: fraction has a zero denominator")]
    ZeroDenominator {
        /// The position of the denominator.
        at: Position,
    },
    /// A token that the grammar does not allow here.
    #[error("{at}: expected {expected}")]
    UnexpectedToken {
        /// The position of the token.
        at: Position,
        /// What the grammar allows here.
        expected: Expected,
    },
    /// The file ended where the grammar requires more.
    #[error("{at}: file ended, expected {expected}")]
    UnexpectedEnd {
        /// The end of the file.
        at: Position,
        /// What the grammar requires here.
        expected: Expected,
    },
    /// A keyword, such as `states`, used where a name is required.
    #[error("{at}: a keyword cannot be used as a name")]
    KeywordAsName {
        /// The position of the keyword.
        at: Position,
    },
    /// A name that is not declared as a state or parameter.
    #[error("{at}: name is not declared")]
    UndeclaredName {
        /// The position of the name.
        at: Position,
    },
    /// A name declared twice, as two states, two parameters, or both.
    #[error("{at}: name is already declared at {first}")]
    RepeatedName {
        /// The position of the second declaration.
        at: Position,
        /// The position of the first declaration.
        first: Position,
    },
    /// A parameter used where a state is required, such as `from a:`.
    #[error("{at}: a parameter cannot be used as a state")]
    ParameterAsState {
        /// The position of the parameter name.
        at: Position,
    },
    /// A state used where a parameter is required, such as the weight `up`.
    #[error("{at}: a state cannot be used in a weight")]
    StateAsParameter {
        /// The position of the state name.
        at: Position,
    },
    /// A row that appears before `states` has declared any state.
    #[error("{at}: row appears before the states are declared")]
    StateBeforeDeclaration {
        /// The position of the row's `from`.
        at: Position,
    },
    /// A `states` line with no names.
    #[error("{at}: chain declares no states")]
    NoStates {
        /// The position of `states`.
        at: Position,
    },
    /// More states than the limit.
    #[error("{at}: chain declares more than {limit} states")]
    TooManyStates {
        /// The position of the first state beyond the limit.
        at: Position,
        /// The largest accepted number of states.
        limit: usize,
    },
    /// More parameters than the limit.
    #[error("{at}: file declares more than {limit} parameters")]
    TooManyParameters {
        /// The position of the first parameter beyond the limit.
        at: Position,
        /// The largest accepted number of parameters.
        limit: usize,
    },
    /// A state with no `from` row.
    #[error("{at}: state has no row")]
    MissingRow {
        /// The position where the state is declared.
        at: Position,
    },
    /// A second row for the same state.
    #[error("{at}: state already has a row at {first}")]
    RepeatedRow {
        /// The position of the second row's state name.
        at: Position,
        /// The position of the first row's state name.
        first: Position,
    },
    /// A state listed twice in one row.
    #[error("{at}: state already appears in this row at {first}")]
    RepeatedTarget {
        /// The position of the second occurrence.
        at: Position,
        /// The position of the first occurrence.
        first: Position,
    },
    /// A row whose weights do not sum to exactly one for every parameter
    /// value.
    #[error("{at}: row weights do not sum to one")]
    RowNotNormalised {
        /// The position of the row's `from`.
        at: Position,
    },
    /// A weight below zero for some parameter value in range.
    #[error("{at}: weight can be below zero")]
    WeightBelowZero {
        /// The position of the weight.
        at: Position,
    },
    /// A weight above one for some parameter value in range.
    #[error("{at}: weight can be above one")]
    WeightAboveOne {
        /// The position of the weight.
        at: Position,
    },
    /// A parameter interval that contains no value, such as `]1, 1[`.
    #[error("{at}: parameter interval is empty")]
    EmptyInterval {
        /// The position of the interval's opening bracket.
        at: Position,
    },
    /// A parameter that no weight uses.
    #[error("{at}: parameter is never used")]
    UnusedParameter {
        /// The position of the parameter's name in its declaration.
        at: Position,
    },
    /// A product of a parameter with something else, such as `a*b`, which
    /// would make the weight nonlinear.
    #[error("{at}: weight is not affine")]
    NonlinearWeight {
        /// The position of the `*`.
        at: Position,
    },
    /// Input after the model, such as a second model.
    #[error("{at}: a file holds exactly one model")]
    TrailingInput {
        /// The position of the first token after the model.
        at: Position,
    },
}

impl ModelError {
    /// The position of the fault.
    #[must_use]
    pub fn position(&self) -> Position {
        match self {
            Self::InputTooLarge { at, .. }
            | Self::UnexpectedCharacter { at }
            | Self::NumberTooLong { at, .. }
            | Self::ZeroDenominator { at }
            | Self::UnexpectedToken { at, .. }
            | Self::UnexpectedEnd { at, .. }
            | Self::KeywordAsName { at }
            | Self::UndeclaredName { at }
            | Self::RepeatedName { at, .. }
            | Self::ParameterAsState { at }
            | Self::StateAsParameter { at }
            | Self::StateBeforeDeclaration { at }
            | Self::NoStates { at }
            | Self::TooManyStates { at, .. }
            | Self::TooManyParameters { at, .. }
            | Self::MissingRow { at }
            | Self::RepeatedRow { at, .. }
            | Self::RepeatedTarget { at, .. }
            | Self::RowNotNormalised { at }
            | Self::WeightBelowZero { at }
            | Self::WeightAboveOne { at }
            | Self::EmptyInterval { at }
            | Self::UnusedParameter { at }
            | Self::NonlinearWeight { at }
            | Self::TrailingInput { at } => *at,
        }
    }

    /// A stable kebab-case name for the kind of fault, such as
    /// `row-not-normalised`, for tests and tools that match on it.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::InputTooLarge { .. } => "input-too-large",
            Self::UnexpectedCharacter { .. } => "unexpected-character",
            Self::NumberTooLong { .. } => "number-too-long",
            Self::ZeroDenominator { .. } => "zero-denominator",
            Self::UnexpectedToken { .. } => "unexpected-token",
            Self::UnexpectedEnd { .. } => "unexpected-end",
            Self::KeywordAsName { .. } => "keyword-as-name",
            Self::UndeclaredName { .. } => "undeclared-name",
            Self::RepeatedName { .. } => "repeated-name",
            Self::ParameterAsState { .. } => "parameter-as-state",
            Self::StateAsParameter { .. } => "state-as-parameter",
            Self::StateBeforeDeclaration { .. } => "state-before-declaration",
            Self::NoStates { .. } => "no-states",
            Self::TooManyStates { .. } => "too-many-states",
            Self::TooManyParameters { .. } => "too-many-parameters",
            Self::MissingRow { .. } => "missing-row",
            Self::RepeatedRow { .. } => "repeated-row",
            Self::RepeatedTarget { .. } => "repeated-target",
            Self::RowNotNormalised { .. } => "row-not-normalised",
            Self::WeightBelowZero { .. } => "weight-below-zero",
            Self::WeightAboveOne { .. } => "weight-above-one",
            Self::EmptyInterval { .. } => "empty-interval",
            Self::UnusedParameter { .. } => "unused-parameter",
            Self::NonlinearWeight { .. } => "nonlinear-weight",
            Self::TrailingInput { .. } => "trailing-input",
        }
    }
}
