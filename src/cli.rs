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
