# Tajima DST

DST is the first export target because it is the commercial production interchange format. This compiler follows the public record layout and is checked bit for bit against pyembroidery's `DstWriter.write`. There is no official vendor specification. pyembroidery is the interoperability oracle, not the engine.

## File layout

A file is a 512-byte header followed by 3-byte records. The last record is the end record `00 00 F3`.

Design coordinates are millimetres with Y positive downward, the same direction as a design canvas and as pyembroidery's pattern space. The file stores the negated Y. The reader negates it back, so a round trip returns the design-space point.

One count is 0.1 mm. Conversion is `millimetres * 10`, then half-to-even rounding, which is Python 3 `round`. A value within `1e-8` of a `n + 0.5` boundary is treated as a tie so that decimal millimetre inputs survive the binary multiply. Dividing by `0.1` is not used, because that is a different binary operation.

Examples: `round(0.5) = 0`, `round(1.5) = 2`, `round(2.5) = 2`, `round(-1.5) = -2`.

## Header

The header is sequential CR-terminated ASCII, then `0x1A`, then spaces through byte 512. Fields are not fixed-width slots glued at hard offsets. Each field is written in this order:

| Field | Meaning |
|---|---|
| `LA:` | Design name, 16 ASCII characters, space padded. Non-ASCII, CR, LF, and `0x1A` become `_`. An empty name becomes `Untitled`. |
| `ST:` | Count of 3-byte records, including splits and the end record. Width 7, space padded. Maximum 9999999. |
| `CO:` | Colour-change records. Width 3. Maximum 999. |
| `+X:` `-X:` | Absolute value of the maximum and minimum X, in counts. Width 5. |
| `+Y:` `-Y:` | Absolute value of the maximum and minimum pattern-space Y, in counts. Width 5. |
| `AX:` `AY:` | End position. `AY` is the negation of pattern-space Y. A leading `+` or `-`, then width 5. |
| `MX:` `MY:` | `+0`. |
| `PD:` | `******`. |

`AX` is not written immediately after `-X`. `+Y` and `-Y` sit between them.

Extent and end fields hold five digits, so a coordinate must stay within ±9999.9 mm (±99999 counts). The compiler rejects anything outside that range and says to move the design closer to the origin.

With `WriteOptions.extended_header`, thread stops that fit in the remaining header bytes are written as `TC:#RRGGBB,description,catalog`. The reader rebuilds a `ThreadSpec` from those lines: brand empty, name from the description, code from the catalog, colour from the hex. The default writer does not emit `TC` lines, matching pyembroidery's non-extended writer.

## Records

Each record is three bytes. Movement uses signed weights ±1, ±3, ±9, ±27, and ±81 counts. The packer is the threshold greedy used by pyembroidery, not balanced ternary:

- magnitude greater than 40 selects 81
- greater than 13 selects 27
- greater than 4 selects 9
- greater than 1 selects 3
- greater than 0 selects 1

The same thresholds apply to the negative side. A stitch or jump component must already be in −121..=121. Every pair in that square round-trips, and the packed bytes match `encode_record` for both stitch and jump.

X bits, in pattern space:

| Weight | Positive | Negative |
|---|---|---|
| 81 | byte 2 bit 2 | byte 2 bit 3 |
| 27 | byte 1 bit 2 | byte 1 bit 3 |
| 9 | byte 0 bit 2 | byte 0 bit 3 |
| 3 | byte 1 bit 0 | byte 1 bit 1 |
| 1 | byte 0 bit 0 | byte 0 bit 1 |

Y bits are packed after negating pattern Y, so these are file-space bits:

| Weight | Positive file Y | Negative file Y |
|---|---|---|
| 81 | byte 2 bit 5 | byte 2 bit 4 |
| 27 | byte 1 bit 5 | byte 1 bit 4 |
| 9 | byte 0 bit 5 | byte 0 bit 4 |
| 3 | byte 1 bit 7 | byte 1 bit 6 |
| 1 | byte 0 bit 7 | byte 0 bit 6 |

Control lives in byte 2. Classification checks the more specific mask first:

| Command | Byte 2 | Notes |
|---|---|---|
| End | `0xF3` | Both Y ±81 bits are set and cancel. `dx` and `dy` are ignored. |
| Colour change | `0xC3` | `dx` and `dy` are ignored. |
| Sequin mode | `0x43` | Read back as a jump plus `sequin_mode_toggles` metadata. |
| Jump | `0x83` | Bit 7 set, plus the stitch bits 0 and 1. |
| Stitch | `0x03` | Bits 0 and 1. |

`encode_record(1, -1, stitch)` is `81 00 03`.

## What the compiler does with a plan

The pen starts at `(0, 0)`. A sewing stitch jumps to its `from` point when the pen is elsewhere, then sews to `to`. Satin, fill, motif, and applique all become needle-down stitch records. DST does not store those classes. The reader returns them as running stitches, sets `object_id` to 0, and records that fact in `dst_kind_note`.

Jumps longer than ±121 counts on an axis are split. The split uses integer truncation toward zero of `delta * step / steps`, with `steps = ceil(max(|dx|, |dy|) / 121)`. The parts sum to the original delta and each component stays within ±121. Even splits such as 300 counts (30.0 mm) become 100 + 100 + 100 and match a pyembroidery pattern that was already split that way. pyembroidery's float interpolation can miss an odd endpoint by one count. This compiler keeps the exact sum.

`compile_dst` validates first. A sewing stitch longer than the caller's limit, or whose rounded axis delta exceeds ±121 counts, is an error. Splitting a sewing stitch would add needle penetrations, so the strict compile refuses it. `write_dst` still segments an oversize sewing delta so a low-level caller cannot emit an illegal record. The FFI compile uses the strict path: 12.1 mm maximum sewing stitch, 1000 mm maximum jump.

Colour change and trim move the pen to the command with jumps when needed, then write a zero-delta colour record or the trim jog. Byte-identical comparison with pyembroidery holds when the colour or trim already sits on the pen, which is how `DstWriter` emits them. pyembroidery updates its internal position by the command delta but writes a zero-delta colour record and a net-zero trim, so a colour change that also travels will not match byte for byte. This compiler makes that travel explicit.

Trim has no universal DST command. The body uses pyembroidery's `trim_at=3` signature in pattern counts: jump `(+2, +2)`, `(-4, -4)`, `(+2, +2)`. The net move is zero, and the jog is not added to the design bounds. The bytes are `82 41 83`, `82 82 83`, `82 41 83`. The reader collapses that exact three-jump sequence back into one trim before converting counts to millimetres. `ReadOptions.collapse_trim_signature` defaults to true. A real design that sews that exact jog will be read as a trim. Set the option to false to keep the jumps.

`ST` counts on-disk records, including the three trim jumps and the end record. pyembroidery's `DstWriter.write` counts pattern commands, so a trim makes its `ST` smaller than the number of 3-byte records. A plan with one stitch, an in-place trim, another stitch, and end has six records. This compiler writes `ST:      6`. The trim body still matches. Unsplit files with no trim (the square, a negative-Y stitch, and an in-place colour change) are byte-identical to `DstWriter.write`.

Do not compare files through `EmbPattern.write_dst`. That path runs pyembroidery's encoder and may insert jumps or tie-ins before the writer sees the pattern.

## Reader errors

A file shorter than 512 bytes, or a body that ends mid-record, is an error. The message says the file is truncated and that the next step is to export it again. A missing end record is metadata `missing_end`, not a hard error, so a partial design can still be inspected. When the header `ST` value disagrees with the records actually present, the reader stores `dst_st_mismatch` and still returns the stitches.

## Golden file

`fixtures/dst/square_running.dst` is a 10 mm square:

- `(0, 0)` to `(10, 0)`
- `(10, 0)` to `(10, 10)`
- `(10, 10)` to `(0, 10)`
- end record at `(0 mm, 10 mm)`, which is `(0, 100)` counts

The header begins `LA:Square` and the body is `09 04 07 60 10 13 06 08 0b 00 00 f3`. The file is 524 bytes. It matches both this compiler and `DstWriter.write`.
