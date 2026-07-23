use std::error::Error;
use std::fmt;

pub const MAX_FRAME_BYTES: usize = 1 << 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PixelFormat {
    Gray8 = 0,
    Yuv422p8 = 1,
    Gray10 = 2,
    Yuv422p10 = 3,
    Gray12 = 4,
    Yuv422p12 = 5,
    Gray16 = 6,
    Yuv422p16 = 7,
}

impl TryFrom<u8> for PixelFormat {
    type Error = CodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Gray8),
            1 => Ok(Self::Yuv422p8),
            2 => Ok(Self::Gray10),
            3 => Ok(Self::Yuv422p10),
            4 => Ok(Self::Gray12),
            5 => Ok(Self::Yuv422p12),
            6 => Ok(Self::Gray16),
            7 => Ok(Self::Yuv422p16),
            _ => Err(CodecError::Malformed("unknown pixel format")),
        }
    }
}

impl PixelFormat {
    pub const fn bit_depth(self) -> u8 {
        match self {
            Self::Gray8 | Self::Yuv422p8 => 8,
            Self::Gray10 | Self::Yuv422p10 => 10,
            Self::Gray12 | Self::Yuv422p12 => 12,
            Self::Gray16 | Self::Yuv422p16 => 16,
        }
    }

    pub const fn is_grayscale(self) -> bool {
        matches!(
            self,
            Self::Gray8 | Self::Gray10 | Self::Gray12 | Self::Gray16
        )
    }

    pub const fn is_high_bit_depth(self) -> bool {
        self.bit_depth() > 8
    }

    pub fn from_layout_and_depth(grayscale: bool, bit_depth: u8) -> Result<Self, CodecError> {
        match (grayscale, bit_depth) {
            (true, 8) => Ok(Self::Gray8),
            (false, 8) => Ok(Self::Yuv422p8),
            (true, 10) => Ok(Self::Gray10),
            (false, 10) => Ok(Self::Yuv422p10),
            (true, 12) => Ok(Self::Gray12),
            (false, 12) => Ok(Self::Yuv422p12),
            (true, 16) => Ok(Self::Gray16),
            (false, 16) => Ok(Self::Yuv422p16),
            _ => Err(CodecError::InvalidInput(
                "bit depth must be 8, 10, 12, or 16",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub const fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    pub fn validate(self) -> Result<(), CodecError> {
        if self.numerator == 0 || self.denominator == 0 {
            return Err(CodecError::InvalidInput(
                "frame-rate numerator and denominator must be nonzero",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plane {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Plane {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, CodecError> {
        let expected = checked_area(width, height)?;
        if data.len() != expected {
            return Err(CodecError::InvalidInput(
                "plane data length does not match dimensions",
            ));
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub frame_rate: FrameRate,
    pub planes: Vec<Plane>,
}

impl Frame {
    pub fn gray8(
        width: u32,
        height: u32,
        frame_rate: FrameRate,
        data: Vec<u8>,
    ) -> Result<Self, CodecError> {
        let frame = Self {
            width,
            height,
            format: PixelFormat::Gray8,
            frame_rate,
            planes: vec![Plane::new(width, height, data)?],
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn yuv422p8(
        width: u32,
        height: u32,
        frame_rate: FrameRate,
        y: Vec<u8>,
        cb: Vec<u8>,
        cr: Vec<u8>,
    ) -> Result<Self, CodecError> {
        let chroma_width = width.div_ceil(2);
        let frame = Self {
            width,
            height,
            format: PixelFormat::Yuv422p8,
            frame_rate,
            planes: vec![
                Plane::new(width, height, y)?,
                Plane::new(chroma_width, height, cb)?,
                Plane::new(chroma_width, height, cr)?,
            ],
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn raw_len(&self) -> usize {
        self.planes.iter().map(|plane| plane.data.len()).sum()
    }

    pub fn validate(&self) -> Result<(), CodecError> {
        self.frame_rate.validate()?;
        if self.width == 0 || self.height == 0 {
            return Err(CodecError::InvalidInput("frame dimensions must be nonzero"));
        }
        let expected = match self.format {
            PixelFormat::Gray8 => vec![(self.width, self.height)],
            PixelFormat::Yuv422p8 => vec![
                (self.width, self.height),
                (self.width.div_ceil(2), self.height),
                (self.width.div_ceil(2), self.height),
            ],
            _ => {
                return Err(CodecError::InvalidInput(
                    "8-bit Frame requires an 8-bit pixel format",
                ));
            }
        };
        if self.planes.len() != expected.len() {
            return Err(CodecError::InvalidInput(
                "plane count does not match pixel format",
            ));
        }
        let mut total = 0usize;
        for (plane, &(width, height)) in self.planes.iter().zip(&expected) {
            if plane.width != width || plane.height != height {
                return Err(CodecError::InvalidInput(
                    "plane dimensions do not match pixel format",
                ));
            }
            let area = checked_area(width, height)?;
            if plane.data.len() != area {
                return Err(CodecError::InvalidInput(
                    "plane data length does not match dimensions",
                ));
            }
            total = total
                .checked_add(area)
                .ok_or(CodecError::LimitExceeded("frame is too large"))?;
        }
        if total > MAX_FRAME_BYTES {
            return Err(CodecError::LimitExceeded("frame is too large"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plane16 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u16>,
}

impl Plane16 {
    pub fn new(width: u32, height: u32, bit_depth: u8, data: Vec<u16>) -> Result<Self, CodecError> {
        let expected = checked_area(width, height)?;
        if data.len() != expected {
            return Err(CodecError::InvalidInput(
                "plane data length does not match dimensions",
            ));
        }
        let max_sample = sample_max(bit_depth)?;
        if data.iter().any(|&sample| sample > max_sample) {
            return Err(CodecError::InvalidInput(
                "plane sample exceeds signaled bit depth",
            ));
        }
        expected
            .checked_mul(size_of::<u16>())
            .filter(|&bytes| bytes <= MAX_FRAME_BYTES)
            .ok_or(CodecError::LimitExceeded("plane is too large"))?;
        Ok(Self {
            width,
            height,
            data,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame16 {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub frame_rate: FrameRate,
    pub planes: Vec<Plane16>,
}

impl Frame16 {
    pub fn gray(
        width: u32,
        height: u32,
        bit_depth: u8,
        frame_rate: FrameRate,
        data: Vec<u16>,
    ) -> Result<Self, CodecError> {
        let format = PixelFormat::from_layout_and_depth(true, bit_depth)?;
        if !format.is_high_bit_depth() {
            return Err(CodecError::InvalidInput(
                "Frame16 bit depth must be 10, 12, or 16",
            ));
        }
        let frame = Self {
            width,
            height,
            format,
            frame_rate,
            planes: vec![Plane16::new(width, height, bit_depth, data)?],
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn yuv422(
        width: u32,
        height: u32,
        bit_depth: u8,
        frame_rate: FrameRate,
        y: Vec<u16>,
        cb: Vec<u16>,
        cr: Vec<u16>,
    ) -> Result<Self, CodecError> {
        let format = PixelFormat::from_layout_and_depth(false, bit_depth)?;
        if !format.is_high_bit_depth() {
            return Err(CodecError::InvalidInput(
                "Frame16 bit depth must be 10, 12, or 16",
            ));
        }
        let chroma_width = width.div_ceil(2);
        let frame = Self {
            width,
            height,
            format,
            frame_rate,
            planes: vec![
                Plane16::new(width, height, bit_depth, y)?,
                Plane16::new(chroma_width, height, bit_depth, cb)?,
                Plane16::new(chroma_width, height, bit_depth, cr)?,
            ],
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn bit_depth(&self) -> u8 {
        self.format.bit_depth()
    }

    pub fn raw_len(&self) -> usize {
        self.planes
            .iter()
            .map(|plane| plane.data.len() * size_of::<u16>())
            .sum()
    }

    pub fn validate(&self) -> Result<(), CodecError> {
        self.frame_rate.validate()?;
        if self.width == 0 || self.height == 0 {
            return Err(CodecError::InvalidInput("frame dimensions must be nonzero"));
        }
        if !self.format.is_high_bit_depth() {
            return Err(CodecError::InvalidInput(
                "Frame16 requires a 10-, 12-, or 16-bit pixel format",
            ));
        }
        let expected = if self.format.is_grayscale() {
            vec![(self.width, self.height)]
        } else {
            vec![
                (self.width, self.height),
                (self.width.div_ceil(2), self.height),
                (self.width.div_ceil(2), self.height),
            ]
        };
        if self.planes.len() != expected.len() {
            return Err(CodecError::InvalidInput(
                "plane count does not match pixel format",
            ));
        }
        let max_sample = sample_max(self.bit_depth())?;
        let mut total_bytes = 0usize;
        for (plane, &(width, height)) in self.planes.iter().zip(&expected) {
            let area = checked_area(width, height)?;
            if plane.width != width || plane.height != height || plane.data.len() != area {
                return Err(CodecError::InvalidInput(
                    "plane dimensions do not match pixel format",
                ));
            }
            if plane.data.iter().any(|&sample| sample > max_sample) {
                return Err(CodecError::InvalidInput(
                    "plane sample exceeds signaled bit depth",
                ));
            }
            total_bytes = total_bytes
                .checked_add(
                    area.checked_mul(size_of::<u16>())
                        .ok_or(CodecError::LimitExceeded("frame is too large"))?,
                )
                .ok_or(CodecError::LimitExceeded("frame is too large"))?;
        }
        if total_bytes > MAX_FRAME_BYTES {
            return Err(CodecError::LimitExceeded("frame is too large"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodecOptions {
    pub quality: u8,
    pub tile_width: u16,
    pub tile_height: u16,
    pub threads: usize,
}

impl Default for CodecOptions {
    fn default() -> Self {
        Self {
            quality: 90,
            tile_width: 256,
            tile_height: 128,
            threads: 1,
        }
    }
}

impl CodecOptions {
    pub fn validate(self) -> Result<(), CodecError> {
        if !(1..=100).contains(&self.quality) {
            return Err(CodecError::InvalidInput("quality must be in 1..=100"));
        }
        if self.tile_width == 0 || self.tile_height == 0 {
            return Err(CodecError::InvalidInput("tile dimensions must be nonzero"));
        }
        if self.threads == 0 {
            return Err(CodecError::InvalidInput("thread count must be nonzero"));
        }
        Ok(())
    }

    pub(crate) fn quantization_step(self) -> i32 {
        1 + i32::from((100 - self.quality) / 5)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodecError {
    InvalidInput(&'static str),
    Malformed(&'static str),
    LimitExceeded(&'static str),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
            Self::Malformed(message) => write!(f, "malformed stream: {message}"),
            Self::LimitExceeded(message) => write!(f, "resource limit exceeded: {message}"),
        }
    }
}

impl Error for CodecError {}

pub(crate) fn checked_area(width: u32, height: u32) -> Result<usize, CodecError> {
    let area = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(CodecError::LimitExceeded("plane is too large"))?;
    if area > MAX_FRAME_BYTES {
        return Err(CodecError::LimitExceeded("plane is too large"));
    }
    Ok(area)
}

pub(crate) fn sample_max(bit_depth: u8) -> Result<u16, CodecError> {
    match bit_depth {
        8 | 10 | 12 => Ok((1u16 << bit_depth) - 1),
        16 => Ok(u16::MAX),
        _ => Err(CodecError::InvalidInput(
            "bit depth must be 8, 10, 12, or 16",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_bit_frames_validate_sample_range_and_raw_size() {
        let frame = Frame16::yuv422(
            3,
            2,
            10,
            FrameRate::new(24, 1),
            vec![0, 1, 2, 3, 1022, 1023],
            vec![512; 4],
            vec![1023; 4],
        )
        .unwrap();
        assert_eq!(frame.format, PixelFormat::Yuv422p10);
        assert_eq!(frame.raw_len(), 28);

        assert!(Frame16::gray(1, 1, 12, FrameRate::new(24, 1), vec![4096]).is_err());
        assert!(Frame16::gray(1, 1, 8, FrameRate::new(24, 1), vec![255]).is_err());
    }

    #[test]
    fn every_supported_layout_depth_has_a_distinct_format() {
        for bit_depth in [8, 10, 12, 16] {
            for grayscale in [false, true] {
                let format = PixelFormat::from_layout_and_depth(grayscale, bit_depth).unwrap();
                assert_eq!(format.bit_depth(), bit_depth);
                assert_eq!(format.is_grayscale(), grayscale);
                assert_eq!(PixelFormat::try_from(format as u8).unwrap(), format);
            }
        }
    }
}
