// One line per interesting event, on stderr.
//
// The protocol says stdout carries nothing but responses, so everything here
// goes to stderr. The format is the same one the C++ implementation emits --
// `[dbdb]: <event> pid=<n> key=value ...` -- so the tables in the root README's
// bug catalog can be read against either implementation without changing how
// the lines are picked apart. Nothing parses them automatically; they are for a
// person reading a timeline afterwards.

use std::process;

// Keys and values are whatever the caller typed, so they get cut short and
// scrubbed before going into a trace line. A stray newline in one of them would
// split a record in two and a stray quote would make it hard to see where a
// value ended.
const PREVIEW_LIMIT: usize = 40;

pub fn quote(text: &str) -> String {
    let mut out = String::new();
    out.push('"');
    let mut taken = 0;
    for c in text.chars() {
        if taken == PREVIEW_LIMIT {
            out.push_str("...");
            break;
        }
        if c >= ' ' && c < '\u{7F}' && c != '"' {
            out.push(c);
        } else {
            out.push('.');
        }
        taken += 1;
    }
    out.push('"');
    out
}

// fields is already formatted as `name=value name=value`; callers build it with
// format!, which keeps this module from needing to know about every kind of
// value an event might carry.
pub fn emit(event: &str, fields: &str) {
    eprintln!("[dbdb]: {} pid={} {}", event, process::id(), fields);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_wraps_an_ordinary_key() {
        assert_eq!(quote("accountA"), "\"accountA\"");
    }

    #[test]
    fn quote_replaces_anything_that_would_break_a_line() {
        assert_eq!(quote("a\nb"), "\"a.b\"");
        assert_eq!(quote("a\"b"), "\"a.b\"");
    }

    #[test]
    fn quote_cuts_a_long_value_short() {
        let quoted = quote(&"x".repeat(100));
        assert_eq!(quoted.len(), PREVIEW_LIMIT + 5); // two quotes and the ellipsis
        assert!(quoted.ends_with("...\""));
    }
}
