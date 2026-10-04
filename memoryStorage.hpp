#pragma once
#include<mutex>
#include "storage.hpp"

class MemoryStorage : public Storage
{
private:
    std::mutex m;
    std::vector<Bytes> blobs;
    Address root = NULL_ADDRESS;
public:
    Address write(const Bytes& data) override { blobs.push_back(data); return blobs.size(); }
    Bytes read(Address address) override { return blobs.at(address - 1); }
    void commit_root_address(Address address) override { root = address; }
    Address get_root_address() override { return root; }
    void lock() override { m.lock(); }
    void unlock() override { m.unlock(); }
};
