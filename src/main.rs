use num_complex::Complex;
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicU16, Ordering::Relaxed},
        Arc,
    },
    thread,
};

const SCREEN_WIDTH: usize = 500 * 1;
const SCREEN_HEIGHT: usize = 300 * 1;

const ITERATIONS_R: usize = 50;
const ITERATIONS_G: usize = 40;
const ITERATIONS_B: usize = 30;

const POINTS: usize = 10_000_000;

const COMPLEX_PLANE_VIEW_WIDTH: f64 = 4.3;
const COMPLEX_PLANE_VIEW_HEIGHT: f64 =
    (SCREEN_HEIGHT as f64 / SCREEN_WIDTH as f64) * COMPLEX_PLANE_VIEW_WIDTH;

const PAN_RIGHT: f64 = 0.5;

const TOP_LEFT: Complex<f64> = Complex::<f64> {
    re: COMPLEX_PLANE_VIEW_WIDTH / -2.0 - PAN_RIGHT,
    im: COMPLEX_PLANE_VIEW_HEIGHT / 2.0,
};

const PIXEL_WIDTH: f64 = COMPLEX_PLANE_VIEW_WIDTH as f64 / SCREEN_WIDTH as f64;
const PIXEL_HEIGHT: f64 = PIXEL_WIDTH;

#[derive(Debug)]
struct Pixel {
    x: usize,
    y: usize,
}

type BuddhabrotChannel = Vec<Vec<AtomicU16>>;

fn get_pixel(c: &Complex<f64>) -> Option<Pixel> {
    if c.re < TOP_LEFT.re
        || c.re > TOP_LEFT.re + COMPLEX_PLANE_VIEW_WIDTH
        || c.im > TOP_LEFT.im
        || c.im < TOP_LEFT.im - COMPLEX_PLANE_VIEW_HEIGHT
    {
        return None;
    }

    return Some(Pixel {
        x: ((c.re - TOP_LEFT.re) / PIXEL_WIDTH) as usize,
        y: ((TOP_LEFT.im - c.im) / PIXEL_HEIGHT) as usize,
    });
}

fn pixels_to_png(
    r: &BuddhabrotChannel,
    g: &BuddhabrotChannel,
    b: &BuddhabrotChannel,
    fname: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut image = image::ImageBuffer::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);

    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            image.put_pixel(
                x as u32,
                y as u32,
                image::Rgb([
                    r[y][x].load(Relaxed) as u8,
                    g[y][x].load(Relaxed) as u8,
                    b[y][x].load(Relaxed) as u8,
                ]),
            );
        }
    }

    image.save(fname)?;

    Ok(())
}

fn generate(r: &BuddhabrotChannel, g: &BuddhabrotChannel, b: &BuddhabrotChannel, pow: f64) {
    let mut rng = rand::thread_rng();

    // Create a two dimensional array of pixels
    let mut visited = [Complex::default(); ITERATIONS_R];
    for _ in 0..POINTS {
        // Generate a random complex number
        let c = Complex::<f64> {
            re: rng.gen::<f64>() * COMPLEX_PLANE_VIEW_WIDTH as f64 + TOP_LEFT.re,
            im: TOP_LEFT.im - rng.gen::<f64>() * COMPLEX_PLANE_VIEW_HEIGHT as f64,
        };

        let mut z = Complex::<f64> { re: 0.0, im: 0.0 };
        for i in 0..ITERATIONS_R {
            // Calculate the next complex number
            z = z.powf(pow) + c;

            visited[i] = z;

            if z.re * z.re + z.im * z.im > 4.0 {
                let should_green = i < ITERATIONS_G;
                let should_blue = i < ITERATIONS_B;

                for (i, v) in visited.iter().take(i).enumerate() {
                    let pixel = get_pixel(&v);

                    if let Some(pixel) = pixel {
                        r[pixel.y][pixel.x].fetch_add(1, Relaxed);
                        if should_green && i < ITERATIONS_G {
                            g[pixel.y][pixel.x].fetch_add(1, Relaxed);

                            if should_blue && i < ITERATIONS_B {
                                b[pixel.y][pixel.x].fetch_add(1, Relaxed);
                            }
                        }
                    }
                }

                break;
            }
        }
    }
}

trait Normalize {
    fn normalize(&self);
}

impl Normalize for BuddhabrotChannel {
    fn normalize(self: &BuddhabrotChannel) {
        let mut max = 0;

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let value = self[y][x].load(Relaxed);
                if value > max {
                    max = value;
                }
            }
        }

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let value = self[y][x].load(Relaxed);
                self[y][x].store(((value as f64 / max as f64) * 255.0) as u16, Relaxed);
            }
        }
    }
}

fn generate_channel(pow: f64) -> (
    Arc<BuddhabrotChannel>,
    Arc<BuddhabrotChannel>,
    Arc<BuddhabrotChannel>,
) {
    let num_cores = 32;

    let mut threads = vec![];

    let mut r: BuddhabrotChannel = Vec::with_capacity(SCREEN_HEIGHT);
    let mut g: BuddhabrotChannel = Vec::with_capacity(SCREEN_HEIGHT);
    let mut b: BuddhabrotChannel = Vec::with_capacity(SCREEN_HEIGHT);

    for _ in 0..SCREEN_HEIGHT {
        let mut row_r = Vec::with_capacity(SCREEN_WIDTH);
        let mut row_g = Vec::with_capacity(SCREEN_WIDTH);
        let mut row_b = Vec::with_capacity(SCREEN_WIDTH);
        for _ in 0..SCREEN_WIDTH {
            row_r.push(AtomicU16::new(0));
            row_g.push(AtomicU16::new(0));
            row_b.push(AtomicU16::new(0));
        }
        r.push(row_r);
        g.push(row_g);
        b.push(row_b);
    }

    let r = Arc::new(r);
    let g = Arc::new(g);
    let b = Arc::new(b);

    for _i in 0..num_cores {
        let r = Arc::clone(&r);
        let g = Arc::clone(&g);
        let b = Arc::clone(&b);
        threads.push(thread::spawn(move || {
            generate(&r, &g, &b, pow);
        }));
    }

    threads.into_iter().for_each(|t| t.join().unwrap());

    r.normalize();
    g.normalize();
    b.normalize();

    (r, g, b)
}

fn main() {
    const FRAMES: usize = 10 * 60;
    const FROM: f64 = 1.0;
    const TO: f64 = 2.0;

    for i in 0..(FRAMES) {
        let done = i as f64 / FRAMES as f64;
        let (r, g, b) = generate_channel((i as f64 / FRAMES as f64) * (TO - FROM) + FROM);
        pixels_to_png(&r, &g, &b, format!("frame-{:08}.png", i)).unwrap();
        println!("{:.2}% Done", done * 100.0);

    }

}
