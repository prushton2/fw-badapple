use std::{fs::DirEntry, process::Command};
use std::sync::Arc;
use image::{ImageBuffer, ImageReader, Rgb};
use clap::Parser;

use fwinputmodule::led_matrix;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File to source frames from
    #[arg(short = 'f', default_value_t = "./badapple.mp4".to_owned())]
    file: String,

    /// Get the frames in the video
    #[arg(short = 'g', default_value_t = false)]
    get_frames: bool,

    /// Scale the frames to the provided resolution
    #[arg(short = 's', default_value_t = false)]
    scale_frames: bool,

    /// Framerate of the source video
    #[arg(short = 'r', default_value_t = 30)]
    framerate: u32,

    /// Use multithreading for video scaling
    #[arg(long, default_value_t = false)]
    multithread: bool,

    /// Width of the scaled resolution
    #[arg(short = 'W', default_value_t = 34)]
    scaled_width: u32,

    /// Height of the scaled resolution
    #[arg(short = 'H', default_value_t = 9)]
    scaled_height: u32
}

fn main() {
    let args = Arc::new(Args::parse());

    if args.get_frames {
        let _ = std::fs::remove_dir_all("frames");
        let _ = std::fs::create_dir("frames");
        
        let _ = Command::new("ffmpeg")
            .args(["-i", &args.file, "./frames/frame%04d.jpg"])
            .output();
    }

    if args.scale_frames {
        let _ = std::fs::remove_dir_all("scaled_frames");
        let _ = std::fs::create_dir("scaled_frames");
    
        let frames: Vec<DirEntry> = std::fs::read_dir("./frames").expect("Cannot read frames").filter(|e| e.is_ok()).map(|e| e.unwrap()).collect();
        
        if args.multithread {
            let mut threads = vec![];
            
            for frame in frames {
                let arc = args.clone();
                threads.push(std::thread::spawn(|| {downscale_frame(frame, arc);}))
            }
            
            for thread in threads {
                let _ = thread.join();
            }
        } else {
            for frame in frames {
                downscale_frame(frame, args.clone());
            }
        }

    }

    // we need to drop the framerate to about 5 fps since thats the limit on the module
    // this also means we need to only display every nth frame where n is the framerate ratio
    // let framerate_ratio = args.framerate / 5;

    let mut frames: Vec<DirEntry> = std::fs::read_dir("scaled_frames").expect("msg").filter(|e| e.is_ok()).map(|e| e.unwrap()).collect();

    frames.sort_by(|a, b| {
        if a.path().to_owned() > b.path().to_owned() {
            return std::cmp::Ordering::Greater
        } else if a.path().to_owned() == b.path().to_owned() {
            return std::cmp::Ordering::Equal
        }
        return std::cmp::Ordering::Less;
    });

    let mut matrices = led_matrix::discover::<led_matrix::SimpleMatrix>();

    for i in 0..frames.len() {

        let frame: image::ImageBuffer<Rgb<u8>, Vec<u8>> = ImageReader::open(frames[i].path())
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
            .to_rgb8();

        for (x, y, pixel) in frame.enumerate_pixels() {
            if y < 9 {
                matrices[0].write_buffer(y as usize, (33-x) as usize, pixel.0[0]);
            } else {
                matrices[1].write_buffer((y - 9) as usize, (33-x) as usize, pixel.0[0]);
            }
        }

        let _ = matrices[0].draw_bw();
        let _ = matrices[1].draw_bw();
        std::thread::sleep(std::time::Duration::from_millis((1000.0/args.framerate as f32) as u64));
    }

}

fn downscale_frame(file: DirEntry, args: Arc<Args>) {
    let frame: image::ImageBuffer<Rgb<u8>, Vec<u8>> = ImageReader::open(file.path())
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();

    let mut scaled_buffer: image::ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(args.scaled_width, args.scaled_height);
    //  buffer for averaging pixel values together (count, R, G, B)
    let mut scaled_intermediate_buffer: Vec<Vec<(u32, u32, u32, u32)>> = vec![vec![(0,0,0,0); args.scaled_height as usize]; args.scaled_width as usize];

    for (x, y, pixel) in frame.enumerate_pixels() {
        let destination = downscale_to(x, y, frame.dimensions(), (args.scaled_width, args.scaled_height));

        let value = &mut scaled_intermediate_buffer[destination.0 as usize][destination.1 as usize];

        value.0 += 1;
        value.1 += pixel.0[0] as u32;
        value.2 += pixel.0[1] as u32;
        value.3 += pixel.0[2] as u32;
    }

    for x in 0..scaled_intermediate_buffer.len() {
        for y in 0..scaled_intermediate_buffer[x].len() {
            scaled_buffer.put_pixel(x as u32, y as u32, 
                Rgb([
                    (scaled_intermediate_buffer[x][y].1 / scaled_intermediate_buffer[x][y].0) as u8,
                    (scaled_intermediate_buffer[x][y].2 / scaled_intermediate_buffer[x][y].0) as u8,
                    (scaled_intermediate_buffer[x][y].3 / scaled_intermediate_buffer[x][y].0) as u8
                ])
            );
        }
    }

    let new_path = file.path().to_str().unwrap().replace("frames", "scaled_frames");

    let _ = scaled_buffer.save(new_path);
}

fn downscale_to(x: u32, y: u32, source: (u32, u32), destination: (u32, u32)) -> (u32, u32) {
    let ratio = (
        destination.0 as f64 / source.0 as f64,
        destination.1 as f64 / source.1 as f64
    );
    (
        (ratio.0 * x as f64) as u32, 
        (ratio.1 * y as f64) as u32
    )
}