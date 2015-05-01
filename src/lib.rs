//! This crates provides a performant iterator over a linear range of
//! characters.
//!
//! The iterator is inclusive of its endpoint, and correctly handles
//! the surrogate range (`0xD800`-`0xDFFF`). This induces only one
//! extra branch (or conditional-move) compared to a direct `x..y`
//! integer iterator that doesn't handle the surrogate range.
//!
//! [Source](https://github.com/huonw/char-iter)
//!
//! # Installation
//!
//! Add this to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! char-iter = "0.1"
//! ```
//!
//! # Examples
//!
//! ```rust
//! let v: Vec<char> = char_iter::new('a', 'f').collect();
//! assert_eq!(v, &['a', 'b', 'c', 'd', 'e', 'f']);
//! ```
//!
//! Reverse iteration is supported:
//!
//! ```rust
//! // (codepoints 224 to 230)
//! let v: Vec<char> = char_iter::new('à', 'æ').rev().collect();
//! assert_eq!(v, &['æ', 'å', 'ä', 'ã', 'â', 'á', 'à']);
//! ```
//!
//! The surrogate range is skipped:
//!
//! ```rust
//! let v: Vec<char> = char_iter::new('\u{D7FF}', '\u{E000}').collect();
//! // 0xD800, ... 0xDFFF are missing
//! assert_eq!(v, &['\u{D7FF}', '\u{E000}']);
//! ```

#![cfg_attr(all(test, feature = "unstable"), feature(test))]

/// An iterator over a linear range of characters.
///
/// This is constructed by the `new` function at the top level.
pub struct Iter {
    start: char,
    end: char,
    finished: bool,
}

/// Create a new iterator over the characters (specifically Unicode
/// Scalar Values) from `start` to `end`, inclusive.
///
/// # Panics
///
/// This panics if `start > end`.
pub fn new(start: char, end: char) -> Iter {
    assert!(start <= end);
    Iter {
        start: start,
        end: end,
        finished: false
    }
}

const SUR_START: u32 = 0xD800;
const SUR_END: u32 = 0xDFFF;
const BEFORE_SUR: u32 = SUR_START - 1;
const AFTER_SUR: u32 = SUR_END + 1;

enum Dir { Forward, Backward }

#[inline(always)]
fn step(c: char, d: Dir) -> char {
    let val = c as u32;
    let new_val = match d {
        Dir::Forward => if val == BEFORE_SUR {AFTER_SUR} else {val + 1},
        Dir::Backward => if val == AFTER_SUR {BEFORE_SUR} else {val - 1},
    };
    debug_assert!(std::char::from_u32(new_val).is_some());
    unsafe {std::mem::transmute(new_val)}
}

impl Iterator for Iter {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.finished {
            return None
        }
        let ret = Some(self.start);
        if self.start == self.end {
            self.finished = true;
        } else {
            self.start = step(self.start, Dir::Forward)
        }
        ret
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = if self.finished {
            0
        } else {
            let start = self.start as u32;
            let end = self.end as u32;
            let naive_count = (end - start + 1) as usize;
            if start <= BEFORE_SUR && end >= AFTER_SUR {
                naive_count - (SUR_END - SUR_START + 1) as usize
            } else {
                naive_count
            }
        };
        (len, Some(len))
    }
}
impl DoubleEndedIterator for Iter {
    fn next_back(&mut self) -> Option<char> {
        if self.finished {
            return None
        }
        let ret = Some(self.end);
        if self.start == self.end {
            self.finished = true;
        } else {
            self.end = step(self.end, Dir::Backward)
        }
        ret
    }
}

impl ExactSizeIterator for Iter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let v: Vec<char> = new('a', 'f').collect();
        assert_eq!(v, &['a', 'b', 'c', 'd', 'e', 'f']);
    }
    #[test]
    fn smoke_rev() {
        let v: Vec<char> = new('a', 'f').rev().collect();
        assert_eq!(v, &['f', 'e', 'd', 'c', 'b', 'a']);
    }
    #[test]
    fn smoke_size_hint() {
        let mut iter = new('a', 'f');
        assert_eq!(iter.size_hint(), (6, Some(6)));
        for i in (0..6).rev() {
            iter.next();
            assert_eq!(iter.size_hint(), (i, Some(i)));
        }
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }
    #[test]
    fn smoke_rev_size_hint() {
        let mut iter = new('a', 'f').rev();
        assert_eq!(iter.size_hint(), (6, Some(6)));
        for i in (0..6).rev() {
            iter.next();
            assert_eq!(iter.size_hint(), (i, Some(i)));
        }
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }
    #[test]
    fn equal() {
        let v: Vec<char> = new('a', 'a').collect();
        assert_eq!(v, &['a']);
    }
    #[test]
    fn equal_rev() {
        let v: Vec<char> = new('a', 'a').rev().collect();
        assert_eq!(v, &['a']);
    }
    #[test]
    fn equal_size_hint() {
        let mut iter = new('a', 'a');
        assert_eq!(iter.size_hint(), (1, Some(1)));
        for i in (0..1).rev() {
            iter.next();
            assert_eq!(iter.size_hint(), (i, Some(i)));
        }
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }
    #[test]
    fn equal_rev_size_hint() {
        let mut iter = new('a', 'a').rev();
        assert_eq!(iter.size_hint(), (1, Some(1)));
        for i in (0..1).rev() {
            iter.next();
            assert_eq!(iter.size_hint(), (i, Some(i)));
        }
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    const S: char = '\u{D7FF}';
    const E: char = '\u{E000}';
    #[test]
    fn surrogate() {
        let v: Vec<char> = new(S, E).collect();
        assert_eq!(v, &[S, E]);
    }
    #[test]
    fn surrogate_rev() {
        let v: Vec<char> = new(S, E).rev().collect();
        assert_eq!(v, &[E, S]);
    }
    #[test]
    fn surrogate_size_hint() {
        let mut iter = new(S, E);
        assert_eq!(iter.size_hint(), (2, Some(2)));
        for i in (0..2).rev() {
            iter.next();
            assert_eq!(iter.size_hint(), (i, Some(i)));
        }
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }
    #[test]
    fn surrogate_rev_size_hint() {
        let mut iter = new(S, E).rev();
        assert_eq!(iter.size_hint(), (2, Some(2)));
        for i in (0..2).rev() {
            iter.next();
            assert_eq!(iter.size_hint(), (i, Some(i)));
        }
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn full_range() {
        let iter = new('\u{0}', '\u{10FFFF}');
        let mut count = 1_114_112 - 2048;
        assert_eq!(iter.size_hint(), (count, Some(count)));

        for (i, c) in (0..0xD800).chain(0xE000..0x10FFFF + 1).zip(iter) {
            assert_eq!(::std::char::from_u32(i).unwrap(), c);
            count -= 1;
        }
        assert_eq!(count, 0);
    }

    #[should_panic]
    #[test]
    fn invalid() {
        new('b','a');
    }
}
#[cfg(all(test, feature = "unstable"))]
mod benches {
    use super::*;
    extern crate test;

    #[bench]
    fn count(b: &mut test::Bencher) {
        b.iter(|| new('\u{0}', '\u{10FFFF}').count())
    }
    #[bench]
    fn count_baseline(b: &mut test::Bencher) {
        // this isn't the same range or the same length, but it's
        // close enough.
        b.iter(|| (0..0x10FFFF + 1).count())
    }
}
