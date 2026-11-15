//! The funnel's rule, counted.
//!
//! Rule: cut every body into records of 81 symbols; a record seen twice or more is a function; a record seen
//! once is a singleton and is passed on. ("Once is an accident, twice is a function. Three is confirmation you get
//! for free.")
//!
//!   funnel BITS BODY_BYTES < STREAM
//!
//! STREAM is bodies of exactly BODY_BYTES each, one after another (for example the trained function tables of the
//! pieces of a corpus). BITS is the level the bodies are read at: 6, 8, 10 or 12 bits per symbol (the floors 64,
//! 256, 1024, 4096). One row is printed when the number of bodies read is a power of three, and one at the end.
//! Records are keyed by the first 128 bits of their SHA-256. Integers only.

use std::collections::HashMap;
use std::io::Read;

const REC: usize = 81;

const K: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3,
    0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da,
    0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13,
    0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070,
    0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3, 0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208,
    0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

/// SHA-256 of a short message, first 128 bits.
fn sha128(msg: &[u8]) -> u128 {
    let mut h: [u32; 8] = [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19];
    let mut buf = Vec::with_capacity(msg.len() + 72);
    buf.extend_from_slice(msg);
    buf.push(0x80);
    while buf.len() % 64 != 56 {
        buf.push(0);
    }
    buf.extend_from_slice(&((msg.len() as u64) * 8).to_be_bytes());
    for block in buf.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    (u128::from(h[0]) << 96) | (u128::from(h[1]) << 64) | (u128::from(h[2]) << 32) | u128::from(h[3])
}

/// The record that starts at bit `start` and is `nbits` long, left-aligned into `out`.
fn record_bits(body: &[u8], start: usize, nbits: usize, out: &mut [u8]) {
    let (byte, shift) = (start / 8, (start % 8) as u32);
    let full = nbits.div_ceil(8);
    for (i, slot) in out.iter_mut().take(full).enumerate() {
        let a = body[byte + i];
        let b = if shift > 0 && byte + i + 1 < body.len() { body[byte + i + 1] } else { 0 };
        *slot = if shift == 0 { a } else { (a << shift) | (b >> (8 - shift)) };
    }
    let spare = (full * 8 - nbits) as u32;
    if spare > 0 {
        out[full - 1] &= 0xFFu8 << spare;
    }
}

fn row(bits: usize, bodies: u64, body_bytes: usize, records: u64, map: &HashMap<u128, u32>) {
    let (mut functions, mut tied, mut singles, mut top) = (0u64, 0u64, 0u64, 0u64);
    for &c in map.values() {
        let c = u64::from(c);
        if c >= 2 {
            functions += 1;
            tied += c;
        } else {
            singles += 1;
        }
        top = top.max(c);
    }
    let distinct = map.len() as u64;
    let rec_bytes = (REC * bits).div_ceil(8) as u64;
    println!(
        "FUNNEL|level={}|bodies={bodies}|bytes={}|records={records}|distinct={distinct}|functions_seen_twice_or_more={functions}|records_that_tie={tied}|tie_permille={}|singletons={singles}|most_repeated_record_count={top}|records_that_tie_without_the_most_repeated={}|tie_permille_without_it={}|catalogue_bytes={}|distinct_per_body={}|json=0",
        1u32 << bits,
        bodies * body_bytes as u64,
        if records > 0 { tied * 1000 / records } else { 0 },
        tied.saturating_sub(top),
        if records > top { tied.saturating_sub(top) * 1000 / (records - top) } else { 0 },
        distinct * rec_bytes,
        if bodies > 0 { distinct / bodies } else { 0 }
    );
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let parsed = (a.get(1).and_then(|s| s.parse::<usize>().ok()), a.get(2).and_then(|s| s.parse::<usize>().ok()));
    let (Some(bits), Some(body_bytes)) = parsed else {
        println!("FUNNEL_REFUSE|reason=usage:_funnel_BITS_BODY_BYTES_<_STREAM|json=0");
        std::process::exit(1);
    };
    if ![6, 8, 10, 12].contains(&bits) || body_bytes == 0 {
        println!("FUNNEL_REFUSE|reason=BITS_is_6,_8,_10_or_12|json=0");
        std::process::exit(1);
    }
    let nbits = REC * bits;
    let per_body = body_bytes * 8 / nbits;
    let mut map: HashMap<u128, u32> = HashMap::new();
    let mut body = vec![0u8; body_bytes];
    let mut rec = vec![0u8; nbits.div_ceil(8)];
    let (mut bodies, mut records, mut next) = (0u64, 0u64, 1u64);
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut got = 0;
        while got < body_bytes {
            match input.read(&mut body[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) => {
                    println!("FUNNEL_REFUSE|reason={}|json=0", e.to_string().replace(' ', "_"));
                    std::process::exit(1);
                }
            }
        }
        if got < body_bytes {
            break;
        }
        for i in 0..per_body {
            if bits == 8 {
                *map.entry(sha128(&body[i * REC..(i + 1) * REC])).or_insert(0) += 1;
            } else {
                record_bits(&body, i * nbits, nbits, &mut rec);
                *map.entry(sha128(&rec)).or_insert(0) += 1;
            }
        }
        records += per_body as u64;
        bodies += 1;
        if bodies == next {
            row(bits, bodies, body_bytes, records, &map);
            next *= 3;
        }
    }
    if bodies * 3 != next {
        row(bits, bodies, body_bytes, records, &map);
    }
}
