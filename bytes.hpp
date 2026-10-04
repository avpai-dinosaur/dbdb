#pragma once
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

using Bytes = std::vector<std::byte>;

inline Bytes to_bytes(const std::string& s)
{
    const std::byte* p = reinterpret_cast<const std::byte*>(s.data());
    return Bytes(p, p + s.size());
}

inline std::string to_string(const Bytes& b)
{
    return std::string(reinterpret_cast<const char*>(b.data()), b.size());
}

// Appends x to buffer in little-endian order.
inline void put_u64(uint64_t x, Bytes& buffer)
{
    for (size_t i = 0; i < sizeof(uint64_t); ++i) {
        buffer.push_back(static_cast<std::byte>((x >> (i * 8)) & 0xFF));
    }
}

// Reads a little-endian u64 at idx and advances idx past it.
inline uint64_t get_u64(const Bytes& buffer, size_t& idx)
{
    size_t N = sizeof(uint64_t);
    if (idx > buffer.size() || buffer.size() - idx < N)
    {
        throw std::runtime_error("truncated buffer");
    }
    uint64_t out = 0x00;
    for (size_t i = 0; i < N; ++i) {
        out |= static_cast<uint64_t>(buffer[idx]) << (i * 8);
        ++idx;
    }
    return out;
}

// Appends b to buffer as [u64 length][bytes].
inline void put_bytes(const Bytes& b, Bytes& buffer)
{
    put_u64(b.size(), buffer);
    for (size_t i = 0; i < b.size(); ++i) {
        buffer.push_back(b[i]);
    }
}

// Reads a [u64 length][bytes] record at idx and advances idx past it.
inline Bytes get_bytes(const Bytes& buffer, size_t& idx)
{
    uint64_t length = get_u64(buffer, idx);
    if (idx > buffer.size() || buffer.size() - idx < length)
    {
        throw std::runtime_error("truncated buffer");
    }
    Bytes out;
    for (size_t i = 0; i < length; ++i) {
        out.push_back(buffer[idx]);
        ++idx;
    }
    return out;
}
