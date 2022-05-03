//! Fundamentals for recipes.

use std::{borrow, io, ops};


//------------ Recipe --------------------------------------------------------


/// A type that knows how to assemble some data and add it to a fragment.
pub trait Recipe {
    /// Assembles the data andd appends it a fragment.
    fn assemble(&self, target: &mut Fragment);

    /// Assembles the data into a new vec.
    fn to_fragment(&self) -> Fragment {
        let mut frag = Fragment::new();
        self.assemble(&mut frag);
        frag
    }
}

impl<'a, T: Recipe> Recipe for &'a T {
    fn assemble(&self, target: &mut Fragment) {
        (*self).assemble(target)
    }
}

impl<T: Recipe + 'static> From<T> for Box<dyn Recipe> {
    fn from(src: T) -> Self {
        Box::new(src)
    }
}

impl<
    N0: Recipe,
    N1: Recipe,
> Recipe for (N0, N1) {
    fn assemble(&self, target: &mut Fragment) {
        self.0.assemble(target);
        self.1.assemble(target);
    }
}

impl<
    N0: Recipe,
    N1: Recipe,
    N2: Recipe,
> Recipe for (N0, N1, N2) {
    fn assemble(&self, target: &mut Fragment) {
        self.0.assemble(target);
        self.1.assemble(target);
        self.2.assemble(target);
    }
}

impl<
    N0: Recipe,
    N1: Recipe,
    N2: Recipe,
    N3: Recipe,
> Recipe for (N0, N1, N2, N3) {
    fn assemble(&self, target: &mut Fragment) {
        self.0.assemble(target);
        self.1.assemble(target);
        self.2.assemble(target);
        self.3.assemble(target);
    }
}

impl<
    N0: Recipe,
    N1: Recipe,
    N2: Recipe,
    N3: Recipe,
    N4: Recipe,
> Recipe for (N0, N1, N2, N3, N4) {
    fn assemble(&self, target: &mut Fragment) {
        self.0.assemble(target);
        self.1.assemble(target);
        self.2.assemble(target);
        self.3.assemble(target);
        self.4.assemble(target);
    }
}

impl<
    N0: Recipe,
    N1: Recipe,
    N2: Recipe,
    N3: Recipe,
    N4: Recipe,
    N5: Recipe,
> Recipe for (N0, N1, N2, N3, N4, N5) {
    fn assemble(&self, target: &mut Fragment) {
        self.0.assemble(target);
        self.1.assemble(target);
        self.2.assemble(target);
        self.3.assemble(target);
        self.4.assemble(target);
        self.5.assemble(target);
    }
}


//------------ Fragment ------------------------------------------------------

/// A fragment of data produced by executing a recipt.
#[derive(Clone, Debug, Default, Hash, Ord, PartialOrd)]
pub struct Fragment {
    data: Vec<u8>,
}

impl Fragment {
    /// Creates a new, empty fragment.
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns the content of the fragment as a slice.
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Appends a single octet the the fragment.
    pub fn push(&mut self, octet: u8) {
        self.data.push(octet)
    }

    /// Appends a the content of a slice of octets to the fragment.
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.data.extend_from_slice(slice)
    }
}


//--- PartialEq and Eq

impl<T: AsRef<[u8]>> PartialEq<T> for Fragment {
    fn eq(&self, other: &T) -> bool {
        self.data.eq(other.as_ref())
    }
}

impl Eq for Fragment { }


//--- Deref, AsRef, Borrow

impl ops::Deref for Fragment {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for Fragment {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl borrow::Borrow<[u8]> for Fragment {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}


//--- io::Write

impl io::Write for Fragment {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}


//------------ iter ----------------------------------------------------------

/// Returns a recipe iterating over and assembling the items of an iterator.
pub fn iter<T>(iter: T) -> Iter<T> {
    Iter(iter)
}

pub struct Iter<T>(T);

impl<T> Recipe for Iter<T>
where
    for<'a> &'a T: IntoIterator,
    for<'a> <&'a T as IntoIterator>::Item: Recipe,
{
    fn assemble(&self, target: &mut Fragment) {
        for item in &self.0 {
            item.assemble(target)
        }
    }
}


//------------ empty ---------------------------------------------------------

/// Returns an empty recipe.
pub fn empty() -> Empty {
    Empty
}

pub struct Empty;

impl Recipe for Empty {
    fn assemble(&self, _: &mut Fragment) {
    }
}


//------------ literal -------------------------------------------------------

/// Constructs a recipe adding a literal.
///
/// The function accepts any static object that implements `AsRef<[u8]>`.
/// Appart from actual string and bytes literals, these are also `u8` arrays,
/// which comes in handy when describing actual binary data.
pub fn literal<T: AsRef<[u8]> + 'static>(literal: T) -> Literal<T> {
    Literal(literal)
}

pub struct Literal<T>(T);

impl<T: AsRef<[u8]> + 'static> Recipe for Literal<T> {
    fn assemble(&self, target: &mut Fragment) {
        target.extend_from_slice(self.0.as_ref())
    }
}


//------------ hex -----------------------------------------------------------

/// Returns a recipe writing out the given hex string as binary data.
///
/// The string must consist of hex digits and white space only with an even
/// number of hex digits. Pairs of hex digits are then interpreted as the
/// values of an octet in hexadecimal notation.
///
/// The function accepts any static object that implements `AsRef<str>`.
pub fn hex<T: AsRef<str>>(hex: T) -> Hex<T> {
    Hex(hex).check()
}

pub struct Hex<T>(T);

impl<T: AsRef<str>> Hex<T> {
    /// Checks that the contained string is valid.
    ///
    /// Panics if it isn’t.
    fn check(self) -> Self {
        let mut count = 0;
        for ch in self.0.as_ref().chars() {
            if ch.is_ascii_whitespace() { }
            else if ch.is_digit(16) {
                count += 1
            }
            else {
                panic!("Invalid hex string '{}'", self.0.as_ref())
            }
        }
        if count % 2 != 0 {
            panic!("Uneven hex string '{}'", self.0.as_ref())
        }
        self
    }
}

impl<T: AsRef<str>> Recipe for Hex<T> {
    fn assemble(&self, target: &mut Fragment) {
        // The contained string has been checked, so we can assume it to be
        // only whitespace and an even number of hex digits.
        let mut chars = self.0.as_ref().chars().filter_map(|ch| {
            ch.to_digit(16).map(|ch| ch as u8)
        });
        while let Some(ch1) = chars.next(){
            target.push((ch1 << 4) | chars.next().unwrap())
        }
    }
}


//------------ be ------------------------------------------------------------

/// Returns a recipe writing the given integer in big-endian encoding.
pub fn be<T: IntoBigEndian>(int: T) -> Literal<T::Literal> {
    Literal(int.into_be())
}

pub trait IntoBigEndian {
    type Literal: AsRef<[u8]> + 'static;

    fn into_be(self) -> Self::Literal;
}

macro_rules! into_be {
    ( $type:ident) => {
        impl IntoBigEndian for $type {
            type Literal = [u8; ($type::BITS as usize) >> 3];

            fn into_be(self) -> Self::Literal {
                self.to_be_bytes()
            }
        }
    }
}

into_be!(u16);
into_be!(u32);
into_be!(u64);
into_be!(u128);
into_be!(i16);
into_be!(i32);
into_be!(i64);
into_be!(i128);

impl IntoBigEndian for u8 {
    type Literal = [u8; 1];

    fn into_be(self) -> Self::Literal {
        [self]
    }
}


impl IntoBigEndian for i8 {
    type Literal = [u8; 1];

    fn into_be(self) -> Self::Literal {
        [self as u8]
    }
}


//------------ exec ----------------------------------------------------------

/// Returns a recipe executing the given closure whenever data is assembled.
pub fn exec<Op: Fn(&mut Fragment) + 'static>(op: Op) -> Exec<Op> {
    Exec(op).into()
}

pub struct Exec<Op>(Op);

impl<Op: Fn(&mut Fragment) + 'static> Recipe for Exec<Op> {
    fn assemble(&self, target: &mut Fragment) {
        (self.0)(target)
    }
}


