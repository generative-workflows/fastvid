use fastvid::{CodecOptions, Frame16, FrameRate, Plane16, analyze_entropy16, encode16};
use std::env;
use std::fs;
use std::process::ExitCode;

const TILE_WIDTH: usize = 256;
const TILE_HEIGHT: usize = 128;
const MAX_RICE_PARAMETER: u8 = 16;

#[derive(Clone)]
struct Band {
    width: usize,
    height: usize,
    values: Vec<i32>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("squeeze_model: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() != 8 || arguments[0] != "yuv422p16le" {
        return Err(
            "usage: squeeze_model yuv422p16le SAMPLE INPUT WIDTH HEIGHT FPS FRAMES BIT_DEPTH"
                .into(),
        );
    }
    let sample = &arguments[1];
    let input = &arguments[2];
    let width: u32 = arguments[3].parse()?;
    let height: u32 = arguments[4].parse()?;
    let frame_rate = parse_rate(&arguments[5])?;
    let frames: usize = arguments[6].parse()?;
    let bit_depth: u8 = arguments[7].parse()?;
    let raw = fs::read(input)?;
    let y_len = area(width, height)?;
    let chroma_width = width.div_ceil(2);
    let chroma_len = area(chroma_width, height)?;
    let samples_per_frame = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    let bytes_per_frame = samples_per_frame
        .checked_mul(2)
        .ok_or("frame is too large")?;
    if frames == 0 || raw.len() != frames * bytes_per_frame {
        return Err("input length does not match dimensions and frame count".into());
    }

    print_header();
    for (frame_index, bytes) in raw.chunks_exact(bytes_per_frame).enumerate() {
        let values: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let frame = Frame16::yuv422(
            width,
            height,
            bit_depth,
            frame_rate,
            values[..y_len].to_vec(),
            values[y_len..y_len + chroma_len].to_vec(),
            values[y_len + chroma_len..].to_vec(),
        )?;
        let encoded = encode16(
            &frame,
            CodecOptions {
                quality: 100,
                threads: 1,
                ..CodecOptions::default()
            },
        )?;
        let current = analyze_entropy16(&encoded)?;
        let mut model_index = 0usize;
        for (plane_index, plane) in frame.planes.iter().enumerate() {
            let nominal_width = if plane_index == 0 {
                TILE_WIDTH
            } else {
                TILE_WIDTH.div_ceil(2)
            };
            for tile_y in (0..plane.height as usize).step_by(TILE_HEIGHT) {
                for tile_x in (0..plane.width as usize).step_by(nominal_width) {
                    let tile = extract_tile(plane, tile_x, tile_y, nominal_width, TILE_HEIGHT);
                    let current_bytes = current
                        .get(model_index)
                        .ok_or("entropy model omitted a tile")?
                        .actual_payload_bytes;
                    let horizontal_bands = horizontal_split(&tile);
                    if inverse_horizontal(&horizontal_bands) != tile.values {
                        return Err("horizontal transform did not invert exactly".into());
                    }
                    let vertical_bands = vertical_split(&tile);
                    if inverse_vertical(&vertical_bands) != tile.values {
                        return Err("vertical transform did not invert exactly".into());
                    }
                    let two_dimensional_bands = two_dimensional_split(&tile);
                    if inverse_two_dimensional(&two_dimensional_bands) != tile.values {
                        return Err("two-dimensional transform did not invert exactly".into());
                    }
                    let horizontal = split_cost(&horizontal_bands);
                    let vertical = split_cost(&vertical_bands);
                    let two_dimensional = split_cost(&two_dimensional_bands);
                    let candidates = [
                        ("current", current_bytes),
                        ("horizontal", horizontal),
                        ("vertical", vertical),
                        ("two-dimensional", two_dimensional),
                    ];
                    let (winner, best) = candidates
                        .into_iter()
                        .min_by_key(|(name, bytes)| (*bytes, *name != "current", *name))
                        .expect("candidate set is nonempty");
                    println!(
                        "{sample}\t{frame_index}\t{bit_depth}\t{plane_index}\t{model_index}\t{}\t{}\t{current_bytes}\t{horizontal}\t{vertical}\t{two_dimensional}\t{best}\t{winner}",
                        tile.width, tile.height
                    );
                    model_index += 1;
                }
            }
        }
        if model_index != current.len() {
            return Err("tile traversal disagrees with entropy model".into());
        }
    }
    Ok(())
}

fn print_header() {
    println!(
        "sample\tframe\tbit_depth\tplane\ttile\twidth\theight\tcurrent_bytes\t\
         horizontal_bytes\tvertical_bytes\ttwo_dimensional_bytes\tbest_bytes\twinner"
    );
}

fn extract_tile(
    plane: &Plane16,
    x: usize,
    y: usize,
    nominal_width: usize,
    nominal_height: usize,
) -> Band {
    let width = nominal_width.min(plane.width as usize - x);
    let height = nominal_height.min(plane.height as usize - y);
    let mut values = Vec::with_capacity(width * height);
    for row in y..y + height {
        let start = row * plane.width as usize + x;
        values.extend(
            plane.data[start..start + width]
                .iter()
                .map(|&sample| i32::from(sample)),
        );
    }
    Band {
        width,
        height,
        values,
    }
}

fn horizontal_split(input: &Band) -> Vec<Band> {
    let low_width = input.width.div_ceil(2);
    let high_width = input.width / 2;
    let mut low = Vec::with_capacity(low_width * input.height);
    let mut high = Vec::with_capacity(high_width * input.height);
    for row in input.values.chunks_exact(input.width) {
        for pair in row.chunks_exact(2) {
            let (average, detail) = forward_pair(pair[0], pair[1]);
            low.push(average);
            high.push(detail);
        }
        if input.width & 1 != 0 {
            low.push(row[input.width - 1]);
        }
    }
    vec![
        Band {
            width: low_width,
            height: input.height,
            values: low,
        },
        Band {
            width: high_width,
            height: input.height,
            values: high,
        },
    ]
}

fn vertical_split(input: &Band) -> Vec<Band> {
    let low_height = input.height.div_ceil(2);
    let high_height = input.height / 2;
    let mut low = Vec::with_capacity(input.width * low_height);
    let mut high = Vec::with_capacity(input.width * high_height);
    for pair_y in 0..high_height {
        let first = 2 * pair_y * input.width;
        let second = first + input.width;
        for x in 0..input.width {
            let (average, detail) = forward_pair(input.values[first + x], input.values[second + x]);
            low.push(average);
            high.push(detail);
        }
    }
    if input.height & 1 != 0 {
        low.extend_from_slice(&input.values[(input.height - 1) * input.width..]);
    }
    vec![
        Band {
            width: input.width,
            height: low_height,
            values: low,
        },
        Band {
            width: input.width,
            height: high_height,
            values: high,
        },
    ]
}

fn two_dimensional_split(input: &Band) -> Vec<Band> {
    let horizontal = horizontal_split(input);
    let mut output = vertical_split(&horizontal[0]);
    output.extend(vertical_split(&horizontal[1]));
    output
}

fn inverse_horizontal(bands: &[Band]) -> Vec<i32> {
    let low = &bands[0];
    let high = &bands[1];
    let width = low.width + high.width;
    let mut output = Vec::with_capacity(width * low.height);
    for y in 0..low.height {
        let low_row = &low.values[y * low.width..(y + 1) * low.width];
        let high_row = &high.values[y * high.width..(y + 1) * high.width];
        for x in 0..high.width {
            let pair = inverse_pair(low_row[x], high_row[x]);
            output.push(pair.0);
            output.push(pair.1);
        }
        if width & 1 != 0 {
            output.push(low_row[low.width - 1]);
        }
    }
    output
}

fn inverse_vertical(bands: &[Band]) -> Vec<i32> {
    let low = &bands[0];
    let high = &bands[1];
    let height = low.height + high.height;
    let mut output = vec![0; low.width * height];
    for y in 0..high.height {
        for x in 0..low.width {
            let pair = inverse_pair(
                low.values[y * low.width + x],
                high.values[y * high.width + x],
            );
            output[2 * y * low.width + x] = pair.0;
            output[(2 * y + 1) * low.width + x] = pair.1;
        }
    }
    if height & 1 != 0 {
        output[(height - 1) * low.width..]
            .copy_from_slice(&low.values[(low.height - 1) * low.width..low.height * low.width]);
    }
    output
}

fn inverse_two_dimensional(bands: &[Band]) -> Vec<i32> {
    let low_horizontal = Band {
        width: bands[0].width,
        height: bands[0].height + bands[1].height,
        values: inverse_vertical(&bands[..2]),
    };
    let high_horizontal = Band {
        width: bands[2].width,
        height: bands[2].height + bands[3].height,
        values: inverse_vertical(&bands[2..]),
    };
    inverse_horizontal(&[low_horizontal, high_horizontal])
}

fn split_cost(bands: &[Band]) -> usize {
    let payloads: Vec<usize> = bands.iter().map(band_payload_cost).collect();
    1 + payloads
        .iter()
        .take(payloads.len() - 1)
        .map(|&bytes| varint_length(bytes as u64))
        .sum::<usize>()
        + payloads.iter().sum::<usize>()
}

fn band_payload_cost(band: &Band) -> usize {
    if band.values.is_empty() {
        return 0;
    }
    let mut folded = Vec::with_capacity(band.values.len());
    let mut above = vec![0i32; band.width];
    for row in band.values.chunks_exact(band.width) {
        let mut left = 0i32;
        let mut upper_left = 0i32;
        for (x, &value) in row.iter().enumerate() {
            let prediction = paeth(left, above[x], upper_left);
            folded.push(zigzag(value - prediction));
            upper_left = above[x];
            above[x] = value;
            left = value;
        }
    }
    entropy_payload_cost(&folded)
}

fn entropy_payload_cost(folded: &[u32]) -> usize {
    let mut zero_run_bytes = 0usize;
    let mut run = 0u64;
    for &value in folded {
        if value == 0 {
            run += 1;
        } else {
            count_zero_run(&mut zero_run_bytes, &mut run);
            zero_run_bytes += varint_length(u64::from(value) * 2 - 1);
        }
    }
    count_zero_run(&mut zero_run_bytes, &mut run);

    let mut best_rice_bits = u64::MAX;
    for parameter in 0..=MAX_RICE_PARAMETER {
        let quotient_sum = folded
            .iter()
            .map(|&value| u64::from(value >> parameter))
            .sum::<u64>();
        let bits = folded.len() as u64 * (u64::from(parameter) + 1) + quotient_sum;
        best_rice_bits = best_rice_bits.min(bits);
        if quotient_sum == 0 {
            break;
        }
    }
    zero_run_bytes.min(best_rice_bits.div_ceil(8) as usize)
}

fn count_zero_run(bytes: &mut usize, run: &mut u64) {
    if *run != 0 {
        *bytes += varint_length((*run - 1) * 2);
        *run = 0;
    }
}

fn forward_pair(first: i32, second: i32) -> (i32, i32) {
    let average = (first + second + i32::from(first > second)) >> 1;
    (average, first - second)
}

fn inverse_pair(average: i32, detail: i32) -> (i32, i32) {
    let first = average + detail / 2;
    (first, first - detail)
}

fn paeth(left: i32, above: i32, upper_left: i32) -> i32 {
    let estimate = left + above - upper_left;
    let left_distance = (estimate - left).abs();
    let above_distance = (estimate - above).abs();
    let upper_left_distance = (estimate - upper_left).abs();
    if left_distance <= above_distance && left_distance <= upper_left_distance {
        left
    } else if above_distance <= upper_left_distance {
        above
    } else {
        upper_left
    }
}

fn zigzag(value: i32) -> u32 {
    if value >= 0 {
        value as u32 * 2
    } else {
        value.unsigned_abs() * 2 - 1
    }
}

fn varint_length(value: u64) -> usize {
    (64 - (value | 1).leading_zeros() as usize).div_ceil(7)
}

fn parse_rate(value: &str) -> Result<FrameRate, Box<dyn std::error::Error>> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or("frame rate must be NUMERATOR/DENOMINATOR")?;
    Ok(FrameRate::new(numerator.parse()?, denominator.parse()?))
}

fn area(width: u32, height: u32) -> Result<usize, Box<dyn std::error::Error>> {
    (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "frame is too large".into())
}

#[cfg(test)]
mod tests {
    use super::{forward_pair, inverse_pair};

    #[test]
    fn reversible_pair_covers_unsigned_boundaries() {
        for first in 0..=u16::MAX {
            for second in [0, 1, 32767, 32768, 65534, 65535, first] {
                let transformed = forward_pair(i32::from(first), i32::from(second));
                assert_eq!(
                    inverse_pair(transformed.0, transformed.1),
                    (i32::from(first), i32::from(second))
                );
            }
        }
    }

    #[test]
    fn reversible_pair_covers_signed_detail_extrema() {
        let values = [-131070, -65535, -1, 0, 1, 65535, 131070];
        for &first in &values {
            for &second in &values {
                let transformed = forward_pair(first, second);
                assert_eq!(inverse_pair(transformed.0, transformed.1), (first, second));
            }
        }
    }
}
