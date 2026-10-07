// The file under the database.
//
// It is append-only: a write always goes to the end and hands back the offset
// it landed at, and nothing already written is ever overwritten. The one
// exception is the first eight bytes, the root pointer, which say where the
// current tree starts. Updating that pointer is what "commit" means -- until it
// moves, everything appended after it is unreachable, so a crash part-way
// through a commit leaves the previous tree exactly as it was.

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use crate::bytes;
use crate::trace;

pub type Address = u64;

// Offset 0 is the root pointer, so no record can ever begin there. That makes 0
// usable as "there is no node here" without needing a separate flag.
pub const NULL_ADDRESS: Address = 0;

pub struct FileStorage {
    file: File,
    // The root pointer as of the last time it was read or written. Only used
    // for tracing; get_root_address is what anybody who needs the real value
    // calls.
    root: Address,
}

impl FileStorage {
    pub fn open(path: &str) -> Result<FileStorage, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(|e| format!("could not open {}: {}", path, e))?;

        let mut storage = FileStorage {
            file,
            root: NULL_ADDRESS,
        };

        // Two processes can reach a brand new file at the same moment, so the
        // root pointer has to be written under the lock. Whichever one gets
        // there second sees a file that is already eight bytes long and leaves
        // it alone.
        storage.lock()?;
        let length = storage.length()?;
        if length == 0 {
            storage.commit_root_address(NULL_ADDRESS)?;
        }
        storage.unlock()?;

        Ok(storage)
    }

    // Blocks until no other process holds the lock. Only writers take it:
    // readers follow a root pointer they have already read, and the tree it
    // points at never changes, so they have nothing to wait for.
    pub fn lock(&mut self) -> Result<(), String> {
        self.file
            .lock()
            .map_err(|e| format!("could not lock the database: {}", e))
    }

    pub fn unlock(&mut self) -> Result<(), String> {
        self.file
            .unlock()
            .map_err(|e| format!("could not unlock the database: {}", e))
    }

    // Appends data and returns the offset it was written at.
    pub fn write(&mut self, data: &[u8]) -> Result<Address, String> {
        let address = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|e| format!("could not seek to the end of the database: {}", e))?;

        // The length and the data go out together so a short write cannot land
        // a length with nothing behind it.
        let mut record = Vec::new();
        bytes::put_bytes(data, &mut record);
        self.file
            .write_all(&record)
            .map_err(|e| format!("could not append {} bytes: {}", record.len(), e))?;

        // std::fs::File is unbuffered, so this has already reached the
        // operating system. It has not necessarily reached the disk -- there is
        // no fsync here, the same as in the C++ and Python implementations.
        Ok(address)
    }

    pub fn read(&mut self, address: Address) -> Result<Vec<u8>, String> {
        let length = self.length()?;
        self.file
            .seek(SeekFrom::Start(address))
            .map_err(|e| format!("could not seek to {}: {}", address, e))?;

        let mut header = [0u8; 8];
        self.file
            .read_exact(&mut header)
            .map_err(|e| format!("could not read the record header at {}: {}", address, e))?;
        let mut idx = 0;
        let size = bytes::get_u64(&header, &mut idx)?;

        // A record cannot be longer than the file holding it. Without this
        // check a corrupt header would ask for an allocation of whatever
        // garbage happened to be on disk.
        if size > length {
            return Err(format!(
                "the record at {} claims {} bytes but the database is only {} bytes long",
                address, size, length
            ));
        }

        let mut data = vec![0u8; size as usize];
        self.file
            .read_exact(&mut data)
            .map_err(|e| format!("could not read the record at {}: {}", address, e))?;
        Ok(data)
    }

    // Points the database at a new tree. This is the single write that makes a
    // transaction visible to everybody else.
    pub fn commit_root_address(&mut self, address: Address) -> Result<(), String> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|e| format!("could not seek to the root pointer: {}", e))?;

        trace::emit(
            "root_commit",
            &format!("old_root={} new_root={}", self.root, address),
        );

        let mut record = Vec::new();
        bytes::put_u64(address, &mut record);
        self.file
            .write_all(&record)
            .map_err(|e| format!("could not write the root pointer: {}", e))?;
        self.root = address;
        Ok(())
    }

    pub fn get_root_address(&mut self) -> Result<Address, String> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|e| format!("could not seek to the root pointer: {}", e))?;

        let mut header = [0u8; 8];
        self.file
            .read_exact(&mut header)
            .map_err(|e| format!("could not read the root pointer: {}", e))?;

        let mut idx = 0;
        self.root = bytes::get_u64(&header, &mut idx)?;
        Ok(self.root)
    }

    fn length(&mut self) -> Result<u64, String> {
        let metadata = self
            .file
            .metadata()
            .map_err(|e| format!("could not measure the database: {}", e))?;
        Ok(metadata.len())
    }
}

// Dropping the File closes its descriptor, and the operating system releases
// the lock with it. That is what makes `exit` and end-of-input safe: a session
// that walks away in the middle of a transaction never leaves the database
// locked against everybody else.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::temp_path;

    #[test]
    fn a_new_database_starts_with_an_empty_root() {
        let path = temp_path("storage-new");
        let mut storage = FileStorage::open(&path).unwrap();
        assert_eq!(storage.get_root_address().unwrap(), NULL_ADDRESS);
        // Nothing but the root pointer.
        assert_eq!(storage.length().unwrap(), 8);
    }

    #[test]
    fn a_record_can_be_read_back_from_the_address_it_was_written_at() {
        let path = temp_path("storage-roundtrip");
        let mut storage = FileStorage::open(&path).unwrap();

        let first = storage.write(b"hello").unwrap();
        let second = storage.write(b"goodbye").unwrap();
        assert_ne!(first, NULL_ADDRESS);
        assert_ne!(first, second);

        assert_eq!(storage.read(first).unwrap(), b"hello");
        assert_eq!(storage.read(second).unwrap(), b"goodbye");
    }

    #[test]
    fn the_root_pointer_is_what_a_second_connection_sees() {
        let path = temp_path("storage-root");
        let address = {
            let mut storage = FileStorage::open(&path).unwrap();
            let address = storage.write(b"a tree, supposedly").unwrap();
            storage.commit_root_address(address).unwrap();
            address
        };

        let mut reopened = FileStorage::open(&path).unwrap();
        assert_eq!(reopened.get_root_address().unwrap(), address);
    }

    #[test]
    fn a_nonsense_address_is_an_error_and_not_a_panic() {
        let path = temp_path("storage-garbage");
        let mut storage = FileStorage::open(&path).unwrap();
        assert!(storage.read(999_999).is_err());
    }
}
