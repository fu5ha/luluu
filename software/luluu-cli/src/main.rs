use std::{path::PathBuf, fs::File, io::Write};

use clap::{Parser, Subcommand};

use eyre::WrapErr;
use luluu_enc::{Rgb565BE, Rgba8888, Rgb565NE, MagicBytes};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Convert {
        /// The file to convert
        #[arg(value_name = "FILEPATH")]
        file_path: PathBuf,

        /// Override the output's frame rate. Derived from the GIF if not provided.
        #[arg(short, long, value_name = "FRAMERATE")]
        frame_rate: Option<u8>,
    }
}

fn main() -> Result<(), eyre::Error> {
    pretty_env_logger::init_custom_env("debug,luluu_enc=warn");

    let cli = Cli::parse();
    match &cli.command {
        Commands::Convert { file_path, frame_rate } => {
            let file = File::open(file_path)
                .wrap_err_with(|| format!("Failed to read file to convert from {}", file_path.display()))?;

            let mut decode_opts = gif::DecodeOptions::new();
            decode_opts.set_color_output(gif::ColorOutput::RGBA);

            let mut decoder = decode_opts.read_info(file)
                .wrap_err_with(|| "Found the file, but failed to read it as a GIF.")?;

            let mut delay = None;
            let mut size = None;

            let mut data_buf: Vec<Rgb565BE> = Vec::new();

            let mut frame_number: u16 = 0;
            while let Some(frame) = decoder.read_next_frame()
                .wrap_err_with(|| format!("Failed to read frame {} of provided GIF", frame_number))?
            {
                let frame_size: u8  = match (frame.width, frame.height) {
                    (60, 60) => 60,
                    (120, 120) => 120,
                    (240, 240) => 240,
                    _ => eyre::bail!("Provided GIF is not the right size. It must be either 60x60, 120x120, or 240x240 pixels!"),
                };

                if *size.get_or_insert(frame_size) != frame_size {
                    eyre::bail!("Provided GIF has frames with different sizes. All frames must be 60x60, 120x120, or 240x240 pixels!");
                }

                match &mut delay {
                    delay @ None => *delay = Some(frame.delay),
                    Some(delay) => if *delay != frame.delay {
                        log::warn!("Detected uneven delay between frames. This is unsupported; computing frame rate based on first frame delay.")
                    }
                }

                let frame_data_len = frame_size as usize * frame_size as usize;
                let data_start_idx = data_buf.len();
                data_buf.resize(data_buf.len() + frame_data_len, Rgb565BE::ZERO);

                let src_pixels = Rgba8888::cast_bytes(&*frame.buffer);
                let dst_pixels = &mut data_buf[data_start_idx..(data_start_idx + frame_data_len)];

                for (src_pixel, dst_pixel) in src_pixels.iter().zip(dst_pixels) {
                    let [r, g, b] = src_pixel.rgb();
                    let col_dre_srgb = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
                    // let col_near = decode_srgb(col_dre_srgb);
                    let r_5 = ((col_dre_srgb.0 * (luluu_enc::MAX_5 as f32)) as u8).max(1u8);
                    let g_6 = ((col_dre_srgb.1 * (luluu_enc::MAX_6 as f32)) as u8).max(1u8);
                    let b_5 = ((col_dre_srgb.2 * (luluu_enc::MAX_5 as f32)) as u8).max(1u8);
                    let pixel_565 = Rgb565NE::pack_565(r_5, g_6, b_5);
                    *dst_pixel = pixel_565.to_be();
                }

                frame_number = frame_number
                    .checked_add(1)
                    .ok_or(eyre::eyre!("Too many frames in provided GIF!"))?;
            }

            if frame_number == 0 {
                eyre::bail!("Found a GIF but it had zero frames.");
            }

            let size = luluu_enc::Size(size.unwrap());

            let mut frame_rate = luluu_enc::FrameRate(frame_rate.unwrap_or_else(|| {
                let delay = delay.unwrap().max(1);
                (100 / delay) as u8
            }));

            frame_rate.make_nearest_supported(size).unwrap();

            // supported frame rates: 1, 2, 3, 4, 5, 6, 8, 10, 12, 15

            drop(decoder);

            let header = luluu_enc::Header {
                magic: MagicBytes::CORRECT,
                version: luluu_enc::Version::ZERO,
                encoding: luluu_enc::Encoding::RGB565BE,
                size,
                frame_rate,
                n_frames: luluu_enc::NumFrames::from_u16(frame_number),
            };

            let mut out_file_path = file_path.clone();
            out_file_path.set_extension("LU");

            let mut out_file = File::create(&out_file_path)
                .wrap_err_with(|| format!("Could not create output file: {}", out_file_path.display()))?;

            out_file.write_all(header.as_bytes())
                .wrap_err_with(|| "Failed to write output file.")?;

            out_file.write_all(Rgb565BE::slice_as_bytes(&data_buf))
                .wrap_err_with(|| "Failed to write output file.")?;
        }
    }

    Ok(())
}
