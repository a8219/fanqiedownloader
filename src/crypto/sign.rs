//! Request signing helpers (x-argus / x-ladon / x-khronos).
//!
//! Derived from fanqie-assistant (GPL-3.0). See THIRD_PARTY_NOTICES.md.

use base64::Engine;
use rand::Rng;

use crate::crypto::md5;

pub const AID: &str = "1967";
#[allow(dead_code)]
pub const LICENSE_ID: &str = "1611921764";
pub const SDK_VERSION_INT: u32 = 134_744_640;
pub const CALL_TYPE: u32 = 738;

pub const READING_BASE: &str = "https://reading.snssdk.com";
pub const REGISTER_URL: &str = "https://i.snssdk.com/service/2/device_register/?tt_data=a";
pub const APP_UA: &str = "com.dragon.read";

/// 32-byte signing salt.
pub const SIGN_KEY: [u8; 32] = [
    0xac, 0x1a, 0xda, 0xae, 0x95, 0xa7, 0xaf, 0x94, 0xa5, 0x11, 0x4a, 0xb3, 0xb3, 0xa9, 0x7d, 0xd8,
    0x00, 0x50, 0xaa, 0x0a, 0x39, 0x31, 0x4c, 0x40, 0x52, 0x8c, 0xae, 0xc9, 0x52, 0x56, 0xc2, 0x8c,
];

/// 16-byte shared AES key used by device/key registration.
pub const SHARED_KEY: [u8; 16] = [
    0xac, 0x25, 0xc6, 0x7d, 0xdd, 0x8f, 0x38, 0xc1, 0xb3, 0x7a, 0x23, 0x48, 0x82, 0x8e, 0x22, 0x2e,
];

/// Salt mixed into the TT encrypt key derivation.
pub const FIXED_STRING_B64: &str =
    "TdTC5rgxYgkOUrPHpnM7pByyRiuCmrWKGWs521cXdST0m69/COjWjSanLjfBqVovHwWlGJKu8pSXMrYqOKrdWA==";

#[derive(Debug, Clone)]
pub struct SignedHeaders {
    pub x_argus: String,
    pub x_ladon: String,
    pub x_khronos: String,
}

/// Sign a URL (+ optional body) and return the three required headers.
pub fn sign_request(url: &str, body: Option<&[u8]>) -> SignedHeaders {
    let raw_query = url
        .split_once('?')
        .map(|(_, q)| q.split('#').next().unwrap_or(q))
        .unwrap_or("");

    let stub = match body {
        Some(b) if !b.is_empty() => md5::md5_hex(b),
        _ => String::new(),
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    SignedHeaders {
        x_argus: build_argus(raw_query, &stub, ts),
        x_ladon: build_ladon(ts),
        x_khronos: ts.to_string(),
    }
}

fn build_argus(query: &str, stub: &str, ts: u32) -> String {
    let mut rng = rand::thread_rng();

    // Ticket = md5(query + stub + timestamp)
    let mut seed = String::with_capacity(query.len() + stub.len() + 12);
    seed.push_str(query);
    seed.push_str(stub);
    seed.push_str(&ts.to_string());
    let ticket = md5::md5_hex(seed.as_bytes());

    let mut payload = Vec::with_capacity(96);
    payload.push(0x08); // field 1 varint
    payload.push(0x58); // magic
    payload.push(0x53);
    payload.push(0x58);
    payload.push(0x32);
    payload.push(0x12); // field 2 len-delimited
    payload.push(ticket.len() as u8);
    payload.extend_from_slice(ticket.as_bytes());
    payload.push(0x18); // field 3 varint
    write_varint(&mut payload, ts as u64);
    payload.push(0x20); // field 4 varint
    write_varint(&mut payload, CALL_TYPE as u64);
    payload.push(0x28); // field 5 varint
    write_varint(&mut payload, SDK_VERSION_INT as u64);

    let pad: [u8; 8] = rng.gen();
    payload.extend_from_slice(&pad);

    base64::engine::general_purpose::STANDARD.encode(payload)
}

fn build_ladon(ts: u32) -> String {
    let mut rng = rand::thread_rng();
    let nonce: [u8; 16] = rng.gen();

    let mut payload = Vec::with_capacity(48);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ts.to_le_bytes());
    payload.extend_from_slice(AID.as_bytes());

    for (i, b) in payload.iter_mut().enumerate() {
        *b ^= SIGN_KEY[i % SIGN_KEY.len()];
    }

    base64::engine::general_purpose::STANDARD.encode(payload)
}

fn write_varint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        buf.push(((v & 0x7f) | 0x80) as u8);
        v >>= 7;
    }
    buf.push(v as u8);
}
