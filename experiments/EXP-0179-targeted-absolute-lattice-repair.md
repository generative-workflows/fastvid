# EXP-0179 — Targeted absolute-lattice repair

Status: **REJECTED**

Date: 2026-07-30

Candidate revision: `eac723c7fd1d58ffabe4d226048f712e877b1050`.
Baseline revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Combining EXP-0178's absolute sample lattice with the smallest repair steps
suggested by EXP-0176 will clear the five remaining generation failures while
leaving already-repaired gray16 and RGB16 at baseline precision. At q90 the
changed cells use YUV8 step 1, YUV10 step 2, YUV16 step 108, RGB10 step 3,
and gray10 step 3. Absolute-lattice prediction is otherwise EXP-0178.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0179-targeted-lattice-repair-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0179-targeted-lattice-repair-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 421,325,148 | 5.108636x | 97.152237 | 0.524836 | 91.368111 / 2.237914 |

The candidate expanded rejection output by 73,491,195 bytes (21.13%). YUV8
and RGB10 generation quality were repaired, but YUV10 (two samples), YUV16,
and gray10 still failed. Both single-frame RGB10 samples exceeded the 1.0 ms
encode and 0.5 ms decode limits. Correctness and coverage passed. Full was not
run.

## Conclusion

Reject on generation quality, timing, and compression. Refinement is not
monotonic under the edit sequence: procedural YUV10 Butteraugli worsened from
2.109168 at step 3 to 2.237914 at step 2. Targeted step tuning cannot fund or
reliably clear the gate. Further work should test the lossless boundary and
error-decorrelating quantization rather than tune this map.

Related: [EXP-0178](EXP-0178-absolute-sample-lattice.md) and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
