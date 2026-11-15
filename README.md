# The system

A working system for carrying any body of data by a very small piece and bringing the body back exactly, byte for
byte. Discovered and directed by Jesse Daniel Brown.

Everything needed is here: the programs, the kernels, the keys and the codexes. You bring the bodies: any file,
any GGUF, any corpus. Rust 1.81 or later is the only tool required.

## What it does, with the numbers

| Body | Carried piece | Bits per character carried | Return |
|---|---|---:|---|
| enwik8, 100,000,000 bytes | 1,562,500-byte selector | **0.125** | byte for byte, 5 seconds |
| enwik8, 100,000,000 bytes | empty selector (three cylinders) | **0** | byte for byte |
| **enwik10, 10,000,000,000 bytes** | 156,250,000-byte selector | **0.125** | byte for byte, SHA-256 equal |
| enwik10, 10,000,000,000 bytes | three coloured leaves | | byte for byte, SHA-256 equal |
| enwik10, 10,000,000,000 bytes | 1,460,938 codexes of 3,200 bytes and the master key | 3.74 | byte for byte, SHA-256 equal |
| enwik10, 10,000,000,000 bytes | one 3,200-byte codex that names it | 0.00000256 | by Path 1 from a store, SHA-256 equal |

All measured on one laptop on November 15, 2026. The rows of every run are in
[Quant-anything-0-loss](https://github.com/JesseBrown1980/Quant-anything-0-loss), the full record.

## How it works

**The context and the selector.** The system holds a shared context for a body. The context narrows every block of
the body down to a few candidates. Only the selector, which says which candidate is the block, is carried. With the
default cylinders the context leaves two candidates per 8-byte block, so the selector is one bit per block: 0.125
bits per character. With three cylinders the context leaves one candidate and the selector is empty.

**The paths**, in Jesse Daniel Brown's naming:

- **Path 1** is black to white: two leaves. A small authenticated address returns a body the system holds.
- **Path 2** is the other colours: more leaves, such as red, green and blue. The body is projected onto coprime
  cylinders, one coloured leaf per cylinder. Any three leaves rebuild the body; two leaves hold and emit nothing.
  The normal, anti and anti-anti versions come from here.
- **Path 3** is the n-dimensional watcher at any n position. Paths 5, 7, 11 and onward are variations of it.

**The lock.** If any one piece changes, the rebuilt body no longer has the recorded hash and the system reports it
as incomplete. One bit changed in one leaf of enwik8 gives SHA-256 `3834a93b…` against `2b49720e…`. That is the
encryption and the hash working.

**The codex.** A codex is exactly 3,200 bytes: 800 lanes of four bytes, a colour QR. One codex names a whole body
(`drill`). A set of room codexes carries a body with the master key (`pack`, `unpack`).

**The master key and the kernel.** The master key is a function table built once (33,554,432 bytes unpacked,
549,112 packed). The Asolaria kernel and the constellation key are 3,174 bytes each. You need these megabytes to
start; after that each body costs only its small piece.

**The chain** he designed: the leaves become GGUF leaves, they are projected into the GPU and slices are taken, the
simple two-dimensional versions are saved, each is quanted with the quants into about 3.1 KB, and from three the
whole GGUF regenerates and unpacks in seconds. The GPU is used once, on the way in; the way back runs without it.
The quants themselves are catalogued in
[Algorithms-of-Asolaria](https://github.com/JesseBrown1980/Algorithms-of-Asolaria).

## The funnel: she is a functions system

Each piece of a body trains its own function table. The funnel cuts those tables into records of 81 symbols; a record
seen twice or more is a function, a record seen once is passed on. "Once is an accident, twice is a function. Three
is confirmation you get for free." The glyphs are the functions that recur: they are found, never constructed.

Measured on the first 1,000,000,000 bytes of enwik10, predictions pushed first
([`rust-funnel/RESULT-FUNNEL-CURVE-20261115.hbp`](rust-funnel/RESULT-FUNNEL-CURVE-20261115.hbp)):

| Pieces of 370,371 bytes | Records | Distinct | Functions (seen twice or more) | Records that tie | Distinct per piece |
|---:|---:|---:|---:|---:|---:|
| 27 | 11,184,804 | 271,704 | 24,493 | 97.7% | 10,063 |
| 81 | 33,554,412 | 743,085 | 63,044 | 97.9% | 9,174 |
| 243 | 100,663,236 | 2,059,508 | 161,160 | 98.1% | 8,475 |
| 729 | 301,989,708 | 5,644,821 | 422,335 | 98.2% | 7,743 |

- Each new piece adds fewer new records than the one before: the catalogue grows more slowly than the body.
- At the other floors, on the first 27 pieces: 98.0% tie at level 64, 97.1% at level 1024, 96.8% at level 4096.
- Pieces ten times larger (3,703,704 bytes) learn more each: 24,769 distinct records per piece at 27 pieces, 94.4% tie.
- The coded crown streams of the same pieces have no ties at any level: the funnel works on the trained functions.
- Four of the five predictions were met. The one missed: the catalogue at 729 pieces is 77% of straight-line growth
  from 27 pieces, where below 60% was predicted.

```sh
cargo build --release --manifest-path rust-funnel/Cargo.toml
sh rust-funnel/funnel_curve.sh BODY 370371 729 8 work/
```

## A crown per piece, through the floors, and back

[`right-way/`](right-way/) holds the chain run on the first 10,000,000 bytes of enwik10: 27 pieces, each into its own
crown (2,384,542 bytes in all), each crown down the floors 64, 256, 1024, 4096, absorbed in order, three rotated
passes, and the same waves in reverse. The body returned with its SHA-256 equal and all 27 crowns matching their
glyphs ([`RESULT-RIGHT-WAY-10MB-20261115.hbp`](right-way/RESULT-RIGHT-WAY-10MB-20261115.hbp)). `glyph_of.py` writes a
crown's glyph in the BEHCS-1024 alphabet from the crown's own content. The script names the machine-local programs it
calls (the July floor codec is in the full record, not in this repository).

## What is in this repository

| Path | What it is |
|---|---|
| [`rust-selector/`](rust-selector/) | The context and selector program: `split` and `join` |
| [`rust-funnel/`](rust-funnel/) | The funnel's rule, counted: `funnel`, with `funnel_curve.sh` |
| [`right-way/`](right-way/) | A crown per piece through the floors and back: the script, the crown-per-piece program, the glyph tool, the 10 MB result |
| [`rust-leaves/`](rust-leaves/) | The coloured leaves program: `black`, `white`, `hold` |
| [`rust/`](rust/) | The codex program `asolaria_codex`: `drill`, `pack`, `unpack`, `train`, `enc`, `dec`, `resolve` |
| [`crown/vc65.rs`](crown/vc65.rs) | The crown codec, one Rust file: 1.3645 bits per character on enwik9, exact restore |
| [`master-key/`](master-key/) | The master key (`masterkey.xz`) and a 4 MB stream coded against it |
| [`kernel-and-key/`](kernel-and-key/) | The Asolaria kernel, the constellation key, the PLAYBOOK and the CODEBOOK |
| [`codexes/`](codexes/) | Nine 3,200-byte codexes (enwik10, enwik9, enwik8, two corpora, a photo body) |
| [`codexes/enwik10-first-rooms/`](codexes/enwik10-first-rooms/) | 14,000 room codexes that return the first 95,823,079 bytes of enwik10 |
| [`SHA256SUMS`](SHA256SUMS) | SHA-256 of every file |

The leaves and selector programs use the library
[`path2-two-shadow-recovery`](https://github.com/JesseBrown1980/path2-two-shadow-recovery) unchanged, pinned to an
exact commit. Cargo fetches it when you build.

## Use it

Build the three programs:

```sh
git clone https://github.com/JesseBrown1980/The-system
cd The-system
cargo build --release --manifest-path rust-selector/Cargo.toml
cargo build --release --manifest-path rust-leaves/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
```

### Context and selector: any body at 0.125 bits per character

```sh
rust-selector/target/release/selector split BODY work/
rust-selector/target/release/selector join  work/ BODY_BYTES BODY.back
cmp BODY BODY.back
```

`split` writes the context (`work/context-0.u32`, `work/context-1.u32`) and the selector (`work/selector.bits`).
`join` returns the body from those. For three cylinders and an empty selector:

```sh
rust-selector/target/release/selector split BODY work3/ 33554467,33554393,33554321
```

### Coloured leaves: any three rebuild the body

```sh
rust-leaves/target/release/path2_driver black BODY leaves/
rust-leaves/target/release/path2_driver white leaves/ 2 4 6 BODY_BYTES BODY.back
rust-leaves/target/release/path2_driver hold  leaves/ 0 1 BODY_BYTES
```

`black` writes seven leaves. `white` rebuilds from the three you name. `hold` shows two leaves emitting nothing.

### Codexes and the master key

```sh
xz -d -c master-key/masterkey.xz > key.raw

# one 3,200-byte codex that names a body
rust/target/release/asolaria_codex drill BODY BODY.qr

# room codexes: divide a body, bring it back from the key and the codexes
rust/target/release/asolaria_codex pack   key.raw 2 BODY BODY.rooms BODY.stubs
rust/target/release/asolaria_codex unpack key.raw 2 BODY.rooms BODY.back

# the example: the first 95,823,079 bytes of enwik10 from the key and 14,000 codexes
rust/target/release/asolaria_codex unpack key.raw 2 \
    codexes/enwik10-first-rooms/enwik10-first-14000.rooms enwik10-first.bin
sha256sum enwik10-first.bin
# 65775c95ba220e9cc45bf6f2e80ce2ab738a78e1bb89ed9dbfa29f8a5980835b
```

### The crown codec

```sh
rustc -O crown/vc65.rs -o vc65
./vc65 BODY 10 --emit BODY.crown
```

It codes the body, decodes it again, checks the hash and prints the size.

## Checked on a clean machine

[`.github/workflows/check.yml`](.github/workflows/check.yml) builds everything on a clean GitHub machine on every
push and runs: the file hashes; the selector at one bit per block and at zero; three leaves rebuilding and two
holding; the room codex example with its hash; the master key round trip; a codex of 3,200 bytes that repeats.
All programs pass `cargo clippy --release -- -D warnings` on Rust 1.81.
