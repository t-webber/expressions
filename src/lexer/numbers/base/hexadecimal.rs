use crate::errors::{compile::CompileError, location::Location};
#[allow(clippy::wildcard_imports)]
use crate::lexer::numbers::types::arch_types::*;
use crate::lexer::numbers::types::{Number, NumberType, ERR_PREFIX};
use crate::{parse_int_from_radix, to_error, to_warning};

#[derive(Default, Debug)]
struct HexFloatParse {
    int_part: String,
    decimal_part: String,
    exponent: String,
    exponent_neg: Option<bool>,
    state: HexFloatParseState,
}

impl HexFloatParse {
    fn push(&mut self, ch: char) {
        match self.state {
            HexFloatParseState::Int => self.int_part.push(ch),
            HexFloatParseState::Decimal => self.decimal_part.push(ch),
            HexFloatParseState::Exponent => self.exponent.push(ch),
        }
    }

    fn get_exp(&self) -> u32 {
        if self.exponent.is_empty() {
            0
        } else {
            self.exponent
                .parse()
                .expect("Never fails: contains only ascii digits and not empty")
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug)]
enum HexFloatParseState {
    #[default]
    Int,
    Decimal,
    Exponent,
}

trait FloatingPoint<T> {
    const MANTISSA_SIZE: u32;
    type Unsigned;
    fn from_unsigned(val: T, location: &Location, warning: &mut Option<CompileError>) -> Self;
    fn from_usize(val: usize, location: &Location, warning: &mut Option<CompileError>) -> Self;
}

macro_rules! impl_floating_point {
    ($x:expr, $($ftype:ident)*) => {
        $(#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        impl FloatingPoint<concat_idents!($ftype, IntPart)> for $ftype {
            type Unsigned = concat_idents!($ftype, IntPart);

            const MANTISSA_SIZE: u32 = $x;

            fn from_unsigned(
                val: Self::Unsigned,
                location: &Location,
                warning: &mut Option<CompileError>,
            ) -> Self {
                if val >= (2 as Self::Unsigned).pow(Self::MANTISSA_SIZE) {
                    *warning = Some(to_warning!(
                        location,
                        "value overflow, given number will be crapped"
                    ));
                }
                val as Self
            }

            fn from_usize(
                val: usize,
                location: &Location,
                warning: &mut Option<CompileError>,
            ) -> Self {
                if val >= 2usize.pow(Self::MANTISSA_SIZE) {
                    *warning = Some(to_warning!(
                        location,
                        "value overflow, given number will be crapped"
                    ));
                }
                val as Self
            }
        })*
    };
}

impl_floating_point!(23, Float Double LongDouble);

macro_rules! parse_hexadecimal_float {
    ($warning:expr, $location:ident, $nb_type:ident, $float_parse:ident, $($t:ident)*) => {{
        match $nb_type {
            $(NumberType::$t => {
                let int_part = $t::from_unsigned(
                    <concat_idents!($t, IntPart)>::from_str_radix(&$float_parse.int_part, 16).expect("2 <= <= 36"),
                    $location, $warning);
                #[allow(clippy::as_conversions)]
                let exponent = $t::from_unsigned((2 as concat_idents!($t, IntPart)).pow($float_parse.get_exp()), $location, $warning);
                let mut decimal_part: $t = 0.;
                for (idx, ch) in $float_parse.decimal_part.chars().enumerate() {
                    let digit_value = $t::from_unsigned(hex_char_to_int(ch).into(), $location, $warning);
                    let exponent_pow = $t::from(16.).powf($t::from_usize(idx, $location, $warning) + 1.);
                    decimal_part += digit_value / exponent_pow;
                }
                if $float_parse.exponent_neg.unwrap_or(false) {
                    Number::$t((int_part + decimal_part) / exponent)
                } else {
                    Number::$t((int_part + decimal_part) * exponent)
                }
            },)*
            _ => panic!("Never happens: nb_type is float"),
        }
    }};
}

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
        _ => panic!("function called on non hex char"),
    }
}

fn get_hex_float_state(literal: &str, location: &Location) -> Result<HexFloatParse, CompileError> {
    let mut float_parse = HexFloatParse::default();
    for ch in literal.chars() {
        match ch {
            '+' | '-' if float_parse.state != HexFloatParseState::Exponent => panic!("never happens: + or - always are after a p character in hex literal"),
            '+' | '-' if float_parse.exponent_neg.is_some() => Err(to_error!(location, "{ERR_PREFIX}maximum one sign is allowed in a number literal."))?,
            '-' => float_parse.exponent_neg = Some(true),
            '+' => float_parse.exponent_neg = Some(false),
            _ if float_parse.state == HexFloatParseState::Exponent && ch.is_ascii_digit() => float_parse.push(ch),
            _ if float_parse.state == HexFloatParseState::Exponent => Err(to_error!(location, "{ERR_PREFIX}invalid character for exponent. Expected an ascii digit, but found '{ch}'"))?,
            _ if ch.is_ascii_hexdigit() => float_parse.push(ch),
            '.' if float_parse.state == HexFloatParseState::Int => float_parse.state = HexFloatParseState::Decimal,
            '.' if float_parse.state == HexFloatParseState::Decimal  => Err(to_error!(location, "{ERR_PREFIX}maximum one '.' in number constant, but 2 were found."))?, 
            '.' if float_parse.state == HexFloatParseState::Exponent  => Err(to_error!(location, "{ERR_PREFIX}exponent must be an integer, but found a period."))?, 
            'p' | 'P' if float_parse.state == HexFloatParseState::Exponent => Err(to_error!(location, "{ERR_PREFIX}maximum one 'p' in number constant, but 2 were found."))?, 
            'p' | 'P'  => float_parse.state = HexFloatParseState::Exponent,
            _ => Err(to_error!(location, "{ERR_PREFIX}invalid character '{ch}' found in number constant"))?, 
        };
    }
    Ok(float_parse)
}

#[allow(clippy::panic_in_result_fn)]
pub fn to_hex_value(
    literal: &str,
    nb_type: &NumberType,
    location: &Location,
) -> Result<Number, CompileError> {
    let float_parse = get_hex_float_state(literal, location)?;
    if nb_type.is_int() {
        parse_int_from_radix!(location,
           nb_type, literal, "never fails", 16, Int Long LongLong UInt ULong ULongLong
        )
    } else {
        let mut warning: Option<CompileError> = None;
        #[allow(clippy::float_arithmetic, clippy::wildcard_enum_match_arm)]
        Ok(
            parse_hexadecimal_float!(&mut warning, location, nb_type, float_parse, Float Double LongDouble),
        )
    }
}
