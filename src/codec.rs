use crate::model::{
    ByteFormatModel, CodecError, CodecOptions, Frame, FrameRate, PixelFormat, Plane,
    PredictorCandidateModel, PredictorModelMode, TileEntropyModel, TilePredictorModel,
    TileResidualMappingModel, checked_area, fold_bounded_residual,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAGIC: &[u8; 4] = b"FVID";
const LEGACY_VERSION: u8 = 0;
const PREDICTOR_VERSION: u8 = 2;
const VERSION: u8 = 3;
const HEADER_LEN: usize = 32;
const DIRECTORY_ENTRY_LEN: usize = 32;
const MAX_TILES: usize = 1 << 20;
const ENTROPY_ZERO_RUN: u8 = 0;
const ENTROPY_RICE_BASE: u8 = 1;
const MAX_RICE_PARAMETER: u8 = 8;
const ENTROPY_ORDER0: u8 = ENTROPY_RICE_BASE + MAX_RICE_PARAMETER + 1;
const ENTROPY_ORDER0_4: u8 = ENTROPY_ORDER0 + 1;
const RANS_MIN_TABLE_LOG: u8 = 8;
const RANS_MAX_TABLE_LOG: u8 = 12;
const RANS_BYTE_L: u32 = 1 << 23;
const RANS_INTERLEAVED_STATES: usize = 4;
const RANS_INTERLEAVED_MAX_OVERHEAD_PER_MILLE: usize = 5;
const PREDICT_SPATIAL: u8 = 0;
const PREDICT_TEMPORAL: u8 = 1;
const PREDICT_AVERAGE: u8 = 2;
const PREDICT_CLAMP_GRADIENT: u8 = 3;
const PREDICT_HALF_GRADIENT: u8 = 4;
const MAX_PREDICTION_MODE: u8 = PREDICT_HALF_GRADIENT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Tile {
    plane: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug)]
struct DirectoryEntry {
    tile: Tile,
    entropy_mode: u8,
    prediction_mode: u8,
    offset: usize,
    length: usize,
}

struct EncodedTile {
    entropy_mode: u8,
    prediction_mode: u8,
    payload: Vec<u8>,
}

#[derive(Clone, Copy)]
struct PredictionContext<'a> {
    mode: u8,
    reference: Option<&'a Plane>,
    tile: Tile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInfo {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub frame_rate: FrameRate,
    pub quality: u8,
    pub tile_width: u16,
    pub tile_height: u16,
    pub tile_count: usize,
    pub zero_run_tiles: usize,
    pub rice_tiles: usize,
    pub spatial_tiles: usize,
    pub temporal_tiles: usize,
    pub encoded_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedTile {
    pub plane: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

struct Parsed<'a> {
    info: StreamInfo,
    entries: Vec<DirectoryEntry>,
    bytes: &'a [u8],
}

pub fn encode(frame: &Frame, options: CodecOptions) -> Result<Vec<u8>, CodecError> {
    encode_internal(frame, None, false, options)
}

/// Encodes a predicted frame using the previously reconstructed frame.
///
/// The reference must be the decoder output for the preceding coded frame,
/// not the original input, so lossy encoder and decoder state cannot drift.
pub fn encode_with_reference(
    frame: &Frame,
    reference: &Frame,
    options: CodecOptions,
) -> Result<Vec<u8>, CodecError> {
    validate_reference(frame, reference)?;
    // research/0007: temporal prediction wins on low/moderate motion but can
    // expand high-motion frames. A luma SAD prepass avoids running two encoders.
    let prefer_temporal = temporal_prediction_is_promising(frame, reference);
    encode_internal(frame, Some(reference), prefer_temporal, options)
}

fn encode_internal(
    frame: &Frame,
    reference: Option<&Frame>,
    prefer_temporal: bool,
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
    let step = options.quantization_step();
    let quantizer = Quantizer::new(step);
    let payloads = parallel_map(tiles.len(), options.threads, |index| {
        encode_best_tile(
            &frame.planes[tiles[index].plane],
            reference.map(|frame| &frame.planes[tiles[index].plane]),
            tiles[index],
            &quantizer,
            prefer_temporal,
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
    output.push(frame.format as u8);
    output.push(options.quality);
    output.push(0);
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

pub fn decode(bytes: &[u8], threads: usize) -> Result<Frame, CodecError> {
    decode_internal(bytes, None, threads)
}

/// Decodes a predicted frame using the preceding reconstructed frame.
pub fn decode_with_reference(
    bytes: &[u8],
    reference: &Frame,
    threads: usize,
) -> Result<Frame, CodecError> {
    decode_internal(bytes, Some(reference), threads)
}

fn decode_internal(
    bytes: &[u8],
    reference: Option<&Frame>,
    threads: usize,
) -> Result<Frame, CodecError> {
    if threads == 0 {
        return Err(CodecError::InvalidInput("thread count must be nonzero"));
    }
    let parsed = parse(bytes)?;
    if let Some(reference) = reference {
        validate_reference_info(&parsed.info, reference)?;
    }
    let step = quantization_step(parsed.info.quality);
    let decoded = parallel_map(parsed.entries.len(), threads, |index| {
        let entry = parsed.entries[index];
        decode_tile_payload(
            entry.tile,
            &parsed.bytes[entry.offset..entry.offset + entry.length],
            step,
            entry.entropy_mode,
            entry.prediction_mode,
            reference.map(|frame| &frame.planes[entry.tile.plane]),
        )
    });
    let mut decoded_tiles = Vec::with_capacity(decoded.len());
    for tile in decoded {
        decoded_tiles.push(tile?);
    }
    assemble_frame(&parsed.info, decoded_tiles)
}

pub fn decode_tile(bytes: &[u8], tile_index: usize) -> Result<DecodedTile, CodecError> {
    let parsed = parse(bytes)?;
    let entry = *parsed
        .entries
        .get(tile_index)
        .ok_or(CodecError::InvalidInput("tile index is out of range"))?;
    let data = decode_tile_payload(
        entry.tile,
        &parsed.bytes[entry.offset..entry.offset + entry.length],
        quantization_step(parsed.info.quality),
        entry.entropy_mode,
        entry.prediction_mode,
        None,
    )?;
    Ok(DecodedTile {
        plane: entry.tile.plane,
        x: entry.tile.x,
        y: entry.tile.y,
        width: entry.tile.width,
        height: entry.tile.height,
        data,
    })
}

pub fn inspect(bytes: &[u8]) -> Result<StreamInfo, CodecError> {
    Ok(parse(bytes)?.info)
}

/// Models byte-oriented alternatives from the normative residual symbols
/// without changing or re-encoding the stream.
pub fn analyze_entropy(bytes: &[u8]) -> Result<Vec<TileEntropyModel>, CodecError> {
    let parsed = parse(bytes)?;
    parsed
        .entries
        .iter()
        .map(|entry| {
            let payload = &parsed.bytes[entry.offset..entry.offset + entry.length];
            let sample_count = checked_area(entry.tile.width, entry.tile.height)?;
            let model = model_folded_payload(
                payload,
                sample_count,
                entry.entropy_mode,
                u32::from(u8::MAX) * 2,
            )?;
            let order0 = model.order0_size();
            let context_order0 = model.context_order0_size();
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
                distinct_symbols: order0.distinct_symbols,
                ideal_order0_bytes: order0.ideal_bytes,
                order0_supported: order0.supported,
                order0_table_log: order0.table_log,
                order0_payload_bytes: order0.payload_bytes,
                order0_table_bytes: order0.table_bytes,
                order0_complete_bytes: order0.complete_bytes,
                context_order0_supported: context_order0.supported,
                context_order0_contexts: context_order0.contexts,
                context_order0_threshold: context_order0.threshold,
                context_order0_payload_bytes: context_order0.payload_bytes,
                context_order0_table_bytes: context_order0.table_bytes,
                context_order0_control_bytes: context_order0.control_bytes,
                context_order0_complete_bytes: context_order0.complete_bytes,
            })
        })
        .collect()
}

/// Models predictor-bounded residual symbols without changing the stream.
///
/// `reference` has the same meaning as in [`encode_with_reference`]. The
/// encoder's motion gate is applied before tiles are modeled.
pub fn analyze_residual_mapping(
    frame: &Frame,
    reference: Option<&Frame>,
    options: CodecOptions,
) -> Result<Vec<TileResidualMappingModel>, CodecError> {
    frame.validate()?;
    options.validate()?;
    if let Some(reference) = reference {
        validate_reference(frame, reference)?;
    }
    let reference =
        reference.filter(|reference| temporal_prediction_is_promising(frame, reference));
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let quantizer = Quantizer::new(options.quantization_step());
    Ok(tiles
        .into_iter()
        .map(|tile| {
            let plane = &frame.planes[tile.plane];
            let reference_plane = reference.map(|frame| &frame.planes[tile.plane]);
            model_residual_mapping_tile(plane, reference_plane, tile, &quantizer)
        })
        .collect())
}

/// Models compatible spatial predictors and tile-local temporal prediction.
pub fn analyze_predictors(
    frame: &Frame,
    reference: Option<&Frame>,
    options: CodecOptions,
) -> Result<Vec<TilePredictorModel>, CodecError> {
    frame.validate()?;
    options.validate()?;
    if let Some(reference) = reference {
        validate_reference(frame, reference)?;
    }
    let current_reference =
        reference.filter(|reference| temporal_prediction_is_promising(frame, reference));
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let quantizer = Quantizer::new(options.quantization_step());
    Ok(tiles
        .into_iter()
        .map(|tile| {
            model_predictor_tile(
                &frame.planes[tile.plane],
                reference.map(|frame| &frame.planes[tile.plane]),
                current_reference.map(|frame| &frame.planes[tile.plane]),
                tile,
                &quantizer,
            )
            .0
        })
        .collect())
}

/// Models one sequence frame and returns the oracle reconstruction.
///
/// Separate references keep temporal costs exact after lossy predictor
/// choices cause the current and oracle reconstruction lineages to diverge.
pub fn analyze_predictor_frame(
    frame: &Frame,
    current_reference: Option<&Frame>,
    oracle_reference: Option<&Frame>,
    options: CodecOptions,
) -> Result<(Vec<TilePredictorModel>, Frame), CodecError> {
    frame.validate()?;
    options.validate()?;
    if current_reference.is_some() != oracle_reference.is_some() {
        return Err(CodecError::InvalidInput(
            "current and oracle references must have matching dependency depth",
        ));
    }
    if let Some(reference) = current_reference {
        validate_reference(frame, reference)?;
    }
    if let Some(reference) = oracle_reference {
        validate_reference(frame, reference)?;
    }
    let selected_current =
        current_reference.filter(|reference| temporal_prediction_is_promising(frame, reference));
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let quantizer = Quantizer::new(options.quantization_step());
    let modeled: Vec<(TilePredictorModel, Vec<u8>)> = tiles
        .iter()
        .map(|&tile| {
            model_predictor_tile(
                &frame.planes[tile.plane],
                oracle_reference.map(|frame| &frame.planes[tile.plane]),
                selected_current.map(|frame| &frame.planes[tile.plane]),
                tile,
                &quantizer,
            )
        })
        .collect();
    let mut reconstruction = frame.clone();
    for plane in &mut reconstruction.planes {
        plane.data.fill(0);
    }
    for ((_, data), tile) in modeled.iter().zip(&tiles) {
        let plane = &mut reconstruction.planes[tile.plane];
        let tile_width = tile.width as usize;
        for (row, source) in data.chunks_exact(tile_width).enumerate() {
            let start = (tile.y as usize + row) * plane.width as usize + tile.x as usize;
            plane.data[start..start + tile_width].copy_from_slice(source);
        }
    }
    Ok((
        modeled.into_iter().map(|(model, _)| model).collect(),
        reconstruction,
    ))
}

fn validate_reference(frame: &Frame, reference: &Frame) -> Result<(), CodecError> {
    frame.validate()?;
    reference.validate()?;
    if frame.width != reference.width
        || frame.height != reference.height
        || frame.format != reference.format
    {
        return Err(CodecError::InvalidInput(
            "reference dimensions or format do not match frame",
        ));
    }
    Ok(())
}

fn temporal_prediction_is_promising(frame: &Frame, reference: &Frame) -> bool {
    const MAX_MEAN_LUMA_DIFFERENCE: u64 = 5;
    let current = &frame.planes[0].data;
    let previous = &reference.planes[0].data;
    let absolute_difference: u64 = current
        .iter()
        .zip(previous)
        .map(|(&current, &previous)| u64::from(current.abs_diff(previous)))
        .sum();
    absolute_difference <= current.len() as u64 * MAX_MEAN_LUMA_DIFFERENCE
}

fn validate_reference_info(info: &StreamInfo, reference: &Frame) -> Result<(), CodecError> {
    reference.validate()?;
    if info.width != reference.width
        || info.height != reference.height
        || info.format != reference.format
    {
        return Err(CodecError::InvalidInput(
            "reference dimensions or format do not match stream",
        ));
    }
    Ok(())
}

fn assemble_frame(info: &StreamInfo, tiles: Vec<Vec<u8>>) -> Result<Frame, CodecError> {
    let dimensions = plane_dimensions(info.width, info.height, info.format);
    let mut planes = Vec::with_capacity(dimensions.len());
    for &(width, height) in &dimensions {
        planes.push(Plane {
            width,
            height,
            data: vec![0; checked_area(width, height)?],
        });
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
        let tile_width = usize::try_from(tile.width)
            .map_err(|_| CodecError::LimitExceeded("tile is too large"))?;
        let x =
            usize::try_from(tile.x).map_err(|_| CodecError::LimitExceeded("tile is too large"))?;
        let y =
            usize::try_from(tile.y).map_err(|_| CodecError::LimitExceeded("tile is too large"))?;
        let plane_width = usize::try_from(plane.width)
            .map_err(|_| CodecError::LimitExceeded("plane is too large"))?;
        for row in 0..usize::try_from(tile.height)
            .map_err(|_| CodecError::LimitExceeded("tile is too large"))?
        {
            let source = row * tile_width;
            let destination = (y + row) * plane_width + x;
            plane.data[destination..destination + tile_width]
                .copy_from_slice(&decoded[source..source + tile_width]);
        }
    }
    let frame = Frame {
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
    if &bytes[0..4] != MAGIC {
        return Err(CodecError::Malformed("bad magic"));
    }
    let version = bytes[4];
    if ![LEGACY_VERSION, PREDICTOR_VERSION, VERSION].contains(&version) {
        return Err(CodecError::Malformed("unsupported version"));
    }
    let format = PixelFormat::try_from(bytes[5])?;
    if format.bit_depth() != 8 {
        return Err(CodecError::Malformed(
            "8-bit stream has a high-bit pixel format",
        ));
    }
    let quality = bytes[6];
    if !(1..=100).contains(&quality) {
        return Err(CodecError::Malformed("quality is out of range"));
    }
    if bytes[7] != 0 {
        return Err(CodecError::Malformed("nonzero reserved header byte"));
    }
    let width = get_u32(bytes, 8)?;
    let height = get_u32(bytes, 12)?;
    let tile_width = get_u16(bytes, 16)?;
    let tile_height = get_u16(bytes, 18)?;
    let frame_rate = FrameRate::new(get_u32(bytes, 20)?, get_u32(bytes, 24)?);
    let tile_count = usize::try_from(get_u32(bytes, 28)?)
        .map_err(|_| CodecError::LimitExceeded("too many tiles"))?;
    if width == 0 || height == 0 || tile_width == 0 || tile_height == 0 {
        return Err(CodecError::Malformed("zero dimension"));
    }
    frame_rate
        .validate()
        .map_err(|_| CodecError::Malformed("zero frame-rate component"))?;
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
    let mut zero_run_tiles = 0usize;
    let mut rice_tiles = 0usize;
    let mut spatial_tiles = 0usize;
    let mut temporal_tiles = 0usize;
    for (index, expected_tile) in expected.into_iter().enumerate() {
        let start = HEADER_LEN + index * DIRECTORY_ENTRY_LEN;
        if bytes[start + 3] != 0 {
            return Err(CodecError::Malformed("nonzero reserved directory bytes"));
        }
        let entropy_mode = bytes[start + 1];
        if entropy_mode != ENTROPY_ZERO_RUN
            && !(ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER).contains(&entropy_mode)
            && !(version == VERSION && matches!(entropy_mode, ENTROPY_ORDER0 | ENTROPY_ORDER0_4))
        {
            return Err(CodecError::Malformed("unknown tile entropy mode"));
        }
        let prediction_mode = bytes[start + 2];
        let maximum_prediction_mode = if version == LEGACY_VERSION {
            PREDICT_TEMPORAL
        } else {
            MAX_PREDICTION_MODE
        };
        if prediction_mode > maximum_prediction_mode {
            return Err(CodecError::Malformed("unknown tile prediction mode"));
        }
        if entropy_mode == ENTROPY_ZERO_RUN {
            zero_run_tiles += 1;
        } else {
            rice_tiles += 1;
        }
        if prediction_mode == PREDICT_TEMPORAL {
            temporal_tiles += 1;
        } else {
            spatial_tiles += 1;
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
        let length = usize::try_from(get_u32(bytes, start + 28)?)
            .map_err(|_| CodecError::LimitExceeded("payload is too large"))?;
        if offset != next_payload_offset {
            return Err(CodecError::Malformed("payloads are not contiguous"));
        }
        let end = offset
            .checked_add(length)
            .ok_or(CodecError::LimitExceeded("payload is too large"))?;
        if end > bytes.len() {
            return Err(CodecError::Malformed("truncated tile payload"));
        }
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
        let columns = plane_width.div_ceil(nominal_width);
        let rows = plane_height.div_ceil(nominal_height);
        let count = usize::try_from(columns)
            .ok()
            .and_then(|columns| {
                usize::try_from(rows)
                    .ok()
                    .and_then(|rows| columns.checked_mul(rows))
            })
            .ok_or(CodecError::LimitExceeded("too many tiles"))?;
        if tiles.len().saturating_add(count) > MAX_TILES {
            return Err(CodecError::LimitExceeded("too many tiles"));
        }
        for row in 0..rows {
            let y = row * nominal_height;
            for column in 0..columns {
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

struct Quantizer {
    residuals: [i16; 511],
    step: i32,
}

impl Quantizer {
    fn new(step: i32) -> Self {
        Self {
            residuals: std::array::from_fn(|index| quantize(index as i32 - 255, step) as i16),
            step,
        }
    }

    #[inline]
    fn quantize(&self, residual: i32) -> i32 {
        debug_assert!((-255..=255).contains(&residual));
        i32::from(self.residuals[(residual + 255) as usize])
    }
}

fn encode_tile(
    plane: &Plane,
    reference: Option<&Plane>,
    tile: Tile,
    quantizer: &Quantizer,
) -> EncodedTile {
    if let Some(reference) = reference {
        return encode_temporal_tile(plane, reference, tile, quantizer);
    }
    encode_spatial_tile(plane, tile, quantizer)
}

fn encode_best_tile(
    plane: &Plane,
    reference: Option<&Plane>,
    tile: Tile,
    quantizer: &Quantizer,
    prefer_temporal: bool,
) -> EncodedTile {
    const MODES: [SpatialPredictor; 4] = [
        SpatialPredictor::Paeth,
        SpatialPredictor::Average,
        SpatialPredictor::ClampGradient,
        SpatialPredictor::HalfGradient,
    ];
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut rows: [Vec<u8>; 4] = std::array::from_fn(|_| vec![0u8; width]);
    let mut spatial_residuals: [ResidualAccumulator; 4] =
        std::array::from_fn(|_| ResidualAccumulator::new(width * height));
    let mut spatial_squared_errors = [0u64; 4];
    let mut temporal_residuals = reference.map(|_| ResidualAccumulator::new(width * height));
    let mut temporal_squared_error = 0u64;
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = [0u8; 4];
        let mut upper_left = [0u8; 4];
        for (x, &sample) in plane.data[row_start..row_start + width].iter().enumerate() {
            for mode_index in 0..MODES.len() {
                let above = rows[mode_index][x];
                let prediction = i32::from(spatial_prediction(
                    MODES[mode_index],
                    left[mode_index],
                    above,
                    upper_left[mode_index],
                ));
                let quantized = quantizer.quantize(i32::from(sample) - prediction);
                let reconstructed = (prediction + quantized * quantizer.step).clamp(0, 255) as u8;
                let error = i64::from(sample) - i64::from(reconstructed);
                spatial_squared_errors[mode_index] += (error * error) as u64;
                rows[mode_index][x] = reconstructed;
                upper_left[mode_index] = above;
                left[mode_index] = reconstructed;
                spatial_residuals[mode_index].push(quantized);
            }
            if let (Some(reference), Some(residuals)) = (reference, temporal_residuals.as_mut()) {
                let prediction = i32::from(reference.data[row_start + x]);
                let quantized = quantizer.quantize(i32::from(sample) - prediction);
                let reconstructed = (prediction + quantized * quantizer.step).clamp(0, 255);
                let error = i64::from(sample) - i64::from(reconstructed);
                temporal_squared_error += (error * error) as u64;
                residuals.push(quantized);
            }
        }
    }
    let mut candidates: Vec<(u8, ResidualAccumulator, u64, usize)> = MODES
        .into_iter()
        .zip(spatial_residuals)
        .zip(spatial_squared_errors)
        .map(|((mode, residuals), squared_error)| {
            let bytes = residuals.cost().1;
            (
                spatial_prediction_mode(mode),
                residuals,
                squared_error,
                bytes,
            )
        })
        .collect();
    if let Some(residuals) = temporal_residuals {
        let bytes = residuals.cost().1;
        candidates.push((PREDICT_TEMPORAL, residuals, temporal_squared_error, bytes));
    }
    let minimum_bytes = candidates
        .iter()
        .map(|candidate| candidate.3)
        .min()
        .expect("spatial candidates are nonempty");
    let preferred_mode = if prefer_temporal && reference.is_some() {
        PREDICT_TEMPORAL
    } else {
        PREDICT_SPATIAL
    };
    let selected = candidates
        .iter()
        .position(|candidate| candidate.0 == preferred_mode && candidate.3 == minimum_bytes)
        .unwrap_or_else(|| {
            candidates
                .iter()
                .enumerate()
                .filter(|(_, candidate)| candidate.3 == minimum_bytes)
                .min_by_key(|(_, candidate)| (candidate.2, candidate.0))
                .map(|(index, _)| index)
                .expect("a minimum candidate exists")
        });
    let (prediction_mode, residuals, _, _) = candidates.swap_remove(selected);
    residuals.finish(prediction_mode)
}

struct ResidualAccumulator {
    folded: Vec<u16>,
    histogram: [u32; 511],
    zero_run_bytes: usize,
    zero_run: u32,
}

impl ResidualAccumulator {
    fn new(sample_count: usize) -> Self {
        Self {
            folded: Vec::with_capacity(sample_count),
            histogram: [0; 511],
            zero_run_bytes: 0,
            zero_run: 0,
        }
    }

    fn push(&mut self, quantized: i32) {
        let folded = zigzag(quantized);
        self.folded.push(folded as u16);
        self.histogram[folded as usize] += 1;
        if quantized == 0 {
            self.zero_run += 1;
        } else {
            count_zero_run(&mut self.zero_run_bytes, &mut self.zero_run);
            self.zero_run_bytes += varint_length(folded * 2 - 1);
        }
    }

    fn cost(&self) -> (u8, usize) {
        let mut zero_run_bytes = self.zero_run_bytes;
        let mut zero_run = self.zero_run;
        count_zero_run(&mut zero_run_bytes, &mut zero_run);
        let (rice_parameter, rice_bits) = best_rice_parameter(&self.histogram, self.folded.len());
        let rice_bytes = rice_bits.div_ceil(8);
        let legacy = if rice_bytes >= zero_run_bytes {
            (ENTROPY_ZERO_RUN, zero_run_bytes)
        } else {
            (ENTROPY_RICE_BASE + rice_parameter, rice_bytes)
        };
        if let Some(plan) = build_rans_plan(&self.histogram, self.folded.len()) {
            let extra_state_bytes = (RANS_INTERLEAVED_STATES - 1) * 4;
            let use_interleaved = extra_state_bytes * 1_000
                <= plan.modeled_bytes * RANS_INTERLEAVED_MAX_OVERHEAD_PER_MILLE;
            let (mode, bytes) = if use_interleaved {
                (ENTROPY_ORDER0_4, plan.modeled_bytes + extra_state_bytes)
            } else {
                (ENTROPY_ORDER0, plan.modeled_bytes)
            };
            if bytes < legacy.1 {
                return (mode, bytes);
            }
        }
        legacy
    }

    fn finish(mut self, prediction_mode: u8) -> EncodedTile {
        count_zero_run(&mut self.zero_run_bytes, &mut self.zero_run);
        let (rice_parameter, rice_bits) = best_rice_parameter(&self.histogram, self.folded.len());
        let rice_bytes = rice_bits.div_ceil(8);
        let legacy_bytes = rice_bytes.min(self.zero_run_bytes);
        if let Some(plan) = build_rans_plan(&self.histogram, self.folded.len()) {
            let extra_state_bytes = (RANS_INTERLEAVED_STATES - 1) * 4;
            let use_interleaved = extra_state_bytes * 1_000
                <= plan.modeled_bytes * RANS_INTERLEAVED_MAX_OVERHEAD_PER_MILLE;
            let (entropy_mode, payload) = if use_interleaved {
                (
                    ENTROPY_ORDER0_4,
                    encode_rans_payload_interleaved(&self.folded, &plan),
                )
            } else {
                (
                    ENTROPY_ORDER0,
                    encode_rans_payload_with_states::<1>(&self.folded, &plan),
                )
            };
            if payload.len() < legacy_bytes {
                return EncodedTile {
                    entropy_mode,
                    prediction_mode,
                    payload,
                };
            }
        }
        if rice_bytes >= self.zero_run_bytes {
            let mut zero_run_payload = Vec::with_capacity(self.zero_run_bytes);
            let mut zero_run = 0;
            for &folded in &self.folded {
                if folded == 0 {
                    zero_run += 1;
                } else {
                    flush_zero_run(&mut zero_run_payload, &mut zero_run);
                    put_varint(&mut zero_run_payload, u32::from(folded) * 2 - 1);
                }
            }
            flush_zero_run(&mut zero_run_payload, &mut zero_run);
            debug_assert_eq!(zero_run_payload.len(), self.zero_run_bytes);
            return EncodedTile {
                entropy_mode: ENTROPY_ZERO_RUN,
                prediction_mode,
                payload: zero_run_payload,
            };
        }

        let mut rice_payload = Vec::with_capacity(rice_bytes);
        let mut writer = BitWriter::new(&mut rice_payload);
        for folded in self.folded {
            writer.put_rice(u32::from(folded), rice_parameter);
        }
        writer.finish();
        debug_assert_eq!(rice_payload.len(), rice_bytes);
        EncodedTile {
            entropy_mode: ENTROPY_RICE_BASE + rice_parameter,
            prediction_mode,
            payload: rice_payload,
        }
    }
}

#[derive(Clone, Copy)]
struct RansSymbol {
    value: u16,
    frequency: u16,
    cumulative: u16,
}

struct RansPlan {
    table_log: u8,
    symbols: Vec<RansSymbol>,
    lookup: [u16; 511],
    modeled_bytes: usize,
}

fn build_rans_plan(histogram: &[u32; 511], sample_count: usize) -> Option<RansPlan> {
    let observed: Vec<(u16, u32)> = histogram
        .iter()
        .enumerate()
        .filter(|(_, count)| **count != 0)
        .map(|(symbol, &count)| (symbol as u16, count))
        .collect();
    let mut best = None;
    for table_log in RANS_MIN_TABLE_LOG..=RANS_MAX_TABLE_LOG {
        let Some(plan) = build_rans_plan_for_log(&observed, sample_count, table_log) else {
            continue;
        };
        if best.as_ref().is_none_or(|current: &RansPlan| {
            (plan.modeled_bytes, table_log) < (current.modeled_bytes, current.table_log)
        }) {
            best = Some(plan);
        }
    }
    best
}

fn build_rans_plan_for_log(
    observed: &[(u16, u32)],
    sample_count: usize,
    table_log: u8,
) -> Option<RansPlan> {
    let table_size = 1u32 << table_log;
    if observed.len() as u32 > table_size {
        return None;
    }
    let frequencies = normalize_rans_frequencies(observed, table_size, sample_count as u32);
    let mut symbols = Vec::with_capacity(observed.len());
    let mut lookup = [0u16; 511];
    let mut cumulative = 0u16;
    let mut table_bytes = 1 + varint_length(observed.len() as u32) + 4;
    let mut previous = 0u16;
    let mut payload_bits = 0.0;
    for (index, (&(value, count), &frequency)) in observed.iter().zip(&frequencies).enumerate() {
        let symbol_index = u16::try_from(index + 1).ok()?;
        lookup[value as usize] = symbol_index;
        symbols.push(RansSymbol {
            value,
            frequency,
            cumulative,
        });
        table_bytes += varint_length(u32::from(value - previous));
        if index + 1 != observed.len() {
            table_bytes += varint_length(u32::from(frequency));
        }
        payload_bits += f64::from(count) * (f64::from(table_size) / f64::from(frequency)).log2();
        cumulative = cumulative.checked_add(frequency)?;
        previous = value;
    }
    if u32::from(cumulative) != table_size {
        return None;
    }
    Some(RansPlan {
        table_log,
        symbols,
        lookup,
        modeled_bytes: table_bytes + (payload_bits / 8.0).ceil() as usize,
    })
}

fn normalize_rans_frequencies(
    observed: &[(u16, u32)],
    table_size: u32,
    sample_count: u32,
) -> Vec<u16> {
    let remaining = table_size - observed.len() as u32;
    let mut frequencies = Vec::with_capacity(observed.len());
    let mut remainders = Vec::with_capacity(observed.len());
    let mut assigned = 0u32;
    for &(value, count) in observed {
        let scaled = u64::from(count) * u64::from(remaining);
        let frequency = 1 + (scaled / u64::from(sample_count)) as u16;
        frequencies.push(frequency);
        assigned += u32::from(frequency);
        remainders.push((scaled % u64::from(sample_count), value));
    }
    remainders
        .sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    for &(_, value) in remainders.iter().take((table_size - assigned) as usize) {
        let index = observed
            .binary_search_by_key(&value, |(candidate, _)| *candidate)
            .expect("remainder symbol came from observed symbols");
        frequencies[index] += 1;
    }
    debug_assert_eq!(
        frequencies
            .iter()
            .map(|&value| u32::from(value))
            .sum::<u32>(),
        table_size
    );
    frequencies
}

fn rans_advance_encoder(state: u32, symbol: RansSymbol, table_log: u8) -> u32 {
    let frequency = u32::from(symbol.frequency);
    ((state / frequency) << table_log) + state % frequency + u32::from(symbol.cumulative)
}

#[cfg(test)]
fn encode_rans_payload(folded: &[u16], plan: &RansPlan) -> Vec<u8> {
    encode_rans_payload_with_states::<1>(folded, plan)
}

fn encode_rans_payload_interleaved(folded: &[u16], plan: &RansPlan) -> Vec<u8> {
    encode_rans_payload_with_states::<RANS_INTERLEAVED_STATES>(folded, plan)
}

// research/0030: independent rANS states expose instruction-level parallelism
// without changing the normalized table. The shared byte stream remains in
// raster decode order because reverse-raster renormalization bytes are
// reversed as one sequence after encoding.
fn encode_rans_payload_with_states<const STATES: usize>(
    folded: &[u16],
    plan: &RansPlan,
) -> Vec<u8> {
    debug_assert!(STATES.is_power_of_two());
    let mut states = [RANS_BYTE_L; STATES];
    let mut renormalized = Vec::new();
    for (index, &value) in folded.iter().enumerate().rev() {
        let state = &mut states[index & (STATES - 1)];
        let symbol = plan.symbols[usize::from(plan.lookup[value as usize] - 1)];
        let threshold =
            (u64::from(RANS_BYTE_L >> plan.table_log) << 8) * u64::from(symbol.frequency);
        while u64::from(*state) >= threshold {
            renormalized.push(*state as u8);
            *state >>= 8;
        }
        *state = rans_advance_encoder(*state, symbol, plan.table_log);
    }
    let mut payload = Vec::with_capacity(plan.modeled_bytes + (STATES - 1) * 4 + 8);
    payload.push(plan.table_log);
    put_varint(&mut payload, plan.symbols.len() as u32);
    let mut previous = 0u16;
    for (index, symbol) in plan.symbols.iter().enumerate() {
        put_varint(&mut payload, u32::from(symbol.value - previous));
        if index + 1 != plan.symbols.len() {
            put_varint(&mut payload, u32::from(symbol.frequency));
        }
        previous = symbol.value;
    }
    for state in states {
        put_u32(&mut payload, state);
    }
    payload.extend(renormalized.into_iter().rev());
    payload
}

fn encode_temporal_tile(
    plane: &Plane,
    reference: &Plane,
    tile: Tile,
    quantizer: &Quantizer,
) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut residuals = ResidualAccumulator::new(width * height);
    for y in 0..height {
        let start = (origin_y + y) * plane_width + origin_x;
        for (&sample, &prediction) in plane.data[start..start + width]
            .iter()
            .zip(&reference.data[start..start + width])
        {
            residuals.push(quantizer.quantize(i32::from(sample) - i32::from(prediction)));
        }
    }
    residuals.finish(PREDICT_TEMPORAL)
}

fn encode_spatial_tile(plane: &Plane, tile: Tile, quantizer: &Quantizer) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u8; width];
    let mut residuals = ResidualAccumulator::new(width * height);
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = 0;
        let mut upper_left = 0;
        for (&sample, reconstructed_slot) in plane.data[row_start..row_start + width]
            .iter()
            .zip(&mut reconstructed_row)
        {
            let above = *reconstructed_slot;
            let prediction = i32::from(paeth(left, above, upper_left));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed = (prediction + quantized * quantizer.step).clamp(0, 255) as u8;
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            residuals.push(quantized);
        }
    }
    residuals.finish(PREDICT_SPATIAL)
}

fn model_residual_mapping_tile(
    plane: &Plane,
    reference: Option<&Plane>,
    tile: Tile,
    quantizer: &Quantizer,
) -> TileResidualMappingModel {
    let current = encode_tile(plane, reference, tile, quantizer);
    let bounded = if let Some(reference) = reference {
        model_bounded_temporal_tile(plane, reference, tile, quantizer)
    } else {
        model_bounded_spatial_tile(plane, tile, quantizer)
    };
    TileResidualMappingModel {
        plane: tile.plane,
        width: tile.width,
        height: tile.height,
        temporal_prediction: reference.is_some(),
        source_zero_run: current.entropy_mode == ENTROPY_ZERO_RUN,
        bounded_zero_run: bounded.0 == ENTROPY_ZERO_RUN,
        sample_count: tile.width as usize * tile.height as usize,
        actual_payload_bytes: current.payload.len(),
        bounded_payload_bytes: bounded.1,
    }
}

#[derive(Clone, Copy)]
enum SpatialPredictor {
    Paeth,
    Average,
    ClampGradient,
    HalfGradient,
}

struct ModeledPredictor {
    summary: PredictorCandidateModel,
    reconstruction: Vec<u8>,
    #[cfg(test)]
    encoded: EncodedTile,
}

fn model_predictor_tile(
    plane: &Plane,
    available_reference: Option<&Plane>,
    current_reference: Option<&Plane>,
    tile: Tile,
    quantizer: &Quantizer,
) -> (TilePredictorModel, Vec<u8>) {
    let paeth = model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::Paeth);
    let average = model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::Average);
    let clamp_gradient =
        model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::ClampGradient);
    let half_gradient =
        model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::HalfGradient);
    let temporal = available_reference
        .map(|reference| model_temporal_predictor(plane, reference, tile, quantizer));
    let current_temporal = current_reference
        .map(|reference| model_temporal_predictor(plane, reference, tile, quantizer));
    let current_matches_candidate_reference = matches!((available_reference, current_reference), (Some(available), Some(current)) if available == current);
    let (current_mode, current) = if let Some(candidate) = &current_temporal {
        (PredictorModelMode::Temporal, candidate.summary)
    } else {
        (PredictorModelMode::Paeth, paeth.summary)
    };
    let candidates = [
        (PredictorModelMode::Paeth, paeth.summary),
        (PredictorModelMode::Average, average.summary),
        (PredictorModelMode::ClampGradient, clamp_gradient.summary),
        (PredictorModelMode::HalfGradient, half_gradient.summary),
    ];
    let minimum_bytes = candidates
        .iter()
        .map(|(_, candidate)| candidate.payload_bytes)
        .chain(
            temporal
                .as_ref()
                .map(|candidate| candidate.summary.payload_bytes),
        )
        .min()
        .expect("spatial candidates are nonempty");
    let current_is_oracle_candidate =
        current_mode == PredictorModelMode::Paeth || current_matches_candidate_reference;
    let (oracle_mode, oracle) =
        if current_is_oracle_candidate && current.payload_bytes == minimum_bytes {
            (current_mode, current)
        } else {
            candidates
                .into_iter()
                .chain(
                    temporal
                        .as_ref()
                        .map(|candidate| (PredictorModelMode::Temporal, candidate.summary)),
                )
                .filter(|(_, candidate)| candidate.payload_bytes == minimum_bytes)
                .min_by_key(|(_, candidate)| candidate.squared_error)
                .expect("a minimum candidate exists")
        };
    let reconstruction = match oracle_mode {
        PredictorModelMode::Paeth => paeth.reconstruction.clone(),
        PredictorModelMode::Average => average.reconstruction.clone(),
        PredictorModelMode::ClampGradient => clamp_gradient.reconstruction.clone(),
        PredictorModelMode::HalfGradient => half_gradient.reconstruction.clone(),
        PredictorModelMode::Temporal => temporal
            .as_ref()
            .expect("temporal oracle has a reference")
            .reconstruction
            .clone(),
    };
    (
        TilePredictorModel {
            plane: tile.plane,
            width: tile.width,
            height: tile.height,
            sample_count: tile.width as usize * tile.height as usize,
            current_mode,
            oracle_mode,
            current,
            oracle,
            paeth: paeth.summary,
            average: average.summary,
            clamp_gradient: clamp_gradient.summary,
            half_gradient: half_gradient.summary,
            temporal: temporal.map(|candidate| candidate.summary),
        },
        reconstruction,
    )
}

fn model_temporal_predictor(
    plane: &Plane,
    reference: &Plane,
    tile: Tile,
    quantizer: &Quantizer,
) -> ModeledPredictor {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut residuals = ResidualAccumulator::new(width * height);
    let mut reconstruction = Vec::with_capacity(width * height);
    let mut squared_error = 0u64;
    let mut max_error = 0u32;
    for y in 0..height {
        let start = (origin_y + y) * plane_width + origin_x;
        for (&sample, &prediction) in plane.data[start..start + width]
            .iter()
            .zip(&reference.data[start..start + width])
        {
            let prediction = i32::from(prediction);
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed = (prediction + quantized * quantizer.step).clamp(0, 255);
            let error = i64::from(sample) - i64::from(reconstructed);
            squared_error += (error * error) as u64;
            max_error = max_error.max(error.unsigned_abs() as u32);
            reconstruction.push(reconstructed as u8);
            residuals.push(quantized);
        }
    }
    candidate_model(
        residuals.finish(PREDICT_TEMPORAL),
        squared_error,
        max_error,
        reconstruction,
    )
}

fn model_spatial_predictor(
    plane: &Plane,
    tile: Tile,
    quantizer: &Quantizer,
    mode: SpatialPredictor,
) -> ModeledPredictor {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u8; width];
    let mut residuals = ResidualAccumulator::new(width * height);
    let mut reconstruction = Vec::with_capacity(width * height);
    let mut squared_error = 0u64;
    let mut max_error = 0u32;
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = 0;
        let mut upper_left = 0;
        for (&sample, reconstructed_slot) in plane.data[row_start..row_start + width]
            .iter()
            .zip(&mut reconstructed_row)
        {
            let above = *reconstructed_slot;
            let prediction = i32::from(spatial_prediction(mode, left, above, upper_left));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed = (prediction + quantized * quantizer.step).clamp(0, 255) as u8;
            let error = i64::from(sample) - i64::from(reconstructed);
            squared_error += (error * error) as u64;
            max_error = max_error.max(error.unsigned_abs() as u32);
            reconstruction.push(reconstructed);
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            residuals.push(quantized);
        }
    }
    candidate_model(
        residuals.finish(spatial_prediction_mode(mode)),
        squared_error,
        max_error,
        reconstruction,
    )
}

fn candidate_model(
    encoded: EncodedTile,
    squared_error: u64,
    max_error: u32,
    reconstruction: Vec<u8>,
) -> ModeledPredictor {
    ModeledPredictor {
        summary: PredictorCandidateModel {
            payload_bytes: encoded.payload.len(),
            squared_error,
            max_error,
            zero_run: encoded.entropy_mode == ENTROPY_ZERO_RUN,
        },
        reconstruction,
        #[cfg(test)]
        encoded,
    }
}

fn spatial_prediction_mode(mode: SpatialPredictor) -> u8 {
    match mode {
        SpatialPredictor::Paeth => PREDICT_SPATIAL,
        SpatialPredictor::Average => PREDICT_AVERAGE,
        SpatialPredictor::ClampGradient => PREDICT_CLAMP_GRADIENT,
        SpatialPredictor::HalfGradient => PREDICT_HALF_GRADIENT,
    }
}

fn spatial_prediction(mode: SpatialPredictor, left: u8, above: u8, upper_left: u8) -> u8 {
    match mode {
        SpatialPredictor::Paeth => paeth(left, above, upper_left),
        SpatialPredictor::Average => ((u16::from(left) + u16::from(above)) / 2) as u8,
        SpatialPredictor::ClampGradient => {
            (i32::from(left) + i32::from(above) - i32::from(upper_left)).clamp(0, 255) as u8
        }
        SpatialPredictor::HalfGradient => {
            let average = (i32::from(left) + i32::from(above)) / 2;
            (average + (average - i32::from(upper_left)) / 2).clamp(0, 255) as u8
        }
    }
}

fn model_bounded_temporal_tile(
    plane: &Plane,
    reference: &Plane,
    tile: Tile,
    quantizer: &Quantizer,
) -> (u8, usize) {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut folded = Vec::with_capacity(width * height);
    for y in 0..height {
        let start = (origin_y + y) * plane_width + origin_x;
        for (&sample, &prediction) in plane.data[start..start + width]
            .iter()
            .zip(&reference.data[start..start + width])
        {
            let prediction = i32::from(prediction);
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            folded.push(fold_bounded_residual(
                quantized,
                quantizer.quantize(-prediction),
                quantizer.quantize(i32::from(u8::MAX) - prediction),
            ) as u16);
        }
    }
    modeled_entropy_cost(&folded)
}

fn model_bounded_spatial_tile(plane: &Plane, tile: Tile, quantizer: &Quantizer) -> (u8, usize) {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u8; width];
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
            let prediction = i32::from(paeth(left, above, upper_left));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed = (prediction + quantized * quantizer.step).clamp(0, 255) as u8;
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            folded.push(fold_bounded_residual(
                quantized,
                quantizer.quantize(-prediction),
                quantizer.quantize(i32::from(u8::MAX) - prediction),
            ) as u16);
        }
    }
    modeled_entropy_cost(&folded)
}

fn modeled_entropy_cost(folded: &[u16]) -> (u8, usize) {
    let mut histogram = [0u32; 511];
    let mut zero_run_bytes = 0usize;
    let mut zero_run = 0u32;
    for &value in folded {
        histogram[value as usize] += 1;
        if value == 0 {
            zero_run += 1;
        } else {
            count_zero_run(&mut zero_run_bytes, &mut zero_run);
            zero_run_bytes += varint_length(u32::from(value) * 2 - 1);
        }
    }
    count_zero_run(&mut zero_run_bytes, &mut zero_run);
    let (parameter, rice_bits) = best_rice_parameter(&histogram, folded.len());
    let rice_bytes = rice_bits.div_ceil(8);
    if rice_bytes >= zero_run_bytes {
        (ENTROPY_ZERO_RUN, zero_run_bytes)
    } else {
        (ENTROPY_RICE_BASE + parameter, rice_bytes)
    }
}

fn model_folded_payload(
    payload: &[u8],
    sample_count: usize,
    entropy_mode: u8,
    max_folded: u32,
) -> Result<ByteFormatModel, CodecError> {
    let mut model = ByteFormatModel::default();
    if matches!(entropy_mode, ENTROPY_ORDER0 | ENTROPY_ORDER0_4) {
        let folded = if entropy_mode == ENTROPY_ORDER0 {
            decode_rans_symbols(payload, sample_count, max_folded)?
        } else {
            decode_rans_symbols_interleaved(payload, sample_count, max_folded)?
        };
        for folded in folded {
            model.push(folded);
        }
        return Ok(model);
    }
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
        let folded = reader.get_rice(entropy_mode - ENTROPY_RICE_BASE)?;
        if folded > max_folded {
            return Err(CodecError::Malformed("Rice residual is out of range"));
        }
        model.push(folded);
    }
    reader.finish()?;
    Ok(model)
}

fn decode_tile_payload(
    tile: Tile,
    payload: &[u8],
    step: i32,
    mode: u8,
    prediction_mode: u8,
    reference: Option<&Plane>,
) -> Result<Vec<u8>, CodecError> {
    let sample_count = checked_area(tile.width, tile.height)?;
    let width =
        usize::try_from(tile.width).map_err(|_| CodecError::LimitExceeded("tile is too large"))?;
    let prediction = PredictionContext {
        mode: prediction_mode,
        reference,
        tile,
    };
    if mode == ENTROPY_ZERO_RUN {
        return decode_zero_run_payload(payload, sample_count, width, step, prediction);
    }
    if matches!(mode, ENTROPY_ORDER0 | ENTROPY_ORDER0_4) {
        let folded = if mode == ENTROPY_ORDER0 {
            decode_rans_symbols(payload, sample_count, u32::from(u8::MAX) * 2)?
        } else {
            decode_rans_symbols_interleaved(payload, sample_count, u32::from(u8::MAX) * 2)?
        };
        let mut output = vec![0u8; sample_count];
        for (sample_index, value) in folded.into_iter().enumerate() {
            reconstruct_sample(
                &mut output,
                sample_index,
                width,
                unzigzag(value),
                step,
                prediction,
            )?;
        }
        return Ok(output);
    }
    if !(ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER).contains(&mode) {
        return Err(CodecError::Malformed("unknown tile entropy mode"));
    }
    decode_rice_payload(
        payload,
        sample_count,
        width,
        step,
        mode - ENTROPY_RICE_BASE,
        prediction,
    )
}

fn decode_rans_symbols(
    payload: &[u8],
    sample_count: usize,
    max_folded: u32,
) -> Result<Vec<u32>, CodecError> {
    decode_rans_symbols_with_states::<1>(payload, sample_count, max_folded)
}

fn decode_rans_symbols_interleaved(
    payload: &[u8],
    sample_count: usize,
    max_folded: u32,
) -> Result<Vec<u32>, CodecError> {
    decode_rans_symbols_with_states::<RANS_INTERLEAVED_STATES>(payload, sample_count, max_folded)
}

fn decode_rans_symbols_with_states<const STATES: usize>(
    payload: &[u8],
    sample_count: usize,
    max_folded: u32,
) -> Result<Vec<u32>, CodecError> {
    debug_assert!(STATES.is_power_of_two());
    let mut cursor = 0usize;
    let table_log = *payload
        .get(cursor)
        .ok_or(CodecError::Malformed("truncated order-0 table"))?;
    cursor += 1;
    if !(RANS_MIN_TABLE_LOG..=RANS_MAX_TABLE_LOG).contains(&table_log) {
        return Err(CodecError::Malformed("order-0 table log is out of range"));
    }
    let table_size = 1u32 << table_log;
    let symbol_count = usize::try_from(get_varint(payload, &mut cursor)?)
        .map_err(|_| CodecError::LimitExceeded("order-0 alphabet is too large"))?;
    if symbol_count == 0 || symbol_count > 511 || symbol_count > table_size as usize {
        return Err(CodecError::Malformed(
            "order-0 alphabet size is out of range",
        ));
    }
    let mut symbols = Vec::with_capacity(symbol_count);
    let mut previous = 0u32;
    let mut cumulative = 0u32;
    for index in 0..symbol_count {
        let delta = get_varint(payload, &mut cursor)?;
        if index != 0 && delta == 0 {
            return Err(CodecError::Malformed(
                "order-0 symbols are not strictly increasing",
            ));
        }
        let value = previous
            .checked_add(delta)
            .filter(|&value| value <= max_folded)
            .ok_or(CodecError::Malformed("order-0 symbol is out of range"))?;
        let frequency = if index + 1 == symbol_count {
            table_size
                .checked_sub(cumulative)
                .filter(|&frequency| frequency != 0)
                .ok_or(CodecError::Malformed(
                    "order-0 frequencies exceed the table",
                ))?
        } else {
            let frequency = get_varint(payload, &mut cursor)?;
            if frequency == 0
                || cumulative
                    .checked_add(frequency)
                    .is_none_or(|sum| sum >= table_size)
            {
                return Err(CodecError::Malformed("order-0 frequency is out of range"));
            }
            frequency
        };
        symbols.push(RansSymbol {
            value: value as u16,
            frequency: frequency as u16,
            cumulative: cumulative as u16,
        });
        cumulative += frequency;
        previous = value;
    }
    if cumulative != table_size {
        return Err(CodecError::Malformed(
            "order-0 frequencies do not fill the table",
        ));
    }
    let state_end = cursor
        .checked_add(4 * STATES)
        .filter(|&end| end <= payload.len())
        .ok_or(CodecError::Malformed("truncated order-0 state"))?;
    let mut states = [RANS_BYTE_L; STATES];
    for (index, state) in states.iter_mut().enumerate() {
        *state = get_u32(payload, cursor + index * 4)?;
        if *state < RANS_BYTE_L {
            return Err(CodecError::Malformed("invalid order-0 final state"));
        }
    }
    cursor = state_end;
    let mut decoding_table = Vec::with_capacity(table_size as usize);
    for symbol in &symbols {
        decoding_table.extend(std::iter::repeat_n(*symbol, usize::from(symbol.frequency)));
    }
    if decoding_table.len() != table_size as usize {
        return Err(CodecError::Malformed(
            "order-0 frequencies do not fill the table",
        ));
    }

    let mut output = Vec::with_capacity(sample_count);
    let mut sample_index = 0usize;
    if STATES == RANS_INTERLEAVED_STATES {
        while sample_index + RANS_INTERLEAVED_STATES <= sample_count {
            let slots = [
                states[0] & (table_size - 1),
                states[1] & (table_size - 1),
                states[2] & (table_size - 1),
                states[3] & (table_size - 1),
            ];
            let decoded = [
                decoding_table[slots[0] as usize],
                decoding_table[slots[1] as usize],
                decoding_table[slots[2] as usize],
                decoding_table[slots[3] as usize],
            ];
            output.push(u32::from(decoded[0].value));
            output.push(u32::from(decoded[1].value));
            output.push(u32::from(decoded[2].value));
            output.push(u32::from(decoded[3].value));
            let next = [
                rans_decoder_next(states[0], decoded[0], slots[0], table_log),
                rans_decoder_next(states[1], decoded[1], slots[1], table_log),
                rans_decoder_next(states[2], decoded[2], slots[2], table_log),
                rans_decoder_next(states[3], decoded[3], slots[3], table_log),
            ];
            if next.iter().any(|&state| state > u64::from(u32::MAX)) {
                return Err(CodecError::Malformed("order-0 state overflow"));
            }
            states[0] = next[0] as u32;
            states[1] = next[1] as u32;
            states[2] = next[2] as u32;
            states[3] = next[3] as u32;
            rans_renormalize(&mut states[0], payload, &mut cursor)?;
            rans_renormalize(&mut states[1], payload, &mut cursor)?;
            rans_renormalize(&mut states[2], payload, &mut cursor)?;
            rans_renormalize(&mut states[3], payload, &mut cursor)?;
            sample_index += RANS_INTERLEAVED_STATES;
        }
    }
    while sample_index < sample_count {
        let state = &mut states[sample_index & (STATES - 1)];
        let slot = *state & (table_size - 1);
        let symbol = decoding_table[slot as usize];
        output.push(u32::from(symbol.value));
        *state = rans_advance_decoder(*state, symbol, slot, table_log)?;
        rans_renormalize(state, payload, &mut cursor)?;
        sample_index += 1;
    }
    if cursor != payload.len() {
        return Err(CodecError::Malformed("trailing order-0 payload bytes"));
    }
    if states.iter().any(|&state| state != RANS_BYTE_L) {
        return Err(CodecError::Malformed("noncanonical order-0 initial state"));
    }
    Ok(output)
}

#[inline]
fn rans_advance_decoder(
    state: u32,
    symbol: RansSymbol,
    slot: u32,
    table_log: u8,
) -> Result<u32, CodecError> {
    let next = rans_decoder_next(state, symbol, slot, table_log);
    u32::try_from(next).map_err(|_| CodecError::Malformed("order-0 state overflow"))
}

#[inline]
fn rans_decoder_next(state: u32, symbol: RansSymbol, slot: u32, table_log: u8) -> u64 {
    u64::from(symbol.frequency) * u64::from(state >> table_log)
        + u64::from(slot - u32::from(symbol.cumulative))
}

#[inline]
fn rans_renormalize(state: &mut u32, payload: &[u8], cursor: &mut usize) -> Result<(), CodecError> {
    while *state < RANS_BYTE_L {
        let byte = *payload
            .get(*cursor)
            .ok_or(CodecError::Malformed("truncated order-0 payload"))?;
        *cursor += 1;
        *state = (*state << 8) | u32::from(byte);
    }
    Ok(())
}

fn decode_zero_run_payload(
    payload: &[u8],
    sample_count: usize,
    width: usize,
    step: i32,
    prediction: PredictionContext<'_>,
) -> Result<Vec<u8>, CodecError> {
    let mut output = vec![0u8; sample_count];
    let mut cursor = 0usize;
    let mut sample_index = 0usize;
    while sample_index < sample_count {
        let token = get_varint(payload, &mut cursor)?;
        if token & 1 == 0 {
            let run = usize::try_from(token / 2)
                .ok()
                .and_then(|run| run.checked_add(1))
                .ok_or(CodecError::Malformed("zero run is too long"))?;
            let run_end = sample_index
                .checked_add(run)
                .ok_or(CodecError::Malformed("zero run is too long"))?;
            if run_end > sample_count {
                return Err(CodecError::Malformed("zero run exceeds tile"));
            }
            if prediction.mode == PREDICT_TEMPORAL {
                copy_temporal_zero_run(&mut output, sample_index, run_end, width, prediction)?;
                sample_index = run_end;
            } else {
                reconstruct_spatial_zero_run(
                    &mut output,
                    sample_index,
                    run_end,
                    width,
                    prediction.mode,
                )?;
                sample_index = run_end;
            }
        } else {
            let zigzag_value = token.div_ceil(2);
            let quantized = unzigzag(zigzag_value);
            if quantized == 0 {
                return Err(CodecError::Malformed("non-canonical zero residual"));
            }
            reconstruct_sample(
                &mut output,
                sample_index,
                width,
                quantized,
                step,
                prediction,
            )?;
            sample_index += 1;
        }
    }
    if cursor != payload.len() {
        return Err(CodecError::Malformed("trailing tile payload bytes"));
    }
    Ok(output)
}

fn reconstruct_spatial_zero_run(
    output: &mut [u8],
    start: usize,
    end: usize,
    width: usize,
    prediction_mode: u8,
) -> Result<(), CodecError> {
    match spatial_predictor_from_mode(prediction_mode)? {
        SpatialPredictor::Paeth => {
            reconstruct_spatial_zero_run_with(output, start, end, width, paeth)
        }
        SpatialPredictor::Average => {
            reconstruct_spatial_zero_run_with(output, start, end, width, |left, above, _| {
                ((u16::from(left) + u16::from(above)) / 2) as u8
            })
        }
        SpatialPredictor::ClampGradient => reconstruct_spatial_zero_run_with(
            output,
            start,
            end,
            width,
            |left, above, upper_left| {
                (i32::from(left) + i32::from(above) - i32::from(upper_left)).clamp(0, 255) as u8
            },
        ),
        SpatialPredictor::HalfGradient => reconstruct_spatial_zero_run_with(
            output,
            start,
            end,
            width,
            |left, above, upper_left| {
                let average = (i32::from(left) + i32::from(above)) / 2;
                (average + (average - i32::from(upper_left)) / 2).clamp(0, 255) as u8
            },
        ),
    }
    Ok(())
}

fn reconstruct_spatial_zero_run_with(
    output: &mut [u8],
    mut start: usize,
    end: usize,
    width: usize,
    predict: impl Fn(u8, u8, u8) -> u8,
) {
    while start < end {
        let x = start % width;
        let y = start / width;
        let span = (width - x).min(end - start);
        for offset in 0..span {
            let index = start + offset;
            let column = x + offset;
            let left = if column > 0 { output[index - 1] } else { 0 };
            let above = if y > 0 { output[index - width] } else { 0 };
            let upper_left = if column > 0 && y > 0 {
                output[index - width - 1]
            } else {
                0
            };
            output[index] = predict(left, above, upper_left);
        }
        start += span;
    }
}

fn copy_temporal_zero_run(
    output: &mut [u8],
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
        let reference_start =
            (context.tile.y as usize + y) * plane_width + context.tile.x as usize + x;
        output[start..start + span]
            .copy_from_slice(&reference.data[reference_start..reference_start + span]);
        start += span;
    }
    Ok(())
}

fn decode_rice_payload(
    payload: &[u8],
    sample_count: usize,
    width: usize,
    step: i32,
    parameter: u8,
    prediction: PredictionContext<'_>,
) -> Result<Vec<u8>, CodecError> {
    let mut output = vec![0u8; sample_count];
    let mut reader = BitReader::new(payload);
    for sample_index in 0..sample_count {
        let folded = reader.get_rice(parameter)?;
        if folded > 510 {
            return Err(CodecError::Malformed("Rice residual is out of range"));
        }
        reconstruct_sample(
            &mut output,
            sample_index,
            width,
            unzigzag(folded),
            step,
            prediction,
        )?;
    }
    reader.finish()?;
    Ok(output)
}

fn reconstruct_sample(
    output: &mut [u8],
    index: usize,
    width: usize,
    quantized: i32,
    step: i32,
    context: PredictionContext<'_>,
) -> Result<(), CodecError> {
    let x = index % width;
    let y = index / width;
    let left = if x > 0 { output[index - 1] } else { 0 };
    let above = if y > 0 { output[index - width] } else { 0 };
    let upper_left = if x > 0 && y > 0 {
        output[index - width - 1]
    } else {
        0
    };
    let prediction = match context.mode {
        PREDICT_TEMPORAL => {
            let reference = context.reference.ok_or(CodecError::InvalidInput(
                "predicted frame requires a reference",
            ))?;
            let plane_width = reference.width as usize;
            let reference_index =
                (context.tile.y as usize + y) * plane_width + context.tile.x as usize + x;
            i32::from(reference.data[reference_index])
        }
        mode => i32::from(spatial_prediction(
            spatial_predictor_from_mode(mode)?,
            left,
            above,
            upper_left,
        )),
    };
    output[index] = (prediction + quantized * step).clamp(0, 255) as u8;
    Ok(())
}

fn spatial_predictor_from_mode(mode: u8) -> Result<SpatialPredictor, CodecError> {
    match mode {
        PREDICT_SPATIAL => Ok(SpatialPredictor::Paeth),
        PREDICT_AVERAGE => Ok(SpatialPredictor::Average),
        PREDICT_CLAMP_GRADIENT => Ok(SpatialPredictor::ClampGradient),
        PREDICT_HALF_GRADIENT => Ok(SpatialPredictor::HalfGradient),
        _ => Err(CodecError::Malformed("unknown tile prediction mode")),
    }
}

fn paeth(left: u8, above: u8, upper_left: u8) -> u8 {
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

fn quantization_step(quality: u8) -> i32 {
    1 + i32::from((100 - quality) / 5)
}

fn zigzag(value: i32) -> u32 {
    if value >= 0 {
        (value as u32) * 2
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

fn best_rice_parameter(histogram: &[u32; 511], sample_count: usize) -> (u8, usize) {
    let mut best_parameter = 0;
    let mut best_bits = usize::MAX;
    for parameter in 0..=MAX_RICE_PARAMETER {
        let mut bits = sample_count * (usize::from(parameter) + 1);
        for (value, &frequency) in histogram.iter().enumerate().skip(1) {
            bits += (value >> parameter) * frequency as usize;
        }
        if bits < best_bits {
            best_parameter = parameter;
            best_bits = bits;
        }
    }
    (best_parameter, best_bits)
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
        let quotient = value >> parameter;
        let code_bits = quotient + 1 + u32::from(parameter);
        if code_bits <= u32::from(64 - self.buffered_bits) {
            let remainder_mask = (1u64 << parameter) - 1;
            let code = (1u64 << quotient) | ((u64::from(value) & remainder_mask) << (quotient + 1));
            self.buffer |= code << self.buffered_bits;
            self.buffered_bits += code_bits as u8;
            self.flush_bytes();
            return;
        }
        self.put_zeros(quotient);
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
        debug_assert!(count <= 8);
        if count != 0 {
            let mask = (1u64 << count) - 1;
            self.buffer |= (u64::from(value) & mask) << self.buffered_bits;
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

    fn get_rice(&mut self, parameter: u8) -> Result<u32, CodecError> {
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
            quotient += zeros;
            if quotient > 510 {
                return Err(CodecError::Malformed("Rice quotient is out of range"));
            }
            self.consume(zeros as u8);
            if self.buffered_bits != 0 {
                self.consume(1);
                break;
            }
        }
        let remainder = self.get_bits(parameter)?;
        quotient
            .checked_shl(u32::from(parameter))
            .and_then(|value| value.checked_add(remainder))
            .ok_or(CodecError::Malformed("Rice residual overflow"))
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
            let encoded_length = *cursor - start;
            let canonical_length = if value < (1 << 7) {
                1
            } else if value < (1 << 14) {
                2
            } else if value < (1 << 21) {
                3
            } else if value < (1 << 28) {
                4
            } else {
                5
            };
            if encoded_length != canonical_length {
                return Err(CodecError::Malformed("non-canonical variable integer"));
            }
            return Ok(value);
        }
    }
    Err(CodecError::Malformed("variable integer overflow"))
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
                    let value = operation(index);
                    output.lock().expect("worker output lock poisoned")[index] = Some(value);
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
    let bytes: [u8; 2] = input
        .get(offset..offset + 2)
        .ok_or(CodecError::Malformed("truncated integer"))?
        .try_into()
        .expect("slice has exact length");
    Ok(u16::from_le_bytes(bytes))
}

fn get_u32(input: &[u8], offset: usize) -> Result<u32, CodecError> {
    let bytes: [u8; 4] = input
        .get(offset..offset + 4)
        .ok_or(CodecError::Malformed("truncated integer"))?
        .try_into()
        .expect("slice has exact length");
    Ok(u32::from_le_bytes(bytes))
}

fn get_u64(input: &[u8], offset: usize) -> Result<u64, CodecError> {
    let bytes: [u8; 8] = input
        .get(offset..offset + 8)
        .ok_or(CodecError::Malformed("truncated integer"))?
        .try_into()
        .expect("slice has exact length");
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rice_bytes(value: u32, parameter: u8, alignment: u8, fused: bool) -> Vec<u8> {
        let mut output = Vec::new();
        let mut writer = BitWriter::new(&mut output);
        writer.put_bits((1u32 << alignment) - 1, alignment);
        if fused {
            writer.put_rice(value, parameter);
        } else {
            writer.put_zeros(value >> parameter);
            writer.put_bits(1, 1);
            writer.put_bits(value, parameter);
        }
        writer.finish();
        output
    }

    fn patterned_frame(width: u32, height: u32) -> Frame {
        let luma: Vec<u8> = (0..width * height)
            .map(|index| {
                let x = index % width;
                let y = index / width;
                ((x * 3 + y * 5 + (x / 7) * 11) & 255) as u8
            })
            .collect();
        let chroma_width = width.div_ceil(2);
        let cb: Vec<u8> = (0..chroma_width * height)
            .map(|index| (96 + (index % chroma_width) % 41) as u8)
            .collect();
        let cr: Vec<u8> = (0..chroma_width * height)
            .map(|index| (160 - (index / chroma_width) % 37) as u8)
            .collect();
        Frame::yuv422p8(width, height, FrameRate::new(24_000, 1_001), luma, cb, cr).unwrap()
    }

    fn encode_spatial_tile_full_reference(
        plane: &Plane,
        tile: Tile,
        quantizer: &Quantizer,
    ) -> EncodedTile {
        let width = tile.width as usize;
        let height = tile.height as usize;
        let plane_width = plane.width as usize;
        let origin_x = tile.x as usize;
        let origin_y = tile.y as usize;
        let mut reconstructed = vec![0u8; width * height];
        let mut residuals = ResidualAccumulator::new(width * height);
        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;
                let left = if x > 0 { reconstructed[index - 1] } else { 0 };
                let above = if y > 0 {
                    reconstructed[index - width]
                } else {
                    0
                };
                let upper_left = if x > 0 && y > 0 {
                    reconstructed[index - width - 1]
                } else {
                    0
                };
                let plane_index = (origin_y + y) * plane_width + origin_x + x;
                let prediction = i32::from(paeth(left, above, upper_left));
                let sample = i32::from(plane.data[plane_index]);
                let quantized = quantizer.quantize(sample - prediction);
                reconstructed[index] =
                    (prediction + quantized * quantizer.step).clamp(0, 255) as u8;
                residuals.push(quantized);
            }
        }
        residuals.finish(PREDICT_SPATIAL)
    }

    #[test]
    fn stream_info_mode_counts_cover_every_tile() {
        let frame = patterned_frame(513, 257);
        let options = CodecOptions {
            quality: 100,
            threads: 2,
            ..CodecOptions::default()
        };
        let intra_bytes = encode(&frame, options).unwrap();
        let intra = inspect(&intra_bytes).unwrap();
        assert_eq!(intra.zero_run_tiles + intra.rice_tiles, intra.tile_count);
        assert_eq!(intra.spatial_tiles + intra.temporal_tiles, intra.tile_count);
        assert_eq!(intra.spatial_tiles, intra.tile_count);
        let models = analyze_entropy(&intra_bytes).unwrap();
        assert_eq!(models.len(), intra.tile_count);
        assert_eq!(
            models.iter().map(|model| model.sample_count).sum::<usize>(),
            frame.raw_len()
        );
        assert_eq!(
            models
                .iter()
                .map(|model| model.actual_payload_bytes)
                .sum::<usize>(),
            intra_bytes.len() - HEADER_LEN - intra.tile_count * DIRECTORY_ENTRY_LEN
        );

        let temporal_bytes = encode_with_reference(&frame, &frame, options).unwrap();
        let temporal = inspect(&temporal_bytes).unwrap();
        assert_eq!(
            temporal.zero_run_tiles + temporal.rice_tiles,
            temporal.tile_count
        );
        assert_eq!(temporal.temporal_tiles, temporal.tile_count);
        let models = analyze_entropy(&temporal_bytes).unwrap();
        assert!(models.iter().all(|model| model.temporal_prediction));
        assert!(models.iter().all(|model| model.source_zero_run));
    }

    #[test]
    fn quantizer_table_matches_scalar_for_every_codec_residual() {
        for step in 1..=20 {
            let table = Quantizer::new(step);
            for residual in -255..=255 {
                assert_eq!(table.quantize(residual), quantize(residual, step));
            }
        }
    }

    #[test]
    fn rolling_spatial_row_matches_full_reconstruction() {
        let frame = patterned_frame(37, 23);
        let tile = Tile {
            plane: 0,
            x: 3,
            y: 2,
            width: 31,
            height: 19,
        };
        for quality in [90, 100] {
            let quantizer = Quantizer::new(quantization_step(quality));
            let rolling = encode_spatial_tile(&frame.planes[0], tile, &quantizer);
            let reference = encode_spatial_tile_full_reference(&frame.planes[0], tile, &quantizer);
            assert_eq!(rolling.entropy_mode, reference.entropy_mode);
            assert_eq!(rolling.prediction_mode, reference.prediction_mode);
            assert_eq!(rolling.payload, reference.payload);
        }
    }

    #[test]
    fn quality_100_round_trips_odd_dimensions() {
        let frame = patterned_frame(37, 23);
        let options = CodecOptions {
            quality: 100,
            tile_width: 13,
            tile_height: 11,
            threads: 3,
        };
        let encoded = encode(&frame, options).unwrap();
        let decoded = decode(&encoded, 4).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn lossy_error_is_bounded_by_quantizer() {
        let frame = patterned_frame(65, 33);
        let options = CodecOptions {
            quality: 90,
            tile_width: 16,
            tile_height: 15,
            threads: 2,
        };
        let decoded = decode(&encode(&frame, options).unwrap(), 2).unwrap();
        for (expected, actual) in frame.planes.iter().zip(&decoded.planes) {
            for (&expected, &actual) in expected.data.iter().zip(&actual.data) {
                assert!(expected.abs_diff(actual) <= 1);
            }
        }
    }

    #[test]
    fn individual_tile_matches_full_decode() {
        let frame = patterned_frame(40, 25);
        let options = CodecOptions {
            quality: 95,
            tile_width: 16,
            tile_height: 10,
            threads: 2,
        };
        let encoded = encode(&frame, options).unwrap();
        let full = decode(&encoded, 2).unwrap();
        let tile = decode_tile(&encoded, 1).unwrap();
        let plane = &full.planes[tile.plane];
        for row in 0..tile.height as usize {
            let full_start = (tile.y as usize + row) * plane.width as usize + tile.x as usize;
            let tile_start = row * tile.width as usize;
            assert_eq!(
                &tile.data[tile_start..tile_start + tile.width as usize],
                &plane.data[full_start..full_start + tile.width as usize]
            );
        }
    }

    #[test]
    fn interleaved_order0_tile_matches_full_decode() {
        let mut state = 0x9e37_79b9u32;
        let samples = (0..256 * 128)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 30) * 85) as u8
            })
            .collect();
        let frame = Frame::gray8(256, 128, FrameRate::new(24, 1), samples).unwrap();
        let encoded = encode(
            &frame,
            CodecOptions {
                quality: 100,
                tile_width: 256,
                tile_height: 128,
                threads: 1,
            },
        )
        .unwrap();
        assert_eq!(encoded[HEADER_LEN + 1], ENTROPY_ORDER0_4);
        let full = decode(&encoded, 1).unwrap();
        let tile = decode_tile(&encoded, 0).unwrap();
        assert_eq!(tile.data, full.planes[0].data);
        assert_eq!(full, frame);
    }

    #[test]
    fn malformed_streams_are_rejected() {
        let frame = patterned_frame(16, 16);
        let encoded = encode(&frame, CodecOptions::default()).unwrap();
        assert!(decode(&encoded[..encoded.len() - 1], 1).is_err());

        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode(&trailing, 1).is_err());

        let mut bad_magic = encoded;
        bad_magic[0] = b'X';
        assert!(decode(&bad_magic, 1).is_err());

        let one_pixel =
            Frame::gray8(1, 1, FrameRate::new(24, 1), vec![0]).expect("valid one-pixel frame");
        let mut overlong_run = encode(&one_pixel, CodecOptions::default()).unwrap();
        let payload_offset = HEADER_LEN + DIRECTORY_ENTRY_LEN;
        assert_eq!(overlong_run[payload_offset], 0);
        overlong_run[payload_offset] = 2;
        assert!(decode(&overlong_run, 1).is_err());
    }

    #[test]
    fn zigzag_is_a_bijection_for_codec_residuals() {
        for value in -255..=255 {
            assert_eq!(unzigzag(zigzag(value)), value);
        }
    }

    #[test]
    fn adaptive_rice_mode_round_trips_dense_residuals() {
        let mut state = 1u32;
        let data: Vec<u8> = (0..64 * 64)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();
        let frame = Frame::gray8(64, 64, FrameRate::new(24, 1), data).unwrap();
        let options = CodecOptions {
            quality: 100,
            tile_width: 64,
            tile_height: 64,
            threads: 1,
        };
        let encoded = encode(&frame, options).unwrap();
        assert!(
            (ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER)
                .contains(&encoded[HEADER_LEN + 1])
        );
        assert_eq!(decode(&encoded, 1).unwrap(), frame);
    }

    #[test]
    fn fused_rice_writer_matches_reference_exhaustively() {
        for parameter in 0..=MAX_RICE_PARAMETER {
            for value in 0..=510 {
                for alignment in 0..8 {
                    assert_eq!(
                        rice_bytes(value, parameter, alignment, true),
                        rice_bytes(value, parameter, alignment, false)
                    );
                }
            }
        }
    }

    #[test]
    fn rice_padding_and_unknown_modes_are_rejected() {
        let mut payload = Vec::new();
        let mut writer = BitWriter::new(&mut payload);
        for value in [0, 1, 2, 510] {
            writer.put_rice(value, 3);
        }
        writer.finish();
        let mut reader = BitReader::new(&payload);
        for expected in [0, 1, 2, 510] {
            assert_eq!(reader.get_rice(3).unwrap(), expected);
        }
        reader.finish().unwrap();

        *payload.last_mut().unwrap() |= 0x80;
        let mut reader = BitReader::new(&payload);
        for _ in 0..4 {
            reader.get_rice(3).unwrap();
        }
        assert!(reader.finish().is_err());

        let frame = patterned_frame(16, 16);
        let mut encoded = encode(&frame, CodecOptions::default()).unwrap();
        encoded[HEADER_LEN + 1] = ENTROPY_ORDER0_4 + 1;
        assert!(decode(&encoded, 1).is_err());
    }

    #[test]
    fn order0_payload_round_trips_sparse_and_extreme_alphabets() {
        let cases = [
            vec![0u16; 32_768],
            (0..=510).collect(),
            (0..32_768)
                .map(|index| {
                    if index % 97 == 0 {
                        510
                    } else if index % 7 == 0 {
                        3
                    } else {
                        0
                    }
                })
                .collect(),
            (0..32_768)
                .map(|index| ((index * 73 + index / 11) % 511) as u16)
                .collect(),
        ];
        for folded in cases {
            let mut histogram = [0u32; 511];
            for &value in &folded {
                histogram[value as usize] += 1;
            }
            let plan = build_rans_plan(&histogram, folded.len()).unwrap();
            let payload = encode_rans_payload(&folded, &plan);
            assert!(payload.len().abs_diff(plan.modeled_bytes) <= 4);
            let observed: Vec<(u16, u32)> = histogram
                .iter()
                .enumerate()
                .filter(|(_, count)| **count != 0)
                .map(|(symbol, &count)| (symbol as u16, count))
                .collect();
            let exact_minimum = (RANS_MIN_TABLE_LOG..=RANS_MAX_TABLE_LOG)
                .filter_map(|table_log| build_rans_plan_for_log(&observed, folded.len(), table_log))
                .map(|candidate| encode_rans_payload(&folded, &candidate).len())
                .min()
                .unwrap();
            assert!(payload.len() <= exact_minimum + 4);
            assert_eq!(
                decode_rans_symbols(&payload, folded.len(), 510).unwrap(),
                folded
                    .iter()
                    .map(|&value| u32::from(value))
                    .collect::<Vec<_>>()
            );

            let interleaved = encode_rans_payload_interleaved(&folded, &plan);
            assert!(interleaved.len().abs_diff(plan.modeled_bytes + 12) <= 4);
            assert_eq!(
                decode_rans_symbols_interleaved(&interleaved, folded.len(), 510).unwrap(),
                folded
                    .iter()
                    .map(|&value| u32::from(value))
                    .collect::<Vec<_>>()
            );

            let mut trailing = payload.clone();
            trailing.push(0);
            assert!(decode_rans_symbols(&trailing, folded.len(), 510).is_err());
            for end in 0..payload.len().min(16) {
                assert!(decode_rans_symbols(&payload[..end], folded.len(), 510).is_err());
            }
            let mut interleaved_trailing = interleaved.clone();
            interleaved_trailing.push(0);
            assert!(
                decode_rans_symbols_interleaved(&interleaved_trailing, folded.len(), 510).is_err()
            );
            for end in 0..interleaved.len().min(28) {
                assert!(
                    decode_rans_symbols_interleaved(&interleaved[..end], folded.len(), 510)
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn modeled_order0_log_tracks_exact_synthetic_minimum() {
        let mut mismatches = 0usize;
        let mut maximum_overhead = 0usize;
        let cases = 256usize;
        for case in 0..cases {
            let sample_count = 257 + case * 97 % 4096;
            let mut state = 0x9e37_79b9u32 ^ case as u32;
            let mut folded = Vec::with_capacity(sample_count);
            for index in 0..sample_count {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let value = match case % 4 {
                    0 => state as usize % 511,
                    1 => match state % 100 {
                        0..=69 => 0,
                        70..=89 => 1,
                        90..=96 => 2,
                        _ => 3 + state as usize % 32,
                    },
                    2 => state as usize % (2 + case % 127),
                    _ => (index / (1 + case % 31) + state as usize % 3) % 511,
                };
                folded.push(value as u16);
            }
            let mut histogram = [0u32; 511];
            for &value in &folded {
                histogram[value as usize] += 1;
            }
            let modeled = build_rans_plan(&histogram, folded.len()).unwrap();
            let modeled_bytes = encode_rans_payload(&folded, &modeled).len();
            let observed: Vec<(u16, u32)> = histogram
                .iter()
                .enumerate()
                .filter(|(_, count)| **count != 0)
                .map(|(symbol, &count)| (symbol as u16, count))
                .collect();
            let (exact_bytes, exact_log) = (RANS_MIN_TABLE_LOG..=RANS_MAX_TABLE_LOG)
                .filter_map(|table_log| {
                    build_rans_plan_for_log(&observed, folded.len(), table_log)
                        .map(|plan| (encode_rans_payload(&folded, &plan).len(), table_log))
                })
                .min()
                .unwrap();
            mismatches += usize::from(modeled.table_log != exact_log);
            maximum_overhead = maximum_overhead.max(modeled_bytes - exact_bytes);
        }
        println!(
            "synthetic cases={cases} log_mismatches={mismatches} maximum_overhead={maximum_overhead}"
        );
        assert!(maximum_overhead <= 4);
    }

    #[test]
    fn order0_table_validation_rejects_noncanonical_inputs() {
        for payload in [
            vec![RANS_MIN_TABLE_LOG - 1],
            vec![RANS_MAX_TABLE_LOG + 1],
            vec![RANS_MIN_TABLE_LOG, 0],
            vec![RANS_MIN_TABLE_LOG, 2, 0, 1, 0],
            vec![RANS_MIN_TABLE_LOG, 1, 0, 0, 0, 0, 0],
        ] {
            assert!(decode_rans_symbols(&payload, 1, 510).is_err());
        }
    }

    #[test]
    fn rice_codes_round_trip_every_codec_residual() {
        for parameter in 0..=MAX_RICE_PARAMETER {
            let mut payload = Vec::new();
            let mut writer = BitWriter::new(&mut payload);
            for value in 0..=510 {
                writer.put_rice(value, parameter);
            }
            writer.finish();

            let mut reader = BitReader::new(&payload);
            for expected in 0..=510 {
                assert_eq!(reader.get_rice(parameter).unwrap(), expected);
            }
            reader.finish().unwrap();
        }
    }

    #[test]
    fn temporal_frames_require_reference_and_round_trip_exactly() {
        let reference = patterned_frame(65, 33);
        let mut frame = reference.clone();
        for (plane_index, plane) in frame.planes.iter_mut().enumerate() {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if (index + plane_index) % 17 == 0 {
                    *sample = sample.saturating_add(3);
                }
            }
        }
        let options = CodecOptions {
            quality: 100,
            tile_width: 16,
            tile_height: 15,
            threads: 2,
        };
        let encoded = encode_with_reference(&frame, &reference, options).unwrap();
        assert_eq!(encoded[HEADER_LEN + 2], PREDICT_TEMPORAL);
        assert!(decode(&encoded, 2).is_err());
        assert_eq!(
            decode_with_reference(&encoded, &reference, 2).unwrap(),
            frame
        );

        let wrong_reference = patterned_frame(64, 33);
        assert!(decode_with_reference(&encoded, &wrong_reference, 1).is_err());
        assert!(encode_with_reference(&frame, &wrong_reference, options).is_err());
    }

    #[test]
    fn unknown_prediction_modes_are_rejected() {
        let frame = patterned_frame(16, 16);
        let mut encoded = encode(&frame, CodecOptions::default()).unwrap();
        encoded[HEADER_LEN + 2] = MAX_PREDICTION_MODE + 1;
        assert!(decode(&encoded, 1).is_err());
    }

    #[test]
    fn high_motion_frames_fall_back_to_spatial_prediction() {
        let reference = patterned_frame(32, 24);
        let mut frame = reference.clone();
        for plane in &mut frame.planes {
            for sample in &mut plane.data {
                *sample = 255 - *sample;
            }
        }
        let options = CodecOptions {
            quality: 100,
            ..CodecOptions::default()
        };
        let encoded = encode_with_reference(&frame, &reference, options).unwrap();
        assert_ne!(encoded[HEADER_LEN + 2], PREDICT_TEMPORAL);
        assert_eq!(decode(&encoded, 1).unwrap(), frame);
    }

    #[test]
    fn residual_mapping_model_matches_current_tile_payloads() {
        let reference = patterned_frame(65, 33);
        let mut frame = reference.clone();
        for plane in &mut frame.planes {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if index % 31 == 0 {
                    *sample = sample.saturating_add(1);
                }
            }
        }
        let options = CodecOptions {
            quality: 90,
            tile_width: 16,
            tile_height: 15,
            threads: 2,
        };
        for reference in [None, Some(&reference)] {
            let selected =
                reference.filter(|reference| temporal_prediction_is_promising(&frame, reference));
            let tiles = expected_tiles(
                frame.width,
                frame.height,
                frame.format,
                options.tile_width,
                options.tile_height,
            )
            .unwrap();
            let quantizer = Quantizer::new(options.quantization_step());
            let models = analyze_residual_mapping(&frame, reference, options).unwrap();
            for (model, tile) in models.iter().zip(&tiles) {
                let expected = encode_tile(
                    &frame.planes[tile.plane],
                    selected.map(|frame| &frame.planes[tile.plane]),
                    *tile,
                    &quantizer,
                );
                assert_eq!(model.actual_payload_bytes, expected.payload.len());
                assert_eq!(
                    model.source_zero_run,
                    expected.entropy_mode == ENTROPY_ZERO_RUN
                );
                assert_eq!(
                    model.temporal_prediction,
                    expected.prediction_mode == PREDICT_TEMPORAL
                );
                assert!(model.bounded_payload_bytes > 0);
            }
        }
    }

    #[test]
    fn modeled_predictor_selector_stays_near_exact_oracle_and_within_error_bound() {
        let reference = patterned_frame(65, 33);
        let mut frame = reference.clone();
        for plane in &mut frame.planes {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if index % 31 == 0 {
                    *sample = sample.saturating_add(1);
                }
            }
        }
        for quality in [90, 100] {
            let options = CodecOptions {
                quality,
                tile_width: 16,
                tile_height: 15,
                threads: 2,
            };
            for reference in [None, Some(&reference)] {
                let encoded = if let Some(reference) = reference {
                    encode_with_reference(&frame, reference, options).unwrap()
                } else {
                    encode(&frame, options).unwrap()
                };
                let parsed = parse(&encoded).unwrap();
                let models = analyze_predictors(&frame, reference, options).unwrap();
                let error_bound = (options.quantization_step() / 2) as u32;
                for (tile_index, (model, entry)) in models.iter().zip(&parsed.entries).enumerate() {
                    assert!(
                        entry.length >= model.oracle.payload_bytes,
                        "tile {tile_index}, quality {quality}, reference {}",
                        reference.is_some()
                    );
                    assert!(
                        entry.length <= model.oracle.payload_bytes + 8,
                        "tile {tile_index}, quality {quality}, reference {}, exact {}, selected {}",
                        reference.is_some(),
                        model.oracle.payload_bytes,
                        entry.length
                    );
                    for candidate in [
                        Some(model.paeth),
                        Some(model.average),
                        Some(model.clamp_gradient),
                        Some(model.half_gradient),
                        model.temporal,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        assert!(candidate.max_error <= error_bound);
                        if quality == 100 {
                            assert_eq!(candidate.squared_error, 0);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn compatible_predictors_cover_every_8bit_boundary_pair() {
        for left in 0..=u8::MAX {
            for above in 0..=u8::MAX {
                for upper_left in [0, u8::MAX] {
                    assert_eq!(
                        spatial_prediction(SpatialPredictor::Average, left, above, upper_left),
                        ((u16::from(left) + u16::from(above)) / 2) as u8
                    );
                    assert_eq!(
                        spatial_prediction(
                            SpatialPredictor::ClampGradient,
                            left,
                            above,
                            upper_left
                        ),
                        (i32::from(left) + i32::from(above) - i32::from(upper_left)).clamp(0, 255)
                            as u8
                    );
                    let average = (i32::from(left) + i32::from(above)) / 2;
                    assert_eq!(
                        spatial_prediction(SpatialPredictor::HalfGradient, left, above, upper_left),
                        (average + (average - i32::from(upper_left)) / 2).clamp(0, 255) as u8
                    );
                }
            }
        }
    }

    #[test]
    fn predictor_oracle_propagates_exact_q100_reconstruction() {
        let first = patterned_frame(65, 33);
        let mut second = first.clone();
        for plane in &mut second.planes {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if index % 19 == 0 {
                    *sample = sample.saturating_add(2);
                }
            }
        }
        let options = CodecOptions {
            quality: 100,
            tile_width: 16,
            tile_height: 15,
            threads: 1,
        };
        let (_, first_oracle) = analyze_predictor_frame(&first, None, None, options).unwrap();
        assert_eq!(first_oracle, first);
        let (_, second_oracle) =
            analyze_predictor_frame(&second, Some(&first), Some(&first_oracle), options).unwrap();
        assert_eq!(second_oracle, second);
    }

    #[test]
    fn every_version_two_spatial_mode_decodes_individual_tiles() {
        let frame = patterned_frame(65, 33);
        let tile = Tile {
            plane: 0,
            x: 0,
            y: 0,
            width: 16,
            height: 15,
        };
        let quantizer = Quantizer::new(1);
        for mode in [
            SpatialPredictor::Paeth,
            SpatialPredictor::Average,
            SpatialPredictor::ClampGradient,
            SpatialPredictor::HalfGradient,
        ] {
            let modeled = model_spatial_predictor(&frame.planes[0], tile, &quantizer, mode);
            let decoded = decode_tile_payload(
                tile,
                &modeled.encoded.payload,
                1,
                modeled.encoded.entropy_mode,
                modeled.encoded.prediction_mode,
                None,
            )
            .unwrap();
            assert_eq!(decoded, modeled.reconstruction);
        }
    }

    #[test]
    fn legacy_version_accepts_legacy_modes_and_rejects_new_modes() {
        let frame = Frame::gray8(1, 1, FrameRate::new(24, 1), vec![17]).unwrap();
        let mut legacy = encode(
            &frame,
            CodecOptions {
                quality: 100,
                ..CodecOptions::default()
            },
        )
        .unwrap();
        assert_eq!(legacy[HEADER_LEN + 2], PREDICT_SPATIAL);
        legacy[4] = LEGACY_VERSION;
        assert_eq!(decode(&legacy, 1).unwrap(), frame);

        legacy[HEADER_LEN + 2] = PREDICT_AVERAGE;
        assert!(decode(&legacy, 1).is_err());

        let mut predictor_version = encode(
            &frame,
            CodecOptions {
                quality: 100,
                ..CodecOptions::default()
            },
        )
        .unwrap();
        predictor_version[4] = PREDICTOR_VERSION;
        predictor_version[HEADER_LEN + 1] = ENTROPY_ORDER0;
        assert!(decode(&predictor_version, 1).is_err());
    }
}
