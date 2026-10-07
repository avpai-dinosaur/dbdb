// The dbdb command line.
//
// Two ways in. One operation per invocation:
//
//   dbdb DBNAME get KEY
//   dbdb DBNAME set KEY VALUE
//   dbdb DBNAME delete KEY
//
// each of which is a whole transaction by itself -- connect, do the thing,
// commit, exit -- and a session on stdin:
//
//   dbdb DBNAME [repl]
//
// where set and delete do NOT commit. The first one opens a transaction that is
// held until an explicit `commit`, which is what makes several writes land
// together. Reaching `exit` or the end of input throws away anything
// uncommitted and releases the lock.
//
// ../antithesis/CONTRACT.md is the normative description of the session
// protocol, and it is what the test harness drives. The short version: one
// command per line in, exactly one response line out, flushed straight away,
// and nothing but responses on stdout.

mod bytes;
mod db;
mod storage;
#[cfg(test)]
mod testing;
mod trace;
mod tree;

use std::io;
use std::io::BufRead;
use std::io::StdoutLock;
use std::io::Write;
use std::process;

use db::Dbdb;

const EXIT_OK: i32 = 0;
const EXIT_USAGE: i32 = 1;
// A `get` for a key that is not there is not a failure, but a shell script
// wants to be able to tell it apart from one that is, so it gets its own code.
// The session protocol says NOTFOUND instead and does not care what this is.
const EXIT_NOT_FOUND: i32 = 2;
const EXIT_ERROR: i32 = 3;

fn usage() -> i32 {
    eprintln!("usage:");
    eprintln!("  dbdb DBNAME get KEY");
    eprintln!("  dbdb DBNAME set KEY VALUE");
    eprintln!("  dbdb DBNAME delete KEY");
    eprintln!("  dbdb DBNAME [repl]      read commands from stdin");
    EXIT_USAGE
}

// Splits off the next space-delimited token and moves idx past it. Returns an
// empty string once the line is used up.
//
// This walks the bytes of the line rather than its characters, which is safe
// because the only thing it ever splits on is an ASCII space, and a space byte
// can never turn up in the middle of a multi-byte character in UTF-8.
fn next_token(line: &str, idx: &mut usize) -> String {
    let raw = line.as_bytes();
    while *idx < raw.len() && raw[*idx] == b' ' {
        *idx += 1;
    }
    let start = *idx;
    while *idx < raw.len() && raw[*idx] != b' ' {
        *idx += 1;
    }
    line[start..*idx].to_string()
}

// Whatever is left of the line, minus the spaces separating it from the token
// before it. Unlike next_token this keeps spaces, so a value can contain them.
fn rest_of_line(line: &str, idx: &mut usize) -> String {
    let raw = line.as_bytes();
    while *idx < raw.len() && raw[*idx] == b' ' {
        *idx += 1;
    }
    line[*idx..].to_string()
}

// A response is exactly one line, so an error message that happened to contain
// a newline would leave the caller reading the second half as if it were the
// answer to its next command.
fn one_line(text: &str) -> String {
    text.replace('\n', " ").replace('\r', " ")
}

// Writes one response line and flushes it. The caller writes a command and then
// blocks waiting for this, so a response sitting in a buffer is not a slow
// answer, it is a deadlock. Returns false if stdout has gone away, which is the
// session's cue to stop.
fn respond(out: &mut StdoutLock, text: &str) -> bool {
    if writeln!(out, "{}", text).is_err() {
        return false;
    }
    out.flush().is_ok()
}

fn repl(db: &mut Dbdb) -> i32 {
    let mut input = io::stdin().lock();
    let mut out = io::stdout().lock();
    let mut line = String::new();

    loop {
        line.clear();
        let read = match input.read_line(&mut line) {
            Ok(read) => read,
            Err(e) => {
                // Nothing here can fix a stdin that will not read -- trying
                // again would spin on the same bad byte forever -- so this is
                // treated the same as the input ending.
                eprintln!("dbdb: could not read a command: {}", e);
                return EXIT_OK;
            }
        };
        if read == 0 {
            return EXIT_OK; // end of input behaves like exit
        }

        let command = line.trim_end_matches('\n').trim_end_matches('\r');
        let mut idx = 0;
        let verb = next_token(command, &mut idx);
        if verb.is_empty() {
            continue; // blank line
        }
        if verb == "exit" || verb == "quit" {
            return EXIT_OK;
        }

        // Every arm produces the one line that answers this command. A database
        // error turns into ERR rather than ending the session: the process
        // stays up and the next command is still read, because dying here would
        // look like a crash to whatever is supervising this process. A failure
        // part-way through a transaction leaves the lock held, so the caller can
        // still retry, commit, or exit.
        let response = match verb.as_str() {
            "get" => {
                let key = next_token(command, &mut idx);
                if key.is_empty() {
                    "ERR get requires a key".to_string()
                } else {
                    match db.get(&key) {
                        Ok(Some(value)) => format!("VALUE {}", value),
                        Ok(None) => "NOTFOUND".to_string(),
                        Err(e) => format!("ERR {}", one_line(&e)),
                    }
                }
            }
            "set" => {
                let key = next_token(command, &mut idx);
                if key.is_empty() || idx >= command.len() {
                    "ERR set requires a key and a value".to_string()
                } else {
                    let value = rest_of_line(command, &mut idx);
                    match db.set(&key, &value) {
                        Ok(()) => "OK".to_string(),
                        Err(e) => format!("ERR {}", one_line(&e)),
                    }
                }
            }
            "delete" => {
                let key = next_token(command, &mut idx);
                if key.is_empty() {
                    "ERR delete requires a key".to_string()
                } else {
                    match db.remove(&key) {
                        Ok(()) => "OK".to_string(),
                        Err(e) => format!("ERR {}", one_line(&e)),
                    }
                }
            }
            "commit" => match db.commit() {
                Ok(()) => "OK".to_string(),
                Err(e) => format!("ERR {}", one_line(&e)),
            },
            other => format!("ERR unknown command: {}", one_line(other)),
        };

        if !respond(&mut out, &response) {
            return EXIT_OK;
        }
    }
}

fn run(args: &[String]) -> i32 {
    if args.len() < 2 {
        return usage();
    }
    let dbname = &args[1];

    let mut db = match Dbdb::connect(dbname) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("dbdb: {}", e);
            return EXIT_ERROR;
        }
    };

    if args.len() == 2 || (args.len() == 3 && args[2] == "repl") {
        return repl(&mut db);
    }
    if args.len() < 4 {
        return usage();
    }

    let verb = &args[2];
    let key = &args[3];

    // One operation, then commit: each of these is a complete transaction, so
    // there is nothing for the caller to commit afterwards.
    let outcome = if verb == "get" && args.len() == 4 {
        match db.get(key) {
            Ok(Some(value)) => {
                println!("{}", value);
                Ok(EXIT_OK)
            }
            Ok(None) => {
                eprintln!("key not found: {}", key);
                Ok(EXIT_NOT_FOUND)
            }
            Err(e) => Err(e),
        }
    } else if verb == "set" && args.len() == 5 {
        match db.set(key, &args[4]) {
            Ok(()) => db.commit().map(|_| EXIT_OK),
            Err(e) => Err(e),
        }
    } else if verb == "delete" && args.len() == 4 {
        match db.remove(key) {
            Ok(()) => db.commit().map(|_| EXIT_OK),
            Err(e) => Err(e),
        }
    } else {
        return usage();
    };

    match outcome {
        Ok(code) => code,
        Err(e) => {
            eprintln!("dbdb: {}", e);
            EXIT_ERROR
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // run owns the connection, so the file is closed and the lock released
    // before exit -- process::exit does not run destructors.
    let code = run(&args);
    process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_splits_into_a_verb_and_a_key() {
        let line = "set accountA full";
        let mut idx = 0;
        assert_eq!(next_token(line, &mut idx), "set");
        assert_eq!(next_token(line, &mut idx), "accountA");
        assert_eq!(rest_of_line(line, &mut idx), "full");
    }

    #[test]
    fn a_value_keeps_its_spaces() {
        let line = "set key  a value   with gaps";
        let mut idx = 0;
        assert_eq!(next_token(line, &mut idx), "set");
        assert_eq!(next_token(line, &mut idx), "key");
        assert_eq!(rest_of_line(line, &mut idx), "a value   with gaps");
    }

    #[test]
    fn an_empty_line_has_no_verb() {
        let mut idx = 0;
        assert_eq!(next_token("", &mut idx), "");
        let mut idx = 0;
        assert_eq!(next_token("    ", &mut idx), "");
    }

    #[test]
    fn a_multibyte_value_is_not_cut_in_half() {
        let line = "set key héllo wörld";
        let mut idx = 0;
        assert_eq!(next_token(line, &mut idx), "set");
        assert_eq!(next_token(line, &mut idx), "key");
        assert_eq!(rest_of_line(line, &mut idx), "héllo wörld");
    }

    #[test]
    fn an_error_message_never_spans_two_lines() {
        assert_eq!(one_line("broke\nin two"), "broke in two");
        assert_eq!(one_line("broke\r\nin two"), "broke  in two");
    }
}
