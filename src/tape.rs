use crate::{char_ext::CharExt, etc::count_indent};

/// Text input and an index associated with an element in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tape<'a> {
    /// The input text.
    pub raw: &'a [u8],

    /// The current position in the input.
    pub pos: usize,
}

impl<'a> Tape<'a> {
    pub fn new(raw: &'a [u8]) -> Self {
        Self { raw, pos: 0 }
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
        pos == 0 || self.raw.get(pos - 1).is_none_or(|ch| ch.is_ws())
    }

    /// Returns true if the character at the given position has clearance on its right side.
    #[must_use]
    #[inline]
    pub fn is_r_clear(&self, pos: usize) -> bool {
        self.raw.get(pos + 1).is_none_or(|ch| ch.is_ws())
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

    /// Advances `pos` until `pred` returns true.
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

    /// Advances `pos` until `query` is found.
    ///
    /// Returns `true` if found and `pos` is left pointing at the match,
    /// or `false` and `pos` is restored to its original value.
    #[inline]
    pub fn seek_at(&mut self, query: &'a [u8]) -> bool {
        self.seek(|_, pos| self.raw[pos..].starts_with(query))
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
    /// Returns `true` if found (leaving `pos` at the match), or `false`
    /// and restores `pos` on failure.
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
    pub fn at(&self, query: &'_ [u8]) -> bool {
        self.raw.starts_with(query)
    }

    /// Returns true if there are no non-whitespace characters between
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
            if !c.is_ws() {
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
