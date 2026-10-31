//! TT encrypt/decrypt for the device registration body.
//! AES-128-CBC with SHA-512 derived key (PKCS#7 padding).

use crate::crypto::sign::FIXED_STRING_B64;
use base64::Engine;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha2::{Digest, Sha512};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// Gzip compress.
pub fn gzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).expect("gzip write");
    enc.finish().expect("gzip finish")
}

/// Gunzip decompress.
pub fn gunzip(data: &[u8]) -> Vec<u8> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let mut dec = GzDecoder::new(data);
    let mut out = Vec::new();
    let _ = dec.read_to_end(&mut out);
    out
}

/// TT encrypt: SHA-512 key derivation + AES-128-CBC.
pub fn tt_encrypt(data: &[u8]) -> Vec<u8> {
    use rand::Rng;

    let fixed_string = base64::engine::general_purpose::STANDARD
        .decode(FIXED_STRING_B64)
        .unwrap_or_default();

    let random_bytes: [u8; 32] = rand::thread_rng().gen();

    let mut h1 = Sha512::new();
    h1.update(random_bytes);
    h1.update(&fixed_string);
    let hash1 = h1.finalize();

    let mut h2 = Sha512::new();
    h2.update(hash1);
    let hash_value = h2.finalize();

    let key: [u8; 16] = hash_value[..16].try_into().unwrap();
    let iv: [u8; 16] = hash_value[16..32].try_into().unwrap();

    let compressed = gzip(data);

    let mut h3 = Sha512::new();
    h3.update(&compressed);
    let digest = h3.finalize();

    let mut payload = Vec::with_capacity(64 + compressed.len());
    payload.extend_from_slice(&digest);
    payload.extend_from_slice(&compressed);

    let encrypted = aes_cbc_encrypt(&key, &iv, &payload);

    let mut out = Vec::with_capacity(6 + 32 + encrypted.len());
    out.extend_from_slice(&[116u8, 99, 5, 16, 0, 0]);
    out.extend_from_slice(&random_bytes);
    out.extend_from_slice(&encrypted);
    out
}

/// AES-128-CBC encrypt with PKCS#7 padding.
pub fn aes_cbc_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let k: [u8; 16] = key.try_into().unwrap_or([0u8; 16]);
    let v: [u8; 16] = iv.try_into().unwrap_or([0u8; 16]);

    let mut buf = vec![0u8; data.len() + 16];
    buf[..data.len()].copy_from_slice(data);

    let ct = Aes128CbcEnc::new(&k.into(), &v.into())
        .encrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buf, data.len())
        .expect("AES-CBC encrypt");
    ct.to_vec()
}

/// AES-128-CBC decrypt with PKCS#7 unpadding (tolerant).
pub fn aes_cbc_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let k: [u8; 16] = key.try_into().unwrap_or([0u8; 16]);
    let v: [u8; 16] = iv.try_into().unwrap_or([0u8; 16]);
    let mut buf = data.to_vec();

    match Aes128CbcDec::new(&k.into(), &v.into())
        .decrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buf)
    {
        Ok(pt) => pt.to_vec(),
        // Malformed padding: fall back to raw in-place decrypt (no unpad) so the
        // caller can still work with whatever plaintext we can recover.
        // Malformed padding or non-block-aligned input: return empty so the
        // caller falls back to the web reader path.
        Err(_) => Vec::new(),
    }
}
