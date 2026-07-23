use fastvid::{
    CodecOptions, Frame, FrameRate, PixelFormat, compare_plane, decode, decode_with_reference,
    encode, encode_with_reference, inspect, ssim_plane,
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
        Some("benchmark-yuv422") => benchmark_yuv422(&arguments[2..]),
        Some("benchmark-access-yuv422") => benchmark_access_yuv422(&arguments[2..]),
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
    for (name, (reference, actual)) in ["Y", "Cb", "Cr"]
        .into_iter()
        .zip(frame.planes.iter().zip(&decoded.planes))
    {
        let metrics =
            compare_plane(&reference.data, &actual.data).ok_or("metric input mismatch")?;
        println!(
            "{name} quality:      max error {}, MSE {:.6}, PSNR {:.3} dB",
            metrics.max_error, metrics.mse, metrics.psnr_db
        );
    }
    let luma = &frame.planes[0];
    let luma_ssim = ssim_plane(
        &luma.data,
        &decoded.planes[0].data,
        luma.width as usize,
        luma.height as usize,
    )
    .ok_or("SSIM input mismatch")?;
    println!("Y block SSIM:   {luma_ssim:.8}");
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
    let frame = frame_from_yuv422(
        &raw,
        width,
        height,
        FrameRate::new(fps_numerator, fps_denominator),
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

fn benchmark_yuv422(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Standard sequence measurements are defined in research/0006 and
    // EVALUATION_METHODOLOGY.md. Codec timing intentionally excludes metrics.
    if !(7..=8).contains(&arguments.len()) {
        return Err(
            "benchmark-yuv422 needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS [GOP]"
                .into()
        );
    }
    let input = &arguments[0];
    let width: u32 = arguments[1].parse()?;
    let height: u32 = arguments[2].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[3])?;
    let frame_count: usize = arguments[4].parse()?;
    let quality: u8 = arguments[5].parse()?;
    let threads: usize = arguments[6].parse()?;
    let gop: usize = arguments.get(7).map_or(Ok(1), |value| value.parse())?;
    if frame_count == 0 {
        return Err("frame count must be nonzero".into());
    }
    if gop == 0 {
        return Err("GOP must be nonzero".into());
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
    let mut encoded_bytes = 0usize;
    let mut encode_time = std::time::Duration::ZERO;
    let mut decode_time = std::time::Duration::ZERO;
    let mut squared_errors = [0.0; 3];
    let mut plane_samples = [0usize; 3];
    let mut max_error = 0u8;
    let mut luma_ssim = 0.0;
    let mut previous = None;
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = frame_from_yuv422(bytes, width, height, frame_rate)?;
        let predicted = frame_index % gop != 0;
        let start = Instant::now();
        let encoded = if predicted {
            encode_with_reference(
                &frame,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
                options,
            )?
        } else {
            encode(&frame, options)?
        };
        encode_time += start.elapsed();
        encoded_bytes = encoded_bytes
            .checked_add(encoded.len())
            .ok_or("encoded sequence is too large")?;
        let start = Instant::now();
        let decoded = if predicted {
            decode_with_reference(
                &encoded,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
                threads,
            )?
        } else {
            decode(&encoded, threads)?
        };
        decode_time += start.elapsed();
        for (plane, (reference, actual)) in frame.planes.iter().zip(&decoded.planes).enumerate() {
            let metrics =
                compare_plane(&reference.data, &actual.data).ok_or("metric input mismatch")?;
            squared_errors[plane] += metrics.mse * metrics.samples as f64;
            plane_samples[plane] += metrics.samples;
            max_error = max_error.max(metrics.max_error);
        }
        luma_ssim += ssim_plane(
            &frame.planes[0].data,
            &decoded.planes[0].data,
            width as usize,
            height as usize,
        )
        .ok_or("SSIM input mismatch")?;
        previous = Some(decoded);
    }
    let raw_bytes = raw.len();
    let megapixels = f64::from(width) * f64::from(height) * frame_count as f64 / 1_000_000.0;
    let encode_seconds = encode_time.as_secs_f64();
    let decode_seconds = decode_time.as_secs_f64();
    let source_seconds = frame_count as f64 * f64::from(fps_denominator) / f64::from(fps_numerator);
    let encode_raw_mb_s = raw_bytes as f64 / encode_seconds / 1_000_000.0;
    let decode_raw_mb_s = raw_bytes as f64 / decode_seconds / 1_000_000.0;
    let encoded_stream_mb_s = encoded_bytes as f64 / source_seconds / 1_000_000.0;
    let encoded_stream_mbps = encoded_stream_mb_s * 8.0;
    let psnr = |plane: usize| {
        let mse = squared_errors[plane] / plane_samples[plane] as f64;
        if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * ((255.0 * 255.0) / mse).log10()
        }
    };
    println!(
        "input\tframes\tquality\tthreads\tgop\traw_bytes\tencoded_bytes\tratio\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error"
    );
    println!(
        "{}\t{frame_count}\t{quality}\t{threads}\t{gop}\t{raw_bytes}\t{encoded_bytes}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{encode_raw_mb_s:.3}\t{decode_raw_mb_s:.3}\t{encoded_stream_mb_s:.6}\t{encoded_stream_mbps:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.8}\t{max_error}",
        input,
        raw_bytes as f64 / encoded_bytes as f64,
        encode_seconds * 1000.0,
        decode_seconds * 1000.0,
        megapixels / encode_seconds,
        megapixels / decode_seconds,
        psnr(0),
        psnr(1),
        psnr(2),
        luma_ssim / frame_count as f64,
    );
    Ok(())
}

fn benchmark_access_yuv422(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Codec-only warm-cache random access; source/container I/O and sequence
    // encoding are intentionally outside the timed region (research/0010).
    if arguments.len() != 9 {
        return Err(
            "benchmark-access-yuv422 needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS GOP TARGETS"
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
    let mut targets = Vec::new();
    for value in arguments[8].split(',') {
        let target: usize = value.parse()?;
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets.sort_unstable();
    if frame_count == 0 {
        return Err("frame count must be nonzero".into());
    }
    if gop == 0 {
        return Err("GOP must be nonzero".into());
    }
    if targets.is_empty() || targets.iter().any(|&target| target >= frame_count) {
        return Err("target frame is out of range".into());
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
    let mut encoded_frames = Vec::with_capacity(frame_count);
    let mut previous = None;
    let mut expected_targets = vec![None; frame_count];
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = frame_from_yuv422(bytes, width, height, frame_rate)?;
        let predicted = frame_index % gop != 0;
        let encoded = if predicted {
            encode_with_reference(
                &frame,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
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
                    .expect("non-key frame has a preceding decoded frame"),
                threads,
            )?
        } else {
            decode(&encoded, threads)?
        };
        if targets.binary_search(&frame_index).is_ok() {
            expected_targets[frame_index] = Some(decoded.clone());
        }
        encoded_frames.push(encoded);
        previous = Some(decoded);
    }

    let target_megapixels = f64::from(width) * f64::from(height) / 1_000_000.0;
    println!(
        "input\ttarget_frame\tkeyframe_frame\tdependency_frames\tdecoded_frames\tquality\tthreads\tgop\tencoded_bytes_read\taccess_ms\tuseful_mpps\twork_mpps\tuseful_raw_mb_s\taccess_amplification"
    );
    for target in targets {
        let keyframe = target / gop * gop;
        let encoded_bytes_read =
            encoded_frames[keyframe..=target]
                .iter()
                .try_fold(0usize, |sum, encoded| {
                    sum.checked_add(encoded.len())
                        .ok_or("access byte count is too large")
                })?;
        let start = Instant::now();
        let mut reference = None;
        for (frame_index, encoded) in encoded_frames[keyframe..=target].iter().enumerate() {
            let absolute_index = keyframe + frame_index;
            let decoded = if absolute_index.is_multiple_of(gop) {
                decode(encoded, threads)?
            } else {
                decode_with_reference(
                    encoded,
                    reference
                        .as_ref()
                        .expect("predicted target has a preceding decoded frame"),
                    threads,
                )?
            };
            reference = Some(decoded);
        }
        let access_seconds = start.elapsed().as_secs_f64();
        if reference.as_ref() != expected_targets[target].as_ref() {
            return Err("random-access reconstruction differs from sequential decode".into());
        }

        let dependency_frames = target - keyframe;
        let decoded_frames = dependency_frames + 1;
        let useful_mpps = target_megapixels / access_seconds;
        let work_mpps = target_megapixels * decoded_frames as f64 / access_seconds;
        let useful_raw_mb_s = frame_len as f64 / access_seconds / 1_000_000.0;
        println!(
            "{input}\t{target}\t{keyframe}\t{dependency_frames}\t{decoded_frames}\t{quality}\t{threads}\t{gop}\t{encoded_bytes_read}\t{:.3}\t{useful_mpps:.3}\t{work_mpps:.3}\t{useful_raw_mb_s:.3}\t{:.3}",
            access_seconds * 1000.0,
            decoded_frames as f64,
        );
    }
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
  fastvid benchmark-yuv422 INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS [GOP]
  fastvid benchmark-access-yuv422 INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS GOP TARGETS
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
