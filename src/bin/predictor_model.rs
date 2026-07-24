use fastvid::{
    CodecOptions, Frame, Frame16, FrameRate, PredictorModelMode, TilePredictorModel,
    analyze_predictor_frame, analyze_predictor_frame16, decode, decode_with_reference, decode16,
    decode16_with_reference, encode, encode_with_reference, encode16, encode16_with_reference,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("predictor_model: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("yuv422") => model_8bit(&arguments[1..]),
        Some("yuv422p16le") => model_high_bit(&arguments[1..]),
        _ => Err(
            "usage: predictor_model yuv422 SAMPLE INPUT WIDTH HEIGHT FPS FRAMES QUALITY THREADS GOP\n\
             or: predictor_model yuv422p16le SAMPLE INPUT WIDTH HEIGHT FPS FRAMES BIT_DEPTH QUALITY THREADS GOP"
                .into(),
        ),
    }
}

fn model_8bit(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 9 {
        return Err("yuv422 mode needs 9 arguments".into());
    }
    let sample = &arguments[0];
    let raw = fs::read(&arguments[1])?;
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let frame_rate = parse_rate(&arguments[4])?;
    let frame_count: usize = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    let threads: usize = arguments[7].parse()?;
    let gop: usize = arguments[8].parse()?;
    validate_counts(frame_count, gop)?;
    let y_len = area(width, height)?;
    let chroma_len = area(width.div_ceil(2), height)?;
    let frame_len = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    validate_sequence(raw.len(), frame_len, frame_count)?;
    let options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    print_header();
    let mut previous = None;
    let mut oracle_previous = None;
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = Frame::yuv422p8(
            width,
            height,
            frame_rate,
            bytes[..y_len].to_vec(),
            bytes[y_len..y_len + chroma_len].to_vec(),
            bytes[y_len + chroma_len..].to_vec(),
        )?;
        let predicted = !frame_index.is_multiple_of(gop);
        let reference = predicted.then(|| {
            previous
                .as_ref()
                .expect("predicted frame has a reconstructed reference")
        });
        let oracle_reference = predicted.then(|| {
            oracle_previous
                .as_ref()
                .expect("predicted frame has an oracle reference")
        });
        let (models, oracle_reconstruction) =
            analyze_predictor_frame(&frame, reference, oracle_reference, options)?;
        print_models(sample, frame_index, 8, quality, gop, &models);
        let encoded = if let Some(reference) = reference {
            encode_with_reference(&frame, reference, options)?
        } else {
            encode(&frame, options)?
        };
        previous = Some(if let Some(reference) = reference {
            decode_with_reference(&encoded, reference, threads)?
        } else {
            decode(&encoded, threads)?
        });
        oracle_previous = Some(oracle_reconstruction);
    }
    Ok(())
}

fn model_high_bit(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 10 {
        return Err("yuv422p16le mode needs 10 arguments".into());
    }
    let sample = &arguments[0];
    let raw = fs::read(&arguments[1])?;
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let frame_rate = parse_rate(&arguments[4])?;
    let frame_count: usize = arguments[5].parse()?;
    let bit_depth: u8 = arguments[6].parse()?;
    let quality: u8 = arguments[7].parse()?;
    let threads: usize = arguments[8].parse()?;
    let gop: usize = arguments[9].parse()?;
    validate_counts(frame_count, gop)?;
    let y_len = area(width, height)?;
    let chroma_len = area(width.div_ceil(2), height)?;
    let samples_per_frame = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    let frame_len = samples_per_frame
        .checked_mul(size_of::<u16>())
        .ok_or("frame is too large")?;
    validate_sequence(raw.len(), frame_len, frame_count)?;
    let options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    print_header();
    let mut previous = None;
    let mut oracle_previous = None;
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
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
        let predicted = !frame_index.is_multiple_of(gop);
        let reference = predicted.then(|| {
            previous
                .as_ref()
                .expect("predicted frame has a reconstructed reference")
        });
        let oracle_reference = predicted.then(|| {
            oracle_previous
                .as_ref()
                .expect("predicted frame has an oracle reference")
        });
        let (models, oracle_reconstruction) =
            analyze_predictor_frame16(&frame, reference, oracle_reference, options)?;
        print_models(sample, frame_index, bit_depth, quality, gop, &models);
        let encoded = if let Some(reference) = reference {
            encode16_with_reference(&frame, reference, options)?
        } else {
            encode16(&frame, options)?
        };
        previous = Some(if let Some(reference) = reference {
            decode16_with_reference(&encoded, reference, threads)?
        } else {
            decode16(&encoded, threads)?
        });
        oracle_previous = Some(oracle_reconstruction);
    }
    Ok(())
}

fn print_header() {
    println!(
        "sample\tframe\tbit_depth\tquality\tgop\ttile\tplane\twidth\theight\tsamples\tcurrent_mode\toracle_mode\tcurrent_entropy\toracle_entropy\tcurrent_bytes\toracle_bytes\tcurrent_sse\toracle_sse\tcurrent_max_error\toracle_max_error\tpaeth_bytes\tpaeth_sse\taverage_bytes\taverage_sse\tclamp_bytes\tclamp_sse\thalf_bytes\thalf_sse\ttemporal_bytes\ttemporal_sse"
    );
}

fn print_models(
    sample: &str,
    frame: usize,
    bit_depth: u8,
    quality: u8,
    gop: usize,
    models: &[TilePredictorModel],
) {
    for (tile, model) in models.iter().enumerate() {
        let (temporal_bytes, temporal_sse) = model
            .temporal
            .map(|candidate| {
                (
                    candidate.payload_bytes.to_string(),
                    candidate.squared_error.to_string(),
                )
            })
            .unwrap_or_default();
        println!(
            "{sample}\t{frame}\t{bit_depth}\t{quality}\t{gop}\t{tile}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{temporal_bytes}\t{temporal_sse}",
            model.plane,
            model.width,
            model.height,
            model.sample_count,
            mode_name(model.current_mode),
            mode_name(model.oracle_mode),
            entropy_name(model.current.zero_run),
            entropy_name(model.oracle.zero_run),
            model.current.payload_bytes,
            model.oracle.payload_bytes,
            model.current.squared_error,
            model.oracle.squared_error,
            model.current.max_error,
            model.oracle.max_error,
            model.paeth.payload_bytes,
            model.paeth.squared_error,
            model.average.payload_bytes,
            model.average.squared_error,
            model.clamp_gradient.payload_bytes,
            model.clamp_gradient.squared_error,
            model.half_gradient.payload_bytes,
            model.half_gradient.squared_error,
        );
    }
}

fn mode_name(mode: PredictorModelMode) -> &'static str {
    match mode {
        PredictorModelMode::Paeth => "paeth",
        PredictorModelMode::Average => "average",
        PredictorModelMode::ClampGradient => "clamp-gradient",
        PredictorModelMode::HalfGradient => "half-gradient",
        PredictorModelMode::Temporal => "temporal",
    }
}

fn entropy_name(zero_run: bool) -> &'static str {
    if zero_run { "zero-run" } else { "rice" }
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

fn validate_counts(frame_count: usize, gop: usize) -> Result<(), Box<dyn std::error::Error>> {
    if frame_count == 0 || gop == 0 {
        return Err("frame count and GOP must be nonzero".into());
    }
    Ok(())
}

fn validate_sequence(
    bytes: usize,
    frame_len: usize,
    frame_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = frame_len
        .checked_mul(frame_count)
        .ok_or("sequence is too large")?;
    if bytes != expected {
        return Err(format!("input has {bytes} bytes; expected {expected}").into());
    }
    Ok(())
}
