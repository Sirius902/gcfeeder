#pragma once

#include <concepts>
#include <limits>
#include <type_traits>

namespace util {
template <std::integral To, std::integral From>
To lossy_cast(From from) {
    constexpr auto to_min = std::numeric_limits<To>::min();
    constexpr auto to_max = std::numeric_limits<To>::max();

    if (std::cmp_less(from, to_min)) {
        return to_min;
    } else if (std::cmp_greater(from, to_max)) {
        return to_max;
    } else {
        return static_cast<To>(from);
    }
}
}  // namespace util
