use crate::model::{CodecError, CodecOptions, Frame, FrameRate, PixelFormat, Plane, checked_area};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAGIC: &[u8; 4] = b"FVID";
const VERSION: u8 = 0;
const HEADER_LEN: usize = 32;
const DIRECTORY_ENTRY_LEN: usize = 32;
const MAX_TILES: usize = 1 << 20;

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
    offset: usize,
    length: usize,
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
    let payloads = parallel_map(tiles.len(), options.threads, |index| {
        encode_tile(&frame.planes[tiles[index].plane], tiles[index], step)
    });

    let directory_bytes = tiles
        .len()
        .checked_mul(DIRECTORY_ENTRY_LEN)
        .ok_or(CodecError::LimitExceeded("tile directory is too large"))?;
    let payload_start = HEADER_LEN
        .checked_add(directory_bytes)
        .ok_or(CodecError::LimitExceeded("stream is too large"))?;
    let payload_bytes = payloads.iter().try_fold(0usize, |sum, payload| {
        sum.checked_add(payload.len())
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
    for (&tile, payload) in tiles.iter().zip(&payloads) {
        output.push(
            u8::try_from(tile.plane).map_err(|_| CodecError::LimitExceeded("too many planes"))?,
        );
        output.extend_from_slice(&[0; 3]);
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
            u32::try_from(payload.len())
                .map_err(|_| CodecError::LimitExceeded("tile payload is too large"))?,
        );
        offset += payload.len();
    }
    for payload in payloads {
        output.extend_from_slice(&payload);
    }
    debug_assert_eq!(output.len(), stream_len);
    Ok(output)
}

pub fn decode(bytes: &[u8], threads: usize) -> Result<Frame, CodecError> {
    if threads == 0 {
        return Err(CodecError::InvalidInput("thread count must be nonzero"));
    }
    let parsed = parse(bytes)?;
    let step = quantization_step(parsed.info.quality);
    let decoded = parallel_map(parsed.entries.len(), threads, |index| {
        let entry = parsed.entries[index];
        decode_tile_payload(
            entry.tile,
            &parsed.bytes[entry.offset..entry.offset + entry.length],
            step,
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
    if bytes[4] != VERSION {
        return Err(CodecError::Malformed("unsupported version"));
    }
    let format = PixelFormat::try_from(bytes[5])?;
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
    for (index, expected_tile) in expected.into_iter().enumerate() {
        let start = HEADER_LEN + index * DIRECTORY_ENTRY_LEN;
        if bytes[start + 1..start + 4] != [0; 3] {
            return Err(CodecError::Malformed("nonzero reserved directory bytes"));
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
    match format {
        PixelFormat::Gray8 => vec![(width, height)],
        PixelFormat::Yuv422p8 => vec![
            (width, height),
            (width.div_ceil(2), height),
            (width.div_ceil(2), height),
        ],
    }
}

fn encode_tile(plane: &Plane, tile: Tile, step: i32) -> Vec<u8> {
    let width = tile.width as usize;
    let height = tile.height as usize;
    let plane_width = plane.width as usize;
    let origin_x = tile.x as usize;
    let origin_y = tile.y as usize;
    let mut reconstructed = vec![0u8; width * height];
    let mut payload = Vec::with_capacity(width * height);
    let mut zero_run = 0u32;
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
            let prediction = i32::from(paeth(left, above, upper_left));
            let sample = i32::from(plane.data[(origin_y + y) * plane_width + origin_x + x]);
            let quantized = quantize(sample - prediction, step);
            reconstructed[index] = (prediction + quantized * step).clamp(0, 255) as u8;
            if quantized == 0 {
                zero_run += 1;
            } else {
                flush_zero_run(&mut payload, &mut zero_run);
                put_varint(&mut payload, zigzag(quantized) * 2 - 1);
            }
        }
    }
    flush_zero_run(&mut payload, &mut zero_run);
    payload
}

fn decode_tile_payload(tile: Tile, payload: &[u8], step: i32) -> Result<Vec<u8>, CodecError> {
    let sample_count = checked_area(tile.width, tile.height)?;
    let width =
        usize::try_from(tile.width).map_err(|_| CodecError::LimitExceeded("tile is too large"))?;
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
            while sample_index < run_end {
                reconstruct_sample(&mut output, sample_index, width, 0, step);
                sample_index += 1;
            }
        } else {
            let zigzag_value = token.div_ceil(2);
            let quantized = unzigzag(zigzag_value);
            if quantized == 0 {
                return Err(CodecError::Malformed("non-canonical zero residual"));
            }
            reconstruct_sample(&mut output, sample_index, width, quantized, step);
            sample_index += 1;
        }
    }
    if cursor != payload.len() {
        return Err(CodecError::Malformed("trailing tile payload bytes"));
    }
    Ok(output)
}

fn reconstruct_sample(output: &mut [u8], index: usize, width: usize, quantized: i32, step: i32) {
    let x = index % width;
    let y = index / width;
    let left = if x > 0 { output[index - 1] } else { 0 };
    let above = if y > 0 { output[index - width] } else { 0 };
    let upper_left = if x > 0 && y > 0 {
        output[index - width - 1]
    } else {
        0
    };
    let prediction = i32::from(paeth(left, above, upper_left));
    output[index] = (prediction + quantized * step).clamp(0, 255) as u8;
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
}
