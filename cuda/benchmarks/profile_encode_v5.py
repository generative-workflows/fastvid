import argparse
from pathlib import Path

import torch

from benchmark_encode_v5 import load_yuv422
from fastvid_cuda import encode_v5


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("width", type=int)
    parser.add_argument("height", type=int)
    parser.add_argument("bit_depth", type=int)
    parser.add_argument("quality", type=int)
    args = parser.parse_args()
    planes = load_yuv422(args.input, args.width, args.height)
    for _ in range(3):
        encode_v5(planes, bit_depth=args.bit_depth, quality=args.quality)
    torch.cuda.synchronize()
    with torch.profiler.profile(
        activities=[torch.profiler.ProfilerActivity.CPU, torch.profiler.ProfilerActivity.CUDA],
        record_shapes=True,
        profile_memory=True,
    ) as profiler:
        encode_v5(planes, bit_depth=args.bit_depth, quality=args.quality)
        torch.cuda.synchronize()
    print(profiler.key_averages().table(sort_by="self_cuda_time_total", row_limit=30))


if __name__ == "__main__":
    main()
