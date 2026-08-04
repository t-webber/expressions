//! Module that tries to convert a string into a valid constant C number,
//! whatever the size, type and encoding base.

use core::str;

use super::base::{binary, decimal, hexadecimal, octal};
use super::types::arch_types::Int;
use super::types::{Base, ERR_PREFIX, Number, NumberSign, NumberType};
use crate::Res;
use crate::errors::api::{ErrorLocation, LocationPointer};
use crate::lexer::types::api::{Ident, LexingData};

/// Whether the type is a double or a float.
enum DoubleType {
    /// Double type (no leading `f`).
    Double,
    /// Float.
    Float,
    /// Integer.
    Int,
}

impl DoubleType {
    /// Makes the double a float.
    fn into_float(self) -> Result<Self, String> {
        Err(match self {
            Self::Double => return Ok(Self::Float),
            Self::Float => format!("{ERR_PREFIX}found 2 'f' characters, but only 1 is needed."),
            Self::Int => format!(
                "{ERR_PREFIX}a 'f' suffix only works on `double` constants. Please insert a full stop or an 'e' exponent character before the 'f'."
            ),
        })
    }
}

/// Number of `long` ident read.
enum LongCount {
    /// no `long`
    None,
    /// `long`
    One,
    /// `long long`
    Two,
}

impl LongCount {
    /// Increments the number `long` idents that were found.
    const fn increment(&mut self) -> Option<&'static str> {
        *self = match self {
            Self::None => Self::One,
            Self::One => Self::Two,
            Self::Two => return Some("found 3 'l' characters, but max is 2 (`long long`)."),
        };
        None
    }
}

/// Finds the base of the number constant by looking at the prefix
///
/// # Returns
///
/// This function returns
///
/// - [`Base::Binary`] if the literal starts with "0b";
/// - [`Base::Hexadecimal`] if the literal starts with "0x";
/// - [`Base::Octal`] if the literal starts with "0";
/// - [`Base::Decimal`] in every other case.
fn as_base(literal: &str, nb_type: NumberType, location: ErrorLocation) -> Res<Base> {
    let mut chars = literal.chars();
    let first = chars.next().expect("len >= 1");
    let second = chars.next().expect("len >= 2");

    match (first, second) {
        ('0', 'x') => Res::ok(Base::Hexadecimal),
        ('0', 'b') if nb_type.is_int() => Res::ok(Base::Binary),
        ('0', 'b') => location
            .fail(format!("{ERR_PREFIX}a binary must be an integer."))
            .into_res(),
        ('0', '0'..='7') if nb_type.is_int() => Res::ok(Base::Octal),
        ('0', ch) if nb_type.is_int() => location
            .fail(format!("{ERR_PREFIX}found invalid character '{ch}' in octal base."))
            .into_res(),
        _ => Res::ok(Base::Decimal),
    }
}

/// Finds an invalid character with the base found with the prefix of the
/// constant.
///
/// # Examples
///
/// ```ignore
/// assert!(as_first_invalid_char("1032", &Base::Binary) == Some('3'));
/// assert!(as_first_invalid_char("1032", &Base::Octal) == None);
/// ```
fn as_first_invalid_char(literal: &str, base: &Base) -> Option<char> {
    let mut chars = literal.chars();
    match base {
        Base::Binary => chars.find(|ch| !matches!(ch, '0' | '1')),
        Base::Decimal => chars.find(|ch| !matches!(ch, '0'..='9' | '.' | 'e' | 'E' | '+' | '-')),
        Base::Hexadecimal =>
            chars.find(|ch| !ch.is_ascii_hexdigit() && !matches!(ch, '.' | 'p' | 'P' | '+' | '-')),
        Base::Octal => chars.find(|ch| !ch.is_ascii_octdigit()),
    }
}

/// Gets the type of the number constant by looking at the suffix.
///
/// # Returns
///
/// This functions returns a [`NumberType`], that is computed with the following
/// rules:
///
/// - a 'l' suffix means `Long`, 'll' means `Long Long`;
/// - a 'u' suffix means 'Unsigned';
/// - the suffix is case insensitive;
/// - you can combine the rules: 'ul' is `ULong` (`unsigned long`).
///
/// # Errors
///
/// This functions returns an error if
///
/// - there are multiple 'u' in the suffix;
/// - if there is a 'i' suffix (for complex numbers);
/// - there are more than 2 'l's in the suffix.
fn as_number_type(literal: &str, location: ErrorLocation) -> Res<NumberType> {
    let is_hex = literal.starts_with("0x");

    if is_hex && literal.contains('.') && !literal.contains(['p', 'P']) {
        return location
            .fail(
                "Hexadecimal float must contain exponent after full stop. Please add missing 'p'."
                    .to_owned(),
            )
            .into_res();
    }

    // will be computed below
    let chars = literal.chars().rev();
    let mut l_count = LongCount::None;
    let mut unsigned = false;
    let mut double = if literal.contains('.')
        || (is_hex && (literal.contains(['p', 'P'])))
        || (!is_hex && (literal.contains(['e', 'E'])))
    {
        DoubleType::Double
    } else {
        DoubleType::Int
    };

    for ch in chars {
        match ch {
            'u' | 'U' if unsigned => {
                return location
                    .fail("found 2 'u' characters.".to_owned())
                    .into_res();
            }
            'u' | 'U' => unsigned = true,
            'l' | 'L' =>
                if let Some(err) = l_count.increment() {
                    return location.fail(err.to_owned()).into_res();
                },
            'f' | 'F' if is_hex && matches!(double, DoubleType::Int) => break,
            'f' | 'F' => match double.into_float() {
                Ok(new) => double = new,
                Err(err) => return location.fail(err).into_res(),
            },
            'i' | 'I' =>
                return location
                    .fail("imaginary constants are a GCC extension.".to_owned())
                    .into_res(),
            _ => break,
        }
    }

    // get the type from the characteristics
    let err = match (double, unsigned, l_count) {
        (DoubleType::Int, false, LongCount::None) => return Res::ok(NumberType::Int),
        (DoubleType::Int, false, LongCount::One) => return Res::ok(NumberType::Long),
        (DoubleType::Int, false, LongCount::Two) => return Res::ok(NumberType::LongLong),
        (DoubleType::Int, true, LongCount::None) => return Res::ok(NumberType::UInt),
        (DoubleType::Int, true, LongCount::One) => return Res::ok(NumberType::ULong),
        (DoubleType::Int, true, LongCount::Two) => return Res::ok(NumberType::ULongLong),
        (DoubleType::Double, false, LongCount::None) => return Res::ok(NumberType::Double),
        (DoubleType::Double, false, LongCount::One) => return Res::ok(NumberType::LongDouble),
        (DoubleType::Double, false, LongCount::Two) =>
            format!("{ERR_PREFIX}`long long double` doesn't exist."),
        (DoubleType::Double, true, _) => format!("{ERR_PREFIX}a `double` can't be `unsigned`."),
        (DoubleType::Float, false, LongCount::None) => return Res::ok(NumberType::Float),
        (DoubleType::Float, false, LongCount::One | LongCount::Two) => format!(
            "{ERR_PREFIX}a `float` can't be `long`. Did you mean `long double`? Remove the leading 'f' if that is the case."
        ),
        (DoubleType::Float, true, _) => format!("{ERR_PREFIX}a `float` can't be `unsigned`."),
    };
    location.fail(err).into_res()
}

/// Functions to try parse a literal into a number.
///
/// # Returns
///
/// - `Some(number)` if literal is a number
/// - `None` otherwise
///
/// # Errors
///
/// This function doesn't return any errors, but writes them directly to
/// `lex_data` (cf. [`LexingData`]).
pub fn literal_to_number(
    lex_data: &mut LexingData,
    literal: &Ident,
    location: &LocationPointer,
) -> Option<Number> {
    if literal.is_empty() || !literal.is_number() {
        return None;
    }

    if literal.len() == 1 {
        return Some(Number::Int(literal.value().parse::<Int>().expect("one char")));
    }

    let len = literal.len();
    let begin_location = location.to_past(len, len);

    literal_to_number_err(literal.value(), begin_location, lex_data.last_is_minus())
        .store_errors(&mut |err| lex_data.push_err(err))
}

/// Tried to convert a literal to a number by computing the exact base and size.
///
/// If the size isn't big enough, the compiler returns a warning and tried to
/// increase the size (cf. [`NumberType::incr_size`]).
fn literal_to_number_err(literal: &str, location: ErrorLocation, signed: bool) -> Res<Number> {
    as_number_type(literal, location).and_then(|mut nb_type| {
    as_base(literal, nb_type, location).and_then(|base| {


    let value = literal
        .get(base.prefix_size()..literal.len().checked_sub(nb_type.suffix_size()).expect("literal contains the suffix"))
        .expect("never happens as suffix size + prefix size <= len, as 'x' and 'b' can't be used as suffix");

    if value.is_empty() {
        return Res::from_err(location.fail(format!(
            "{ERR_PREFIX}found no digits between prefix and suffix. Please add at least one digit.",
        )));
    }

    if let Some(ch) = as_first_invalid_char(value, &base) {
        return Res::from_err(
            location
                .fail(format!("{ERR_PREFIX}found invalid character '{ch}' in {base} base.")),
        );
    }

    let mut error = None;
    let sign = match (nb_type.is_unsigned(), signed) {
        (true, true) => {
            error = Some(location.warn("Found an unsigned constant after a negative sign. Consider removing the `u` prefix.".to_owned()));
            NumberSign::Unsigned
        }
        (true, false) => NumberSign::Unsigned,
        (false, true) => NumberSign::Signed,
        (false, false) => NumberSign::None,
    };

    loop {
        let parse_res = match base {
            Base::Binary => binary::to_bin_value(value, nb_type, location),
            Base::Decimal => decimal::to_decimal_value(value, nb_type, location),
            Base::Hexadecimal => hexadecimal::to_hex_value(value, nb_type, location),
            Base::Octal => octal::to_oct_value(value, nb_type, location),
        };
        if parse_res.overflowed()
            && let Some(new_type) = nb_type.incr_size(sign)
        {
            nb_type = new_type;
        } else if let Some(err) = error {
            return parse_res.ignore_overflow(literal, location).add_err(err);
        } else {
            return parse_res.ignore_overflow(literal, location);
        }
    }

})
})
}
