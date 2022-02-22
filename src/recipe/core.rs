//! Fundamentals for recipes.

use std::io;
use std::str::FromStr;


pub fn write_recipe(
    recipe: &Recipe, target: &mut impl io::Write
) -> Result<(), io::Error> {
    let mut buf = Vec::new();
    recipe.append(&mut buf);
    target.write_all(&buf)
}


//------------ AppendData ----------------------------------------------------

pub trait AppendData {
    fn append(&self, target: &mut Vec<u8>);
}

impl<T: AsRef<[u8]>> AppendData for T {
    fn append(&self, target: &mut Vec<u8>) {
        target.extend_from_slice(self.as_ref())
    }
}

//------------ Recipe --------------------------------------------------------

pub type Recipe = Box<dyn AppendData>;

impl<T: AppendData + 'static> From<T> for Recipe {
    fn from(src: T) -> Self {
        Box::new(src)
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

impl<const N: usize> AppendData for Sequence<N> {
    fn append(&self, target: &mut Vec<u8>) {
        for item in &self.items {
            item.append(target)
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

impl AppendData for Hex {
    fn append(&self, target: &mut Vec<u8>) {
        let mut s = self.0.as_str();
        while !s.is_empty() {
            let (octet, tail) = s.split_at(2);
            target.push(u8::from_str(octet).unwrap());
            s = tail;
        }
    }
}

