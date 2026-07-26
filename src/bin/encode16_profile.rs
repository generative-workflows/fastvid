use fastvid::{CodecOptions, Frame16, FrameRate, encode16, encode16_parallel_full_tile};
use std::env;
use std::fs;
use std::hint::black_box;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("encode16_profile: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if !(8..=10).contains(&arguments.len()) {
        return Err(
            "usage: encode16_profile INPUT WIDTH HEIGHT FRAMES BIT_DEPTH QUALITY TILE_WIDTH TILE_HEIGHT [REPETITIONS [VARIANT]]"
                .into(),
        );
    }
    let width: u32 = arguments[1].parse()?;
    let height: u32 = arguments[2].parse()?;
    let frames: usize = arguments[3].parse()?;
    let bit_depth: u8 = arguments[4].parse()?;
    let quality: u8 = arguments[5].parse()?;
    let tile_width: u16 = arguments[6].parse()?;
    let tile_height: u16 = arguments[7].parse()?;
    let repetitions: usize = arguments.get(8).map_or(Ok(1), |value| value.parse())?;
    let variant = arguments.get(9).map(String::as_str).unwrap_or("baseline");
    if !matches!(variant, "baseline" | "bounded-full-tile") {
        return Err("variant must be baseline or bounded-full-tile".into());
    }
    if repetitions == 0 {
        return Err("repetitions must be nonzero".into());
    }
    let y_samples = width as usize * height as usize;
    let chroma_samples = width.div_ceil(2) as usize * height as usize;
    let frame_samples = y_samples + 2 * chroma_samples;
    let bytes = fs::read(&arguments[0])?;
    if bytes.len() != frame_samples * frames * 2 {
        return Err("input length does not match dimensions and frame count".into());
    }
    let samples: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let mut input_frames = Vec::with_capacity(frames);
    for frame in samples.chunks_exact(frame_samples) {
        input_frames.push(Frame16::yuv422(
            width,
            height,
            bit_depth,
            FrameRate::new(24, 1),
            frame[..y_samples].to_vec(),
            frame[y_samples..y_samples + chroma_samples].to_vec(),
            frame[y_samples + chroma_samples..].to_vec(),
        )?);
    }
    let options = CodecOptions {
        quality,
        tile_width,
        tile_height,
        threads: 1,
    };
    let mut encoded_bytes = 0usize;
    for _ in 0..repetitions {
        for frame in &input_frames {
            let encoded = if variant == "bounded-full-tile" {
                encode16_parallel_full_tile(black_box(frame), options)?
            } else {
                encode16(black_box(frame), options)?
            };
            encoded_bytes += black_box(encoded.len());
        }
    }
    println!("frames\trepetitions\tencoded_bytes");
    println!("{frames}\t{repetitions}\t{encoded_bytes}");
    Ok(())
}
