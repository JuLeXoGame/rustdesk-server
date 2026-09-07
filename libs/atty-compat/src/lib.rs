//! Compatibility API for transitive atty users, implemented by the Rust standard library.
#![forbid(unsafe_code)]

use std::io::{self, IsTerminal};

#[derive(Clone, Copy, Debug)]
pub enum Stream {
    Stdout,
    Stderr,
    Stdin,
}

pub fn is(stream: Stream) -> bool {
    match stream {
        Stream::Stdout => io::stdout().is_terminal(),
        Stream::Stderr => io::stderr().is_terminal(),
        Stream::Stdin => io::stdin().is_terminal(),
    }
}

pub fn isnt(stream: Stream) -> bool {
    !is(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_standard_library() {
        for (stream, expected) in [
            (Stream::Stdin, io::stdin().is_terminal()),
            (Stream::Stdout, io::stdout().is_terminal()),
            (Stream::Stderr, io::stderr().is_terminal()),
        ] {
            assert_eq!(is(stream), expected);
            assert_eq!(isnt(stream), !expected);
        }
    }

    #[test]
    fn redirected_streams_are_not_terminals() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::redirected_child", "--nocapture"])
            .env("RELAISDESK_ATTY_TEST_CHILD", "1")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }

    #[test]
    fn redirected_child() {
        if std::env::var_os("RELAISDESK_ATTY_TEST_CHILD").is_some() {
            assert!(isnt(Stream::Stdin));
            assert!(isnt(Stream::Stdout));
            assert!(isnt(Stream::Stderr));
        }
    }
}
