// The key/value store itself: an unbalanced binary search tree whose nodes are
// never modified.
//
// Changing a key does not edit the node holding it. It builds a new node, and
// new copies of every node on the path from the root down to it, and leaves the
// old ones alone -- so the old tree is still a complete, readable tree right up
// until the root pointer moves. Everything off that path is shared between the
// two versions rather than copied, so a change costs one new node per level and
// not a whole new tree.
//
// Nothing here takes the lock or decides when a transaction starts. That lives
// in db.rs; this module only knows how to look things up and how to write a
// tree out.

use std::rc::Rc;

use crate::bytes;
use crate::storage::Address;
use crate::storage::FileStorage;
use crate::storage::NULL_ADDRESS;
use crate::trace;

pub struct Node {
    pub key: String,
    pub value: String,
    pub left: NodeRef,
    pub right: NodeRef,
}

// Where a child is. Three shapes matter:
//
//   address == NULL, node == None     there is no child here
//   address != NULL, node == None     the child is on disk and has not been read yet
//   address == NULL, node == Some(..) the child was made by set or delete and
//                                     has not been written yet
//
// A node that has an address is already on disk, which is why committing can
// stop as soon as it reaches one.
#[derive(Clone)]
pub struct NodeRef {
    pub address: Address,
    pub node: Option<Rc<Node>>,
}

impl NodeRef {
    pub fn null() -> NodeRef {
        NodeRef {
            address: NULL_ADDRESS,
            node: None,
        }
    }

    pub fn stored(address: Address) -> NodeRef {
        NodeRef {
            address,
            node: None,
        }
    }

    fn in_memory(key: String, value: String, left: NodeRef, right: NodeRef) -> NodeRef {
        NodeRef {
            address: NULL_ADDRESS,
            node: Some(Rc::new(Node {
                key,
                value,
                left,
                right,
            })),
        }
    }
}

fn encode(key: &str, value: &str, left: Address, right: Address) -> Vec<u8> {
    let mut buffer = Vec::new();
    bytes::put_bytes(key.as_bytes(), &mut buffer);
    bytes::put_bytes(value.as_bytes(), &mut buffer);
    bytes::put_u64(left, &mut buffer);
    bytes::put_u64(right, &mut buffer);
    buffer
}

fn decode(buffer: &[u8]) -> Result<Node, String> {
    let mut idx = 0;
    let key = bytes::get_bytes(buffer, &mut idx)?;
    let value = bytes::get_bytes(buffer, &mut idx)?;
    let left = bytes::get_u64(buffer, &mut idx)?;
    let right = bytes::get_u64(buffer, &mut idx)?;

    let key = String::from_utf8(key).map_err(|e| format!("a stored key is not utf-8: {}", e))?;
    let value =
        String::from_utf8(value).map_err(|e| format!("a stored value is not utf-8: {}", e))?;

    Ok(Node {
        key,
        value,
        left: NodeRef::stored(left),
        right: NodeRef::stored(right),
    })
}

// Hands back the node a reference points at, reading it off disk if that is
// where it still lives. A stored node is read again every time it is visited.
// Keeping the decoded node in the reference would save a lot of reads, but a
// node on disk never changes, so reading it twice can never give two different
// answers.
fn load(storage: &mut FileStorage, nref: &NodeRef) -> Result<Option<Rc<Node>>, String> {
    if let Some(node) = &nref.node {
        return Ok(Some(Rc::clone(node)));
    }
    if nref.address == NULL_ADDRESS {
        return Ok(None);
    }
    let buffer = storage.read(nref.address)?;
    Ok(Some(Rc::new(decode(&buffer)?)))
}

fn get_helper(
    storage: &mut FileStorage,
    nref: &NodeRef,
    key: &str,
) -> Result<Option<String>, String> {
    let node = match load(storage, nref)? {
        None => return Ok(None),
        Some(node) => node,
    };
    if node.key.as_str() < key {
        return get_helper(storage, &node.right, key);
    }
    if node.key.as_str() > key {
        return get_helper(storage, &node.left, key);
    }
    Ok(Some(node.value.clone()))
}

// Returns the root of a tree that is the old one with key set to value. The old
// tree is left untouched, and every node the new tree does not need a fresh
// copy of is shared with it.
fn insert_helper(
    storage: &mut FileStorage,
    nref: &NodeRef,
    key: &str,
    value: &str,
) -> Result<NodeRef, String> {
    let node = match load(storage, nref)? {
        None => {
            return Ok(NodeRef::in_memory(
                key.to_string(),
                value.to_string(),
                NodeRef::null(),
                NodeRef::null(),
            ))
        }
        Some(node) => node,
    };

    if node.key.as_str() < key {
        let right = insert_helper(storage, &node.right, key, value)?;
        return Ok(NodeRef::in_memory(
            node.key.clone(),
            node.value.clone(),
            node.left.clone(),
            right,
        ));
    }
    if node.key.as_str() > key {
        let left = insert_helper(storage, &node.left, key, value)?;
        return Ok(NodeRef::in_memory(
            node.key.clone(),
            node.value.clone(),
            left,
            node.right.clone(),
        ));
    }
    // The key is already here, so this is an overwrite: same key, same children,
    // new value.
    Ok(NodeRef::in_memory(
        node.key.clone(),
        value.to_string(),
        node.left.clone(),
        node.right.clone(),
    ))
}

fn get_min(storage: &mut FileStorage, nref: &NodeRef) -> Result<Option<Rc<Node>>, String> {
    let node = match load(storage, nref)? {
        None => return Ok(None),
        Some(node) => node,
    };
    if load(storage, &node.left)?.is_none() {
        return Ok(Some(node));
    }
    get_min(storage, &node.left)
}

// Returns the root of a tree that is the old one without key. Removing a key
// that is not there walks to where it would have been and rebuilds the path it
// walked, which is wasted work but not an error -- the protocol wants delete to
// be safe to retry.
fn remove_helper(storage: &mut FileStorage, nref: &NodeRef, key: &str) -> Result<NodeRef, String> {
    let node = match load(storage, nref)? {
        None => return Ok(NodeRef::null()),
        Some(node) => node,
    };

    if node.key.as_str() < key {
        let right = remove_helper(storage, &node.right, key)?;
        return Ok(NodeRef::in_memory(
            node.key.clone(),
            node.value.clone(),
            node.left.clone(),
            right,
        ));
    }
    if node.key.as_str() > key {
        let left = remove_helper(storage, &node.left, key)?;
        return Ok(NodeRef::in_memory(
            node.key.clone(),
            node.value.clone(),
            left,
            node.right.clone(),
        ));
    }

    // This is the node to drop. With no children it just goes away, and with
    // one child that child takes its place.
    let has_left = load(storage, &node.left)?.is_some();
    let has_right = load(storage, &node.right)?.is_some();
    if !has_left && !has_right {
        return Ok(NodeRef::null());
    }
    if !has_right {
        return Ok(node.left.clone());
    }
    if !has_left {
        return Ok(node.right.clone());
    }

    // With two children, the smallest key on the right is the one that can take
    // this node's place without disturbing the ordering, so it moves up and is
    // removed from where it was.
    let successor = match get_min(storage, &node.right)? {
        None => return Err("the right subtree disappeared while deleting".to_string()),
        Some(successor) => successor,
    };
    let right = remove_helper(storage, &node.right, &successor.key)?;
    Ok(NodeRef::in_memory(
        successor.key.clone(),
        successor.value.clone(),
        node.left.clone(),
        right,
    ))
}

// Writes out every node that is not on disk yet, deepest first, and returns the
// address the tree now starts at. Children are written before their parent
// because a parent records where its children went.
fn commit_helper(storage: &mut FileStorage, nref: &NodeRef) -> Result<Address, String> {
    if nref.address != NULL_ADDRESS {
        return Ok(nref.address);
    }
    let node = match load(storage, nref)? {
        None => return Ok(NULL_ADDRESS),
        Some(node) => node,
    };

    let left = commit_helper(storage, &node.left)?;
    let right = commit_helper(storage, &node.right)?;
    let address = storage.write(&encode(&node.key, &node.value, left, right))?;
    trace::emit(
        "node_commit",
        &format!(
            "addr={} key={} value={} left={} right={}",
            address,
            trace::quote(&node.key),
            trace::quote(&node.value),
            left,
            right
        ),
    );
    Ok(address)
}

#[cfg(test)]
fn in_order_helper(
    storage: &mut FileStorage,
    nref: &NodeRef,
    so_far: &mut Vec<String>,
) -> Result<(), String> {
    let node = match load(storage, nref)? {
        None => return Ok(()),
        Some(node) => node,
    };
    in_order_helper(storage, &node.left, so_far)?;
    so_far.push(node.key.clone());
    in_order_helper(storage, &node.right, so_far)
}

pub struct BinaryTree {
    root: NodeRef,
}

impl BinaryTree {
    pub fn new(root_address: Address) -> BinaryTree {
        BinaryTree {
            root: NodeRef::stored(root_address),
        }
    }

    pub fn get(&self, storage: &mut FileStorage, key: &str) -> Result<Option<String>, String> {
        get_helper(storage, &self.root, key)
    }

    pub fn set(&mut self, storage: &mut FileStorage, key: &str, value: &str) -> Result<(), String> {
        // Cloned first because the new root is built out of the old one, and the
        // old one is still sitting in self while that happens.
        let root = self.root.clone();
        self.root = insert_helper(storage, &root, key, value)?;
        Ok(())
    }

    pub fn remove(&mut self, storage: &mut FileStorage, key: &str) -> Result<(), String> {
        let root = self.root.clone();
        self.root = remove_helper(storage, &root, key)?;
        Ok(())
    }

    pub fn commit(&mut self, storage: &mut FileStorage) -> Result<(), String> {
        let root = self.root.clone();
        let address = commit_helper(storage, &root)?;
        // Only now, with every node of the new tree safely on disk, does the
        // database start pointing at it.
        storage.commit_root_address(address)?;
        // What was an in-memory tree is the stored tree now, so the in-memory
        // copy can be dropped and the address followed instead.
        self.root = NodeRef::stored(address);
        Ok(())
    }

    // Catches this tree up to whatever has been committed. Anybody holding the
    // old root keeps seeing the old tree, which is still intact; this just stops
    // looking at it.
    pub fn refresh(&mut self, storage: &mut FileStorage, reason: &str) -> Result<(), String> {
        let latest = storage.get_root_address()?;
        trace::emit(
            "view_refresh",
            &format!(
                "reason={} old_root={} new_root={}",
                trace::quote(reason),
                self.root.address,
                latest
            ),
        );
        if latest != self.root.address {
            self.root = NodeRef::stored(latest);
        }
        Ok(())
    }

    // Every key, in order. Nothing in the protocol asks for this; it is here so
    // the tests can check that the shape of the tree survives a delete.
    #[cfg(test)]
    pub fn in_order(&self, storage: &mut FileStorage) -> Result<Vec<String>, String> {
        let mut keys = Vec::new();
        in_order_helper(storage, &self.root, &mut keys)?;
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::temp_path;

    fn fresh() -> (FileStorage, BinaryTree) {
        let path = temp_path("tree");
        let mut storage = FileStorage::open(&path).unwrap();
        let root = storage.get_root_address().unwrap();
        let tree = BinaryTree::new(root);
        (storage, tree)
    }

    #[test]
    fn a_node_survives_a_round_trip_through_the_file() {
        let encoded = encode("accountA", "a value with spaces", 24, 48);
        let node = decode(&encoded).unwrap();
        assert_eq!(node.key, "accountA");
        assert_eq!(node.value, "a value with spaces");
        assert_eq!(node.left.address, 24);
        assert_eq!(node.right.address, 48);
    }

    #[test]
    fn a_key_can_be_read_back_before_it_is_committed() {
        let (mut storage, mut tree) = fresh();
        tree.set(&mut storage, "accountA", "full").unwrap();
        assert_eq!(
            tree.get(&mut storage, "accountA").unwrap(),
            Some("full".to_string())
        );
    }

    #[test]
    fn a_missing_key_is_not_an_error() {
        let (mut storage, tree) = fresh();
        assert_eq!(tree.get(&mut storage, "nothing here").unwrap(), None);
    }

    #[test]
    fn setting_a_key_twice_keeps_the_second_value() {
        let (mut storage, mut tree) = fresh();
        tree.set(&mut storage, "accountA", "full").unwrap();
        tree.set(&mut storage, "accountA", "empty").unwrap();
        tree.commit(&mut storage).unwrap();
        assert_eq!(
            tree.get(&mut storage, "accountA").unwrap(),
            Some("empty".to_string())
        );
        assert_eq!(tree.in_order(&mut storage).unwrap(), vec!["accountA"]);
    }

    #[test]
    fn a_committed_tree_can_be_read_back_from_the_file() {
        let path = temp_path("tree-reopen");
        {
            let mut storage = FileStorage::open(&path).unwrap();
            let mut tree = BinaryTree::new(storage.get_root_address().unwrap());
            for key in ["m", "c", "x", "a", "e"] {
                tree.set(&mut storage, key, "value").unwrap();
            }
            tree.commit(&mut storage).unwrap();
        }

        let mut storage = FileStorage::open(&path).unwrap();
        let tree = BinaryTree::new(storage.get_root_address().unwrap());
        assert_eq!(
            tree.in_order(&mut storage).unwrap(),
            vec!["a", "c", "e", "m", "x"]
        );
    }

    #[test]
    fn deleting_leaves_the_other_keys_in_order() {
        let (mut storage, mut tree) = fresh();
        for key in ["m", "c", "x", "a", "e", "p"] {
            tree.set(&mut storage, key, key).unwrap();
        }

        // "c" has two children, so this is the case that has to promote a
        // successor rather than just unhook a node.
        tree.remove(&mut storage, "c").unwrap();
        tree.commit(&mut storage).unwrap();

        assert_eq!(tree.get(&mut storage, "c").unwrap(), None);
        assert_eq!(
            tree.in_order(&mut storage).unwrap(),
            vec!["a", "e", "m", "p", "x"]
        );
        assert_eq!(tree.get(&mut storage, "e").unwrap(), Some("e".to_string()));
    }

    #[test]
    fn deleting_a_key_that_is_not_there_changes_nothing() {
        let (mut storage, mut tree) = fresh();
        tree.set(&mut storage, "accountA", "full").unwrap();
        tree.remove(&mut storage, "accountB").unwrap();
        assert_eq!(tree.in_order(&mut storage).unwrap(), vec!["accountA"]);
    }

    #[test]
    fn uncommitted_writes_are_not_in_the_file() {
        let path = temp_path("tree-uncommitted");
        {
            let mut storage = FileStorage::open(&path).unwrap();
            let mut tree = BinaryTree::new(storage.get_root_address().unwrap());
            tree.set(&mut storage, "accountA", "full").unwrap();
            // No commit: the tree only ever existed in this process.
        }

        let mut storage = FileStorage::open(&path).unwrap();
        let tree = BinaryTree::new(storage.get_root_address().unwrap());
        assert_eq!(tree.get(&mut storage, "accountA").unwrap(), None);
    }

    #[test]
    fn committing_twice_only_writes_what_changed() {
        let (mut storage, mut tree) = fresh();
        tree.set(&mut storage, "accountA", "full").unwrap();
        tree.commit(&mut storage).unwrap();
        let after_first = storage.get_root_address().unwrap();

        // Nothing new to write, so the tree is already where it was.
        tree.commit(&mut storage).unwrap();
        assert_eq!(storage.get_root_address().unwrap(), after_first);
    }
}
