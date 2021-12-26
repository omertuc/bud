#!cargo r

use derive_deref::{Deref, DerefMut};
use rand::Rng;
use std::{
    ops::Add,
    sync::{Arc, Mutex},
    thread,
};

const SCREEN_WIDTH: usize = 2560;
const SCREEN_HEIGHT: usize = 1440;

const ITERATIONS_R: usize = 200;
const ITERATIONS_G: usize = 100;
const ITERATIONS_B: usize = 50;

const POINTS: usize = 1_000_000_000;

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

fn pixels_to_png(r: Buddhabrot, g: Buddhabrot, b: Buddhabrot) -> Result<(), Box<dyn std::error::Error>> {
    let mut image = image::ImageBuffer::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);

    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            image.put_pixel(
                x as u32,
                y as u32,
                image::Rgb([
                    r[y][x] as u8,
                    g[y][x] as u8,
                    b[y][x] as u8
                ]),
            );
        }
    }

    image.save("buddhabrot.png")?;

    Ok(())
}

#[derive(Deref, DerefMut, Debug, Clone)]
struct Buddhabrot(Vec<Vec<usize>>);

impl Buddhabrot {
    fn normalize(self) -> Buddhabrot {
        let mut max = 0;
        for row in self.0.iter() {
            for &color in row.iter() {
                if color > max {
                    max = color;
                }
            }
        }

        let mut new_pixels = Vec::with_capacity(SCREEN_HEIGHT);
        for row in self.0.iter() {
            let mut new_row = Vec::with_capacity(SCREEN_WIDTH);
            for &color in row.iter() {
                new_row.push((((color as f64) / (max as f64)) * 255.0) as usize);
            }
            new_pixels.push(new_row);
        }

        Buddhabrot(new_pixels)
    }
}

impl Add<Buddhabrot> for Buddhabrot {
    type Output = Buddhabrot;

    fn add(self, other: Buddhabrot) -> Buddhabrot {
        let mut new_pixels = Vec::with_capacity(self.0.len());

        for (row_a, row_b) in self.0.iter().zip(other.0.iter()) {
            let mut new_row = Vec::with_capacity(row_a.len());

            for (a, b) in row_a.iter().zip(row_b.iter()) {
                new_row.push(a + b);
            }

            new_pixels.push(new_row);
        }

        Buddhabrot(new_pixels)
    }
}

fn generate(iterations: usize) -> Buddhabrot {
    let mut rng = rand::thread_rng();

    // Create a two dimensional array of pixels
    let mut pixels: Vec<Vec<usize>> = vec![vec![0; SCREEN_WIDTH]; SCREEN_HEIGHT];

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
                        pixels[pixel.y][pixel.x] += 1;
                    }
                }
                break;
            }
        }
    }

    Buddhabrot(pixels)
}

fn generate_channel(iterations: usize) -> Buddhabrot {
    let num_cores = num_cpus::get();

    let results = Arc::new(Mutex::new(vec![]));
    let mut threads = vec![];

    for i in 1..num_cores {
        let results = results.clone();
        threads.push(thread::spawn(move || {
            let result = generate(iterations);

            println!("Thread {} finished", i);

            let mut results = results.lock().unwrap();
            results.push(result);
        }));
    }

    threads.into_iter().for_each(|t| t.join().unwrap());

    let results = (*results.lock().unwrap()).clone();
    let joint_buddha = results.into_iter().fold(Buddhabrot(vec![vec![0; SCREEN_WIDTH]; SCREEN_HEIGHT]), |acc, x| {
        acc + x
    });

    joint_buddha
}

fn main() {
    dbg!("Generating red");
    let r = generate_channel(ITERATIONS_R).normalize();
    dbg!("Generating green");
    let g = generate_channel(ITERATIONS_G).normalize();
    dbg!("Generating blue");
    let b = generate_channel(ITERATIONS_B).normalize();

    pixels_to_png(r, g, b).unwrap();
}
