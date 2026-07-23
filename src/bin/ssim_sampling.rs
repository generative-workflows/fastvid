use fastvid::{
    CodecOptions, Frame, FrameRate, decode, decode_with_reference, encode, encode_with_reference,
    ssim_plane_sampled,
};
use std::env;
use std::fs;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration, Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ssim_sampling: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() != 8 {
        return Err(
            "usage: ssim_sampling INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS GOP"
                .into(),
        );
    }
    let input = &arguments[0];
    let width: u32 = arguments[1].parse()?;
    let height: u32 = arguments[2].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[3])?;
    let frame_count: usize = arguments[4].parse()?;
    let quality: u8 = arguments[5].parse()?;
    let threads: usize = arguments[6].parse()?;
    let gop: usize = arguments[7].parse()?;
    if frame_count == 0 || gop == 0 {
        return Err("frame count and GOP must be nonzero".into());
    }

    let raw = fs::read(input)?;
    let y_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    let chroma_len = (width.div_ceil(2) as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    let frame_len = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    if raw.len()
        != frame_len
            .checked_mul(frame_count)
            .ok_or("input is too large")?
    {
        return Err("input length does not match the declared YUV422p8 sequence".into());
    }

    let frame_rate = FrameRate::new(fps_numerator, fps_denominator);
    let options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    let strides = [1usize, 2, 5];
    let mut score_sums = [0.0; 3];
    let mut metric_times = [Duration::ZERO; 3];
    let mut previous = None;

    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = frame_from_yuv422(bytes, width, height, frame_rate)?;
        let predicted = !frame_index.is_multiple_of(gop);
        let encoded = if predicted {
            encode_with_reference(
                &frame,
                previous
                    .as_ref()
                    .expect("predicted frame has a preceding reconstruction"),
                options,
            )?
        } else {
            encode(&frame, options)?
        };
        let decoded = if predicted {
            decode_with_reference(
                &encoded,
                previous
                    .as_ref()
                    .expect("predicted frame has a preceding reconstruction"),
                threads,
            )?
        } else {
            decode(&encoded, threads)?
        };

        let mut frame_scores = [None; 3];
        // Rotate the first metric across repetitions so no stride always gets
        // the coldest cache. Three timings per stride are accumulated.
        for repetition in 0..3 {
            for offset in 0..3 {
                let index = (frame_index + repetition + offset) % 3;
                let start = Instant::now();
                let score = ssim_plane_sampled(
                    &frame.planes[0].data,
                    &decoded.planes[0].data,
                    width as usize,
                    height as usize,
                    strides[index],
                )
                .ok_or("SSIM input mismatch")?;
                metric_times[index] += start.elapsed();
                black_box(score);
                if let Some(previous_score) = frame_scores[index] {
                    if previous_score != score {
                        return Err("SSIM result changed across repetitions".into());
                    }
                } else {
                    frame_scores[index] = Some(score);
                }
            }
        }
        for (sum, score) in score_sums.iter_mut().zip(frame_scores) {
            *sum += score.expect("every stride was evaluated");
        }
        previous = Some(decoded);
    }

    let scores = score_sums.map(|sum| sum / frame_count as f64);
    let metric_ms = metric_times.map(|time| time.as_secs_f64() * 1000.0 / 3.0);
    let blocks_wide = (width as usize).div_ceil(8);
    let blocks_high = (height as usize).div_ceil(8);
    let block_counts =
        strides.map(|stride| blocks_wide.div_ceil(stride) * blocks_high.div_ceil(stride));
    println!(
        "input\twidth\theight\tframes\tquality\tthreads\tgop\texact_ssim\tsample2_ssim\tsample5_ssim\tsample2_abs_error\tsample5_abs_error\texact_metric_ms\tsample2_metric_ms\tsample5_metric_ms\tsample2_speedup\tsample5_speedup\texact_blocks_per_frame\tsample2_blocks_per_frame\tsample5_blocks_per_frame"
    );
    println!(
        "{input}\t{width}\t{height}\t{frame_count}\t{quality}\t{threads}\t{gop}\t{:.12}\t{:.12}\t{:.12}\t{:.12}\t{:.12}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}",
        scores[0],
        scores[1],
        scores[2],
        (scores[1] - scores[0]).abs(),
        (scores[2] - scores[0]).abs(),
        metric_ms[0],
        metric_ms[1],
        metric_ms[2],
        metric_ms[0] / metric_ms[1],
        metric_ms[0] / metric_ms[2],
        block_counts[0],
        block_counts[1],
        block_counts[2],
    );
    Ok(())
}

fn frame_from_yuv422(
    raw: &[u8],
    width: u32,
    height: u32,
    frame_rate: FrameRate,
) -> Result<Frame, Box<dyn std::error::Error>> {
    let y_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    let chroma_len = (width.div_ceil(2) as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    if raw.len() != y_len + 2 * chroma_len {
        return Err("input length does not match one planar YUV422p8 frame".into());
    }
    Ok(Frame::yuv422p8(
        width,
        height,
        frame_rate,
        raw[..y_len].to_vec(),
        raw[y_len..y_len + chroma_len].to_vec(),
        raw[y_len + chroma_len..].to_vec(),
    )?)
}

fn parse_rate(value: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or("frame rate must be NUMERATOR/DENOMINATOR")?;
    Ok((numerator.parse()?, denominator.parse()?))
}
