#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable", feature(allocator_api))]

//! This is a bit vector implementation with guaranteed `[u8]` [LSB 0][1]
//! representation and the ability to get safe immutable and mutable views into its
//! internal vector for easy I/O.
//!
//! [1]: https://en.wikipedia.org/wiki/Bit_numbering#LSB_0_bit_numbering
//!
//! It mirrors the API of `std::vec::Vec` as much as possible. Notable differences:
//! - `BitVec`'s non-consuming iterator enumerates `bool`s instead of `&bool`s.

// TODO: Flesh out docs.

extern crate alloc;

#[cfg(feature = "unstable")]
use core::alloc::Allocator;
use core::fmt;
use core::num::Wrapping;
use core::write;
use core::prelude::rust_2021::*;
use alloc::vec::Vec;
use alloc::vec;
#[cfg(feature = "unstable")]
use alloc::alloc::Global;

#[cfg(feature = "serde")]
#[macro_use] extern crate serde;

/// Bit vector with guaranteed `[u8]` LSB 0 representation and safe mutable access to this slice.
/// Slices into the bit vector are guaranteed to have the unused bits on the last byte set to 0.
#[cfg(not(feature = "unstable"))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone, Default, PartialEq, Eq)]
pub struct BitVec {
    nbits: usize,
    vec: Vec<u8>,
}

/// Bit vector with guaranteed `[u8]` LSB 0 representation and safe mutable access to this slice.
/// Slices into the bit vector are guaranteed to have the unused bits on the last byte set to 0.
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone)]
pub struct BitVec<A: Allocator = Global> {
    nbits: usize,
    vec: Vec<u8, A>,
}

// Explicitly allow comparisons between BitVecs regardless of whether
// they use the same allocator or whether their allocator implements
// PartialEq or not.
#[cfg(feature = "unstable")]
impl<A: Allocator, B: Allocator> PartialEq<BitVec<B>> for BitVec<A> {
    
    fn eq(&self, other: &BitVec<B>) -> bool {
        self.nbits == other.nbits && self.vec == other.vec
    }

}

#[cfg(feauture = "unstable")]
impl Default for BitVec {
    
    fn default() -> Self {
        Self { nbits: 0, vec: Vec::new() }
    }

}

#[cfg(feature = "unstable")]
impl<A: Allocator> Eq for BitVec<A> {}

fn bytes_in_bits(nbits: usize) -> usize {
    // #bytes = #ceil(nbits / 8)
    (nbits + 7) / 8
}

fn byte_from_bool(bit: bool) -> u8 {
    if bit { !0u8 } else { 0u8 }
}

#[cfg(feature = "unstable")]
impl<A: Allocator> BitVec<A> {
    ////////////////////////////////////////
    // Constructors

    /// Constructs an empty `BitVec`.
    pub const fn new_in(alloc: A) -> Self {
        Self { vec: Vec::new_in(alloc), nbits: 0 }
    }

    /// Constructs a `BitVec` from bytes.
    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        Self { vec: Vec::with_capacity_in(bytes_in_bits(capacity), alloc), nbits: 0 }
    }

}

impl BitVec {
    ////////////////////////////////////////
    // Constructors

    /// Constructs an empty `BitVec`.
    pub const fn new() -> Self {
        Self { vec: Vec::new(), nbits: 0 }
    }

    /// Constructs an empty `BitVec` with the given capacity.
    ///
    /// The bit vector will be able to hold at least capacity bits without reallocating. If
    /// capacity is 0, the bit vector will not allocate.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { vec: Vec::with_capacity(bytes_in_bits(capacity)), nbits: 0 }
    }

    /// Constructs a `BitVec` from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut vec = Self { vec: Vec::from(bytes), nbits: bytes.len() * 8 };
        vec.set_unused_zero();
        vec
    }

    /// Constructs a `BitVec` from bools.
    pub fn from_bools(bools: &[bool]) -> Self {
        let mut vec = Self::with_capacity(bools.len());
        for &b in bools {
            vec.push(b);
        }
        vec
    }

    /// Constructs a `BitVec` from a repeating bit value.
    pub fn from_elem(len: usize, value: bool) -> Self {
        let mut vec = Self {
            vec: vec![byte_from_bool(value); bytes_in_bits(len)],
            nbits: len,
        };
        vec.set_unused_zero();
        vec
    }

}

macro_rules! impl_bitvec {
    ($into_bytes_type: ty) => {

        ////////////////////////////////////////
        // Converters/views

        /// Returns a byte slice view of the data.
        pub fn as_bytes(&self) -> &[u8] { &self.vec }

        /// Invokes the given function on a mut byte slice view of the data. After `f` completes, the
        /// trailing unused bits of the last byte are automatically set to 0.
        pub fn with_bytes_mut<U, F: FnOnce(&mut [u8]) -> U>(&mut self, f: F) -> U {
            let val = f(&mut self.vec);
            self.set_unused_zero();
            val
        }

        /// Consumes the `self` and returns the underlying `Vec<u8>` of length `ceil(self.len()/8)`.
        /// The values of the bits in the last byte of `Vec<u8>` beyond the length of the `BitVec` are
        /// 0.
        pub fn into_bytes(self) -> $into_bytes_type { self.vec }

        ////////////////////////////////////////
        // Getters/setters

        /// Returns the length of the bit vector.
        pub fn len(&self) -> usize { self.nbits }

        /// Returns whether the vector is empty.
        pub fn is_empty(&self) -> bool { self.nbits == 0 }

        /// Validates the index for validity or panics.
        fn validate_index(&self, index: usize) {
            assert!(self.nbits <= self.vec.len() * 8,
                    "Expected #bits {} <= 8 x (#bytes {} in vec).", self.nbits, self.vec.len());
            if index >= self.nbits { panic!("Index {} out of bounds [0, {})", index, self.nbits); }
        }

        /// Gets the bit at the given `index`.
        pub fn get(&self, index: usize) -> Option<bool> {
            if index < self.len() {
                Some(unsafe { self.get_unchecked(index) })
            } else {
                None
            }
        }

        /// Sets the bit at the given `index`. Panics if `index` exceeds length.
        pub fn set(&mut self, index: usize, value: bool) {
            self.validate_index(index);
            unsafe { self.set_unchecked(index, value) };
        }

        /// Swaps two elements in the `BitVec`.
        pub fn swap(&mut self, i: usize, j: usize) {
            self.validate_index(i);
            self.validate_index(j);
            unsafe {
                let val_i = self.get_unchecked(i);
                let val_j = self.get_unchecked(j);
                self.set_unchecked(i, val_j);
                self.set_unchecked(j, val_i);
            }
        }

        /// Gets the bit at the given `index` without bounds checking.
        pub unsafe fn get_unchecked(&self, index: usize) -> bool {
            let byte = self.vec.get_unchecked(index / 8);
            let pattern = 1u8 << (index % 8);
            (*byte & pattern) != 0u8
        }

        /// Sets the bit at the given `index` without bounds checking.
        pub unsafe fn set_unchecked(&mut self, index: usize, value: bool) {
            let byte = self.vec.get_unchecked_mut(index / 8);
            let pattern = 1u8 << (index % 8);
            *byte = if value { *byte |  pattern }
                    else     { *byte & !pattern };
        }

        ////////////////////////////////////////
        // Adding/removing items

        /// Pushes a boolean to the end of the `BitVec`.
        pub fn push(&mut self, value: bool) {
            let nbits = self.nbits; // avoid mutable borrow error
            if nbits % 8 == 0 {
                self.vec.push(if value { 1u8 } else { 0u8 });
            } else {
                unsafe { self.set_unchecked(nbits, value) };
            }
            self.nbits += 1;
        }

         /// Pops a boolean from the end of the `BitVec`.
        pub fn pop(&mut self) -> Option<bool> {
            if self.nbits == 0 { return None }
            self.nbits -= 1;

            // Get the popped bit value to return.
            let nbits = self.nbits; // avoid mutable borrow error
            let value = unsafe { self.get_unchecked(nbits) };
            // Set the popped bit value to 0.
            unsafe { self.set_unchecked(nbits, false); }
            // Pop off the last byte from the underlying vector if it has no active bits.
            if self.nbits % 8 == 0 {
                assert!(self.nbits == (self.vec.len() - 1) * 8,
                    "Expected #bits {} == 8 x (#bytes {} in vec - 1) after bit pop and before vec pop.",
                    self.nbits, self.vec.len());
                self.vec.pop();
            }

            Some(value)
        }

        /// Clears the `BitVec`, removing all values.
        pub fn clear(&mut self) {
            self.vec.clear();
            self.nbits = 0;
        }

        /// Returns the number of booleans that the bitvec can hold without reallocating.
        pub fn capacity(&self) -> usize {
            self.vec.capacity() * 8
        }

        /// Reserves capacity for at least additional more booleans to be inserted in the given
        /// `BitVec`. The collection may reserve more space to avoid frequent reallocations.
        pub fn reserve(&mut self, additional: usize) {
            self.vec.reserve(bytes_in_bits(additional))
        }

        /// Shorten a vector, dropping excess elements.
        ///
        /// If `len` is greater than the vector's current length, this has no effect.
        pub fn truncate(&mut self, len: usize) {
            if len < self.len() {
            let nbytes = bytes_in_bits(len);
            self.vec.truncate(nbytes);
            self.nbits = len;
            self.set_unused_zero()
            }
        }

        /// Reserves capacity for at least additional more booleans to be inserted in the given
        /// `BitVec`. The collection may reserve more space to avoid frequent reallocations.
        pub fn resize(&mut self, new_len: usize, value: bool) {
            if new_len > self.len() {
                let additional = new_len - self.len();
                self.reserve(additional);
                for _ in 0..additional {
                    self.push(value);
                }
            } else {
                self.truncate(new_len);
            }
        }


        ////////////////////////////////////////
        // Helpers

        /// Sets the extra unused bits in the bitvector to 0.
        fn set_unused_zero(&mut self) {
            if self.nbits % 8 == 0 { return }
            let len = self.vec.len(); // avoid mutable borrow error
            assert!(len > 0);

            let byte = unsafe { self.vec.get_unchecked_mut(len - 1) };
            // Pattern with all 1's in the used bits only, avoiding overflow check in debug.
            let pattern = (Wrapping(1u8 << (self.nbits % 8)) - Wrapping(1u8)).0;
            *byte &= pattern;
        }
    }
}

#[cfg(not(feature = "unstable"))]
impl BitVec {
    impl_bitvec!(Vec<u8>);

    ////////////////////////////////////////
    // Iterators

    /// Returns an iterator for the booleans in the bitvec.
    pub fn iter(&self) -> Iter {
        self.into_iter()
    }
}

#[cfg(feature = "unstable")]
impl<A: Allocator> BitVec<A> {
    impl_bitvec!(Vec<u8, A>);

    ////////////////////////////////////////
    // Iterators

    /// Returns an iterator for the booleans in the bitvec.
    pub fn iter(&self) -> Iter<A> {
        self.into_iter()
    }
}

macro_rules! impl_display {
    () => {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            for (val, index) in self.iter().zip(0..usize::max_value()) {
                if index > 0 && index % 8 == 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", if val { "1" } else { "." })?;
            }
            Ok(())
        }
    }
}

#[cfg(not(feature = "unstable"))]
impl fmt::Debug for BitVec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BitVec{{{:?}: {}}}", self.nbits, &self)
    }
}

#[cfg(not(feature = "unstable"))]
impl fmt::Display for BitVec {
    impl_display!();
}

#[cfg(feature = "unstable")]
impl<A: Allocator> fmt::Debug for BitVec<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BitVec{{{:?}: {}}}", self.nbits, &self)
    }
}

#[cfg(feature = "unstable")]
impl<A: Allocator> fmt::Display for BitVec<A> {
    impl_display!();
}

impl Extend<bool> for BitVec {
    fn extend<T>(&mut self, iterable: T)
        where T: IntoIterator<Item = bool>
    {
        let iter = iterable.into_iter();
        let (min, max) = iter.size_hint();
        self.reserve(max.unwrap_or(min));
        for val in iter { self.push(val); }
    }
}

impl<'a> Extend<&'a bool> for BitVec {
    fn extend<T>(&mut self, iterable: T)
        where T: IntoIterator<Item = &'a bool>
    {
        let iter = iterable.into_iter();
        let (min, max) = iter.size_hint();
        self.reserve(max.unwrap_or(min));
        for val in iter { self.push(*val); }
    }
}

impl core::iter::FromIterator<bool> for BitVec {
    fn from_iter<T>(iterable: T) -> Self
        where T: IntoIterator<Item = bool>
    {
        let iter = iterable.into_iter();
        let (min, max) = iter.size_hint();
        let mut vec = BitVec::with_capacity(max.unwrap_or(min));
        for val in iter { vec.push(val); }
        vec
    }
}

impl<'a> core::iter::FromIterator<&'a bool> for BitVec {
    fn from_iter<T>(iterable: T) -> Self
        where T: IntoIterator<Item = &'a bool>
    {
        let iter = iterable.into_iter();
        let (min, max) = iter.size_hint();
        let mut vec = BitVec::with_capacity(max.unwrap_or(min));
        for &val in iter { vec.push(val); }
        vec
    }
}

impl From<&[bool]> for BitVec {
    fn from(bools: &[bool]) -> Self {
        BitVec::from_bools(bools)
    }
}

impl From<&Vec<bool>> for BitVec {
    fn from(bools: &Vec<bool>) -> Self {
        BitVec::from_bools(bools)
    }
}

impl From<Vec<bool>> for BitVec {
        fn from(bools: Vec<bool>) -> Self {
        BitVec::from_bools(&bools)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Iterators

macro_rules! impl_iter {
    () => {
        type Item = bool;

        fn size_hint(&self) -> (usize, Option<usize>) {
            let remaining = self.vec.len() - self.index;
            (remaining, Some(remaining))
        }

        fn count(self) -> usize {
            self.vec.len() - self.index
        }

        fn last(self) -> Option<Self::Item> {
            let len = self.vec.len();
            if self.index < len {
                Some(unsafe { self.vec.get_unchecked(len - 1) })
            } else {
                None
            }
        }

        fn nth(&mut self, count: usize) -> Option<Self::Item> {
            self.index = if count >= self.vec.nbits - self.index {
                self.vec.nbits
            } else {
                self.index + count
            };
            self.next()
        }

        fn next(&mut self) -> Option<Self::Item> {
            if self.index >= self.vec.nbits {
                None
            } else {
                let val = unsafe { self.vec.get_unchecked(self.index) };
                self.index += 1;
                Some(val)
            }
        }
    };
}

pub use self::iter::*;

#[cfg(not(feature = "unstable"))]
mod iter {
    use super::BitVec;

    /// Allows forward iteration through the bits of a bit vector.
    #[derive(Clone)]
    pub struct Iter<'a>
    {
        vec: &'a BitVec,
        index: usize,
    }

    /// Consumes and allows forward iteration through the bits of a bit vector.
    pub struct IntoIter
    {
        vec: BitVec,
        index: usize,
    }

    impl<'a> Iterator for Iter<'a> {
        impl_iter!();
    }

    impl Iterator for IntoIter {
        impl_iter!();
    }

    impl<'a> IntoIterator for &'a BitVec {
        type Item = bool;
        type IntoIter = Iter<'a>;
        fn into_iter(self) -> Self::IntoIter {
            Iter {
                vec: self,
                index: 0,
            }
        }
    }

    impl IntoIterator for BitVec {
        type Item = bool;
        type IntoIter = IntoIter;
        fn into_iter(self) -> Self::IntoIter {
            IntoIter {
                vec: self,
                index: 0,
            }
        }
    }
}

#[cfg(feature = "unstable")]
mod iter {
    use alloc::alloc::Global;
    use core::alloc::Allocator;
    use super::BitVec;

    /// Allows forward iteration through the bits of a bit vector.
    #[derive(Clone)]
    pub struct Iter<'a, A: Allocator = Global>
    {
        vec: &'a BitVec<A>,
        index: usize,
    }

    /// Consumes and allows forward iteration through the bits of a bit vector.
    pub struct IntoIter<A: Allocator = Global>
    {
        vec: BitVec<A>,
        index: usize,
    }

    impl<'a, A: Allocator> Iterator for Iter<'a, A> {
        impl_iter!();
    }

    impl<A: Allocator> Iterator for IntoIter<A> {
        impl_iter!();
    }

    impl<'a, A: Allocator> IntoIterator for &'a BitVec<A> {
        type Item = bool;
        type IntoIter = Iter<'a, A>;
        fn into_iter(self) -> Self::IntoIter {
            Iter::<A> {
                vec: self,
                index: 0,
            }
        }
    }

    impl<A: Allocator> IntoIterator for BitVec<A> {
        type Item = bool;
        type IntoIter = IntoIter<A>;
        fn into_iter(self) -> Self::IntoIter {
            IntoIter::<A> {
                vec: self,
                index: 0,
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Indexing operations

static TRUE: bool = true;
static FALSE: bool = false;

#[cfg(not(feature = "unstable"))]
impl core::ops::Index<usize> for BitVec {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        let value = unsafe { self.get_unchecked(index) };
        if value { &TRUE } else { &FALSE }
    }
}

#[cfg(feature = "unstable")]
impl<A: Allocator> core::ops::Index<usize> for BitVec<A> {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        let value = unsafe { self.get_unchecked(index) };
        if value { &TRUE } else { &FALSE }
    }
}
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use super::BitVec;
    use alloc::{vec::Vec, vec, format};

    #[test]
    fn test_index() {
        let vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        assert_eq!(vec[0], true);
        assert_eq!(vec[4], false);
        assert_eq!(vec[15], true);
    }

    #[test]
    fn test_constructors_for_empty() {
        let vec = BitVec::new();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.as_bytes(), &[]);

        let vec = BitVec::with_capacity(0);
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.as_bytes(), &[]);

        let vec = BitVec::with_capacity(1);
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 8);
        assert_eq!(vec.as_bytes(), &[]);

        let vec = BitVec::with_capacity(8);
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 8);
        assert_eq!(vec.as_bytes(), &[]);

        let vec = BitVec::with_capacity(9);
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 16);
        assert_eq!(vec.as_bytes(), &[]);
    }

    #[test]
    fn test_convert_to_bools() {
        let from: &[bool] = &[true, false, false, true, true, false, false, true, true, true, false];
        let vec: BitVec = BitVec::from_bools(from);
        let bools: Vec<bool> = (&vec).iter().collect();
        assert_eq!(bools, from);
        let bools: Vec<bool> = vec.iter().collect();
        assert_eq!(bools, from);
    }

    #[test]
    fn test_convert_from_bools() {
        use core::iter::FromIterator;

        let from: &[bool] = &[true, false, false, true, true, false, false, true, true, true, false];
        let vec: BitVec = BitVec::from_bools(from);
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.as_bytes(), &[0x99, 0x03]);

        let vec: BitVec = from.into();
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.as_bytes(), &[0x99, 0x03]);

        let from = &vec![true, false, false, true, true, false, false, true, true, true, false];
        let vec: BitVec = from.into();
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.as_bytes(), &[0x99, 0x03]);
        let vec = BitVec::from_iter(from);
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.as_bytes(), &[0x99, 0x03]);

        let from = vec![true, false, false, true, true, false, false, true, true, true, false];
        let vec: BitVec = from.clone().into();
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.as_bytes(), &[0x99, 0x03]);
        let vec = BitVec::from_iter(from);
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.as_bytes(), &[0x99, 0x03]);
    }

    #[test]
    fn test_constructors_from_bytes() {
        let vec = BitVec::from_bytes(&[0xab, 0xcd]);
        assert_eq!(vec.len(), 16);
        assert_eq!(vec.as_bytes(), &[0xab, 0xcd]);

        let vec = BitVec::from_elem(4, true);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec.as_bytes(), &[0x0f]);

        let vec = BitVec::from_elem(31, true);
        assert_eq!(vec.len(), 31);
        assert_eq!(vec.as_bytes(), &[0xff, 0xff, 0xff, 0x7f]);

        let vec = BitVec::from_elem(4, false);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec.as_bytes(), &[0]);

        let vec = BitVec::from_elem(31, false);
        assert_eq!(vec.len(), 31);
        assert_eq!(vec.as_bytes(), &[0, 0, 0, 0]);
    }

    #[test]
    fn test_with_bytes_mut() {
        let mut vec = BitVec::from_elem(28, false);
        assert_eq!(vec.len(), 28);
        assert_eq!(vec.as_bytes(), &[0, 0, 0, 0]);

        // Fill the underlying buffers with all 1s.
        vec.with_bytes_mut(|slice| {
            assert_eq!(slice.len(), 4);
            for i in 0..4 { slice[i] = 0xff; }
        });
        // Expect the unused bytes to be zeroed out.
        assert_eq!(vec.as_bytes(), &[0xff, 0xff, 0xff, 0x0f]);
    }

    #[test]
    fn test_into_bytes() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0xe3]);
        vec.pop(); vec.pop();
        assert_eq!(vec.len(), 54);
        let vec = vec.into_bytes();
        assert_eq!(vec, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23]);
    }

    #[test]
    fn test_get_set_index() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        assert_eq!(vec.as_bytes(), &[0xef, 0xa5, 0x71]);
        assert_eq!(Some(true), vec.get(8));
        assert_eq!(true, vec[8]);

        vec.set(8, true);
        assert_eq!(Some(true), vec.get(8));
        assert_eq!(true, vec[8]);
        assert_eq!(vec.as_bytes(), &[0xef, 0xa5, 0x71]);

        vec.set(8, false);
        assert_eq!(Some(false), vec.get(8));
        assert_eq!(false, vec[8]);
        assert_eq!(vec.as_bytes(), &[0xef, 0xa4, 0x71]);

        vec.set(7, false);
        assert_eq!(Some(false), vec.get(7));
        assert_eq!(false, vec[7]);
        assert_eq!(vec.as_bytes(), &[0x6f, 0xa4, 0x71]);

        assert_eq!(None, vec.get(vec.len()));
    }

    #[test]
    fn test_pop_to_empty() {
        let mut vec = BitVec::new();
        assert_eq!(vec.pop(), None);
        assert_eq!(vec.pop(), None);

        let mut vec = BitVec::from_bytes(&[0b01111111]);
        assert_eq!(vec.pop(), Some(false));
        assert_eq!(vec.len(), 7);
        for _ in 0..7 {
            assert_eq!(vec.pop(), Some(true));
        }
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.pop(), None);
        assert_eq!(vec.pop(), None);
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_pop_push() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0b11100011]);
        assert_eq!(vec.len(), 56);

        // Pop 2 bits and expect the slice view to show zeros for them.
        assert_eq!(vec.pop(), Some(true));
        assert_eq!(vec.pop(), Some(true));
        assert_eq!(vec.len(), 54);
        assert_eq!(vec.as_bytes(), &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0b00100011]);

        // Finish popping the byte and expect the slice to be one byte shorter.
        assert_eq!(vec.pop(), Some(true));
        assert_eq!(vec.pop(), Some(false));
        assert_eq!(vec.pop(), Some(false));
        assert_eq!(vec.pop(), Some(false));
        assert_eq!(vec.pop(), Some(true));
        assert_eq!(vec.pop(), Some(true));
        assert_eq!(vec.len(), 48);
        assert_eq!(vec.as_bytes(), &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45]);

        // Push another byte in.
        for _ in 0..4 {
            vec.push(true);
            vec.push(false);
        }
        assert_eq!(vec.len(), 56);
        assert_eq!(vec.as_bytes(), &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0b01010101]);
    }

    #[test]
    fn test_clear() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0xe3]);
        assert_eq!(vec.len(), 56);
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.as_bytes(), &[]);
    }

    fn assert_iter_eq<I: IntoIterator<Item=bool>>(vec: I, expected: &Vec<bool>) {
        let actual: Vec<bool> = vec.into_iter().collect();
        assert_eq!(&actual, expected);
    }

    #[test]
    fn test_iter() {
        let l = true;
        let o = false;

        assert_iter_eq(&BitVec::new(), &Vec::new());

        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        // low bit to high bit:       f       e        5       a        1       7
        assert_iter_eq(&vec, &vec![l,l,l,l,o,l,l,l, l,o,l,o,o,l,o,l, l,o,o,o,l,l,l,o]);
        vec.pop(); vec.pop();
        
        // low bit to high bit:       f       e        5       a        1     3
        assert_iter_eq(&vec, &vec![l,l,l,l,o,l,l,l, l,o,l,o,o,l,o,l, l,o,o,o,l,l]);
    }

    #[test]
    fn test_into_iter() {
        let l = true;
        let o = false;

        assert_iter_eq(&BitVec::new(), &Vec::new());

        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        // low bit to high bit:       f       e        5       a        1       7
        assert_iter_eq(vec.clone(), &vec![l,l,l,l,o,l,l,l, l,o,l,o,o,l,o,l, l,o,o,o,l,l,l,o]);
        vec.pop(); vec.pop();

        // low bit to high bit:       f       e        5       a        1     3
        assert_iter_eq(vec.clone(), &vec![l,l,l,l,o,l,l,l, l,o,l,o,o,l,o,l, l,o,o,o,l,l]);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn test_set_validation() {
        let _ = &BitVec::from_bytes(&[0xef, 0xa5, 0x71]).set(24, true);
    }

    #[test]
    fn test_eq() {
        let vec1 = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        let mut vec2 = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        assert!(vec1 == vec2);
        vec2.push(true);
        assert!(vec1 != vec2);
        vec2.pop();
        assert!(vec1 == vec2);
        vec2.set(3, false);
        assert!(vec1 != vec2);
    }

    #[test]
    fn test_clone() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        assert_eq!(vec, vec.clone());
        vec.pop(); vec.pop();
        assert_eq!(vec, vec.clone());
    }

    #[test]
    fn test_debug() {
        assert_eq!(
            format!("{:?}", &BitVec::from_bytes(&[0xef, 0xa5, 0x71])),
            "BitVec{24: 1111.111 1.1..1.1 1...111.}"
        )
    }

    #[test]
    fn test_swap() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        vec.swap(0, 23);
        assert_eq!(vec.len(), 24);
        assert_eq!(vec.as_bytes(), &[0xee, 0xa5, 0xf1]);
        vec.swap(0, 5);
        assert_eq!(vec.len(), 24);
        assert_eq!(vec.as_bytes(), &[0xcf, 0xa5, 0xf1]);
    }

    #[test]
    fn test_capacity_reserve() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);
        assert_eq!(vec.len(), 24);
        assert!(vec.capacity() >= vec.len());
        let new_capacity = 2 * vec.capacity();
        vec.reserve(new_capacity);
        assert!(vec.capacity() >= new_capacity);
    }

    #[test]
    fn test_truncate_extend() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);

        vec.truncate(25);
        assert_eq!(vec.len(), 24);
        assert_eq!(vec.as_bytes(), &[0xef, 0xa5, 0x71]);

        vec.truncate(12);
        assert_eq!(vec.len(), 12);
        assert_eq!(vec.as_bytes(), &[0xef, 0x05]);

        vec.extend(core::iter::repeat(true).take(5));
        assert_eq!(vec.len(), 17);
        assert_eq!(vec.as_bytes(), &[0xef, 0xf5, 0x01]);

        vec.extend(core::iter::repeat(&true).take(6));
        assert_eq!(vec.len(), 23);
        assert_eq!(vec.as_bytes(), &[0xef, 0xf5, 0x7f]);
    }

    #[test]
    fn test_resize() {
        let mut vec = BitVec::from_bytes(&[0xef, 0xa5, 0x71]);

        vec.resize(24, true);
        assert_eq!(vec.len(), 24);
        assert_eq!(vec.as_bytes(), &[0xef, 0xa5, 0x71]);

        vec.resize(12, true);
        assert_eq!(vec.len(), 12);
        assert_eq!(vec.as_bytes(), &[0xef, 0x05]);

        vec.resize(17, true);
        assert_eq!(vec.len(), 17);
        assert_eq!(vec.as_bytes(), &[0xef, 0xf5, 0x01]);
    }

    #[test]
    fn test_iter_overrides() {
        let from: &[bool] = &[true, false, false, true, true, false, false, true, true, true, false];
        let vec = BitVec::from_bools(from);
        assert_eq!(vec.len(), 11);
        assert_eq!(vec.iter().size_hint(), (11, Some(11)));
        assert_eq!(vec.iter().count(), 11);
        assert_eq!(vec.iter().last(), Some(false));

        // nth from scratch
        for (index, &b) in from.iter().enumerate() {
            assert_eq!(vec.iter().nth(index), Some(b));
        }
        assert_eq!(vec.iter().nth(11), None);

        // partially-consumed iterators
        let mut iter = vec.iter();
        for (index, &b) in from.iter().enumerate() {
            assert_eq!(iter.size_hint(), (11 - index, Some(11 - index)));
            assert_eq!(iter.clone().count(), 11 - index);
            assert_eq!(iter.clone().last(), Some(false));
            assert_eq!(iter.nth(0), Some(b));
        }
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.clone().count(), 0);
        assert_eq!(iter.clone().last(), None);
        assert_eq!(iter.nth(0), None);
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn test_custom_allocator() {
        use alloc::alloc::Global;
        
        let mut vec = Vec::new_in(Global);
        vec.push(false);
        vec.push(true);
        assert_eq!(vec[1], true);
    }
}
