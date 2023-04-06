use anyhow::{Context, Result};
use image::{io::Reader as ImageReader, GenericImageView};
use itertools::Itertools;
use minifb::{Key, Window, WindowOptions};
use rustfft::{num_complex::Complex, FftPlanner};

const WIDTH: usize = 2400;
const HEIGHT: usize = 1600;

fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
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
        buffer[i + j * WIDTH] = from_u8_rgb(r, g, b);

        if j == line {
            buffer[i + j * WIDTH] = 0xff0000;
        }
    }
}

fn draw_pixel(buffer: &mut Vec<u32>, i: u32, j: u32, pixel: u32) {}

fn fft(draw_buf: &mut Vec<u32>, img: &image::DynamicImage, line: usize) {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(img.width() as usize);

    let mut buffer = vec![
        Complex {
            re: 0.0f32,
            im: 0.0f32
        };
        img.width() as usize
    ];

    for (i, j, image::Rgba(p)) in img.pixels() {
        if j == line as u32 {
            let [_, g, _, _] = p;
            buffer[i as usize].re = g as f32;
        }
    }

    fft.process(&mut buffer);

    // with skip(1) we remove the DC part
    for (i, Complex { re, im }) in buffer.iter().skip(1).enumerate() {
        if i * 4 >= buffer.len() {
            break;
        }

        let mag = ((*re) * 0.02f32) as i32;
        //let mag = (((*re) * 1.0f32).ln() * 50.0f32) as i32;
        //print!("{} ", mag);

        for m in 0..(mag).abs() {
            draw_buf[4 * i + 0 + WIDTH * (((HEIGHT as i32) - 400 - m * mag.signum()) as usize)] =
                0xff00ff;
            draw_buf[4 * i + 1 + WIDTH * (((HEIGHT as i32) - 400 - m * mag.signum()) as usize)] =
                0xff00ff;
            draw_buf[4 * i + 2 + WIDTH * (((HEIGHT as i32) - 400 - m * mag.signum()) as usize)] =
                0xff00ff;
            draw_buf[4 * i + 3 + WIDTH * (((HEIGHT as i32) - 400 - m * mag.signum()) as usize)] =
                0xff00ff;
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
    let img = ImageReader::open(img_fname)?.decode()?;

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
        fft(&mut buffer, &img, frame);

        for (i, j, image::Rgba(p)) in img.pixels() {
            let i = i as usize;
            let j = j as usize;
            let [r, g, b, a] = p;

            if j == (frame % HEIGHT) {
                //for u in 0..((r as usize) * 2) {
                //    buffer[i + WIDTH * (HEIGHT - 5 - u)] |= (r as u32) << 16;
                //}

                for u in 0..((g as usize) * 2) {
                    buffer[i + WIDTH * (HEIGHT - 5 - u)] |= (g as u32) << 8;
                }

                //for u in 0..((b as usize) * 2) {
                //    buffer[i + WIDTH * (HEIGHT - 5 - u)] |= (b as u32);
                //}
            }
        }

        frame += 1;

        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }

    Ok(())
}
