use fastvid::{
    CodecOptions, Frame16, FrameRate, analyze_parallel_shards16, encode16_parallel_full_tile,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("v5_entropy_model: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if !(10..=11).contains(&arguments.len()) {
        return Err("usage: v5_entropy_model SAMPLE INPUT WIDTH HEIGHT FPS BIT_DEPTH QUALITY THREADS TILE_WIDTH TILE_HEIGHT [FRAMES]".into());
    }
    let sample = &arguments[0];
    let raw = fs::read(&arguments[1])?;
    let width: u32 = arguments[2].parse()?;
    let height: u32 = arguments[3].parse()?;
    let frame_rate = parse_rate(&arguments[4])?;
    let bit_depth: u8 = arguments[5].parse()?;
    let quality: u8 = arguments[6].parse()?;
    let threads: usize = arguments[7].parse()?;
    let tile_width: u16 = arguments[8].parse()?;
    let tile_height: u16 = arguments[9].parse()?;
    let frame_count: usize = arguments
        .get(10)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(1);
    if frame_count == 0 {
        return Err("frame count must be nonzero".into());
    }
    let y_len = area(width, height)?;
    let chroma_len = area(width.div_ceil(2), height)?;
    let samples = y_len
        .checked_add(2 * chroma_len)
        .ok_or("frame is too large")?;
    let frame_bytes = samples.checked_mul(2).ok_or("frame is too large")?;
    if raw.len()
        != frame_bytes
            .checked_mul(frame_count)
            .ok_or("input is too large")?
    {
        return Err("input length does not match the declared planar YUV422 u16 frames".into());
    }
    println!(
        "sample\tframe\twidth\theight\tbit_depth\tquality\traw_bytes\tencoded_bytes\tstream_overhead_bytes\tshards\tzero_run_shards\trice_shards\tblock_pack_shards\torder0_supported_shards\torder0_winning_shards\tcurrent_shard_bytes\toracle_shard_bytes\toracle_stream_bytes\toracle_saving_percent"
    );
    for (frame_index, frame_raw) in raw.chunks_exact(frame_bytes).enumerate() {
        let values = frame_raw
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        let frame = Frame16::yuv422(
            width,
            height,
            bit_depth,
            frame_rate,
            values[..y_len].to_vec(),
            values[y_len..y_len + chroma_len].to_vec(),
            values[y_len + chroma_len..].to_vec(),
        )?;
        let encoded = encode16_parallel_full_tile(
            &frame,
            CodecOptions {
                quality,
                threads,
                tile_width,
                tile_height,
            },
        )?;
        let models = analyze_parallel_shards16(&encoded)?;
        let current_shard_bytes = models
            .iter()
            .map(|model| model.current_complete_bytes as u64)
            .sum::<u64>();
        let oracle_shard_bytes = models
            .iter()
            .map(|model| model.oracle_complete_bytes)
            .sum::<u64>();
        let stream_overhead_bytes = encoded.len() as u64 - current_shard_bytes;
        let oracle_stream_bytes = stream_overhead_bytes + oracle_shard_bytes;
        let supported = models.iter().filter(|model| model.order0_supported).count();
        let winners = models
            .iter()
            .filter(|model| {
                model.order0_supported
                    && model.order0_complete_bytes < model.current_complete_bytes as u64
            })
            .count();
        let zero_run = models
            .iter()
            .filter(|model| model.current_mode == 0)
            .count();
        let block_pack = models
            .iter()
            .filter(|model| model.current_mode == 18)
            .count();
        let rice = models.len() - zero_run - block_pack;
        println!(
            "{sample}\t{frame_index}\t{width}\t{height}\t{bit_depth}\t{quality}\t{}\t{}\t{stream_overhead_bytes}\t{}\t{zero_run}\t{rice}\t{block_pack}\t{supported}\t{winners}\t{current_shard_bytes}\t{oracle_shard_bytes}\t{oracle_stream_bytes}\t{:.6}",
            frame_raw.len(),
            encoded.len(),
            models.len(),
            100.0 * (1.0 - oracle_stream_bytes as f64 / encoded.len() as f64),
        );
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
