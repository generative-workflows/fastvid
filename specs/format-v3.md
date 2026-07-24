# Fastvid experimental bitstream, version 3

Status: **experimental; incompatible changes are allowed**

Version 3 adds a tile-local order-0 byte-rANS entropy mode to the 8-bit
format. Header and directory sizes, prediction, quantization, tile order, and
the version-2 zero-run/Rice modes are unchanged. High-bit encoders continue
to emit version 2.

## Header and directory

The header is the 32-byte version-2 header with byte 4 set to 3. Version 3 is
valid only for an 8-bit `PixelFormat`, and reserved header byte 7 remains
zero.

Directory entropy-mode byte 1 has the following meanings:

| Mode | Entropy representation |
|---:|---|
| 0 | canonical zero-run/varint |
| 1–9 | Rice parameter 0–8 |
| 10 | order-0 byte-rANS |

Prediction-mode byte 2 retains all version-2 meanings. Order-0 coding is
local to one tile and does not change spatial or temporal dependency depth.

## Order-0 payload

An entropy-mode-10 payload consists of:

```text
u8 table_log
varint symbol_count
repeat symbol_count times:
    varint symbol_delta
    varint frequency       # omitted for the final symbol
little-endian u32 final_state
byte[] renormalization
```

`table_log` is in 8 through 12 and defines `table_size = 2^table_log`.
`symbol_count` is in 1 through `min(511, table_size)`. Symbols are folded
residuals in 0 through 510 and appear in strictly increasing order.
`symbol_delta` is relative to the preceding value, initially zero; every
delta after the first is nonzero.

Every transmitted frequency is positive. The final frequency is
`table_size - sum(previous frequencies)`, must be positive, and is omitted
from the stream. Frequencies sum exactly to `table_size`. A symbol's
`cumulative` value is the sum of preceding frequencies.

All integers called `varint` use the existing canonical unsigned `u32`
base-128 syntax. The state word is little-endian.

## rANS state machine

Let `L = 2^23`, `M = 2^table_log`, and let a symbol have frequency `f` and
cumulative value `c`.

Encoding begins at state `L` and processes folded residuals in reverse raster
order. Before advancing a symbol, emit the low state byte and shift the state
right by eight while:

```text
state >= ((L >> table_log) << 8) * f
```

Then advance:

```text
state = ((state / f) << table_log) + (state mod f) + c
```

The encoder stores the resulting final state followed by emitted
renormalization bytes in reverse emission order.

Decoding starts from the stored final state, which must be at least `L`. For
each tile sample in raster order:

```text
slot  = state & (M - 1)
symbol = table[slot]
state = f * (state >> table_log) + slot - c
while state < L:
    state = (state << 8) | next_payload_byte
```

The frequency-expanded decoding table maps each interval `[c, c + f)` to its
symbol. Decoding exactly the tile sample count must consume every payload
byte and finish at state `L`; otherwise the payload is malformed.

## Reference-encoder selection

For each predictor candidate, the reference encoder normalizes observed
frequencies at table logs 8 through 12. It gives every observed symbol one
count, apportions the remaining counts by largest remainder (ties by lower
symbol), and chooses the lowest modeled complete byte cost, breaking ties by
lower table log.

Predictor selection uses this logarithmic payload estimate. Only the selected
predictor is materialized as rANS. The encoder retains order-0 mode only when
its exact payload is strictly smaller than the selected zero-run/Rice
payload. This search policy is non-normative; compliant encoders may choose
any valid entropy and prediction modes.

## Compatibility and validation

The version-3 8-bit decoder accepts versions 0, 2, and 3. Entropy mode 10 is
malformed in versions 0 and 2. Legacy prediction-mode restrictions remain
unchanged. The high-bit decoder accepts versions 1 and 2 and does not accept
version 3.

Quality 100 still has quantization step one and therefore reconstructs every
sample exactly for every entropy mode.
