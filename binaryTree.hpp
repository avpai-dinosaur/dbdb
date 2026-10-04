#pragma once
#include <string>
#include <optional>
#include <memory>
#include <vector>
#include "logicalBase.hpp"

struct Node;

struct NodeRef : ValueRef
{
    explicit NodeRef(Address a = NULL_ADDRESS, std::shared_ptr<const Node> r = nullptr);
    const Value* get_value(Storage& storage) const override;
};

struct Node : Value
{
    NodeRef left;
    NodeRef right;
    Node() = default;
    explicit Node(std::string k, Bytes v, NodeRef l, NodeRef r) : Value(k, v), left(std::move(l)), right(std::move(r)) {}
    Bytes encode() const override;
    void decode(const Bytes& buffer) override;
};

// Defined after Node so shared_ptr<const Node> can convert to shared_ptr<const Value>.
inline NodeRef::NodeRef(Address a, std::shared_ptr<const Node> r) : ValueRef(a, std::move(r)) {}

class BinaryTree : public LogicalBase
{
private:
    const Node* get_node(const NodeRef& nodeRef) { return static_cast<const Node*>(nodeRef.get_value(storage)); }

    std::optional<Bytes> get_helper(const std::string& key, const NodeRef& nodeRef);
    NodeRef insert_helper(const std::string& key, const Bytes& value, const NodeRef& nodeRef);
    NodeRef remove_helper(const std::string& key, const NodeRef& nodeRef);
    const Node* get_min(const NodeRef& nodeRef);
    void in_order_helper(const NodeRef& nodeRef, std::vector<std::string>& soFar);
    Address commit_helper(const NodeRef& nodeRef);

    NodeRef root;
protected:
    std::optional<Bytes> get_entry(const std::string& key) override { return get_helper(key, root); }
    void set_entry(const std::string& key, const Bytes& value) override { root = insert_helper(key, value, root); }
    void remove_entry(const std::string& key) override { root = remove_helper(key, root); }
    void commit_data() override { storage.commit_root_address(commit_helper(root)); }
    void refresh_database_view() override;
public:
    explicit BinaryTree(Storage& storage) : LogicalBase(storage) { root = NodeRef(storage.get_root_address(), nullptr); }
    std::vector<std::string> in_order() { std::vector<std::string> values; in_order_helper(root, values); return values; }
};
