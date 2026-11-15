#!/bin/sh
# The funnel's rule counted over the trained functions of a body's pieces.
# Each piece gets its own function table (asolaria_codex train, order 2: 33,554,432 bytes); the tables are streamed
# into the funnel, which prints a row at 1, 3, 9, 27, 81, 243, 729 pieces.
#   sh funnel_curve.sh BODY PIECE_BYTES PIECES BITS WORKDIR
BODY=$1; PS=$2; N=$3; BITS=$4; W=$5
HERE=$(cd "$(dirname "$0")" && pwd)
CODEX=$HERE/../rust/target/release/asolaria_codex
FUNNEL=$HERE/target/release/funnel
mkdir -p "$W"
i=0
while [ $i -lt $N ]; do
  dd if="$BODY" of="$W/piece.bin" bs=$PS skip=$i count=1 status=none
  "$CODEX" train "$W/table.fn" 2 "$W/piece.bin" > /dev/null || exit 1
  cat "$W/table.fn"
  i=$((i+1))
done | "$FUNNEL" $BITS 33554432
rm -f "$W/piece.bin" "$W/table.fn"
