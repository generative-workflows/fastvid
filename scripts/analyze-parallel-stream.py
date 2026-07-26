#!/usr/bin/env python3
"""Audit version-5 shard geometry and serialization overhead."""

import math
import struct
import sys

SHARD_SYMBOLS = 4096
BLOCK_SYMBOLS = 128
BLOCK_PACK_MODE = 18
PARALLEL_MODE = 19
FULL_TILE_PREDICTOR = 6


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def get_varint(body: bytes, cursor: int) -> tuple[int, int]:
    value = 0
    shift = 0
    for _ in range(5):
        byte = body[cursor]
        cursor += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, cursor
        shift += 7
    raise ValueError("oversized varint")


def audit_zero_run(body: bytes, count: int) -> tuple[int, int]:
    cursor = 0
    decoded = 0
    while decoded < count:
        token, cursor = get_varint(body, cursor)
        decoded += token // 2 + 1 if token % 2 == 0 else 1
    if decoded != count or cursor != len(body):
        raise ValueError("noncanonical zero-run shard")
    return 0, count


def get_bit(body: bytes, bit: int) -> int:
    return body[bit // 8] >> (bit % 8) & 1


def audit_rice(body: bytes, count: int, parameter: int) -> tuple[int, int, int]:
    lane_count = min(4, count)
    table_bytes = (lane_count - 1) * 4
    lengths = list(struct.unpack_from(f"<{lane_count - 1}I", body, 0))
    lengths.append(len(body) - table_bytes - sum(lengths))
    cursor = table_bytes
    padding_bits = 0
    maximum_lane_symbols = 0
    for lane, length in enumerate(lengths):
        lane_body = body[cursor : cursor + length]
        lane_symbols = (count - lane + lane_count - 1) // lane_count
        maximum_lane_symbols = max(maximum_lane_symbols, lane_symbols)
        bit = 0
        for _ in range(lane_symbols):
            while get_bit(lane_body, bit) == 0:
                bit += 1
            bit += 1 + parameter
        padding_bits += len(lane_body) * 8 - bit
        if any(get_bit(lane_body, trailing) for trailing in range(bit, len(lane_body) * 8)):
            raise ValueError("nonzero Rice padding")
        cursor += length
    if cursor != len(body):
        raise ValueError("bad Rice lane lengths")
    return table_bytes, maximum_lane_symbols, padding_bits


def audit_block_pack(body: bytes, count: int) -> tuple[int, int, int]:
    cursor = 0
    decoded = 0
    controls = 0
    padding_bits = 0
    while decoded < count:
        width = body[cursor]
        cursor += 1
        controls += 1
        block_count = min(BLOCK_SYMBOLS, count - decoded)
        packed_bytes = (block_count * width + 7) // 8
        padding_bits += packed_bytes * 8 - block_count * width
        cursor += packed_bytes
        decoded += block_count
    if cursor != len(body):
        raise ValueError("noncanonical block-pack shard")
    return controls, BLOCK_SYMBOLS, padding_bits


def main() -> None:
    stream = open(sys.argv[1], "rb").read()
    if stream[:4] != b"FVID" or stream[4] != 5:
        raise ValueError("expected a version-5 Fastvid stream")
    width, height = struct.unpack_from("<II", stream, 8)
    bit_depth = stream[7] + 8
    tile_count = struct.unpack_from("<I", stream, 28)[0]
    directory_end = 32 + tile_count * 32
    shard_samples: list[int] = []
    shard_bytes: list[int] = []
    modes = {mode: 0 for mode in range(19)}
    shard_header_bytes = 0
    mode_control_bytes = 0
    padding_bits = 0
    maximum_predictor_span = 0
    maximum_entropy_span = 0
    for tile_index in range(tile_count):
        entry = 32 + tile_index * 32
        entropy_mode = stream[entry + 1]
        predictor_mode = stream[entry + 2]
        tile_width, tile_height = struct.unpack_from("<II", stream, entry + 12)
        offset = struct.unpack_from("<Q", stream, entry + 20)[0]
        length = struct.unpack_from("<I", stream, entry + 28)[0]
        if entropy_mode != PARALLEL_MODE or predictor_mode != FULL_TILE_PREDICTOR:
            raise ValueError("unexpected version-5 directory modes")
        maximum_predictor_span = max(
            maximum_predictor_span, tile_width + tile_height - 1
        )
        sample_count = tile_width * tile_height
        decoded = 0
        cursor = offset
        end = offset + length
        while decoded < sample_count:
            mode = stream[cursor]
            body_length = struct.unpack_from("<H", stream, cursor + 1)[0]
            body = stream[cursor + 3 : cursor + 3 + body_length]
            count = min(SHARD_SYMBOLS, sample_count - decoded)
            if len(body) != body_length:
                raise ValueError("truncated shard body")
            modes[mode] += 1
            shard_header_bytes += 3
            if mode == 0:
                controls, entropy_span = audit_zero_run(body, count)
                shard_padding = 0
            elif 1 <= mode <= 17:
                controls, entropy_span, shard_padding = audit_rice(
                    body, count, mode - 1
                )
            elif mode == BLOCK_PACK_MODE:
                controls, entropy_span, shard_padding = audit_block_pack(body, count)
            else:
                raise ValueError("unknown shard mode")
            mode_control_bytes += controls
            padding_bits += shard_padding
            maximum_entropy_span = max(maximum_entropy_span, entropy_span)
            shard_samples.append(count)
            shard_bytes.append(3 + body_length)
            cursor += 3 + body_length
            decoded += count
        if cursor != end:
            raise ValueError("trailing tile bytes")

    complete_metadata = 32 + tile_count * 32 + shard_header_bytes + mode_control_bytes
    luma_megapixels = width * height / 1_000_000
    mode_text = ",".join(f"{mode}:{count}" for mode, count in modes.items() if count)
    print(
        "input\twidth\theight\tbit_depth\ttiles\tshards\tshards_per_luma_mp\t"
        "max_predictor_dag_span\tmax_entropy_state_span\tp50_shard_samples\t"
        "p95_shard_samples\tmax_shard_samples\tp50_shard_bytes\tp95_shard_bytes\t"
        "max_shard_bytes\tmetadata_bytes\tmetadata_bits_per_luma_pixel\t"
        "metadata_percent\tpadding_bits\tpadding_bits_per_luma_pixel\tmodes"
    )
    print(
        f"{sys.argv[1]}\t{width}\t{height}\t{bit_depth}\t{tile_count}\t"
        f"{len(shard_samples)}\t{len(shard_samples) / luma_megapixels:.3f}\t"
        f"{maximum_predictor_span}\t{maximum_entropy_span}\t"
        f"{percentile(shard_samples, 0.5)}\t{percentile(shard_samples, 0.95)}\t"
        f"{max(shard_samples)}\t{percentile(shard_bytes, 0.5)}\t"
        f"{percentile(shard_bytes, 0.95)}\t{max(shard_bytes)}\t"
        f"{complete_metadata}\t{complete_metadata * 8 / width / height:.6f}\t"
        f"{complete_metadata / len(stream) * 100:.6f}\t{padding_bits}\t"
        f"{padding_bits / width / height:.6f}\t{mode_text}"
    )


if __name__ == "__main__":
    main()
