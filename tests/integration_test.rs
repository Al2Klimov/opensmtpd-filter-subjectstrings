use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn filter_cmd(extra_args: &[&str]) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_opensmtpd-filter-contentstrings"))
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn binary")
}

fn run_filter(extra_args: &[&str], input: &[u8]) -> (String, String) {
    let mut child = filter_cmd(extra_args);
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input)
        .expect("Failed to write stdin");
    let output = child.wait_with_output().expect("Failed to wait for child");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

/// Build the OpenSMTPD protocol lines for a complete mail transaction.
fn make_session_input(session: &str, token: &str, mail_lines: &[&str]) -> Vec<u8> {
    let mut input = Vec::new();
    writeln!(input, "config|ready").unwrap();
    writeln!(input, "report|1|1000|smtp-in|tx-begin|{}", session).unwrap();
    for line in mail_lines {
        writeln!(
            input,
            "filter|1|1000|smtp-in|data-line|{}|{}|{}",
            session, token, line
        )
        .unwrap();
    }
    // end-of-data marker
    writeln!(
        input,
        "filter|1|1000|smtp-in|data-line|{}|{}|.",
        session, token
    )
    .unwrap();
    writeln!(input, "filter|1|1000|smtp-in|commit|{}|{}", session, token).unwrap();
    input
}

#[test]
fn config_ready_registers_correctly() {
    let (stdout, _) = run_filter(&[], b"config|ready\n");
    assert!(stdout.contains("register|report|smtp-in|tx-begin\n"));
    assert!(stdout.contains("register|filter|smtp-in|data-line\n"));
    assert!(stdout.contains("register|filter|smtp-in|commit\n"));
    assert!(stdout.contains("register|report|smtp-in|link-disconnect\n"));
    assert!(stdout.contains("register|ready\n"));
}

#[test]
fn clean_mail_is_allowed_with_empty_blacklist() {
    let input = make_session_input(
        "sess1",
        "tok1",
        &[
            "From: sender@example.com",
            "Subject: Hello",
            "",
            "This is a clean message.",
        ],
    );
    let (stdout, _) = run_filter(&[], &input);
    assert!(stdout.contains("filter-result|sess1|tok1|proceed\n"));
}

#[test]
fn mail_with_blacklisted_literal_in_subject_is_rejected() {
    let path = std::env::temp_dir().join("filter_literal_subject.txt");
    fs::write(&path, "badword\n").unwrap();

    let input = make_session_input(
        "sess2",
        "tok2",
        &[
            "From: sender@example.com",
            "Subject: This contains badword here",
            "",
            "Normal body.",
        ],
    );
    let (stdout, _) = run_filter(&["literal", path.to_str().unwrap()], &input);
    assert!(stdout.contains("filter-result|sess2|tok2|reject|550 Blacklisted keyphrase found\n"));
    fs::remove_file(&path).ok();
}

#[test]
fn mail_with_blacklisted_literal_in_body_is_rejected() {
    let path = std::env::temp_dir().join("filter_literal_body.txt");
    fs::write(&path, "forbidden\n").unwrap();

    let input = make_session_input(
        "sess3",
        "tok3",
        &[
            "From: sender@example.com",
            "Subject: Normal subject",
            "",
            "This message contains forbidden content.",
        ],
    );
    let (stdout, _) = run_filter(&["literal", path.to_str().unwrap()], &input);
    assert!(stdout.contains("filter-result|sess3|tok3|reject|550 Blacklisted keyphrase found\n"));
    fs::remove_file(&path).ok();
}

#[test]
fn mail_without_blacklisted_literal_is_allowed() {
    let path = std::env::temp_dir().join("filter_no_match_literal.txt");
    fs::write(&path, "forbidden\n").unwrap();

    let input = make_session_input(
        "sess4",
        "tok4",
        &[
            "From: sender@example.com",
            "Subject: Normal subject",
            "",
            "This message is totally fine.",
        ],
    );
    let (stdout, _) = run_filter(&["literal", path.to_str().unwrap()], &input);
    assert!(stdout.contains("filter-result|sess4|tok4|proceed\n"));
    fs::remove_file(&path).ok();
}

#[test]
fn mail_matching_regex_blacklist_is_rejected() {
    let path = std::env::temp_dir().join("filter_regex_blacklist.txt");
    fs::write(&path, r"sp[a@]m").unwrap();

    let input = make_session_input(
        "sess5",
        "tok5",
        &[
            "From: sender@example.com",
            "Subject: Normal",
            "",
            "Buy our sp@m product today!",
        ],
    );
    let (stdout, _) = run_filter(&["regex", path.to_str().unwrap()], &input);
    assert!(stdout.contains("filter-result|sess5|tok5|reject|550 Blacklisted keyphrase found\n"));
    fs::remove_file(&path).ok();
}

#[test]
fn data_lines_are_echoed_back() {
    let input = make_session_input(
        "sess6",
        "tok6",
        &[
            "From: sender@example.com",
            "Subject: Echo test",
            "",
            "Body.",
        ],
    );
    let (stdout, _) = run_filter(&[], &input);
    assert!(stdout.contains("filter-dataline|sess6|tok6|From: sender@example.com\n"));
    assert!(stdout.contains("filter-dataline|sess6|tok6|Subject: Echo test\n"));
    assert!(stdout.contains("filter-dataline|sess6|tok6|.\n"));
}

#[test]
fn link_disconnect_removes_session() {
    // After link-disconnect the session should be cleaned up (no crash, no leftover data).
    let mut input = Vec::new();
    writeln!(input, "config|ready").unwrap();
    writeln!(input, "report|1|1000|smtp-in|tx-begin|sess7").unwrap();
    writeln!(
        input,
        "filter|1|1000|smtp-in|data-line|sess7|tok7|From: x@x.com"
    )
    .unwrap();
    writeln!(input, "report|1|1000|smtp-in|link-disconnect|sess7").unwrap();
    // commit after disconnect: session is gone, filter should still produce a result
    writeln!(input, "filter|1|1000|smtp-in|commit|sess7|tok7").unwrap();

    let (stdout, _) = run_filter(&[], &input);
    // After disconnect the session buffer is gone; commit treats missing session as allow
    assert!(stdout.contains("filter-result|sess7|tok7|proceed\n"));
}
