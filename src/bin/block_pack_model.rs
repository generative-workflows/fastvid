use std::env;
use std::fs;
use std::process::ExitCode;

const TILE_WIDTH: usize = 256;
const TILE_HEIGHT: usize = 128;
const BLOCK_SYMBOLS: usize = 128;
const HEADER_LEN: usize = 32;
const DIRECTORY_ENTRY_LEN: usize = 32;
const MAX_RICE_PARAMETER: u8 = 16;

#[derive(Clone, Copy, Default)]
struct Model {
    current_payload: usize,
    packed_payload: usize,
    hybrid_payload: usize,
    tiles: usize,
    packed_tiles: usize,
    packed_tiles_by_plane: [usize; 3],
    savings_by_plane: [usize; 3],
    squared_error: u64,
    max_error: u32,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("block_pack_model: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() != 7 {
        return Err(
            "usage: block_pack_model SAMPLE INPUT WIDTH HEIGHT FRAMES BIT_DEPTH QUALITY".into(),
        );
    }
    let sample = &arguments[0];
    let bytes = fs::read(&arguments[1])?;
    let width: usize = arguments[2].parse()?;
    let height: usize = arguments[3].parse()?;
    let frames: usize = arguments[4].parse()?;
    let bit_depth: u8 = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    if width == 0 || height == 0 || frames == 0 {
        return Err("dimensions and frame count must be nonzero".into());
    }
    if !matches!(bit_depth, 10 | 12 | 16) || !(1..=100).contains(&quality) {
        return Err("unsupported bit depth or quality".into());
    }
    let chroma_width = width.div_ceil(2);
    let y_samples = width.checked_mul(height).ok_or("frame is too large")?;
    let chroma_samples = chroma_width
        .checked_mul(height)
        .ok_or("frame is too large")?;
    let frame_samples = y_samples
        .checked_add(2 * chroma_samples)
        .ok_or("frame is too large")?;
    let expected_bytes = frame_samples
        .checked_mul(frames)
        .and_then(|samples| samples.checked_mul(2))
        .ok_or("sequence is too large")?;
    if bytes.len() != expected_bytes {
        return Err(format!("input has {} bytes; expected {expected_bytes}", bytes.len()).into());
    }
    let samples: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let max_sample = if bit_depth == 16 {
        u16::MAX
    } else {
        (1u16 << bit_depth) - 1
    };
    if samples.iter().any(|&value| value > max_sample) {
        return Err("input sample exceeds signaled bit depth".into());
    }
    let base_step = 1 + i32::from((100 - quality) / 5);
    let step = 1 + ((base_step - 1) << (bit_depth - 8));
    let mut model = Model::default();
    for frame in samples.chunks_exact(frame_samples) {
        for (plane, plane_width, start) in [
            (0usize, width, 0usize),
            (1, chroma_width, y_samples),
            (2, chroma_width, y_samples + chroma_samples),
        ] {
            let plane_samples = if plane == 0 {
                y_samples
            } else {
                chroma_samples
            };
            let plane_data = &frame[start..start + plane_samples];
            let nominal_width = if plane == 0 {
                TILE_WIDTH
            } else {
                TILE_WIDTH.div_ceil(2)
            };
            for origin_y in (0..height).step_by(TILE_HEIGHT) {
                let tile_height = TILE_HEIGHT.min(height - origin_y);
                for origin_x in (0..plane_width).step_by(nominal_width) {
                    let tile_width = nominal_width.min(plane_width - origin_x);
                    model = add(
                        model,
                        model_tile(
                            plane,
                            plane_data,
                            plane_width,
                            origin_x,
                            origin_y,
                            tile_width,
                            tile_height,
                            max_sample,
                            step,
                        ),
                    );
                }
            }
        }
    }
    let overhead = frames * HEADER_LEN + model.tiles * DIRECTORY_ENTRY_LEN;
    let current_stream = overhead + model.current_payload;
    let packed_stream = overhead + model.packed_payload;
    let hybrid_stream = overhead + model.hybrid_payload;
    println!(
        "sample\tframes\tbit_depth\tquality\ttiles\tpacked_tiles\tpacked_y_tiles\tpacked_cb_tiles\tpacked_cr_tiles\tsavings_y_bytes\tsavings_cb_bytes\tsavings_cr_bytes\tcurrent_payload_bytes\tpacked_payload_bytes\thybrid_payload_bytes\toverhead_bytes\tcurrent_stream_bytes\tpacked_stream_bytes\thybrid_stream_bytes\tpacked_delta\thybrid_delta\tsquared_error\tmax_error"
    );
    println!(
        "{sample}\t{frames}\t{bit_depth}\t{quality}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{overhead}\t{current_stream}\t{packed_stream}\t{hybrid_stream}\t{:.8}\t{:.8}\t{}\t{}",
        model.tiles,
        model.packed_tiles,
        model.packed_tiles_by_plane[0],
        model.packed_tiles_by_plane[1],
        model.packed_tiles_by_plane[2],
        model.savings_by_plane[0],
        model.savings_by_plane[1],
        model.savings_by_plane[2],
        model.current_payload,
        model.packed_payload,
        model.hybrid_payload,
        packed_stream as f64 / current_stream as f64 - 1.0,
        hybrid_stream as f64 / current_stream as f64 - 1.0,
        model.squared_error,
        model.max_error
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn model_tile(
    plane_index: usize,
    plane: &[u16],
    plane_width: usize,
    origin_x: usize,
    origin_y: usize,
    width: usize,
    height: usize,
    max_sample: u16,
    step: i32,
) -> Model {
    let mut reconstructed_row = vec![0u16; width];
    let mut folded = Vec::with_capacity(width * height);
    let mut squared_error = 0u64;
    let mut max_error = 0u32;
    for y in 0..height {
        let row_start = (origin_y + y) * plane_width + origin_x;
        let mut left = 0u16;
        let mut upper_left = 0u16;
        for (x, &sample) in plane[row_start..row_start + width].iter().enumerate() {
            let above = reconstructed_row[x];
            let prediction = (i32::from(left) + i32::from(above) - i32::from(upper_left))
                .clamp(0, i32::from(max_sample));
            let quantized = quantize(i32::from(sample) - prediction, step);
            let reconstructed =
                (prediction + quantized * step).clamp(0, i32::from(max_sample)) as u16;
            let error = (i32::from(sample) - i32::from(reconstructed)).unsigned_abs();
            squared_error += u64::from(error) * u64::from(error);
            max_error = max_error.max(error);
            reconstructed_row[x] = reconstructed;
            upper_left = above;
            left = reconstructed;
            folded.push(zigzag(quantized));
        }
    }
    let current_payload = entropy_bytes(&folded);
    let packed_payload = block_pack_bytes(&folded);
    Model {
        current_payload,
        packed_payload,
        hybrid_payload: current_payload.min(packed_payload),
        tiles: 1,
        packed_tiles: usize::from(packed_payload < current_payload),
        packed_tiles_by_plane: std::array::from_fn(|plane| {
            usize::from(plane == plane_index && packed_payload < current_payload)
        }),
        savings_by_plane: std::array::from_fn(|plane| {
            if plane == plane_index {
                current_payload.saturating_sub(packed_payload)
            } else {
                0
            }
        }),
        squared_error,
        max_error,
    }
}

fn block_pack_bytes(folded: &[u32]) -> usize {
    folded
        .chunks(BLOCK_SYMBOLS)
        .map(|block| {
            let maximum = block.iter().copied().max().unwrap_or(0);
            let width = (u32::BITS - maximum.leading_zeros()) as usize;
            1 + (block.len() * width).div_ceil(8)
        })
        .sum()
}

fn entropy_bytes(folded: &[u32]) -> usize {
    let mut zero_run_bytes = 0usize;
    let mut zero_run = 0u32;
    for &value in folded {
        if value == 0 {
            zero_run += 1;
        } else {
            if zero_run != 0 {
                zero_run_bytes += varint_length((zero_run - 1) * 2);
                zero_run = 0;
            }
            zero_run_bytes += varint_length(value * 2 - 1);
        }
    }
    if zero_run != 0 {
        zero_run_bytes += varint_length((zero_run - 1) * 2);
    }
    let rice_bytes = (0..=MAX_RICE_PARAMETER)
        .map(|parameter| {
            folded.iter().fold(0usize, |bits, &value| {
                bits + (value >> parameter) as usize + 1 + usize::from(parameter)
            })
        })
        .min()
        .expect("Rice parameter range is nonempty")
        .div_ceil(8);
    zero_run_bytes.min(rice_bytes)
}

fn quantize(value: i32, step: i32) -> i32 {
    let magnitude = (value.abs() + step / 2) / step;
    if value < 0 { -magnitude } else { magnitude }
}

fn zigzag(value: i32) -> u32 {
    ((value << 1) ^ (value >> 31)) as u32
}

fn varint_length(mut value: u32) -> usize {
    let mut length = 1;
    while value >= 0x80 {
        value >>= 7;
        length += 1;
    }
    length
}

fn add(left: Model, right: Model) -> Model {
    Model {
        current_payload: left.current_payload + right.current_payload,
        packed_payload: left.packed_payload + right.packed_payload,
        hybrid_payload: left.hybrid_payload + right.hybrid_payload,
        tiles: left.tiles + right.tiles,
        packed_tiles: left.packed_tiles + right.packed_tiles,
        packed_tiles_by_plane: std::array::from_fn(|plane| {
            left.packed_tiles_by_plane[plane] + right.packed_tiles_by_plane[plane]
        }),
        savings_by_plane: std::array::from_fn(|plane| {
            left.savings_by_plane[plane] + right.savings_by_plane[plane]
        }),
        squared_error: left.squared_error + right.squared_error,
        max_error: left.max_error.max(right.max_error),
    }
}
