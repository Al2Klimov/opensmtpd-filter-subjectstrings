## About

OpenSMTPd filter which rejects eMails based on configurable content patterns.

## Build

Compile like any other Rust program: `cargo build -r`

Find the resulting binary directly under `target/release/`.

## Usage

Integrate this filter into smtpd.conf(5).

### Command-line interface

```
opensmtpd-filter-contentstrings [literal|regex PATTERNS_FILE ...]
```

The binary takes any number of pattern lists as arguments.
Each one is a pair of the kind of patterns, either "literal" or "regex",
and the path to the file on the local filesystem.

### Pattern list file format

Empty lines are ignored. The others must be UTF-8.

Every non-empty line is a phrase to disallow in eMails' subject or body text.
