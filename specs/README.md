# Fastvid specification workspace

`format-v0.md` is the executable-design specification for the experimental
bitstream. `Fastvid.lean` starts the formal model with the signed residual
mapping used by the Rust implementation.

Version zero has no stability guarantee. A stable version will require:

- fixed conformance vectors;
- fully specified malformed-input behavior and resource limits;
- checked arithmetic for every length and dimension;
- formal correspondence for selected Rust routines through Aeneas;
- an explicit patent and license review.

