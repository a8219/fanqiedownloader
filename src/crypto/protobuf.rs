//! Minimal protobuf writer (varint + length-delimited only).

#[derive(Default)]
pub struct ProtobufWriter {
    buf: Vec<u8>,
}

impl ProtobufWriter {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(256) }
    }

    pub fn varint(&mut self, field: u32, value: u64) -> &mut Self {
        self.buf.push((field << 3) as u8);
        self.raw_varint(value);
        self
    }

    pub fn bytes(&mut self, field: u32, data: &[u8]) -> &mut Self {
        self.buf.push(((field << 3) | 2) as u8);
        self.raw_varint(data.len() as u64);
        self.buf.extend_from_slice(data);
        self
    }

    pub fn string(&mut self, field: u32, s: &str) -> &mut Self {
        self.bytes(field, s.as_bytes())
    }

    fn raw_varint(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.buf.push(((v & 0x7f) | 0x80) as u8);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}
