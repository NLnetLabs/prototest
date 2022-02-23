//! Encoding data using DER.

use super::core;
use super::core::{AppendData, Recipe};


//============ High-level Recipes ============================================

//------------ boolean -------------------------------------------------------

/// Returns a recipe for writing a DER-encoded boolean.
pub fn boolean(x: bool) -> Recipe {
    value(universal(false, 1), 
        if x {
            core::literal([0xFF]).into()
        }
        else {
            core::literal([0x00]).into()
        }
    )
}


//------------ integer -------------------------------------------------------

/// Returns a recipe for writing a DER-encoded integer.
pub fn integer(int: impl Integer) -> Recipe {
    value(universal(false, 2), IntegerContent(int).into())
}

/// Returns a recipe for writing an implicitely tagged integer.
///
/// The recipe will implicitely tag the integer as context specific with the
/// given tag number.
pub fn context_integer(tag_number: u128, int: impl Integer) -> Recipe {
    value(
        tag(Class::Context, false, tag_number),
        IntegerContent(int).into()
    )
}

struct IntegerContent<I>(I);

impl<I: Integer> AppendData for IntegerContent<I> {
    fn append(&self, target: &mut Vec<u8>) {
        self.0.append_integer(target)
    }
}

pub trait Integer: 'static {
    fn append_integer(&self, target: &mut Vec<u8>);
}

impl Integer for u8 {
    fn append_integer(&self, target: &mut Vec<u8>) {
        if *self > 127 {
            target.push(0);
        }
        target.push(*self);
    }
}

// XXX Add impls for all built-in integer types.


fn append_integer_str(_s: &str, _target: &mut Vec<u8>) {
    // XXX Treat self as a Rust integer literal of indefinite length.
    unimplemented!()
}

impl Integer for &'static str {
    fn append_integer(&self, target: &mut Vec<u8>) {
        append_integer_str(self, target);
    }
}

impl Integer for String {
    fn append_integer(&self, target: &mut Vec<u8>) {
        append_integer_str(self.as_str(), target);
    }
}

impl<const N: usize> Integer for [u8; N] {
    fn append_integer(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.as_ref())
    }
}

impl Integer for &'static [u8] {
    fn append_integer(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self)
    }
}

impl Integer for Vec<u8> {
    fn append_integer(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self)
    }
}


//------------ bitstring -----------------------------------------------------

/// Returns a recipe for writing a DER-encoded bitstring.
///
/// The content of the bit string will be taken from whatever the _content_
/// recipe produces. The number of unused bits in the last octet of the
/// content is given via _unused_. Note that for a correctly encoded bit
/// string, the unused bits need to be zero. The recipe does _not_ ensure they
/// are.
///
/// Naturally, _unused_ cannot be larger than 7. However, in order to make
/// it possible to create broken values, the recipe does not check this
/// either.
pub fn bitstring(unused: u8, content: Recipe) -> Recipe {
    value(universal(false, 3), core::sequence([[unused].into(), content]))
}


//------------ octetstring ---------------------------------------------------

/// Returns a recipe for writing the given content as DER octet string.
pub fn octetstring(content: Recipe) -> Recipe {
    value(universal(false, 4), content)
}


//------------ null ----------------------------------------------------------

/// Returns a recipe for a DER-encoded null value.
pub fn null() -> Recipe {
    value(universal(false, 5), [].into())
}


//------------ sequence ------------------------------------------------------

/// Returns a recipe for writing a values as a DER sequence.
pub fn sequence<const N: usize>(items: [Recipe; N]) -> Recipe {
    value(universal(true, 16), core::sequence(items))
}


//------------ set -----------------------------------------------------------

/// Returns a recipe for writing a values as a DER set.
pub fn set<const N: usize>(items: [Recipe; N]) -> Recipe {
    value(universal(true, 17), core::sequence(items))
}


//------------ oid -----------------------------------------------------------

/// Returns a recipe for writing an object identifier.
pub fn oid<const N: usize>(items: [u128; N]) -> Recipe {
    value(universal(false, 6), Oid(items).into())
}

struct Oid<const N: usize>([u128; N]);

impl<const N: usize> AppendData for Oid<N> {
    fn append(&self, target: &mut Vec<u8>) {
        append_base_7((self.0[0] * 40) + self.0[1], target);
        for value in &self.0[2..] {
            append_base_7(*value, target)
        }
    }
}


//------------ printable_string ----------------------------------------------

/// Returns a recipe for writing the given content as PrintableString.
pub fn printable_string(content: impl Into<Recipe>) -> Recipe {
    value(universal(false, 19), content.into())
}


//------------ ia5_string ----------------------------------------------------

/// Returns a recipe for writing the given content as IA5String.
pub fn ia5_string(content: Recipe) -> Recipe {
    value(universal(false, 22), content)
}


//------------ utc_time ------------------------------------------------------

/// Returns a recipe for writing a time as a UTCTime value.
pub fn utc_time(time: impl UtcTime) -> Recipe {
    value(universal(false, 23), UtcTimeContent(time).into())
}

struct UtcTimeContent<T>(T);

impl<T: UtcTime> AppendData for UtcTimeContent<T> {
    fn append(&self, target: &mut Vec<u8>) {
        self.0.append_utc_time(target)
    }
}

pub trait UtcTime: 'static {
    fn append_utc_time(&self, target: &mut Vec<u8>);
}

impl UtcTime for &'static str {
    fn append_utc_time(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.as_bytes())
    }
}

impl UtcTime for String {
    fn append_utc_time(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.as_bytes())
    }
}

#[cfg(feature = "chrono")]
impl UtcTime for chrono::DateTime<chrono::offset::Utc> {
    fn append_utc_time(&self, target: &mut Vec<u8>) {
        use std::io::Write;

        write!(target, "{}", self.format("%y%m%d%H%M%SZ")).unwrap();
    }
}


//------------ generalized_time ----------------------------------------------

/// Returns a recipe for writing a time as a GeneralizedTime value.
pub fn generalized_time(time: impl GeneralizedTime) -> Recipe {
    value(universal(false, 24), GeneralizedTimeContent(time).into())
}

struct GeneralizedTimeContent<T>(T);

impl<T: GeneralizedTime> AppendData for GeneralizedTimeContent<T> {
    fn append(&self, target: &mut Vec<u8>) {
        self.0.append_generalized_time(target)
    }
}

pub trait GeneralizedTime: 'static {
    fn append_generalized_time(&self, target: &mut Vec<u8>);
}

impl GeneralizedTime for &'static str {
    fn append_generalized_time(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.as_bytes())
    }
}

impl GeneralizedTime for String {
    fn append_generalized_time(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.as_bytes())
    }
}

#[cfg(feature = "chrono")]
impl GeneralizedTime for chrono::DateTime<chrono::offset::Utc> {
    fn append_generalized_time(&self, target: &mut Vec<u8>) {
        use std::io::Write;

        write!(target, "{}", self.format("%Y%m%d%H%M%SZ")).unwrap();
    }
}


//------------ x509_time -----------------------------------------------------

/// Returns a recipe for writing a time value following the rules of RFC 5280.
#[cfg(feature = "chrono")]
pub fn x509_time(time: chrono::DateTime<chrono::offset::Utc>) -> Recipe {
    if chrono::Datelike::year(&time) >= 2050 {
        utc_time(time)
    }
    else {
        generalized_time(time)
    }
}


//------------ explicit ------------------------------------------------------

/// Returns a recipe for creating an explicitely context-specific taged value.
pub fn explicit(number: u128, content: Recipe) -> Recipe {
    value(tag(Class::Context, true, number), content)
}


//============ Low-level Recipes =============================================

//------------ value ---------------------------------------------------------

/// Returns a recipe for a generic DER value.
///
/// The recipe produces the tag of the value by running the _tag_ recipe,
/// the both the length and actual content by running the _content_ recipe.
pub fn value(tag: Recipe, content: Recipe) -> Recipe {
    Value { tag, content }.into()
}

struct Value {
    tag: Recipe,
    content: Recipe
}

impl Value {
    fn append_length(length: usize, target: &mut Vec<u8>) {
        // 10.1. Always definite form with the minimal number of octets.
        // So, if < 128 short form, otherwise long form.
        if length < 128 {
            // 8.1.3.4. short form - just the value.
            target.push(length as u8);
        }
        else {
            // 8.1.3.5. long form - first octet provides the number of
            // subsequent octets with bit 8 forced to 1.
            let usize_octets = (usize::BITS >> 3) as usize;
            let octets = (
                ((usize::BITS - length.leading_zeros()) >> 3) + 1
            ) as usize;
            target.push((octets as u8) | 0x80);
            target.extend_from_slice(
                &length.to_be_bytes()[usize_octets - octets..]
            );
        }
    }
}

impl AppendData for Value {
    fn append(&self, target: &mut Vec<u8>) {
        let buf = self.content.to_vec();
        self.tag.append(target);
        Self::append_length(buf.len(), target);
        target.extend_from_slice(&buf);
    }
}


//------------ tag -----------------------------------------------------------

/// Returns a recipe for a DER encoded tag.
pub fn tag(class: Class, constructed: bool, number: u128) -> Recipe {
    Tag { class, constructed, number }.into()
}

pub fn universal(constructed: bool, number: u128) -> Recipe {
    tag(Class::Universal, constructed, number)
}

pub fn context(constructed: bool, number: u128) -> Recipe {
    tag(Class::Context, constructed, number)
}

struct Tag {
    class: Class,
    constructed: bool,
    number: u128
}

impl AppendData for Tag {
    fn append(&self, target: &mut Vec<u8>) {
        let mut first = match self.class {
            Class::Universal => 0,
            Class::Application => 0b0100_0000,
            Class::Context => 0b1000_0000,
            Class::Private => 0b1100_0000,
        };
        if self.constructed {
            first = first | 0b0010_0000;
        }
        if self.number < 31 {
            target.push(first | self.number as u8);
        }
        else {
            target.push(first | 0b0001_1111);
            append_base_7(self.number, target)
        }
    }
}

fn append_base_7(mut number: u128, target: &mut Vec<u8>) {
    // Convert the number into base 7. We use bytes for
    // the digits and leave the left-most bit at 0. A 128 bit number
    // can be at most 19 digits long. So we start with an empty octet
    // array of that length and then shift the number into it.
    let mut digits = [0u8; 19];
    for i in (0..19_usize).rev() {
        digits[i] = (number as u8) & 0b0111_1111;
        number = number >> 7;
    }

    // Now skip over empty octets and then add the remaining ones. All
    // but the last one need to have the left-most bit set.
    let mut idx = 0;
    while idx < 18 && digits[idx] == 0 {
        idx += 1;
    }
    while idx < 18 {
        target.push(digits[idx] | 0b1000_0000);
        idx += 1;
    }
    target.push(digits[18]);
}

//------------ Class ---------------------------------------------------------

/// The class portion of a DER tag.
#[derive(Clone, Copy, Debug)]
pub enum Class {
    Universal,
    Application,
    Context,
    Private,
}

