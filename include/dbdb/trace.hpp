#pragma once
#include <cstdio>
#include <initializer_list>
#include <string>
#include <type_traits>
#include <unistd.h>
#include "bytes.hpp"

namespace dbdb::trace
{

inline std::string preview(const std::string& s)
{
    constexpr size_t LIMIT = 40;
    std::string out;
    for (size_t i = 0; i < s.size() && i < LIMIT; ++i)
    {
        out += (s[i] >= 0x20 && s[i] < 0x7F && s[i] != '"') ? s[i] : '.';
    }
    if (s.size() > LIMIT) out += "...";
    return out;
}

struct Field
{
    const char* name;
    std::string value;

    template <typename T, typename = std::enable_if_t<std::is_integral_v<T>>>
    Field(const char* n, T v) : name(n), value(std::to_string(v)) {}
    Field(const char* n, const std::string& v) : name(n), value('"' + preview(v) + '"') {}
    Field(const char* n, const Bytes& v) : name(n), value('"' + preview(to_string(v)) + '"') {}
};

inline void emit(const char* event, std::initializer_list<Field> fields)
{
    std::string record = "[dbdb]: ";
    record += event;
    record += " pid=" + std::to_string(::getpid());
    for (const Field& field : fields)
    {
        record += ' ';
        record += field.name;
        record += '=';
        record += field.value;
    }
    record += '\n';
    std::fwrite(record.data(), 1, record.size(), stderr);
}

}
