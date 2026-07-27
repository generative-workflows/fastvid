use fastvid::{
    CodecOptions, Frame16, FrameRate, decode16, decode16_with_reference, encode16,
    encode16_with_reference,
};
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("v2_temporal_sweep: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 12 {
        return Err(
            "usage: v2_temporal_sweep SAMPLE INPUT DECODED WIDTH HEIGHT FPS BIT_DEPTH QUALITY THREADS TILE_WIDTH TILE_HEIGHT GOP"
                .into(),
        );
    }
    let sample = &arguments[0];
    let input = &arguments[1];
    let decoded_path = &arguments[2];
    let width: u32 = arguments[3].parse()?;
    let height: u32 = arguments[4].parse()?;
    let frame_rate = parse_rate(&arguments[5])?;
    let bit_depth: u8 = arguments[6].parse()?;
    let options = CodecOptions {
        quality: arguments[7].parse()?,
        threads: arguments[8].parse()?,
        tile_width: arguments[9].parse()?,
        tile_height: arguments[10].parse()?,
    };
    let gop: usize = arguments[11].parse()?;
    if gop == 0 {
        return Err("GOP must be nonzero".into());
    }

    let y_len = area(width, height)?;
    let chroma_len = area(width.div_ceil(2), height)?;
    let sample_count = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    let frame_bytes = sample_count.checked_mul(2).ok_or("frame is too large")?;
    let input_size = File::open(input)?.metadata()?.len() as usize;
    if input_size == 0 || !input_size.is_multiple_of(frame_bytes) {
        return Err("input is not a whole number of planar YUV422 u16 frames".into());
    }
    let frame_count = input_size / frame_bytes;
    let mut input = BufReader::new(File::open(input)?);
    let mut output = BufWriter::new(File::create(decoded_path)?);
    let mut raw = vec![0u8; frame_bytes];
    let mut previous: Option<Frame16> = None;

    println!("sample\tframe\tframes\tquality\tgop\tkeyframe\traw_bytes\tencoded_bytes\texact");
    for frame_index in 0..frame_count {
        input.read_exact(&mut raw)?;
        let values = raw
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        let source = Frame16::yuv422(
            width,
            height,
            bit_depth,
            frame_rate,
            values[..y_len].to_vec(),
            values[y_len..y_len + chroma_len].to_vec(),
            values[y_len + chroma_len..].to_vec(),
        )?;
        let keyframe = frame_index.is_multiple_of(gop);
        let encoded = if keyframe {
            encode16(&source, options)?
        } else {
            encode16_with_reference(
                &source,
                previous
                    .as_ref()
                    .expect("non-key frame has a reconstructed predecessor"),
                options,
            )?
        };
        let decoded = if keyframe {
            decode16(&encoded, options.threads)?
        } else {
            decode16_with_reference(
                &encoded,
                previous
                    .as_ref()
                    .expect("non-key frame has a reconstructed predecessor"),
                options.threads,
            )?
        };
        for plane in &decoded.planes {
            for &sample in &plane.data {
                output.write_all(&sample.to_le_bytes())?;
            }
        }
        println!(
            "{sample}\t{frame_index}\t{frame_count}\t{}\t{gop}\t{keyframe}\t{frame_bytes}\t{}\t{}",
            options.quality,
            encoded.len(),
            source == decoded,
        );
        previous = Some(decoded);
    }
    output.flush()?;
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
