#pragma once

#include <cmath>
#include <concepts>
#include <limits>
#include <type_traits>
#include <utility>

namespace util {
template <std::integral To, std::integral From>
constexpr To lossy_cast(From from) {
    constexpr auto to_min = std::numeric_limits<To>::min();
    constexpr auto to_max = std::numeric_limits<To>::max();

    if (std::cmp_less_equal(from, to_min)) {
        return to_min;
    } else if (std::cmp_greater_equal(from, to_max)) {
        return to_max;
    } else {
        return static_cast<To>(from);
    }
}

template <std::floating_point To, std::floating_point From>
constexpr To lossy_cast(From from) {
    return static_cast<To>(from);
}

template <std::floating_point To, std::integral From>
constexpr To lossy_cast(From from) {
    return static_cast<To>(from);
}

template <std::integral To, std::floating_point From>
constexpr To lossy_cast(From from) {
    constexpr auto to_min = std::numeric_limits<To>::min();
    constexpr auto to_max = std::numeric_limits<To>::max();

    if (std::isnan(from)) {
        return To{0};
    } else if (std::isinf(from)) {
        return (from > From{0}) ? to_max : to_min;
    } else {
        if (from <= static_cast<From>(to_min)) {
            return to_min;
        } else if (from >= static_cast<From>(to_max)) {
            return to_max;
        } else {
            return static_cast<To>(from);
        }
    }
}
}  // namespace util
