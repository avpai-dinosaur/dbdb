#include <atomic>
#include <chrono>
#include <cstdio>
#include <thread>
#include "doctest.h"
#include "../src/fileStorage.hpp"

static const char* TEST_DB = "test_fileStorage.db";

TEST_CASE("new file starts empty and write never returns NULL_ADDRESS")
{
    std::remove(TEST_DB);
    {
        FileStorage storage(TEST_DB);
        CHECK(storage.get_root_address() == NULL_ADDRESS);
        Address addr = storage.write(to_bytes("hello"));
        CHECK(addr != NULL_ADDRESS);
        CHECK(to_string(storage.read(addr)) == "hello");
    }
    std::remove(TEST_DB);
}

TEST_CASE("lock blocks a second client until the first unlocks")
{
    std::remove(TEST_DB);
    {
        FileStorage a(TEST_DB);
        FileStorage b(TEST_DB);
        std::atomic<bool> bGotLock = false;

        a.lock();
        std::thread t([&] { b.lock(); bGotLock = true; b.unlock(); });

        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        CHECK_FALSE(bGotLock.load());   // b must still be waiting

        a.unlock();
        t.join();
        CHECK(bGotLock.load());
    }
    std::remove(TEST_DB);
}
