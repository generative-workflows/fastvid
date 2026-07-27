from pathlib import Path

from setuptools import setup
from torch.utils.cpp_extension import BuildExtension, CUDAExtension


ROOT = Path(__file__).parent

setup(
    name="fastvid-cuda",
    version="0.1.0",
    packages=["fastvid_cuda"],
    ext_modules=[
        CUDAExtension(
            "fastvid_cuda._C",
            [
                str(ROOT / "csrc" / "fastvid_cuda.cpp"),
                str(ROOT / "csrc" / "decode_v5.cu"),
            ],
            extra_compile_args={
                "cxx": ["-O3", "-std=c++17"],
                "nvcc": ["-O3", "--use_fast_math", "-lineinfo", "-std=c++17"],
            },
        )
    ],
    cmdclass={"build_ext": BuildExtension.with_options(use_ninja=True)},
)
