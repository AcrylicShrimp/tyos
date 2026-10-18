//! Command words: whitespace, quotes, and backslash escaping; no expansion.

use alloc::{string::String, vec::Vec};
use core::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnclosedQuote(char),
    TrailingEscape,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnclosedQuote(quote) => write!(f, "unclosed {quote} quote"),
            Self::TrailingEscape => f.write_str("backslash must be followed by a character"),
        }
    }
}

pub fn parse(line: &str) -> Result<Vec<String>, ParseError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote = None;
    let mut chars = line.chars();

    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some('\''), _) => word.push(ch),
            (_, '\\') => {
                word.push(chars.next().ok_or(ParseError::TrailingEscape)?);
                started = true;
            }
            (Some(_), _) => word.push(ch),
            (None, '\'' | '"') => {
                quote = Some(ch);
                started = true;
            }
            (None, _) if ch.is_whitespace() => {
                if started {
                    words.push(core::mem::take(&mut word));
                    started = false;
                }
            }
            (None, _) => {
                word.push(ch);
                started = true;
            }
        }
    }

    if let Some(quote) = quote {
        return Err(ParseError::UnclosedQuote(quote));
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn empty_and_whitespace_only_lines_have_no_command() {
        assert_eq!(parse(" \t\r\n　 ").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn separates_arguments_and_preserves_quoted_spaces() {
        assert_eq!(
            parse(" echo  hello '한 글' \"two words\" ").unwrap(),
            vec!["echo", "hello", "한 글", "two words"]
        );
    }

    #[test]
    fn empty_quotes_and_adjacent_fragments_form_words() {
        assert_eq!(
            parse("echo '' \"\" a' b'\"c\"").unwrap(),
            vec!["echo", "", "", "a bc"]
        );
    }

    #[test]
    fn backslash_escapes_except_inside_single_quotes() {
        assert_eq!(
            parse(r#"echo a\ b \" '\n' "a\"b""#).unwrap(),
            vec!["echo", "a b", "\"", "\\n", "a\"b"]
        );
    }

    #[test]
    fn rejects_unfinished_syntax_without_partial_execution() {
        assert_eq!(parse("exit 'oops"), Err(ParseError::UnclosedQuote('\'')));
        assert_eq!(parse("echo \"oops"), Err(ParseError::UnclosedQuote('"')));
        assert_eq!(parse("echo \\"), Err(ParseError::TrailingEscape));
    }

    #[test]
    fn expansion_and_operators_are_literal_text() {
        assert_eq!(
            parse("echo $HOME * a|b").unwrap(),
            vec!["echo", "$HOME", "*", "a|b"]
        );
    }
}
