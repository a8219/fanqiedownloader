//! Minimal protobuf writer for building Argus/Ladon payloads.

pub struct ProtobufWriter {
    buf: Vec<u8>,
}

impl ProtobufWriter {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(256) }
    }

    pub fn write_varint(&mut self, field: u32, value: u64) {
        self.buf.push((field << 3) | 0);
        let mut v = value;
        while v >= 0x80 {
            self.buf.push(((v & 0x7F) | 0x80) as u8);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }

    pub fn write_string(&mut self, field: u32, s: &str) {
        self.buf.push((field << 3) | 2);
        let bytes = s.as_bytes();
        self.write_len(bytes.len() as u64);
        self.buf.extend_from_slice(bytes);
    }

    pub fn write_bytes(&mut self, field: u32, data: &[u8]) {
        self.buf.push((field << 3) | 2);
        self.write_len(data.len() as u64);
        self.buf.extend_from_slice(data);
    }

    fn write_len(&mut self, len: u64) {
        let mut v = len;
        while v >= 0x80 {
            self.buf.push(((v & 0x7F) | 0x80) as u8);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}
