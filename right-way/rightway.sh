#!/bin/sh
# The right way, as the laws and the operator describe it, with the existing programs:
#   the body divided into 27 pieces (one rime dimension)
#   -> each piece into its own small crown (crown codec vc65, unchanged compress/decompress)
#   -> each crown named by its glyph: a 3,200-byte codex and an 8-byte handle
#   -> each crown down the floors 64 / 256 / 1024 / 4096 (July Omega codec), full return at each floor
#   -> the kernel absorbs the floored crowns in order (they are streamed in as copies)
#   -> three rotated passes of the kernel body: normal, anti, anti-anti (three cylinders, Path 2)
# and then the same waves in reverse: three passes -> kernel body -> floors up -> crowns -> pieces -> the body.
#   sh rightway.sh BODY WORKDIR
BODY=$1; W=$2
C=/c/tmp/opus55/rightway/crown_piece.exe
OMEGA=/c/Users/acer/omega-hyper-codec-integration-v0/omega_hyper_codec_v0.exe
SEL=/c/tmp/opus55/newperson/The-system/rust-selector/target/release/selector.exe
DRILL=/c/tmp/opus55/newperson/The-system/rust/target/release/asolaria_codex.exe
PIECES=27; PAR=6; CPAR=1
rm -rf "$W"; mkdir -p "$W/p" "$W/c" "$W/g" "$W/f" "$W/back"
N=$(stat -c%s "$BODY"); BSHA=$(sha256sum "$BODY" | cut -c1-64)
echo "RW0|body_bytes=$N|body_sha256=$BSHA|pieces=$PIECES|json=0"
T0=$(date +%s)

# 1. divide
PS=$(( (N + PIECES - 1) / PIECES ))
split -b $PS -d -a 2 "$BODY" "$W/p/piece-"
echo "RW1|stage=divided|piece_bytes=$PS|pieces=$(ls $W/p | wc -l)|json=0"

# 2. a crown per piece, PAR at a time
S=$(date +%s)
ls $W/p | xargs -P $CPAR -I{} sh -c "$C e $W/p/{} 10 $W/c/{}.crown > $W/c/{}.row"
CB=$(cat $W/c/*.crown | wc -c)
[ "$(ls $W/c/*.crown | wc -l)" = "$PIECES" ] || { echo "RWHALT|crowns_made=$(ls $W/c/*.crown 2>/dev/null | wc -l)|wanted=$PIECES|instrument_fault"; exit 1; }
echo "RW2|stage=crowns|crowns=$(ls $W/c/*.crown | wc -l)|crown_bytes=$CB|seconds=$(( $(date +%s)-S ))|json=0"

# 3. the glyph of each crown: its 3,200-byte codex and 8-byte handle
: > $W/kernel.stubs.hex
for f in $W/c/*.crown; do b=$(basename $f .crown); $DRILL drill $f $W/g/$b.qr > /dev/null; sha256sum $W/g/$b.qr | cut -c1-16 >> $W/kernel.stubs.hex; done
echo "RW3|stage=glyphs|codexes=$(ls $W/g | wc -l)|codex_bytes_each=3200|distinct_handles=$(sort -u $W/kernel.stubs.hex | wc -l)|json=0"

# 4. each crown down the floors
S=$(date +%s)
ls $W/c/*.crown | xargs -n1 basename | sed 's/.crown$//' | xargs -P $PAR -I{} sh -c "$OMEGA pack $W/c/{}.crown $W/f/{}.omega raw > /dev/null"
FB=$(cat $W/f/*.omega | wc -c)
echo "RW4|stage=floors_64_256_1024_4096|archives=$(ls $W/f | wc -l)|floor_bytes=$FB|seconds=$(( $(date +%s)-S ))|json=0"

# 5. the kernel absorbs them in order; the index says where each one sits
: > $W/kernel.body; : > $W/kernel.index
for f in $W/f/*.omega; do echo "$(basename $f) $(stat -c%s $f)" >> $W/kernel.index; cat $f >> $W/kernel.body; done
KB=$(stat -c%s $W/kernel.body)
echo "RW5|stage=kernel_absorbed|kernel_body_bytes=$KB|index_bytes=$(stat -c%s $W/kernel.index)|json=0"

# 6. three rotated passes: normal, anti, anti-anti
$SEL split $W/kernel.body $W/three 33554467,33554393,33554321 > $W/three.row
TB=$(cat $W/three/context-*.u32 | wc -c)
echo "RW6|stage=three_passes_normal_anti_antianti|bytes_each=$(stat -c%s $W/three/context-0.u32)|three_bytes=$TB|selector_bytes=$(stat -c%s $W/three/selector.bits)|json=0"
FWD=$(( $(date +%s)-T0 ))

# ---- the same waves in reverse, reading only: the three passes, the index, the piece count
T1=$(date +%s)
rm -rf $W/p $W/c $W/f $W/kernel.body
$SEL join $W/three $KB $W/back/kernel.body > /dev/null
mkdir -p $W/back/f $W/back/c $W/back/p
off=0
while read name len; do tail -c +$((off+1)) $W/back/kernel.body | head -c $len > $W/back/f/$name; off=$((off+len)); done < $W/kernel.index
ls $W/back/f | sed 's/.omega$//' | xargs -P $PAR -I{} sh -c "$OMEGA unpack $W/back/f/{}.omega $W/back/c/{}.crown > /dev/null"
ls $W/back/c | sed 's/.crown$//' | xargs -P $CPAR -I{} sh -c "$C d $W/back/c/{}.crown 10 $W/back/p/{} > /dev/null"
cat $W/back/p/piece-* > $W/back/body
GSHA=$(sha256sum $W/back/body | cut -c1-64)
# every returned crown has the glyph recorded on the way in
GOK=0; i=0
for f in $W/back/c/*.crown; do i=$((i+1)); $DRILL drill $f $W/back/q.qr > /dev/null; h=$(sha256sum $W/back/q.qr | cut -c1-16); [ "$h" = "$(sed -n ${i}p $W/kernel.stubs.hex)" ] && GOK=$((GOK+1)); done
echo "RWBACK|body_bytes=$(stat -c%s $W/back/body)|body_sha256=$GSHA|equals_body=$([ "$GSHA" = "$BSHA" ] && echo 1 || echo 0)|glyphs_matching=$GOK|seconds=$(( $(date +%s)-T1 ))|json=0"
echo "RWSUM|body_bytes=$N|crown_bytes=$CB|crown_bits_per_character_x1000=$(( CB*8000/N ))|floor_bytes=$FB|three_passes_bytes=$TB|glyph_codex_bytes=$(( PIECES*3200 ))|handle_bytes=$(( PIECES*8 ))|forward_seconds=$FWD|json=0"
