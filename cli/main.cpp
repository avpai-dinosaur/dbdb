#include <iostream>
#include <string>
#include "dbdb/dbdb.hpp"
#include "dbdb/binaryTree.hpp"

static int usage()
{
    std::cerr << "usage:\n"
              << "  dbdb DBNAME get KEY\n"
              << "  dbdb DBNAME set KEY VALUE\n"
              << "  dbdb DBNAME delete KEY\n";
    return 1;
}

int main(int argc, char* argv[])
{
    if (argc < 4) return usage();
    std::string dbname = argv[1];
    std::string verb = argv[2];
    std::string key = argv[3];

    DBDB db = DBDB::connect<BinaryTree>(dbname);

    if (verb == "get" && argc == 4)
    {
        std::optional<Bytes> value = db.get(key);
        if (!value)
        {
            std::cerr << "key not found: " << key << "\n";
            return 2;
        }
        std::cout << to_string(*value) << "\n";
    }
    else if (verb == "set" && argc == 5)
    {
        db.set(key, to_bytes(argv[4]));
        db.commit();
    }
    else if (verb == "delete" && argc == 4)
    {
        db.remove(key);
        db.commit();
    }
    else
    {
        return usage();
    }
    return 0;
}
