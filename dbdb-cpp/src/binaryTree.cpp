#include "dbdb/binaryTree.hpp"
#include "dbdb/trace.hpp"

Bytes Node::encode() const
{
    Bytes buffer;
    buffer.reserve(sizeof(Address) * 4 + key.size() + value.size());
    put_bytes(to_bytes(key), buffer);
    put_bytes(value, buffer);
    put_u64(left.address, buffer);
    put_u64(right.address, buffer);
    return buffer;
}

void Node::decode(const Bytes& buffer)
{
    size_t idx = 0;
    key = to_string(get_bytes(buffer, idx));
    value = get_bytes(buffer, idx);
    left = NodeRef(get_u64(buffer, idx), nullptr);
    right = NodeRef(get_u64(buffer, idx), nullptr);
}

const Value* NodeRef::get_value(Storage& storage) const
{
    if (reference == nullptr)
    {
        if (address == NULL_ADDRESS) return nullptr;
        std::shared_ptr<Node> node = std::make_shared<Node>();
        node->decode(storage.read(address));
        reference = node;
    }
    return reference.get();
}

std::optional<Bytes> BinaryTree::get_helper(const std::string& key, const NodeRef& nodeRef)
{
    const Node* node = get_node(nodeRef);
    if (node == nullptr) return std::nullopt; 
    if (node->key < key) return get_helper(key, node->right);
    if (node->key > key) return get_helper(key, node->left);
    return node->value;
}

NodeRef BinaryTree::insert_helper(const std::string& key, const Bytes& value, const NodeRef& nodeRef)
{
    const Node* node = get_node(nodeRef);
    if (node == nullptr) return NodeRef(NULL_ADDRESS, std::make_shared<Node>(key, value, NodeRef(), NodeRef()));
    if (node->key < key) return NodeRef(NULL_ADDRESS, std::make_shared<Node>(node->key, node->value, node->left, insert_helper(key, value, node->right)));
    if (node->key > key) return NodeRef(NULL_ADDRESS, std::make_shared<Node>(node->key, node->value, insert_helper(key, value, node->left), node->right));
    return NodeRef(NULL_ADDRESS, std::make_shared<Node>(node->key, value, node->left, node->right));
}

const Node* BinaryTree::get_min(const NodeRef& nodeRef)
{
    const Node* node = get_node(nodeRef);
    if (node == nullptr) return nullptr;
    if (get_node(node->left) == nullptr) return node;
    return get_min(node->left);
}

NodeRef BinaryTree::remove_helper(const std::string& key, const NodeRef& nodeRef) 
{
    const Node* node = get_node(nodeRef);
    if (node == nullptr) return NodeRef();
    if (node->key < key) return NodeRef(NULL_ADDRESS, std::make_shared<Node>(node->key, node->value, node->left, remove_helper(key, node->right)));
    if (node->key > key) return NodeRef(NULL_ADDRESS, std::make_shared<Node>(node->key, node->value, remove_helper(key, node->left), node->right));
    if (get_node(node->left) == nullptr && get_node(node->right) == nullptr) return NodeRef();
    if (get_node(node->right) == nullptr) return node->left;
    if (get_node(node->left) == nullptr) return node->right;
    const Node* inOrderSuccessor = get_min(node->right);
    return NodeRef(NULL_ADDRESS, std::make_shared<Node>(inOrderSuccessor->key, inOrderSuccessor->value, node->left, remove_helper(inOrderSuccessor->key, node->right)));
}

void BinaryTree::in_order_helper(const NodeRef& nodeRef, std::vector<std::string>& soFar)
{
    const Node* node = get_node(nodeRef);
    if (node == nullptr) return;
    in_order_helper(node->left, soFar);
    soFar.push_back(node->key);
    in_order_helper(node->right, soFar);
}

Address BinaryTree::commit_helper(const NodeRef& nodeRef)
{
    if (nodeRef.address != NULL_ADDRESS) return nodeRef.address;
    const Node* node = get_node(nodeRef);
    if (node == nullptr) return NULL_ADDRESS;
    node->left.address = commit_helper(node->left);
    node->right.address = commit_helper(node->right);
    Address commitAddress = storage.write(node->encode());
    dbdb::trace::emit("node_commit", {{"addr", commitAddress},
                                      {"key", node->key},
                                      {"value", node->value},
                                      {"left", node->left.address},
                                      {"right", node->right.address}});
    return commitAddress;
}

void BinaryTree::refresh_database_view(const char* reason) 
{ 
    Address latest = storage.get_root_address();
    dbdb::trace::emit("view_refresh", {{"reason", reason},
                                       {"old_root", root.address},
                                       {"new_root", latest}});
    if (latest != root.address)
    {
        root = NodeRef(latest, nullptr); 
    }
}
