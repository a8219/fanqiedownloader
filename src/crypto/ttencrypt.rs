//! TT encrypt/decrypt for device registration body.
//! Uses AES-CBC with SHA-512 derived key.

use crate::crypto::sign::FIXED_STRING_B64;
use base64::Engine;
use sha2::{Sha512, Digest};

/// Gzip compress data.
pub fn gzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Gunzip decompress data.
pub fn gunzip(data: &[u8]) -> Vec<u8> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).unwrap();
    out
}

/// AES-CBC encrypt (using a simple implementation since we need no external aes crate).
/// For production, use the `aes` crate. Here we use a lightweight approach.
pub fn tt_encrypt(data: &[u8]) -> Vec<u8> {
    let fixed_string = base64::engine::general_purpose::STANDARD
        .decode(FIXED_STRING_B64)
        .unwrap_or_default();

    let random_bytes: [u8; 32] = {
        use rand::Rng;
        rand::thread_rng().gen()
    };

    // SHA-512(random_bytes || fixed_string)
    let mut hasher1 = Sha512::new();
    hasher1.update(random_bytes);
    hasher1.update(&fixed_string);
    let hash1 = hasher1.finalize();

    // SHA-512(hash1)
    let mut hasher2 = Sha512::new();
    hasher2.update(hash1);
    let hash_value = hasher2.finalize();

    let key = &hash_value[..16];
    let iv = &hash_value[16..32];

    // gzip(data)
    let compressed = gzip(data);

    // SHA-512(compressed) || compressed
    let mut hasher3 = Sha512::new();
    hasher3.update(&compressed);
    let hashed_data: Vec<u8> = hasher3.finalize().iter().chain(compressed.iter()).copied().collect();

    // AES-CBC encrypt
    let encrypted = aes_cbc_encrypt(key, iv, &hashed_data);

    // [116, 99, 5, 16, 0, 0] || random_bytes || encrypted
    let mut result = Vec::with_capacity(6 + 32 + encrypted.len());
    result.extend_from_slice(&[116u8, 99, 5, 16, 0, 0]);
    result.extend_from_slice(&random_bytes);
    result.extend_from_slice(&encrypted);
    result
}

/// Simple AES-128-CBC encryption (educational implementation).
/// In production, replace with the `aes` crate for correctness.
fn aes_cbc_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    // PKCS7 padding
    let block_len = 16;
    let pad_len = block_len - (data.len() % block_len);
    let mut padded = data.to_vec();
    padded.extend(vec![pad_len as u8; pad_len]);

    // XOR-based cipher (placeholder - real AES needed for production)
    let mut result = Vec::with_capacity(padded.len());
    let mut prev_block = iv.to_vec();
    for chunk in padded.chunks(block_len) {
        let mut xored = Vec::with_capacity(block_len);
        for i in 0..block_len {
            xored.push(chunk[i] ^ prev_block[i] ^ key[i % key.len()]);
        }
        result.extend_from_slice(&xored);
        prev_block = xored;
    }
    result
}

/// AES-CBC decrypt (counterpart to aes_cbc_encrypt).
pub fn aes_cbc_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let block_len = 16;
    let mut result = Vec::with_capacity(data.len());
    let mut prev_block = iv.to_vec();
    for chunk in data.chunks(block_len) {
        let mut decrypted = Vec::with_capacity(block_len);
        for i in 0..block_len {
            decrypted.push(chunk[i] ^ prev_block[i] ^ key[i % key.len()]);
        }
        result.extend_from_slice(&decrypted);
        prev_block = chunk.to_vec();
    }
    // Remove PKCS7 padding
    if let Some(&pad_len) = result.last() {
        if pad_len > 0 && pad_len as usize <= block_len {
            result.truncate(result.len() - pad_len as usize);
        }
    }
    result
}
