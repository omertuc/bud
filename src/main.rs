#!cargo r

use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicU16, Ordering::Relaxed},
        Arc,
    },
    thread,
};

const SCREEN_WIDTH: usize = 512;
const SCREEN_HEIGHT: usize = 288;

const ITERATIONS_R: usize = 200;
const ITERATIONS_G: usize = 100;
const ITERATIONS_B: usize = 50;

const POINTS: usize = 10_000_0;

const COMPLEX_PLANE_VIEW_WIDTH: f64 = 4.3;
const COMPLEX_PLANE_VIEW_HEIGHT: f64 =
    (SCREEN_HEIGHT as f64 / SCREEN_WIDTH as f64) * COMPLEX_PLANE_VIEW_WIDTH;

const PAN_RIGHT: f64 = 0.5;

const TOP_LEFT: Complex = Complex {
    re: COMPLEX_PLANE_VIEW_WIDTH / -2.0 - PAN_RIGHT,
    im: COMPLEX_PLANE_VIEW_HEIGHT / 2.0,
};

const PIXEL_WIDTH: f64 = COMPLEX_PLANE_VIEW_WIDTH as f64 / SCREEN_WIDTH as f64;
const PIXEL_HEIGHT: f64 = PIXEL_WIDTH;

#[derive(Debug, Copy, Clone)]
struct Complex {
    re: f64,
    im: f64,
}

#[derive(Debug)]
struct Pixel {
    x: usize,
    y: usize,
}

type Buddhabrot = Vec<Vec<AtomicU16>>;

fn get_pixel(c: &Complex) -> Option<Pixel> {
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

impl Complex {
    fn add(&self, other: &Complex) -> Complex {
        Complex {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    fn mul(&self, other: &Complex) -> Complex {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    fn square(&self) -> Complex {
        self.mul(self)
    }

    fn abssq(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

fn pixels_to_png(
    r: &Buddhabrot,
    g: &Buddhabrot,
    b: &Buddhabrot,
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

    image.save("buddhabrot.bmp")?;

    Ok(())
}

fn generate(iterations: usize, pixels: &Buddhabrot) {
    let mut rng = rand::thread_rng();

    // Create a two dimensional array of pixels

    for _ in 0..POINTS {
        // Generate a random complex number
        let c = Complex {
            re: rng.gen::<f64>() * COMPLEX_PLANE_VIEW_WIDTH as f64 + TOP_LEFT.re,
            im: TOP_LEFT.im - rng.gen::<f64>() * COMPLEX_PLANE_VIEW_HEIGHT as f64,
        };

        let mut visited = Vec::with_capacity(iterations);

        let mut z = Complex { re: 0.0, im: 0.0 };

        for _ in 0..iterations {
            // Calculate the next complex number
            z = z.square().add(&c);

            visited.push(z);

            if z.abssq() > 4.0 {
                for v in visited {
                    let pixel = get_pixel(&v);
                    if let Some(pixel) = pixel {
                        pixels[pixel.y][pixel.x].fetch_add(1, Relaxed);
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

impl Normalize for Buddhabrot {
    fn normalize(self: &Buddhabrot) {
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

unsafe fn generate_channel(iterations: usize) -> Arc<Buddhabrot> {
    let num_cores = 32;

    let mut threads = vec![];

    let mut pixels: Buddhabrot = Vec::with_capacity(SCREEN_HEIGHT);
    for _ in 0..SCREEN_HEIGHT {
        let mut row = Vec::with_capacity(SCREEN_WIDTH);
        for _ in 0..SCREEN_WIDTH {
            row.push(AtomicU16::new(0));
        }
        pixels.push(row);
    }

    let pixels = Arc::new(pixels);

    for _i in 0..num_cores {
        let pixels = Arc::clone(&pixels);
        threads.push(thread::spawn(move || {
            generate(iterations, &pixels);
        }));
    }

    threads.into_iter().for_each(|t| t.join().unwrap());

    pixels
}

fn main() {
    unsafe {
        println!("Generating red");
        let r = generate_channel(ITERATIONS_R);
        println!("Generating green");
        let g = generate_channel(ITERATIONS_G);
        println!("Generating blue");
        let b = generate_channel(ITERATIONS_B);

        println!("Normalizing red");
        r.normalize();
        println!("Normalizing green");
        g.normalize();
        println!("Normalizing blue");
        b.normalize();

        pixels_to_png(&r, &g, &b).unwrap();
    }
}
