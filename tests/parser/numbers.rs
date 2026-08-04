crate::ast!(

two_float: "0.2ff"

plus_trigraph: "+??'"

empty_digit: "0x"

signed_unsigned: "-1u"

overflow: "0xffffffffffffffffffffffffffffffffffffffffffffff"

overflow_warning: "0xffffffffffff.fp2"

invalid_exponent: "0xf.fpa"

empty_hex: "0x"

invalid_char_octal: "08"

invalid_char_decimal: "2b"

invalid_char_hexadecimal: "0xg"

invalid_char_bin: "0b4"

float_binary: "0b1."

long_float: "0.fl"

float_not_double: "0f"

hex_float_without_exp: "0xf.f"

overflow_exp: "0x0.0p999999999999999999"

empty_exp: "0x0.0p"

overflow_unsigned: "999999999999999999999u
    -999999999999999999999"

invalid_suffix: "1uu
2lll
3i
4.ll
5.l
6.fu
7.u
"

);
