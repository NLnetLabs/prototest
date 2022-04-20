//! Fundamentals for recipes.
//!
//! The basis for all recipes is the trait [`Assemble`]. Anything that
//! implements this trait is considered a recipe. All the actual [`Recipe`]
//! type does is wrap an `Assemble` trait object to make it more convenient
//! to deal recipes.
//!
//! When assembling actual data from a recipe, this data is stored in a
//! [`Fragment`] rather than a `Vec<u8>` so we can add useful convenience
//! functions and trait implementations to it later on.
//!
//! This module also provides a number of useful functions to create recipes
//! from other recipes.

use std::{borrow, io, ops};
use std::str::FromStr;


//------------ Assemble ------------------------------------------------------

/// A type that knows how to assemble some data and add it to a fragment.
pub trait Assemble {
    /// Assembles the data and appends it to `target`.
    fn assemble(&self, target: &mut Fragment);
}

impl<T: AsRef<[u8]>> Assemble for T {
    fn assemble(&self, target: &mut Fragment) {
        target.extend_from_slice(self.as_ref())
    }
}

//------------ Recipe --------------------------------------------------------

pub struct Recipe(Box<dyn Assemble>);

impl Recipe {
    pub fn assemble(&self, target: &mut Fragment) {
        self.0.assemble(target);
    }

    pub fn to_fragment(&self) -> Fragment {
        let mut buf = Fragment::new();
        self.assemble(&mut buf);
        buf
    }

    pub fn write(
        &self, target: &mut impl io::Write
    ) -> Result<(), io::Error> {
        target.write_all(&self.to_fragment())
    }
}

impl<T: Assemble + 'static> From<T> for Recipe {
    fn from(src: T) -> Self {
        Recipe(Box::new(src))
    }
}


//------------ Fragment ------------------------------------------------------

/// A fragment of data produced by executing a recipt.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
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


//------------ sequence ------------------------------------------------------

/// Constructs a recipe invoking a sequence of other recipes in order.
pub fn sequence<const N: usize>(items: [Recipe; N]) -> Recipe {
    Sequence { items }.into()
}

struct Sequence<const N: usize> {
    items: [Recipe; N],
}

impl<const N: usize> Assemble for Sequence<N> {
    fn assemble(&self, target: &mut Fragment) {
        for item in &self.items {
            item.assemble(target)
        }
    }
}


//------------ literal -------------------------------------------------------

/// Constructs a recipe adding a literal.
///
/// The function accepts any static object that implements `AsRef<[u8]>`.
/// Appart from actual string and bytes literals, these are also `u8` arrays,
/// which comes in handy when describing actual binary data.
pub fn literal(literal: impl AsRef<[u8]> + 'static) -> Recipe {
    literal.into()
}


//------------ hex -----------------------------------------------------------

/// Returns a recipe writing out the given hex string.
pub fn hex(hex: impl Into<String>) -> Recipe {
    Hex(hex.into()).into()
}

struct Hex(String);

impl Assemble for Hex {
    fn assemble(&self, target: &mut Fragment) {
        let mut s = self.0.as_str();
        while !s.is_empty() {
            let (octet, tail) = s.split_at(2);
            target.push(u8::from_str(octet).unwrap());
            s = tail;
        }
    }
}


//------------ exec ----------------------------------------------------------

/// Returns a recipe executing the given closure whenever data is assembled.
pub fn exec(op: impl Fn(&mut Fragment) + 'static) -> Recipe {
    Exec(op).into()
}

struct Exec<Op>(Op);

impl<Op: Fn(&mut Fragment) + 'static> Assemble for Exec<Op> {
    fn assemble(&self, target: &mut Fragment) {
        (self.0)(target)
    }
}

