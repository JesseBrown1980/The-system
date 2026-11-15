//! The residual selector: a shared context and a small carried piece.
//!
//! The body is read in 8-byte blocks. Each block is projected onto coprime cylinders with the library
//! `path2-two-shadow-recovery`, unchanged. The cylinder shadows are the CONTEXT. The context leaves a small number of
//! candidates for each block; the SELECTOR says which candidate is the block. Context and selector together return
//! the body exactly.
//!
//!   selector split BODY OUTDIR [P1,P2,...]   write OUTDIR/context-I.u32 (one per cylinder) and OUTDIR/selector.bits
//!   selector join  OUTDIR BODY_BYTES OUT     the body again, from the context and the selector
//!
//! The default cylinders are 3037000507 and 3037000493. Their product is just above 2^63, so the context leaves two
//! candidates per 8-byte block and the selector is one bit per block: 0.125 bits per character.
//! With three cylinders near 2^25 the context leaves one candidate and the selector is empty.

use path2_two_shadow_recovery::{crt_many, residual_candidate_count_for, residual_selector_bits_for, shadow};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

const BLOCK: usize = 8;
const CHUNK: usize = 8 << 20;
const DEFAULT: [u64; 2] = [3_037_000_507, 3_037_000_493];

struct BitWriter<W: Write> {
    w: W,
    acc: u64,
    n: u32,
    bytes: u64,
}
impl<W: Write> BitWriter<W> {
    fn put(&mut self, value: u64, bits: u32) -> std::io::Result<()> {
        for k in (0..bits).rev() {
            self.acc = (self.acc << 1) | ((value >> k) & 1);
            self.n += 1;
            if self.n == 8 {
                self.w.write_all(&[self.acc as u8])?;
                self.bytes += 1;
                self.acc = 0;
                self.n = 0;
            }
        }
        Ok(())
    }
    fn finish(mut self) -> std::io::Result<u64> {
        if self.n > 0 {
            self.w.write_all(&[(self.acc << (8 - self.n)) as u8])?;
            self.bytes += 1;
        }
        self.w.flush()?;
        Ok(self.bytes)
    }
}

struct BitReader<R: Read> {
    r: R,
    acc: u8,
    n: u32,
}
impl<R: Read> BitReader<R> {
    fn get(&mut self, bits: u32) -> std::io::Result<u64> {
        let mut v = 0u64;
        for _ in 0..bits {
            if self.n == 0 {
                let mut b = [0u8; 1];
                self.r.read_exact(&mut b)?;
                self.acc = b[0];
                self.n = 8;
            }
            v = (v << 1) | u64::from(self.acc >> 7);
            self.acc <<= 1;
            self.n -= 1;
        }
        Ok(v)
    }
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn modulus(primes: &[u64]) -> u128 {
    primes.iter().map(|&p| u128::from(p)).product()
}

fn split(body: &str, dir: &str, primes: &[u64]) -> Result<(), String> {
    for &p in primes {
        if p > u64::from(u32::MAX) {
            return Err(format!("cylinder {p} is above 32 bits"));
        }
    }
    let candidates = residual_candidate_count_for(BLOCK, primes).map_err(|h| format!("{h:?}"))?;
    let bits = residual_selector_bits_for(BLOCK, primes).map_err(|h| format!("{h:?}"))?;
    let m = modulus(primes);
    std::fs::create_dir_all(dir).map_err(err)?;
    let mut f = BufReader::with_capacity(CHUNK, File::open(body).map_err(|e| format!("{body}: {e}"))?);
    let mut ctx = Vec::new();
    for i in 0..primes.len() {
        ctx.push(BufWriter::with_capacity(1 << 22, File::create(format!("{dir}/context-{i}.u32")).map_err(err)?));
    }
    let mut sel = BitWriter { w: BufWriter::with_capacity(1 << 22, File::create(format!("{dir}/selector.bits")).map_err(err)?), acc: 0, n: 0, bytes: 0 };
    let mut buf = vec![0u8; CHUNK];
    let (mut total, mut blocks) = (0u64, 0u64);
    loop {
        let mut got = 0;
        while got < CHUNK {
            let n = f.read(&mut buf[got..]).map_err(err)?;
            if n == 0 {
                break;
            }
            got += n;
        }
        if got == 0 {
            break;
        }
        for chunk in buf[..got].chunks(BLOCK) {
            let mut x = 0u128;
            for &b in chunk {
                x = (x << 8) | u128::from(b);
            }
            x <<= 8 * (BLOCK - chunk.len()) as u32;
            for (out, &p) in ctx.iter_mut().zip(primes) {
                out.write_all(&(shadow(x, p) as u32).to_le_bytes()).map_err(err)?;
            }
            sel.put((x / m) as u64, bits).map_err(err)?;
            blocks += 1;
        }
        total += got as u64;
    }
    for out in &mut ctx {
        out.flush().map_err(err)?;
    }
    let sel_bytes = sel.finish().map_err(err)?;
    let list: Vec<String> = primes.iter().map(u64::to_string).collect();
    std::fs::write(format!("{dir}/cylinders.txt"), list.join(",") + "\n").map_err(err)?;
    println!(
        "SPLIT|body_bytes={total}|blocks={blocks}|cylinders={}|candidates_left_by_the_context={candidates}|selector_bits_per_block={bits}|context_bytes={}|selector_bytes={sel_bytes}|selector_bits_per_character_x1000={}|json=0",
        list.join(","),
        blocks * 4 * primes.len() as u64,
        if total > 0 { sel_bytes * 8000 / total } else { 0 }
    );
    Ok(())
}

fn join(dir: &str, len: u64, out: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(format!("{dir}/cylinders.txt")).map_err(err)?;
    let primes: Vec<u64> = text.trim().split(',').map(|s| s.parse::<u64>().map_err(err)).collect::<Result<_, _>>()?;
    let bits = residual_selector_bits_for(BLOCK, &primes).map_err(|h| format!("{h:?}"))?;
    let m = modulus(&primes);
    let mut ctx = Vec::new();
    for i in 0..primes.len() {
        ctx.push(BufReader::with_capacity(1 << 22, File::open(format!("{dir}/context-{i}.u32")).map_err(err)?));
    }
    let mut sel = BitReader { r: BufReader::with_capacity(1 << 22, File::open(format!("{dir}/selector.bits")).map_err(err)?), acc: 0, n: 0 };
    let mut w = BufWriter::with_capacity(1 << 22, File::create(out).map_err(err)?);
    let mut done = 0u64;
    let mut residues = vec![(0u64, 0u64); primes.len()];
    while done < len {
        for ((slot, f), &p) in residues.iter_mut().zip(ctx.iter_mut()).zip(&primes) {
            let mut b = [0u8; 4];
            f.read_exact(&mut b).map_err(err)?;
            *slot = (u64::from(u32::from_le_bytes(b)), p);
        }
        let base = crt_many(&residues).map_err(|h| format!("{h:?}"))?;
        let x = base + u128::from(if bits > 0 { sel.get(bits).map_err(err)? } else { 0 }) * m;
        let take = (len - done).min(BLOCK as u64) as usize;
        let bytes = (x as u64).to_be_bytes();
        w.write_all(&bytes[..take]).map_err(err)?;
        done += take as u64;
    }
    w.flush().map_err(err)?;
    println!("JOIN|body_bytes={done}|inputs=context_and_selector_only|json=0");
    Ok(())
}

fn run(a: &[String]) -> Result<(), String> {
    match (a.get(1).map(String::as_str), a.len()) {
        (Some("split"), 4) => split(&a[2], &a[3], &DEFAULT),
        (Some("split"), 5) => {
            let primes: Vec<u64> = a[4].split(',').map(|s| s.parse::<u64>().map_err(err)).collect::<Result<_, _>>()?;
            split(&a[2], &a[3], &primes)
        }
        (Some("join"), 5) => join(&a[2], a[3].parse::<u64>().map_err(err)?, &a[4]),
        _ => Err("usage: selector split BODY OUTDIR [P1,P2,...] | selector join OUTDIR BODY_BYTES OUT".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(why) = run(&args) {
        println!("SELECTOR_REFUSE|reason={}|json=0", why.replace(' ', "_"));
        std::process::exit(1);
    }
}
