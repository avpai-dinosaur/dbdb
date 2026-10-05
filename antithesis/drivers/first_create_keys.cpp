#include <cstdlib>
#include <iostream>
#include <string>
#include "dbdb/binaryTree.hpp"
#include "dbdb/dbdb.hpp"

int main()
{
    std::cout << "first_create_keys: started\n";
    std::string path = "/data/dbdb.db";
    DBDB db = DBDB::connect<BinaryTree>(path);
    db.set("accountA", to_bytes("full"));
    db.set("accountB", to_bytes("empty"));
    db.commit();
    std::cout << "first_create_keys: finished - accountA(full) accountB(empty)\n";
    return 0;
}