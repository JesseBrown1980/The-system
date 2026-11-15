//! The codex programs in Rust: integers only, no Node, no V8, no dependencies.
//!
//! One binary with the five steps that the repository's `.mjs` programs perform, written so that each output is the
//! same bytes as the program it replaces:
//!
//!   drill   SRC OUT.qr                      any stream to the fixed 3,200-byte codex            (drill3k.mjs)
//!   train   KEY.raw ORDER FILE[:LIMIT] ...   the master key: the function table of the bodies    (roundtrip/train.mjs)
//!   enc     KEY.raw ORDER INPUT N OUT        code the first N bytes of INPUT against the key     (roundtrip/enc.mjs)
//!   dec     KEY.raw ORDER STREAM N OUT       decode N bytes from the key and the stream alone    (roundtrip/dec.mjs)
//!   resolve CODEX STORE-INDEX.hbp            Path 1: the codex names its body, the store returns it (resolve_path1.mjs)
//!   pack    KEY.raw ORDER INPUT ROOMS STUBS [MAX_ROOMS]   divide the body, as it streams, into 3,200-byte room codexes
//!   unpack  KEY.raw ORDER ROOMS OUT          the body again, from the key and the rooms alone
//!
//! The `.mjs` programs compute a probability as Math.round(((b + 0.4) / (a + b + 0.8)) * S) in IEEE-754 doubles. This
//! program reproduces that value with integers only (see `prob`), so its outputs are the published bytes.

use std::env;
use std::fs::File;
use std::io::{Read, Write};

const CTX2: usize = 65536;
const LANES: usize = 800;

// ---------------------------------------------------------------- SHA-256
const K256: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe,
    0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
    0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b,
    0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
    0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

struct Sha256 {
    h: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Sha256 { h: [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19], buf: Vec::with_capacity(128), len: 0 }
    }
    fn block(h: &mut [u32; 8], b: &[u8]) {
        let mut w = [0u32; 64];
        for (i, c) in b.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = *h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K256[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for (x, y) in h.iter_mut().zip(v) {
            *x = x.wrapping_add(y);
        }
    }
    fn update(&mut self, data: &[u8]) {
        self.len += data.len() as u64;
        self.buf.extend_from_slice(data);
        let whole = self.buf.len() / 64 * 64;
        for b in self.buf[..whole].chunks_exact(64) {
            Self::block(&mut self.h, b);
        }
        self.buf.drain(..whole);
    }
    fn hex(mut self) -> String {
        let bits = self.len * 8;
        self.buf.push(0x80);
        while self.buf.len() % 64 != 56 {
            self.buf.push(0);
        }
        self.buf.extend_from_slice(&bits.to_be_bytes());
        let rest = std::mem::take(&mut self.buf);
        for b in rest.chunks_exact(64) {
            Self::block(&mut self.h, b);
        }
        let mut out = String::with_capacity(64);
        for x in self.h {
            out.push_str(&format!("{x:08x}"));
        }
        out
    }
}

fn sha_of_file(path: &str) -> Result<(String, u64), String> {
    let mut f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 22];
    let mut n = 0u64;
    loop {
        let got = f.read(&mut buf).map_err(|e| e.to_string())?;
        if got == 0 {
            break;
        }
        h.update(&buf[..got]);
        n += got as u64;
    }
    Ok((h.hex(), n))
}

// ---------------------------------------------------------------- the bit-tree counts
struct Counts {
    n0: Vec<u32>,
    n1: Vec<u32>,
    ctx: usize,
    order: usize,
}

impl Counts {
    fn new(order: usize) -> Self {
        let ctx = if order == 1 { 256 } else { CTX2 };
        Counts { n0: vec![0; ctx * 256], n1: vec![0; ctx * 256], ctx, order }
    }
    /// Feed bytes with the two previous bytes carried in `carry`.
    fn feed(&mut self, buf: &[u8], carry: &mut (usize, usize)) {
        let (mut c1, mut c2) = *carry;
        for &b in buf {
            let base = if self.order == 1 { c1 } else { (c2 << 8) | c1 } * 256;
            let mut node = 1usize;
            for k in (0..8).rev() {
                let bit = usize::from((b >> k) & 1);
                if bit == 1 {
                    self.n1[base + node] = self.n1[base + node].wrapping_add(1);
                } else {
                    self.n0[base + node] = self.n0[base + node].wrapping_add(1);
                }
                node = (node << 1) | bit;
            }
            c2 = c1;
            c1 = usize::from(b);
        }
        *carry = (c1, c2);
    }
    /// Stream the first `limit` bytes of a file through the counts; nothing of the stream is kept.
    fn stream(&mut self, path: &str, limit: Option<u64>) -> Result<u64, String> {
        let mut f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let mut buf = vec![0u8; 1 << 22];
        let mut carry = (0usize, 0usize);
        let mut read = 0u64;
        loop {
            let want = limit.map_or(buf.len() as u64, |l| (l - read).min(buf.len() as u64)) as usize;
            if want == 0 {
                break;
            }
            let got = f.read(&mut buf[..want]).map_err(|e| e.to_string())?;
            if got == 0 {
                break;
            }
            self.feed(&buf[..got], &mut carry);
            read += got as u64;
        }
        Ok(read)
    }
}

/// Round an integer N * 2^e0 to 53 significant bits, ties to even (`sticky` marks discarded non-zero bits below N).
fn round53(n: u128, e0: i32, sticky: bool) -> (u128, i32) {
    let len = 128 - n.leading_zeros();
    if len <= 53 {
        return (n, e0);
    }
    let s = len - 53;
    let mut m = n >> s;
    let rem = n & ((1u128 << s) - 1);
    let half = 1u128 << (s - 1);
    if rem > half || (rem == half && (sticky || m & 1 == 1)) {
        m += 1;
    }
    if m == 1u128 << 53 {
        return (m >> 1, e0 + s as i32 + 1);
    }
    (m, e0 + s as i32)
}

/// The value the original programs compute, Math.round(((b + 0.4) / (a + b + 0.8)) * scale) in IEEE-754 doubles,
/// reproduced here with integers only: each of the four operations is rounded to 53 bits, ties to even, as a double
/// is, and the last step is the exact rounding to the nearest integer, ties upward. The published codexes and the
/// master key were written with that rule, and at an exact half (for example a = 0, b = 16) the double lands just
/// below it, so the exact rational rounded half up gives other bytes.
fn prob(a: u32, b: u32, scale: u64) -> u64 {
    const M: u128 = 7_205_759_403_792_794; // 0.4 = M * 2^-54 and 0.8 = M * 2^-53 as doubles
    let (m1, e1) = round53((u128::from(b) << 54) + M, -54, false); // b + 0.4
    let (m2, e2) = round53(((u128::from(a) + u128::from(b)) << 53) + M, -53, false); // a + b + 0.8
    // the quotient of two doubles: m1 / m2 lies in (2^-22, 2^34); shift so the integer quotient has at least 54 bits
    let lift = 54 + (128 - m2.leading_zeros()) as i32 - (128 - m1.leading_zeros()) as i32 + 1;
    let (q, r) = ((m1 << lift) / m2, (m1 << lift) % m2);
    let (mq, eq) = round53(q, e1 - e2 - lift, r != 0);
    let (my, ey) = round53(mq * u128::from(scale), eq, false);
    // nearest integer, ties upward
    if ey >= 0 {
        return (my << ey) as u64;
    }
    let k = (-ey) as u32;
    if k > 100 {
        return 0;
    }
    ((my + (1u128 << (k - 1))) >> k) as u64
}

// ---------------------------------------------------------------- drill: any stream to 3,200 bytes
fn drill(src: &str, out: &str) -> Result<(), String> {
    let mut c = Counts::new(2);
    let read = c.stream(src, None)?;
    let mut qr = [0u32; LANES];
    let mut live = 0u64;
    for i in 0..c.n0.len() {
        let (a, b) = (c.n0[i], c.n1[i]);
        if u64::from(a) + u64::from(b) == 0 {
            continue;
        }
        live += 1;
        let p = prob(a, b, 4095) as u32;
        let lane = i % LANES;
        qr[lane] = (qr[lane] ^ p).wrapping_mul(0x9E37_79B1).wrapping_add(i as u32);
    }
    let bytes: Vec<u8> = qr.iter().flat_map(|x| x.to_le_bytes()).collect();
    File::create(out).and_then(|mut f| f.write_all(&bytes)).map_err(|e| format!("{out}: {e}"))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    println!("DRILL|src={src}|streamed_bytes={read}|visited_nodes={live}|codex_bytes={}|codex_sha256={}|contexts={}|json=0", bytes.len(), h.hex(), c.ctx);
    Ok(())
}

// ---------------------------------------------------------------- train: the master key
fn load_key(path: &str, order: usize) -> Result<Vec<u16>, String> {
    let mut raw = Vec::new();
    File::open(path).and_then(|mut f| f.read_to_end(&mut raw)).map_err(|e| format!("{path}: {e}"))?;
    let want = if order == 1 { 256 } else { CTX2 } * 256 * 2;
    if raw.len() != want {
        return Err(format!("{path}: {} bytes, expected {want} for order {order}", raw.len()));
    }
    Ok(raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
}

fn train(out: &str, order: usize, bodies: &[String]) -> Result<(), String> {
    let mut c = Counts::new(order);
    let mut total = 0u64;
    for b in bodies {
        // FILE or FILE:LIMIT; a drive letter such as D: is part of the path
        let (path, limit) = match b.rfind(':') {
            Some(i) if i > 1 && b[i + 1..].chars().all(|ch| ch.is_ascii_digit()) && !b[i + 1..].is_empty() => (&b[..i], b[i + 1..].parse::<u64>().ok()),
            _ => (b.as_str(), None),
        };
        let read = c.stream(path, limit)?;
        total += read;
        println!("TRAIN_BODY|path={path}|streamed_bytes={read}|kept=0|json=0");
    }
    let mut live = 0u64;
    let mut bytes = Vec::with_capacity(c.n0.len() * 2);
    for i in 0..c.n0.len() {
        let (a, b) = (c.n0[i], c.n1[i]);
        let p = if u64::from(a) + u64::from(b) == 0 {
            2048
        } else {
            live += 1;
            prob(a, b, 4096).clamp(1, 4095) as u16
        };
        bytes.extend_from_slice(&p.to_le_bytes());
    }
    File::create(out).and_then(|mut f| f.write_all(&bytes)).map_err(|e| format!("{out}: {e}"))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    println!("TRAIN|order={order}|streamed_and_discarded_bytes={total}|nodes={}|visited={live}|key_raw_bytes={}|key_raw_sha256={}|json=0", c.n0.len(), bytes.len(), h.hex());
    Ok(())
}

// ---------------------------------------------------------------- the range coder (12-bit probabilities of a 1 bit)
struct Enc {
    low: u64,
    range: u32,
    cache: u32,
    pending: u64,
    first: bool,
    out: Vec<u8>,
}

impl Enc {
    fn new() -> Self {
        Enc { low: 0, range: 0xFFFF_FFFF, cache: 0, pending: 0, first: true, out: Vec::new() }
    }
    fn shift_low(&mut self) {
        let hi = (self.low >> 32) as u32;
        if hi != 0 || self.low < 0xFF00_0000 {
            if self.first {
                self.first = false;
            } else {
                self.out.push((self.cache.wrapping_add(hi) & 0xFF) as u8);
            }
            while self.pending > 0 {
                self.out.push((0xFFu32.wrapping_add(hi) & 0xFF) as u8);
                self.pending -= 1;
            }
            self.cache = ((self.low as u32) >> 24) & 0xFF;
        } else {
            self.pending += 1;
        }
        self.low = (self.low % 0x0100_0000) * 256;
    }
    fn bit(&mut self, p1: u16, bit: u8) {
        let p0 = 4096 - u32::from(p1);
        let bound = (self.range / 4096) * p0;
        if bit == 0 {
            self.range = bound;
        } else {
            self.low += u64::from(bound);
            self.range -= bound;
        }
        while self.range < 0x0100_0000 {
            self.range = self.range.wrapping_mul(256);
            self.shift_low();
        }
    }
    /// Bytes the stream would have if it were closed now.
    fn closed_len(&self) -> usize {
        self.out.len() + self.pending as usize + 5
    }
    fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.shift_low();
        }
        self.out
    }
}

struct Dec<'a> {
    b: &'a [u8],
    pos: usize,
    code: u32,
    range: u32,
}

impl<'a> Dec<'a> {
    fn new(b: &'a [u8]) -> Self {
        let mut d = Dec { b, pos: 0, code: 0, range: 0xFFFF_FFFF };
        for _ in 0..4 {
            d.code = d.code.wrapping_mul(256).wrapping_add(d.next());
        }
        d
    }
    fn next(&mut self) -> u32 {
        let v = self.b.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        u32::from(v)
    }
    fn bit(&mut self, p1: u16) -> usize {
        let p0 = 4096 - u32::from(p1);
        let bound = (self.range / 4096) * p0;
        let bit = if self.code < bound {
            self.range = bound;
            0
        } else {
            self.code -= bound;
            self.range -= bound;
            1
        };
        while self.range < 0x0100_0000 {
            self.range = self.range.wrapping_mul(256);
            self.code = self.code.wrapping_mul(256).wrapping_add(self.next());
        }
        bit
    }
}

fn enc(key: &str, order: usize, input: &str, n: usize, out: &str) -> Result<(), String> {
    let p = load_key(key, order)?;
    let mut data = vec![0u8; n];
    File::open(input).and_then(|mut f| f.read_exact(&mut data)).map_err(|e| format!("{input}: {e}"))?;
    let mut e = Enc::new();
    let (mut c1, mut c2) = (0usize, 0usize);
    for &b in &data {
        let base = if order == 1 { c1 } else { (c2 << 8) | c1 } * 256;
        let mut node = 1usize;
        for k in (0..8).rev() {
            let bit = (b >> k) & 1;
            e.bit(p[base + node], bit);
            node = (node << 1) | usize::from(bit);
        }
        c2 = c1;
        c1 = usize::from(b);
    }
    let stream = e.finish();
    File::create(out).and_then(|mut f| f.write_all(&stream)).map_err(|e| format!("{out}: {e}"))?;
    let mut h = Sha256::new();
    h.update(&stream);
    println!("ENC|input_bytes={n}|stream_bytes={}|stream_sha256={}|json=0", stream.len(), h.hex());
    Ok(())
}

fn dec(key: &str, order: usize, stream: &str, n: usize, out: &str) -> Result<(), String> {
    let p = load_key(key, order)?;
    let mut s = Vec::new();
    File::open(stream).and_then(|mut f| f.read_to_end(&mut s)).map_err(|e| format!("{stream}: {e}"))?;
    let mut d = Dec::new(&s);
    let mut data = vec![0u8; n];
    let (mut c1, mut c2) = (0usize, 0usize);
    for slot in data.iter_mut() {
        let base = if order == 1 { c1 } else { (c2 << 8) | c1 } * 256;
        let mut node = 1usize;
        for _ in 0..8 {
            node = (node << 1) | d.bit(p[base + node]);
        }
        let b = (node & 0xFF) as u8;
        *slot = b;
        c2 = c1;
        c1 = usize::from(b);
    }
    File::create(out).and_then(|mut f| f.write_all(&data)).map_err(|e| format!("{out}: {e}"))?;
    let mut h = Sha256::new();
    h.update(&data);
    println!("DEC|decoded_bytes={n}|decoded_sha256={}|inputs=key_and_stream_only|json=0", h.hex());
    Ok(())
}

// ---------------------------------------------------------------- rooms: the body divided into 3,200-byte codexes
//
// The body is divided as it streams in. Each piece is coded against the master key into one room of exactly ROOM
// bytes: two bytes of piece length, the coded piece, zero padding. A piece ends when its coded form has filled the
// room. Every room decodes by itself from the key, so the rooms and the key return the body with no store. Beside the
// rooms goes a table of 8-byte stubs, one per room (the first 8 bytes of the room's SHA-256).
const ROOM: usize = 3200;
const ROOM_MARGIN: usize = 16; // one more byte can add at most 12 bytes of code

fn pack(key: &str, order: usize, input: &str, rooms_out: &str, stubs_out: &str, max_rooms: u64) -> Result<(), String> {
    let p = load_key(key, order)?;
    let mut f = std::io::BufReader::with_capacity(1 << 22, File::open(input).map_err(|e| format!("{input}: {e}"))?);
    let mut rooms = std::io::BufWriter::with_capacity(1 << 22, File::create(rooms_out).map_err(|e| format!("{rooms_out}: {e}"))?);
    let mut stubs = std::io::BufWriter::new(File::create(stubs_out).map_err(|e| format!("{stubs_out}: {e}"))?);
    let (mut count, mut body, mut least, mut most) = (0u64, 0u64, usize::MAX, 0usize);
    let mut e = Enc::new();
    let (mut c1, mut c2, mut n) = (0usize, 0usize, 0usize);
    let mut close = |e: Enc, n: usize, count: &mut u64, least: &mut usize, most: &mut usize| -> Result<(), String> {
        let code = e.finish();
        let mut room = vec![0u8; ROOM];
        room[..2].copy_from_slice(&(n as u16).to_le_bytes());
        room[2..2 + code.len()].copy_from_slice(&code);
        let mut h = Sha256::new();
        h.update(&room);
        let hex = h.hex();
        let stub: Vec<u8> = (0..8).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
        rooms.write_all(&room).map_err(|x| x.to_string())?;
        stubs.write_all(&stub).map_err(|x| x.to_string())?;
        *count += 1;
        *least = (*least).min(n);
        *most = (*most).max(n);
        Ok(())
    };
    let mut byte = [0u8; 1];
    loop {
        if f.read(&mut byte).map_err(|x| x.to_string())? == 0 {
            break;
        }
        let b = byte[0];
        let base = if order == 1 { c1 } else { (c2 << 8) | c1 } * 256;
        let mut node = 1usize;
        for k in (0..8).rev() {
            let bit = (b >> k) & 1;
            e.bit(p[base + node], bit);
            node = (node << 1) | usize::from(bit);
        }
        c2 = c1;
        c1 = usize::from(b);
        n += 1;
        body += 1;
        if e.closed_len() + ROOM_MARGIN > ROOM - 2 || n == 65535 {
            close(std::mem::replace(&mut e, Enc::new()), n, &mut count, &mut least, &mut most)?;
            (c1, c2, n) = (0, 0, 0);
            if count == max_rooms {
                break;
            }
        }
    }
    if n > 0 {
        close(e, n, &mut count, &mut least, &mut most)?;
    }
    rooms.flush().map_err(|x| x.to_string())?;
    stubs.flush().map_err(|x| x.to_string())?;
    println!(
        "PACK|body_bytes_taken={body}|rooms={count}|room_bytes={ROOM}|rooms_file_bytes={}|stub_table_bytes={}|least_piece_bytes={least}|most_piece_bytes={most}|mean_piece_bytes={}|json=0",
        count * ROOM as u64,
        count * 8,
        if count > 0 { body / count } else { 0 }
    );
    Ok(())
}

fn unpack(key: &str, order: usize, rooms_in: &str, out: &str) -> Result<(), String> {
    let p = load_key(key, order)?;
    let mut f = std::io::BufReader::with_capacity(1 << 22, File::open(rooms_in).map_err(|e| format!("{rooms_in}: {e}"))?);
    let mut w = std::io::BufWriter::with_capacity(1 << 22, File::create(out).map_err(|e| format!("{out}: {e}"))?);
    let mut h = Sha256::new();
    let mut room = vec![0u8; ROOM];
    let (mut count, mut total) = (0u64, 0u64);
    let mut piece: Vec<u8> = Vec::with_capacity(65536);
    while f.read_exact(&mut room).is_ok() {
        let n = usize::from(u16::from_le_bytes([room[0], room[1]]));
        let mut d = Dec::new(&room[2..]);
        let (mut c1, mut c2) = (0usize, 0usize);
        piece.clear();
        for _ in 0..n {
            let base = if order == 1 { c1 } else { (c2 << 8) | c1 } * 256;
            let mut node = 1usize;
            for _ in 0..8 {
                node = (node << 1) | d.bit(p[base + node]);
            }
            let b = (node & 0xFF) as u8;
            piece.push(b);
            c2 = c1;
            c1 = usize::from(b);
        }
        h.update(&piece);
        w.write_all(&piece).map_err(|x| x.to_string())?;
        count += 1;
        total += n as u64;
    }
    w.flush().map_err(|x| x.to_string())?;
    println!("UNPACK|rooms={count}|body_bytes={total}|body_sha256={}|inputs=key_and_rooms_only|json=0", h.hex());
    Ok(())
}

// ---------------------------------------------------------------- Path 1: the codex names its body
fn resolve(codex: &str, index: &str) -> Result<(), String> {
    let (name, n) = sha_of_file(codex)?;
    println!("CODEX|bytes={n}|sha256={name}|json=0");
    let mut text = String::new();
    File::open(index).and_then(|mut f| f.read_to_string(&mut text)).map_err(|e| format!("{index}: {e}"))?;
    for line in text.lines() {
        let field = |k: &str| line.split('|').find_map(|x| x.strip_prefix(k).and_then(|v| v.strip_prefix('=')));
        if field("codex_sha256") == Some(name.as_str()) {
            let (path, want) = (field("path").unwrap_or(""), field("bytes").unwrap_or(""));
            let (sha, got) = sha_of_file(path)?;
            println!("RESOLVE|corpus={}|returned_bytes={got}|expected_bytes={want}|sha256={sha}|path=1_store|json=0", field("corpus").unwrap_or(""));
            return Ok(());
        }
    }
    println!("RESOLVE|result=NOT_IN_STORE|json=0");
    Ok(())
}

fn run(a: &[String]) -> Result<(), String> {
    let num = |s: &String| s.parse::<usize>().map_err(|_| format!("not a number: {s}"));
    match (a.get(1).map(String::as_str), a.len()) {
        (Some("drill"), 4) => drill(&a[2], &a[3]),
        (Some("train"), n) if n >= 5 => train(&a[2], num(&a[3])?, &a[4..]),
        (Some("enc"), 7) => enc(&a[2], num(&a[3])?, &a[4], num(&a[5])?, &a[6]),
        (Some("dec"), 7) => dec(&a[2], num(&a[3])?, &a[4], num(&a[5])?, &a[6]),
        (Some("resolve"), 4) => resolve(&a[2], &a[3]),
        (Some("pack"), 7 | 8) => pack(&a[2], num(&a[3])?, &a[4], &a[5], &a[6], a.get(7).map_or(u64::MAX, |x| x.parse::<u64>().unwrap_or(u64::MAX))),
        (Some("unpack"), 6) => unpack(&a[2], num(&a[3])?, &a[4], &a[5]),
        _ => Err("usage: asolaria_codex drill SRC OUT.qr | train KEY.raw ORDER FILE[:LIMIT]... | enc KEY.raw ORDER INPUT N OUT | dec KEY.raw ORDER STREAM N OUT | resolve CODEX STORE-INDEX.hbp | pack KEY.raw ORDER INPUT ROOMS STUBS [MAX_ROOMS] | unpack KEY.raw ORDER ROOMS OUT".into()),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(why) = run(&args) {
        println!("CODEX_REFUSE|reason={}|json=0", why.replace(' ', "_"));
        std::process::exit(1);
    }
}
