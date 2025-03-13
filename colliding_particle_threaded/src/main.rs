use rand::Rng;
use std::time::{Duration, Instant};
use scoped_threadpool::Pool;

use std::sync::atomic::{AtomicUsize, Ordering};

// Set to true to enable debug output, printing particle positions
const DEBUG : bool = false;
const NUM_OF_THREADS: usize = 4;

#[derive(Debug, Copy, Clone)]
struct Particle {
    x: f64,
    y: f64,
}

impl Particle {
    fn new(x: f64, y: f64) -> Self {
        Particle { x, y }
    }

    fn collide(&self, other: &Particle, threshold: f64) -> bool {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let distance_squared = dx * dx + dy * dy;
        distance_squared <= threshold * threshold
    }
}

fn thread_move_particles(list: &mut [Particle], enclosure_size: f64) {
    for particle in list {
        if DEBUG {
            println!("Current position: ({}, {})", particle.x, particle.y)
        }
        loop {
            let rand_x: f64 = rand::random();
            let rand_y: f64 = rand::random();

            let new_x = particle.x + ((rand_x - 0.5) * 2.0);
            let new_y = particle.y + ((rand_y - 0.5) * 2.0);

            if new_x >= 0.0 && new_x <= enclosure_size && new_y >= 0.0 && new_y <= enclosure_size {
                particle.x = new_x;
                particle.y = new_y;
                break;
            }
        }

        if DEBUG {
            println!("New position: ({}, {})", particle.x, particle.y);
        }
    }
}

fn thread_check_collisions(chunk: &[Particle], threshold: f64, collision_counter: &AtomicUsize) {
    for i in 0..chunk.len() {
        for j in (i + 1)..chunk.len() {
            if chunk[i].collide(&chunk[j], threshold) {
                collision_counter.fetch_add(1, Ordering::SeqCst);
                if DEBUG {
                    println!(
                        "Collision detected between particles at ({}, {}) and ({}, {})",
                        chunk[i].x, chunk[i].y, chunk[j].x, chunk[j].y
                    );
                }
            }
        }
    }
}

struct ParticleSystem {
    particles: Vec<Particle>,
    collision_counter: AtomicUsize,
}

impl ParticleSystem {
    fn new(num_particles: usize, max_x: f64, max_y: f64) -> Self {

        let mut particles = Vec::with_capacity(num_particles);
        let mut rng = rand::thread_rng();

        for _ in 0..num_particles {
            let x = rng.gen_range(0.0..max_x);
            let y = rng.gen_range(0.0..max_y);
            particles.push(Particle::new(x, y));
        }

        if DEBUG {
            for particle in &particles {
                println!("Particle at ({}, {})", particle.x, particle.y);
            }
            println!("\nCreated a particle system with {} particles\n", num_particles);
        }
        ParticleSystem {
            particles,
            collision_counter: AtomicUsize::new(0),
        }    }

    fn move_particles(&mut self, enclosure_size: f64, pool: &mut Pool) {
        pool.scoped(|scope| {
            for chunk in self.particles.chunks_mut(NUM_OF_THREADS) {
                scope.execute(move || {
                    thread_move_particles(chunk, enclosure_size);
                });
            }
        });
    }

    fn check_collisions(&self, threshold: f64, pool: &mut Pool) {
        pool.scoped(|scope| {
            for chunk in self.particles.chunks(NUM_OF_THREADS) {
                let collision_counter = &self.collision_counter;
                scope.execute(move || {
                    thread_check_collisions(chunk, threshold, collision_counter);
                });
            }
        });
    }
}

fn main() {
    let enclosure_size = 10.0;
    let collision_threshold = 0.1;

    let mut particle_system = ParticleSystem::new(100, enclosure_size, enclosure_size);

    let start = Instant::now();
    let duration = Duration::new(10, 0);

    let mut count = 0;

    println!("\n\nMoving particles...");

    // initialise thread pool
    let mut pool = Pool::new(NUM_OF_THREADS as u32);

    while Instant::now() - start < duration {
        count += 1;
        pool.scoped(|scope| {
            // Move particles
            for chunk in particle_system.particles.chunks_mut(NUM_OF_THREADS) {
                scope.execute(move || {
                    thread_move_particles(chunk, enclosure_size);
                });
            }

            // Check collisions
            for chunk in particle_system.particles.chunks(NUM_OF_THREADS) {
                let collision_counter = &particle_system.collision_counter;
                scope.execute(move || {
                    thread_check_collisions(chunk, collision_threshold, collision_counter);
                });
            }
        });
    }

    println!("Particles moved {} times in 10 seconds", count);

    println!("Total number of collisions: {}", particle_system.collision_counter.load(Ordering::SeqCst));
}