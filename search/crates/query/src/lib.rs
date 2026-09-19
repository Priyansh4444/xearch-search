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
/// Rejects malformed, empty or oversized queries, unsupported operators and
/// conflicting author filters (two distinct authors on one conjunctive path).
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
    let request_author = author.map(normalize_author).transpose()?;
    let Some(expression) = expression else {
        return request_author.map_or_else(
            || invalid("Enter a search."),
            |handle| Ok(Expr::Author(handle)),
        );
    };
    let analysis = analyze_authors(&expression);
    if analysis.conflict {
        return invalid("Search one author at a time, or remove the @ filters.");
    }
    match request_author {
        // An empty reachable set means the query itself filters no author,
        // so the request-level handle combines cleanly.
        Some(handle) if analysis.possible.contains(&handle) => Ok(expression),
        Some(handle) if analysis.possible.is_empty() => {
            Ok(Expr::And(vec![expression, Expr::Author(handle)]))
        }
        Some(_) => invalid("Search one author at a time, or remove the @ filters."),
        None => Ok(expression),
    }
}

/// Authors reachable on one conjunctive path, and whether any single path
/// already requires two distinct authors. An `Or` splits independent paths;
/// an `And` combines one path from every child, so two children conflict as
/// soon as their reachable sets span distinct authors. `Not` keeps the
/// filtered path, since a negated author still restricts results.
struct AuthorAnalysis {
    possible: std::collections::BTreeSet<String>,
    conflict: bool,
}

fn analyze_authors(expr: &Expr) -> AuthorAnalysis {
    let mut possible = std::collections::BTreeSet::new();
    let conflict;
    match expr {
        Expr::Author(handle) => {
            possible.insert(handle.clone());
            conflict = false;
        }
        Expr::Term(_) | Expr::Phrase(_) | Expr::Since(_) | Expr::Until(_) => conflict = false,
        Expr::Not(child) => {
            let inner = analyze_authors(child);
            possible = inner.possible;
            conflict = inner.conflict;
        }
        Expr::Or(children) => {
            let mut any_conflict = false;
            for child in children {
                let inner = analyze_authors(child);
                possible.extend(inner.possible);
                any_conflict = any_conflict || inner.conflict;
            }
            conflict = any_conflict;
        }
        Expr::And(children) => {
            let analyzed: Vec<AuthorAnalysis> = children.iter().map(analyze_authors).collect();
            let cross_conflict = analyzed.windows(2).any(|pair| match pair {
                [left, right] => {
                    !left.possible.is_empty()
                        && !right.possible.is_empty()
                        && left.possible.union(&right.possible).count() > 1
                }
                _ => false,
            });
            conflict = analyzed.iter().any(|inner| inner.conflict) || cross_conflict;
            for inner in &analyzed {
                possible.extend(inner.possible.iter().cloned());
            }
        }
    }
    AuthorAnalysis { possible, conflict }
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
        if clauses.len() == 1 {
            return Ok(clauses.swap_remove(0));
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
        if clauses.len() == 1 {
            return Ok(clauses.swap_remove(0));
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

    #[test]
    fn conflicting_authors_rejected_but_single_path_ok() {
        for query in ["from:a from:b", "@a @b", "(from:a OR from:b) from:c"] {
            assert!(parse(query, None).is_err(), "{query}");
        }
        // One author per conjunctive path is fine.
        assert!(parse("from:a OR from:b", None).is_ok());
        // Request-level author conflicts with an in-query author.
        assert!(parse("from:a", Some("b")).is_err());
        // Request author matching the in-query author is accepted once.
        assert!(parse("from:a", Some("a")).is_ok());
        // A negated author is still an author filter on the same path.
        assert!(parse("-from:a from:b", None).is_err());
    }
}
