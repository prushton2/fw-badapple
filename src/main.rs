use std::{fs::DirEntry, process::Command};
use std::sync::Arc;
use image::{ImageBuffer, ImageReader, Rgb};
use clap::Parser;
use serialport::{self, SerialPortType};

mod matrix;

use matrix::Matrix;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'f', default_value_t = "./badapple.mp4".to_owned())]
    file: String,

    #[arg(short = 'g', default_value_t = false)]
    get_frames: bool,

    #[arg(short = 's', default_value_t = false)]
    scale_frames: bool,

    #[arg(short = 'W', default_value_t = 200)]
    scaled_width: u32,

    #[arg(short = 'H', default_value_t = 100)]
    scaled_height: u32
}

fn main() {
    let args = Args::parse();

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
    
        let frames = std::fs::read_dir("./frames").expect("Cannot read frames");
        let mut threads = vec![];

        let arc_args = Arc::new(args);
    
        for frame_result in frames {
            let arc = arc_args.clone();
            threads.push(std::thread::spawn(|| {downscale_frame(frame_result, arc);}))
        }
    
        for thread in threads {
            let _ = thread.join();
        }
    }

    let mut frames: Vec<DirEntry> = std::fs::read_dir("scaled_frames").expect("msg").filter(|e| e.is_ok()).map(|e| e.unwrap()).collect();

    frames.sort_by(|a, b| {
        if a.path().to_owned() > b.path().to_owned() {
            return std::cmp::Ordering::Greater
        } else if a.path().to_owned() == b.path().to_owned() {
            return std::cmp::Ordering::Equal
        }
        return std::cmp::Ordering::Less;
    });

    let mut matrices = init();

    for file in frames {
        let frame: image::ImageBuffer<Rgb<u8>, Vec<u8>> = ImageReader::open(file.path())
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

        matrices[0].save_cols();
        matrices[1].save_cols();
        let _ = matrices[0].flush_buffer();
        let _ = matrices[1].flush_buffer();
        std::thread::sleep(std::time::Duration::from_millis((1000.0/30.0) as u64));
    }

}


fn init() -> Vec<Matrix> {
    let ports: Vec<serialport::SerialPortInfo> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            match &e.port_type {
                SerialPortType::UsbPort(port_info) => {
                    return port_info.manufacturer == Some("Framework Computer Inc".to_owned()) && port_info.product == Some("LED Matrix Input Module".to_owned())
                },
                _ => false
            }
        })
        .collect();

    for port in &ports {
        println!("{:?}", port);
    }

    let matrices: Vec<Matrix> = ports.iter().map(|e| Matrix::from_device_label(&e.port_name)).collect();
    return matrices;
}
fn downscale_frame(frame_result: Result<DirEntry, std::io::Error>, args: Arc<Args>) {
    let file = match frame_result {
        Ok(t) => t,
        Err(_) => return
    };

    let frame: image::ImageBuffer<Rgb<u8>, Vec<u8>> = ImageReader::open(file.path())
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();

    let mut scaled_buffer: image::ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(args.scaled_width, args.scaled_height);

    for (x, y, pixel) in frame.enumerate_pixels() {
        let destination = downscale_to(x, y, frame.dimensions(), (args.scaled_width, args.scaled_height));

        scaled_buffer.put_pixel(destination.0, destination.1, Rgb(
            [
                pixel.0[0],
                pixel.0[1],
                pixel.0[2],
            ]
        ));
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