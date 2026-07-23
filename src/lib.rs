#![forbid(unsafe_code)]

//! Experimental Fastvid codec.
//!
//! The version-zero bitstream is unstable. Its purpose is to make codec ideas
//! measurable while keeping tile independence and malformed-input handling
//! explicit.

mod codec;
mod codec16;
mod metrics;
mod model;

pub use codec::{
    DecodedTile, StreamInfo, decode, decode_tile, decode_with_reference, encode,
    encode_with_reference, inspect,
};
pub use codec16::{
    DecodedTile16, decode_tile16, decode16, decode16_with_reference, encode16,
    encode16_with_reference, inspect16,
};
pub use metrics::{
    QualityMetrics, QualityMetrics16, compare_plane, compare_plane16, ssim_plane, ssim_plane16,
};
pub use model::{CodecError, CodecOptions, Frame, Frame16, FrameRate, PixelFormat, Plane, Plane16};
