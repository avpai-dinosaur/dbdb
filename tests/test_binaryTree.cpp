#include "doctest.h"
#include "dbdb/binaryTree.hpp"
#include "dbdb/memoryStorage.hpp"

static std::string get_str(BinaryTree& bt, const std::string& key)
{
    std::optional<Bytes> v = bt.get(key);
    if (!v) return "<none>";
    return to_string(*v);
}

TEST_CASE("get on empty tree returns nothing")
{
    MemoryStorage storage;
    BinaryTree bt(storage);
    CHECK_FALSE(bt.get("missing").has_value());
}

TEST_CASE("set then get")
{
    MemoryStorage storage;
    BinaryTree bt(storage);
    bt.set("a", to_bytes("1"));
    CHECK(get_str(bt, "a") == "1");
}

TEST_CASE("set existing key overwrites")
{
    MemoryStorage storage;
    BinaryTree bt(storage);
    bt.set("a", to_bytes("1"));
    bt.set("a", to_bytes("2"));
    CHECK(get_str(bt, "a") == "2");
}

TEST_CASE("remove node with two children")
{
    MemoryStorage storage;
    BinaryTree bt(storage);
    bt.set("m", to_bytes("m"));
    bt.set("c", to_bytes("c"));
    bt.set("x", to_bytes("x"));
    bt.remove("m");
    CHECK(get_str(bt, "m") == "<none>");
    CHECK(get_str(bt, "c") == "c");
    CHECK(get_str(bt, "x") == "x");
}

TEST_CASE("remove node where successor has children")
{
    MemoryStorage storage;
    BinaryTree bt(storage);
    bt.set("m", to_bytes("m"));
    bt.set("c", to_bytes("c"));
    bt.set("x", to_bytes("x"));
    bt.set("n", to_bytes("n"));
    bt.set("o", to_bytes("o"));
    bt.set("p", to_bytes("p"));
    bt.remove("m");
    CHECK(get_str(bt, "m") == "<none>");
    CHECK(get_str(bt, "c") == "c");
    CHECK(get_str(bt, "x") == "x");
    CHECK(get_str(bt, "n") == "n");
    CHECK(get_str(bt, "o") == "o");
    CHECK(get_str(bt, "p") == "p");
    std::vector<std::string> expected = {"c", "n", "o", "p", "x"};
    CHECK(bt.in_order() == expected);
}

TEST_CASE("storage gets atomic commit")
{
    MemoryStorage storage;
    BinaryTree bt(storage);

    bt.set("m", to_bytes("m"));
    CHECK(storage.get_root_address() == NULL_ADDRESS);
    bt.commit();
    CHECK(storage.get_root_address() == 1);
}

TEST_CASE("snapshot gives consistent reads")
{
    MemoryStorage storage;
    BinaryTree bt1(storage);
    BinaryTree bt2(storage);

    bt1.set("m", to_bytes("m"));
    bt1.commit();
    CHECK(get_str(bt2, "m") == "m");
    bt1.set("m", to_bytes("o"));
    CHECK(get_str(bt2, "m") == "m");
}

TEST_CASE("get during a write sees pending changes")
{
    MemoryStorage storage;
    BinaryTree bt(storage);

    bt.set("a", to_bytes("1"));
    bt.commit();

    bt.set("a", to_bytes("2"));
    bt.set("b", to_bytes("3"));
    CHECK(get_str(bt, "a") == "2");
    CHECK(get_str(bt, "b") == "3");

    bt.remove("a");
    CHECK(get_str(bt, "a") == "<none>");
    CHECK(get_str(bt, "b") == "3");
    bt.commit();

    BinaryTree fresh(storage);
    CHECK(get_str(fresh, "a") == "<none>");
    CHECK(get_str(fresh, "b") == "3");
}

TEST_CASE("remove missing key is harmless")
{
    MemoryStorage storage;
    BinaryTree bt(storage);
    bt.set("a", to_bytes("1"));
    bt.remove("zzz");
    CHECK(get_str(bt, "a") == "1");
}
