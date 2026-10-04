#pragma once
#include <cstdint>
#include "bytes.hpp"

using Address = std::uint64_t;

constexpr Address NULL_ADDRESS = 0;

class Storage
{
public:
    virtual ~Storage() = default;
    virtual void lock() = 0;
    virtual void unlock() = 0;
    virtual Address write(const Bytes& data) = 0;
    virtual Bytes read(Address address) = 0;
    virtual void commit_root_address(Address address) = 0;
    virtual Address get_root_address() = 0;
};
