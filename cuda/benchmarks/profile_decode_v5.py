import argparse
from pathlib import Path

import torch
from torch.profiler import ProfilerActivity, profile

from fastvid_cuda import decode_v5


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("stream", type=Path)
    parser.add_argument("--predictor", choices=("wavefront", "serial"), default="wavefront")
    args = parser.parse_args()
    encoded = torch.frombuffer(bytearray(args.stream.read_bytes()), dtype=torch.uint8).cuda()
    decode_v5(encoded, predictor=args.predictor)
    torch.cuda.synchronize()
    with profile(activities=[ProfilerActivity.CPU, ProfilerActivity.CUDA]) as profiler:
        decode_v5(encoded, predictor=args.predictor)
        torch.cuda.synchronize()
    print(profiler.key_averages().table(sort_by="cuda_time_total", row_limit=20))


if __name__ == "__main__":
    main()
