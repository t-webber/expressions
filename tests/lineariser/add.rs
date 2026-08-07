crate::ssa!(

pointer_int: "&1 + 2"
pointer_int_rev: "1 + &2"
pointer_val: "int x; &x + 2"
pointer_complex: "_Complex int x; &1 + x"
pointer_decimal: "_Decimal32 x; &1 + x"

complex: "_Complex int x; x + 1"
imaginary: "_Imaginary int x; x + 1"
imaginary_and_imaginary: "_Imaginary int x; x + x"

void: "void x; x + 1"

decimal_float: "_Decimal32 x; float y; x+y"
decimal_128: "_Decimal128 x; x+x"
decimal_32: "_Decimal32 x; x+x"
decimal_64_32: "_Decimal64 x; _Decimal32 y; x+y"

long_double: "long double x; x + 1"
double_float: "double x; float y; x + y"
float_float: "float x; x+x"

int_char_plus_one: "char x; x + 1"
int_bool_plus_char: "bool x; char y; x + y"
int_short_short: "short int x; short int y; x + y"

int_signed_unsigned: "int x; unsigned int y; x + y"
long_signed_unsigned: "long int x; unsigned long int y; x + y"
longlong_signed_unsigned: "long long int x; unsigned long long int y; x + y"

longlong_plus_long: "long long int x; long int y; x + y"
longlong_plus_ulong: "long long int x; unsigned long int y; x + y"
ulonglong_plus_long: "unsigned long long int x; long int y; x + y"

long_plus_int: "long int x; int y; x + y"
ulong_plus_int: "unsigned long int x; int y; x + y"
longlong_plus_uint: "long long int x; unsigned int y; x + y"
huge_int: "unsigned long long int x; 18446744073709551615ULL + x"

);
