use crate::codec::StreamInfo;
use crate::model::{
    ByteFormatModel, CodecError, CodecOptions, Frame16, FrameRate, MAX_FRAME_BYTES, PixelFormat,
    Plane16, PredictorBandModel, PredictorCandidateModel, PredictorModelMode, TileEntropyModel,
    TilePredictorModel, TileResidualMappingModel, checked_area, fold_bounded_residual, sample_max,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAGIC: &[u8; 4] = b"FVID";
const LEGACY_VERSION: u8 = 1;
const VERSION: u8 = 2;
const HEADER_LEN: usize = 32;
const DIRECTORY_ENTRY_LEN: usize = 32;
const MAX_TILES: usize = 1 << 20;
const ENTROPY_ZERO_RUN: u8 = 0;
const ENTROPY_RICE_BASE: u8 = 1;
const MAX_RICE_PARAMETER: u8 = 16;
const ENTROPY_BLOCK_PACK: u8 = ENTROPY_RICE_BASE + MAX_RICE_PARAMETER + 1;
const BLOCK_PACK_SYMBOLS: usize = 128;
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
    encode_internal(frame, None, false, options)
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
    let prefer_temporal = absolute_difference <= current.len() as u64 * threshold;
    encode_internal(frame, Some(reference), prefer_temporal, options)
}

fn encode_internal(
    frame: &Frame16,
    reference: Option<&Frame16>,
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
    let step = quantization_step(options.quality, frame.bit_depth());
    let quantizer = Quantizer16::new(step, frame.bit_depth());
    let payloads =
        if options.threads == 1 && reference.is_none() && frame.bit_depth() == 10 && step > 1 {
            encode_intra_tile_pairs(frame, &tiles, &quantizer)
        } else {
            parallel_map(tiles.len(), options.threads, |index| {
                let tile = tiles[index];
                encode_best_tile(
                    &frame.planes[tile.plane],
                    reference.map(|frame| &frame.planes[tile.plane]),
                    tile,
                    &quantizer,
                    prefer_temporal,
                )
            })
        };
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

fn encode_intra_tile_pairs(
    frame: &Frame16,
    tiles: &[Tile],
    quantizer: &Quantizer16,
) -> Vec<EncodedTile> {
    let mut payloads = Vec::with_capacity(tiles.len());
    let mut pairs = tiles.chunks_exact(2);
    for pair in &mut pairs {
        let first = pair[0];
        let second = pair[1];
        let first_plane = &frame.planes[first.plane];
        let second_plane = &frame.planes[second.plane];
        let first_selection = sample_fixed_gradient_entropy(first_plane, first, quantizer);
        let second_selection = sample_fixed_gradient_entropy(second_plane, second, quantizer);
        let same_rice_parameter = first_selection.0 == second_selection.0
            && first_selection.0 != ENTROPY_ZERO_RUN
            && !first_selection.1
            && !second_selection.1;
        if first.plane == second.plane
            && first.width == second.width
            && first.height == second.height
            && same_rice_parameter
        {
            let parameter = first_selection.0 - ENTROPY_RICE_BASE;
            let encoded = match parameter {
                0 => Some(encode_fixed_gradient_rice_tile_pair_specialized::<0>(
                    first_plane,
                    first,
                    second,
                    quantizer,
                )),
                4 => Some(encode_fixed_gradient_rice_tile_pair_specialized::<4>(
                    first_plane,
                    first,
                    second,
                    quantizer,
                )),
                _ => None,
            };
            if let Some((first_encoded, second_encoded)) = encoded {
                payloads.push(first_encoded);
                payloads.push(second_encoded);
                continue;
            }
        }
        payloads.push(encode_sampled_fixed_gradient_tile_selected(
            first_plane,
            first,
            quantizer,
            first_selection,
        ));
        payloads.push(encode_sampled_fixed_gradient_tile_selected(
            second_plane,
            second,
            quantizer,
            second_selection,
        ));
    }
    if let Some(&tile) = pairs.remainder().first() {
        payloads.push(encode_sampled_fixed_gradient_tile(
            &frame.planes[tile.plane],
            tile,
            quantizer,
        ));
    }
    payloads
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
            let order0 = model.order0_size();
            let context_order0 = model.context_order0_size();
            let rice4_shard = (ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER)
                .contains(&entry.entropy_mode)
                .then(|| model.rice_lane_size(entry.entropy_mode - ENTROPY_RICE_BASE, 4096, 4));
            let diagonal_order = (entry.prediction_mode != PREDICT_TEMPORAL).then(|| {
                model.diagonal_order_size(
                    entry.tile.width as usize,
                    entry.tile.height as usize,
                    true,
                )
            });
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
                rice4_shard_supported: rice4_shard.is_some(),
                rice4_shard_payload_bytes: rice4_shard.unwrap_or_default().payload_bytes,
                rice4_shard_control_bytes: rice4_shard.unwrap_or_default().control_bytes,
                rice4_shard_complete_bytes: rice4_shard.unwrap_or_default().complete_bytes,
                diagonal_order_supported: diagonal_order.is_some(),
                raster_best_bytes: diagonal_order.unwrap_or_default().raster_best_bytes,
                raster_rice_bytes: diagonal_order.unwrap_or_default().raster_rice_bytes,
                diagonal_zero_run_bytes: diagonal_order.unwrap_or_default().diagonal_zero_run_bytes,
                diagonal_rice_bytes: diagonal_order.unwrap_or_default().diagonal_rice_bytes,
                diagonal_block_bytes: diagonal_order.unwrap_or_default().diagonal_block_bytes,
                diagonal_best_bytes: diagonal_order.unwrap_or_default().diagonal_best_bytes,
            })
        })
        .collect()
}

/// High-bit equivalent of [`crate::codec::analyze_residual_mapping`].
pub fn analyze_residual_mapping16(
    frame: &Frame16,
    reference: Option<&Frame16>,
    options: CodecOptions,
) -> Result<Vec<TileResidualMappingModel>, CodecError> {
    frame.validate()?;
    options.validate()?;
    if let Some(reference) = reference {
        validate_reference(frame, reference)?;
    }
    let reference = reference.filter(|reference| {
        let threshold = 5u64 << (frame.bit_depth() - 8);
        let current = &frame.planes[0].data;
        let previous = &reference.planes[0].data;
        let absolute_difference: u64 = current
            .iter()
            .zip(previous)
            .map(|(&current, &previous)| u64::from(current.abs_diff(previous)))
            .sum();
        absolute_difference <= current.len() as u64 * threshold
    });
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let quantizer = Quantizer16::new(
        quantization_step(options.quality, frame.bit_depth()),
        frame.bit_depth(),
    );
    Ok(tiles
        .into_iter()
        .map(|tile| {
            let plane = &frame.planes[tile.plane];
            let reference_plane = reference.map(|frame| &frame.planes[tile.plane]);
            model_residual_mapping_tile(plane, reference_plane, tile, &quantizer)
        })
        .collect())
}

/// High-bit equivalent of [`crate::codec::analyze_predictors`].
pub fn analyze_predictors16(
    frame: &Frame16,
    reference: Option<&Frame16>,
    options: CodecOptions,
) -> Result<Vec<TilePredictorModel>, CodecError> {
    frame.validate()?;
    options.validate()?;
    if let Some(reference) = reference {
        validate_reference(frame, reference)?;
    }
    let current_reference = reference.filter(|reference| {
        let threshold = 5u64 << (frame.bit_depth() - 8);
        let current = &frame.planes[0].data;
        let previous = &reference.planes[0].data;
        let absolute_difference: u64 = current
            .iter()
            .zip(previous)
            .map(|(&current, &previous)| u64::from(current.abs_diff(previous)))
            .sum();
        absolute_difference <= current.len() as u64 * threshold
    });
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let quantizer = Quantizer16::new(
        quantization_step(options.quality, frame.bit_depth()),
        frame.bit_depth(),
    );
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

/// High-bit equivalent of [`crate::codec::analyze_predictor_frame`].
pub fn analyze_predictor_frame16(
    frame: &Frame16,
    current_reference: Option<&Frame16>,
    oracle_reference: Option<&Frame16>,
    options: CodecOptions,
) -> Result<(Vec<TilePredictorModel>, Frame16), CodecError> {
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
    let selected_current = current_reference.filter(|reference| {
        let threshold = 5u64 << (frame.bit_depth() - 8);
        let current = &frame.planes[0].data;
        let previous = &reference.planes[0].data;
        let absolute_difference: u64 = current
            .iter()
            .zip(previous)
            .map(|(&current, &previous)| u64::from(current.abs_diff(previous)))
            .sum();
        absolute_difference <= current.len() as u64 * threshold
    });
    let tiles = expected_tiles(
        frame.width,
        frame.height,
        frame.format,
        options.tile_width,
        options.tile_height,
    )?;
    let quantizer = Quantizer16::new(
        quantization_step(options.quality, frame.bit_depth()),
        frame.bit_depth(),
    );
    let modeled: Vec<(TilePredictorModel, Vec<u16>)> = tiles
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
    if &bytes[0..4] != MAGIC {
        return Err(CodecError::Malformed("bad magic"));
    }
    let version = bytes[4];
    if version != LEGACY_VERSION && version != VERSION {
        return Err(CodecError::Malformed(
            "unsupported high-bit Fastvid version",
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
    let mut block_pack_tiles = 0;
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
            && !(version == VERSION && entropy_mode == ENTROPY_BLOCK_PACK)
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
        rice_tiles += usize::from(
            (ENTROPY_RICE_BASE..=ENTROPY_RICE_BASE + MAX_RICE_PARAMETER).contains(&entropy_mode),
        );
        block_pack_tiles += usize::from(entropy_mode == ENTROPY_BLOCK_PACK);
        spatial_tiles += usize::from(prediction_mode != PREDICT_TEMPORAL);
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
            block_pack_tiles,
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

fn encode_best_tile(
    plane: &Plane16,
    reference: Option<&Plane16>,
    tile: Tile,
    quantizer: &Quantizer16,
    prefer_temporal: bool,
) -> EncodedTile {
    if let Some(reference) = reference.filter(|_| prefer_temporal) {
        return encode_temporal_tile(plane, reference, tile, quantizer);
    }
    encode_sampled_fixed_gradient_tile(plane, tile, quantizer)
}

#[allow(dead_code)]
fn encode_best_tile_exhaustive(
    plane: &Plane16,
    reference: Option<&Plane16>,
    tile: Tile,
    quantizer: &Quantizer16,
    prefer_temporal: bool,
) -> EncodedTile {
    if quantizer.max_sample == i32::from(u16::MAX)
        && let Some(reference) = reference
    {
        return encode_temporal_tile(plane, reference, tile, quantizer);
    }
    const INTER_MODES: [SpatialPredictor; 2] =
        [SpatialPredictor::Paeth, SpatialPredictor::ClampGradient];
    const INTRA_MODES: [SpatialPredictor; 3] = [
        SpatialPredictor::Paeth,
        SpatialPredictor::Average,
        SpatialPredictor::ClampGradient,
    ];
    let modes: &[SpatialPredictor] = if reference.is_some() {
        &INTER_MODES
    } else {
        &INTRA_MODES
    };
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut rows: [Vec<u16>; 3] = std::array::from_fn(|_| vec![0u16; width]);
    let mut spatial_folded: [Vec<u32>; 3] =
        std::array::from_fn(|_| Vec::with_capacity(width * height));
    let mut spatial_squared_errors = [0u64; 3];
    let mut temporal_folded = reference.map(|_| Vec::with_capacity(width * height));
    let mut temporal_squared_error = 0u64;
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = [0u16; 3];
        let mut upper_left = [0u16; 3];
        for (x, &sample) in plane.data[row_start..row_start + width].iter().enumerate() {
            for mode_index in 0..modes.len() {
                let above = rows[mode_index][x];
                let prediction = i32::from(spatial_prediction(
                    modes[mode_index],
                    left[mode_index],
                    above,
                    upper_left[mode_index],
                    quantizer.max_sample as u16,
                ));
                let quantized = quantizer.quantize(i32::from(sample) - prediction);
                let reconstructed =
                    (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
                let error = i64::from(sample) - i64::from(reconstructed);
                spatial_squared_errors[mode_index] += (error * error) as u64;
                rows[mode_index][x] = reconstructed;
                upper_left[mode_index] = above;
                left[mode_index] = reconstructed;
                spatial_folded[mode_index].push(zigzag(quantized));
            }
            if let (Some(reference), Some(folded)) = (reference, temporal_folded.as_mut()) {
                let prediction = i32::from(reference.data[row_start + x]);
                let quantized = quantizer.quantize(i32::from(sample) - prediction);
                let reconstructed =
                    (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample);
                let error = i64::from(sample) - i64::from(reconstructed);
                temporal_squared_error += (error * error) as u64;
                folded.push(zigzag(quantized));
            }
        }
    }
    let mut candidates: Vec<(u8, Vec<u32>, u64, usize)> = modes
        .iter()
        .copied()
        .zip(spatial_folded)
        .zip(spatial_squared_errors)
        .map(|((mode, folded), squared_error)| {
            let bytes = modeled_entropy_cost(&folded).1;
            (spatial_prediction_mode(mode), folded, squared_error, bytes)
        })
        .collect();
    if let Some(folded) = temporal_folded {
        let bytes = modeled_entropy_cost(&folded).1;
        candidates.push((PREDICT_TEMPORAL, folded, temporal_squared_error, bytes));
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
    let (prediction_mode, folded, _, _) = candidates.swap_remove(selected);
    finish_entropy(folded, prediction_mode)
}

fn encode_sampled_fixed_gradient_tile(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
) -> EncodedTile {
    let selection = sample_fixed_gradient_entropy(plane, tile, quantizer);
    encode_sampled_fixed_gradient_tile_selected(plane, tile, quantizer, selection)
}

fn encode_sampled_fixed_gradient_tile_selected(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
    (sampled_mode, try_block_pack): (u8, bool),
) -> EncodedTile {
    if try_block_pack {
        return encode_fixed_gradient_block_tile(plane, tile, quantizer);
    }
    if sampled_mode != ENTROPY_ZERO_RUN {
        return encode_fixed_gradient_rice_tile(
            plane,
            tile,
            quantizer,
            sampled_mode - ENTROPY_RICE_BASE,
        );
    }
    encode_fixed_gradient_tile(plane, tile, quantizer)
}

fn sample_fixed_gradient_entropy(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
) -> (u8, bool) {
    let width = tile.width as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let y = tile.height as usize / 2;
    let row_start = (origin_y + y) * plane_width + origin_x;
    let mut folded = Vec::with_capacity(width);
    for x in 0..width {
        let source = row_start + x;
        let left = if x == 0 { 0 } else { plane.data[source - 1] };
        let above = if y == 0 {
            0
        } else {
            plane.data[source - plane_width]
        };
        let upper_left = if x == 0 || y == 0 {
            0
        } else {
            plane.data[source - plane_width - 1]
        };
        let prediction = i32::from(spatial_prediction(
            SpatialPredictor::ClampGradient,
            left,
            above,
            upper_left,
            quantizer.max_sample as u16,
        ));
        let quantized = quantizer.quantize(i32::from(plane.data[source]) - prediction);
        folded.push(zigzag(quantized));
    }
    let (legacy_mode, legacy_bytes) = modeled_entropy_cost(&folded);
    (
        legacy_mode,
        legacy_mode != ENTROPY_ZERO_RUN && modeled_block_pack_cost(&folded) < legacy_bytes,
    )
}

fn modeled_block_pack_cost(folded: &[u32]) -> usize {
    folded
        .chunks(BLOCK_PACK_SYMBOLS)
        .map(|block| {
            let maximum = block.iter().copied().max().unwrap_or(0);
            let width = (u32::BITS - maximum.leading_zeros()) as usize;
            1 + (block.len() * width).div_ceil(8)
        })
        .sum()
}

fn encode_fixed_gradient_block_tile(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u16; width];
    let mut payload = Vec::with_capacity(width * height);
    let mut block = [0u32; BLOCK_PACK_SYMBOLS];
    let mut block_len = 0usize;
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = 0;
        let mut upper_left = 0;
        for (&sample, reconstructed_slot) in plane.data[row_start..row_start + width]
            .iter()
            .zip(&mut reconstructed_row)
        {
            let above = *reconstructed_slot;
            let prediction = i32::from(spatial_prediction(
                SpatialPredictor::ClampGradient,
                left,
                above,
                upper_left,
                quantizer.max_sample as u16,
            ));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed =
                (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            let folded = zigzag(quantized);
            block[block_len] = folded;
            block_len += 1;
            if block_len == BLOCK_PACK_SYMBOLS {
                put_fixed_block(&mut payload, &block);
                block_len = 0;
            }
        }
    }
    if block_len != 0 {
        put_fixed_block(&mut payload, &block[..block_len]);
    }
    EncodedTile {
        entropy_mode: ENTROPY_BLOCK_PACK,
        prediction_mode: PREDICT_CLAMP_GRADIENT,
        payload,
    }
}

fn put_fixed_block(output: &mut Vec<u8>, folded: &[u32]) {
    let maximum = folded.iter().copied().max().unwrap_or(0);
    let width = (u32::BITS - maximum.leading_zeros()) as u8;
    output.push(width);
    if width <= 8 {
        let mut chunks = folded.chunks_exact(8);
        for chunk in &mut chunks {
            let mut packed = 0u64;
            for (index, &value) in chunk.iter().enumerate() {
                packed |= u64::from(value) << (index * usize::from(width));
            }
            output.extend_from_slice(&packed.to_le_bytes()[..usize::from(width)]);
        }
        let remainder = chunks.remainder();
        if remainder.is_empty() {
            return;
        }
        let mut writer = BitWriter::new(output);
        for &value in remainder {
            writer.put_bits(value, width);
        }
        writer.finish();
        return;
    }
    let mut writer = BitWriter::new(output);
    for &value in folded {
        writer.put_bits(value, width);
    }
    writer.finish();
}

fn encode_fixed_gradient_rice_tile(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
    rice_parameter: u8,
) -> EncodedTile {
    match rice_parameter {
        0 => encode_fixed_gradient_rice_tile_specialized::<0>(plane, tile, quantizer),
        4 => encode_fixed_gradient_rice_tile_specialized::<4>(plane, tile, quantizer),
        _ => encode_fixed_gradient_rice_tile_scalar(plane, tile, quantizer, rice_parameter),
    }
}

fn encode_fixed_gradient_rice_tile_specialized<const RICE_PARAMETER: u8>(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u16; width];
    let mut payload = Vec::with_capacity(width * height);
    {
        let mut writer = BitWriter::new(&mut payload);
        for y in 0..height {
            let row_start = (origin_y + y) * plane_width + origin_x;
            let source_row = &plane.data[row_start..row_start + width];
            let mut left = 0;
            let mut upper_left = 0;
            let mut x = 0;
            while x + 4 <= width {
                let mut values = [0u32; 4];
                for offset in 0..4 {
                    let sample = source_row[x + offset];
                    let reconstructed_slot = &mut reconstructed_row[x + offset];
                    let above = *reconstructed_slot;
                    let prediction = i32::from(spatial_prediction(
                        SpatialPredictor::ClampGradient,
                        left,
                        above,
                        upper_left,
                        quantizer.max_sample as u16,
                    ));
                    let quantized = quantizer.quantize(i32::from(sample) - prediction);
                    let reconstructed = (prediction + quantized * quantizer.step)
                        .clamp(0, quantizer.max_sample)
                        as u16;
                    *reconstructed_slot = reconstructed;
                    upper_left = above;
                    left = reconstructed;
                    values[offset] = zigzag(quantized);
                }
                writer.put_rice4_specialized::<RICE_PARAMETER>(values);
                x += 4;
            }
            while x < width {
                let sample = source_row[x];
                let reconstructed_slot = &mut reconstructed_row[x];
                let above = *reconstructed_slot;
                let prediction = i32::from(spatial_prediction(
                    SpatialPredictor::ClampGradient,
                    left,
                    above,
                    upper_left,
                    quantizer.max_sample as u16,
                ));
                let quantized = quantizer.quantize(i32::from(sample) - prediction);
                let reconstructed =
                    (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
                *reconstructed_slot = reconstructed;
                upper_left = above;
                left = reconstructed;
                writer.put_rice(zigzag(quantized), RICE_PARAMETER);
                x += 1;
            }
        }
        writer.finish();
    }
    EncodedTile {
        entropy_mode: ENTROPY_RICE_BASE + RICE_PARAMETER,
        prediction_mode: PREDICT_CLAMP_GRADIENT,
        payload,
    }
}

fn encode_fixed_gradient_rice_tile_pair_specialized<const RICE_PARAMETER: u8>(
    plane: &Plane16,
    first: Tile,
    second: Tile,
    quantizer: &Quantizer16,
) -> (EncodedTile, EncodedTile) {
    debug_assert_eq!(first.plane, second.plane);
    debug_assert_eq!(first.width, second.width);
    debug_assert_eq!(first.height, second.height);
    let width = first.width as usize;
    let height = first.height as usize;
    let plane_width = plane.width as usize;
    let first_origin_x = first.x as usize;
    let first_origin_y = first.y as usize;
    let second_origin_x = second.x as usize;
    let second_origin_y = second.y as usize;
    let mut first_reconstructed_row = vec![0u16; width];
    let mut second_reconstructed_row = vec![0u16; width];
    let mut first_payload = Vec::with_capacity(width * height);
    let mut second_payload = Vec::with_capacity(width * height);
    {
        let mut first_writer = BitWriter::new(&mut first_payload);
        let mut second_writer = BitWriter::new(&mut second_payload);
        for y in 0..height {
            let first_start = (first_origin_y + y) * plane_width + first_origin_x;
            let second_start = (second_origin_y + y) * plane_width + second_origin_x;
            let first_source_row = &plane.data[first_start..first_start + width];
            let second_source_row = &plane.data[second_start..second_start + width];
            let mut first_left = 0;
            let mut first_upper_left = 0;
            let mut second_left = 0;
            let mut second_upper_left = 0;
            let mut x = 0;
            while x + 4 <= width {
                let mut first_values = [0u32; 4];
                let mut second_values = [0u32; 4];
                for offset in 0..4 {
                    let index = x + offset;

                    let first_above = first_reconstructed_row[index];
                    let first_prediction = i32::from(spatial_prediction(
                        SpatialPredictor::ClampGradient,
                        first_left,
                        first_above,
                        first_upper_left,
                        quantizer.max_sample as u16,
                    ));
                    let first_quantized =
                        quantizer.quantize(i32::from(first_source_row[index]) - first_prediction);
                    let first_reconstructed = (first_prediction + first_quantized * quantizer.step)
                        .clamp(0, quantizer.max_sample)
                        as u16;
                    first_reconstructed_row[index] = first_reconstructed;
                    first_upper_left = first_above;
                    first_left = first_reconstructed;
                    first_values[offset] = zigzag(first_quantized);

                    let second_above = second_reconstructed_row[index];
                    let second_prediction = i32::from(spatial_prediction(
                        SpatialPredictor::ClampGradient,
                        second_left,
                        second_above,
                        second_upper_left,
                        quantizer.max_sample as u16,
                    ));
                    let second_quantized =
                        quantizer.quantize(i32::from(second_source_row[index]) - second_prediction);
                    let second_reconstructed =
                        (second_prediction + second_quantized * quantizer.step)
                            .clamp(0, quantizer.max_sample) as u16;
                    second_reconstructed_row[index] = second_reconstructed;
                    second_upper_left = second_above;
                    second_left = second_reconstructed;
                    second_values[offset] = zigzag(second_quantized);
                }
                first_writer.put_rice4_specialized::<RICE_PARAMETER>(first_values);
                second_writer.put_rice4_specialized::<RICE_PARAMETER>(second_values);
                x += 4;
            }
            while x < width {
                let first_above = first_reconstructed_row[x];
                let first_prediction = i32::from(spatial_prediction(
                    SpatialPredictor::ClampGradient,
                    first_left,
                    first_above,
                    first_upper_left,
                    quantizer.max_sample as u16,
                ));
                let first_quantized =
                    quantizer.quantize(i32::from(first_source_row[x]) - first_prediction);
                let first_reconstructed = (first_prediction + first_quantized * quantizer.step)
                    .clamp(0, quantizer.max_sample)
                    as u16;
                first_reconstructed_row[x] = first_reconstructed;
                first_upper_left = first_above;
                first_left = first_reconstructed;

                let second_above = second_reconstructed_row[x];
                let second_prediction = i32::from(spatial_prediction(
                    SpatialPredictor::ClampGradient,
                    second_left,
                    second_above,
                    second_upper_left,
                    quantizer.max_sample as u16,
                ));
                let second_quantized =
                    quantizer.quantize(i32::from(second_source_row[x]) - second_prediction);
                let second_reconstructed = (second_prediction + second_quantized * quantizer.step)
                    .clamp(0, quantizer.max_sample)
                    as u16;
                second_reconstructed_row[x] = second_reconstructed;
                second_upper_left = second_above;
                second_left = second_reconstructed;

                first_writer.put_rice(zigzag(first_quantized), RICE_PARAMETER);
                second_writer.put_rice(zigzag(second_quantized), RICE_PARAMETER);
                x += 1;
            }
        }
        first_writer.finish();
        second_writer.finish();
    }
    (
        EncodedTile {
            entropy_mode: ENTROPY_RICE_BASE + RICE_PARAMETER,
            prediction_mode: PREDICT_CLAMP_GRADIENT,
            payload: first_payload,
        },
        EncodedTile {
            entropy_mode: ENTROPY_RICE_BASE + RICE_PARAMETER,
            prediction_mode: PREDICT_CLAMP_GRADIENT,
            payload: second_payload,
        },
    )
}

fn encode_fixed_gradient_rice_tile_scalar(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
    rice_parameter: u8,
) -> EncodedTile {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u16; width];
    let mut payload = Vec::with_capacity(width * height);
    {
        let mut writer = BitWriter::new(&mut payload);
        for y in 0..height {
            let row_start = (origin_y + y) * plane_width + origin_x;
            let mut left = 0;
            let mut upper_left = 0;
            for (&sample, reconstructed_slot) in plane.data[row_start..row_start + width]
                .iter()
                .zip(&mut reconstructed_row)
            {
                let above = *reconstructed_slot;
                let prediction = i32::from(spatial_prediction(
                    SpatialPredictor::ClampGradient,
                    left,
                    above,
                    upper_left,
                    quantizer.max_sample as u16,
                ));
                let quantized = quantizer.quantize(i32::from(sample) - prediction);
                let reconstructed =
                    (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
                *reconstructed_slot = reconstructed;
                upper_left = above;
                left = reconstructed;
                writer.put_rice(zigzag(quantized), rice_parameter);
            }
        }
        writer.finish();
    }
    EncodedTile {
        entropy_mode: ENTROPY_RICE_BASE + rice_parameter,
        prediction_mode: PREDICT_CLAMP_GRADIENT,
        payload,
    }
}

fn encode_fixed_gradient_tile(plane: &Plane16, tile: Tile, quantizer: &Quantizer16) -> EncodedTile {
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
            let prediction = i32::from(spatial_prediction(
                SpatialPredictor::ClampGradient,
                left,
                above,
                upper_left,
                quantizer.max_sample as u16,
            ));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed =
                (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            folded.push(zigzag(quantized));
        }
    }
    finish_entropy(folded, PREDICT_CLAMP_GRADIENT)
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

fn model_residual_mapping_tile(
    plane: &Plane16,
    reference: Option<&Plane16>,
    tile: Tile,
    quantizer: &Quantizer16,
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
    reconstruction: Vec<u16>,
    parallel_entropy: ParallelEntropyModel,
    #[cfg(test)]
    encoded: EncodedTile,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ParallelEntropyModel {
    payload_bytes: usize,
    control_bytes: usize,
    complete_bytes: usize,
    max_span: usize,
}

fn model_predictor_tile(
    plane: &Plane16,
    available_reference: Option<&Plane16>,
    current_reference: Option<&Plane16>,
    tile: Tile,
    quantizer: &Quantizer16,
) -> (TilePredictorModel, Vec<u16>) {
    let paeth = model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::Paeth);
    let average = model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::Average);
    let clamp_gradient =
        model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::ClampGradient);
    let half_gradient =
        model_spatial_predictor(plane, tile, quantizer, SpatialPredictor::HalfGradient);
    let band16_clamp = model_clamp_gradient_bands(plane, tile, quantizer, 16);
    let band32_clamp = model_clamp_gradient_bands(plane, tile, quantizer, 32);
    let band64_clamp = model_clamp_gradient_bands(plane, tile, quantizer, 64);
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
            band16_clamp,
            band32_clamp,
            band64_clamp,
        },
        reconstruction,
    )
}

fn model_clamp_gradient_bands(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
    band_height: u32,
) -> PredictorBandModel {
    debug_assert!(band_height != 0);
    let mut bands = 0usize;
    let mut payload_bytes = 0usize;
    let mut parallel_payload_bytes = 0usize;
    let mut parallel_entropy_control_bytes = 0usize;
    let mut max_entropy_span = 0usize;
    let mut squared_error = 0u64;
    let mut max_error = 0u32;
    for offset_y in (0..tile.height).step_by(band_height as usize) {
        let band = Tile {
            y: tile.y + offset_y,
            height: band_height.min(tile.height - offset_y),
            ..tile
        };
        let modeled =
            model_spatial_predictor(plane, band, quantizer, SpatialPredictor::ClampGradient);
        bands += 1;
        payload_bytes += modeled.summary.payload_bytes;
        parallel_payload_bytes += modeled.parallel_entropy.payload_bytes;
        parallel_entropy_control_bytes += modeled.parallel_entropy.control_bytes;
        max_entropy_span = max_entropy_span.max(modeled.parallel_entropy.max_span);
        squared_error += modeled.summary.squared_error;
        max_error = max_error.max(modeled.summary.max_error);
    }
    let control_bytes = bands.saturating_sub(1) * 5;
    PredictorBandModel {
        bands,
        max_band_samples: tile.width as usize * band_height.min(tile.height) as usize,
        payload_bytes,
        control_bytes,
        complete_bytes: payload_bytes + control_bytes,
        squared_error,
        max_error,
        parallel_payload_bytes,
        parallel_control_bytes: control_bytes + parallel_entropy_control_bytes,
        parallel_complete_bytes: parallel_payload_bytes
            + control_bytes
            + parallel_entropy_control_bytes,
        max_entropy_span,
    }
}

fn model_temporal_predictor(
    plane: &Plane16,
    reference: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
) -> ModeledPredictor {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut folded = Vec::with_capacity(width * height);
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
            let reconstructed =
                (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample);
            let error = i64::from(sample) - i64::from(reconstructed);
            squared_error += (error * error) as u64;
            max_error = max_error.max(error.unsigned_abs() as u32);
            reconstruction.push(reconstructed as u16);
            folded.push(zigzag(quantized));
        }
    }
    let parallel_entropy = model_parallel_entropy(&folded);
    candidate_model(
        finish_entropy(folded, PREDICT_TEMPORAL),
        squared_error,
        max_error,
        reconstruction,
        parallel_entropy,
    )
}

fn model_spatial_predictor(
    plane: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
    mode: SpatialPredictor,
) -> ModeledPredictor {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed_row = vec![0u16; width];
    let mut folded = Vec::with_capacity(width * height);
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
            let prediction = i32::from(spatial_prediction(
                mode,
                left,
                above,
                upper_left,
                quantizer.max_sample as u16,
            ));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed =
                (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
            let error = i64::from(sample) - i64::from(reconstructed);
            squared_error += (error * error) as u64;
            max_error = max_error.max(error.unsigned_abs() as u32);
            reconstruction.push(reconstructed);
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            folded.push(zigzag(quantized));
        }
    }
    let parallel_entropy = model_parallel_entropy(&folded);
    candidate_model(
        finish_entropy(folded, spatial_prediction_mode(mode)),
        squared_error,
        max_error,
        reconstruction,
        parallel_entropy,
    )
}

fn candidate_model(
    encoded: EncodedTile,
    squared_error: u64,
    max_error: u32,
    reconstruction: Vec<u16>,
    parallel_entropy: ParallelEntropyModel,
) -> ModeledPredictor {
    ModeledPredictor {
        summary: PredictorCandidateModel {
            payload_bytes: encoded.payload.len(),
            squared_error,
            max_error,
            zero_run: encoded.entropy_mode == ENTROPY_ZERO_RUN,
        },
        reconstruction,
        parallel_entropy,
        #[cfg(test)]
        encoded,
    }
}

// research/0038: preserve raster locality while bounding entropy state after
// dependency-exact wavefront prediction.
fn model_parallel_entropy(folded: &[u32]) -> ParallelEntropyModel {
    const SHARD_SYMBOLS: usize = 4096;
    const LANES: usize = 4;
    let shard_count = folded.len().div_ceil(SHARD_SYMBOLS);
    let mut result = ParallelEntropyModel::default();
    for (shard_index, shard) in folded.chunks(SHARD_SYMBOLS).enumerate() {
        let zero_run_bytes = modeled_zero_run_cost(shard);
        let mut format = ByteFormatModel::default();
        for &value in shard {
            format.push(value);
        }
        let rice_parameter = best_rice_parameter(shard).0;
        let rice = format.rice_lane_size(rice_parameter, SHARD_SYMBOLS, LANES);
        if zero_run_bytes as u64 <= rice.complete_bytes {
            result.payload_bytes += zero_run_bytes;
            result.max_span = result.max_span.max(shard.len());
        } else {
            result.payload_bytes += rice.payload_bytes as usize;
            result.control_bytes += rice.control_bytes as usize;
            result.max_span = result.max_span.max(shard.len().div_ceil(LANES));
        }
        // One mode/parameter byte per shard and one u32 length except for the
        // final shard; the enclosing predictor band delimits the final one.
        result.control_bytes += 1;
        if shard_index + 1 != shard_count {
            result.control_bytes += 4;
        }
    }
    result.complete_bytes = result.payload_bytes + result.control_bytes;
    result
}

fn spatial_prediction_mode(mode: SpatialPredictor) -> u8 {
    match mode {
        SpatialPredictor::Paeth => PREDICT_SPATIAL,
        SpatialPredictor::Average => PREDICT_AVERAGE,
        SpatialPredictor::ClampGradient => PREDICT_CLAMP_GRADIENT,
        SpatialPredictor::HalfGradient => PREDICT_HALF_GRADIENT,
    }
}

#[cfg(test)]
fn predictor_model_mode_code(mode: PredictorModelMode) -> u8 {
    match mode {
        PredictorModelMode::Paeth => PREDICT_SPATIAL,
        PredictorModelMode::Average => PREDICT_AVERAGE,
        PredictorModelMode::ClampGradient => PREDICT_CLAMP_GRADIENT,
        PredictorModelMode::HalfGradient => PREDICT_HALF_GRADIENT,
        PredictorModelMode::Temporal => PREDICT_TEMPORAL,
    }
}

fn spatial_prediction(
    mode: SpatialPredictor,
    left: u16,
    above: u16,
    upper_left: u16,
    max_sample: u16,
) -> u16 {
    match mode {
        SpatialPredictor::Paeth => paeth(left, above, upper_left),
        SpatialPredictor::Average => ((u32::from(left) + u32::from(above)) / 2) as u16,
        SpatialPredictor::ClampGradient => (i32::from(left) + i32::from(above)
            - i32::from(upper_left))
        .clamp(0, i32::from(max_sample)) as u16,
        SpatialPredictor::HalfGradient => {
            let average = (i32::from(left) + i32::from(above)) / 2;
            (average + (average - i32::from(upper_left)) / 2).clamp(0, i32::from(max_sample)) as u16
        }
    }
}

fn model_bounded_temporal_tile(
    plane: &Plane16,
    reference: &Plane16,
    tile: Tile,
    quantizer: &Quantizer16,
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
                quantizer.quantize(quantizer.max_sample - prediction),
            ));
        }
    }
    modeled_entropy_cost(&folded)
}

fn model_bounded_spatial_tile(plane: &Plane16, tile: Tile, quantizer: &Quantizer16) -> (u8, usize) {
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
            let prediction = i32::from(paeth(left, above, upper_left));
            let quantized = quantizer.quantize(i32::from(sample) - prediction);
            let reconstructed =
                (prediction + quantized * quantizer.step).clamp(0, quantizer.max_sample) as u16;
            *reconstructed_slot = reconstructed;
            upper_left = above;
            left = reconstructed;
            folded.push(fold_bounded_residual(
                quantized,
                quantizer.quantize(-prediction),
                quantizer.quantize(quantizer.max_sample - prediction),
            ));
        }
    }
    modeled_entropy_cost(&folded)
}

fn modeled_entropy_cost(folded: &[u32]) -> (u8, usize) {
    let zero_run_bytes = modeled_zero_run_cost(folded);
    let (parameter, rice_bits) = best_rice_parameter(folded);
    let rice_bytes = usize::try_from(rice_bits.div_ceil(8)).expect("modeled tile size fits usize");
    if rice_bytes >= zero_run_bytes {
        (ENTROPY_ZERO_RUN, zero_run_bytes)
    } else {
        (ENTROPY_RICE_BASE + parameter, rice_bytes)
    }
}

fn modeled_zero_run_cost(folded: &[u32]) -> usize {
    let mut zero_run_bytes = 0usize;
    let mut zero_run = 0u32;
    for &value in folded {
        if value == 0 {
            zero_run += 1;
        } else {
            count_zero_run(&mut zero_run_bytes, &mut zero_run);
            zero_run_bytes += varint_length(value * 2 - 1);
        }
    }
    count_zero_run(&mut zero_run_bytes, &mut zero_run);
    zero_run_bytes
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
    if entropy_mode == ENTROPY_BLOCK_PACK {
        let mut cursor = 0usize;
        let mut decoded = 0usize;
        let max_width = (u32::BITS - max_folded.leading_zeros()) as u8;
        while decoded < sample_count {
            let width = *payload
                .get(cursor)
                .ok_or(CodecError::Malformed("truncated block-pack control"))?;
            cursor += 1;
            if width > max_width {
                return Err(CodecError::Malformed("block-pack width is out of range"));
            }
            let count = BLOCK_PACK_SYMBOLS.min(sample_count - decoded);
            let bytes = (count * usize::from(width)).div_ceil(8);
            let end = cursor
                .checked_add(bytes)
                .filter(|&end| end <= payload.len())
                .ok_or(CodecError::Malformed("truncated block-pack payload"))?;
            let mut reader = BitReader::new(&payload[cursor..end]);
            for _ in 0..count {
                let folded = reader.get_bits(width)?;
                if folded > max_folded {
                    return Err(CodecError::Malformed("residual is out of range"));
                }
                model.push(folded);
            }
            reader.finish()?;
            cursor = end;
            decoded += count;
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
    if entropy_mode == ENTROPY_BLOCK_PACK {
        return decode_block_pack(payload, count, width, step, context, max_sample);
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

fn decode_block_pack(
    payload: &[u8],
    sample_count: usize,
    width: usize,
    step: i32,
    context: PredictionContext<'_>,
    max_sample: u16,
) -> Result<Vec<u16>, CodecError> {
    let max_folded = u32::from(max_sample) * 2;
    let max_width = (u32::BITS - max_folded.leading_zeros()) as u8;
    let mut output = vec![0u16; sample_count];
    let mut cursor = 0usize;
    let mut index = 0usize;
    while index < sample_count {
        let block_width = *payload
            .get(cursor)
            .ok_or(CodecError::Malformed("truncated block-pack control"))?;
        cursor += 1;
        if block_width > max_width {
            return Err(CodecError::Malformed("block-pack width is out of range"));
        }
        let count = BLOCK_PACK_SYMBOLS.min(sample_count - index);
        let bytes = (count * usize::from(block_width)).div_ceil(8);
        let end = cursor
            .checked_add(bytes)
            .filter(|&end| end <= payload.len())
            .ok_or(CodecError::Malformed("truncated block-pack payload"))?;
        if block_width <= 8 {
            let group_count = count / 8;
            let group_bytes = usize::from(block_width);
            for group in 0..group_count {
                let start = cursor + group * group_bytes;
                let mut packed_bytes = [0u8; 8];
                packed_bytes[..group_bytes].copy_from_slice(&payload[start..start + group_bytes]);
                let mut packed = u64::from_le_bytes(packed_bytes);
                let mask = if block_width == 0 {
                    0
                } else {
                    (1u64 << block_width) - 1
                };
                for _ in 0..8 {
                    let folded = (packed & mask) as u32;
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
                    packed >>= block_width;
                }
            }
            let remainder = count % 8;
            if remainder != 0 {
                let remainder_start = cursor + group_count * group_bytes;
                let mut reader = BitReader::new(&payload[remainder_start..end]);
                for _ in 0..remainder {
                    let folded = reader.get_bits(block_width)?;
                    if folded > max_folded {
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
                reader.finish()?;
            }
            cursor = end;
            continue;
        }
        let mut reader = BitReader::new(&payload[cursor..end]);
        for _ in 0..count {
            let folded = reader.get_bits(block_width)?;
            if folded > max_folded {
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
        reader.finish()?;
        cursor = end;
    }
    if cursor != payload.len() {
        return Err(CodecError::Malformed("trailing tile payload bytes"));
    }
    Ok(output)
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
                reconstruct_spatial_zero_run(
                    &mut output,
                    index,
                    end,
                    width,
                    context.mode,
                    max_sample,
                )?;
                index = end;
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

fn reconstruct_spatial_zero_run(
    output: &mut [u16],
    start: usize,
    end: usize,
    width: usize,
    prediction_mode: u8,
    max_sample: u16,
) -> Result<(), CodecError> {
    match spatial_predictor_from_mode(prediction_mode)? {
        SpatialPredictor::Paeth => {
            reconstruct_spatial_zero_run_with(output, start, end, width, paeth)
        }
        SpatialPredictor::Average => {
            reconstruct_spatial_zero_run_with(output, start, end, width, |left, above, _| {
                ((u32::from(left) + u32::from(above)) / 2) as u16
            })
        }
        SpatialPredictor::ClampGradient => reconstruct_spatial_zero_run_with(
            output,
            start,
            end,
            width,
            |left, above, upper_left| {
                (i32::from(left) + i32::from(above) - i32::from(upper_left))
                    .clamp(0, i32::from(max_sample)) as u16
            },
        ),
        SpatialPredictor::HalfGradient => reconstruct_spatial_zero_run_with(
            output,
            start,
            end,
            width,
            |left, above, upper_left| {
                let average = (i32::from(left) + i32::from(above)) / 2;
                (average + (average - i32::from(upper_left)) / 2).clamp(0, i32::from(max_sample))
                    as u16
            },
        ),
    }
    Ok(())
}

fn reconstruct_spatial_zero_run_with(
    output: &mut [u16],
    mut start: usize,
    end: usize,
    width: usize,
    predict: impl Fn(u16, u16, u16) -> u16,
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
            let source = (context.tile.y as usize + y) * reference.width as usize
                + context.tile.x as usize
                + x;
            reference.data[source]
        }
        mode => spatial_prediction(
            spatial_predictor_from_mode(mode)?,
            left,
            above,
            upper_left,
            max_sample,
        ),
    };
    output[index] =
        (i32::from(prediction) + quantized * step).clamp(0, i32::from(max_sample)) as u16;
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

    fn put_rice4_specialized<const RICE_PARAMETER: u8>(&mut self, values: [u32; 4]) {
        let mut packed = 0u64;
        let mut packed_bits = 0u32;
        for value in values {
            let quotient = value >> RICE_PARAMETER;
            let code_bits = quotient + 1 + u32::from(RICE_PARAMETER);
            if code_bits > 64 - packed_bits {
                for value in values {
                    self.put_rice(value, RICE_PARAMETER);
                }
                return;
            }
            let remainder_mask = (1u64 << RICE_PARAMETER) - 1;
            let code = (1u64 << quotient) | ((u64::from(value) & remainder_mask) << (quotient + 1));
            packed |= code << packed_bits;
            packed_bits += code_bits;
        }
        let available = u32::from(64 - self.buffered_bits);
        self.buffer |= packed << self.buffered_bits;
        if packed_bits <= available {
            self.buffered_bits += packed_bits as u8;
            self.flush_bytes();
            return;
        }
        self.buffered_bits = 64;
        self.flush_bytes();
        self.buffer = packed >> available;
        self.buffered_bits = (packed_bits - available) as u8;
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
        debug_assert!(count <= 17);
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
    fn fused_high_bit_rice_writer_matches_boundaries_and_fallbacks() {
        let mut values = vec![0, 1, 2, 255, 256, 65_535, 65_536, 131_070];
        for parameter in 0..=MAX_RICE_PARAMETER {
            for quotient in [0, 1, 47, 48, 55, 56, 62, 63, 64, 65] {
                let value = (quotient << parameter).min(131_070u32);
                values.push(value);
                values.push(value.saturating_add(1).min(131_070));
            }
        }
        values.sort_unstable();
        values.dedup();
        for parameter in 0..=MAX_RICE_PARAMETER {
            for &value in &values {
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
    fn specialized_four_symbol_rice_writer_matches_scalar_groups() {
        let groups = [
            [0, 1, 2, 3],
            [15, 16, 31, 32],
            [255, 256, 65_535, 65_536],
            [131_070, 0, 1, 2],
        ];
        for parameter in [0, 4] {
            for alignment in 0..8 {
                for values in groups {
                    let mut scalar = Vec::new();
                    let mut scalar_writer = BitWriter::new(&mut scalar);
                    scalar_writer.put_bits(0, alignment);
                    for value in values {
                        scalar_writer.put_rice(value, parameter);
                    }
                    scalar_writer.finish();

                    let mut batched = Vec::new();
                    let mut batched_writer = BitWriter::new(&mut batched);
                    batched_writer.put_bits(0, alignment);
                    if parameter == 0 {
                        batched_writer.put_rice4_specialized::<0>(values);
                    } else {
                        batched_writer.put_rice4_specialized::<4>(values);
                    }
                    batched_writer.finish();
                    assert_eq!(
                        batched, scalar,
                        "parameter={parameter} alignment={alignment}"
                    );
                }
            }
        }
    }

    #[test]
    fn interleaved_rice_tile_pairs_match_independent_tiles() {
        let width = 14;
        let height = 5;
        let data = (0..width * height)
            .map(|index| ((index * 977 + index * index * 17 + 131) % 1024) as u16)
            .collect();
        let plane = Plane16::new(width, height, 10, data).unwrap();
        let first = Tile {
            plane: 0,
            x: 0,
            y: 0,
            width: 7,
            height,
        };
        let second = Tile { x: 7, ..first };
        let quantizer = Quantizer16::new(quantization_step(90, 10), 10);
        {
            let first_scalar =
                encode_fixed_gradient_rice_tile_specialized::<0>(&plane, first, &quantizer);
            let second_scalar =
                encode_fixed_gradient_rice_tile_specialized::<0>(&plane, second, &quantizer);
            let paired = encode_fixed_gradient_rice_tile_pair_specialized::<0>(
                &plane, first, second, &quantizer,
            );
            assert_eq!(paired.0.entropy_mode, first_scalar.entropy_mode);
            assert_eq!(paired.0.prediction_mode, first_scalar.prediction_mode);
            assert_eq!(paired.0.payload, first_scalar.payload);
            assert_eq!(paired.1.entropy_mode, second_scalar.entropy_mode);
            assert_eq!(paired.1.prediction_mode, second_scalar.prediction_mode);
            assert_eq!(paired.1.payload, second_scalar.payload);
        }
        {
            let first_scalar =
                encode_fixed_gradient_rice_tile_specialized::<4>(&plane, first, &quantizer);
            let second_scalar =
                encode_fixed_gradient_rice_tile_specialized::<4>(&plane, second, &quantizer);
            let paired = encode_fixed_gradient_rice_tile_pair_specialized::<4>(
                &plane, first, second, &quantizer,
            );
            assert_eq!(paired.0.entropy_mode, first_scalar.entropy_mode);
            assert_eq!(paired.0.prediction_mode, first_scalar.prediction_mode);
            assert_eq!(paired.0.payload, first_scalar.payload);
            assert_eq!(paired.1.entropy_mode, second_scalar.entropy_mode);
            assert_eq!(paired.1.prediction_mode, second_scalar.prediction_mode);
            assert_eq!(paired.1.payload, second_scalar.payload);
        }
    }

    #[test]
    fn independent_band_model_charges_each_added_boundary() {
        let width = 7;
        let height = 17;
        let data = (0..width * height)
            .map(|index| ((index * 71 + index * index * 3) % 1024) as u16)
            .collect();
        let plane = Plane16::new(width, height, 10, data).unwrap();
        let tile = Tile {
            plane: 0,
            x: 0,
            y: 0,
            width,
            height,
        };
        let quantizer = Quantizer16::new(quantization_step(90, 10), 10);
        let bands = model_clamp_gradient_bands(&plane, tile, &quantizer, 16);
        let first = model_spatial_predictor(
            &plane,
            Tile { height: 16, ..tile },
            &quantizer,
            SpatialPredictor::ClampGradient,
        );
        let second = model_spatial_predictor(
            &plane,
            Tile {
                y: 16,
                height: 1,
                ..tile
            },
            &quantizer,
            SpatialPredictor::ClampGradient,
        );
        assert_eq!(bands.bands, 2);
        assert_eq!(bands.max_band_samples, width as usize * 16);
        assert_eq!(
            bands.payload_bytes,
            first.summary.payload_bytes + second.summary.payload_bytes
        );
        assert_eq!(bands.control_bytes, 5);
        assert_eq!(bands.complete_bytes, bands.payload_bytes + 5);
        assert_eq!(
            bands.squared_error,
            first.summary.squared_error + second.summary.squared_error
        );
        assert_eq!(
            bands.max_error,
            first.summary.max_error.max(second.summary.max_error)
        );
    }

    #[test]
    fn parallel_entropy_model_charges_shards_modes_and_rice_lanes() {
        let zero_runs = model_parallel_entropy(&vec![0; 5000]);
        assert_eq!(
            zero_runs,
            ParallelEntropyModel {
                payload_bytes: 4,
                control_bytes: 6,
                complete_bytes: 10,
                max_span: 4096,
            }
        );

        let rice = model_parallel_entropy(&vec![1; 4096]);
        assert_eq!(
            rice,
            ParallelEntropyModel {
                payload_bytes: 1024,
                control_bytes: 13,
                complete_bytes: 1037,
                max_span: 1024,
            }
        );
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
        invalid_entropy[HEADER_LEN + 1] = ENTROPY_BLOCK_PACK + 1;
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

    #[test]
    fn residual_mapping_model_matches_current_high_bit_tile_payloads() {
        let reference = patterned_frame(12, true);
        let mut frame = reference.clone();
        for plane in &mut frame.planes {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if index % 31 == 0 {
                    *sample = sample.saturating_add(1).min(4095);
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
            let selected = reference.filter(|reference| {
                let threshold = 5u64 << (frame.bit_depth() - 8);
                frame.planes[0]
                    .data
                    .iter()
                    .zip(&reference.planes[0].data)
                    .map(|(&current, &previous)| u64::from(current.abs_diff(previous)))
                    .sum::<u64>()
                    <= frame.planes[0].data.len() as u64 * threshold
            });
            let tiles = expected_tiles(
                frame.width,
                frame.height,
                frame.format,
                options.tile_width,
                options.tile_height,
            )
            .unwrap();
            let quantizer = Quantizer16::new(
                quantization_step(options.quality, frame.bit_depth()),
                frame.bit_depth(),
            );
            let models = analyze_residual_mapping16(&frame, reference, options).unwrap();
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
    fn predictor_oracle_matches_current_high_bit_payloads_and_error_bound() {
        let reference = patterned_frame(12, true);
        let mut frame = reference.clone();
        for plane in &mut frame.planes {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if index % 31 == 0 {
                    *sample = sample.saturating_add(1).min(4095);
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
                    encode16_with_reference(&frame, reference, options).unwrap()
                } else {
                    encode16(&frame, options).unwrap()
                };
                let parsed = parse(&encoded).unwrap();
                let models = analyze_predictors16(&frame, reference, options).unwrap();
                let error_bound = (quantization_step(quality, 12) / 2) as u32;
                for (model, entry) in models.iter().zip(&parsed.entries) {
                    assert_eq!(model.oracle.payload_bytes, entry.length);
                    assert_eq!(
                        model.oracle.zero_run,
                        entry.entropy_mode == ENTROPY_ZERO_RUN
                    );
                    assert_eq!(
                        predictor_model_mode_code(model.oracle_mode),
                        entry.prediction_mode
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
    fn compatible_high_bit_predictors_cover_extrema() {
        for bit_depth in [10, 12, 16] {
            let maximum = sample_max(bit_depth).unwrap();
            for left in [0, 1, maximum / 2, maximum - 1, maximum] {
                for above in [0, 1, maximum / 2, maximum - 1, maximum] {
                    for upper_left in [0, maximum] {
                        assert!(
                            spatial_prediction(
                                SpatialPredictor::Average,
                                left,
                                above,
                                upper_left,
                                maximum
                            ) <= maximum
                        );
                        assert!(
                            spatial_prediction(
                                SpatialPredictor::ClampGradient,
                                left,
                                above,
                                upper_left,
                                maximum
                            ) <= maximum
                        );
                        assert!(
                            spatial_prediction(
                                SpatialPredictor::HalfGradient,
                                left,
                                above,
                                upper_left,
                                maximum
                            ) <= maximum
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn high_bit_predictor_oracle_propagates_exact_q100_reconstruction() {
        let first = patterned_frame(12, true);
        let mut second = first.clone();
        for plane in &mut second.planes {
            for (index, sample) in plane.data.iter_mut().enumerate() {
                if index % 19 == 0 {
                    *sample = sample.saturating_add(2).min(4095);
                }
            }
        }
        let options = CodecOptions {
            quality: 100,
            tile_width: 16,
            tile_height: 15,
            threads: 1,
        };
        let (_, first_oracle) = analyze_predictor_frame16(&first, None, None, options).unwrap();
        assert_eq!(first_oracle, first);
        let (_, second_oracle) =
            analyze_predictor_frame16(&second, Some(&first), Some(&first_oracle), options).unwrap();
        assert_eq!(second_oracle, second);
    }

    #[test]
    fn every_high_bit_version_two_spatial_mode_decodes_individual_tiles() {
        let frame = patterned_frame(12, true);
        let tile = Tile {
            plane: 0,
            x: 0,
            y: 0,
            width: frame.planes[0].width,
            height: frame.planes[0].height,
        };
        let quantizer = Quantizer16::new(1, 12);
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
                4095,
            )
            .unwrap();
            assert_eq!(decoded, modeled.reconstruction);
        }
    }

    #[test]
    fn block_pack_round_trips_and_rejects_malformed_blocks() {
        let frame = patterned_frame(10, true);
        let tile = Tile {
            plane: 0,
            x: 0,
            y: 0,
            width: frame.planes[0].width,
            height: frame.planes[0].height,
        };
        let quantizer = Quantizer16::new(1, 10);
        let encoded = encode_fixed_gradient_block_tile(&frame.planes[0], tile, &quantizer);
        let decoded = decode_tile_payload(
            tile,
            &encoded.payload,
            1,
            encoded.entropy_mode,
            encoded.prediction_mode,
            None,
            1023,
        )
        .unwrap();
        assert_eq!(decoded, frame.planes[0].data);

        let tiny = Tile {
            plane: 0,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        for payload in [&[18][..], &[1][..], &[1, 0x80][..], &[0, 0][..]] {
            assert!(
                decode_tile_payload(
                    tiny,
                    payload,
                    1,
                    ENTROPY_BLOCK_PACK,
                    PREDICT_CLAMP_GRADIENT,
                    None,
                    1023,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn word_fixed_blocks_match_scalar_writer_for_every_width_and_length() {
        for width in 0..=17 {
            let mask = if width == 0 { 0 } else { (1u32 << width) - 1 };
            for length in 0..=BLOCK_PACK_SYMBOLS {
                let mut folded: Vec<u32> = (0..length)
                    .map(|index| (index as u32).wrapping_mul(0x9e37_79b9) & mask)
                    .collect();
                if let Some(first) = folded.first_mut() {
                    *first = mask;
                }
                let mut optimized = Vec::new();
                put_fixed_block(&mut optimized, &folded);
                let actual_width = folded
                    .iter()
                    .copied()
                    .max()
                    .map_or(0, |maximum| (u32::BITS - maximum.leading_zeros()) as u8);
                let mut reference = vec![actual_width];
                let mut writer = BitWriter::new(&mut reference);
                for &value in &folded {
                    writer.put_bits(value, actual_width);
                }
                writer.finish();
                assert_eq!(optimized, reference, "width {width}, length {length}");
            }
        }
    }

    #[test]
    fn high_bit_legacy_version_accepts_only_legacy_modes() {
        let frame = Frame16::gray(1, 1, 12, FrameRate::new(24, 1), vec![17]).unwrap();
        let mut legacy = encode16(
            &frame,
            CodecOptions {
                quality: 100,
                ..CodecOptions::default()
            },
        )
        .unwrap();
        assert_eq!(legacy[HEADER_LEN + 2], PREDICT_SPATIAL);
        legacy[4] = LEGACY_VERSION;
        assert_eq!(decode16(&legacy, 1).unwrap(), frame);

        legacy[HEADER_LEN + 2] = PREDICT_AVERAGE;
        assert!(decode16(&legacy, 1).is_err());
    }
}
