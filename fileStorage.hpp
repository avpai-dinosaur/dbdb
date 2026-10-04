#pragma once
#include <fstream>
#include <string>
#include "storage.hpp"

class FileStorage : public Storage
{
private:
    std::fstream file;
    int lockFd = -1; 
    void write_u64(std::uint64_t n);
    std::uint64_t read_u64();
public:
    explicit FileStorage(const std::string& path);
    ~FileStorage() override;
    FileStorage(const FileStorage&) = delete;              // copying would close lockFd twice
    FileStorage& operator=(const FileStorage&) = delete;
    void lock() override;
    void unlock() override;
    Address write(const Bytes& data) override;
    Bytes read(Address address) override;
    void commit_root_address(Address address) override;
    Address get_root_address() override;
};
