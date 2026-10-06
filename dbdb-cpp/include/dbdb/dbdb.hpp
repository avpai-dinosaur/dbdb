#pragma once
#include <memory>
#include <optional>
#include <string>
#include "logicalBase.hpp"
#include "storage.hpp"
#include "fileStorage.hpp"

class DBDB
{
private:
    std::unique_ptr<Storage> storage;
    std::unique_ptr<LogicalBase> kvStore;
    explicit DBDB(std::unique_ptr<Storage> s, std::unique_ptr<LogicalBase> l) : storage(std::move(s)), kvStore(std::move(l)) {}
public:
    template <typename LogicalStore>
    static DBDB connect(std::unique_ptr<Storage> storage)
    {
        static_assert(std::is_base_of_v<LogicalBase, LogicalStore>, "Tree must derive from LogicalBase");
        std::unique_ptr<LogicalStore> store = std::make_unique<LogicalStore>(*storage);
        return DBDB(std::move(storage), std::move(store));
    }
    template <typename LogicalStore>
    static DBDB connect(const std::string& path) { return connect<LogicalStore>(std::make_unique<FileStorage>(path)); }
    std::optional<Bytes> get(const std::string& key) { return kvStore->get(key); }
    void set(const std::string& key, const Bytes& value) { return kvStore->set(key, value); }
    void remove(const std::string& key) { return kvStore->remove(key); }
    void commit() { return kvStore->commit(); }
};
