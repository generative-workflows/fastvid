use crate::codec::StreamInfo;
use crate::model::{
    ByteFormatModel, CodecError, CodecOptions, Frame16, FrameRate, MAX_FRAME_BYTES, PixelFormat,
    Plane16, TileEntropyModel, checked_area, sample_max,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAGIC: &[u8; 4] = b"FVID";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 32;
const DIRECTORY_ENTRY_LEN: usize = 32;
const MAX_TILES: usize = 1 << 20;
const ENTROPY_ZERO_RUN: u8 = 0;
const ENTROPY_RICE_BASE: u8 = 1;
const MAX_RICE_PARAMETER: u8 = 16;
const PREDICT_SPATIAL: u8 = 0;
const PREDICT_TEMPORAL: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Tile {
    plane: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
struct DirectoryEntry {
    tile: Tile,
    entropy_mode: u8,
    prediction_mode: u8,
    offset: usize,
    length: usize,
}

struct Parsed<'a> {
    info: StreamInfo,
    entries: Vec<DirectoryEntry>,
    bytes: &'a [u8],
}

struct EncodedTile {
    entropy_mode: u8,
    prediction_mode: u8,
    payload: Vec<u8>,
}

#[derive(Clone, Copy)]
struct PredictionContext<'a> {
    mode: u8,
    reference: Option<&'a Plane16>,
    tile: Tile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedTile16 {
    pub plane: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u16>,
}

pub fn encode16(frame: &Frame16, options: CodecOptions) -> Result<Vec<u8>, CodecError> {
    encode_internal(frame, None, options)
}

pub fn encode16_with_reference(
    frame: &Frame16,
    reference: &Frame16,
    options: CodecOptions,
) -> Result<Vec<u8>, CodecError> {
    validate_reference(frame, reference)?;
    let threshold = 5u64 << (frame.bit_depth() - 8);
    let current = &frame.planes[0].data;
    let previous = &reference.planes[0].data;
    let absolute_difference: u64 = current
        .iter()
        .zip(previous)
        .map(|(&current, &previous)| u64::from(current.abs_diff(previous)))
        .sum();
    let selected = (absolute_difference <= current.len() as u64 * threshold).then_some(reference);
    encode_internal(frame, selected, options)
}

fn encode_internal(
    frame: &Frame16,
    reference: Option<&Frame16>,
    options: CodecOptions,
) -> Result<Vec<u8>, CodecError> {
    frame.validate()?;
    options.validate()?;
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let step = quantization_step(options.quality, frame.bit_depth());
    let quantizer = Quantizer16::new(step, frame.bit_depth());
    let payloads = parallel_map(tiles.len(), options.threads, |index| {
        let tile = tiles[index];
        encode_tile(
            &frame.planes[tile.plane],
            reference.map(|frame| &frame.planes[tile.plane]),
            tile,
            &quantizer,
        )
    });
    let directory_bytes = tiles
        .len()
        .checked_mul(DIRECTORY_ENTRY_LEN)
        .ok_or(CodecError::LimitExceeded("tile directory is too large"))?;
    let payload_start = HEADER_LEN
        .checked_add(directory_bytes)
        .ok_or(CodecError::LimitExceeded("stream is too large"))?;
    let payload_bytes = payloads.iter().try_fold(0usize, |sum, encoded| {
        sum.checked_add(encoded.payload.len())
            .ok_or(CodecError::LimitExceeded("stream is too large"))
    })?;
    let stream_len = payload_start
        .checked_add(payload_bytes)
        .ok_or(CodecError::LimitExceeded("stream is too large"))?;

    let mut output = Vec::with_capacity(stream_len);
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.push(if frame.format.is_grayscale() { 0 } else { 1 });
    output.push(options.quality);
    output.push(frame.bit_depth() - 8);
    put_u32(&mut output, frame.width);
    put_u32(&mut output, frame.height);
    put_u16(&mut output, options.tile_width);
    put_u16(&mut output, options.tile_height);
    put_u32(&mut output, frame.frame_rate.numerator);
    put_u32(&mut output, frame.frame_rate.denominator);
    put_u32(
        &mut output,
        u32::try_from(tiles.len()).map_err(|_| CodecError::LimitExceeded("too many tiles"))?,
    );
    let mut offset = payload_start;
    for (&tile, encoded) in tiles.iter().zip(&payloads) {
        output.push(
            u8::try_from(tile.plane).map_err(|_| CodecError::LimitExceeded("too many planes"))?,
        );
        output.push(encoded.entropy_mode);
        output.push(encoded.prediction_mode);
        output.push(0);
        put_u32(&mut output, tile.x);
        put_u32(&mut output, tile.y);
        put_u32(&mut output, tile.width);
        put_u32(&mut output, tile.height);
        put_u64(
            &mut output,
            u64::try_from(offset).map_err(|_| CodecError::LimitExceeded("stream is too large"))?,
        );
        put_u32(
            &mut output,
            u32::try_from(encoded.payload.len())
                .map_err(|_| CodecError::LimitExceeded("tile payload is too large"))?,
        );
        offset += encoded.payload.len();
    }
    for encoded in payloads {
        output.extend_from_slice(&encoded.payload);
    }
    debug_assert_eq!(output.len(), stream_len);
    Ok(output)
}

pub fn decode16(bytes: &[u8], threads: usize) -> Result<Frame16, CodecError> {
    decode_internal(bytes, None, threads)
}

pub fn decode16_with_reference(
    bytes: &[u8],
    reference: &Frame16,
    threads: usize,
) -> Result<Frame16, CodecError> {
    decode_internal(bytes, Some(reference), threads)
}

fn decode_internal(
    bytes: &[u8],
    reference: Option<&Frame16>,
    threads: usize,
) -> Result<Frame16, CodecError> {
    if threads == 0 {
        return Err(CodecError::InvalidInput("thread count must be nonzero"));
    }
    let parsed = parse(bytes)?;
    if let Some(reference) = reference {
        validate_reference_info(&parsed.info, reference)?;
    }
    let step = quantization_step(parsed.info.quality, parsed.info.format.bit_depth());
    let max_sample = sample_max(parsed.info.format.bit_depth())?;
    let decoded = parallel_map(parsed.entries.len(), threads, |index| {
        let entry = parsed.entries[index];
        decode_tile_payload(
            entry.tile,
            &parsed.bytes[entry.offset..entry.offset + entry.length],
            step,
            entry.entropy_mode,
            entry.prediction_mode,
            reference.map(|frame| &frame.planes[entry.tile.plane]),
            max_sample,
        )
    });
    let mut tiles = Vec::with_capacity(decoded.len());
    for tile in decoded {
        tiles.push(tile?);
    }
    assemble_frame(&parsed.info, tiles)
}

pub fn decode_tile16(bytes: &[u8], tile_index: usize) -> Result<DecodedTile16, CodecError> {
    let parsed = parse(bytes)?;
    let entry = *parsed
        .entries
        .get(tile_index)
        .ok_or(CodecError::InvalidInput("tile index is out of range"))?;
    if entry.prediction_mode == PREDICT_TEMPORAL {
        return Err(CodecError::InvalidInput(
            "predicted tile requires a reference frame",
        ));
    }
    let bit_depth = parsed.info.format.bit_depth();
    let data = decode_tile_payload(
        entry.tile,
        &parsed.bytes[entry.offset..entry.offset + entry.length],
        quantization_step(parsed.info.quality, bit_depth),
        entry.entropy_mode,
        entry.prediction_mode,
        None,
        sample_max(bit_depth)?,
    )?;
    Ok(DecodedTile16 {
        plane: entry.tile.plane,
        x: entry.tile.x,
        y: entry.tile.y,
        width: entry.tile.width,
        height: entry.tile.height,
        data,
    })
}

pub fn inspect16(bytes: &[u8]) -> Result<StreamInfo, CodecError> {
    Ok(parse(bytes)?.info)
}

/// High-bit equivalent of [`crate::codec::analyze_entropy`].
pub fn analyze_entropy16(bytes: &[u8]) -> Result<Vec<TileEntropyModel>, CodecError> {
    let parsed = parse(bytes)?;
    let max_folded = u32::from(sample_max(parsed.info.format.bit_depth())?) * 2;
    parsed
        .entries
        .iter()
        .map(|entry| {
            let payload = &parsed.bytes[entry.offset..entry.offset + entry.length];
            let sample_count = checked_high_area(entry.tile.width, entry.tile.height)?;
            let model =
                model_folded_payload(payload, sample_count, entry.entropy_mode, max_folded)?;
            Ok(TileEntropyModel {
                plane: entry.tile.plane,
                width: entry.tile.width,
                height: entry.tile.height,
                temporal_prediction: entry.prediction_mode == PREDICT_TEMPORAL,
                source_zero_run: entry.entropy_mode == ENTROPY_ZERO_RUN,
                sample_count: model
                    .sample_count()
                    .ok_or(CodecError::LimitExceeded("too many residual symbols"))?,
                zero_symbols: model
                    .zero_symbols()
                    .ok_or(CodecError::LimitExceeded("too many residual symbols"))?,
                actual_payload_bytes: entry.length,
                stream_vbyte_bytes: model.stream_vbyte_bytes(),
                stream_vbyte_0124_bytes: model.stream_vbyte_0124_bytes(),
            })
        })
        .collect()
}

fn validate_reference(frame: &Frame16, reference: &Frame16) -> Result<(), CodecError> {
    frame.validate()?;
    reference.validate()?;
    if frame.width != reference.width
        || frame.height != reference.height
        || frame.format != reference.format
    {
        return Err(CodecError::InvalidInput(
            "reference dimensions, layout, or bit depth do not match frame",
        ));
    }
    Ok(())
}

fn validate_reference_info(info: &StreamInfo, reference: &Frame16) -> Result<(), CodecError> {
    reference.validate()?;
    if info.width != reference.width
        || info.height != reference.height
        || info.format != reference.format
    {
        return Err(CodecError::InvalidInput(
            "reference dimensions, layout, or bit depth do not match stream",
        ));
    }
    Ok(())
}

fn assemble_frame(info: &StreamInfo, tiles: Vec<Vec<u16>>) -> Result<Frame16, CodecError> {
    let dimensions = plane_dimensions(info.width, info.height, info.format);
    let mut planes = Vec::with_capacity(dimensions.len());
    for &(width, height) in &dimensions {
        planes.push(Plane16::new(
            width,
            height,
            info.format.bit_depth(),
            vec![0; checked_area(width, height)?],
        )?);
    }
    let expected = expected_tiles(
        info.width,
        info.height,
        info.format,
        info.tile_width,
        info.tile_height,
    )?;
    for (tile, decoded) in expected.into_iter().zip(tiles) {
        let plane = &mut planes[tile.plane];
        let tile_width = tile.width as usize;
        let x = tile.x as usize;
        let y = tile.y as usize;
        let plane_width = plane.width as usize;
        for row in 0..tile.height as usize {
            let source = row * tile_width;
            let destination = (y + row) * plane_width + x;
            plane.data[destination..destination + tile_width]
                .copy_from_slice(&decoded[source..source + tile_width]);
        }
    }
    let frame = Frame16 {
        width: info.width,
        height: info.height,
        format: info.format,
        frame_rate: info.frame_rate,
        planes,
    };
    frame.validate()?;
    Ok(frame)
}

fn parse(bytes: &[u8]) -> Result<Parsed<'_>, CodecError> {
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::Malformed("truncated header"));
    }
    if &bytes[0..4] != MAGIC || bytes[4] != VERSION {
        return Err(CodecError::Malformed(
            "not a version-one high-bit Fastvid stream",
        ));
    }
    let grayscale = match bytes[5] {
        0 => true,
        1 => false,
        _ => return Err(CodecError::Malformed("unknown pixel layout")),
    };
    let bit_depth = bytes[7]
        .checked_add(8)
        .ok_or(CodecError::Malformed("invalid bit depth"))?;
    let format = PixelFormat::from_layout_and_depth(grayscale, bit_depth)
        .map_err(|_| CodecError::Malformed("unsupported bit depth"))?;
    if !format.is_high_bit_depth() {
        return Err(CodecError::Malformed(
            "high-bit decoder requires 10, 12, or 16 bits",
        ));
    }
    let quality = bytes[6];
    if !(1..=100).contains(&quality) {
        return Err(CodecError::Malformed("quality is out of range"));
    }
    let width = get_u32(bytes, 8)?;
    let height = get_u32(bytes, 12)?;
    let tile_width = get_u16(bytes, 16)?;
    let tile_height = get_u16(bytes, 18)?;
    let frame_rate = FrameRate::new(get_u32(bytes, 20)?, get_u32(bytes, 24)?);
    let tile_count = get_u32(bytes, 28)? as usize;
    if width == 0 || height == 0 || tile_width == 0 || tile_height == 0 {
        return Err(CodecError::Malformed("zero dimension"));
    }
    frame_rate
        .validate()
        .map_err(|_| CodecError::Malformed("zero frame-rate component"))?;
    validate_frame_dimensions(width, height, format)?;
    if tile_count > MAX_TILES {
        return Err(CodecError::LimitExceeded("too many tiles"));
    }
    let expected = expected_tiles(width, height, format, tile_width, tile_height)?;
    if tile_count != expected.len() {
        return Err(CodecError::Malformed(
            "tile count does not match dimensions",
        ));
    }
    let directory_end = HEADER_LEN
        .checked_add(
            tile_count
                .checked_mul(DIRECTORY_ENTRY_LEN)
                .ok_or(CodecError::LimitExceeded("tile directory is too large"))?,
        )
        .ok_or(CodecError::LimitExceeded("tile directory is too large"))?;
    if directory_end > bytes.len() {
        return Err(CodecError::Malformed("truncated tile directory"));
    }
    let mut entries = Vec::with_capacity(tile_count);
    let mut next_payload_offset = directory_end;
    let mut zero_run_tiles = 0;
    let mut rice_tiles = 0;
    let mut spatial_tiles = 0;
    let mut temporal_tiles = 0;
    for (index, expected_tile) in expected.into_iter().enumerate() {
        let start = HEADER_LEN + index * DIRECTORY_ENTRY_LEN;
        if bytes[start + 3] != 0 {
            return Err(CodecError::Malformed("nonzero reserved directory byte"));
        }
        let entropy_mode = bytes[start + 1];
        if entropy_mode != ENTROPY_ZERO_RUN
            && !(ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER).contains(&entropy_mode)
        {
            return Err(CodecError::Malformed("unknown tile entropy mode"));
        }
        let prediction_mode = bytes[start + 2];
        if prediction_mode > PREDICT_TEMPORAL {
            return Err(CodecError::Malformed("unknown tile prediction mode"));
        }
        let tile = Tile {
            plane: usize::from(bytes[start]),
            x: get_u32(bytes, start + 4)?,
            y: get_u32(bytes, start + 8)?,
            width: get_u32(bytes, start + 12)?,
            height: get_u32(bytes, start + 16)?,
        };
        if tile != expected_tile {
            return Err(CodecError::Malformed("non-canonical tile directory"));
        }
        let offset = usize::try_from(get_u64(bytes, start + 20)?)
            .map_err(|_| CodecError::LimitExceeded("payload offset is too large"))?;
        let length = get_u32(bytes, start + 28)? as usize;
        if offset != next_payload_offset {
            return Err(CodecError::Malformed("payloads are not contiguous"));
        }
        let end = offset
            .checked_add(length)
            .ok_or(CodecError::LimitExceeded("payload is too large"))?;
        if end > bytes.len() {
            return Err(CodecError::Malformed("truncated tile payload"));
        }
        zero_run_tiles += usize::from(entropy_mode == ENTROPY_ZERO_RUN);
        rice_tiles += usize::from(entropy_mode != ENTROPY_ZERO_RUN);
        spatial_tiles += usize::from(prediction_mode == PREDICT_SPATIAL);
        temporal_tiles += usize::from(prediction_mode == PREDICT_TEMPORAL);
        entries.push(DirectoryEntry {
            tile,
            entropy_mode,
            prediction_mode,
            offset,
            length,
        });
        next_payload_offset = end;
    }
    if next_payload_offset != bytes.len() {
        return Err(CodecError::Malformed("trailing stream bytes"));
    }
    Ok(Parsed {
        info: StreamInfo {
            width,
            height,
            format,
            frame_rate,
            quality,
            tile_width,
            tile_height,
            tile_count,
            zero_run_tiles,
            rice_tiles,
            spatial_tiles,
            temporal_tiles,
            encoded_bytes: bytes.len(),
        },
        entries,
        bytes,
    })
}

fn expected_tiles(
    width: u32,
    height: u32,
    format: PixelFormat,
    tile_width: u16,
    tile_height: u16,
) -> Result<Vec<Tile>, CodecError> {
    let mut tiles = Vec::new();
    for (plane, &(plane_width, plane_height)) in
        plane_dimensions(width, height, format).iter().enumerate()
    {
        checked_area(plane_width, plane_height)?;
        let nominal_width = if plane == 0 {
            u32::from(tile_width)
        } else {
            u32::from(tile_width).div_ceil(2)
        };
        let nominal_height = u32::from(tile_height);
        for row in 0..plane_height.div_ceil(nominal_height) {
            let y = row * nominal_height;
            for column in 0..plane_width.div_ceil(nominal_width) {
                let x = column * nominal_width;
                tiles.push(Tile {
                    plane,
                    x,
                    y,
                    width: nominal_width.min(plane_width - x),
                    height: nominal_height.min(plane_height - y),
                });
            }
        }
    }
    Ok(tiles)
}

fn validate_frame_dimensions(
    width: u32,
    height: u32,
    format: PixelFormat,
) -> Result<(), CodecError> {
    let total_samples = plane_dimensions(width, height, format)
        .into_iter()
        .try_fold(0usize, |sum, (plane_width, plane_height)| {
            sum.checked_add(checked_area(plane_width, plane_height)?)
                .ok_or(CodecError::LimitExceeded("frame is too large"))
        })?;
    total_samples
        .checked_mul(size_of::<u16>())
        .filter(|&bytes| bytes <= MAX_FRAME_BYTES)
        .ok_or(CodecError::LimitExceeded("frame is too large"))?;
    Ok(())
}

fn checked_high_area(width: u32, height: u32) -> Result<usize, CodecError> {
    let samples = checked_area(width, height)?;
    samples
        .checked_mul(size_of::<u16>())
        .filter(|&bytes| bytes <= MAX_FRAME_BYTES)
        .ok_or(CodecError::LimitExceeded("plane is too large"))?;
    Ok(samples)
}

fn plane_dimensions(width: u32, height: u32, format: PixelFormat) -> Vec<(u32, u32)> {
    if format.is_grayscale() {
        vec![(width, height)]
    } else {
        vec![
            (width, height),
            (width.div_ceil(2), height),
            (width.div_ceil(2), height),
        ]
    }
}

struct Quantizer16 {
    residuals: Vec<i32>,
    max_sample: i32,
    step: i32,
}

impl Quantizer16 {
    fn new(step: i32, bit_depth: u8) -> Self {
        let max_sample = i32::from(sample_max(bit_depth).expect("validated bit depth"));
        Self {
            residuals: (-max_sample..=max_sample)
                .map(|residual| quantize(residual, step))
                .collect(),
            max_sample,
            step,
        }
    }

    #[inline]
    fn quantize(&self, residual: i32) -> i32 {
        debug_assert!((-self.max_sample..=self.max_sample).contains(&residual));
        self.residuals[(residual + self.max_sample) as usize]
    }
}

fn encode_tile(
    plane: &Plane16,
    reference: Option<&Plane16>,
    tile: Tile,
    quantizer: &Quantizer16,
) -> EncodedTile {
    if let Some(reference) = reference {
        return encode_temporal_tile(plane, reference, tile, quantizer);
    }
    encode_spatial_tile(plane, tile, quantizer)
}

fn encode_temporal_tile(
    plane: &Plane16,
    reference: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut folded = Vec::with_capacity(width * height);
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        for (&sample, &prediction) in plane.data[row_start..row_start + width]
            .iter()
            .zip(&reference.data[row_start..row_start + width])
        {
            let quantized = quantizer.quantize(i32::from(sample) - i32::from(prediction));
            folded.push(zigzag(quantized));
        }
    }
    finish_entropy(folded, PREDICT_TEMPORAL)
}

fn encode_spatial_tile(plane: &Plane16, tile: Tile, quantizer: &Quantizer16) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u16; width];
    let mut folded = Vec::with_capacity(width * height);
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = 0;
        let mut upper_left = 0;
        for (&sample, reconstructed_slot) in plane.data[row_start..row_start + width]
            .iter()
            .zip(&mut reconstructed_row)
        {
            let above = *reconstructed_slot;
            let prediction = paeth(left, above, upper_left);
            let prediction_i32 = i32::from(prediction);
            let quantized = quantizer.quantize(i32::from(sample) - prediction_i32);
            let reconstructed =
                (prediction_i32 + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            folded.push(zigzag(quantized));
        }
    }
    finish_entropy(folded, PREDICT_SPATIAL)
}

fn finish_entropy(folded: Vec<u32>, prediction_mode: u8) -> EncodedTile {
    let mut zero_run_bytes = 0usize;
    let mut run = 0u32;
    for &value in &folded {
        if value == 0 {
            run += 1;
        } else {
            count_zero_run(&mut zero_run_bytes, &mut run);
            zero_run_bytes += varint_length(value * 2 - 1);
        }
    }
    count_zero_run(&mut zero_run_bytes, &mut run);
    let (rice_parameter, rice_bits) = best_rice_parameter(&folded);
    if rice_bits.div_ceil(8) >= zero_run_bytes as u64 {
        let mut payload = Vec::with_capacity(zero_run_bytes);
        let mut run = 0;
        for &value in &folded {
            if value == 0 {
                run += 1;
            } else {
                flush_zero_run(&mut payload, &mut run);
                put_varint(&mut payload, value * 2 - 1);
            }
        }
        flush_zero_run(&mut payload, &mut run);
        return EncodedTile {
            entropy_mode: ENTROPY_ZERO_RUN,
            prediction_mode,
            payload,
        };
    }
    let rice_bytes =
        usize::try_from(rice_bits.div_ceil(8)).expect("Rice won against a usize-sized payload");
    let mut payload = Vec::with_capacity(rice_bytes);
    let mut writer = BitWriter::new(&mut payload);
    for value in folded {
        writer.put_rice(value, rice_parameter);
    }
    writer.finish();
    EncodedTile {
        entropy_mode: ENTROPY_RICE_BASE + rice_parameter,
        prediction_mode,
        payload,
    }
}

fn model_folded_payload(
    payload: &[u8],
    sample_count: usize,
    entropy_mode: u8,
    max_folded: u32,
) -> Result<ByteFormatModel, CodecError> {
    let mut model = ByteFormatModel::default();
    if entropy_mode == ENTROPY_ZERO_RUN {
        let mut cursor = 0usize;
        let mut decoded = 0usize;
        while decoded < sample_count {
            let token = get_varint(payload, &mut cursor)?;
            if token & 1 == 0 {
                let run = usize::try_from(token / 2)
                    .ok()
                    .and_then(|run| run.checked_add(1))
                    .ok_or(CodecError::Malformed("zero run is too long"))?;
                decoded = decoded
                    .checked_add(run)
                    .filter(|&end| end <= sample_count)
                    .ok_or(CodecError::Malformed("zero run exceeds tile"))?;
                model.push_zeros(run);
            } else {
                let folded = token.div_ceil(2);
                if folded == 0 || folded > max_folded {
                    return Err(CodecError::Malformed("residual is out of range"));
                }
                model.push(folded);
                decoded += 1;
            }
        }
        if cursor != payload.len() {
            return Err(CodecError::Malformed("trailing tile payload bytes"));
        }
        return Ok(model);
    }
    if !(ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER).contains(&entropy_mode) {
        return Err(CodecError::Malformed("unknown tile entropy mode"));
    }
    let mut reader = BitReader::new(payload);
    for _ in 0..sample_count {
        model.push(reader.get_rice(entropy_mode - ENTROPY_RICE_BASE, max_folded)?);
    }
    reader.finish()?;
    Ok(model)
}

fn decode_tile_payload(
    tile: Tile,
    payload: &[u8],
    step: i32,
    entropy_mode: u8,
    prediction_mode: u8,
    reference: Option<&Plane16>,
    max_sample: u16,
) -> Result<Vec<u16>, CodecError> {
    let count = checked_high_area(tile.width, tile.height)?;
    let width = tile.width as usize;
    let context = PredictionContext {
        mode: prediction_mode,
        reference,
        tile,
    };
    if entropy_mode == ENTROPY_ZERO_RUN {
        return decode_zero_run(payload, count, width, step, context, max_sample);
    }
    decode_rice(
        payload,
        count,
        width,
        step,
        entropy_mode - ENTROPY_RICE_BASE,
        context,
        max_sample,
    )
}

fn decode_zero_run(
    payload: &[u8],
    sample_count: usize,
    width: usize,
    step: i32,
    context: PredictionContext<'_>,
    max_sample: u16,
) -> Result<Vec<u16>, CodecError> {
    let max_folded = u32::from(max_sample) * 2;
    let mut output = vec![0u16; sample_count];
    let mut cursor = 0;
    let mut index = 0;
    while index < sample_count {
        let token = get_varint(payload, &mut cursor)?;
        if token & 1 == 0 {
            let run = (token / 2) as usize + 1;
            let end = index
                .checked_add(run)
                .ok_or(CodecError::Malformed("zero run is too long"))?;
            if end > sample_count {
                return Err(CodecError::Malformed("zero run exceeds tile"));
            }
            if context.mode == PREDICT_TEMPORAL {
                copy_temporal_zero_run(&mut output, index, end, width, context)?;
                index = end;
            } else {
                while index < end {
                    reconstruct(&mut output, index, width, 0, step, context, max_sample)?;
                    index += 1;
                }
            }
        } else {
            let folded = token.div_ceil(2);
            if folded == 0 || folded > max_folded {
                return Err(CodecError::Malformed("residual is out of range"));
            }
            reconstruct(
                &mut output,
                index,
                width,
                unzigzag(folded),
                step,
                context,
                max_sample,
            )?;
            index += 1;
        }
    }
    if cursor != payload.len() {
        return Err(CodecError::Malformed("trailing tile payload bytes"));
    }
    Ok(output)
}

fn copy_temporal_zero_run(
    output: &mut [u16],
    mut start: usize,
    end: usize,
    tile_width: usize,
    context: PredictionContext<'_>,
) -> Result<(), CodecError> {
    let reference = context.reference.ok_or(CodecError::InvalidInput(
        "predicted frame requires a reference",
    ))?;
    let plane_width = reference.width as usize;
    while start < end {
        let x = start % tile_width;
        let y = start / tile_width;
        let span = (tile_width - x).min(end - start);
        let source = (context.tile.y as usize + y) * plane_width + context.tile.x as usize + x;
        output[start..start + span].copy_from_slice(&reference.data[source..source + span]);
        start += span;
    }
    Ok(())
}

fn decode_rice(
    payload: &[u8],
    sample_count: usize,
    width: usize,
    step: i32,
    parameter: u8,
    context: PredictionContext<'_>,
    max_sample: u16,
) -> Result<Vec<u16>, CodecError> {
    let max_folded = u32::from(max_sample) * 2;
    let mut output = vec![0u16; sample_count];
    let mut reader = BitReader::new(payload);
    for index in 0..sample_count {
        let folded = reader.get_rice(parameter, max_folded)?;
        reconstruct(
            &mut output,
            index,
            width,
            unzigzag(folded),
            step,
            context,
            max_sample,
        )?;
    }
    reader.finish()?;
    Ok(output)
}

fn reconstruct(
    output: &mut [u16],
    index: usize,
    width: usize,
    quantized: i32,
    step: i32,
    context: PredictionContext<'_>,
    max_sample: u16,
) -> Result<(), CodecError> {
    let x = index % width;
    let y = index / width;
    let prediction = if context.mode == PREDICT_TEMPORAL {
        let reference = context.reference.ok_or(CodecError::InvalidInput(
            "predicted frame requires a reference",
        ))?;
        let source =
            (context.tile.y as usize + y) * reference.width as usize + context.tile.x as usize + x;
        reference.data[source]
    } else {
        let left = if x > 0 { output[index - 1] } else { 0 };
        let above = if y > 0 { output[index - width] } else { 0 };
        let upper_left = if x > 0 && y > 0 {
            output[index - width - 1]
        } else {
            0
        };
        paeth(left, above, upper_left)
    };
    output[index] =
        (i32::from(prediction) + quantized * step).clamp(0, i32::from(max_sample)) as u16;
    Ok(())
}

fn paeth(left: u16, above: u16, upper_left: u16) -> u16 {
    let left_i = i32::from(left);
    let above_i = i32::from(above);
    let upper_left_i = i32::from(upper_left);
    let estimate = left_i + above_i - upper_left_i;
    let left_distance = (estimate - left_i).abs();
    let above_distance = (estimate - above_i).abs();
    let upper_left_distance = (estimate - upper_left_i).abs();
    if left_distance <= above_distance && left_distance <= upper_left_distance {
        left
    } else if above_distance <= upper_left_distance {
        above
    } else {
        upper_left
    }
}

fn quantize(value: i32, step: i32) -> i32 {
    let magnitude = (value.abs() + step / 2) / step;
    if value < 0 { -magnitude } else { magnitude }
}

pub(crate) fn quantization_step(quality: u8, bit_depth: u8) -> i32 {
    let base = 1 + i32::from((100 - quality) / 5);
    1 + ((base - 1) << (bit_depth - 8))
}

fn zigzag(value: i32) -> u32 {
    if value >= 0 {
        value as u32 * 2
    } else {
        value.unsigned_abs() * 2 - 1
    }
}

fn unzigzag(value: u32) -> i32 {
    if value & 1 == 0 {
        (value / 2) as i32
    } else {
        -((value / 2) as i32) - 1
    }
}

fn best_rice_parameter(folded: &[u32]) -> (u8, u64) {
    let mut best_parameter = 0;
    let mut best_bits = u64::MAX;
    for parameter in 0..=MAX_RICE_PARAMETER {
        let quotient_sum = folded
            .iter()
            .map(|&value| u64::from(value >> parameter))
            .sum::<u64>();
        let bits = folded.len() as u64 * (u64::from(parameter) + 1) + quotient_sum;
        if bits < best_bits {
            best_parameter = parameter;
            best_bits = bits;
        }
        if quotient_sum == 0 {
            break;
        }
    }
    (best_parameter, best_bits)
}

fn put_varint(output: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            return;
        }
    }
}

fn get_varint(input: &[u8], cursor: &mut usize) -> Result<u32, CodecError> {
    let start = *cursor;
    let mut value = 0u32;
    for byte_index in 0..5 {
        let byte = *input
            .get(*cursor)
            .ok_or(CodecError::Malformed("truncated variable integer"))?;
        *cursor += 1;
        if byte_index == 4 && byte > 0x0f {
            return Err(CodecError::Malformed("variable integer overflow"));
        }
        value |= u32::from(byte & 0x7f) << (byte_index * 7);
        if byte & 0x80 == 0 {
            if *cursor - start != varint_length(value) {
                return Err(CodecError::Malformed("non-canonical variable integer"));
            }
            return Ok(value);
        }
    }
    Err(CodecError::Malformed("variable integer overflow"))
}

fn varint_length(value: u32) -> usize {
    if value < 1 << 7 {
        1
    } else if value < 1 << 14 {
        2
    } else if value < 1 << 21 {
        3
    } else if value < 1 << 28 {
        4
    } else {
        5
    }
}

fn flush_zero_run(output: &mut Vec<u8>, run: &mut u32) {
    if *run != 0 {
        put_varint(output, (*run - 1) * 2);
        *run = 0;
    }
}

fn count_zero_run(bytes: &mut usize, run: &mut u32) {
    if *run != 0 {
        *bytes += varint_length((*run - 1) * 2);
        *run = 0;
    }
}

struct BitWriter<'a> {
    output: &'a mut Vec<u8>,
    buffer: u64,
    buffered_bits: u8,
}

impl<'a> BitWriter<'a> {
    fn new(output: &'a mut Vec<u8>) -> Self {
        Self {
            output,
            buffer: 0,
            buffered_bits: 0,
        }
    }

    fn put_rice(&mut self, value: u32, parameter: u8) {
        self.put_zeros(value >> parameter);
        self.put_bits(1, 1);
        self.put_bits(value, parameter);
    }

    fn put_zeros(&mut self, mut count: u32) {
        while count != 0 {
            let added = count.min(u32::from(64 - self.buffered_bits));
            self.buffered_bits += added as u8;
            count -= added;
            self.flush_bytes();
        }
    }

    fn put_bits(&mut self, value: u32, count: u8) {
        debug_assert!(count <= 16);
        if count != 0 {
            self.buffer |= (u64::from(value) & ((1u64 << count) - 1)) << self.buffered_bits;
            self.buffered_bits += count;
            self.flush_bytes();
        }
    }

    fn flush_bytes(&mut self) {
        while self.buffered_bits >= 8 {
            self.output.push(self.buffer as u8);
            self.buffer >>= 8;
            self.buffered_bits -= 8;
        }
    }

    fn finish(self) {
        if self.buffered_bits != 0 {
            self.output.push(self.buffer as u8);
        }
    }
}

struct BitReader<'a> {
    input: &'a [u8],
    cursor: usize,
    buffer: u64,
    buffered_bits: u8,
    consumed_bits: usize,
}

impl<'a> BitReader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            cursor: 0,
            buffer: 0,
            buffered_bits: 0,
            consumed_bits: 0,
        }
    }

    fn get_rice(&mut self, parameter: u8, max_folded: u32) -> Result<u32, CodecError> {
        let mut quotient = 0u32;
        loop {
            self.fill_buffer();
            if self.buffered_bits == 0 {
                return Err(CodecError::Malformed("truncated Rice payload"));
            }
            let zeros = self
                .buffer
                .trailing_zeros()
                .min(u32::from(self.buffered_bits));
            quotient = quotient
                .checked_add(zeros)
                .ok_or(CodecError::Malformed("Rice quotient overflow"))?;
            if quotient > max_folded {
                return Err(CodecError::Malformed("Rice residual is out of range"));
            }
            self.consume(zeros as u8);
            if self.buffered_bits != 0 {
                self.consume(1);
                break;
            }
        }
        let remainder = self.get_bits(parameter)?;
        let value = quotient
            .checked_shl(u32::from(parameter))
            .and_then(|value| value.checked_add(remainder))
            .ok_or(CodecError::Malformed("Rice residual overflow"))?;
        if value > max_folded {
            return Err(CodecError::Malformed("Rice residual is out of range"));
        }
        Ok(value)
    }

    fn get_bits(&mut self, count: u8) -> Result<u32, CodecError> {
        self.fill_buffer();
        if self.buffered_bits < count {
            return Err(CodecError::Malformed("truncated Rice payload"));
        }
        let mask = if count == 0 { 0 } else { (1u64 << count) - 1 };
        let value = (self.buffer & mask) as u32;
        self.consume(count);
        Ok(value)
    }

    fn fill_buffer(&mut self) {
        while self.buffered_bits <= 56 && self.cursor < self.input.len() {
            self.buffer |= u64::from(self.input[self.cursor]) << self.buffered_bits;
            self.cursor += 1;
            self.buffered_bits += 8;
        }
    }

    fn consume(&mut self, count: u8) {
        if count == 64 {
            self.buffer = 0;
        } else {
            self.buffer >>= count;
        }
        self.buffered_bits -= count;
        self.consumed_bits += usize::from(count);
    }

    fn finish(self) -> Result<(), CodecError> {
        let used_bytes = self.consumed_bits.div_ceil(8);
        if used_bytes != self.input.len() {
            return Err(CodecError::Malformed("trailing Rice payload bytes"));
        }
        if !self.consumed_bits.is_multiple_of(8) {
            let padding_mask = !((1u8 << (self.consumed_bits % 8)) - 1);
            if self.input[used_bytes - 1] & padding_mask != 0 {
                return Err(CodecError::Malformed("nonzero Rice padding bits"));
            }
        }
        Ok(())
    }
}

fn put_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn get_u16(input: &[u8], offset: usize) -> Result<u16, CodecError> {
    Ok(u16::from_le_bytes(
        input
            .get(offset..offset + 2)
            .ok_or(CodecError::Malformed("truncated integer"))?
            .try_into()
            .expect("exact length"),
    ))
}

fn get_u32(input: &[u8], offset: usize) -> Result<u32, CodecError> {
    Ok(u32::from_le_bytes(
        input
            .get(offset..offset + 4)
            .ok_or(CodecError::Malformed("truncated integer"))?
            .try_into()
            .expect("exact length"),
    ))
}

fn get_u64(input: &[u8], offset: usize) -> Result<u64, CodecError> {
    Ok(u64::from_le_bytes(
        input
            .get(offset..offset + 8)
            .ok_or(CodecError::Malformed("truncated integer"))?
            .try_into()
            .expect("exact length"),
    ))
}

fn parallel_map<T, F>(length: usize, threads: usize, operation: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync,
{
    let worker_count = threads.min(length.max(1));
    if worker_count == 1 {
        return (0..length).map(operation).collect();
    }
    let next = AtomicUsize::new(0);
    let output: Mutex<Vec<Option<T>>> =
        Mutex::new(std::iter::repeat_with(|| None).take(length).collect());
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let operation = &operation;
            let next = &next;
            let output = &output;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= length {
                        break;
                    }
                    output.lock().expect("worker output lock poisoned")[index] =
                        Some(operation(index));
                }
            });
        }
    });
    output
        .into_inner()
        .expect("worker output lock poisoned")
        .into_iter()
        .map(|value| value.expect("worker failed to produce output"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(quality: u8) -> CodecOptions {
        CodecOptions {
            quality,
            tile_width: 4,
            tile_height: 3,
            threads: 2,
        }
    }

    fn patterned_frame(bit_depth: u8, grayscale: bool) -> Frame16 {
        let width = 7;
        let height = 5;
        let max = sample_max(bit_depth).unwrap();
        let make_plane = |samples: usize, seed: u32| {
            (0..samples)
                .map(|index| {
                    let value = (index as u32 * 977 + seed * 131 + (index as u32).pow(2) * 17)
                        % (u32::from(max) + 1);
                    value as u16
                })
                .collect()
        };
        if grayscale {
            Frame16::gray(
                width,
                height,
                bit_depth,
                FrameRate::new(24, 1),
                make_plane((width * height) as usize, 1),
            )
            .unwrap()
        } else {
            let chroma_samples = (width.div_ceil(2) * height) as usize;
            Frame16::yuv422(
                width,
                height,
                bit_depth,
                FrameRate::new(24, 1),
                make_plane((width * height) as usize, 1),
                make_plane(chroma_samples, 2),
                make_plane(chroma_samples, 3),
            )
            .unwrap()
        }
    }

    #[test]
    fn quality_100_round_trips_every_high_depth_and_layout() {
        for bit_depth in [10, 12, 16] {
            for grayscale in [true, false] {
                let frame = patterned_frame(bit_depth, grayscale);
                let encoded = encode16(&frame, options(100)).unwrap();
                let info = inspect16(&encoded).unwrap();
                assert_eq!(info.format, frame.format);
                assert_eq!(info.quality, 100);
                assert_eq!(encoded[4], VERSION);
                assert_eq!(encoded[7], bit_depth - 8);
                assert_eq!(decode16(&encoded, 2).unwrap(), frame);
                let models = analyze_entropy16(&encoded).unwrap();
                assert_eq!(models.len(), info.tile_count);
                assert_eq!(
                    models.iter().map(|model| model.sample_count).sum::<usize>(),
                    frame.planes.iter().map(|plane| plane.data.len()).sum()
                );
                for tile_index in 0..info.tile_count {
                    let tile = decode_tile16(&encoded, tile_index).unwrap();
                    assert_eq!(tile.data.len(), (tile.width * tile.height) as usize);
                }
            }
        }
    }

    #[test]
    fn lossy_error_is_bounded_at_every_high_depth() {
        for bit_depth in [10, 12, 16] {
            let frame = patterned_frame(bit_depth, false);
            for quality in [1, 60, 90, 99] {
                let decoded = decode16(&encode16(&frame, options(quality)).unwrap(), 2).unwrap();
                let bound = (quantization_step(quality, bit_depth) / 2) as u16;
                for (reference, actual) in frame.planes.iter().zip(&decoded.planes) {
                    assert!(
                        reference
                            .data
                            .iter()
                            .zip(&actual.data)
                            .all(|(&expected, &actual)| expected.abs_diff(actual) <= bound)
                    );
                }
            }
        }
    }

    #[test]
    fn temporal_high_bit_frames_round_trip_and_require_matching_reference() {
        let reference = patterned_frame(12, false);
        let mut current = reference.clone();
        for plane in &mut current.planes {
            for sample in plane.data.iter_mut().step_by(11) {
                *sample = sample.saturating_add(1).min(4095);
            }
        }
        let encoded = encode16_with_reference(&current, &reference, options(100)).unwrap();
        let info = inspect16(&encoded).unwrap();
        assert!(info.temporal_tiles > 0);
        assert!(decode16(&encoded, 1).is_err());
        assert_eq!(
            decode16_with_reference(&encoded, &reference, 2).unwrap(),
            current
        );

        let wrong_depth = patterned_frame(10, false);
        assert!(decode16_with_reference(&encoded, &wrong_depth, 1).is_err());
        assert!(encode16_with_reference(&current, &wrong_depth, options(100)).is_err());
    }

    #[test]
    fn widened_rice_codes_round_trip_extreme_residuals() {
        let values = [0, 1, 2, 510, 2046, 8190, 131_070];
        for parameter in 0..=MAX_RICE_PARAMETER {
            let mut payload = Vec::new();
            let mut writer = BitWriter::new(&mut payload);
            for value in values {
                writer.put_rice(value, parameter);
            }
            writer.finish();
            let mut reader = BitReader::new(&payload);
            for value in values {
                assert_eq!(reader.get_rice(parameter, 131_070).unwrap(), value);
            }
            reader.finish().unwrap();
        }
    }

    #[test]
    fn malformed_high_bit_streams_are_rejected_before_decode() {
        let frame = patterned_frame(16, true);
        let encoded = encode16(&frame, options(100)).unwrap();

        let mut invalid_depth = encoded.clone();
        invalid_depth[7] = 1;
        assert!(inspect16(&invalid_depth).is_err());

        let mut invalid_layout = encoded.clone();
        invalid_layout[5] = 2;
        assert!(inspect16(&invalid_layout).is_err());

        let mut reserved = encoded.clone();
        reserved[HEADER_LEN + 3] = 1;
        assert!(inspect16(&reserved).is_err());

        let mut invalid_entropy = encoded.clone();
        invalid_entropy[HEADER_LEN + 1] = ENTROPY_RICE_BASE + MAX_RICE_PARAMETER + 1;
        assert!(inspect16(&invalid_entropy).is_err());

        let mut huge = encoded;
        huge[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        huge[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            inspect16(&huge),
            Err(CodecError::LimitExceeded(_))
        ));
    }

    #[test]
    fn quantizer_table_matches_scalar_for_every_high_bit_residual() {
        for bit_depth in [10, 12, 16] {
            let max_sample = i32::from(sample_max(bit_depth).unwrap());
            for quality in 1..=100 {
                let step = quantization_step(quality, bit_depth);
                let quantizer = Quantizer16::new(step, bit_depth);
                assert_eq!(
                    quantizer.residuals.len(),
                    (u32::from(sample_max(bit_depth).unwrap()) * 2 + 1) as usize
                );
                for residual in -max_sample..=max_sample {
                    assert_eq!(quantizer.quantize(residual), quantize(residual, step));
                }
            }
        }
    }

    fn best_rice_parameter_full_scan(folded: &[u32]) -> (u8, u64) {
        let mut best_parameter = 0;
        let mut best_bits = u64::MAX;
        for parameter in 0..=MAX_RICE_PARAMETER {
            let bits = folded.iter().fold(
                folded.len() as u64 * (u64::from(parameter) + 1),
                |sum, &value| sum + u64::from(value >> parameter),
            );
            if bits < best_bits {
                best_parameter = parameter;
                best_bits = bits;
            }
        }
        (best_parameter, best_bits)
    }

    #[test]
    fn early_rice_termination_matches_full_parameter_scan() {
        for value in 0..=131_070 {
            assert_eq!(
                best_rice_parameter(&[value]),
                best_rice_parameter_full_scan(&[value])
            );
        }
        for stride in [1, 3, 7, 257, 4093] {
            let mixed: Vec<u32> = (0..=131_070).step_by(stride).collect();
            assert_eq!(
                best_rice_parameter(&mixed),
                best_rice_parameter_full_scan(&mixed)
            );
        }
    }
}
