//! Minimal MD5 implementation for request signing.
//! Pure Rust, zero allocations on hot path.

pub struct Md5 {
    state: [u32; 4],
    buffer: [u8; 64],
    buffer_len: usize,
    total_len: u64,
}

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

impl Md5 {
    pub fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            buffer: [0u8; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.total_len += data.len() as u64;
        let mut data = data;
        if self.buffer_len > 0 {
            let need = 64 - self.buffer_len;
            let take = need.min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            data = &data[take..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }
        while data.len() >= 64 {
            let block: [u8; 64] = data[..64].try_into().unwrap();
            self.process_block(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffer_len = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        let bits = self.total_len.wrapping_mul(8);
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            for i in self.buffer_len..64 { self.buffer[i] = 0; }
            let block = self.buffer;
            self.process_block(&block);
            self.buffer_len = 0;
        }
        for i in self.buffer_len..56 { self.buffer[i] = 0; }
        self.buffer[56..64].copy_from_slice(&bits.to_le_bytes());
        let block = self.buffer;
        self.process_block(&block);
        let mut result = [0u8; 16];
        for i in 0..4 {
            result[i * 4..i * 4 + 4].copy_from_slice(&self.state[i].to_le_bytes());
        }
        result
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        for i in 0..64 {
            let (f, g);
            if i < 16 {
                f = (b & c) | (!b & d);
                g = i;
            } else if i < 32 {
                f = (d & b) | (!d & c);
                g = (5 * i + 1) % 16;
            } else if i < 48 {
                f = b ^ c ^ d;
                g = (3 * i + 5) % 16;
            } else {
                f = c ^ (b | !d);
                g = (7 * i) % 16;
            }
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])
                    .rotate_left(S[i])
            );
            a = temp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

pub fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn md5_bytes(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_md5_empty() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }
    #[test]
    fn test_md5_hello() {
        assert_eq!(md5_hex(b"hello"), "5d41402abc4b2a76b9719d911017c592");
    }
}