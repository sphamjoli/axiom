//! Splits a model file into tokens.

use num_bigint::BigInt;

use crate::MAX_DIGITS;
use crate::error::{ModelError, Position};

/// A reserved word of the language, which cannot be used as a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keyword {
    Param,
    In,
    Chain,
    States,
    From,
    Retry,
    Success,
}

impl Keyword {
    fn from_word(word: &str) -> Option<Self> {
        match word {
            "param" => Some(Self::Param),
            "in" => Some(Self::In),
            "chain" => Some(Self::Chain),
            "states" => Some(Self::States),
            "from" => Some(Self::From),
            "retry" => Some(Self::Retry),
            "success" => Some(Self::Success),
            _ => None,
        }
    }
}

/// The kind of a token, with its value for names and numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Name(String),
    Keyword(Keyword),
    Number(BigInt),
    Slash,
    Comma,
    Colon,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Plus,
    Minus,
    Star,
}

/// A token and the position of its first character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) at: Position,
}

/// The tokens of `source` and the position just past its end.
///
/// Spaces, tabs, carriage returns and newlines separate tokens and carry no
/// other meaning; `#` starts a comment that runs to the end of the line.
pub(crate) fn lex(source: &str) -> Result<(Vec<Token>, Position), ModelError> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();
    let mut line = 1;
    let mut column = 1;
    while let Some(&(start, character)) = chars.peek() {
        let at = Position { line, column };
        if character == '\n' {
            chars.next();
            line += 1;
            column = 1;
            continue;
        }
        if matches!(character, ' ' | '\t' | '\r') {
            chars.next();
            column += 1;
            continue;
        }
        if character == '#' {
            while chars.next_if(|&(_, next)| next != '\n').is_some() {}
            continue;
        }
        if character.is_ascii_alphabetic() {
            let mut end = start;
            while let Some((index, next)) =
                chars.next_if(|&(_, next)| next.is_ascii_alphanumeric() || next == '_')
            {
                end = index + next.len_utf8();
                column += 1;
            }
            let word = source.get(start..end).unwrap_or_default();
            let kind = Keyword::from_word(word)
                .map_or_else(|| TokenKind::Name(word.to_owned()), TokenKind::Keyword);
            tokens.push(Token { kind, at });
            continue;
        }
        if character.is_ascii_digit() {
            let mut end = start;
            while let Some((index, next)) = chars.next_if(|&(_, next)| next.is_ascii_digit()) {
                end = index + next.len_utf8();
                column += 1;
            }
            let digits = source.get(start..end).unwrap_or_default();
            if digits.len() > MAX_DIGITS {
                return Err(ModelError::NumberTooLong {
                    at,
                    digits: digits.len(),
                    limit: MAX_DIGITS,
                });
            }
            let value = BigInt::parse_bytes(digits.as_bytes(), 10)
                .ok_or(ModelError::UnexpectedCharacter { at })?;
            tokens.push(Token {
                kind: TokenKind::Number(value),
                at,
            });
            continue;
        }
        let kind = match character {
            '/' => TokenKind::Slash,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            _ => return Err(ModelError::UnexpectedCharacter { at }),
        };
        chars.next();
        column += 1;
        tokens.push(Token { kind, at });
    }
    Ok((tokens, Position { line, column }))
}
