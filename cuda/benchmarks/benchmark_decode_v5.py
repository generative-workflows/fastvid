import argparse
import statistics
import struct
import time
from pathlib import Path

import torch

from fastvid_cuda import decode_v5


def benchmark(path, placement, predictor, warmups, trials):
    payload = bytearray(path.read_bytes())
    encoded = torch.frombuffer(payload, dtype=torch.uint8).clone()
    if placement == "vram":
        encoded = encoded.cuda()
    width, height = struct.unpack_from("<II", payload, 8)
    grayscale = payload[5] == 0
    raw_samples = width * height if grayscale else width * height + 2 * ((width + 1) // 2) * height
    raw_bytes = raw_samples * 2

    for _ in range(warmups):
        decode_v5(encoded, predictor=predictor)
    torch.cuda.synchronize()
    elapsed = []
    for _ in range(trials):
        torch.cuda.synchronize()
        start = time.perf_counter_ns()
        result = decode_v5(encoded, predictor=predictor)
        torch.cuda.synchronize()
        elapsed.append((time.perf_counter_ns() - start) / 1e9)
        if not result:
            raise RuntimeError("decoder returned no planes")
    median = statistics.median(elapsed)
    luma_pixels = width * height
    return {
        "input": str(path),
        "placement": placement,
        "predictor": predictor,
        "width": width,
        "height": height,
        "encoded_bytes": len(payload),
        "raw_bytes": raw_bytes,
        "ratio": raw_bytes / len(payload),
        "median_ms": median * 1e3,
        "decode_gpps": luma_pixels / median / 1e9,
        "raw_gb_s": raw_bytes / median / 1e9,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("streams", nargs="+", type=Path)
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--trials", type=int, default=20)
    parser.add_argument("--predictors", nargs="+", choices=("wavefront", "serial"), default=("wavefront",))
    args = parser.parse_args()
    print(
        "input\tplacement\tpredictor\twidth\theight\tencoded_bytes\traw_bytes\tratio\t"
        "median_ms\tdecode_gpps\traw_gb_s"
    )
    for stream in args.streams:
        for placement in ("dram", "vram"):
            for predictor in args.predictors:
                row = benchmark(stream, placement, predictor, args.warmups, args.trials)
                print(
                    f"{row['input']}\t{row['placement']}\t{row['predictor']}\t"
                    f"{row['width']}\t{row['height']}\t{row['encoded_bytes']}\t"
                    f"{row['raw_bytes']}\t{row['ratio']:.6f}\t{row['median_ms']:.6f}\t"
                    f"{row['decode_gpps']:.9f}\t{row['raw_gb_s']:.6f}"
                )


if __name__ == "__main__":
    main()
