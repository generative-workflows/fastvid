use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

const BLOCK: usize = 64;
const SEARCH_RADIUS: isize = 16;
const SEARCH_STEP: usize = 4;
const SAMPLE_STEP: usize = 4;
const VECTOR_BITS: u64 = 16;

struct BlockResult {
    frame: usize,
    block: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    baseline_bits: u64,
    candidate_bits: u64,
    selected: bool,
    dx: isize,
    dy: isize,
    baseline_sad: u64,
    candidate_sad: u64,
    evaluations: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("motion_model: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() != 8 {
        return Err(
            "usage: motion_model SAMPLE INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY GOP"
                .into(),
        );
    }
    let sample = &arguments[0];
    let raw = fs::read(&arguments[1])?;
    let width: usize = arguments[2].parse()?;
    let height: usize = arguments[3].parse()?;
    validate_rate(&arguments[4])?;
    let frames: usize = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    let gop: usize = arguments[7].parse()?;
    if width == 0 || height == 0 || frames == 0 || gop == 0 || quality > 100 {
        return Err(
            "dimensions, frame count, and GOP must be nonzero; quality must be <= 100".into(),
        );
    }
    let y_len = width.checked_mul(height).ok_or("frame is too large")?;
    let chroma_width = width.div_ceil(2);
    let chroma_len = chroma_width
        .checked_mul(height)
        .ok_or("frame is too large")?;
    let frame_len = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    if raw.len() != frame_len.checked_mul(frames).ok_or("input is too large")? {
        return Err("input length does not match the declared YUV422p8 sequence".into());
    }
    let step = 1 + i32::from((100 - quality) / 5);
    let start = Instant::now();
    let mut results = Vec::new();
    for frame in 1..frames {
        if frame.is_multiple_of(gop) {
            continue;
        }
        let current = &raw[frame * frame_len..(frame + 1) * frame_len];
        let reference = &raw[(frame - 1) * frame_len..frame * frame_len];
        let current_planes = [
            &current[..y_len],
            &current[y_len..y_len + chroma_len],
            &current[y_len + chroma_len..],
        ];
        let reference_planes = [
            &reference[..y_len],
            &reference[y_len..y_len + chroma_len],
            &reference[y_len + chroma_len..],
        ];
        let mut block = 0;
        for y in (0..height).step_by(BLOCK) {
            for x in (0..width).step_by(BLOCK) {
                let block_width = BLOCK.min(width - x);
                let block_height = BLOCK.min(height - y);
                let baseline_sad = sampled_sad(
                    current_planes[0],
                    reference_planes[0],
                    width,
                    x,
                    y,
                    block_width,
                    block_height,
                    0,
                    0,
                );
                let mut best = (baseline_sad, 0isize, 0isize);
                let mut evaluations = 0;
                for dy in (-SEARCH_RADIUS..=SEARCH_RADIUS).step_by(SEARCH_STEP) {
                    for dx in (-SEARCH_RADIUS..=SEARCH_RADIUS).step_by(SEARCH_STEP) {
                        if !candidate_fits(width, height, x, y, block_width, block_height, dx, dy) {
                            continue;
                        }
                        evaluations += 1;
                        let sad = sampled_sad(
                            current_planes[0],
                            reference_planes[0],
                            width,
                            x,
                            y,
                            block_width,
                            block_height,
                            dx,
                            dy,
                        );
                        if (sad, dx.unsigned_abs() + dy.unsigned_abs(), dy, dx)
                            < (
                                best.0,
                                best.1.unsigned_abs() + best.2.unsigned_abs(),
                                best.2,
                                best.1,
                            )
                        {
                            best = (sad, dx, dy);
                        }
                    }
                }
                let baseline_bits = block_rice_bits(
                    current_planes,
                    reference_planes,
                    width,
                    chroma_width,
                    x,
                    y,
                    block_width,
                    block_height,
                    0,
                    0,
                    step,
                );
                let translated_bits = block_rice_bits(
                    current_planes,
                    reference_planes,
                    width,
                    chroma_width,
                    x,
                    y,
                    block_width,
                    block_height,
                    best.1,
                    best.2,
                    step,
                )
                .saturating_add(VECTOR_BITS);
                let selected = (best.1 != 0 || best.2 != 0) && translated_bits < baseline_bits;
                results.push(BlockResult {
                    frame,
                    block,
                    x,
                    y,
                    width: block_width,
                    height: block_height,
                    baseline_bits,
                    candidate_bits: if selected {
                        translated_bits
                    } else {
                        baseline_bits
                    },
                    selected,
                    dx: if selected { best.1 } else { 0 },
                    dy: if selected { best.2 } else { 0 },
                    baseline_sad,
                    candidate_sad: if selected { best.0 } else { baseline_sad },
                    evaluations,
                });
                block += 1;
            }
        }
    }
    let model_ms = start.elapsed().as_secs_f64() * 1000.0;
    let modeled_pixels: usize = results
        .iter()
        .map(|result| result.width * result.height)
        .sum();
    let model_mpps = modeled_pixels as f64 / model_ms / 1000.0;
    println!(
        "sample\tquality\tgop\tframe\tblock\tx\ty\twidth\theight\tbaseline_bits\tcandidate_bits\tselected\tdx\tdy\tbaseline_sad\tcandidate_sad\tsearch_evaluations\tmodel_ms\tmodel_mpps"
    );
    for result in results {
        println!(
            "{sample}\t{quality}\t{gop}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{model_ms:.3}\t{model_mpps:.3}",
            result.frame,
            result.block,
            result.x,
            result.y,
            result.width,
            result.height,
            result.baseline_bits,
            result.candidate_bits,
            u8::from(result.selected),
            result.dx,
            result.dy,
            result.baseline_sad,
            result.candidate_sad,
            result.evaluations,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn sampled_sad(
    current: &[u8],
    reference: &[u8],
    stride: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    dx: isize,
    dy: isize,
) -> u64 {
    let reference_x = x.wrapping_add_signed(dx);
    let reference_y = y.wrapping_add_signed(dy);
    let mut sad = 0u64;
    for row in (0..height).step_by(SAMPLE_STEP) {
        let current_row = (y + row) * stride + x;
        let reference_row = (reference_y + row) * stride + reference_x;
        for column in (0..width).step_by(SAMPLE_STEP) {
            sad += u64::from(
                current[current_row + column].abs_diff(reference[reference_row + column]),
            );
        }
    }
    sad
}

#[allow(clippy::too_many_arguments)]
fn block_rice_bits(
    current: [&[u8]; 3],
    reference: [&[u8]; 3],
    luma_stride: usize,
    chroma_stride: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    dx: isize,
    dy: isize,
    step: i32,
) -> u64 {
    let mut bits = plane_rice_bits(
        current[0],
        reference[0],
        luma_stride,
        x,
        y,
        width,
        height,
        dx,
        dy,
        step,
    );
    let chroma_x = x / 2;
    let chroma_width = width.div_ceil(2);
    for plane in 1..3 {
        bits += plane_rice_bits(
            current[plane],
            reference[plane],
            chroma_stride,
            chroma_x,
            y,
            chroma_width,
            height,
            dx / 2,
            dy,
            step,
        );
    }
    bits
}

#[allow(clippy::too_many_arguments)]
fn plane_rice_bits(
    current: &[u8],
    reference: &[u8],
    stride: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    dx: isize,
    dy: isize,
    step: i32,
) -> u64 {
    let reference_x = x.wrapping_add_signed(dx);
    let reference_y = y.wrapping_add_signed(dy);
    let mut costs = [3u64; 8];
    for row in 0..height {
        let current_row = (y + row) * stride + x;
        let reference_row = (reference_y + row) * stride + reference_x;
        for column in 0..width {
            let residual = i32::from(current[current_row + column])
                - i32::from(reference[reference_row + column]);
            let quantized = quantize(residual, step);
            let folded = zigzag(quantized);
            for (parameter, cost) in costs.iter_mut().enumerate() {
                *cost += u64::from(folded >> parameter) + 1 + parameter as u64;
            }
        }
    }
    costs
        .into_iter()
        .min()
        .expect("Rice parameter set is nonempty")
}

#[allow(clippy::too_many_arguments)]
fn candidate_fits(
    frame_width: usize,
    frame_height: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    dx: isize,
    dy: isize,
) -> bool {
    x.checked_add_signed(dx)
        .and_then(|start| start.checked_add(width))
        .is_some_and(|end| end <= frame_width)
        && y.checked_add_signed(dy)
            .and_then(|start| start.checked_add(height))
            .is_some_and(|end| end <= frame_height)
}

fn quantize(value: i32, step: i32) -> i32 {
    let magnitude = (value.abs() + step / 2) / step;
    if value < 0 { -magnitude } else { magnitude }
}

fn zigzag(value: i32) -> u32 {
    if value >= 0 {
        value as u32 * 2
    } else {
        value.unsigned_abs() * 2 - 1
    }
}

fn validate_rate(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or("frame rate must be NUMERATOR/DENOMINATOR")?;
    let numerator: u32 = numerator.parse()?;
    let denominator: u32 = denominator.parse()?;
    if numerator == 0 || denominator == 0 {
        return Err("frame rate terms must be nonzero".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_bounds_reject_every_out_of_frame_shift() {
        assert!(candidate_fits(1920, 1080, 64, 64, 64, 64, -16, -16));
        assert!(!candidate_fits(1920, 1080, 0, 0, 64, 64, -4, 0));
        assert!(!candidate_fits(1920, 1080, 1856, 0, 64, 64, 4, 0));
        assert!(!candidate_fits(1920, 1080, 0, 1024, 64, 56, 0, 4));
    }

    #[test]
    fn zero_translation_has_minimal_constant_block_cost() {
        let plane = vec![37u8; 64 * 64];
        assert_eq!(
            plane_rice_bits(&plane, &plane, 64, 0, 0, 64, 64, 0, 0, 1),
            3 + 64 * 64
        );
    }
}
