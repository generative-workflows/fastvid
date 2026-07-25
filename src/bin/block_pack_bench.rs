use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

const SYMBOLS: usize = 128;
const WIDTHS: [u8; 13] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("block_pack_bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let iterations = env::args()
        .nth(1)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(20_000usize);
    if iterations == 0 {
        return Err("iterations must be nonzero".into());
    }
    verify()?;
    println!("kernel\tdirection\twidth\titerations\tns_per_block\tmsymbol_s");
    for width in WIDTHS {
        let values = values(width, SYMBOLS);
        let mut encoded = Vec::with_capacity(SYMBOLS * 2);
        baseline_pack(&values, width, &mut encoded);
        for (name, pack) in [
            ("baseline", baseline_pack as fn(&[u32], u8, &mut Vec<u8>)),
            ("word8", word8_pack),
        ] {
            let mut output = Vec::with_capacity(SYMBOLS * 2);
            for _ in 0..1000 {
                output.clear();
                pack(black_box(&values), width, &mut output);
                black_box(output.len());
            }
            let start = Instant::now();
            for _ in 0..iterations {
                output.clear();
                pack(black_box(&values), width, &mut output);
                black_box(output.len());
            }
            report(name, "pack", width, iterations, start.elapsed().as_nanos());
        }
        for (name, unpack) in [
            (
                "baseline",
                baseline_unpack as fn(&[u8], u8, usize, &mut Vec<u32>),
            ),
            ("word8", word8_unpack),
        ] {
            let mut output = Vec::with_capacity(SYMBOLS);
            for _ in 0..1000 {
                output.clear();
                unpack(black_box(&encoded), width, SYMBOLS, &mut output);
                black_box(output.len());
            }
            let start = Instant::now();
            for _ in 0..iterations {
                output.clear();
                unpack(black_box(&encoded), width, SYMBOLS, &mut output);
                black_box(output.len());
            }
            report(
                name,
                "unpack",
                width,
                iterations,
                start.elapsed().as_nanos(),
            );
        }
    }
    Ok(())
}

fn report(kernel: &str, direction: &str, width: u8, iterations: usize, elapsed_ns: u128) {
    let ns_per_block = elapsed_ns as f64 / iterations as f64;
    let msymbol_s = SYMBOLS as f64 * 1000.0 / ns_per_block;
    println!("{kernel}\t{direction}\t{width}\t{iterations}\t{ns_per_block:.3}\t{msymbol_s:.3}");
}

fn verify() -> Result<(), &'static str> {
    for width in 0..=17 {
        for length in 0..=SYMBOLS {
            let values = values(width, length);
            let mut baseline = Vec::new();
            let mut word8 = Vec::new();
            baseline_pack(&values, width, &mut baseline);
            word8_pack(&values, width, &mut word8);
            if baseline != word8 {
                return Err("packed bytes differ");
            }
            let mut decoded = Vec::new();
            word8_unpack(&word8, width, length, &mut decoded);
            if decoded != values {
                return Err("decoded values differ");
            }
        }
    }
    Ok(())
}

fn values(width: u8, length: usize) -> Vec<u32> {
    let mask = if width == 0 { 0 } else { (1u32 << width) - 1 };
    let mut state = 0x243f_6a88u32;
    (0..length)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) & mask
        })
        .collect()
}

fn baseline_pack(values: &[u32], width: u8, output: &mut Vec<u8>) {
    let mut buffer = 0u64;
    let mut buffered = 0u8;
    for &value in values {
        buffer |= u64::from(value) << buffered;
        buffered += width;
        while buffered >= 8 {
            output.push(buffer as u8);
            buffer >>= 8;
            buffered -= 8;
        }
    }
    if buffered != 0 {
        output.push(buffer as u8);
    }
}

fn word8_pack(values: &[u32], width: u8, output: &mut Vec<u8>) {
    if width == 0 {
        return;
    }
    if width <= 8 {
        let mut chunks = values.chunks_exact(8);
        for chunk in &mut chunks {
            let mut packed = 0u64;
            for (index, &value) in chunk.iter().enumerate() {
                packed |= u64::from(value) << (index * usize::from(width));
            }
            output.extend_from_slice(&packed.to_le_bytes()[..usize::from(width)]);
        }
        baseline_pack(chunks.remainder(), width, output);
        return;
    }
    if width <= 16 {
        let mut chunks = values.chunks_exact(8);
        for chunk in &mut chunks {
            let mut packed = 0u128;
            for (index, &value) in chunk.iter().enumerate() {
                packed |= u128::from(value) << (index * usize::from(width));
            }
            output.extend_from_slice(&packed.to_le_bytes()[..usize::from(width)]);
        }
        baseline_pack(chunks.remainder(), width, output);
        return;
    }
    baseline_pack(values, width, output);
}

fn baseline_unpack(input: &[u8], width: u8, count: usize, output: &mut Vec<u32>) {
    if width == 0 {
        output.resize(count, 0);
        return;
    }
    let mask = (1u64 << width) - 1;
    let mut cursor = 0usize;
    let mut buffer = 0u64;
    let mut buffered = 0u8;
    for _ in 0..count {
        while buffered < width {
            buffer |= u64::from(input[cursor]) << buffered;
            cursor += 1;
            buffered += 8;
        }
        output.push((buffer & mask) as u32);
        buffer >>= width;
        buffered -= width;
    }
}

fn word8_unpack(input: &[u8], width: u8, count: usize, output: &mut Vec<u32>) {
    if width == 0 {
        output.resize(count, 0);
        return;
    }
    if width <= 8 {
        let groups = count / 8;
        let bytes = usize::from(width);
        let mask = (1u64 << width) - 1;
        for group in 0..groups {
            let start = group * bytes;
            let mut packed_bytes = [0u8; 8];
            packed_bytes[..bytes].copy_from_slice(&input[start..start + bytes]);
            let mut packed = u64::from_le_bytes(packed_bytes);
            for _ in 0..8 {
                output.push((packed & mask) as u32);
                packed >>= width;
            }
        }
        baseline_unpack(&input[groups * bytes..], width, count % 8, output);
        return;
    }
    if width <= 16 {
        let groups = count / 8;
        let bytes = usize::from(width);
        let mask = (1u128 << width) - 1;
        for group in 0..groups {
            let start = group * bytes;
            let mut packed_bytes = [0u8; 16];
            packed_bytes[..bytes].copy_from_slice(&input[start..start + bytes]);
            let mut packed = u128::from_le_bytes(packed_bytes);
            for _ in 0..8 {
                output.push((packed & mask) as u32);
                packed >>= width;
            }
        }
        baseline_unpack(&input[groups * bytes..], width, count % 8, output);
        return;
    }
    baseline_unpack(input, width, count, output);
}
