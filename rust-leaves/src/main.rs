//! A thin streaming driver around the operator's `path2-two-shadow-recovery` library, used unchanged.
//!
//!   black INPUT OUTDIR             the black side: every 8-byte block of the body projected onto the seven default
//!                                  cylinders; one leaf file per cylinder (one colour each), 4 bytes per block
//!   white OUTDIR I J K LEN OUT     the white side: the body rebuilt from three leaves only, by the library's
//!                                  `MultiCylinder::recover_from`; any other leaf file may be absent
//!   hold  OUTDIR I J LEN           two leaves only: the library's answer
//!
//! All projection and recovery is the library's own code (`project`, `recover_from`). This file only streams.

use path2_two_shadow_recovery::{Held, MultiCylinder, MultiShadows};
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

const CHUNK: usize = 8 << 20; // bytes of body per step, a multiple of the 8-byte block

fn leaf(dir: &str, i: usize) -> String {
    format!("{dir}/leaf-cylinder-{i}.u32")
}

fn black(input: &str, dir: &str) -> Result<(), String> {
    let codec = MultiCylinder::default_60d();
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut f = BufReader::with_capacity(CHUNK, File::open(input).map_err(|e| format!("{input}: {e}"))?);
    let mut outs: Vec<BufWriter<File>> = Vec::new();
    for i in 0..codec.primes.len() {
        outs.push(BufWriter::with_capacity(1 << 22, File::create(leaf(dir, i)).map_err(|e| e.to_string())?));
    }
    let mut buf = vec![0u8; CHUNK];
    let (mut total, mut blocks) = (0u64, 0u64);
    loop {
        let mut got = 0;
        while got < CHUNK {
            let n = f.read(&mut buf[got..]).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            got += n;
        }
        if got == 0 {
            break;
        }
        let sh = codec.project(&buf[..got]);
        for (out, residues) in outs.iter_mut().zip(&sh.residues) {
            for &r in residues {
                let r32 = u32::try_from(r).map_err(|_| "a residue above 32 bits".to_string())?;
                out.write_all(&r32.to_le_bytes()).map_err(|e| e.to_string())?;
            }
        }
        total += got as u64;
        blocks += sh.residues[0].len() as u64;
    }
    for out in &mut outs {
        out.flush().map_err(|e| e.to_string())?;
    }
    println!(
        "BLACK|body_bytes={total}|block_bytes={}|blocks={blocks}|cylinders={}|primes={:?}|leaf_bytes_each={}|all_leaves_bytes={}|json=0",
        codec.block_bytes,
        codec.primes.len(),
        codec.primes,
        blocks * 4,
        blocks * 4 * codec.primes.len() as u64
    );
    Ok(())
}

fn read_u32s(f: &mut BufReader<File>, want: usize) -> Result<Vec<u64>, String> {
    let mut raw = vec![0u8; want * 4];
    f.read_exact(&mut raw).map_err(|e| e.to_string())?;
    Ok(raw.chunks_exact(4).map(|c| u64::from(u32::from_le_bytes([c[0], c[1], c[2], c[3]]))).collect())
}

fn white(dir: &str, pick: &[usize], len: u64, out: Option<&str>) -> Result<(), String> {
    let codec = MultiCylinder::default_60d();
    let mut leaves: Vec<BufReader<File>> = Vec::new();
    for &i in pick {
        leaves.push(BufReader::with_capacity(1 << 22, File::open(leaf(dir, i)).map_err(|e| format!("{}: {e}", leaf(dir, i)))?));
    }
    let mut w = match out {
        Some(p) => Some(BufWriter::with_capacity(1 << 22, File::create(p).map_err(|e| format!("{p}: {e}"))?)),
        None => None,
    };
    let (mut done, mut blocks) = (0u64, 0u64);
    while done < len {
        let take = (len - done).min(CHUNK as u64) as usize;
        let nblocks = take.div_ceil(codec.block_bytes);
        // the library wants one residue list per cylinder; the leaves that are absent stay at zero and are not selected
        let mut residues = vec![vec![0u64; nblocks]; codec.primes.len()];
        for (slot, f) in pick.iter().zip(leaves.iter_mut()) {
            residues[*slot] = read_u32s(f, nblocks)?;
        }
        let sh = MultiShadows { residues, orig_len: take, block_bytes: codec.block_bytes };
        match codec.recover_from(&sh, pick) {
            Ok(bytes) => {
                if let Some(w) = w.as_mut() {
                    w.write_all(&bytes).map_err(|e| e.to_string())?;
                }
            }
            Err(held) => {
                println!("WHITE|leaves={pick:?}|outcome=HELD|reason={held:?}|bytes_emitted={done}|json=0");
                return if held == Held::InsufficientJointCapacity { Ok(()) } else { Err(format!("{held:?}")) };
            }
        }
        done += take as u64;
        blocks += nblocks as u64;
    }
    if let Some(w) = w.as_mut() {
        w.flush().map_err(|e| e.to_string())?;
    }
    println!(
        "WHITE|leaves={pick:?}|outcome=RECOVERED|body_bytes={done}|blocks={blocks}|leaf_bytes_read={}|inputs=three_leaves_only|json=0",
        blocks * 4 * pick.len() as u64
    );
    Ok(())
}

fn run(a: &[String]) -> Result<(), String> {
    let num = |s: &String| s.parse::<u64>().map_err(|_| format!("not a number: {s}"));
    match (a.get(1).map(String::as_str), a.len()) {
        (Some("black"), 4) => black(&a[2], &a[3]),
        (Some("white"), 8) => white(&a[2], &[num(&a[3])? as usize, num(&a[4])? as usize, num(&a[5])? as usize], num(&a[6])?, Some(&a[7])),
        (Some("hold"), 6) => white(&a[2], &[num(&a[3])? as usize, num(&a[4])? as usize], num(&a[5])?, None),
        _ => Err("usage: path2_driver black INPUT OUTDIR | white OUTDIR I J K LEN OUT | hold OUTDIR I J LEN".into()),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(why) = run(&args) {
        println!("PATH2_REFUSE|reason={}|json=0", why.replace(' ', "_"));
        std::process::exit(1);
    }
}
