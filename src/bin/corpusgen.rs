use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

struct Frame422 {
    y: Vec<u8>,
    cb: Vec<u8>,
    cr: Vec<u8>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("corpusgen: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let destination = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: corpusgen DESTINATION")?;
    for directory in ["stills", "videos", "native"] {
        fs::create_dir_all(destination.join(directory))?;
    }

    write_sequence(
        &destination.join("videos/ui-dashboard-scroll-1280x720-24f.yuv"),
        1280,
        720,
        24,
        ui_frame,
    )?;
    write_sequence(
        &destination.join("videos/procedural-scene-cuts-1920x1080-24f.yuv"),
        1920,
        1080,
        24,
        scene_cut_frame,
    )?;
    write_sequence(
        &destination.join("stills/procedural-chroma-edges-1920x1080.yuv"),
        1920,
        1080,
        1,
        chroma_edges_frame,
    )?;
    write_sequence(
        &destination.join("stills/resolution-grid-640x360.yuv"),
        640,
        360,
        1,
        resolution_grid_frame,
    )?;
    write_sequence(
        &destination.join("stills/resolution-grid-3840x2160.yuv"),
        3840,
        2160,
        1,
        resolution_grid_frame,
    )?;
    write_hdr(&destination.join("native/hdr-pq-gradient-3840x2160-yuv444p10le.raw"))?;
    write_alpha(&destination.join("native/alpha-overlays-1024x1024-rgba.raw"))?;
    Ok(())
}

fn write_sequence<F>(
    path: &Path,
    width: usize,
    height: usize,
    frames: usize,
    generator: F,
) -> std::io::Result<()>
where
    F: Fn(usize, usize, usize) -> Frame422,
{
    let mut output = BufWriter::new(File::create(path)?);
    for frame_index in 0..frames {
        let frame = generator(width, height, frame_index);
        output.write_all(&frame.y)?;
        output.write_all(&frame.cb)?;
        output.write_all(&frame.cr)?;
    }
    output.flush()
}

fn frame_422<F>(width: usize, height: usize, mut sample: F) -> Frame422
where
    F: FnMut(usize, usize) -> (u8, u8, u8),
{
    let chroma_width = width.div_ceil(2);
    let mut y_plane = Vec::with_capacity(width * height);
    let mut cb_plane = Vec::with_capacity(chroma_width * height);
    let mut cr_plane = Vec::with_capacity(chroma_width * height);
    for y in 0..height {
        for x in 0..width {
            y_plane.push(sample(x, y).0);
        }
        for x in 0..chroma_width {
            let left = sample(x * 2, y);
            let right = sample((x * 2 + 1).min(width - 1), y);
            cb_plane.push((u16::from(left.1) + u16::from(right.1)).div_ceil(2) as u8);
            cr_plane.push((u16::from(left.2) + u16::from(right.2)).div_ceil(2) as u8);
        }
    }
    Frame422 {
        y: y_plane,
        cb: cb_plane,
        cr: cr_plane,
    }
}

fn ui_frame(width: usize, height: usize, frame: usize) -> Frame422 {
    let phase = frame * 5;
    frame_422(width, height, |x, y| {
        let header = height / 12;
        let sidebar = width / 7;
        let mut luma = if y < header {
            46
        } else if x < sidebar {
            34
        } else {
            24
        };
        let mut cb = 128;
        let mut cr = 128;

        if x >= sidebar + 24 && y >= header + 24 {
            let panel_x = (x - sidebar - 24) / (width / 4);
            let panel_y = (y - header - 24) / (height / 3);
            if (x - sidebar - 24) % (width / 4) < width / 4 - 16
                && (y - header - 24) % (height / 3) < height / 3 - 16
            {
                luma = 42 + ((panel_x + panel_y) % 3) as u8 * 7;
            }
        }
        if y >= header && (y + phase).is_multiple_of(28) {
            luma = 92;
        }
        if x >= sidebar && (x + phase * 2).is_multiple_of(64) {
            luma = luma.saturating_add(24);
        }
        if y > height * 2 / 3 {
            let chart = (x * 17 + phase * 29) % width;
            let curve = height * 5 / 6 + ((chart * 13) % (height / 8));
            if y.abs_diff(curve) <= 2 {
                luma = 196;
                cb = 90;
                cr = 166;
            }
        }
        if x < sidebar && y > header {
            let row = (y + phase) % 72;
            if (18..=25).contains(&row) && x > 24 && x < sidebar - 20 {
                luma = 150;
                cb = 154;
                cr = 106;
            }
        }
        (luma, cb, cr)
    })
}

fn scene_cut_frame(width: usize, height: usize, frame: usize) -> Frame422 {
    frame_422(width, height, |x, y| match frame / 8 {
        0 => {
            let gradient = 24 + (x * 150 / width) as u8;
            let center_x = width / 4 + frame * width / 32;
            let center_y = height / 2;
            let inside = x.abs_diff(center_x) * x.abs_diff(center_x)
                + y.abs_diff(center_y) * y.abs_diff(center_y)
                < (height / 7) * (height / 7);
            if inside {
                (206, 92, 170)
            } else {
                (gradient, 146, 108)
            }
        }
        1 => {
            let noise = hash32(x as u32, y as u32, frame as u32);
            let base = 72 + (y * 92 / height) as i32;
            let grain = ((noise & 63) as i32) - 31;
            (
                clamp_limited(base + grain),
                clamp_chroma(128 + (((noise >> 8) & 31) as i32) - 15),
                clamp_chroma(128 + (((noise >> 16) & 31) as i32) - 15),
            )
        }
        _ => {
            let block = ((x + frame * 17) / 32 + (y + frame * 9) / 32) & 1;
            if block == 0 {
                (42, 176, 92)
            } else {
                (210, 84, 184)
            }
        }
    })
}

fn chroma_edges_frame(width: usize, height: usize, _frame: usize) -> Frame422 {
    frame_422(width, height, |x, y| {
        let luma = 112 + ((x * 24 / width + y * 16 / height) as u8);
        let zone = (x / 48 + y / 72) % 4;
        let (cb, cr) = match zone {
            0 => (48, 128),
            1 => (208, 128),
            2 => (128, 48),
            _ => (128, 208),
        };
        (luma, cb, cr)
    })
}

fn resolution_grid_frame(width: usize, height: usize, _frame: usize) -> Frame422 {
    frame_422(width, height, |x, y| {
        let fine_grid = x.is_multiple_of(8) || y.is_multiple_of(8);
        let coarse_grid = x.is_multiple_of(64) || y.is_multiple_of(64);
        let diagonal = (x + y).is_multiple_of(37) || (x + height - y - 1).is_multiple_of(41);
        let luma = if coarse_grid {
            220
        } else if fine_grid {
            154
        } else if diagonal {
            88
        } else {
            28 + (x * 100 / width) as u8
        };
        (luma, 128, 128)
    })
}

fn write_hdr(path: &Path) -> std::io::Result<()> {
    const WIDTH: usize = 3840;
    const HEIGHT: usize = 2160;
    let mut output = BufWriter::new(File::create(path)?);
    for plane in 0..3 {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let value = match plane {
                    0 => 64 + (x * 876 / (WIDTH - 1)) as isize + ((y / 90) % 2) as isize * 8,
                    1 => 512 + (y as isize - (HEIGHT / 2) as isize) * 384 / HEIGHT as isize,
                    _ => 512 + (x as isize - (WIDTH / 2) as isize) * 384 / WIDTH as isize,
                }
                .clamp(64, 960) as u16;
                output.write_all(&value.to_le_bytes())?;
            }
        }
    }
    output.flush()
}

fn write_alpha(path: &Path) -> std::io::Result<()> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 1024;
    let mut output = BufWriter::new(File::create(path)?);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let dx = x.abs_diff(WIDTH / 2);
            let dy = y.abs_diff(HEIGHT / 2);
            let radial = dx * dx + dy * dy;
            let alpha = if radial < 180 * 180 {
                255
            } else if radial < 430 * 430 {
                255 - ((radial - 180 * 180) * 255 / (430 * 430 - 180 * 180)) as u8
            } else if (x / 32 + y / 32).is_multiple_of(2) {
                48
            } else {
                0
            };
            let rgba = [
                (32 + x * 191 / WIDTH) as u8,
                (32 + y * 191 / HEIGHT) as u8,
                (220usize.saturating_sub((x + y) * 96 / (WIDTH + HEIGHT))) as u8,
                alpha,
            ];
            output.write_all(&rgba)?;
        }
    }
    output.flush()
}

fn hash32(x: u32, y: u32, frame: u32) -> u32 {
    let mut value = x
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(y.wrapping_mul(0x85eb_ca6b))
        .wrapping_add(frame.wrapping_mul(0xc2b2_ae35))
        .wrapping_add(0x27d4_eb2d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x85eb_ca6b);
    value ^= value >> 13;
    value.wrapping_mul(0xc2b2_ae35) ^ (value >> 16)
}

fn clamp_limited(value: i32) -> u8 {
    value.clamp(16, 235) as u8
}

fn clamp_chroma(value: i32) -> u8 {
    value.clamp(16, 240) as u8
}
