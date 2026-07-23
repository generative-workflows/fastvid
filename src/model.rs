use std::error::Error;
use std::fmt;

pub const MAX_FRAME_BYTES: usize = 1 << 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PixelFormat {
    Gray8 = 0,
    Yuv422p8 = 1,
}

impl TryFrom<u8> for PixelFormat {
    type Error = CodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Gray8),
            1 => Ok(Self::Yuv422p8),
            _ => Err(CodecError::Malformed("unknown pixel format")),
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
