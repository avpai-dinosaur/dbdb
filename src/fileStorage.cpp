#include "dbdb/fileStorage.hpp"
#include <cerrno>
#include <cstring>
#include <stdexcept>
#include <fcntl.h>
#include <sys/file.h>
#include <unistd.h>

FileStorage::FileStorage(const std::string& path)
{
    file.open(path, std::ios::in | std::ios::out | std::ios::binary);
    if (!file.is_open())
    {
        std::ofstream create(path, std::ios::binary);
        create.close();
        file.open(path, std::ios::in | std::ios::out | std::ios::binary);
    }
    if (!file.is_open()) throw std::runtime_error("could not open " + path);

    lockFd = ::open(path.c_str(), O_RDWR);
    if (lockFd < 0) throw std::runtime_error("could not open " + path + " for locking: " + std::strerror(errno));

    // Protect against clients creating the same new file.
    lock();
    file.seekg(0, std::ios::end);
    if (file.tellg() == 0) commit_root_address(NULL_ADDRESS);
    unlock();
}

FileStorage::~FileStorage()
{
    if (lockFd >= 0) ::close(lockFd); 
}

void FileStorage::write_u64(std::uint64_t n)
{
    file.write(reinterpret_cast<const char*>(&n), sizeof(n));
}

std::uint64_t FileStorage::read_u64()
{
    std::uint64_t n = 0;
    file.read(reinterpret_cast<char*>(&n), sizeof(n));
    return n;
}

void FileStorage::lock()
{
    // TODO: This is currently a spin lock
    while (::flock(lockFd, LOCK_EX) != 0)
    {
        if (errno != EINTR) throw std::runtime_error(std::string("flock failed: ") + std::strerror(errno));
    }
}

void FileStorage::unlock()
{
    ::flock(lockFd, LOCK_UN);
}

Address FileStorage::write(const Bytes& data)
{
    file.clear();
    file.seekp(0, std::ios::end);
    Address address = file.tellp();
    write_u64(data.size());
    file.write(reinterpret_cast<const char*>(data.data()), data.size());
    file.flush();
    return address;
}

Bytes FileStorage::read(Address address)
{
    file.clear();
    file.seekg(address);
    Bytes data(read_u64());
    file.read(reinterpret_cast<char*>(data.data()), data.size());
    return data;
}

void FileStorage::commit_root_address(Address address)
{
    file.clear();
    file.seekp(0);
    write_u64(address);
    file.flush();
}

Address FileStorage::get_root_address()
{
    file.clear();
    file.seekg(0);
    return read_u64();
}
