use derive_more::Deref;
use memchr::{memchr, memchr2, memchr3, memmem};

use crate::ext::CharExt;

/// A lightweight, zero-copy cursor over a byte slice.
///
/// This `struct` is named such to avoid confusion with the actual record
/// of the current location, `pos`. Unlike a standard `Iterator`, `Tape` is designed specifically for
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
/// The name *"pos"* is used to distinguish indices in a `Tape` from other data structures.
/// It is not guaranteed to be within the acceptable range of indices at any given point,
/// but member functions assume so.
/// 
/// For ergonomics, exhaustion is typically checked by determining whether `cur()` or `next()`
/// are `None`. However, this may also occur if the cursor is decremented to **before** the
/// first element in the underlying slice. For a more comprehensive check, use `is_exhausted()`.
/// 
/// # API
///
/// ## Indexing
/// Returns `Option<T>`.
///
/// | Function | Returns | Side Effects |
/// | :--- | :--- | :--- |
/// | `cur` | Character at current position | None |
/// | `next` | Character at current position | Advances `pos` after index |
/// | `peek` | Character after current position | None |
/// | `peek_back` | Character before current position | None |
///
/// ## Scanning
/// | Function | Purpose | Returns |
/// | :--- | :--- | :--- |
/// | `poll`/`poll_back` | Find position of character forward/backward | `Option<usize>` |
/// | `consume`/`put_back` | Jump over substring forward/backward | `&[T]` |
/// | `seek`/`seek_back` | Jump to character forward/backward if it exists | `bool` |
///
/// ## u8 Specializations
///
/// ### Optimized Seek
/// Jumps to the target and returns `bool`. Optimized using Two-Way search algorithm.
///
/// | Function | Target |
/// | :--- | :--- |
/// | `seek_ch` | Character |
/// | `seek_ch2` | String (len = 2) |
/// | `seek_ch3` | String (len = 3) |
/// | `seek_ch_in_pgraph` | Character w.r.t. spacing |
///
/// ### Other Seek
/// Jumps to the target and returns `bool`.
///
/// | Function | Target |
/// | :--- | :--- |
/// | `seek_at` | String |
/// | `seek_at_in_pgraph` | String w.r.t. spacing |
/// | `seek_in_pgraph` | Character w.r.t. spacing |
///
/// ### Spatial Logic
/// Whether a character is considered whitespace is determined by `CharExt.is_file_ws`.
///
/// | Function | Predicate (Returns `bool`) |
/// | :--- | :--- |
/// | `is_any_clear` | Whitespace before AND after |
/// | `is_l_clear` | Whitespace before |
/// | `is_r_clear` | Whitespace after |
/// | `is_prefix` | First non-whitespace character in line at index |
/// | `is_cur_prefix` | First non-whitespace character in current line |
///
/// ### Miscellaneous
/// | Function | Purpose | Returns |
/// | :--- | :--- | :--- |
/// | `count_indent` | Counts leading tabs in the current line | `usize` |
///
/// # Implementation
/// `#[inline(always)]` should be restricted to functions called often in the
/// main `Scanner`/`Grammar` recursions, where the benefit of inlining is completely certain.
///
/// `raw` should be hidden from users to promote orthogonality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub struct Tape<'a, T> {
    #[deref]
    raw: &'a [T],

    pub pos: usize,
}

impl<'a, T> Tape<'a, T> {
    #[inline]
    #[must_use]
    pub const fn new(raw: &'a [T]) -> Self {
        Self { raw, pos: 0 }
    }

    /// Returns a subslice over the original slice from the current position.
    #[inline(always)]
    #[must_use]
    pub fn rest(self) -> &'a [T] {
        &self.raw[self.pos..self.raw.len()]
    }

    /// Advances the current position by 1 element.
    #[inline(always)]
    pub const fn adv(&mut self) {
        self.pos += 1;
    }

    /// Decrements the current position by 1 element.
    #[inline(always)]
    pub const fn dec(&mut self) {
        self.pos -= 1;
    }

    /// Returns true if the cursor is past the last element.
    pub const fn is_exhausted(&mut self) -> bool {
        self.pos >= self.raw.len()
    }
}

impl<'a, T: Copy + PartialEq> Tape<'a, T> {
    /// Returns the **current** element, if exists, before incrementing the current position.
    ///
    /// This function is primarily used for iteration.
    /// If used for iteration, the current position may be modified concurrently.
    ///
    /// If the tape is exhausted, `pos` will still be incremented.
    #[inline(always)]
    #[must_use]
    pub fn next(&mut self) -> Option<T> {
        let elem = self.raw.get(self.pos);
        self.pos += 1;
        elem.map(|e| *e)
    }

    /// Returns the current element, or `None` if `pos` is out of bounds.
    #[must_use]
    #[inline(always)]
    pub const fn cur(self) -> Option<T> {
        if self.pos < self.raw.len() {
            Some(self.raw[self.pos])
        } else {
            None
        }
    }

    /// Returns the element at `pos + 1`, or `None` if that position is out of bounds.
    #[must_use]
    #[inline(always)]
    pub const fn peek(self) -> Option<T> {
        let pos = self.pos + 1;
        if pos < self.raw.len() {
            Some(self.raw[pos])
        } else {
            None
        }
    }

    /// Returns the element at `pos - 1`, or `None` if that position is out of bounds.
    #[must_use]
    #[inline(always)]
    pub const fn peek_back(self) -> Option<T> {
        let pos = self.pos - 1;
        if pos < self.raw.len() {
            Some(self.raw[pos])
        } else {
            None
        }
    }

    /// Returns the position of the first element returning true, or `None`.
    #[must_use]
    #[inline]
    pub fn poll<F>(self, mut pred: F) -> Option<usize>
    where
        F: FnMut(T, usize) -> bool,
    {
        (self.pos..self.raw.len()).find(|&pos| pred(self.raw[pos], pos))
    }

    /// Returns the position of the last element returning true, or `None`.
    #[must_use]
    #[inline]
    pub fn poll_back<F>(self, mut pred: F) -> Option<usize>
    where
        F: FnMut(T, usize) -> bool,
    {
        (self.pos..self.raw.len())
            .rev()
            .find(|&pos| pred(self.raw[pos], pos))
    }

    /// Advance `pos` until `pred` returns false for the element at the
    /// current position.
    ///
    /// Leaves `pos` pointing at the matching element (or at `text.len()` when none matched).
    /// Returns the subslice iterated over.
    #[inline]
    pub fn consume<F>(&mut self, mut pred: F) -> &'a [T]
    where
        F: FnMut(T, usize) -> bool,
    {
        match self.poll(|elem, pos| !pred(elem, pos)) {
            None => &self.raw[0..0],
            Some(pos) => {
                let res = &self.raw[self.pos..pos];
                self.pos = pos;
                res
            }
        }
    }

    /// Decrement `pos` until `pred` returns false for the element at the
    /// current position.
    ///
    /// Leaves `pos` pointing at the matching element (or at `text.len()` when none matched).
    /// Returns the subslice iterated over.
    #[inline]
    pub fn put_back<F>(&mut self, mut pred: F) -> &'a [T]
    where
        F: FnMut(T, usize) -> bool,
    {
        match self.poll_back(|elem, pos| !pred(elem, pos)) {
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
        F: FnMut(T, usize) -> bool,
    {
        match self.poll(pred) {
            None => false,
            Some(pos) => {
                self.pos = pos;
                true
            }
        }
    }

    /// Decrements `pos` to the first index where `pred` is true.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    #[inline]
    pub fn seek_back<F>(&mut self, pred: F) -> bool
    where
        F: FnMut(T, usize) -> bool,
    {
        match self.poll_back(pred) {
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
    pub fn is_at(self, query: &[T]) -> bool {
        self.raw[self.pos..].starts_with(query)
    }
}

/// `elem` should be used as lambda argument in case any function should be made generic.
impl<'a> Tape<'a, u8> {
    /// Consumes the object let notation key at the current position,
    /// returning it if one exists.
    /// 
    /// If one does not exist, an empty slice is returned.
    ///
    /// See `CharExt` for more details.
    #[inline]
    pub fn consume_file_key(&mut self) -> &'a [u8] {
        if self.cur().is_none_or(|ch| !ch.is_file_key_start()) {
            return &self.raw[0..0];
        }
        let start = self.pos;
        self.adv();
        let rest_len = self.consume(|ch, _| ch.is_file_key_part()).len();
        &self.raw[start..start + 1 + rest_len]
    }

    /// Returns true if the character at the given position has clearance on its left side.
    #[must_use]
    #[inline]
    pub fn is_l_clear(self, pos: usize) -> bool {
        pos == 0 || self.raw.get(pos - 1).is_none_or(|elem| elem.is_file_ws())
    }

    /// Returns true if the character at the given position has clearance on its right side.
    #[must_use]
    #[inline]
    pub fn is_r_clear(self, pos: usize) -> bool {
        self.raw.get(pos + 1).is_none_or(|elem| elem.is_file_ws())
    }

    /// Returns true if the character cluster whose last character is at
    /// the current position has the correct clearance to be a closer
    /// (has clearance on either side).
    #[must_use]
    #[inline]
    pub fn is_any_clear(self, start: usize) -> bool {
        !self.is_l_clear(start) || self.is_r_clear(self.pos)
    }

    /// Returns the position of the first character returning true,
    /// respecting paragraph spacing rules, or `None`.
    #[must_use]
    #[inline]
    pub fn poll_in_pgraph<F>(self, spacing: u8, mut pred: F) -> Option<usize>
    where
        F: FnMut(u8, usize) -> bool,
    {
        let text = self.raw;
        let mut nl_count = 0;
        for (i, &elem) in text.iter().enumerate() {
            if elem == b'\n' {
                nl_count += 1;
                if nl_count >= spacing {
                    return None;
                }
            } else {
                nl_count = 0;
            }
            if pred(elem, i) {
                return Some(i);
            }
        }
        None
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
        match self.poll_in_pgraph(spacing, |elem, pos| !pred(elem, pos)) {
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

    /// Returns true if the current character belongs to a line prefix.
    ///
    /// A character is part of a line prefix if there are no non-whitespace characters between
    /// the current character and the previous newline, the beginning of the input, or
    /// itself if it is a newline.
    #[must_use]
    #[inline]
    pub fn is_cur_prefix(self) -> bool {
        self.is_prefix(self.pos)
    }

    /// Returns true if there are no non-whitespace characters between
    /// the given character and the previous newline, the beginning of the input, or
    /// itself if it is a newline.
    #[must_use]
    #[inline]
    pub fn is_prefix(self, pos: usize) -> bool {
        for i in (0..pos).rev() {
            let c = self.raw[i]; // this is safe because i < self.pos
            if c == b'\n' {
                return true;
            }
            if !c.is_file_ws() {
                return false;
            }
        }
        true
    }

    /// Returns the number of times the current line is indented.
    ///
    /// Counts the number of tabs or the number of space characters divided by 4 (floored).
    ///
    /// Used to determine separation between table cells and indentation of list items.
    #[must_use]
    #[inline]
    pub fn count_indent(self) -> u8 {
        let ws = &self.raw[self.poll_back(|elem, _| elem == b'\n').unwrap_or(0)..self.pos];
        let (tabs, spaces) = ws.iter().fold((0, 0), |(t, s), &elem| match elem {
            b'\t' => (t + 1, s),
            b' ' => (t, s + 1),
            _ => (t, s),
        });
        tabs + (spaces / 4)
    }
}
