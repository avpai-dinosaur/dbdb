// A connection to a database, and the transaction rules that go with it.
//
// The rules are short:
//
//   * A read outside a transaction looks at whatever has been committed by the
//     time it runs.
//   * The first write of a session takes the file lock and keeps it. Writers
//     are therefore serialized against each other; readers never wait.
//   * Nothing a writer does is visible to anybody else until commit moves the
//     root pointer, and commit is also what releases the lock.
//
// A session that goes away without committing -- `exit`, end of input, or the
// process being killed -- loses its writes, because they were never anywhere
// but in its own memory, and drops its lock when its file descriptor closes.

use crate::storage::FileStorage;
use crate::trace;
use crate::tree::BinaryTree;

pub struct Dbdb {
    storage: FileStorage,
    tree: BinaryTree,
    // Whether this session is in the middle of a transaction. The lock and the
    // transaction are the same thing: there is no separate "begin".
    lock_held: bool,
    commit_sequence: u64,
}

impl Dbdb {
    pub fn connect(path: &str) -> Result<Dbdb, String> {
        let mut storage = FileStorage::open(path)?;
        let root = storage.get_root_address()?;
        Ok(Dbdb {
            storage,
            tree: BinaryTree::new(root),
            lock_held: false,
            commit_sequence: 0,
        })
    }

    pub fn get(&mut self, key: &str) -> Result<Option<String>, String> {
        // Outside a transaction, catch up to the latest commit first. Inside
        // one, keep reading the tree this session has been building, so a
        // writer sees its own uncommitted writes and nobody else's.
        if !self.lock_held {
            self.tree.refresh(&mut self.storage, "get_unlocked")?;
        }
        self.tree.get(&mut self.storage, key)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.begin_write("set_locked")?;
        self.tree.set(&mut self.storage, key, value)
    }

    // Deleting a key that is not there still opens a transaction and still
    // succeeds. The protocol asks for that: a caller retrying after a fault
    // must not be told its retry was wrong.
    pub fn remove(&mut self, key: &str) -> Result<(), String> {
        self.begin_write("remove_locked")?;
        self.tree.remove(&mut self.storage, key)
    }

    pub fn commit(&mut self) -> Result<(), String> {
        self.commit_sequence += 1;
        trace::emit("commit_begin", &format!("seq={}", self.commit_sequence));

        // Deliberately before the unlock. If writing the tree out fails, the
        // lock stays held and this session is still in its transaction, so it
        // can try again or give up by exiting -- what it must not do is let
        // somebody else in on top of a half-written commit.
        self.tree.commit(&mut self.storage)?;

        self.storage.unlock()?;
        self.lock_held = false;
        trace::emit("commit_end", &format!("seq={}", self.commit_sequence));
        Ok(())
    }

    // Starts a transaction if one is not already open. reason is only there to
    // say in the trace which command opened it.
    fn begin_write(&mut self, reason: &str) -> Result<(), String> {
        if self.lock_held {
            return Ok(());
        }
        // Blocks until whoever else is writing has committed or gone away.
        self.storage.lock()?;
        self.lock_held = true;
        // Taking the lock is the moment this session gets a fixed view to build
        // on, so the view is refreshed here and not again until commit.
        self.tree.refresh(&mut self.storage, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::temp_path;

    #[test]
    fn a_committed_write_is_visible_to_the_next_connection() {
        let path = temp_path("db-commit");
        {
            let mut db = Dbdb::connect(&path).unwrap();
            db.set("accountA", "full").unwrap();
            db.commit().unwrap();
        }

        let mut db = Dbdb::connect(&path).unwrap();
        assert_eq!(db.get("accountA").unwrap(), Some("full".to_string()));
    }

    #[test]
    fn an_uncommitted_write_is_not() {
        let path = temp_path("db-uncommitted");
        {
            let mut db = Dbdb::connect(&path).unwrap();
            db.set("accountA", "full").unwrap();
        }

        let mut db = Dbdb::connect(&path).unwrap();
        assert_eq!(db.get("accountA").unwrap(), None);
    }

    #[test]
    fn a_writer_can_read_its_own_uncommitted_writes() {
        let path = temp_path("db-own-writes");
        let mut db = Dbdb::connect(&path).unwrap();
        db.set("accountA", "full").unwrap();
        assert_eq!(db.get("accountA").unwrap(), Some("full".to_string()));
    }

    #[test]
    fn both_writes_of_a_transaction_land_together() {
        let path = temp_path("db-atomic");
        {
            let mut db = Dbdb::connect(&path).unwrap();
            db.set("accountA", "full").unwrap();
            db.set("accountB", "empty").unwrap();
            db.commit().unwrap();
        }

        let mut db = Dbdb::connect(&path).unwrap();
        assert_eq!(db.get("accountA").unwrap(), Some("full".to_string()));
        assert_eq!(db.get("accountB").unwrap(), Some("empty".to_string()));
    }

    #[test]
    fn deleting_a_missing_key_succeeds() {
        let path = temp_path("db-delete-missing");
        let mut db = Dbdb::connect(&path).unwrap();
        assert!(db.remove("never set").is_ok());
        db.commit().unwrap();
    }

    #[test]
    fn a_deleted_key_stays_deleted() {
        let path = temp_path("db-delete");
        let mut db = Dbdb::connect(&path).unwrap();
        db.set("accountA", "full").unwrap();
        db.commit().unwrap();
        db.remove("accountA").unwrap();
        db.commit().unwrap();
        assert_eq!(db.get("accountA").unwrap(), None);
    }
}
