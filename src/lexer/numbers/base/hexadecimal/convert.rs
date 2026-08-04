use crate::lexer::numbers::base::hexadecimal::parse::HexFloatData;
use crate::lexer::numbers::types::arch_types::{
    Double, DoubleIntPart, Float, FloatIntPart, LongDouble, LongDoubleIntPart
};
use crate::lexer::numbers::types::{Number, NumberType};

/// Parses the stringified version of a number into a [`HexFloatData`].
macro_rules! parse_hexadecimal_float {
    ($overflow:expr, $nb_type:ident, $float_parse:ident, $($t:ident)*) => {{
        #[expect(clippy::float_arithmetic, clippy::arithmetic_side_effects, clippy::as_conversions, reason="todo")]
        match $nb_type {
            $(NumberType::$t => {
                let int_part = $t::from_unsigned(
                    ${concat($t, IntPart)}::from_str_radix(&$float_parse.int_part, 16).unwrap(),
                    $overflow);
                let exponent = $t::from_unsigned((2 as ${concat($t, IntPart)}).pow($float_parse.as_exp()?), $overflow);
                let mut decimal_part: $t = 0.;
                for (idx, ch) in $float_parse.decimal_part.chars().enumerate() {
                    let digit_value = $t::from_unsigned(hex_char_to_int(ch).into(), $overflow);
                    let exponent_pow = $t::from(16f32).powf($t::from_usize(idx, $overflow) + 1.);
                    decimal_part += digit_value / exponent_pow;
                }
                if $float_parse.exponent_neg.unwrap_or(false) {
                   Number::$t((int_part + decimal_part) / exponent)
                } else {
                    Number::$t((int_part + decimal_part) * exponent)
                }
            },)*
            _ => unreachable!("Never happens: nb_type is float"),
        }
    }};
}

/// Implements the [`FloatingPoint`] for the floating-point types.
macro_rules! impl_floating_point {
    ($x:expr, $($type:ident)*) => {
        $(#[allow(clippy::as_conversions, clippy::cast_precision_loss, clippy::allow_attributes, reason="todo")]
        impl FloatingPoint<${concat($type, IntPart)}> for $type {
            const MANTISSA_SIZE: u32 = $x;

            type Unsigned = ${concat($type, IntPart)};


            fn from_unsigned(
                val: Self::Unsigned,
                overflow: &mut bool,
            ) -> Self {
                if val >= (2 as Self::Unsigned).pow(Self::MANTISSA_SIZE) {
                    *overflow = true;
                }
                val as Self
            }

            fn from_usize(
                val: usize,
                overflow: &mut bool,
            ) -> Self {
                if val >= 2usize.pow(Self::MANTISSA_SIZE) {
                    *overflow = true;
                }
                val as Self
            }
        })*
    };
}

/// Trait to try and convert the integer and decimal part inside the mantissa.
///
/// ``overflow`` is set to true if the value doesn't fix in the mantissa.
trait FloatingPoint<T> {
    /// Size of the mantissa
    ///
    /// In the binary representation of the floating-point
    /// values, there is one part for the exponent, and one point for the
    /// digits, the latter is called 'mantissa'.
    const MANTISSA_SIZE: u32;
    /// The biggest unsigned integer type that can contain the mantissa.
    type Unsigned;
    /// Convert the integer-parsed value into the current floating-point type.
    fn from_unsigned(val: T, overflow: &mut bool) -> Self;
    /// Convert the usize-parsed value into the current floating-point type.
    fn from_usize(val: usize, overflow: &mut bool) -> Self;
}

impl_floating_point!(23, Double Float LongDouble);

/// Parsed an hexadecimal float.
///
/// This is a wrapper for float handling. See [`parse_hexadecimal_float`] for
/// more detail.
pub fn to_hex_float_value(
    overflow: &mut bool,
    nb_type: NumberType,
    float_data: &HexFloatData,
) -> Result<Number, String> {
    Ok(parse_hexadecimal_float!(overflow, nb_type, float_data, Float Double LongDouble))
}

/// Converts a hexadecimal digit to its value.
///
/// # Panics
///
/// This function panics if the char is not a valid hexadecimal digits.
///
/// # Examples
///
/// ```ignore
/// assert!(hex_char_to_int('f') == 15);
/// ```
///
/// ```ignore,should_panic
/// hex_char_to_int('p'); // this panics
/// ```
#[coverage(off)]
fn hex_char_to_int(ch: char) -> u8 {
    match ch {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'a' | 'A' => 10,
        'b' | 'B' => 11,
        'c' | 'C' => 12,
        'd' | 'D' => 13,
        'e' | 'E' => 14,
        'f' | 'F' => 15,
        _ => unreachable!("function called on non hex char"),
    }
}
