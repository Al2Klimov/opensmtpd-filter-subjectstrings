use crate::cnt_iter::CounterIterator;
use regex::Regex;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub(crate) enum Matcher {
    Literal(String),
    RegExp(Regex),
}

pub(crate) enum ParseArgsError {
    UnknownMatcher,
    NoFile,
    EmptyName,
    BadFile(io::Error),
    BadLine(usize, io::Error),
    BadRegex(usize, regex::Error),
}

pub(crate) fn blame_user(err: ParseArgsError, consumed: usize) {
    match err {
        ParseArgsError::UnknownMatcher => {
            eprintln!(
                "Unknown kind of pattern (CLI argument #{}), expected \"literal\"/\"regex\".",
                consumed
            );
        }
        ParseArgsError::NoFile => {
            eprintln!("Unexpected end of CLI arguments, expected file.");
        }
        ParseArgsError::EmptyName => {
            eprintln!(
                "Illegal empty string (CLI argument #{}), expected file.",
                consumed
            );
        }
        ParseArgsError::BadFile(er) => {
            eprintln!(
                "Inaccessible file (CLI argument #{}), error: {}",
                consumed, er
            );
        }
        ParseArgsError::BadLine(no, er) => {
            eprintln!(
                "File read error (CLI argument #{}, line #{}): {}",
                consumed, no, er
            );
        }
        ParseArgsError::BadRegex(no, er) => {
            eprintln!(
                "Invalid regular expression (CLI argument #{}, line #{}): {}",
                consumed, no, er
            );
        }
    }
}

pub(crate) fn parse_cmdline(
    mut args: impl Iterator<Item = OsString>,
) -> (
    Option<OsString>,
    Result<Vec<Matcher>, ParseArgsError>,
    usize,
) {
    let program = args.next();
    let mut ci = CounterIterator::new(args);

    (program, parse_args(&mut ci), ci.taken())
}

fn parse_args(args: &mut dyn Iterator<Item = OsString>) -> Result<Vec<Matcher>, ParseArgsError> {
    let mut blacklist = Vec::new();
    loop {
        match args.next() {
            None => return Ok(blacklist),
            Some(matcher) => match matcher.to_string_lossy().as_ref() {
                "literal" => require_lines(args.next(), |line, _| {
                    blacklist.push(Matcher::Literal(line));
                    Ok(())
                })?,
                "regex" => require_lines(args.next(), |line, no| {
                    blacklist.push(Matcher::RegExp(
                        Regex::new(line.as_str())
                            .map_err(|err| ParseArgsError::BadRegex(no, err))?,
                    ));
                    Ok(())
                })?,
                _ => return Err(ParseArgsError::UnknownMatcher),
            },
        }
    }
}

fn require_lines(
    oarg: Option<OsString>,
    mut on_line: impl FnMut(String, usize) -> Result<(), ParseArgsError>,
) -> Result<(), ParseArgsError> {
    let name = oarg.ok_or(ParseArgsError::NoFile)?;
    if name.is_empty() {
        return Err(ParseArgsError::EmptyName);
    }

    let mut ci = CounterIterator::new(
        BufReader::new(File::open(name).map_err(|err| ParseArgsError::BadFile(err))?).lines(),
    );
    loop {
        match ci.next() {
            None => return Ok(()),
            Some(Err(err)) => return Err(ParseArgsError::BadLine(ci.taken(), err)),
            Some(Ok(line)) => {
                if !line.is_empty() {
                    on_line(line, ci.taken())?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn args(v: &[&str]) -> impl Iterator<Item = OsString> {
        v.iter()
            .map(|s| OsString::from(s))
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn no_args_yields_empty_blacklist() {
        let (_, result, consumed) = parse_cmdline(args(&["prog"]));
        assert_eq!(consumed, 0);
        let matchers = result.ok().expect("expected Ok result");
        assert_eq!(matchers.len(), 0);
    }

    #[test]
    fn unknown_matcher_returns_error() {
        let (_, result, _) = parse_cmdline(args(&["prog", "unknown"]));
        assert!(matches!(result, Err(ParseArgsError::UnknownMatcher)));
    }

    #[test]
    fn missing_file_after_literal_returns_error() {
        let (_, result, _) = parse_cmdline(args(&["prog", "literal"]));
        assert!(matches!(result, Err(ParseArgsError::NoFile)));
    }

    #[test]
    fn missing_file_after_regex_returns_error() {
        let (_, result, _) = parse_cmdline(args(&["prog", "regex"]));
        assert!(matches!(result, Err(ParseArgsError::NoFile)));
    }

    #[test]
    fn empty_file_arg_returns_error() {
        let (_, result, _) = parse_cmdline(args(&["prog", "literal", ""]));
        assert!(matches!(result, Err(ParseArgsError::EmptyName)));
    }

    #[test]
    fn nonexistent_file_returns_bad_file() {
        let (_, result, _) =
            parse_cmdline(args(&["prog", "literal", "/nonexistent/path/file.txt"]));
        assert!(matches!(result, Err(ParseArgsError::BadFile(_))));
    }

    #[test]
    fn literal_file_loads_matchers() {
        let path = std::env::temp_dir().join("filter_literal_matchers.txt");
        fs::write(&path, "spam\nphishing\n").unwrap();
        let (_, result, _) = parse_cmdline(args(&["prog", "literal", path.to_str().unwrap()]));
        let matchers = result.ok().expect("expected Ok result");
        assert_eq!(matchers.len(), 2);
        assert!(matches!(&matchers[0], Matcher::Literal(s) if s == "spam"));
        assert!(matches!(&matchers[1], Matcher::Literal(s) if s == "phishing"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn regex_file_loads_matchers() {
        let path = std::env::temp_dir().join("filter_regex_matchers.txt");
        fs::write(&path, r"sp[a@]m").unwrap();
        let (_, result, _) = parse_cmdline(args(&["prog", "regex", path.to_str().unwrap()]));
        let matchers = result.ok().expect("expected Ok result");
        assert_eq!(matchers.len(), 1);
        assert!(matches!(&matchers[0], Matcher::RegExp(_)));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn invalid_regex_returns_bad_regex() {
        let path = std::env::temp_dir().join("filter_invalid_regex.txt");
        fs::write(&path, "[invalid").unwrap();
        let (_, result, _) = parse_cmdline(args(&["prog", "regex", path.to_str().unwrap()]));
        assert!(matches!(result, Err(ParseArgsError::BadRegex(_, _))));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn empty_lines_in_file_are_skipped() {
        let path = std::env::temp_dir().join("filter_empty_lines.txt");
        fs::write(&path, "\nspam\n\nphishing\n\n").unwrap();
        let (_, result, _) = parse_cmdline(args(&["prog", "literal", path.to_str().unwrap()]));
        let matchers = result.ok().expect("expected Ok result");
        assert_eq!(matchers.len(), 2);
        fs::remove_file(&path).ok();
    }
}
