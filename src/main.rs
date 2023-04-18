use anyhow::{Context, Result};
use image::{io::Reader as ImageReader, GenericImage, GenericImageView, Pixel, Rgb, Rgba};
use itertools::Itertools;
use minifb::{Key, Window, WindowOptions};
use rustfft::{num_complex::Complex, FftPlanner};

const WIDTH: usize = 1500;
const HEIGHT: usize = 800;

#[derive(Copy, Clone)]
enum FTypes {
    RGB(u8, u8, u8),
}

impl Into<u32> for FTypes {
    fn into(self) -> u32 {
        match self {
            FTypes::RGB(r, g, b) => ((r as u32) << 16) | ((g as u32) << 8) | b as u32,
        }
    }
}

impl Into<Rgba<u8>> for FTypes {
    fn into(self) -> Rgba<u8> {
        match self {
            FTypes::RGB(r, g, b) => Rgba([r, g, b, 255]),
        }
    }
}

use FTypes::*;

fn set_pixel<T: Into<u32>>(buffer: &mut Vec<u32>, x: usize, y: usize, pixel: T) {
    if x < WIDTH && y < HEIGHT {
        buffer[x + y * WIDTH] = pixel.into();
    }
}

fn add_pixel<T: Into<u32>>(buffer: &mut Vec<u32>, x: usize, y: usize, pixel: T) {
    if x < WIDTH && y < HEIGHT {
        buffer[x + y * WIDTH] |= pixel.into();
    }
}

fn get_pixel(buffer: &mut Vec<u32>, x: usize, y: usize) -> u32 {
    if x < WIDTH && y < HEIGHT {
        buffer[x + y * WIDTH]
    } else {
        0
    }
}

fn clear_buf(buffer: &mut Vec<u32>) {
    for p in buffer.iter_mut() {
        *p = 0x00;
    }
}

fn draw_img(buffer: &mut Vec<u32>, img: &image::DynamicImage, line: usize) {
    for (i, j, image::Rgba(p)) in img.pixels() {
        let i = i as usize;
        let j = j as usize;
        let [r, g, b, a] = p;
        set_pixel(buffer, i, j, RGB(r, g, b));

        if j == line {
            set_pixel(buffer, i, j, RGB(0xff, 0, 0));
        }
    }
}

fn process_freq_domain(freqs: &mut Vec<Complex<f32>>) {
    for (n, z) in freqs.iter_mut().enumerate() {
        if n > 0 {
            z.re *= 1.0 / (n as f32);
            z.im *= 1.0 / (n as f32);
        }
    }
}

fn draw_freqs(buffer: &mut Vec<u32>, freqs: &[Complex<f32>], RGB(r, g, b): FTypes) {
    // with skip(1) we remove the DC part
    for (i, Complex { re, im }) in freqs.iter().skip(1).enumerate() {
        if i * 2 >= freqs.len() {
            break;
        }

        let mag = ((*re) * 0.02f32) as i32;
        //let mag = (((*re) * 1.0f32).ln() * 50.0f32) as i32;
        //print!("{} ", mag);

        for m in 0..(mag).abs() {
            add_pixel(
                buffer,
                4 * i + 0,
                ((HEIGHT as i32) - 400 - m * mag.signum()) as usize,
                RGB(r, g, b),
            );
            add_pixel(
                buffer,
                4 * i + 1,
                ((HEIGHT as i32) - 400 - m * mag.signum()) as usize,
                RGB(r, g, b),
            );
            add_pixel(
                buffer,
                4 * i + 2,
                ((HEIGHT as i32) - 400 - m * mag.signum()) as usize,
                RGB(r, g, b),
            );
            add_pixel(
                buffer,
                4 * i + 3,
                ((HEIGHT as i32) - 400 - m * mag.signum()) as usize,
                RGB(r, g, b),
            );
        }
    }
}

fn fft(draw_buf: &mut Vec<u32>, img: &mut image::DynamicImage, line: usize) {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(img.width() as usize);

    let mut buffer = vec![
        Complex {
            re: 0.0f32,
            im: 0.0f32
        };
        img.width() as usize
    ];

    // This is a terrible bottleneck, we should use an iterator over the lines of the image
    for (i, j, image::Rgba(p)) in img.pixels() {
        if j == line as u32 {
            let [_, g, _, _] = p;
            buffer[i as usize].re = g as f32;
        }

        if j > line as u32 {
            break;
        }
    }

    fft.process(&mut buffer);

    draw_freqs(draw_buf, &buffer, RGB(0xaa, 0, 0xaa));
    process_freq_domain(&mut buffer);
    draw_freqs(draw_buf, &buffer, RGB(0xaa, 0xaa, 0));

    let fft = planner.plan_fft_inverse(img.width() as usize);
    fft.process(&mut buffer);

    for (n, z) in buffer.iter().enumerate() {
        let g = (z.re / (HEIGHT as f32)) as u8;
        //add_pixel(
        //    draw_buf,
        //    n + img.width() as usize,
        //    HEIGHT - 5 - u,
        //    RGB(0, 0, 0xff),
        //);

        let rgba: Rgba<u8> = RGB(g, g, g).into();
        let mark: Rgba<u8> = RGB(0xff, 0x10, 0x10).into();

        if img.in_bounds(n as u32, line as u32) {
            //img.put_pixel(n as u32, line as u32, mark)
            img.put_pixel(n as u32, line as u32, rgba)
        }
    }
}

fn main() -> Result<()> {
    let args = std::env::args().collect_vec();

    if args.len() < 2 {
        eprintln!("Usage: {}", &args[0]);
        std::process::exit(1);
    }
    let img_fname = &args[1];
    let mut img = ImageReader::open(img_fname)?.decode()?;

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    //for p in buffer.iter_mut() {
    //    *p = 0xffffff;
    //}

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut frame = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        clear_buf(&mut buffer);
        draw_img(&mut buffer, &img, (frame % HEIGHT));

        fft(&mut buffer, &mut img, frame);

        for (i, j, image::Rgba(p)) in img.pixels() {
            let i = i as usize;
            let j = j as usize;
            let [r, g, b, a] = p;

            if j == (frame % HEIGHT) {
                for u in 0..((g as f32 * 0.2) as usize) {
                    add_pixel(&mut buffer, i, HEIGHT - 5 - u, RGB(0, g, 0));
                }
            }
        }

        frame += 1;

        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }

    Ok(())
}
