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
