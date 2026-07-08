// Copyright (c) 2025 Steven Rosenthal smr@dt3.org
// See LICENSE file in root directory for license terms.

// CLI wrapper around the CedarDetect star finding algorithm, used by the
// NeoSextant Android app (built as libcedar_cli.so) and by the desktop
// calibration optimizer (optimize_neosextant.py, detector_mode="cli").
// Reads one image, writes a JSON file: {"stars": [{"x":, "y":, "brightness":}]}
// with stars sorted by brightness descending. This recreates the previously
// unversioned source of libcedar_cli.so so app and optimizer share one
// detection code path.

use std::fs;
use std::time::Instant;

use clap::Parser;
use env_logger;
use image::ImageReader;
use log::info;

use cedar_detect::algorithm::{estimate_noise_from_image, get_stars_from_image};

/// CedarDetect star finder: image in, JSON star list out.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    /// Path of the image file to process.
    #[arg(short, long)]
    input: String,

    /// Path of the JSON output file to write.
    #[arg(short, long)]
    output: String,

    /// Statistical significance factor.
    #[arg(short, long, default_value_t = 8.0)]
    sigma: f64,

    /// Whether image rows should be normalized to have same dark levels.
    /// Relevant only when binning > 1.
    #[arg(short, long, default_value_t = false)]
    normalize_rows: std::primitive::bool,

    /// Whether image should be binned. 1 (no binning), 2, or 4.
    #[arg(short, long, default_value_t = 2)]
    binning: u32,

    /// Whether hot pixels should be detected.
    #[arg(long, default_value_t = true)]
    hot_pixels: std::primitive::bool,
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let img = ImageReader::open(&args.input)
        .unwrap_or_else(|e| panic!("Cannot open '{}': {:?}", args.input, e))
        .decode()
        .unwrap_or_else(|e| panic!("Cannot decode '{}': {:?}", args.input, e));
    let img_u8 = img.to_luma8();
    let (width, height) = img_u8.dimensions();

    let start = Instant::now();
    let noise_estimate = estimate_noise_from_image(&img_u8);
    let (mut stars, _, _, _) = get_stars_from_image(
        &img_u8, noise_estimate, args.sigma,
        args.normalize_rows, args.binning,
        args.hot_pixels, /*return_binned_image=*/false);
    info!("WxH: {}x{}; noise level {}; found {} stars in {:?}",
          width, height, noise_estimate, stars.len(), start.elapsed());

    stars.sort_by(|a, b| b.brightness.partial_cmp(&a.brightness).unwrap());
    let mut entries: Vec<String> = Vec::with_capacity(stars.len());
    for star in &stars {
        entries.push(format!(
            "{{\"x\": {}, \"y\": {}, \"brightness\": {}}}",
            star.centroid_x, star.centroid_y, star.brightness));
    }
    let json = format!("{{\"width\": {}, \"height\": {}, \"stars\": [{}]}}",
                       width, height, entries.join(", "));
    fs::write(&args.output, json)
        .unwrap_or_else(|e| panic!("Cannot write '{}': {:?}", args.output, e));
}
