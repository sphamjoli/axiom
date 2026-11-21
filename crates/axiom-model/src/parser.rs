//! Builds the syntax tree from tokens, following the grammar of the
//! language.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;

use crate::ast::{
    ChainDecl, Endpoint, Entry, File, ModelDecl, Name, ParamDecl, Rational, RetryDecl, Row, Sign,
    Term, Weight,
};
use crate::error::{Expected, ModelError, Position};
use crate::lexer::{Keyword, Token, TokenKind, lex};
use crate::{MAX_INPUT_BYTES, MAX_PARAMETERS, MAX_STATES};

/// Parses `source` into a syntax tree without the static checks.
///
/// Size limits are enforced here, before any arithmetic, so that the work
/// done on a hostile file stays bounded: the file may hold at most
/// [`MAX_INPUT_BYTES`] bytes, each number at most [`MAX_DIGITS`] digits, a
/// chain at most [`MAX_STATES`] states and a file at most [`MAX_PARAMETERS`]
/// parameters.
///
/// [`MAX_DIGITS`]: crate::MAX_DIGITS
///
/// # Errors
///
/// Returns the first lexical or grammatical fault, with its position.
pub fn parse_syntax(source: &str) -> Result<File, ModelError> {
    if source.len() > MAX_INPUT_BYTES {
        return Err(ModelError::InputTooLarge {
            at: Position::START,
            bytes: source.len(),
            limit: MAX_INPUT_BYTES,
        });
    }
    let (tokens, end) = lex(source)?;
    let mut parser = Parser {
        tokens,
        next: 0,
        end,
    };
    parser.file()
}

struct Parser {
    tokens: Vec<Token>,
    next: usize,
    end: Position,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.next)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|token| &token.kind)
    }

    fn error(&self, expected: Expected) -> ModelError {
        match self.peek() {
            Some(token) => ModelError::UnexpectedToken {
                at: token.at,
                expected,
            },
            None => ModelError::UnexpectedEnd {
                at: self.end,
                expected,
            },
        }
    }

    fn expect_token(
        &mut self,
        kind: &TokenKind,
        expected: Expected,
    ) -> Result<Position, ModelError> {
        match self.peek() {
            Some(token) if &token.kind == kind => {
                let at = token.at;
                self.next += 1;
                Ok(at)
            }
            _ => Err(self.error(expected)),
        }
    }

    fn expect_keyword(
        &mut self,
        keyword: Keyword,
        expected: Expected,
    ) -> Result<Position, ModelError> {
        self.expect_token(&TokenKind::Keyword(keyword), expected)
    }

    fn name(&mut self) -> Result<Name, ModelError> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::Name(text),
                at,
            }) => {
                let name = Name {
                    text: text.clone(),
                    at: *at,
                };
                self.next += 1;
                Ok(name)
            }
            Some(Token {
                kind: TokenKind::Keyword(_),
                at,
            }) => Err(ModelError::KeywordAsName { at: *at }),
            _ => Err(self.error(Expected::Name)),
        }
    }

    fn number(&mut self) -> Result<(BigInt, Position), ModelError> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::Number(value),
                at,
            }) => {
                let found = (value.clone(), *at);
                self.next += 1;
                Ok(found)
            }
            _ => Err(self.error(Expected::Rational)),
        }
    }

    fn rational(&mut self) -> Result<Rational, ModelError> {
        let (numerator, at) = self.number()?;
        if self.peek_kind() != Some(&TokenKind::Slash) {
            return Ok(Rational {
                value: BigRational::from_integer(numerator),
                at,
            });
        }
        self.next += 1;
        let (denominator, denominator_at) = self.number()?;
        if denominator.is_zero() {
            return Err(ModelError::ZeroDenominator { at: denominator_at });
        }
        Ok(Rational {
            value: BigRational::new(numerator, denominator),
            at,
        })
    }

    fn file(&mut self) -> Result<File, ModelError> {
        let mut params = Vec::new();
        while let Some(Token {
            kind: TokenKind::Keyword(Keyword::Param),
            at,
        }) = self.peek()
        {
            if params.len() == MAX_PARAMETERS {
                return Err(ModelError::TooManyParameters {
                    at: *at,
                    limit: MAX_PARAMETERS,
                });
            }
            self.next += 1;
            params.push(self.param()?);
        }
        let model = match self.peek_kind() {
            Some(TokenKind::Keyword(Keyword::Chain)) => ModelDecl::Chain(self.chain()?),
            Some(TokenKind::Keyword(Keyword::Retry)) => ModelDecl::Retry(self.retry()?),
            _ => return Err(self.error(Expected::Model)),
        };
        if let Some(token) = self.peek() {
            return Err(ModelError::TrailingInput { at: token.at });
        }
        Ok(File { params, model })
    }

    fn bracket(&mut self) -> Result<(TokenKind, Position), ModelError> {
        match self.peek() {
            Some(Token {
                kind: kind @ (TokenKind::OpenBracket | TokenKind::CloseBracket),
                at,
            }) => {
                let found = (kind.clone(), *at);
                self.next += 1;
                Ok(found)
            }
            _ => Err(self.error(Expected::Bracket)),
        }
    }

    fn param(&mut self) -> Result<ParamDecl, ModelError> {
        let name = self.name()?;
        self.expect_keyword(Keyword::In, Expected::In)?;
        let (open, interval_at) = self.bracket()?;
        let lower = self.rational()?;
        self.expect_token(&TokenKind::Comma, Expected::Comma)?;
        let upper = self.rational()?;
        let (close, _) = self.bracket()?;
        Ok(ParamDecl {
            name,
            interval_at,
            lower: Endpoint {
                value: lower,
                closed: open == TokenKind::OpenBracket,
            },
            upper: Endpoint {
                value: upper,
                closed: close == TokenKind::CloseBracket,
            },
        })
    }

    fn chain(&mut self) -> Result<ChainDecl, ModelError> {
        self.next += 1;
        let name = self.name()?;
        self.expect_token(&TokenKind::OpenBrace, Expected::OpenBrace)?;
        if let Some(Token {
            kind: TokenKind::Keyword(Keyword::From),
            at,
        }) = self.peek()
        {
            return Err(ModelError::StateBeforeDeclaration { at: *at });
        }
        let states_at = self.expect_keyword(Keyword::States, Expected::States)?;
        if matches!(
            self.peek_kind(),
            None | Some(TokenKind::Keyword(Keyword::From) | TokenKind::CloseBrace)
        ) {
            return Err(ModelError::NoStates { at: states_at });
        }
        let mut states = vec![self.name()?];
        while self.peek_kind() == Some(&TokenKind::Comma) {
            self.next += 1;
            let state = self.name()?;
            if states.len() == MAX_STATES {
                return Err(ModelError::TooManyStates {
                    at: state.at,
                    limit: MAX_STATES,
                });
            }
            states.push(state);
        }
        let mut rows = Vec::new();
        loop {
            match self.peek() {
                Some(Token {
                    kind: TokenKind::CloseBrace,
                    ..
                }) => {
                    self.next += 1;
                    break;
                }
                Some(Token {
                    kind: TokenKind::Keyword(Keyword::From),
                    at,
                }) => {
                    let at = *at;
                    self.next += 1;
                    rows.push(self.row(at)?);
                }
                _ => return Err(self.error(Expected::RowOrClose)),
            }
        }
        Ok(ChainDecl {
            name,
            states_at,
            states,
            rows,
        })
    }

    fn row(&mut self, at: Position) -> Result<Row, ModelError> {
        let source = self.name()?;
        self.expect_token(&TokenKind::Colon, Expected::Colon)?;
        let mut entries = vec![self.entry()?];
        while self.peek_kind() == Some(&TokenKind::Comma) {
            self.next += 1;
            entries.push(self.entry()?);
        }
        Ok(Row {
            at,
            source,
            entries,
        })
    }

    fn entry(&mut self) -> Result<Entry, ModelError> {
        let target = self.name()?;
        let weight = self.weight()?;
        Ok(Entry { target, weight })
    }

    fn retry(&mut self) -> Result<RetryDecl, ModelError> {
        self.next += 1;
        let name = self.name()?;
        self.expect_token(&TokenKind::OpenBrace, Expected::OpenBrace)?;
        self.expect_keyword(Keyword::Success, Expected::Success)?;
        let success = self.weight()?;
        self.expect_token(&TokenKind::CloseBrace, Expected::CloseBrace)?;
        Ok(RetryDecl { name, success })
    }

    fn weight(&mut self) -> Result<Weight, ModelError> {
        let at = self.peek().map_or(self.end, |token| token.at);
        let mut terms = vec![(Sign::Plus, self.term()?)];
        loop {
            let sign = match self.peek_kind() {
                Some(TokenKind::Plus) => Sign::Plus,
                Some(TokenKind::Minus) => Sign::Minus,
                _ => break,
            };
            self.next += 1;
            terms.push((sign, self.term()?));
        }
        Ok(Weight { terms, at })
    }

    fn term(&mut self) -> Result<Term, ModelError> {
        let term = match self.peek_kind() {
            Some(TokenKind::Number(_)) => {
                let factor = self.rational()?;
                if self.peek_kind() == Some(&TokenKind::Star) {
                    self.next += 1;
                    Term::Scaled(factor, self.name()?)
                } else {
                    Term::Constant(factor)
                }
            }
            Some(TokenKind::Name(_)) => Term::Parameter(self.name()?),
            Some(TokenKind::Keyword(_)) => {
                let at = self.peek().map_or(self.end, |token| token.at);
                return Err(ModelError::KeywordAsName { at });
            }
            _ => return Err(self.error(Expected::Term)),
        };
        if let Some(Token {
            kind: TokenKind::Star,
            at,
        }) = self.peek()
        {
            return Err(ModelError::NonlinearWeight { at: *at });
        }
        Ok(term)
    }
}
