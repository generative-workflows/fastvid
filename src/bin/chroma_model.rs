use fastvid::{CodecOptions, Frame, FrameRate, analyze_chroma_from_luma};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("chroma_model: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() != 9 {
        return Err(
            "usage: chroma_model SAMPLE CATEGORY INPUT WIDTH HEIGHT FPS FRAMES QUALITY THREADS"
                .into(),
        );
    }
    let sample = &arguments[0];
    let category = &arguments[1];
    let raw = fs::read(&arguments[2])?;
    let width: u32 = arguments[3].parse()?;
    let height: u32 = arguments[4].parse()?;
    let frame_rate = parse_rate(&arguments[5])?;
    let frame_count: usize = arguments[6].parse()?;
    let quality: u8 = arguments[7].parse()?;
    let threads: usize = arguments[8].parse()?;
    if frame_count == 0 {
        return Err("frame count must be nonzero".into());
    }
    let y_len = area(width, height)?;
    let chroma_len = area(width.div_ceil(2), height)?;
    let frame_len = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    if raw.len() != frame_len * frame_count {
        return Err("input length does not match dimensions and frame count".into());
    }
    let options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    println!(
        "sample\tcategory\tframe\tquality\tcurrent_stream_bytes\ttile\tplane\tx\ty\twidth\theight\tsamples\tcurrent_bytes\tcfl_entropy_bytes\tcfl_control_bytes\tcfl_complete_bytes\tdc\talpha_eighths\tsquared_error\tmax_error"
    );
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = Frame::yuv422p8(
            width,
            height,
            frame_rate,
            bytes[..y_len].to_vec(),
            bytes[y_len..y_len + chroma_len].to_vec(),
            bytes[y_len + chroma_len..].to_vec(),
        )?;
        for (tile_index, model) in analyze_chroma_from_luma(&frame, options)?
            .into_iter()
            .enumerate()
        {
            println!(
                "{sample}\t{category}\t{frame_index}\t{quality}\t{}\t{tile_index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                model.current_stream_bytes,
                model.plane,
                model.x,
                model.y,
                model.width,
                model.height,
                model.sample_count,
                model.current_payload_bytes,
                model.cfl_entropy_bytes,
                model.cfl_control_bytes,
                model.cfl_complete_bytes,
                model.dc,
                model.alpha_eighths,
                model.squared_error,
                model.max_error,
            );
        }
    }
    Ok(())
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
