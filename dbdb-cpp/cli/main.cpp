#include <iostream>
#include <optional>
#include <string>
#include "dbdb/dbdb.hpp"
#include "dbdb/binaryTree.hpp"

static int usage()
{
    std::cerr << "usage:\n"
              << "  dbdb DBNAME get KEY\n"
              << "  dbdb DBNAME set KEY VALUE\n"
              << "  dbdb DBNAME delete KEY\n"
              << "  dbdb DBNAME [repl]      read commands from stdin\n";
    return 1;
}

// Splits off the next space-delimited token, advancing idx past it. Returns ""
// once the line is exhausted.
static std::string next_token(const std::string& line, std::size_t& idx)
{
    while (idx < line.size() && line[idx] == ' ') ++idx;
    const std::size_t start = idx;
    while (idx < line.size() && line[idx] != ' ') ++idx;
    return line.substr(start, idx - start);
}

// The remainder of the line, minus the run of spaces separating it from the
// previous token. Unlike next_token this keeps interior spaces, so a value may
// contain them (leading ones are not preserved).
static std::string rest_of_line(const std::string& line, std::size_t& idx)
{
    while (idx < line.size() && line[idx] == ' ') ++idx;
    return line.substr(idx);
}

// A response is exactly one line, so an error message carrying a newline would
// desynchronize the caller's parser.
static std::string one_line(const std::string& s)
{
    std::string out = s;
    for (char& c : out)
    {
        if (c == '\n' || c == '\r') c = ' ';
    }
    return out;
}

// Reads one command per line from stdin and writes exactly one response line to
// stdout for each, so a driver in any language can drive the database over a
// pipe. Traces go to stderr, so stdout carries nothing but responses.
// antithesis/CONTRACT.md is the normative description of this protocol.
//
//   get KEY       -> VALUE <value> | NOTFOUND
//   set KEY VALUE -> OK
//   delete KEY    -> OK
//   commit        -> OK
//   exit          -> (no response, exits 0)
//   anything bad  -> ERR <message>
//
// Unlike the one-shot form, set and delete here do NOT commit: they open a
// transaction (taking the file lock) that is held until an explicit commit, so a
// caller can make several writes atomic. Reaching exit or EOF with uncommitted
// writes discards them and releases the lock.
//
// Keys and values must not contain a newline, and a key must not contain a
// space. Nothing else is escaped -- the protocol stays hand-typable in a
// debugger shell, which is the point.
static int repl(DBDB& db)
{
    std::string line;
    while (std::getline(std::cin, line))
    {
        if (!line.empty() && line.back() == '\r') line.pop_back();   // tolerate CRLF

        std::size_t idx = 0;
        const std::string verb = next_token(line, idx);
        if (verb.empty()) continue;                                  // blank line

        try
        {
            if (verb == "exit" || verb == "quit")
            {
                return 0;
            }
            else if (verb == "get")
            {
                const std::string key = next_token(line, idx);
                if (key.empty()) { std::cout << "ERR get requires a key" << std::endl; continue; }
                const std::optional<Bytes> value = db.get(key);
                if (value) std::cout << "VALUE " << to_string(*value) << std::endl;
                else std::cout << "NOTFOUND" << std::endl;
            }
            else if (verb == "set")
            {
                const std::string key = next_token(line, idx);
                if (key.empty() || idx >= line.size())
                {
                    std::cout << "ERR set requires a key and a value" << std::endl;
                    continue;
                }
                db.set(key, to_bytes(rest_of_line(line, idx)));
                std::cout << "OK" << std::endl;
            }
            else if (verb == "delete")
            {
                const std::string key = next_token(line, idx);
                if (key.empty()) { std::cout << "ERR delete requires a key" << std::endl; continue; }
                db.remove(key);
                std::cout << "OK" << std::endl;
            }
            else if (verb == "commit")
            {
                db.commit();
                std::cout << "OK" << std::endl;
            }
            else
            {
                std::cout << "ERR unknown command: " << one_line(verb) << std::endl;
            }
        }
        catch (const std::exception& e)
        {
            // Report and keep going rather than aborting: an abrupt death would
            // read as a crash to anything supervising this process. A failure
            // part-way through a transaction leaves the lock held, so the caller
            // can still retry, commit, or exit.
            std::cout << "ERR " << one_line(e.what()) << std::endl;
        }
    }
    return 0;   // EOF behaves like exit
}

int main(int argc, char* argv[])
{
    if (argc < 2) return usage();
    const std::string dbname = argv[1];

    try
    {
        DBDB db = DBDB::connect<BinaryTree>(dbname);

        if (argc == 2 || (argc == 3 && std::string(argv[2]) == "repl")) return repl(db);
        if (argc < 4) return usage();

        const std::string verb = argv[2];
        const std::string key = argv[3];

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
    }
    catch (const std::exception& e)
    {
        std::cerr << "dbdb: " << e.what() << "\n";
        return 3;
    }
    return 0;
}
