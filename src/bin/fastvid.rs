use fastvid::{
    CodecOptions, Frame, Frame16, FrameRate, PixelFormat, compare_plane, compare_plane16, decode,
    decode_tile16, decode_with_reference, decode16, decode16_with_reference, encode,
    encode_with_reference, encode16, encode16_parallel, encode16_parallel_full_tile,
    encode16_with_reference, inspect, inspect16, ssim_plane, ssim_plane16,
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
        Some("encode-yuv422p16le") => encode_yuv422p16le(&arguments[2..]),
        Some("encode-yuv422p16le-parallel") => encode_yuv422p16le_parallel(&arguments[2..]),
        Some("encode-yuv422p16le-parallel-full-tile") => {
            encode_yuv422p16le_parallel_full_tile(&arguments[2..])
        }
        Some("benchmark-yuv422") => benchmark_yuv422(&arguments[2..]),
        Some("benchmark-yuv422p16le") => benchmark_yuv422p16le(&arguments[2..]),
        Some("benchmark-yuv422p16le-parallel") => benchmark_yuv422p16le_parallel(&arguments[2..]),
        Some("benchmark-yuv422p16le-parallel-full-tile") => {
            benchmark_yuv422p16le_parallel_full_tile(&arguments[2..])
        }
        Some("benchmark-tile-access-yuv422p16le") => {
            benchmark_tile_access_yuv422p16le(&arguments[2..])
        }
        Some("metrics-yuv422p16le") => metrics_yuv422p16le(&arguments[2..]),
        Some("benchmark-access-yuv422") => benchmark_access_yuv422(&arguments[2..]),
        Some("benchmark-access-yuv422p16le") => benchmark_access_yuv422p16le(&arguments[2..]),
        Some("decode") => decode_file(&arguments[2..]),
        Some("decode16") => decode16_file(&arguments[2..]),
        Some("inspect") => inspect_file(&arguments[2..]),
        Some("-h" | "--help" | "help") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command `{command}`; use --help").into()),
    }
}

fn encode_yuv422p16le(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    encode_yuv422p16le_mode(arguments, HighBitFormat::Baseline)
}

fn encode_yuv422p16le_parallel(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    encode_yuv422p16le_mode(arguments, HighBitFormat::BoundedBands)
}

fn encode_yuv422p16le_parallel_full_tile(
    arguments: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    encode_yuv422p16le_mode(arguments, HighBitFormat::BoundedFullTile)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum HighBitFormat {
    Baseline,
    BoundedBands,
    BoundedFullTile,
}

fn encode_yuv422p16le_mode(
    arguments: &[String],
    format: HighBitFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(arguments.len(), 9 | 10) {
        return Err(
            "encode-yuv422p16le needs INPUT OUTPUT WIDTH HEIGHT FPS_NUM/FPS_DEN BIT_DEPTH QUALITY THREADS TILE_WIDTH [TILE_HEIGHT]"
                .into(),
        );
    }
    let width = arguments[2].parse()?;
    let height = arguments[3].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[4])?;
    let bit_depth = arguments[5].parse()?;
    let quality = arguments[6].parse()?;
    let threads = arguments[7].parse()?;
    let tile_width = arguments[8].parse()?;
    let tile_height = arguments
        .get(9)
        .map_or(Ok(tile_width), |value| value.parse())?;
    let frame = frame16_from_yuv422le(
        &fs::read(&arguments[0])?,
        width,
        height,
        bit_depth,
        FrameRate::new(fps_numerator, fps_denominator),
    )?;
    let options = CodecOptions {
        quality,
        tile_width,
        tile_height,
        threads,
    };
    let encoded = match format {
        HighBitFormat::Baseline => encode16(&frame, options)?,
        HighBitFormat::BoundedBands => encode16_parallel(&frame, options)?,
        HighBitFormat::BoundedFullTile => encode16_parallel_full_tile(&frame, options)?,
    };
    fs::write(&arguments[1], encoded)?;
    Ok(())
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
    if !matches!(arguments.len(), 7 | 8 | 10) {
        return Err(
            "benchmark-yuv422 needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS [GOP [TILE_WIDTH TILE_HEIGHT]]"
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
    let mut options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    if arguments.len() == 10 {
        options.tile_width = arguments[8].parse()?;
        options.tile_height = arguments[9].parse()?;
    }
    let mut encoded_bytes = 0usize;
    let mut zero_run_tiles = 0usize;
    let mut rice_tiles = 0usize;
    let mut spatial_tiles = 0usize;
    let mut temporal_tiles = 0usize;
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
        let info = inspect(&encoded)?;
        zero_run_tiles += info.zero_run_tiles;
        rice_tiles += info.rice_tiles;
        spatial_tiles += info.spatial_tiles;
        temporal_tiles += info.temporal_tiles;
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
        "input\tframes\tquality\tthreads\tgop\ttile_width\ttile_height\traw_bytes\tencoded_bytes\tratio\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error\tzero_run_tiles\trice_tiles\tspatial_tiles\ttemporal_tiles"
    );
    println!(
        "{}\t{frame_count}\t{quality}\t{threads}\t{gop}\t{}\t{}\t{raw_bytes}\t{encoded_bytes}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{encode_raw_mb_s:.3}\t{decode_raw_mb_s:.3}\t{encoded_stream_mb_s:.6}\t{encoded_stream_mbps:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.8}\t{max_error}\t{zero_run_tiles}\t{rice_tiles}\t{spatial_tiles}\t{temporal_tiles}",
        input,
        options.tile_width,
        options.tile_height,
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

fn benchmark_yuv422p16le(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    benchmark_yuv422p16le_mode(arguments, HighBitFormat::Baseline)
}

fn benchmark_yuv422p16le_parallel(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    benchmark_yuv422p16le_mode(arguments, HighBitFormat::BoundedBands)
}

fn benchmark_yuv422p16le_parallel_full_tile(
    arguments: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    benchmark_yuv422p16le_mode(arguments, HighBitFormat::BoundedFullTile)
}

fn benchmark_yuv422p16le_mode(
    arguments: &[String],
    format: HighBitFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(arguments.len(), 8 | 9 | 11) {
        return Err(
            "benchmark-yuv422p16le needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES BIT_DEPTH QUALITY THREADS [GOP [TILE_WIDTH TILE_HEIGHT]]"
                .into(),
        );
    }
    let input = &arguments[0];
    let width: u32 = arguments[1].parse()?;
    let height: u32 = arguments[2].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[3])?;
    let frame_count: usize = arguments[4].parse()?;
    let bit_depth: u8 = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    let threads: usize = arguments[7].parse()?;
    let gop: usize = arguments.get(8).map_or(Ok(1), |value| value.parse())?;
    if frame_count == 0 || gop == 0 {
        return Err("frame count and GOP must be nonzero".into());
    }
    if format != HighBitFormat::Baseline && gop != 1 {
        return Err("bounded-shard prototype is all-intra and requires GOP 1".into());
    }
    let raw = fs::read(input)?;
    let sample_count = yuv422_sample_count(width, height)?;
    let frame_len = sample_count
        .checked_mul(size_of::<u16>())
        .ok_or("frame is too large")?;
    if raw.len()
        != frame_len
            .checked_mul(frame_count)
            .ok_or("input is too large")?
    {
        return Err("input length does not match the declared YUV422p16le sequence".into());
    }
    let frame_rate = FrameRate::new(fps_numerator, fps_denominator);
    let mut options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    if arguments.len() == 11 {
        options.tile_width = arguments[9].parse()?;
        options.tile_height = arguments[10].parse()?;
    }
    let mut encoded_bytes = 0usize;
    let mut encode_time = std::time::Duration::ZERO;
    let mut decode_time = std::time::Duration::ZERO;
    let mut squared_errors = [0.0; 3];
    let mut plane_samples = [0usize; 3];
    let mut max_error = 0u16;
    let mut luma_ssim = 0.0;
    let mut previous = None;
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = frame16_from_yuv422le(bytes, width, height, bit_depth, frame_rate)?;
        let predicted = !frame_index.is_multiple_of(gop);
        let start = Instant::now();
        let encoded = match format {
            HighBitFormat::BoundedBands => encode16_parallel(&frame, options)?,
            HighBitFormat::BoundedFullTile => encode16_parallel_full_tile(&frame, options)?,
            HighBitFormat::Baseline if predicted => encode16_with_reference(
                &frame,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
                options,
            )?,
            HighBitFormat::Baseline => encode16(&frame, options)?,
        };
        encode_time += start.elapsed();
        encoded_bytes = encoded_bytes
            .checked_add(encoded.len())
            .ok_or("encoded sequence is too large")?;
        let start = Instant::now();
        let decoded = if predicted {
            decode16_with_reference(
                &encoded,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
                threads,
            )?
        } else {
            decode16(&encoded, threads)?
        };
        decode_time += start.elapsed();
        for (plane, (reference, actual)) in frame.planes.iter().zip(&decoded.planes).enumerate() {
            let metrics = compare_plane16(&reference.data, &actual.data, bit_depth)
                .ok_or("metric input mismatch")?;
            squared_errors[plane] += metrics.mse * metrics.samples as f64;
            plane_samples[plane] += metrics.samples;
            max_error = max_error.max(metrics.max_error);
        }
        luma_ssim += ssim_plane16(
            &frame.planes[0].data,
            &decoded.planes[0].data,
            width as usize,
            height as usize,
            bit_depth,
        )
        .ok_or("SSIM input mismatch")?;
        previous = Some(decoded);
    }
    let encode_seconds = encode_time.as_secs_f64();
    let decode_seconds = decode_time.as_secs_f64();
    let megapixels = f64::from(width) * f64::from(height) * frame_count as f64 / 1_000_000.0;
    let source_seconds = frame_count as f64 * f64::from(fps_denominator) / f64::from(fps_numerator);
    let peak = if bit_depth == 16 {
        f64::from(u16::MAX)
    } else {
        f64::from((1u16 << bit_depth) - 1)
    };
    let psnr = |plane: usize| {
        let mse = squared_errors[plane] / plane_samples[plane] as f64;
        if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (peak * peak / mse).log10()
        }
    };
    println!(
        "input\tframes\tbit_depth\tquality\tthreads\tgop\ttile_width\ttile_height\traw_bytes\tencoded_bytes\tratio\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error"
    );
    println!(
        "{input}\t{frame_count}\t{bit_depth}\t{quality}\t{threads}\t{gop}\t{}\t{}\t{}\t{encoded_bytes}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.8}\t{max_error}",
        options.tile_width,
        options.tile_height,
        raw.len(),
        raw.len() as f64 / encoded_bytes as f64,
        encode_seconds * 1000.0,
        decode_seconds * 1000.0,
        megapixels / encode_seconds,
        megapixels / decode_seconds,
        raw.len() as f64 / encode_seconds / 1_000_000.0,
        raw.len() as f64 / decode_seconds / 1_000_000.0,
        encoded_bytes as f64 / source_seconds / 1_000_000.0,
        encoded_bytes as f64 / source_seconds / 1_000_000.0 * 8.0,
        psnr(0),
        psnr(1),
        psnr(2),
        luma_ssim / frame_count as f64,
    );
    Ok(())
}

fn benchmark_tile_access_yuv422p16le(
    arguments: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 8 {
        return Err(
            "benchmark-tile-access-yuv422p16le needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN BIT_DEPTH QUALITY ITERATIONS VARIANT"
                .into(),
        );
    }
    let input = &arguments[0];
    let width: u32 = arguments[1].parse()?;
    let height: u32 = arguments[2].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[3])?;
    let bit_depth: u8 = arguments[4].parse()?;
    let quality: u8 = arguments[5].parse()?;
    let iterations: usize = arguments[6].parse()?;
    let variant = &arguments[7];
    if iterations == 0 {
        return Err("iterations must be nonzero".into());
    }
    let raw = fs::read(input)?;
    let frame_bytes = yuv422_sample_count(width, height)?
        .checked_mul(size_of::<u16>())
        .ok_or("frame is too large")?;
    let first_frame = raw.get(..frame_bytes).ok_or("truncated first frame")?;
    let frame = frame16_from_yuv422le(
        first_frame,
        width,
        height,
        bit_depth,
        FrameRate::new(fps_numerator, fps_denominator),
    )?;
    let options = CodecOptions {
        quality,
        threads: 1,
        ..CodecOptions::default()
    };
    let encoded = match variant.as_str() {
        "baseline" => encode16(&frame, options)?,
        "bounded-shard" => encode16_parallel(&frame, options)?,
        "bounded-full-tile" => encode16_parallel_full_tile(&frame, options)?,
        _ => {
            return Err("variant must be baseline, bounded-shard, or bounded-full-tile".into());
        }
    };
    let info = inspect16(&encoded)?;
    let full = decode16(&encoded, 1)?;
    let mut decoded_samples = 0usize;
    let start = Instant::now();
    for iteration in 0..iterations {
        for tile_index in 0..info.tile_count {
            let tile = decode_tile16(&encoded, tile_index)?;
            decoded_samples += tile.data.len();
            if iteration == 0 {
                let plane = &full.planes[tile.plane];
                for row in 0..tile.height as usize {
                    let expected_start =
                        (tile.y as usize + row) * plane.width as usize + tile.x as usize;
                    let actual_start = row * tile.width as usize;
                    assert_eq!(
                        &tile.data[actual_start..actual_start + tile.width as usize],
                        &plane.data[expected_start..expected_start + tile.width as usize]
                    );
                }
            }
        }
    }
    let elapsed = start.elapsed();
    let accesses = info.tile_count * iterations;
    println!(
        "input\tvariant\tbit_depth\tquality\titerations\ttiles\tencoded_bytes\tdecoded_samples\taccess_ms\tns_per_tile\ttile_sample_mpps"
    );
    println!(
        "{input}\t{variant}\t{bit_depth}\t{quality}\t{iterations}\t{}\t{}\t{decoded_samples}\t{:.3}\t{:.3}\t{:.3}",
        info.tile_count,
        encoded.len(),
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1e9 / accesses as f64,
        decoded_samples as f64 / 1_000_000.0 / elapsed.as_secs_f64(),
    );
    Ok(())
}

fn metrics_yuv422p16le(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 6 {
        return Err(
            "metrics-yuv422p16le needs REFERENCE DECODED WIDTH HEIGHT FRAMES BIT_DEPTH".into(),
        );
    }
    let reference = fs::read(&arguments[0])?;
    let decoded = fs::read(&arguments[1])?;
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let frame_count: usize = arguments[4].parse()?;
    let bit_depth: u8 = arguments[5].parse()?;
    if frame_count == 0 {
        return Err("frame count must be nonzero".into());
    }
    let frame_len = yuv422_sample_count(width, height)?
        .checked_mul(size_of::<u16>())
        .ok_or("frame is too large")?;
    let sequence_len = frame_len
        .checked_mul(frame_count)
        .ok_or("sequence is too large")?;
    if reference.len() != sequence_len || decoded.len() != sequence_len {
        return Err("input length does not match the declared YUV422p16le sequence".into());
    }
    let frame_rate = FrameRate::new(24, 1);
    let mut squared_errors = [0.0; 3];
    let mut plane_samples = [0usize; 3];
    let mut max_error = 0u16;
    let mut luma_ssim = 0.0;
    for (reference_bytes, decoded_bytes) in reference
        .chunks_exact(frame_len)
        .zip(decoded.chunks_exact(frame_len))
    {
        let expected =
            frame16_from_yuv422le(reference_bytes, width, height, bit_depth, frame_rate)?;
        let actual = frame16_from_yuv422le(decoded_bytes, width, height, bit_depth, frame_rate)?;
        for (plane, (expected, actual)) in expected.planes.iter().zip(&actual.planes).enumerate() {
            let metrics = compare_plane16(&expected.data, &actual.data, bit_depth)
                .ok_or("metric input mismatch")?;
            squared_errors[plane] += metrics.mse * metrics.samples as f64;
            plane_samples[plane] += metrics.samples;
            max_error = max_error.max(metrics.max_error);
        }
        luma_ssim += ssim_plane16(
            &expected.planes[0].data,
            &actual.planes[0].data,
            width as usize,
            height as usize,
            bit_depth,
        )
        .ok_or("SSIM input mismatch")?;
    }
    let peak = if bit_depth == 16 {
        f64::from(u16::MAX)
    } else {
        f64::from((1u16 << bit_depth) - 1)
    };
    let psnr = |plane: usize| {
        let mse = squared_errors[plane] / plane_samples[plane] as f64;
        if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (peak * peak / mse).log10()
        }
    };
    println!("frames\tbit_depth\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error");
    println!(
        "{frame_count}\t{bit_depth}\t{:.6}\t{:.6}\t{:.6}\t{:.8}\t{max_error}",
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
    if !matches!(arguments.len(), 9 | 11) {
        return Err(
            "benchmark-access-yuv422 needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS GOP TARGETS [TILE_WIDTH TILE_HEIGHT]"
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
    let mut options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    if arguments.len() == 11 {
        options.tile_width = arguments[9].parse()?;
        options.tile_height = arguments[10].parse()?;
    }
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
        "input\ttarget_frame\tkeyframe_frame\tdependency_frames\tdecoded_frames\tquality\tthreads\tgop\ttile_width\ttile_height\tencoded_bytes_read\taccess_ms\tuseful_mpps\twork_mpps\tuseful_raw_mb_s\taccess_amplification"
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
            "{input}\t{target}\t{keyframe}\t{dependency_frames}\t{decoded_frames}\t{quality}\t{threads}\t{gop}\t{}\t{}\t{encoded_bytes_read}\t{:.3}\t{useful_mpps:.3}\t{work_mpps:.3}\t{useful_raw_mb_s:.3}\t{:.3}",
            options.tile_width,
            options.tile_height,
            access_seconds * 1000.0,
            decoded_frames as f64,
        );
    }
    Ok(())
}

fn benchmark_access_yuv422p16le(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Codec-only warm-cache random access for native high-bit input. Source
    // I/O and sequence encoding remain outside the timed region.
    if !matches!(arguments.len(), 10 | 12) {
        return Err(
            "benchmark-access-yuv422p16le needs INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES BIT_DEPTH QUALITY THREADS GOP TARGETS [TILE_WIDTH TILE_HEIGHT]"
                .into(),
        );
    }
    let input = &arguments[0];
    let width: u32 = arguments[1].parse()?;
    let height: u32 = arguments[2].parse()?;
    let (fps_numerator, fps_denominator) = parse_rate(&arguments[3])?;
    let frame_count: usize = arguments[4].parse()?;
    let bit_depth: u8 = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    let threads: usize = arguments[7].parse()?;
    let gop: usize = arguments[8].parse()?;
    let mut targets = Vec::new();
    for value in arguments[9].split(',') {
        let target: usize = value.parse()?;
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets.sort_unstable();
    if frame_count == 0 || gop == 0 {
        return Err("frame count and GOP must be nonzero".into());
    }
    if targets.is_empty() || targets.iter().any(|&target| target >= frame_count) {
        return Err("target frame is out of range".into());
    }
    let raw = fs::read(input)?;
    let frame_len = yuv422_sample_count(width, height)?
        .checked_mul(size_of::<u16>())
        .ok_or("frame is too large")?;
    if raw.len()
        != frame_len
            .checked_mul(frame_count)
            .ok_or("input is too large")?
    {
        return Err("input length does not match the declared YUV422p16le sequence".into());
    }

    let frame_rate = FrameRate::new(fps_numerator, fps_denominator);
    let mut options = CodecOptions {
        quality,
        threads,
        ..CodecOptions::default()
    };
    if arguments.len() == 12 {
        options.tile_width = arguments[10].parse()?;
        options.tile_height = arguments[11].parse()?;
    }
    let mut encoded_frames = Vec::with_capacity(frame_count);
    let mut previous = None;
    let mut expected_targets = vec![None; frame_count];
    for (frame_index, bytes) in raw.chunks_exact(frame_len).enumerate() {
        let frame = frame16_from_yuv422le(bytes, width, height, bit_depth, frame_rate)?;
        let predicted = frame_index % gop != 0;
        let encoded = if predicted {
            encode16_with_reference(
                &frame,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
                options,
            )?
        } else {
            encode16(&frame, options)?
        };
        let decoded = if predicted {
            decode16_with_reference(
                &encoded,
                previous
                    .as_ref()
                    .expect("non-key frame has a preceding decoded frame"),
                threads,
            )?
        } else {
            decode16(&encoded, threads)?
        };
        if targets.binary_search(&frame_index).is_ok() {
            expected_targets[frame_index] = Some(decoded.clone());
        }
        encoded_frames.push(encoded);
        previous = Some(decoded);
    }

    let target_megapixels = f64::from(width) * f64::from(height) / 1_000_000.0;
    println!(
        "input\ttarget_frame\tkeyframe_frame\tdependency_frames\tdecoded_frames\tbit_depth\tquality\tthreads\tgop\ttile_width\ttile_height\tencoded_bytes_read\taccess_ms\tuseful_mpps\twork_mpps\tuseful_raw_mb_s\taccess_amplification"
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
                decode16(encoded, threads)?
            } else {
                decode16_with_reference(
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
            "{input}\t{target}\t{keyframe}\t{dependency_frames}\t{decoded_frames}\t{bit_depth}\t{quality}\t{threads}\t{gop}\t{}\t{}\t{encoded_bytes_read}\t{:.3}\t{useful_mpps:.3}\t{work_mpps:.3}\t{useful_raw_mb_s:.3}\t{:.3}",
            options.tile_width,
            options.tile_height,
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

fn yuv422_sample_count(width: u32, height: u32) -> Result<usize, Box<dyn std::error::Error>> {
    let y_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    let chroma_len = (width.div_ceil(2) as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    y_len
        .checked_add(2 * chroma_len)
        .ok_or_else(|| "frame is too large".into())
}

fn frame16_from_yuv422le(
    raw: &[u8],
    width: u32,
    height: u32,
    bit_depth: u8,
    frame_rate: FrameRate,
) -> Result<Frame16, Box<dyn std::error::Error>> {
    let sample_count = yuv422_sample_count(width, height)?;
    let byte_count = sample_count
        .checked_mul(size_of::<u16>())
        .ok_or("frame is too large")?;
    if raw.len() != byte_count {
        return Err("input length does not match one planar YUV422p16le frame".into());
    }
    let samples: Vec<u16> = raw
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect();
    let y_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    let chroma_len = (width.div_ceil(2) as usize)
        .checked_mul(height as usize)
        .ok_or("frame is too large")?;
    Ok(Frame16::yuv422(
        width,
        height,
        bit_depth,
        frame_rate,
        samples[..y_len].to_vec(),
        samples[y_len..y_len + chroma_len].to_vec(),
        samples[y_len + chroma_len..].to_vec(),
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

fn decode16_file(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 3 {
        return Err("decode16 needs INPUT OUTPUT THREADS".into());
    }
    let frame = decode16(&fs::read(&arguments[0])?, arguments[2].parse()?)?;
    let raw: Vec<u8> = frame
        .planes
        .iter()
        .flat_map(|plane| plane.data.iter().flat_map(|sample| sample.to_le_bytes()))
        .collect();
    fs::write(&arguments[1], raw)?;
    Ok(())
}

fn inspect_file(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.len() != 1 {
        return Err("inspect needs INPUT".into());
    }
    let bytes = fs::read(&arguments[0])?;
    let info = if bytes.get(4) == Some(&1) {
        inspect16(&bytes)?
    } else {
        inspect(&bytes)?
    };
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
  fastvid encode-yuv422p16le INPUT OUTPUT WIDTH HEIGHT FPS_NUM/FPS_DEN BIT_DEPTH QUALITY THREADS TILE_WIDTH [TILE_HEIGHT]
  fastvid encode-yuv422p16le-parallel INPUT OUTPUT WIDTH HEIGHT FPS_NUM/FPS_DEN BIT_DEPTH QUALITY THREADS TILE_WIDTH [TILE_HEIGHT]
  fastvid encode-yuv422p16le-parallel-full-tile INPUT OUTPUT WIDTH HEIGHT FPS_NUM/FPS_DEN BIT_DEPTH QUALITY THREADS TILE_WIDTH [TILE_HEIGHT]
  fastvid benchmark-yuv422 INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS [GOP [TILE_WIDTH TILE_HEIGHT]]
  fastvid benchmark-yuv422p16le INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES BIT_DEPTH QUALITY THREADS [GOP [TILE_WIDTH TILE_HEIGHT]]
  fastvid benchmark-yuv422p16le-parallel INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES BIT_DEPTH QUALITY THREADS [GOP [TILE_WIDTH TILE_HEIGHT]]
  fastvid benchmark-yuv422p16le-parallel-full-tile INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES BIT_DEPTH QUALITY THREADS [GOP [TILE_WIDTH TILE_HEIGHT]]
  fastvid benchmark-tile-access-yuv422p16le INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN BIT_DEPTH QUALITY ITERATIONS VARIANT
  fastvid metrics-yuv422p16le REFERENCE DECODED WIDTH HEIGHT FRAMES BIT_DEPTH
  fastvid benchmark-access-yuv422 INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES QUALITY THREADS GOP TARGETS [TILE_WIDTH TILE_HEIGHT]
  fastvid benchmark-access-yuv422p16le INPUT WIDTH HEIGHT FPS_NUM/FPS_DEN FRAMES BIT_DEPTH QUALITY THREADS GOP TARGETS [TILE_WIDTH TILE_HEIGHT]
  fastvid decode INPUT OUTPUT THREADS
  fastvid decode16 INPUT OUTPUT THREADS
  fastvid inspect INPUT

Raw high-bit input/output is planar Y, Cb, Cr in tightly packed little-endian
u16 words; BIT_DEPTH is 10, 12, or 16. The bitstream is unstable."
    );
}

#[allow(dead_code)]
fn _pixel_format_name(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::Gray8 => "Gray8",
        PixelFormat::Yuv422p8 => "YUV422p8",
        PixelFormat::Gray10 => "Gray10",
        PixelFormat::Yuv422p10 => "YUV422p10",
        PixelFormat::Gray12 => "Gray12",
        PixelFormat::Yuv422p12 => "YUV422p12",
        PixelFormat::Gray16 => "Gray16",
        PixelFormat::Yuv422p16 => "YUV422p16",
    }
}
