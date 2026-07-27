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

/// Read-only size model for one encoded tile's folded residual symbols.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TileEntropyModel {
    pub plane: usize,
    pub width: u32,
    pub height: u32,
    pub temporal_prediction: bool,
    pub source_zero_run: bool,
    pub sample_count: usize,
    pub zero_symbols: usize,
    pub actual_payload_bytes: usize,
    pub stream_vbyte_bytes: u64,
    pub stream_vbyte_0124_bytes: u64,
    pub distinct_symbols: usize,
    pub ideal_order0_bytes: u64,
    pub order0_supported: bool,
    pub order0_table_log: u8,
    pub order0_payload_bytes: u64,
    pub order0_table_bytes: u64,
    pub order0_complete_bytes: u64,
    pub context_order0_supported: bool,
    pub context_order0_contexts: u8,
    pub context_order0_threshold: u32,
    pub context_order0_payload_bytes: u64,
    pub context_order0_table_bytes: u64,
    pub context_order0_control_bytes: u64,
    pub context_order0_complete_bytes: u64,
    pub rice4_shard_supported: bool,
    pub rice4_shard_payload_bytes: u64,
    pub rice4_shard_control_bytes: u64,
    pub rice4_shard_complete_bytes: u64,
    pub diagonal_order_supported: bool,
    pub raster_best_bytes: u64,
    pub raster_rice_bytes: u64,
    pub diagonal_zero_run_bytes: u64,
    pub diagonal_rice_bytes: u64,
    pub diagonal_block_bytes: u64,
    pub diagonal_best_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RiceLaneSizeModel {
    pub(crate) payload_bytes: u64,
    pub(crate) control_bytes: u64,
    pub(crate) complete_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct EntropyOrderSizeModel {
    pub(crate) raster_best_bytes: u64,
    pub(crate) raster_rice_bytes: u64,
    pub(crate) diagonal_zero_run_bytes: u64,
    pub(crate) diagonal_rice_bytes: u64,
    pub(crate) diagonal_block_bytes: u64,
    pub(crate) diagonal_best_bytes: u64,
}

/// Read-only size model for predictor-bounded residual symbols in one tile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TileResidualMappingModel {
    pub plane: usize,
    pub width: u32,
    pub height: u32,
    pub temporal_prediction: bool,
    pub source_zero_run: bool,
    pub bounded_zero_run: bool,
    pub sample_count: usize,
    pub actual_payload_bytes: usize,
    pub bounded_payload_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredictorModelMode {
    Paeth,
    Average,
    ClampGradient,
    HalfGradient,
    Temporal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredictorCandidateModel {
    pub payload_bytes: usize,
    pub squared_error: u64,
    pub max_error: u32,
    pub zero_run: bool,
}

/// Read-only exact-byte model for one tile's compatible predictor candidates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TilePredictorModel {
    pub plane: usize,
    pub width: u32,
    pub height: u32,
    pub sample_count: usize,
    pub current_mode: PredictorModelMode,
    pub oracle_mode: PredictorModelMode,
    pub current: PredictorCandidateModel,
    pub oracle: PredictorCandidateModel,
    pub paeth: PredictorCandidateModel,
    pub average: PredictorCandidateModel,
    pub clamp_gradient: PredictorCandidateModel,
    pub half_gradient: PredictorCandidateModel,
    pub temporal: Option<PredictorCandidateModel>,
    pub band16_clamp: PredictorBandModel,
    pub band32_clamp: PredictorBandModel,
    pub band64_clamp: PredictorBandModel,
}

/// Read-only complete-byte model for independently reconstructed row bands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredictorBandModel {
    pub bands: usize,
    pub max_band_samples: usize,
    pub payload_bytes: usize,
    pub control_bytes: usize,
    pub complete_bytes: usize,
    pub squared_error: u64,
    pub max_error: u32,
    pub parallel_payload_bytes: usize,
    pub parallel_control_bytes: usize,
    pub parallel_complete_bytes: usize,
    pub max_entropy_span: usize,
}

// research/0023: once a causal prediction is known, only residuals in
// [lower, upper] can occur. Alternate signs while both remain possible, then
// encode the sole remaining side contiguously.
pub(crate) fn fold_bounded_residual(value: i32, lower: i32, upper: i32) -> u32 {
    debug_assert!(lower <= value && value <= upper);
    debug_assert!(lower <= 0 && upper >= 0);
    let paired_magnitude = (-lower).min(upper);
    if value < -paired_magnitude || value > paired_magnitude {
        (paired_magnitude + value.abs()) as u32
    } else if value >= 0 {
        value as u32 * 2
    } else {
        value.unsigned_abs() * 2 - 1
    }
}

#[cfg(test)]
pub(crate) fn unfold_bounded_residual(folded: u32, lower: i32, upper: i32) -> Option<i32> {
    if lower > 0 || upper < 0 || folded > (upper - lower) as u32 {
        return None;
    }
    let paired_magnitude = (-lower).min(upper);
    let paired_codes = paired_magnitude as u32 * 2;
    let value = if folded <= paired_codes {
        if folded & 1 == 0 {
            (folded / 2) as i32
        } else {
            -((folded / 2) as i32) - 1
        }
    } else if -lower < upper {
        folded as i32 - paired_magnitude
    } else if upper < -lower {
        -(folded as i32 - paired_magnitude)
    } else {
        return None;
    };
    (lower..=upper).contains(&value).then_some(value)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ByteFormatModel {
    symbols: u64,
    zero_symbols: u64,
    standard_data_bytes: u64,
    zero_aware_data_bytes: u64,
    histogram: Vec<u64>,
    values: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Order0SizeModel {
    pub(crate) distinct_symbols: usize,
    pub(crate) ideal_bytes: u64,
    pub(crate) supported: bool,
    pub(crate) table_log: u8,
    pub(crate) payload_bytes: u64,
    pub(crate) table_bytes: u64,
    pub(crate) complete_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ContextOrder0SizeModel {
    pub(crate) supported: bool,
    pub(crate) contexts: u8,
    pub(crate) threshold: u32,
    pub(crate) payload_bytes: u64,
    pub(crate) table_bytes: u64,
    pub(crate) control_bytes: u64,
    pub(crate) complete_bytes: u64,
}

impl ByteFormatModel {
    pub(crate) fn push(&mut self, value: u32) {
        let symbol = value as usize;
        if self.histogram.len() <= symbol {
            self.histogram.resize(symbol + 1, 0);
        }
        self.histogram[symbol] += 1;
        self.values.push(value);
        self.symbols += 1;
        self.zero_symbols += u64::from(value == 0);
        self.standard_data_bytes += match value {
            0..=0xff => 1,
            0x100..=0xffff => 2,
            0x1_0000..=0xff_ffff => 3,
            _ => 4,
        };
        self.zero_aware_data_bytes += match value {
            0 => 0,
            1..=0xff => 1,
            0x100..=0xffff => 2,
            _ => 4,
        };
    }

    pub(crate) fn push_zeros(&mut self, count: usize) {
        let count = count as u64;
        if self.histogram.is_empty() {
            self.histogram.push(0);
        }
        self.histogram[0] += count;
        self.values.resize(self.values.len() + count as usize, 0);
        self.symbols += count;
        self.zero_symbols += count;
        self.standard_data_bytes += count;
    }

    pub(crate) fn sample_count(&self) -> Option<usize> {
        usize::try_from(self.symbols).ok()
    }

    pub(crate) fn zero_symbols(&self) -> Option<usize> {
        usize::try_from(self.zero_symbols).ok()
    }

    pub(crate) fn stream_vbyte_bytes(&self) -> u64 {
        self.symbols.div_ceil(4) + self.standard_data_bytes
    }

    pub(crate) fn stream_vbyte_0124_bytes(&self) -> u64 {
        self.symbols.div_ceil(4) + self.zero_aware_data_bytes
    }

    pub(crate) fn order0_size(&self) -> Order0SizeModel {
        let observed: Vec<(u32, u64)> = self
            .histogram
            .iter()
            .enumerate()
            .filter(|(_, count)| **count != 0)
            .map(|(symbol, &count)| (symbol as u32, count))
            .collect();
        if observed.is_empty() {
            return Order0SizeModel::default();
        }
        let ideal_bits = observed.iter().fold(0.0, |bits, (_, count)| {
            bits + *count as f64 * (self.symbols as f64 / *count as f64).log2()
        });
        let ideal_bytes = bits_to_bytes(ideal_bits);
        let mut best = None;
        for table_log in 8..=12 {
            let table_size = 1u64 << table_log;
            if observed.len() as u64 > table_size {
                continue;
            }
            let frequencies = normalize_frequencies(&observed, table_size, self.symbols);
            let payload_bits =
                observed
                    .iter()
                    .zip(&frequencies)
                    .fold(0.0, |bits, ((_, count), frequency)| {
                        bits + *count as f64 * (table_size as f64 / *frequency as f64).log2()
                    });
            let payload_bytes = bits_to_bytes(payload_bits);
            let table_bytes = sparse_table_bytes(&observed, &frequencies);
            let complete_bytes = payload_bytes + table_bytes;
            if best.is_none_or(|current: Order0SizeModel| {
                (complete_bytes, table_log) < (current.complete_bytes, current.table_log)
            }) {
                best = Some(Order0SizeModel {
                    distinct_symbols: observed.len(),
                    ideal_bytes,
                    supported: true,
                    table_log,
                    payload_bytes,
                    table_bytes,
                    complete_bytes,
                });
            }
        }
        best.unwrap_or(Order0SizeModel {
            distinct_symbols: observed.len(),
            ideal_bytes,
            ..Order0SizeModel::default()
        })
    }

    pub(crate) fn context_order0_size(&self) -> ContextOrder0SizeModel {
        if self.values.is_empty() {
            return ContextOrder0SizeModel::default();
        }
        let mut best = self.context_order0_candidate(2, 0);
        for threshold in [1, 3, 7, 15] {
            let candidate = self.context_order0_candidate(3, threshold);
            if candidate.supported
                && (!best.supported
                    || (
                        candidate.complete_bytes,
                        candidate.contexts,
                        candidate.threshold,
                    ) < (best.complete_bytes, best.contexts, best.threshold))
            {
                best = candidate;
            }
        }
        best
    }

    pub(crate) fn rice_lane_size(
        &self,
        parameter: u8,
        shard_symbols: usize,
        lanes: usize,
    ) -> RiceLaneSizeModel {
        debug_assert!(shard_symbols != 0);
        debug_assert!(lanes != 0);
        let shard_count = self.values.len().div_ceil(shard_symbols);
        let mut payload_bytes = 0u64;
        let mut control_bytes = 0u64;
        for (shard_index, shard) in self.values.chunks(shard_symbols).enumerate() {
            let active_lanes = lanes.min(shard.len());
            let mut lane_bits = vec![0u64; active_lanes];
            for (index, &value) in shard.iter().enumerate() {
                lane_bits[index % active_lanes] +=
                    u64::from(value >> parameter) + 1 + u64::from(parameter);
            }
            payload_bytes += lane_bits
                .into_iter()
                .map(|bits| bits.div_ceil(8))
                .sum::<u64>();
            control_bytes += (active_lanes.saturating_sub(1) as u64) * 4;
            if shard_index + 1 != shard_count {
                control_bytes += 4;
            }
        }
        RiceLaneSizeModel {
            payload_bytes,
            control_bytes,
            complete_bytes: payload_bytes + control_bytes,
        }
    }

    pub(crate) fn diagonal_order_size(
        &self,
        width: usize,
        height: usize,
        allow_block_pack: bool,
    ) -> EntropyOrderSizeModel {
        // research/0038: execution order need not become entropy storage order;
        // this rejected-format model quantifies that distinction.
        debug_assert_eq!(self.values.len(), width * height);
        let mut diagonal = Vec::with_capacity(self.values.len());
        for sum in 0..width + height - 1 {
            let first_x = sum.saturating_sub(height - 1);
            let last_x = sum.min(width - 1);
            for x in first_x..=last_x {
                let y = sum - x;
                diagonal.push(self.values[y * width + x]);
            }
        }
        let raster_zero = modeled_zero_run_bytes(&self.values);
        let raster_rice = modeled_best_rice_bytes(&self.values);
        let diagonal_zero_run_bytes = modeled_zero_run_bytes(&diagonal);
        let diagonal_rice_bytes = modeled_best_rice_bytes(&diagonal);
        let raster_block = allow_block_pack.then(|| modeled_block_pack_bytes(&self.values));
        let diagonal_block = allow_block_pack.then(|| modeled_block_pack_bytes(&diagonal));
        EntropyOrderSizeModel {
            raster_best_bytes: raster_zero
                .min(raster_rice)
                .min(raster_block.unwrap_or(u64::MAX)),
            raster_rice_bytes: raster_rice,
            diagonal_zero_run_bytes,
            diagonal_rice_bytes,
            diagonal_block_bytes: diagonal_block.unwrap_or(0),
            diagonal_best_bytes: diagonal_zero_run_bytes
                .min(diagonal_rice_bytes)
                .min(diagonal_block.unwrap_or(u64::MAX)),
        }
    }

    fn context_order0_candidate(
        &self,
        context_count: u8,
        threshold: u32,
    ) -> ContextOrder0SizeModel {
        let mut contexts: Vec<ByteFormatModel> = (0..context_count)
            .map(|_| ByteFormatModel::default())
            .collect();
        let mut previous = 0u32;
        for &value in &self.values {
            let context = if previous == 0 {
                0
            } else if context_count == 2 || previous <= threshold {
                1
            } else {
                2
            };
            contexts[context].push(value);
            previous = value;
        }
        let mut payload_bytes = 0u64;
        let mut table_bytes = 0u64;
        let mut control_bytes = 1 + u64::from(context_count == 3);
        for context in contexts {
            let model = context.order0_size();
            if context.symbols != 0 && !model.supported {
                return ContextOrder0SizeModel::default();
            }
            payload_bytes += model.payload_bytes;
            table_bytes += model.table_bytes;
            control_bytes += model_varint_length(model.complete_bytes as u32);
        }
        ContextOrder0SizeModel {
            supported: true,
            contexts: context_count,
            threshold,
            payload_bytes,
            table_bytes,
            control_bytes,
            complete_bytes: payload_bytes + table_bytes + control_bytes,
        }
    }
}

fn modeled_zero_run_bytes(values: &[u32]) -> u64 {
    let mut bytes = 0u64;
    let mut run = 0u32;
    for &value in values {
        if value == 0 {
            run += 1;
        } else {
            if run != 0 {
                bytes += model_varint_length((run - 1) * 2);
                run = 0;
            }
            bytes += model_varint_length(value * 2 - 1);
        }
    }
    if run != 0 {
        bytes += model_varint_length((run - 1) * 2);
    }
    bytes
}

fn modeled_best_rice_bytes(values: &[u32]) -> u64 {
    (0u8..=16)
        .map(|parameter| {
            values
                .iter()
                .map(|&value| u64::from(value >> parameter) + 1 + u64::from(parameter))
                .sum::<u64>()
                .div_ceil(8)
        })
        .min()
        .unwrap_or(0)
}

fn modeled_block_pack_bytes(values: &[u32]) -> u64 {
    values
        .chunks(128)
        .map(|block| {
            let maximum = block.iter().copied().max().unwrap_or(0);
            let width = u64::from(u32::BITS - maximum.leading_zeros());
            1 + (block.len() as u64 * width).div_ceil(8)
        })
        .sum()
}

fn bits_to_bytes(bits: f64) -> u64 {
    (bits / 8.0).ceil() as u64
}

fn normalize_frequencies(observed: &[(u32, u64)], table_size: u64, symbols: u64) -> Vec<u64> {
    debug_assert!(observed.len() as u64 <= table_size);
    debug_assert_eq!(
        observed.iter().map(|(_, count)| count).sum::<u64>(),
        symbols
    );
    let remaining = table_size - observed.len() as u64;
    let mut frequencies = Vec::with_capacity(observed.len());
    let mut remainders = Vec::with_capacity(observed.len());
    let mut assigned = 0u64;
    for &(symbol, count) in observed {
        let scaled = u128::from(count) * u128::from(remaining);
        let frequency = 1 + (scaled / u128::from(symbols)) as u64;
        frequencies.push(frequency);
        assigned += frequency;
        remainders.push((scaled % u128::from(symbols), symbol));
    }
    remainders
        .sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    for &(_, symbol) in remainders.iter().take((table_size - assigned) as usize) {
        let index = observed
            .binary_search_by_key(&symbol, |(candidate, _)| *candidate)
            .expect("remainder symbol came from observed symbols");
        frequencies[index] += 1;
    }
    debug_assert_eq!(frequencies.iter().sum::<u64>(), table_size);
    frequencies
}

fn sparse_table_bytes(observed: &[(u32, u64)], frequencies: &[u64]) -> u64 {
    let mut bytes = 1 + model_varint_length(observed.len() as u32) + 4;
    let mut previous = 0;
    for (index, &(symbol, _)) in observed.iter().enumerate() {
        bytes += model_varint_length(symbol - previous);
        if index + 1 != observed.len() {
            bytes += model_varint_length(frequencies[index] as u32);
        }
        previous = symbol;
    }
    bytes
}

fn model_varint_length(value: u32) -> u64 {
    match value {
        0..=0x7f => 1,
        0x80..=0x3fff => 2,
        0x4000..=0x1f_ffff => 3,
        0x20_0000..=0x0fff_ffff => 4,
        _ => 5,
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

    #[test]
    fn byte_format_model_charges_controls_and_width_boundaries() {
        let mut model = ByteFormatModel::default();
        for value in [0, 1, 0xff, 0x100, 0xffff, 0x1_0000, 0xff_ffff, 0x1_000000] {
            model.push(value);
        }
        assert_eq!(model.sample_count(), Some(8));
        assert_eq!(model.zero_symbols(), Some(1));
        // Two control bytes plus 1+1+1+2+2+3+3+4 data bytes.
        assert_eq!(model.stream_vbyte_bytes(), 19);
        // Two controls plus 0+1+1+2+2+4+4+4 data bytes.
        assert_eq!(model.stream_vbyte_0124_bytes(), 20);

        let mut partial = ByteFormatModel::default();
        partial.push_zeros(5);
        assert_eq!(partial.stream_vbyte_bytes(), 7);
        assert_eq!(partial.stream_vbyte_0124_bytes(), 2);
    }

    #[test]
    fn four_lane_rice_model_charges_alignment_and_lengths() {
        let mut model = ByteFormatModel::default();
        for value in 0..=4 {
            model.push(value);
        }
        let size = model.rice_lane_size(1, 4, 2);
        assert_eq!(size.payload_bytes, 3);
        assert_eq!(size.control_bytes, 8);
        assert_eq!(size.complete_bytes, 11);
    }

    #[test]
    fn diagonal_order_model_reorders_symbols_and_preserves_rice_bits() {
        let mut model = ByteFormatModel::default();
        for index in 0..256 {
            model.push(u32::from(index == 128));
        }
        let size = model.diagonal_order_size(128, 2, false);
        assert_eq!(size.raster_best_bytes, 5);
        assert_eq!(size.raster_rice_bytes, 33);
        assert_eq!(size.diagonal_zero_run_bytes, 4);
        assert_eq!(size.diagonal_rice_bytes, 33);
        assert_eq!(size.diagonal_block_bytes, 0);
        assert_eq!(size.diagonal_best_bytes, 4);
    }

    #[test]
    fn finite_block_order0_model_charges_payload_table_and_state() {
        let empty = ByteFormatModel::default().order0_size();
        assert_eq!(empty, Order0SizeModel::default());

        let mut singleton = ByteFormatModel::default();
        singleton.push_zeros(32_768);
        let singleton = singleton.order0_size();
        assert_eq!(singleton.distinct_symbols, 1);
        assert_eq!(singleton.ideal_bytes, 0);
        assert!(singleton.supported);
        assert_eq!(singleton.table_log, 8);
        assert_eq!(singleton.payload_bytes, 0);
        // log, symbol count, symbol delta, and four-byte final state.
        assert_eq!(singleton.table_bytes, 7);
        assert_eq!(singleton.complete_bytes, 7);

        let mut uniform = ByteFormatModel::default();
        for symbol in 0..256 {
            uniform.push(symbol);
        }
        let uniform = uniform.order0_size();
        assert_eq!(uniform.distinct_symbols, 256);
        assert_eq!(uniform.ideal_bytes, 256);
        assert_eq!(uniform.table_log, 8);
        assert_eq!(uniform.payload_bytes, 256);
        assert_eq!(uniform.table_bytes, 518);
        assert_eq!(uniform.complete_bytes, 774);
    }

    #[test]
    fn finite_block_normalization_is_positive_exact_and_deterministic() {
        let observed = [(0, 900), (1, 90), (127, 9), (16_384, 1)];
        for table_log in 8..=12 {
            let frequencies = normalize_frequencies(&observed, 1 << table_log, 1_000);
            assert!(frequencies.iter().all(|&frequency| frequency > 0));
            assert_eq!(frequencies.iter().sum::<u64>(), 1 << table_log);
            assert_eq!(
                frequencies,
                normalize_frequencies(&observed, 1 << table_log, 1_000)
            );
        }
        assert_eq!(model_varint_length(0x7f), 1);
        assert_eq!(model_varint_length(0x80), 2);
        assert_eq!(model_varint_length(0x3fff), 2);
        assert_eq!(model_varint_length(0x4000), 3);
    }

    #[test]
    fn finite_block_model_rejects_alphabets_larger_than_its_tables() {
        let mut model = ByteFormatModel::default();
        for symbol in 0..4097 {
            model.push(symbol);
        }
        let order0 = model.order0_size();
        assert_eq!(order0.distinct_symbols, 4097);
        assert!(!order0.supported);
        assert!(order0.ideal_bytes > 0);
        assert_eq!(order0.complete_bytes, 0);
    }

    #[test]
    fn causal_context_model_charges_every_table_state_and_length() {
        let mut singleton = ByteFormatModel::default();
        singleton.push_zeros(32_768);
        let context = singleton.context_order0_size();
        assert!(context.supported);
        assert_eq!(context.contexts, 2);
        assert_eq!(context.threshold, 0);
        assert_eq!(context.payload_bytes, 0);
        assert_eq!(context.table_bytes, 7);
        // Mode byte and a one-byte length for both context substreams.
        assert_eq!(context.control_bytes, 3);
        assert_eq!(context.complete_bytes, 10);

        let mut mixed = ByteFormatModel::default();
        for value in [0, 1, 0, 9, 2, 0, 31, 1, 0] {
            mixed.push(value);
        }
        let first = mixed.context_order0_size();
        assert!(first.supported);
        assert!((2..=3).contains(&first.contexts));
        assert_eq!(first, mixed.context_order0_size());
        assert_eq!(
            first.complete_bytes,
            first.payload_bytes + first.table_bytes + first.control_bytes
        );
    }

    #[test]
    fn bounded_residual_mapping_is_bijective_for_every_8bit_encoder_interval() {
        fn quantize(value: i32, step: i32) -> i32 {
            let magnitude = (value.abs() + step / 2) / step;
            if value < 0 { -magnitude } else { magnitude }
        }

        for step in 1..=21 {
            for prediction in 0..=255 {
                let lower = quantize(-prediction, step);
                let upper = quantize(255 - prediction, step);
                let mut seen = vec![false; (upper - lower + 1) as usize];
                for value in lower..=upper {
                    let folded = fold_bounded_residual(value, lower, upper);
                    assert!(folded <= (upper - lower) as u32);
                    assert_eq!(unfold_bounded_residual(folded, lower, upper), Some(value));
                    assert!(!std::mem::replace(&mut seen[folded as usize], true));
                }
                assert!(seen.into_iter().all(|value| value));
                assert_eq!(fold_bounded_residual(0, lower, upper), 0);
            }
        }
    }

    #[test]
    fn bounded_residual_mapping_covers_high_bit_endpoints() {
        fn quantize(value: i32, step: i32) -> i32 {
            let magnitude = (value.abs() + step / 2) / step;
            if value < 0 { -magnitude } else { magnitude }
        }

        for bit_depth in [10, 12, 16] {
            let maximum = i32::from(sample_max(bit_depth).unwrap());
            for step in [1, 1 + (2 << (bit_depth - 8))] {
                for prediction in [0, 1, maximum / 2, maximum - 1, maximum] {
                    let lower = quantize(-prediction, step);
                    let upper = quantize(maximum - prediction, step);
                    for value in lower..=upper {
                        let folded = fold_bounded_residual(value, lower, upper);
                        assert!(folded <= (upper - lower) as u32);
                        assert_eq!(unfold_bounded_residual(folded, lower, upper), Some(value));
                    }
                }
            }
        }
    }
}
