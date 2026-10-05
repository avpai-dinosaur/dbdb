#include <cstdlib>
#include <iostream>
#include <optional>
#include <string>
#include "dbdb/binaryTree.hpp"
#include "dbdb/dbdb.hpp"
#include "antithesis_sdk.h"

int main()
{
    std::string path = "/data/dbdb.db";
    DBDB db = DBDB::connect<BinaryTree>(path);

    std::optional<Bytes> aVal = db.get("accountA");
    bool aValExists = aVal.has_value();
    ALWAYS(aValExists, "accountA exists in db");

    std::optional<Bytes> bVal = db.get("accountB");
    bool bValExists = bVal.has_value();
    ALWAYS(bValExists, "accountB exists in db");

    if (!aValExists || !bValExists) return 0;

    std::string aValStr = to_string(*aVal);
    std::string bValStr = to_string(*bVal);
    int count = 0;
    if (aValStr == "full") count += 1;
    if (bValStr == "full") count += 1;
    ALWAYS(count == 1, "total value between accountA and accountB is 1");

    if (count != 1) return 0;

    if (aValStr == "full")
    {
        db.set("accountA", to_bytes("empty"));
        db.set("accountB", to_bytes("full"));
        REACHABLE("transfer from a to b");
    }
    else
    {
        db.set("accountA", to_bytes("full"));
        db.set("accountB", to_bytes("empty"));
        REACHABLE("transfer from b to a");
    }

    db.commit();
    return 0;
}