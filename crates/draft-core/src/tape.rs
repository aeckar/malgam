use crate::ext::CharExt;
use memchr::{memchr, memchr2, memchr3, memmem, memrchr2};
use std::ops::{Index,Range};

/// Counts the number of tabs or the number of space characters divided by 4 (floored).
/// 
/// Used to determine separation between table cells and indentation of list items.
/// For optimal performance, the given string should only consist of whitespace characters.
/// 
/// This is left private, as users should convert text to `Tape` first.
fn count_indent(ws: &[u8]) -> u8 {
let (tabs, spaces) = ws.iter().fold((0, 0), |(t, s), &ch| match ch {
        b'\t' => (t + 1, s),
        b' ' => (t, s + 1),
        _ => (t, s),
    });
    tabs + (spaces / 4)
}

/// A lightweight, zero-copy cursor over a byte slice.
///
/// Unlike a standard `Iterator`, `Tape` is designed specifically for 
/// non-linear parsing:
/// 
/// * **Backtracking:** Supports moving the cursor backward (`dec`, `peek_back`) 
///   and random access via slicing, which is essential for multi-character 
///   delimiters and lookbehind checks.
/// * **Zero-Copy Slicing:** Because it retains a reference to the `raw` buffer, 
///   methods like `consume` can return efficient `&[u8]` sub-slices without 
///   allocating new memory.
/// * **State Snapshots:** Since `Tape` is `Copy`, it can be cheaply duplicated 
///   to "try" a parsing branch and then discarded if the branch fails, 
///   restoring the original position instantly.
/// 
/// `pos` is used to distinguish indices in a `Tape` from other data structures.
/// It is not guaranteed to be within the acceptable range of indices at any given point,
/// but member functions assume so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tape<'a> {
    pub raw: &'a [u8],
    pub pos: usize,
}

impl<'a> Index<usize> for Tape<'a> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.raw[index]
    }
}

impl<'a> Iterator for Tape<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let ch = *self.raw.get(self.pos)?;
        self.pos += 1;
        Some(ch)
    }
}

impl<'a> Tape<'a> {
    pub fn new(raw: &'a [u8]) -> Self {
        Self { raw, pos: 0 }
    }

    pub unsafe fn to_uf8_unchecked(&self) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.raw[self.pos..]) }
    }

    /// Returns the **current** character, if exists, before incrementing the current position.
    ///
    /// This function is primarily used for iteration.
    /// If used for iteration, the current position may be modified concurrently.
    #[inline(always)]
    pub fn next(&mut self) -> Option<&u8> {
        let ch = self.raw.get(self.pos);
        self.pos += 1;
        ch
    }

    /// Advances the current position by 1 character.
    #[inline(always)]
    pub fn adv(&mut self) {
        self.pos += 1;
    }

    /// Decrements the current position by 1 character.
    #[inline(always)]
    pub fn dec(&mut self) {
        self.pos -= 1;
    }

    /// Returns the current character, or `None` if `pos` is out of bounds.
    #[must_use]
    #[inline(always)]
    pub const fn cur(&self) -> Option<u8> {
        if self.pos < self.raw.len() {
            Some(self.raw[self.pos])
        } else {
            None
        }
    }

    /// Returns the character at `pos + 1`, or `None` if that position is out of bounds.
    #[must_use]
    #[inline(always)]
    pub const fn peek(&self) -> Option<u8> {
        let pos = self.pos + 1;
        if pos < self.raw.len() {
            Some(self.raw[pos])
        } else {
            None
        }
    }

    /// Returns the character at `pos - 1`, or `None` if that position is out of bounds.
    #[must_use]
    #[inline(always)]
    pub const fn peek_back(&self) -> Option<u8> {
        let pos = self.pos - 1;
        if pos < self.raw.len() {
            Some(self.raw[pos])
        } else {
            None
        }
    }

    /// Returns true if the character at the given position has clearance on its left side.
    #[must_use]
    #[inline]
    pub fn is_l_clear(&self, pos: usize) -> bool {
        pos == 0 || self.raw.get(pos - 1).is_none_or(|ch| ch.is_hg_ws())
    }

    /// Returns true if the character at the given position has clearance on its right side.
    #[must_use]
    #[inline]
    pub fn is_r_clear(&self, pos: usize) -> bool {
        self.raw.get(pos + 1).is_none_or(|ch| ch.is_hg_ws())
    }

    /// Returns true if the character cluster whose last character is at
    /// the current position has the correct clearance to be a closer
    /// (has clearance on either side).
    #[must_use]
    #[inline]
    pub fn is_any_clear(&self, start: usize) -> bool {
        !self.is_l_clear(start) || self.is_r_clear(self.pos)
    }

    /// Returns the position of the first character returning true, or `None`.
    #[must_use]
    #[inline]
    pub fn poll<F>(&self, mut pred: F) -> Option<usize>
    where
        F: FnMut(u8, usize) -> bool,
    {
        (self.pos..self.raw.len()).find(|&pos| pred(self.raw[pos], pos))
    }

    /// Returns the position of the last character returning true, or `None`.
    #[must_use]
    #[inline]
    pub fn poll_back<F>(&self, mut pred: F) -> Option<usize>
    where
        F: FnMut(u8, usize) -> bool,
    {
        (self.pos..self.raw.len())
            .rev()
            .find(|&pos| pred(self.raw[pos], pos))
    }

    /// Returns the position of the first character returning true,
    /// respecting paragraph spacing rules, or `None`.
    #[must_use]
    #[inline]
    pub fn poll_in_pgraph<F>(&self, spacing: u8, mut pred: F) -> Option<usize>
    where
        F: FnMut(u8, usize) -> bool,
    {
        let text = self.raw;
        let mut nl_count = 0;
        for (i, &ch) in text.iter().enumerate() {
            if ch == b'\n' {
                nl_count += 1;
                if nl_count >= spacing {
                    return None;
                }
            } else {
                nl_count = 0;
            }
            if pred(ch, i) {
                return Some(i);
            }
        }
        None
    }

    #[inline]
    pub fn slice(&self, range: Range<usize>) -> &'a [u8] {
        &self.raw[range]
    }

    /// Advance `pos` until `pred` returns false for the character at the
    /// current position.
    ///
    /// Leaves `pos` pointing at the matching character (or at `text.len()` when none matched).
    /// Returns the subslice iterated over.
    #[inline]
    pub fn consume<F>(&mut self, mut pred: F) -> &'a [u8]
    where
        F: FnMut(u8, usize) -> bool,
    {
        match self.poll(|ch, pos| !pred(ch, pos)) {
            None => &self.raw[0..0],
            Some(pos) => {
                let res = &self.raw[self.pos..pos];
                self.pos = pos;
                res
            }
        }
    }

    /// Decrement `pos` until `pred` returns false for the character at the
    /// current position.
    ///
    /// Leaves `pos` pointing at the matching character (or at `text.len()` when none matched).
    /// Returns the subslice iterated over.
    #[inline]
    pub fn put_back<F>(&mut self, mut pred: F) -> &'a [u8]
    where
        F: FnMut(u8, usize) -> bool,
    {
        match self.poll_back(|ch, pos| !pred(ch, pos)) {
            None => &self.raw[0..0],
            Some(pos) => {
                let res = &self.raw[self.pos..pos];
                self.pos = pos;
                res
            }
        }
    }

    /// Advances `pos` until `pred` returns false for the character at the
    /// current position, respecting paragraph spacing rules.
    ///
    /// Leaves `pos` pointing at the matching character (or at `text.len()` when none matched).
    /// Returns the subslice iterated over.
    #[inline]
    pub fn consume_in_pgraph<F>(&mut self, spacing: u8, mut pred: F) -> &'a [u8]
    where
        F: FnMut(u8, usize) -> bool,
    {
        match self.poll_in_pgraph(spacing, |ch, pos| !pred(ch, pos)) {
            None => &self.raw[0..0],
            Some(pos) => {
                let res = &self.raw[self.pos..pos];
                self.pos = pos;
                res
            }
        }
    }

    /// Advances `pos` to the first index where `pred` is true.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    #[inline]
    pub fn seek<F>(&mut self, pred: F) -> bool
    where
        F: FnMut(u8, usize) -> bool,
    {
        match self.poll(pred) {
            None => false,
            Some(pos) => {
                self.pos = pos;
                true
            }
        }
    }

    /// Advances `pos` to the first index where `pred` is true.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    /// 
    /// 
    /// Optimized for single byte search using SIMD.
    #[inline]
    pub fn seek_ch(&mut self, query: u8) -> bool {
        if let Some(offset) = memchr(query, &self.raw[self.pos..]) {
            self.pos += offset;
            return true;
        }
        false
    }

    /// Advances `pos` to the first index where `pred` is true.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    /// 
    /// 
    /// Optimized for single byte search using SIMD.
    #[inline]
    pub fn seek_ch2(&mut self, ch0: u8, ch1: u8) -> bool {
        if let Some(offset) = memchr2(ch0, ch1, &self.raw[self.pos..]) {
            self.pos += offset;
            return true;
        }
        false
    }

    /// Advances `pos` to the first index where `pred` is true.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    /// 
    /// 
    /// Optimized for single byte search using SIMD.
    #[inline]
    pub fn seek_ch3(&mut self, ch0: u8, ch1: u8, ch2: u8) -> bool {
        if let Some(offset) = memchr3(ch0, ch1, ch2, &self.raw[self.pos..]) {
            self.pos += offset;
            return true;
        }
        false
    }

    /// Advances `pos` to where `query` is found.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    /// 
    /// Optimized using Two-Way search algorithm.
    #[inline]
    pub fn seek_at(&mut self, query: &'a [u8]) -> bool {
if let Some(offset) = memmem::find(&self.raw[self.pos..], query) {
            self.pos += offset;
            return true;
        }
        false
    }

    /// Advances `pos` until `query` is found within the current paragraph.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    #[inline]
    pub fn seek_at_in_pgraph(&mut self, spacing: u8, query: &'a [u8]) -> bool {
        self.seek_in_pgraph(spacing, |_, pos| self.raw[pos..].starts_with(query))
    }

    /// Advances `pos` until `query` is found within the current paragraph.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    /// 
    /// For multi-byte sequences, use `seek_at_in_pgraph`.
    #[inline]
    pub fn seek_ch_in_pgraph(&mut self, spacing: u8, query: u8) -> bool {
        self.seek_in_pgraph(spacing, |_, pos| self.raw[pos] == query)
    }

    /// Advances `pos` until `pred` returns true within the current paragraph.
    ///
    /// Returns `true` if found (leaving `pos` at the match), or `false`
    /// and `pos` is restored to its original value.
    #[inline]
    pub fn seek_in_pgraph<F>(&mut self, spacing: u8, pred: F) -> bool
    where
        F: FnMut(u8, usize) -> bool,
    {
        match self.poll_in_pgraph(spacing, pred) {
            None => false,
            Some(pos) => {
                self.pos = pos;
                true
            }
        }
    }

    /// Returns true if the substring starting at the current position
    /// starts with the given string.
    #[must_use]
    #[inline]
    pub fn is_at(&self, query: &'_ [u8]) -> bool {
        self.raw[self.pos..].starts_with(query)
    }

    /// Returns true if the current character belongs to a line prefix.
    /// 
    /// A character is part of a line prefix if there are no non-whitespace characters between
    /// the current character and the previous newline, the beginning of the input, or
    /// itself if it is a newline.
    #[must_use]
    #[inline]
    pub fn is_cur_prefix(&self) -> bool {
        self.is_prefix(self.pos)
    }

    /// Returns true if there are no non-whitespace characters between
    /// the given character and the previous newline, the beginning of the input, or
    /// itself if it is a newline.
    #[must_use]
    #[inline]
    pub fn is_prefix(&self, pos: usize) -> bool {
        for i in (0..pos).rev() {
            let c = self.raw[i]; // This is safe because i < self.pos
            if c == b'\n' {
                return true;
            }
            if !c.is_hg_ws() {
                return false;
            }
        }
        true
    }

    /// Returns the number of times the current line is indented.
    #[must_use]
    pub fn count_indent(&self) -> u8 {
        count_indent(&self.raw[self.poll_back(|ch, _| ch == b'\n').unwrap_or(0)..self.pos])
    }
}