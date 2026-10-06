#pragma once
#include <cstdint>
#include <optional>
#include <string>
#include "storage.hpp"
#include "trace.hpp"

struct Value
{
    std::string key;
    Bytes value;
    Value() = default;
    explicit Value(std::string k, Bytes v) : key(std::move(k)), value(std::move(v)) {}
    virtual ~Value() = default;
    virtual Bytes encode() const = 0;
    virtual void decode(const Bytes& buffer) = 0;
};

struct ValueRef
{
    mutable Address address;
    mutable std::shared_ptr<const Value> reference;
    explicit ValueRef(Address a = NULL_ADDRESS, std::shared_ptr<const Value> r = nullptr) : address(a), reference(std::move(r)){}
    virtual ~ValueRef() = default;
    virtual const Value* get_value(Storage& storage) const = 0;
};

class LogicalBase
{
private:
    bool isLockHeld = false;
    std::uint64_t commitSequence = 0;
protected:
    Storage& storage;
    virtual std::optional<Bytes> get_entry(const std::string& key) = 0;
    virtual void set_entry(const std::string& key, const Bytes& value) = 0;
    virtual void remove_entry(const std::string& key) = 0;
    virtual void commit_data() = 0;
    // reason is a tag for debug logs
    virtual void refresh_database_view(const char* reason) = 0;
public:
    explicit LogicalBase(Storage& storage) : storage(storage) {}
    virtual ~LogicalBase() = default;

    std::optional<Bytes> get(const std::string& key) 
    {
        if (!isLockHeld)
        {
            refresh_database_view("get_unlocked");
        }
        return get_entry(key);
    }   

    void set(const std::string& key, const Bytes& value)
    {
        if (!isLockHeld)
        {
            storage.lock();
            isLockHeld = true;
            refresh_database_view("set_locked");
        }
        return set_entry(key, value);
    }

    void remove(const std::string& key)
    {
        if(!isLockHeld)
        {
            storage.lock();
            isLockHeld = true;
            refresh_database_view("remove_locked");
        }
        return remove_entry(key);
    }

    void commit()
    {
        const std::uint64_t seq = ++commitSequence;
        dbdb::trace::emit("commit_begin", {{"seq", seq}});
        commit_data();
        storage.unlock();
        isLockHeld = false;
        dbdb::trace::emit("commit_end", {{"seq", seq}});
    }
};
