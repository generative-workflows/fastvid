use fastvid::{
    CodecOptions, Frame, Frame16, FrameRate, TileEntropyModel, analyze_entropy, analyze_entropy16,
    decode, decode_with_reference, decode16, decode16_with_reference, encode,
    encode_with_reference, encode16, encode16_with_reference,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("entropy_model: {error}");
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
            "usage: entropy_model yuv422 SAMPLE INPUT WIDTH HEIGHT FPS FRAMES QUALITY THREADS GOP\n\
             or: entropy_model yuv422p16le SAMPLE INPUT WIDTH HEIGHT FPS FRAMES BIT_DEPTH QUALITY THREADS GOP"
                .into(),
        ),
    }
}

fn model_8bit(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 9 {
        return Err("yuv422 mode needs 9 arguments".into());
    }
    let sample = &arguments[0];
    let input = &arguments[1];
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let frame_rate = parse_rate(&arguments[4])?;
    let frame_count: usize = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    let threads: usize = arguments[7].parse()?;
    let gop: usize = arguments[8].parse()?;
    validate_counts(frame_count, gop)?;
    let raw = fs::read(input)?;
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
        let encoded = if predicted {
            encode_with_reference(
                &frame,
                previous.as_ref().expect("predicted frame has a reference"),
                options,
            )?
        } else {
            encode(&frame, options)?
        };
        print_models(
            sample,
            frame_index,
            8,
            quality,
            gop,
            &analyze_entropy(&encoded)?,
        );
        previous = Some(if predicted {
            decode_with_reference(
                &encoded,
                previous.as_ref().expect("predicted frame has a reference"),
                threads,
            )?
        } else {
            decode(&encoded, threads)?
        });
    }
    Ok(())
}

fn model_high_bit(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 10 {
        return Err("yuv422p16le mode needs 10 arguments".into());
    }
    let sample = &arguments[0];
    let input = &arguments[1];
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let frame_rate = parse_rate(&arguments[4])?;
    let frame_count: usize = arguments[5].parse()?;
    let bit_depth: u8 = arguments[6].parse()?;
    let quality: u8 = arguments[7].parse()?;
    let threads: usize = arguments[8].parse()?;
    let gop: usize = arguments[9].parse()?;
    validate_counts(frame_count, gop)?;
    let raw = fs::read(input)?;
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
        let encoded = if predicted {
            encode16_with_reference(
                &frame,
                previous.as_ref().expect("predicted frame has a reference"),
                options,
            )?
        } else {
            encode16(&frame, options)?
        };
        print_models(
            sample,
            frame_index,
            bit_depth,
            quality,
            gop,
            &analyze_entropy16(&encoded)?,
        );
        previous = Some(if predicted {
            decode16_with_reference(
                &encoded,
                previous.as_ref().expect("predicted frame has a reference"),
                threads,
            )?
        } else {
            decode16(&encoded, threads)?
        });
    }
    Ok(())
}

fn print_header() {
    println!(
        "sample\tframe\tbit_depth\tquality\tgop\ttile\tplane\twidth\theight\tprediction\tsource_entropy\tsamples\tzeros\tactual_bytes\tstream_vbyte_bytes\tstream_vbyte_0124_bytes\tdistinct_symbols\tideal_order0_bytes\torder0_supported\torder0_table_log\torder0_payload_bytes\torder0_table_bytes\torder0_complete_bytes\tcontext_order0_supported\tcontext_order0_contexts\tcontext_order0_threshold\tcontext_order0_payload_bytes\tcontext_order0_table_bytes\tcontext_order0_control_bytes\tcontext_order0_complete_bytes"
    );
}

fn print_models(
    sample: &str,
    frame: usize,
    bit_depth: u8,
    quality: u8,
    gop: usize,
    models: &[TileEntropyModel],
) {
    for (tile, model) in models.iter().enumerate() {
        println!(
            "{sample}\t{frame}\t{bit_depth}\t{quality}\t{gop}\t{tile}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            model.plane,
            model.width,
            model.height,
            if model.temporal_prediction {
                "temporal"
            } else {
                "spatial"
            },
            if model.source_zero_run {
                "zero-run"
            } else {
                "rice"
            },
            model.sample_count,
            model.zero_symbols,
            model.actual_payload_bytes,
            model.stream_vbyte_bytes,
            model.stream_vbyte_0124_bytes,
            model.distinct_symbols,
            model.ideal_order0_bytes,
            model.order0_supported,
            model.order0_table_log,
            model.order0_payload_bytes,
            model.order0_table_bytes,
            model.order0_complete_bytes,
            model.context_order0_supported,
            model.context_order0_contexts,
            model.context_order0_threshold,
            model.context_order0_payload_bytes,
            model.context_order0_table_bytes,
            model.context_order0_control_bytes,
            model.context_order0_complete_bytes,
        );
    }
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
    if bytes
        != frame_len
            .checked_mul(frame_count)
            .ok_or("input is too large")?
    {
        return Err("input length does not match declared sequence".into());
    }
    Ok(())
}
