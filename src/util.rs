use crate::Matcher;
use std::io::{self, Write};

pub(crate) fn join_write_bytes<'a>(
    writer: &mut dyn Write,
    sep: &[u8],
    mut parts: impl Iterator<Item = &'a [u8]>,
) -> io::Result<()> {
    match parts.next() {
        None => {}
        Some(first) => {
            writer.write_all(first)?;

            for part in parts {
                writer.write_all(sep)?;
                writer.write_all(part)?;
            }
        }
    }

    Ok(())
}

pub(crate) fn scan_content(
    content: Option<&str>,
    kind: &str,
    blacklist: &Vec<Matcher>,
    allow: &mut bool,
    std_err: &mut dyn Write,
) -> io::Result<()> {
    match content {
        None => {}
        Some(content) => {
            for keyphrase in blacklist {
                match keyphrase {
                    Matcher::Literal(text) => {
                        if content.contains(text) {
                            writeln!(std_err, "Forbidden literal found in {}: {}", kind, text)?;

                            *allow = false;
                        }
                    }
                    Matcher::RegExp(rgx) => {
                        if rgx.find(content).is_some() {
                            writeln!(std_err, "Forbidden regex found in {}: {}", kind, rgx)?;

                            *allow = false;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Matcher;

    #[test]
    fn join_write_bytes_empty() {
        let mut buf = Vec::new();
        join_write_bytes(&mut buf, b"|", std::iter::empty::<&[u8]>()).unwrap();
        assert_eq!(buf, b"");
    }

    #[test]
    fn join_write_bytes_single() {
        let mut buf = Vec::new();
        let parts: Vec<&[u8]> = vec![b"hello"];
        join_write_bytes(&mut buf, b"|", parts.into_iter()).unwrap();
        assert_eq!(buf, b"hello");
    }

    #[test]
    fn join_write_bytes_multiple() {
        let mut buf = Vec::new();
        let parts: Vec<&[u8]> = vec![b"a", b"b", b"c"];
        join_write_bytes(&mut buf, b"|", parts.into_iter()).unwrap();
        assert_eq!(buf, b"a|b|c");
    }

    #[test]
    fn scan_content_none_is_noop() {
        let mut allow = true;
        let mut err_buf = Vec::new();
        scan_content(None, "subject", &vec![], &mut allow, &mut err_buf).unwrap();
        assert!(allow);
        assert!(err_buf.is_empty());
    }

    #[test]
    fn scan_content_literal_match_denies() {
        let blacklist = vec![Matcher::Literal("spam".to_string())];
        let mut allow = true;
        let mut err_buf = Vec::new();
        scan_content(
            Some("This is spam content"),
            "body",
            &blacklist,
            &mut allow,
            &mut err_buf,
        )
        .unwrap();
        assert!(!allow);
    }

    #[test]
    fn scan_content_literal_no_match_allows() {
        let blacklist = vec![Matcher::Literal("spam".to_string())];
        let mut allow = true;
        let mut err_buf = Vec::new();
        scan_content(
            Some("This is clean content"),
            "body",
            &blacklist,
            &mut allow,
            &mut err_buf,
        )
        .unwrap();
        assert!(allow);
    }

    #[test]
    fn scan_content_regex_match_denies() {
        let blacklist = vec![Matcher::RegExp(regex::Regex::new(r"sp[a@]m").unwrap())];
        let mut allow = true;
        let mut err_buf = Vec::new();
        scan_content(
            Some("This is sp@m content"),
            "body",
            &blacklist,
            &mut allow,
            &mut err_buf,
        )
        .unwrap();
        assert!(!allow);
    }

    #[test]
    fn scan_content_regex_no_match_allows() {
        let blacklist = vec![Matcher::RegExp(regex::Regex::new(r"sp[a@]m").unwrap())];
        let mut allow = true;
        let mut err_buf = Vec::new();
        scan_content(
            Some("This is clean content"),
            "body",
            &blacklist,
            &mut allow,
            &mut err_buf,
        )
        .unwrap();
        assert!(allow);
    }

    #[test]
    fn scan_content_empty_blacklist_always_allows() {
        let blacklist: Vec<Matcher> = vec![];
        let mut allow = true;
        let mut err_buf = Vec::new();
        scan_content(
            Some("any content at all"),
            "subject",
            &blacklist,
            &mut allow,
            &mut err_buf,
        )
        .unwrap();
        assert!(allow);
    }
}
