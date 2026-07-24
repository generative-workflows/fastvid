# Fastvid experimental bitstream, version 3

Status: **experimental; incompatible changes are allowed**

Version 3 adds tile-local order-0 byte-rANS entropy modes to the 8-bit format.
Header and directory sizes, prediction, quantization, tile order, and the
version-2 zero-run/Rice modes are unchanged. High-bit encoders continue to
emit version 2.

## Header and directory

The header is the 32-byte version-2 header with byte 4 set to 3. Version 3 is
valid only for an 8-bit `PixelFormat`, and reserved header byte 7 remains
zero.

Directory entropy-mode byte 1 has the following meanings:

| Mode | Entropy representation |
|---:|---|
| 0 | canonical zero-run/varint |
| 1–9 | Rice parameter 0–8 |
| 10 | scalar order-0 byte-rANS |
| 11 | four-state interleaved order-0 byte-rANS |

Prediction-mode byte 2 retains all version-2 meanings. Order-0 coding is
local to one tile and does not change spatial or temporal dependency depth.

## Order-0 payload

An order-0 payload consists of:

```text
u8 table_log
varint symbol_count
repeat symbol_count times:
    varint symbol_delta
    varint frequency       # omitted for the final symbol
repeat state_count times:
    little-endian u32 final_state
byte[] renormalization
```

`state_count` is one for entropy mode 10 and four for entropy mode 11. It is
implied by the directory mode and is not stored in the payload.

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

Encoding initializes every state to `L` and processes folded residuals in
reverse raster order. Sample index `i` uses state `i mod state_count`. Before
advancing a symbol, emit the low byte of that sample's state into one shared
renormalization sequence and shift that state right by eight while:

```text
state >= ((L >> table_log) << 8) * f
```

Then advance:

```text
state = ((state / f) << table_log) + (state mod f) + c
```

The encoder stores final states in increasing state-index order followed by
the shared emitted renormalization bytes in reverse emission order.

Decoding starts from the stored final states, each of which must be at least
`L`. For each tile sample `i` in raster order, select state
`i mod state_count` and apply:

```text
slot  = state & (M - 1)
symbol = table[slot]
state = f * (state >> table_log) + slot - c
while state < L:
    state = (state << 8) | next_payload_byte
```

The frequency-expanded decoding table maps each interval `[c, c + f)` to its
symbol. Decoding exactly the tile sample count must consume every payload
byte and finish with every state at `L`; otherwise the payload is malformed.

## Reference-encoder selection

For each predictor candidate, the reference encoder normalizes observed
frequencies at table logs 8 through 12. It gives every observed symbol one
count, apportions the remaining counts by largest remainder (ties by lower
symbol), and chooses the lowest modeled complete byte cost, breaking ties by
lower table log.

Predictor selection uses this logarithmic payload estimate. Only the selected
predictor is materialized as rANS. The reference encoder charges all four
state words and uses mode 11 only when its additional 12 bytes are at most
five per mille of the modeled scalar rANS payload. Otherwise it retains mode
10. It retains either order-0 mode only when its exact payload is strictly
smaller than the selected zero-run/Rice payload. This search policy is
non-normative; compliant encoders may choose any valid entropy and prediction
modes.

## Compatibility and validation

The version-3 8-bit decoder accepts versions 0, 2, and 3. Entropy modes 10 and
11 are malformed in versions 0 and 2. Legacy prediction-mode restrictions
remain unchanged. The high-bit decoder accepts versions 1 and 2 and does not
accept version 3.

Quality 100 still has quantization step one and therefore reconstructs every
sample exactly for every entropy mode.
