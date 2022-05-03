//! Encoding data using DER.

use super::core::{Fragment, Recipe, literal};


//============ Basic Machinery ===============================================

/// A type representing the content of a DER encoded value.
pub trait DerContent {
    /// Returns whether the content is a constructed value or not.
    fn is_constructed(&self) -> bool;

    /// Assembles the content into the target.
    fn assemble_content(&self, target: &mut Fragment);
}

impl<'a, C: DerContent> DerContent for &'a C {
    fn is_constructed(&self) -> bool {
        (*self).is_constructed()
    }

    fn assemble_content(&self, target: &mut Fragment) {
        (*self).assemble_content(target)
    }
}


//------------ constructed ---------------------------------------------------

/// Returns a recipe for turning any recipe into constructed DER content.
pub fn constructed<R>(content: R) -> ConstructedDerContent<R> {
    ConstructedDerContent(content)
}

pub struct ConstructedDerContent<R>(R);

impl<R: Recipe> DerContent for ConstructedDerContent<R> {
    fn is_constructed(&self) -> bool {
        true
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.0.assemble(target)
    }
}


//------------ simple --------------------------------------------------------

/// Returns a recipe for turning any recipe into simple DER content.
pub fn simple<R>(recipe: R) -> SimpleDerContent<R> {
    SimpleDerContent(recipe)
}

pub struct SimpleDerContent<R>(R);

impl<R: Recipe> DerContent for SimpleDerContent<R> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.0.assemble(target)
    }
}


//------------ value et al. --------------------------------------------------

/// Returns a recipe for a DER encoded value.
fn value<C>(tag: Tag, content: C) -> Value<C> {
    Value { tag, content }
}

/// Returns a recipe for a DER encoded value in the universal class.
pub fn universal<C>(number: u128, content: C) -> Value<C> {
    value(Tag::new(Class::Universal, number), content)
}

/// Returns a recipe for a DER encoded value in the universal class.
pub fn application<C>(number: u128, content: C) -> Value<C> {
    value(Tag::new(Class::Application, number), content)
}

/// Returns a recipe for a DER encoded value in the universal class.
pub fn context<C>(number: u128, content: C) -> Value<C> {
    value(Tag::new(Class::Context, number), content)
}

/// Returns a recipe for a DER encoded value in the universal class.
pub fn private<C>(number: u128, content: C) -> Value<C> {
    value(Tag::new(Class::Private, number), content)
}


pub struct Value<C> {
    tag: Tag,
    content: C,
}

impl<C: DerContent> Value<C> {
    fn assemble_length(length: usize, target: &mut Fragment) {
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


impl<C: DerContent> Recipe for Value<C> {
    fn assemble(&self, target: &mut Fragment) {
        let mut content = Fragment::new();
        self.content.assemble_content(&mut content);
        self.tag.assemble(self.content.is_constructed(), target);
        Self::assemble_length(content.len(), target);
        target.extend_from_slice(content.as_ref());
    }
}

impl<C: DerContent> DerContent for Value<C> {
    fn is_constructed(&self) -> bool {
        true
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.assemble(target)
    }
}


//============ Standard Types ================================================

//------------ boolean -------------------------------------------------------

/// Returns a recipe for writing a DER-encoded boolean.
pub fn boolean(x: bool) -> Boolean {
    Boolean(
        if x { 0xFF }
        else { 0x00 }
    )
}

pub struct Boolean(u8);

impl Recipe for Boolean {
    fn assemble(&self, target: &mut Fragment) {
        universal(1, self).assemble(target)
    }
}

impl DerContent for Boolean {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        target.push(self.0)
    }
}


//------------ integer -------------------------------------------------------

/// Returns a recipe for writing a DER-encoded integer.
pub fn integer<C>(int: C) -> Integer<C> {
    Integer(int)
}

pub trait IntegerContent {
    fn assemble_integer(&self, target: &mut Fragment);
}

pub struct Integer<C>(C);

impl<C: IntegerContent> Recipe for Integer<C> {
    fn assemble(&self, target: &mut Fragment) {
        universal(2, self).assemble(target)
    }
}

impl<C: IntegerContent> DerContent for Integer<C> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.0.assemble_integer(target)
    }
}

/// Assembles an unsigned integer from an slice in network byte order.
fn assemble_unsigned_slice(mut slice: &[u8], target: &mut Fragment) {
    // Skip over empty octets.
    while slice.get(0).copied() == Some(0) {
        slice = &slice[1..];
    }

    // If the left-most bit is set, we need to add another octet to signal
    // that we have a positive integer.
    if slice.get(0).copied().unwrap_or(0xFF) & 0x80 != 0 {
        target.push(0);
    }

    // The rest is straightforward.
    target.extend_from_slice(slice)
}

/// Assembles an unsigned integer from an slice in network byte order.
fn assemble_signed_slice(mut slice: &[u8], target: &mut Fragment) {
    // Check the left-most bit. If it is not set, the number is positive
    // and we can fall back to an unsigned slice.
    if slice.get(0).copied().unwrap_or(0xFF) & 0x80 == 0 {
        return assemble_unsigned_slice(slice, target)
    }

    // We have a non-empty negative number. Because of the two’s complement
    // thing, ununsed left octets are 0xFF.
    while slice.get(0).copied() == Some(0xFF) {
        slice = &slice[1..];
    }

    // The rest is straightforward.
    if slice.is_empty() {
        target.push(0xFF)
    }
    else {
        target.extend_from_slice(slice)
    }
}

impl IntegerContent for u8 {
    fn assemble_integer(&self, target: &mut Fragment) {
        if *self > 127 {
            target.push(0);
        }
        target.push(*self);
    }
}

impl IntegerContent for u16 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_unsigned_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for u32 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_unsigned_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for u64 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_unsigned_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for u128 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_unsigned_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for usize {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_unsigned_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for i8 {
    fn assemble_integer(&self, target: &mut Fragment) {
        target.push((*self) as u8)
    }
}

impl IntegerContent for i16 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for i32 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for i64 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for i128 {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(&self.to_be_bytes(), target);
    }
}

impl IntegerContent for isize {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(&self.to_be_bytes(), target);
    }
}

impl<'a> IntegerContent for &'a [u8] {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(self, target);
    }
}

impl IntegerContent for Vec<u8> {
    fn assemble_integer(&self, target: &mut Fragment) {
        assemble_signed_slice(self.as_ref(), target);
    }
}


//------------ integer_slice -------------------------------------------------

/// Returns a recipe for writing a DER-encoded integer given as a slice.
pub fn integer_slice<C>(int: C) -> IntegerSlice<C> {
    IntegerSlice(int)
}

pub struct IntegerSlice<C>(C);

impl<C: AsRef<[u8]>> Recipe for IntegerSlice<C> {
    fn assemble(&self, target: &mut Fragment) {
        universal(2, self).assemble(target)
    }
}

impl<C: AsRef<[u8]>> DerContent for IntegerSlice<C> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        assemble_signed_slice(self.0.as_ref(), target);
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
pub fn bitstring<R: Recipe>(unused: u8, content: R) -> impl Recipe + DerContent {
    StringValue::new(
        Tag::universal(3),
        sequence((literal([unused]), content))
    )
}


//------------ octetstring ---------------------------------------------------

/// Returns a recipe for writing the given content as DER octet string.
pub fn octetstring<R>(content: R) -> StringValue<R> {
    StringValue::new(Tag::universal(4), content)
}


//------------ null ----------------------------------------------------------

/// Returns a recipe for a DER-encoded null value.
pub fn null() -> Null {
    Null
}

pub struct Null;

impl Recipe for Null {
    fn assemble(&self, target: &mut Fragment) {
        universal(5, self).assemble(target)
    }
}

impl DerContent for Null {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, _: &mut Fragment) {
    }
}


//------------ oid -----------------------------------------------------------

/// Returns a recipe for writing an object identifier.
pub fn oid<const N: usize>(items: [u128; N]) -> Oid<N> {
    Oid(items)
}

pub struct Oid<const N: usize>([u128; N]);

impl<const N: usize> DerContent for Oid<N> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        assemble_base_7((self.0[0] * 40) + self.0[1], target);
        for value in &self.0[2..] {
            assemble_base_7(*value, target)
        }
    }
}

impl<const N: usize> Recipe for Oid<N> {
    fn assemble(&self, target: &mut Fragment) {
        universal(6, self).assemble(target)
    }
}


//------------ sequence ------------------------------------------------------

/// Returns a recipe for writing a receipe as the content of a DER sequence.
pub fn sequence(items: impl Recipe) -> impl Recipe {
    universal(16, constructed(items))
}


//------------ set -----------------------------------------------------------

/// Returns a recipe for writing a receipe as the content of a DER set.
pub fn set(items: impl Recipe) -> impl Recipe {
    universal(17, constructed(items))
}


//------------ printable_string ----------------------------------------------

/// Returns a recipe for writing the given content as PrintableString.
///
/// Does not check if the content is a valid printable string.
pub fn printable_string<R>(content: R) -> StringValue<R> {
    StringValue::new(Tag::universal(19), content)
}


//------------ ia5_string ----------------------------------------------------

/// Returns a recipe for writing the given content as IA5String.
///
/// Does not check if the content is a valid printable string.
pub fn ia5_string<R>(content: R) -> StringValue<R> {
    StringValue::new(Tag::universal(22), content)
}


//------------ utc_time and generalized_time ---------------------------------

/// Returns a recipe for writing a time as a UTCTime value.
pub fn utc_time<T>(time: T) -> UtcTime<T> {
    UtcTime(time)
}

/// Returns a recipe for writing a time as a GeneralizedTime value.
pub fn generalized_time<T>(time: T) -> GeneralizedTime<T> {
    GeneralizedTime(time)
}

pub trait TimeContent {
    fn assemble_utc_time(&self, target: &mut Fragment);
    fn assemble_generalized_time(&self, target: &mut Fragment);
}

#[cfg(feature = "chrono")]
impl TimeContent for chrono::DateTime<chrono::offset::Utc> {
    fn assemble_utc_time(&self, target: &mut Fragment) {
        use std::io::Write;

        write!(target, "{}", self.format("%y%m%d%H%M%SZ")).unwrap();
    }

    fn assemble_generalized_time(&self, target: &mut Fragment) {
        use std::io::Write;

        write!(target, "{}", self.format("%Y%m%d%H%M%SZ")).unwrap();
    }
}

pub struct UtcTime<T>(T);

impl<T: TimeContent> Recipe for UtcTime<T> {
    fn assemble(&self, target: &mut Fragment) {
        universal(23, self).assemble(target)
    }
}

impl<T: TimeContent> DerContent for UtcTime<T> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.0.assemble_utc_time(target);
    }
}

pub struct GeneralizedTime<T>(T);

impl<T: TimeContent> Recipe for GeneralizedTime<T> {
    fn assemble(&self, target: &mut Fragment) {
        universal(23, self).assemble(target)
    }
}

impl<T: TimeContent> DerContent for GeneralizedTime<T> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.0.assemble_generalized_time(target);
    }
}


//============ Helper Types ==================================================

//------------ Tag -----------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Tag {
    class: Class,
    number: u128,
}

impl Tag {
    fn new(class: Class, number: u128) -> Self {
        Tag { class, number }
    }

    fn universal(number: u128) -> Self {
        Tag::new(Class::Universal, number)
    }

    fn assemble(&self, constructed: bool, target: &mut Fragment) {
        let mut first = match self.class {
            Class::Universal => 0,
            Class::Application => 0b0100_0000,
            Class::Context => 0b1000_0000,
            Class::Private => 0b1100_0000,
        };
        if constructed {
            first = first | 0b0010_0000;
        }
        if self.number < 31 {
            target.push(first | self.number as u8);
        }
        else {
            target.push(first | 0b0001_1111);
            assemble_base_7(self.number, target)
        }
    }
}

fn assemble_base_7(mut number: u128, target: &mut Fragment) {
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
enum Class {
    Universal,
    Application,
    Context,
    Private,
}


//------------ StringValue ---------------------------------------------------

/// Any of the many string types.
pub struct StringValue<R> {
    tag: Tag,
    content: R,
}

impl<R> StringValue<R> {
    fn new(tag: Tag, content: R) -> Self {
        StringValue { tag, content }
    }
}

impl<R: Recipe> DerContent for StringValue<R> {
    fn is_constructed(&self) -> bool {
        false
    }

    fn assemble_content(&self, target: &mut Fragment) {
        self.content.assemble(target)
    }
}

impl<R: Recipe> Recipe for StringValue<R> {
    fn assemble(&self, target: &mut Fragment) {
        value(self.tag, self).assemble(target)
    }
}


//============ Tests =========================================================

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn der_boolean() {
        assert_eq!(boolean(true).to_fragment(), b"\x01\x01\xFF");
        assert_eq!(boolean(false).to_fragment(), b"\x01\x01\x00");
        assert_eq!(
            context(0, boolean(true)).to_fragment(),
            b"\x80\x01\xFF"
        );
    }

    #[test]
    fn der_integer() {
        assert_eq!(integer(0u8).to_fragment(), b"\x02\x01\x00");
        assert_eq!(integer(0u32).to_fragment(), b"\x02\x01\x00");
        assert_eq!(integer(0i128).to_fragment(), b"\x02\x01\x00");

        assert_eq!(integer(-1i8).to_fragment(), b"\x02\x01\xFF");
        assert_eq!(integer(-1i32).to_fragment(), b"\x02\x01\xFF");
        assert_eq!(integer(-1i128).to_fragment(), b"\x02\x01\xFF");
        assert_eq!(integer(-2i8).to_fragment(), b"\x02\x01\xFE");
        assert_eq!(integer(-2i32).to_fragment(), b"\x02\x01\xFE");
        assert_eq!(integer(-2i128).to_fragment(), b"\x02\x01\xFE");
    }
}

