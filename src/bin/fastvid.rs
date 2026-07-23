use fastvid::{
    CodecOptions, Frame, FrameRate, PixelFormat, compare_plane, decode, encode, inspect,
};
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("fastvid: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();
    match arguments.get(1).map(String::as_str) {
        Some("demo") => demo(&arguments[2..]),
        Some("encode-yuv422") => encode_yuv422(&arguments[2..]),
        Some("decode") => decode_file(&arguments[2..]),
        Some("inspect") => inspect_file(&arguments[2..]),
        Some("-h" | "--help" | "help") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command `{command}`; use --help").into()),
    }
}

fn demo(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let width = parse_or(arguments.first(), 1920u32)?;
    let height = parse_or(arguments.get(1), 1080u32)?;
    let quality = parse_or(arguments.get(2), 90u8)?;
    let threads = parse_or(arguments.get(3), 1usize)?;
    let destination = arguments
        .get(4)
        .map(String::as_str)
        .unwrap_or("artifacts/demo.fvid");
    let frame = synthetic_frame(width, height)?;
    let options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };

    let encode_start = Instant::now();
    let encoded = encode(&frame, options)?;
    let encode_time = encode_start.elapsed();
    let decode_start = Instant::now();
    let decoded = decode(&encoded, threads)?;
    let decode_time = decode_start.elapsed();
    let metrics = compare_plane(&frame.planes[0].data, &decoded.planes[0].data)
        .ok_or("metric input mismatch")?;
    if let Some(parent) = std::path::Path::new(destination).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, &encoded)?;

    let raw_bytes = frame.raw_len();
    let megapixels = f64::from(width) * f64::from(height) / 1_000_000.0;
    println!("frame:          {width}x{height} YUV422p8");
    println!(
        "quality:        {quality} (step {})",
        1 + (100 - quality) / 5
    );
    println!("threads:        {threads}");
    println!("raw bytes:      {raw_bytes}");
    println!("encoded bytes:  {}", encoded.len());
    println!(
        "compression:    {:.3}x",
        raw_bytes as f64 / encoded.len() as f64
    );
    println!(
        "encode:         {:.3} ms ({:.1} MP/s)",
        encode_time.as_secs_f64() * 1000.0,
        megapixels / encode_time.as_secs_f64()
    );
    println!(
        "decode:         {:.3} ms ({:.1} MP/s)",
        decode_time.as_secs_f64() * 1000.0,
        megapixels / decode_time.as_secs_f64()
    );
    println!("luma max error: {}", metrics.max_error);
    println!("luma MSE:       {:.6}", metrics.mse);
    println!("luma PSNR:      {:.3} dB", metrics.psnr_db);
    println!("output:         {destination}");
    Ok(())
}

fn encode_yuv422(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 8 {
        return Err(
            "encode-yuv422 needs INPUT OUTPUT WIDTH HEIGHT FPS_NUM/FPS_DEN QUALITY THREADS TILE_SIZE"
                .into(),
        );
    }
    let input = &arguments[0];
    let output = &arguments[1];
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[4])?;
    let quality: u8 = arguments[5].parse()?;
    let threads: usize = arguments[6].parse()?;
    let tile_size: u16 = arguments[7].parse()?;
    let raw = fs::read(input)?;
    let y_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    let chroma_len = (width.div_ceil(2) as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    if raw.len() != y_len + 2 * chroma_len {
        return Err("input length does not match one planar YUV422p8 frame".into());
    }
    let frame = Frame::yuv422p8(
        width,
        height,
        FrameRate::new(fps_numerator, fps_denominator),
        raw[..y_len].to_vec(),
        raw[y_len..y_len + chroma_len].to_vec(),
        raw[y_len + chroma_len..].to_vec(),
    )?;
    let encoded = encode(
        &frame,
        CodecOptions {
            quality,
            tile_width: tile_size,
            tile_height: tile_size,
            threads,
        },
    )?;
    fs::write(output, encoded)?;
    Ok(())
}

fn decode_file(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 3 {
        return Err("decode needs INPUT OUTPUT THREADS".into());
    }
    let encoded = fs::read(&arguments[0])?;
    let frame = decode(&encoded, arguments[2].parse()?)?;
    let raw: Vec<u8> = frame
        .planes
        .iter()
        .flat_map(|plane| plane.data.iter().copied())
        .collect();
    fs::write(&arguments[1], raw)?;
    Ok(())
}

fn inspect_file(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 1 {
        return Err("inspect needs INPUT".into());
    }
    let info = inspect(&fs::read(&arguments[0])?)?;
    println!("{info:#?}");
    Ok(())
}

fn synthetic_frame(width: u32, height: u32) -> Result<Frame, Box<dyn std::error::Error>> {
    if width == 0 || height == 0 {
        return Err("dimensions must be nonzero".into());
    }
    let mut y = Vec::with_capacity(width as usize * height as usize);
    for row in 0..height {
        for column in 0..width {
            let gradient = (column * 160 / width + row * 64 / height) as u8;
            let bars = if (column / 96 + row / 64) % 2 == 0 {
                12
            } else {
                0
            };
            y.push(gradient.saturating_add(bars));
        }
    }
    let chroma_width = width.div_ceil(2);
    let mut cb = Vec::with_capacity(chroma_width as usize * height as usize);
    let mut cr = Vec::with_capacity(chroma_width as usize * height as usize);
    for row in 0..height {
        for column in 0..chroma_width {
            cb.push((96 + (column * 48 / chroma_width) + (row / 128) % 8) as u8);
            cr.push((176 - (column * 40 / chroma_width) - (row / 96) % 8) as u8);
        }
    }
    Ok(Frame::yuv422p8(
        width,
        height,
        FrameRate::new(24_000, 1_001),
        y,
        cb,
        cr,
    )?)
}

fn parse_or<T>(argument: Option<&String>, default: T) -> Result<T, T::Err>
where
    T: std::str::FromStr,
{
    argument.map_or(Ok(default), |value| value.parse())
}

fn parse_rate(value: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or("frame rate must be NUMERATOR/DENOMINATOR")?;
    Ok((numerator.parse()?, denominator.parse()?))
}

fn print_help() {
    println!(
        "Fastvid experimental codec

USAGE:
  fastvid demo [WIDTH HEIGHT QUALITY THREADS OUTPUT]
  fastvid encode-yuv422 INPUT OUTPUT WIDTH HEIGHT FPS_NUM/FPS_DEN QUALITY THREADS TILE_SIZE
  fastvid decode INPUT OUTPUT THREADS
  fastvid inspect INPUT

Raw YUV422 input/output is planar 8-bit Y, Cb, Cr. The bitstream is unstable."
    );
}

#[allow(dead_code)]
fn _pixel_format_name(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::Gray8 => "Gray8",
        PixelFormat::Yuv422p8 => "YUV422p8",
    }
}
