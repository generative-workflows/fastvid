#![forbid(unsafe_code)]

//! Experimental Fastvid codec.
//!
//! The version-zero bitstream is unstable. Its purpose is to make codec ideas
//! measurable while keeping tile independence and malformed-input handling
//! explicit.

mod codec;
mod metrics;
mod model;

pub use codec::{
    DecodedTile, StreamInfo, decode, decode_tile, decode_with_reference, encode,
    encode_with_reference, inspect,
};
pub use metrics::{QualityMetrics, compare_plane, ssim_plane};
pub use model::{CodecError, CodecOptions, Frame, FrameRate, PixelFormat, Plane};
