#include <stdexcept>
#include "doctest.h"
#include "../src/binaryTree.hpp"
#include "../src/memoryStorage.hpp"

static Node make_node(const std::string& key, const Bytes& value, Address left, Address right)
{
    return Node(key, value, NodeRef(left), NodeRef(right));
}

static Node round_trip(const Node& node)
{
    Node decoded;
    decoded.decode(node.encode());
    return decoded;
}

static Bytes raw(std::initializer_list<int> values)
{
    Bytes b;
    for (int v : values) b.push_back(static_cast<std::byte>(v));
    return b;
}

TEST_CASE("to_bytes/to_string round trip")
{
    CHECK(to_string(to_bytes("hello")) == "hello");
    CHECK(to_bytes("").empty());
}

TEST_CASE("put_u64/get_u64 round trip")
{
    Bytes buffer;
    put_u64(0, buffer);
    put_u64(0x0123456789ABCDEFull, buffer);
    put_u64(UINT64_MAX, buffer);
    CHECK(buffer.size() == 24);

    size_t idx = 0;
    CHECK(get_u64(buffer, idx) == 0);
    CHECK(get_u64(buffer, idx) == 0x0123456789ABCDEFull);
    CHECK(get_u64(buffer, idx) == UINT64_MAX);
    CHECK(idx == 24);
}

TEST_CASE("put_bytes/get_bytes round trip")
{
    Bytes buffer;
    put_bytes(to_bytes("first"), buffer);
    put_bytes(to_bytes("second"), buffer);

    size_t idx = 0;
    CHECK(to_string(get_bytes(buffer, idx)) == "first");
    CHECK(to_string(get_bytes(buffer, idx)) == "second");
    CHECK(idx == buffer.size());
}

TEST_CASE("node encode/decode round trip")
{
    Node decoded = round_trip(make_node("key", to_bytes("value"), 16, 42));
    CHECK(decoded.key == "key");
    CHECK(decoded.value == to_bytes("value"));
    CHECK(decoded.left.address == 16);
    CHECK(decoded.right.address == 42);
}

TEST_CASE("decoded node has no cached children")
{
    Node decoded = round_trip(make_node("k", to_bytes("v"), 1, 2));
    CHECK(decoded.left.reference.get() == nullptr);
    CHECK(decoded.right.reference.get() == nullptr);
}

TEST_CASE("node encode/decode round trip with empty key and empty value")
{
    Node decoded = round_trip(make_node("", Bytes{}, NULL_ADDRESS, NULL_ADDRESS));
    CHECK(decoded.key == "");
    CHECK(decoded.value.empty());
    CHECK(decoded.left.address == NULL_ADDRESS);
    CHECK(decoded.right.address == NULL_ADDRESS);
}

TEST_CASE("node encode/decode round trip with binary value bytes")
{
    Bytes value = raw({0x00, 0xFF, 0x7F, 0x80, 0x00});
    CHECK(round_trip(make_node("bin", value, NULL_ADDRESS, NULL_ADDRESS)).value == value);
}

TEST_CASE("node encode/decode round trip with large addresses")
{
    Node decoded = round_trip(make_node("k", Bytes{}, 0x0123456789ABCDEFull, UINT64_MAX));
    CHECK(decoded.left.address == 0x0123456789ABCDEFull);
    CHECK(decoded.right.address == UINT64_MAX);
}

TEST_CASE("node encoded layout is length-prefixed little-endian")
{
    Bytes expected = raw({
        2, 0, 0, 0, 0, 0, 0, 0,  'h', 'i',   // key length + key
        1, 0, 0, 0, 0, 0, 0, 0,  'v',        // value length + value
        5, 0, 0, 0, 0, 0, 0, 0,              // left address
        0, 1, 0, 0, 0, 0, 0, 0,              // right address (256)
    });
    CHECK(make_node("hi", to_bytes("v"), 5, 256).encode() == expected);
}

TEST_CASE("decode throws on truncated buffer")
{
    Bytes full = make_node("key", to_bytes("value"), 1, 2).encode();
    for (size_t len = 0; len < full.size(); ++len)
    {
        CAPTURE(len);
        Bytes truncated(full.begin(), full.begin() + len);
        Node node;
        CHECK_THROWS_AS(node.decode(truncated), std::runtime_error);
    }
}

TEST_CASE("get_value on empty ref returns nullptr")
{
    MemoryStorage storage;
    NodeRef ref;
    CHECK_FALSE(ref.get_value(storage));
}

TEST_CASE("get_value loads and caches a node from storage")
{
    MemoryStorage storage;
    Address addr = storage.write(make_node("k", to_bytes("v"), NULL_ADDRESS, NULL_ADDRESS).encode());

    NodeRef ref(addr);
    const Value* loaded = ref.get_value(storage);
    REQUIRE(loaded);
    CHECK(loaded->key == "k");
    CHECK(loaded->value == to_bytes("v"));
    CHECK(ref.get_value(storage) == loaded);   // second call uses the cache
}

TEST_CASE("committed tree can be read back by a fresh tree")
{
    MemoryStorage storage;
    BinaryTree writer(storage);
    writer.set("m", to_bytes("1"));
    writer.set("c", to_bytes("2"));
    writer.set("x", to_bytes("3"));
    writer.commit();

    BinaryTree reader(storage);
    CHECK(reader.get("m") == to_bytes("1"));
    CHECK(reader.get("c") == to_bytes("2"));
    CHECK(reader.get("x") == to_bytes("3"));
    std::vector<std::string> expected = {"c", "m", "x"};
    CHECK(reader.in_order() == expected);
}
