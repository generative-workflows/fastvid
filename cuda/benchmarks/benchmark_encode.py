import argparse
import statistics
import time
from pathlib import Path

import torch

from fastvid_cuda import encode


def load_yuv422(path, width, height):
    raw = bytearray(path.read_bytes())
    samples = torch.frombuffer(raw, dtype=torch.uint16).clone()
    y_samples = width * height
    chroma_width = (width + 1) // 2
    chroma_samples = chroma_width * height
    expected = y_samples + 2 * chroma_samples
    if samples.numel() != expected:
        raise ValueError(f"expected {expected * 2} bytes, found {len(raw)}")
    return (
        samples[:y_samples].view(height, width).cuda(),
        samples[y_samples : y_samples + chroma_samples].view(height, chroma_width).cuda(),
        samples[y_samples + chroma_samples :].view(height, chroma_width).cuda(),
    )


def benchmark(planes, bit_depth, quality, frame_rate, tile_size, warmups, trials):
    for _ in range(warmups):
        encode(
            planes,
            bit_depth=bit_depth,
            quality=quality,
            frame_rate=frame_rate,
            tile_size=tile_size,
        )
    torch.cuda.synchronize()
    elapsed = []
    encoded = None
    for _ in range(trials):
        torch.cuda.synchronize()
        start = time.perf_counter_ns()
        encoded = encode(
            planes,
            bit_depth=bit_depth,
            quality=quality,
            frame_rate=frame_rate,
            tile_size=tile_size,
        )
        torch.cuda.synchronize()
        elapsed.append((time.perf_counter_ns() - start) / 1e9)
    median = statistics.median(elapsed)
    pixels = planes[0].numel()
    raw_bytes = sum(plane.numel() * plane.element_size() for plane in planes)
    return {
        "encoded_bytes": encoded.numel(),
        "raw_bytes": raw_bytes,
        "ratio": raw_bytes / encoded.numel(),
        "median_ms": median * 1e3,
        "encode_gpps": pixels / median / 1e9,
        "raw_gb_s": raw_bytes / median / 1e9,
        "encoded": encoded,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("width", type=int)
    parser.add_argument("height", type=int)
    parser.add_argument("bit_depth", type=int)
    parser.add_argument("qualities", nargs="+", type=int)
    parser.add_argument("--frame-rate", default="24/1")
    parser.add_argument("--tile-size", default="256x128")
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--trials", type=int, default=20)
    args = parser.parse_args()
    frame_rate = tuple(map(int, args.frame_rate.split("/")))
    tile_size = tuple(map(int, args.tile_size.split("x")))
    planes = load_yuv422(args.input, args.width, args.height)
    print(
        "input\tquality\twidth\theight\tbit_depth\tencoded_bytes\traw_bytes\tratio\t"
        "median_ms\tencode_gpps\traw_gb_s"
    )
    for quality in args.qualities:
        row = benchmark(
            planes,
            args.bit_depth,
            quality,
            frame_rate,
            tile_size,
            args.warmups,
            args.trials,
        )
        print(
            f"{args.input}\t{quality}\t{args.width}\t{args.height}\t{args.bit_depth}\t"
            f"{row['encoded_bytes']}\t{row['raw_bytes']}\t{row['ratio']:.6f}\t"
            f"{row['median_ms']:.6f}\t{row['encode_gpps']:.9f}\t{row['raw_gb_s']:.6f}"
        )


if __name__ == "__main__":
    main()
