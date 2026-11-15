# glyph_of.py: the glyph of a crown, from the alphabet that already exists.
# The address is the crown's own content (first 60 bits of its SHA-256), written with Q-prism's behcs.encode in the
# local BEHCS-1024 alphabet: six glyphs. Nothing is chosen by hand; the glyph comes from what the crown is.
#   python -I glyph_of.py CROWN [CROWN ...]   ->  one row per crown, utf-8
import sys, os, hashlib, io
sys.path.insert(0, 'C:/Users/acer/q-prism')
from qprism import behcs
out = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', newline='\n')
for path in sys.argv[1:]:
    h = hashlib.sha256(open(path, 'rb').read()).digest()
    v = int.from_bytes(h[:8], 'big') >> 4
    g = behcs.encode(v)
    assert behcs.decode(g) == v and len(g) == behcs.WIDTH
    cps = '.'.join(str(behcs.GLYPHS.index(c)) for c in g)
    out.write('GLYPH|crown=%s|glyphs=%s|codepoints=%s|address60=%015x|alphabet=BEHCS-1024|json=0\n' % (os.path.basename(path), g, cps, v))
out.flush()
