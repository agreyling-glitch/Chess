//! Low-level, big-endian read cursor, used for
//! `.si4` and `.sn4` (see `docs/scid-si4-specification.md` §1: "ALL multi-byte
//! integers ... are stored in BIG-ENDIAN").
//!
//! All reads are bounds-checked: reading past the end of the buffer
//! returns an error rather than panicking (truncated/corrupted file).

/// Read cursor over an in-memory buffer, big-endian.
pub struct BeReader<'a> {
    data: &'a [u8],
    pos:  usize,
}

/// Read error: not enough bytes left in the buffer.
#[derive(Debug, Clone, Copy)]
pub struct Eof;

impl Eof {
    /// Converts to a [`crate::error::ScidError::Truncated`] carrying the
    /// given context label — shorthand for the recurring
    /// `.map_err(|_| ScidError::Truncated("..."))` pattern.
    pub(crate) fn truncated(self, ctx: &'static str) -> crate::error::ScidError {
        crate::error::ScidError::Truncated(ctx)
    }

    /// Converts to a [`crate::error::GameDecodeError::BadMoveStream`]
    /// carrying the given context label — shorthand for the recurring
    /// `.map_err(|_| GameDecodeError::BadMoveStream("..."))` pattern.
    pub(crate) fn bad_move_stream(self, ctx: &'static str) -> crate::error::GameDecodeError {
        crate::error::GameDecodeError::BadMoveStream(ctx)
    }
}

impl<'a> BeReader<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Eof> {
        if self.remaining() < n {
            return Err(Eof);
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    /// Advances the cursor by `n` bytes without returning them.
    ///
    /// # Errors
    /// [`Eof`] if fewer than `n` bytes remain in the buffer.
    pub fn skip(&mut self, n: usize) -> Result<(), Eof> {
        self.take(n).map(|_| ())
    }

    /// Reads a single unsigned byte.
    ///
    /// # Errors
    /// [`Eof`] if the buffer is exhausted.
    pub fn read_u8(&mut self) -> Result<u8, Eof> {
        Ok(self.take(1)?[0])
    }

    /// Unsigned 2-byte integer, most significant byte first.
    ///
    /// # Errors
    /// [`Eof`] if fewer than 2 bytes remain in the buffer.
    pub fn read_u16(&mut self) -> Result<u16, Eof> {
        let b = self.take(2)?;
        Ok((u16::from(b[0]) << 8) | u16::from(b[1]))
    }

    /// Unsigned 3-byte integer (most significant byte first), used for
    /// `numGames`, `autoLoad`, the `.sn4` name counters, etc.
    ///
    /// # Errors
    /// [`Eof`] if fewer than 3 bytes remain in the buffer.
    pub fn read_u24(&mut self) -> Result<u32, Eof> {
        let b = self.take(3)?;
        Ok((u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]))
    }

    /// Unsigned 4-byte integer, most significant byte first.
    ///
    /// # Errors
    /// [`Eof`] if fewer than 4 bytes remain in the buffer.
    pub fn read_u32(&mut self) -> Result<u32, Eof> {
        let b = self.take(4)?;
        Ok((u32::from(b[0]) << 24)
            | (u32::from(b[1]) << 16)
            | (u32::from(b[2]) << 8)
            | u32::from(b[3]))
    }

    /// Reads `n` raw bytes.
    ///
    /// # Errors
    /// [`Eof`] if fewer than `n` bytes remain in the buffer.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], Eof> {
        self.take(n)
    }

    /// Reads a fixed-size string of `n` bytes, truncated at the first null
    /// byte encountered ("C string" convention used by the `.si4` header
    /// and the custom flag labels).
    ///
    /// # Errors
    /// [`Eof`] if fewer than `n` bytes remain in the buffer.
    pub fn read_fixed_cstr(&mut self, n: usize) -> Result<String, Eof> {
        let b = self.take(n)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }

    /// Reads a null-terminated string of variable length
    /// (used for the non-standard starting FEN in the `.sg4`).
    ///
    /// # Errors
    /// [`Eof`] if no null byte is found before the end of the buffer.
    pub fn read_terminated_cstr(&mut self) -> Result<String, Eof> {
        let rest = &self.data[self.pos..];
        let end = rest.iter().position(|&c| c == 0).ok_or(Eof)?;
        let s = String::from_utf8_lossy(&rest[..end]).into_owned();
        self.pos += end + 1; // includes the null byte
        Ok(s)
    }
}

/// Write cursor building a big-endian byte buffer — the write-side mirror
/// of [`BeReader`], used for `.si4`/`.sn4`.
///
/// Unlike [`BeReader`], writing never fails: every method that could
/// theoretically overflow its field width (`write_u24`, `write_fixed_cstr`)
/// masks/truncates defensively instead, matching this crate's established
/// masking philosophy for on-disk values that exceed a format's field
/// width (see `si4::index::write_index_entry`, `si5::index::write_index_entry`).
pub struct BeWriter {
    data: Vec<u8>,
}

impl BeWriter {
    #[must_use]
    pub fn with_capacity(n: usize) -> Self {
        Self { data: Vec::with_capacity(n) }
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.data.len()
    }

    pub fn write_u8(&mut self, v: u8) {
        self.data.push(v);
    }

    /// Unsigned 2-byte integer, most significant byte first.
    pub fn write_u16(&mut self, v: u16) {
        self.data.extend_from_slice(&v.to_be_bytes());
    }

    /// Unsigned 3-byte integer, most significant byte first — masked down
    /// to 24 bits if `v` is larger (see the struct doc).
    pub fn write_u24(&mut self, v: u32) {
        let b = (v & 0x00FF_FFFF).to_be_bytes();
        self.data.extend_from_slice(&b[1..4]);
    }

    /// Unsigned 4-byte integer, most significant byte first.
    pub fn write_u32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_bytes(&mut self, b: &[u8]) {
        self.data.extend_from_slice(b);
    }

    /// Writes `n` zero bytes — used for the derived/heuristic fields this
    /// crate never populates when writing (see `si4::index::write_index_entry`'s
    /// doc), and for unused header fields.
    pub fn write_zeros(&mut self, n: usize) {
        self.data.resize(self.data.len() + n, 0);
    }

    /// Writes `s` as a fixed-size, null-terminated field of exactly `n`
    /// bytes — the write-side mirror of [`BeReader::read_fixed_cstr`]. If
    /// `s` is too long to fit in `n - 1` bytes (leaving room for at least
    /// one null terminator byte) it is truncated at the last valid UTF-8
    /// character boundary that fits, rather than panicking or writing an
    /// invalid string.
    pub fn write_fixed_cstr(&mut self, s: &str, n: usize) {
        let max_len = n.saturating_sub(1);
        let mut copy_len = s.len().min(max_len);
        while copy_len > 0 && !s.is_char_boundary(copy_len) {
            copy_len -= 1;
        }
        self.data.extend_from_slice(&s.as_bytes()[..copy_len]);
        self.write_zeros(n - copy_len);
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}
