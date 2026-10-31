//! Request signing for Tomato APP API (Argus + Ladon).
//! Ported from the Worker's JavaScript implementation.

use crate::crypto::md5;
use base64::Engine;
use rand::Rng;

pub const SIGN_KEY: [u8; 32] = [
    0xac, 0x1a, 0xda, 0xae, 0x95, 0xa7, 0xaf, 0x94,
    0xa5, 0x11, 0x4a, 0xb3, 0xb3, 0xa9, 0x7d, 0xd8,
    0x00, 0x50, 0xaa, 0x0a, 0x39, 0x31, 0x4c, 0x40,
    0x52, 0x8c, 0xae, 0xc9, 0x52, 0x56, 0xc2, 0x8c,
];

pub const AID: &str = "1967";
pub const LICENSE_ID: &str = "1611921764";
pub const SDK_VERSION: &str = "v04.04.05-ov-android";
pub const SDK_VERSION_INT: u32 = 134744640;
pub const CALL_TYPE: u32 = 738;

pub const READING_BASE: &str = "https://reading.snssdk.com";
pub const REGISTER_URL: &str = "https://i.snssdk.com/service/2/device_register/?tt_data=a";
pub const APP_UA: &str = "com.dragon.read";

pub const SHARED_KEY: [u8; 16] = [
    0xac, 0x25, 0xc6, 0x7d, 0xdd, 0x8f, 0x38, 0xc1,
    0xb3, 0x7a, 0x23, 0x48, 0x82, 0x8e, 0x22, 0x2e,
];

pub const FIXED_STRING_B64: &str = "TdTC5rgxYgkOUrPHpnM7pByyRiuCmrWKGWs521cXdST0m69/COjWjSanLjfBqVovHwWlGJKu8pSXMrYqOKrdWA==";

/// Generate the x-argus, x-ladon, x-khronos headers for a signed request.
pub fn generate_headers(raw_query: &str, xss_stub: &str, timestamp: u32) -> SignedHeaders {
    let argus = generate_argus(raw_query, xss_stub, timestamp);
    let ladon = generate_ladon(timestamp);
    SignedHeaders {
        x_argus: argus,
        x_ladon: ladon,
        x_khronos: timestamp.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct SignedHeaders {
    pub x_argus: String,
    pub x_ladon: String,
    pub x_khronos: String,
}

/// Generate Argus header (base64 of protobuf-encoded payload).
fn generate_argus(query: &str, xss_stub: &str, timestamp: u32) -> String {
    let mut rng = rand::thread_rng();
    let magic: u32 = 0x3258_5358; // "XSX2"
    let ticket = md5::md5_hex(format!("{}{}{}", query, xss_stub, timestamp).as_bytes());
    
    // Build a compact Argus payload using protobuf-like encoding
    let mut buf = Vec::with_capacity(128);
    // Field 1 (varint): magic
    buf.push(0x08); // field 1, varint
    encode_varint(&mut buf, magic as u64);
    // Field 2 (string): ticket (md5 hex)
    let ticket_bytes = ticket.as_bytes();
    buf.push(0x12); // field 2, length-delimited
    encode_varint(&mut buf, ticket_bytes.len() as u64);
    buf.extend_from_slice(ticket_bytes);
    // Field 3 (varint): timestamp
    buf.push(0x18);
    encode_varint(&mut buf, timestamp as u64);
    // Field 4 (varint): call type
    buf.push(0x20);
    encode_varint(&mut buf, CALL_TYPE as u64);
    // Field 5 (varint): sdk version int
    buf.push(0x28);
    encode_varint(&mut buf, SDK_VERSION_INT as u64);
    // Pad with random bytes
    let pad: [u8; 8] = rng.gen();
    buf.extend_from_slice(&pad);
    
    base64::engine::general_purpose::STANDARD.encode(&buf)
}

/// Generate Ladon header.
fn generate_ladon(timestamp: u32) -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 16] = rng.gen();
    
    // Build ladon payload
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&random_bytes);
    buf.extend_from_slice(&timestamp.to_le_bytes());
    buf.extend_from_slice(AID.as_bytes());
    
    // Simple XOR-based encoding with sign key
    let key = &SIGN_KEY;
    for (i, b) in buf.iter_mut().enumerate() {
        *b ^= key[i % key.len()];
    }
    
    base64::engine::general_purpose::STANDARD.encode(&buf)
}

fn encode_varint(buf: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buf.push(value as u8);
}

/// Sign a full URL with optional body. Returns headers to attach.
pub fn sign_request(url: &str, body: Option<&[u8]>) -> SignedHeaders {
    let query = url::Url::parse(url)
        .ok()
        .and_then(|u| u.query().map(|q| q.to_string()))
        .unwrap_or_default();
    
    let xss_stub = match body {
        Some(b) if !b.is_empty() => md5::md5_hex(b),
        _ => String::new(),
    };
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    
    generate_headers(&query, &xss_stub, timestamp)
}
