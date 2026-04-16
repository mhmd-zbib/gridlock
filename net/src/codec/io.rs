use super::error::DecodeError;

// ── BufWriter ─────────────────────────────────────────────────────────────────

pub(super) struct BufWriter(Vec<u8>);

impl BufWriter {
    pub(super) fn new() -> Self {
        Self(Vec::with_capacity(64))
    }
    pub(super) fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub(super) fn i8(&mut self, v: i8) {
        self.0.push(v as u8);
    }
    pub(super) fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    pub(super) fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    pub(super) fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_bits().to_be_bytes());
    }
    pub(super) fn bytes(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
    pub(super) fn finish(self) -> Vec<u8> {
        self.0
    }
}

// ── BufReader ─────────────────────────────────────────────────────────────────

pub(super) struct BufReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BufReader<'a> {
    pub(super) fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(super) fn u8(&mut self) -> Result<u8, DecodeError> {
        self.buf
            .get(self.pos)
            .map(|&b| {
                self.pos += 1;
                b
            })
            .ok_or(DecodeError::UnexpectedEof)
    }

    pub(super) fn i8(&mut self) -> Result<i8, DecodeError> {
        self.u8().map(|b| b as i8)
    }

    pub(super) fn u16(&mut self) -> Result<u16, DecodeError> {
        self.bytes(2)
            .map(|b| u16::from_be_bytes(b.try_into().unwrap()))
    }

    pub(super) fn u32(&mut self) -> Result<u32, DecodeError> {
        self.bytes(4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
    }

    pub(super) fn f32(&mut self) -> Result<f32, DecodeError> {
        self.bytes(4)
            .map(|b| f32::from_bits(u32::from_be_bytes(b.try_into().unwrap())))
    }

    pub(super) fn bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos + n;
        if end > self.buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
}
