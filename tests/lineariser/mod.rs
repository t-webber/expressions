//! Lineariser tests.

mod add;
mod literals;
mod op;

crate::ssa!(

simple_definition: "int x = 2;"

simple_declaration: "int y;"

scoped_redeclaration: "int y; { int y = 2; }"

unscoped_redefinition: "{ int y = 2; int y = 3; }"

definition_after_declaration: "int y; int y = 2;"

definition_wrong_type: "int y; char y = 2;"

declaration_wrong_type: "int y = 2; char y;"

multiple_declarations: "int x; int x = nullptr; int x; int x;"

fn_decl: "const char* func(static volatile int** first_argument, struct custom * arg2)"

function_definition: "int**f(int v) {}"

function_def_after_decl: "int f(int v); int f(int v) {} int f(int v);"

function_redefinition: "int f(int v); int f(int v) {} int f(int v) {}"

function_def_wrong_type: "int f(int v); char f(int v) {}"

function_decl_wrong_type: "int f(int v) {} char f(int v)"

function_shadow_variable: "int f; int f(int v);"

variable_shadow_function: "int f(bool v); int f;"

function_return: "int glob() { return glob(2); }"

hello_world: r#"void printf(const char* s); int main() { printf("Hello, world!"); return 1; }"#

check_id_not_skipped: "int x; int x = 2; int x; int y;"

call_undeclared: "int f() { g(1) }"

var_def_in_fn: "void main() { int x; }"

set_to_other: "a b() { c d; e f = d; }"

missing_ret_ty: "b() { int x; }"

fn_sizeof: "sizeof() { int x; }"

fn_kwd_0_args: "sizeof(); alignof(); static_assert();"

fn_kwd_too_many_args: "sizeof(1, 2, 3); alignof(4, 5); static_assert(6, 7, 8, 9);"

use_undeclared: "a b() { c d = e; }"

same_literal_assigned: "char x = 'a'; char y = 'a';"

binary_fn: "a b(int x, int y) { return x+y; }"

binary_undeclared: "int f() {return x+y;}"

use_within: "int x, y = 1, z = x+y;"

binary_valid: "int x, y = 1; int z = x+y;"

fn_wrong_args: "void fn(1)"

fn_no_type: "void fn(blob)"

fn_arg_kw: "void fn(sizeof)"

fn_arg_no_name: "void fn(const)"

call_invalid_decl: "void fn(const, sizeof, blob); fn(1, 2, 3)"

fn_multiple_args_same_name: "void fn(int a, int a, char a);"

ternary_unary: "int a = 1 ? !3 : 4"

ternary_unary_not_found: "int a = 1 ? !b : 4; int c = a;"

ternary_no_question: "int a = 1 ? 2!"

ternary_no_failure: "int a = 1 ? 2! : "

bin_missing_arg: "int a = 1 << "

comma_in_function_decl: "int a, b() {}"

bitfield_var: "int a:2"

function_call_not_arg: "int a(); a(int)"

binary_statement: "0 + int"

ternary_statement : "0 ? int : int"

unary_statement: "!int"

function_call_invalid: "void f(int x, int y); f(2+, 3);"

decl_statement: "int x = +"

return_not_found: "void f() { return g(); }"

return_invalid: "void f() { return int; }"

use_fn_kwd_as_leaf: "int x = sizeof"

unsigned_minus: "const unsigned int a; -a;"

unaries: "int a; !a; &a; a++; a--; --a; ++a; -a; +a; ~a"

dereference_non_pointer: "int a; *a"

pointer_bitwise_not: "~&0"

prefix_incr_not: "int a; ++!a"

postfix_incr_not: "int a; !a++"

multiple_type_names: "int char a b"

struct_on_builtin: "struct int"

struct_on_enum: "struct enum a"

struct_ident_val: "struct a = 0"

todo_attr: "_Generic alignas _BigInt typeof typeof_unqual static int x = 1"

indirection_end_name: "int ** a = 0; *a"

sizeof_type: "sizeof(int)"

dup_qual: "const int const x = 0"

dup_mod: "signed int signed x = 0"

dup_ind_attr: "int *restrict restrict x = 0"

long_long_long: "long unsigned long short long int x = 1"

long1_long1_long2: "long unsigned long short
long int x = 1"

long1_long2_long2: "long unsigned
long short long int x = 1"

long1_long2_long3: "long unsigned
long short
long int x = 1"

complex_decimal: "_Complex const _Decimal128"

complex_decimal_2lines: "_Complex const
_Decimal128 x"

imaginary_decimal: "_Imaginary short _Decimal32"

fn_attr_after_indirection: "_Noreturn int * inline f();"

union: "union A x = 1;"

enum_: "enum A x = 1;"

atomic: "_Atomic int x = 1;"

fn_attr_in_var: "int inline x = 1"

complex_struct: "_Complex struct A b = 2"

fn_arg_shadows_global: "int x = 1; int f(char x);"

non_top_level_fn: "int f() { int g(); }"

fn_two_returns: "int f() { return 1; return 2; }"

parens_none: "()"

parens_expr: "1 * (2 + 3)"

);
