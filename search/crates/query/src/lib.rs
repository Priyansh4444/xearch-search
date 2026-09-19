//! Small, backend-independent Google-style grammar. Stop words are ordinary terms.
use search_model::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Term(String),
    Phrase(String),
    Author(String),
    Since(i64),
    Until(i64),
    And(Vec<Self>),
    Or(Vec<Self>),
    Not(Box<Self>),
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Word(String),
    Phrase(String),
    Open,
    Close,
    Minus,
    Or,
}

/// Parse a query with implicit AND, explicit OR, grouping and exclusion.
///
/// # Errors
/// Rejects malformed, empty or oversized queries and unsupported operators.
pub fn parse(raw: &str, author: Option<&str>) -> Result<Expr> {
    if raw.chars().count() > 300 {
        return invalid("Keep searches under 300 characters.");
    }
    let tokens = lex(raw)?;
    let mut parser = Parser {
        tokens: tokens.into_iter().peekable(),
    };
    let expression = if parser.tokens.peek().is_none() {
        None
    } else {
        Some(parser.or()?)
    };
    if parser.tokens.next().is_some() {
        return invalid("Unexpected closing parenthesis.");
    }
    match (expression, author) {
        (Some(expr), Some(handle)) => Ok(Expr::And(vec![
            expr,
            Expr::Author(normalize_author(handle)?),
        ])),
        (Some(expr), None) => Ok(expr),
        (None, Some(handle)) => Ok(Expr::Author(normalize_author(handle)?)),
        (None, None) => invalid("Enter a search."),
    }
}

/// Normalize a source handle without conflating it with numeric author identity.
///
/// # Errors
/// Returns an error for invalid X handles.
pub fn normalize_author(raw: &str) -> Result<String> {
    let value = raw.strip_prefix('@').unwrap_or(raw);
    if !(1..=15).contains(&value.len())
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return invalid("Invalid author handle.");
    }
    Ok(value.to_ascii_lowercase())
}

fn invalid<T>(message: &str) -> Result<T> {
    Err(Error::Invalid(message.into()))
}

fn lex(raw: &str) -> Result<Vec<Token>> {
    let mut chars = raw.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '(' => tokens.push(Token::Open),
            ')' => tokens.push(Token::Close),
            '-' => tokens.push(Token::Minus),
            '"' => {
                let mut value = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => value.push(
                            chars
                                .next()
                                .ok_or_else(|| Error::Invalid("Unfinished escape.".into()))?,
                        ),
                        _ => value.push(c),
                    }
                }
                if !closed || value.trim().is_empty() {
                    return invalid("Use a nonempty, closed quoted phrase.");
                }
                tokens.push(Token::Phrase(value));
            }
            _ => {
                let mut word = String::from(c);
                while chars
                    .peek()
                    .is_some_and(|c| !c.is_whitespace() && !matches!(c, '(' | ')' | '"'))
                {
                    if let Some(c) = chars.next() {
                        word.push(c);
                    }
                }
                tokens.push(if word == "OR" {
                    Token::Or
                } else {
                    Token::Word(word)
                });
            }
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: std::iter::Peekable<std::vec::IntoIter<Token>>,
}

impl Parser {
    fn or(&mut self) -> Result<Expr> {
        let first = self.and()?;
        let mut clauses = vec![first];
        while self.tokens.peek() == Some(&Token::Or) {
            self.tokens.next();
            clauses.push(self.and()?);
        }
        Ok(Expr::Or(clauses))
    }

    fn and(&mut self) -> Result<Expr> {
        let mut clauses = Vec::new();
        while !matches!(self.tokens.peek(), None | Some(Token::Close | Token::Or)) {
            clauses.push(self.atom()?);
        }
        if clauses.is_empty() {
            return invalid("Missing search expression.");
        }
        Ok(Expr::And(clauses))
    }

    fn atom(&mut self) -> Result<Expr> {
        match self.tokens.next() {
            Some(Token::Minus) => Ok(Expr::Not(Box::new(self.atom()?))),
            Some(Token::Open) => {
                let expr = self.or()?;
                if self.tokens.next() != Some(Token::Close) {
                    return invalid("Missing closing parenthesis.");
                }
                Ok(expr)
            }
            Some(Token::Phrase(value)) => Ok(Expr::Phrase(value)),
            Some(Token::Word(word)) => word_expr(word),
            _ => invalid("Expected a word, phrase or group."),
        }
    }
}

fn word_expr(word: String) -> Result<Expr> {
    if word.starts_with('@') {
        return Ok(Expr::Author(normalize_author(&word)?));
    }
    if let Some((field, value)) = word.split_once(':') {
        return match field {
            "from" => Ok(Expr::Author(normalize_author(value)?)),
            "since" => Ok(Expr::Since(date(value)?)),
            "until" => Ok(Expr::Until(date(value)?)),
            _ => invalid("Supported operators: from:, since:, until:."),
        };
    }
    if word.contains(['*', '~', '^', '[', ']', '\\']) {
        return invalid("Wildcards and backend query syntax are not supported.");
    }
    Ok(Expr::Term(word))
}

fn date(value: &str) -> Result<i64> {
    if value.len() != 10 {
        return invalid("Use dates formatted YYYY-MM-DD.");
    }
    let date: jiff::civil::Date = value
        .parse()
        .map_err(|_| Error::Invalid("Invalid calendar date.".into()))?;
    let timestamp = date
        .at(0, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(timestamp.timestamp().as_millisecond())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_and_stop_words() {
        for query in [
            "the",
            "\"to be or not to be\"",
            "rust OR zig fast",
            "from:@Theo -bad",
            "(rust OR zig) search",
            "since:2026-01-01 until:2026-02-01",
        ] {
            assert!(parse(query, None).is_ok(), "{query}");
        }
        for query in [
            "",
            "()",
            "rust OR",
            "OR rust",
            "\"unclosed",
            "rust)",
            "(rust",
            "foo:bar",
            "since:2026-02-30",
            "*",
            "-",
        ] {
            assert!(parse(query, None).is_err(), "{query}");
        }
        assert_eq!(normalize_author("@TheO").unwrap(), "theo");
    }

    #[test]
    fn and_binds_before_or() {
        let Expr::Or(branches) = parse("rust fast OR zig", None).unwrap() else {
            panic!("expected OR");
        };
        assert_eq!(branches.len(), 2);
        assert_eq!(
            branches[0],
            Expr::And(vec![Expr::Term("rust".into()), Expr::Term("fast".into())])
        );
    }
}
