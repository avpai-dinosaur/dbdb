// Drives the built binary the way the Antithesis harness does, over the line
// protocol in ../../antithesis/CONTRACT.md.
//
// The unit tests check that the database is right. These check that it is
// reachable: one response line per command, flushed straight away, nothing but
// responses on stdout, and the transaction boundaries where the contract says
// they are. Everything here goes through a pipe to a real process, because that
// is all the harness ever sees.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::process::Child;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_path(name: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(format!(
        "dbdb-rust-protocol-{}-{}-{}.db",
        name,
        process::id(),
        n
    ));
    let _ = std::fs::remove_file(&path);
    path.to_string_lossy().to_string()
}

struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Session {
    fn start(path: &str) -> Session {
        // CARGO_BIN_EXE_dbdb is the binary cargo just built, so these tests can
        // never accidentally run some older copy sitting on PATH.
        let mut child = Command::new(env!("CARGO_BIN_EXE_dbdb"))
            .arg(path)
            .arg("repl")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("could not start dbdb");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Session {
            child,
            stdin,
            stdout,
        }
    }

    // Sends one command and waits for its one response. If the database only
    // flushed its answer when its buffer filled up, this would hang here, which
    // is the point: that is exactly what would happen to the harness.
    fn command(&mut self, line: &str) -> String {
        writeln!(self.stdin, "{}", line).expect("could not send a command");
        self.stdin.flush().expect("could not flush a command");

        let mut response = String::new();
        let read = self
            .stdout
            .read_line(&mut response)
            .expect("could not read a response");
        assert!(
            read > 0,
            "dbdb closed stdout instead of answering {:?}",
            line
        );
        response.trim_end_matches('\n').to_string()
    }

    fn exit(mut self) {
        writeln!(self.stdin, "exit").expect("could not send exit");
        self.stdin.flush().unwrap();
        drop(self.stdin);
        let status = self.child.wait().expect("dbdb never exited");
        assert_eq!(status.code(), Some(0), "exit should leave status 0");
    }
}

fn one_shot(path: &str, args: &[&str]) -> process::Output {
    Command::new(env!("CARGO_BIN_EXE_dbdb"))
        .arg(path)
        .args(args)
        .output()
        .expect("could not run dbdb")
}

#[test]
fn a_committed_value_can_be_read_back_by_a_later_session() {
    let path = temp_path("roundtrip");

    let mut first = Session::start(&path);
    assert_eq!(first.command("set accountA full"), "OK");
    assert_eq!(first.command("commit"), "OK");
    first.exit();

    let mut second = Session::start(&path);
    assert_eq!(second.command("get accountA"), "VALUE full");
    second.exit();
}

#[test]
fn a_key_that_was_never_set_answers_notfound() {
    let path = temp_path("notfound");
    let mut db = Session::start(&path);
    assert_eq!(db.command("get accountA"), "NOTFOUND");
    db.exit();
}

// Both sessions are connected, and proved connected, before either one starts
// writing. Connecting takes the lock for a moment of its own -- that is how a
// brand new file gets its root pointer written exactly once -- so a session
// started in the middle of somebody else's transaction would sit waiting for it
// rather than reading anything. A command that comes back is the proof: the
// process cannot answer until it has finished connecting.
fn connected_pair(path: &str) -> (Session, Session) {
    let mut writer = Session::start(path);
    let mut reader = Session::start(path);
    assert_eq!(writer.command("get unset"), "NOTFOUND");
    assert_eq!(reader.command("get unset"), "NOTFOUND");
    (writer, reader)
}

#[test]
fn writes_are_not_visible_until_commit() {
    let path = temp_path("isolation");
    let (mut writer, mut reader) = connected_pair(&path);

    assert_eq!(writer.command("set accountA full"), "OK");

    // The writer sees its own write straight away.
    assert_eq!(writer.command("get accountA"), "VALUE full");

    // Nobody else does, because the root pointer has not moved.
    assert_eq!(reader.command("get accountA"), "NOTFOUND");

    assert_eq!(writer.command("commit"), "OK");

    // And now everybody does.
    assert_eq!(reader.command("get accountA"), "VALUE full");
    reader.exit();
    writer.exit();
}

#[test]
fn exiting_throws_away_uncommitted_writes() {
    let path = temp_path("discard");

    let mut abandoned = Session::start(&path);
    assert_eq!(abandoned.command("set accountA full"), "OK");
    abandoned.exit(); // no commit

    let mut db = Session::start(&path);
    assert_eq!(db.command("get accountA"), "NOTFOUND");
    db.exit();
}

#[test]
fn several_writes_land_together() {
    let path = temp_path("atomic");
    let (mut writer, mut reader) = connected_pair(&path);

    assert_eq!(writer.command("set accountA full"), "OK");
    assert_eq!(writer.command("set accountB empty"), "OK");

    // Half a transaction is never visible: not one account, not the other.
    assert_eq!(reader.command("get accountA"), "NOTFOUND");
    assert_eq!(reader.command("get accountB"), "NOTFOUND");

    assert_eq!(writer.command("commit"), "OK");
    writer.exit();

    assert_eq!(reader.command("get accountA"), "VALUE full");
    assert_eq!(reader.command("get accountB"), "VALUE empty");
    reader.exit();
}

#[test]
fn deleting_a_key_that_is_not_there_is_still_ok() {
    let path = temp_path("delete-missing");
    let mut db = Session::start(&path);

    // Deletion has to be idempotent: a workload retrying after a fault would
    // otherwise see an error for work that had already succeeded.
    assert_eq!(db.command("delete accountA"), "OK");
    assert_eq!(db.command("delete accountA"), "OK");
    assert_eq!(db.command("commit"), "OK");
    assert_eq!(db.command("get accountA"), "NOTFOUND");
    db.exit();
}

#[test]
fn a_deleted_key_is_gone_for_later_sessions_too() {
    let path = temp_path("delete");

    let mut db = Session::start(&path);
    assert_eq!(db.command("set accountA full"), "OK");
    assert_eq!(db.command("set accountB empty"), "OK");
    assert_eq!(db.command("commit"), "OK");
    assert_eq!(db.command("delete accountA"), "OK");
    assert_eq!(db.command("commit"), "OK");
    db.exit();

    let mut after = Session::start(&path);
    assert_eq!(after.command("get accountA"), "NOTFOUND");
    assert_eq!(after.command("get accountB"), "VALUE empty");
    after.exit();
}

#[test]
fn a_value_may_contain_spaces() {
    let path = temp_path("spaces");
    let mut db = Session::start(&path);

    assert_eq!(db.command("set accountA a value with spaces"), "OK");
    assert_eq!(db.command("commit"), "OK");
    assert_eq!(db.command("get accountA"), "VALUE a value with spaces");
    db.exit();
}

#[test]
fn setting_a_key_twice_keeps_the_second_value() {
    let path = temp_path("overwrite");
    let mut db = Session::start(&path);

    assert_eq!(db.command("set accountA full"), "OK");
    assert_eq!(db.command("commit"), "OK");
    assert_eq!(db.command("set accountA empty"), "OK");
    assert_eq!(db.command("commit"), "OK");
    assert_eq!(db.command("get accountA"), "VALUE empty");
    db.exit();
}

#[test]
fn a_bad_command_is_answered_and_not_fatal() {
    let path = temp_path("errors");
    let mut db = Session::start(&path);

    assert!(db.command("nonsense").starts_with("ERR "));
    assert!(db.command("get").starts_with("ERR "));
    assert!(db.command("set keyonly").starts_with("ERR "));
    assert!(db.command("delete").starts_with("ERR "));

    // The point of all four: the session is still usable afterwards.
    assert_eq!(db.command("set accountA full"), "OK");
    assert_eq!(db.command("commit"), "OK");
    assert_eq!(db.command("get accountA"), "VALUE full");
    db.exit();
}

#[test]
fn every_command_gets_one_reply_in_order() {
    let path = temp_path("ordering");
    let mut db = Session::start(&path);

    // If any command answered with two lines, or none, the responses would
    // shift and these would not line up.
    let commands = [
        ("set a 1", "OK"),
        ("set b 2", "OK"),
        ("commit", "OK"),
        ("get a", "VALUE 1"),
        ("get b", "VALUE 2"),
        ("get c", "NOTFOUND"),
        ("delete a", "OK"),
        ("commit", "OK"),
        ("get a", "NOTFOUND"),
        ("get b", "VALUE 2"),
    ];
    for (command, expected) in commands {
        assert_eq!(db.command(command), expected, "command was {:?}", command);
    }
    db.exit();
}

#[test]
fn the_one_shot_form_commits_by_itself() {
    let path = temp_path("one-shot");

    let set = one_shot(&path, &["set", "accountA", "full"]);
    assert!(set.status.success());

    let get = one_shot(&path, &["get", "accountA"]);
    assert!(get.status.success());
    assert_eq!(String::from_utf8_lossy(&get.stdout).trim_end(), "full");

    let missing = one_shot(&path, &["get", "accountB"]);
    assert_eq!(missing.status.code(), Some(2));

    let delete = one_shot(&path, &["delete", "accountA"]);
    assert!(delete.status.success());
    assert_eq!(one_shot(&path, &["get", "accountA"]).status.code(), Some(2));
}
