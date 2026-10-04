// Placeholder driver: proves drivers link against libdbdb and can round-trip a value.
// Replace or extend with real drivers when writing the workload.
#include <cstdlib>
#include <iostream>
#include <optional>
#include <string>
#include "dbdb/binaryTree.hpp"
#include "dbdb/dbdb.hpp"

int main()
{
    const char* env = std::getenv("DBDB_PATH");
    std::string path = env ? env : "/data/dbdb.db";

    DBDB db = DBDB::connect<BinaryTree>(path);
    db.set("smoke", to_bytes("ok"));
    db.commit();

    std::optional<Bytes> value = db.get("smoke");
    if (!value || to_string(*value) != "ok")
    {
        std::cerr << "smoke: read back failed\n";
        return 1;
    }
    std::cout << "smoke: ok\n";
    return 0;
}
