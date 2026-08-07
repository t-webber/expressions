crate::ast!(

unary_binary: "z -= a +~ b * (c *= 2) - d / e % f + g - h * i + j % k * l ^ !m++ & n | o || p && q"

ternary_blocks: "z &= (a /= z) * (b %= y) + c - d / e % f * g + h & i | j ^ k && l ||
        m * n + o - p * q / r + s % t
        ? u
        : v && w ^ x | y && z != 2; ! a>>b"

assign: "a + b >= 0 ? (c ^= 0) * !(e|=1) : (d >>= x[3])"

add_assign: "x += 1"

incr_comment: "a*b++/*x*/"

unfinished_ternary: "a ? b"

cast_list_initialiser: "(int*){1, 2, (int)PI}"

ternary_function: "a ? f(x) : o(y)"

ternary_cast: "a ? (int)f : (void*)o"

ternary_cast_function: "a ? (int)f(x) : (void*)o(y, z)"

missing_rhs_binary: "a + << b"

);
