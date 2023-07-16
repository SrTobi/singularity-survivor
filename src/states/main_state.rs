use std::f32::consts::PI;

use macroquad::prelude::*;

use crate::GameState;

use super::menu_state::MenuState;

const SHIP_HEIGHT: f32 = 25.;
const SHIP_BASE: f32 = 22.;
const ROCKET_SIZE: f32 = 8.;

struct Ship {
    pos: Vec2,
    rot: f32,
    vel: Vec2,
}

struct Bullet {
    pos: Vec2,
    vel: Vec2,
    shot_at: f64,
    collided: bool,
}

struct Asteroid {
    pos: Vec2,
    vel: Vec2,
    rot: f32,
    rot_speed: f32,
    size: f32,
    sides: u8,
    collided: bool,
    shape_idx: usize,
}

impl Asteroid {
    fn new(pos: Vec2, asteroid_shapes: &Vec<AsteroidShape>) -> Asteroid {
        Asteroid {
            pos,
            vel: Vec2::new(rand::gen_range(-1., 1.), rand::gen_range(-1., 1.)),
            rot: 0.,
            rot_speed: rand::gen_range(-2., 2.),
            size: screen_width().min(screen_height()) / 10.,
            sides: rand::gen_range(3, 8),
            collided: false,
            shape_idx: rand::gen_range(0, asteroid_shapes.len()),
        }
    }
}

struct Rocket {
    pos: Vec2,
    vel: Vec2,
    rot: f32,
    collided: bool,
    shot_at: f64,
    steer: bool,
}

/*fn wrap_around(v: &Vec2) -> Vec2 {
    let mut vr = Vec2::new(v.x, v.y);
    if vr.x > screen_width() {
        vr.x = 0.;
    }
    if vr.x < 0. {
        vr.x = screen_width()
    }
    if vr.y > screen_height() {
        vr.y = 0.;
    }
    if vr.y < 0. {
        vr.y = screen_height()
    }
    vr
}*/

fn rand_signum() -> f32 {
    rand::gen_range::<f32>(-1., 1.).signum()
}

fn vec_from_rot(rot: f32) -> Vec2 {
    Vec2::new(rot.sin(), -rot.cos())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RocketSide {
    Right,
    Left,
}

impl RocketSide {
    fn switch(self) -> Self {
        match self {
            RocketSide::Left => RocketSide::Right,
            RocketSide::Right => RocketSide::Left,
        }
    }
}

pub struct MainState {
    ship: Ship,
    last_asteroid_generate_pos: Vec2,
    generated_asteroids: usize,
    bullets: Vec<Bullet>,
    last_shot: f64,
    asteroids: Vec<Asteroid>,
    rockets: Vec<Rocket>,
    rocket_side: RocketSide,
    asteroid_shapes: Vec<AsteroidShape>,
}

impl MainState {
    pub fn new() -> Self {
        let ship = Ship {
            pos: Vec2::new(screen_width() / 2., screen_height() / 2.),
            rot: 0.,
            vel: Vec2::new(0., 0.),
        };
        let screen_center = Vec2::new(screen_width() / 2., screen_height() / 2.);

        let asteroid_shapes: Vec<_> = (0..5).map(|_| AsteroidShape::new()).collect();

        let mut asteroids = Vec::new();
        for _ in 0..250 {
            let x = rand::gen_range(SHIP_HEIGHT * 10., 2.5 * screen_width());
            let y = rand::gen_range(SHIP_HEIGHT * 10., 2.5 * screen_height());
            let pos = Vec2::new(rand_signum() * x, rand_signum() * y);
            asteroids.push(Asteroid::new(screen_center + pos, &asteroid_shapes));
        }

        Self {
            last_asteroid_generate_pos: ship.pos,
            ship,
            generated_asteroids: asteroids.len(),
            bullets: Vec::new(),
            rockets: Vec::new(),
            last_shot: get_time(),
            rocket_side: RocketSide::Right,
            asteroids,
            asteroid_shapes,
        }
    }
}

impl GameState for MainState {
    fn do_frame(&mut self) -> Option<Box<dyn GameState>> {
        let screen_diag_length_squared =
            Vec2::new(screen_width(), screen_height()).length_squared();
        let screen_diag_length = screen_diag_length_squared.sqrt();
        let world_diag_length = screen_diag_length * 5.;

        let frame_t = get_time();
        let rotation = self.ship.rot.to_radians();

        let mut acc = -self.ship.vel / 100.; // Friction

        // Forward
        if is_key_down(KeyCode::Up) {
            acc = vec_from_rot(rotation) / 3.;
        } else if is_key_down(KeyCode::Down) {
            acc = -vec_from_rot(rotation) / 5.;
        }

        // Shot
        if is_key_down(KeyCode::Space) && frame_t - self.last_shot > 0.5 {
            let rot_vec = vec_from_rot(rotation);
            self.bullets.push(Bullet {
                pos: self.ship.pos + rot_vec * SHIP_HEIGHT / 2.,
                vel: rot_vec * 10.,
                shot_at: frame_t,
                collided: false,
            });
            self.last_shot = frame_t;
        }

        // shoot rocket
        if is_key_down(KeyCode::LeftAlt) && frame_t - self.last_shot > 0.01 {
            let sf = match self.rocket_side {
                RocketSide::Left => -1.,
                RocketSide::Right => 1.,
            };
            let rot_vec = vec_from_rot(rotation + sf * rand::gen_range(1.0, 1.4) * PI / 2.);
            self.rocket_side = self.rocket_side.switch();
            self.rockets.push(Rocket {
                pos: self.ship.pos + rot_vec * SHIP_HEIGHT / 2.,
                vel: self.ship.vel * 0.9 + rot_vec * rand::gen_range(0.7, 1.2),
                rot: self.ship.rot,
                shot_at: frame_t,
                collided: false,
                steer: false,
            });
            self.last_shot = frame_t;
        }

        // Steer
        if is_key_down(KeyCode::Right) {
            self.ship.rot += 4.;
        } else if is_key_down(KeyCode::Left) {
            self.ship.rot -= 4.;
        }

        // Euler integration
        self.ship.vel += acc;
        if self.ship.vel.length() > 5. {
            self.ship.vel = self.ship.vel.normalize() * 5.;
        }
        self.ship.pos += self.ship.vel;
        //self.ship.pos = wrap_around(&self.ship.pos);

        // Move each bullet
        for bullet in self.bullets.iter_mut() {
            bullet.pos += bullet.vel;
            //bullet.pos = wrap_around(&bullet.pos);
        }

        // Move each rocket
        for rocket in self.rockets.iter_mut() {
            if rocket.shot_at + 0.3 < frame_t {
                if rocket.vel.length() > 8. {
                    rocket.steer = true;
                }
                if rocket.steer {
                    let rrot = vec_from_rot(rocket.rot.to_radians());

                    // steer rocket
                    let target = self.asteroids.iter().min_by_key(|a| {
                        let angle = rrot.angle_between(a.pos - rocket.pos).to_degrees().abs();
                        let angle = (angle as i32).max(20);

                        (angle, a.pos.distance(rocket.pos) as i32)
                    });

                    if let Some(target) = target {
                        let angle = rrot.angle_between(target.pos - rocket.pos).to_degrees();
                        rocket.rot += angle.min(10.);
                    }
                }

                // accelerate rocket
                let acc = 0.6 * vec_from_rot(rocket.rot.to_radians());
                rocket.vel += acc;
                if rocket.vel.length() > 15. {
                    rocket.vel = rocket.vel.normalize() * 15.;
                }
            }
            rocket.pos += rocket.vel;
            //rocket.pos = wrap_around(&rocket.pos);
        }

        // Move each asteroid
        for asteroid in self.asteroids.iter_mut() {
            asteroid.pos += asteroid.vel;
            //asteroid.pos = wrap_around(&asteroid.pos);
            asteroid.rot += asteroid.rot_speed;
        }

        // Bullet lifetime
        self.bullets.retain(|bullet| bullet.shot_at + 2.5 > frame_t);

        let mut new_asteroids = Vec::new();
        for asteroid in self.asteroids.iter_mut() {
            // Asteroid/ship collision
            if (asteroid.pos - self.ship.pos).length() < asteroid.size + SHIP_HEIGHT / 3. {
                return Some(Box::new(MenuState::Lost));
            }

            let mut hit_vel = None;

            // Asteroid/bullet collision
            for bullet in self.bullets.iter_mut() {
                if (asteroid.pos - bullet.pos).length() < asteroid.size {
                    bullet.collided = true;
                    hit_vel = Some(bullet.vel);
                    break;
                }
            }

            // Asteroid/rocket collision
            for rocket in self.rockets.iter_mut() {
                if (asteroid.pos - rocket.pos).length() < (asteroid.size + ROCKET_SIZE) {
                    rocket.collided = true;
                    hit_vel = Some(rocket.vel);
                    break;
                }
            }

            if let Some(hit_vel) = hit_vel {
                asteroid.collided = true;

                // Break the asteroid
                if asteroid.sides > 3 {
                    new_asteroids.push(Asteroid {
                        pos: asteroid.pos,
                        vel: Vec2::new(hit_vel.y, -hit_vel.x).normalize() * rand::gen_range(1., 3.),
                        rot: rand::gen_range(0., 360.),
                        rot_speed: rand::gen_range(-2., 2.),
                        size: asteroid.size * 0.8,
                        sides: asteroid.sides - 1,
                        collided: false,
                        shape_idx: rand::gen_range(0, self.asteroid_shapes.len()),
                    });
                    new_asteroids.push(Asteroid {
                        pos: asteroid.pos,
                        vel: Vec2::new(-hit_vel.y, hit_vel.x).normalize() * rand::gen_range(1., 3.),
                        rot: rand::gen_range(0., 360.),
                        rot_speed: rand::gen_range(-2., 2.),
                        size: asteroid.size * 0.8,
                        sides: asteroid.sides - 1,
                        collided: false,
                        shape_idx: rand::gen_range(0, self.asteroid_shapes.len()),
                    })
                }
                break;
            }
        }

        // generate new asteroids
        if self.last_asteroid_generate_pos.distance(self.ship.pos) > 50. {
            let gen_vec = self.ship.pos - self.last_asteroid_generate_pos;
            let asteroid_per_pixel = 10. / (screen_height() * screen_width());
            let new_x_pixel = gen_vec.x.abs() * screen_height();
            let new_y_pixel = gen_vec.y.abs() * screen_width();
            let new_pixels = 5. * new_x_pixel + 5. * new_y_pixel - gen_vec.x * gen_vec.y;
            let amount_new_asteroids = asteroid_per_pixel * new_pixels;
            info!("new asteroids: {}", amount_new_asteroids);
            let amount_new_asteroids = rand::gen_range(
                (0.8 * amount_new_asteroids) as usize,
                2 + (amount_new_asteroids * 1.2) as usize,
            );

            for _ in 0..amount_new_asteroids {
                let x_pixel_ratio = new_x_pixel / (new_x_pixel + new_y_pixel);
                let pos = if rand::gen_range(0., 1.) < x_pixel_ratio {
                    // x
                    Vec2::new(
                        gen_vec.x.signum()
                            * (2.5 * screen_width() - rand::gen_range(0., gen_vec.x.abs())),
                        rand::gen_range(-2.5 * screen_height(), 2.5 * screen_height()),
                    )
                } else {
                    // y
                    Vec2::new(
                        rand::gen_range(-2.5 * screen_width(), 2.5 * screen_width()),
                        gen_vec.y.signum()
                            * (2.5 * screen_height() - rand::gen_range(0., gen_vec.y.abs())),
                    )
                };

                self.generated_asteroids += 1;
                new_asteroids.push(Asteroid::new(self.ship.pos + pos, &self.asteroid_shapes))
            }

            self.last_asteroid_generate_pos = self.ship.pos;
        }

        // Remove the collided objects
        self.bullets
            .retain(|bullet| bullet.shot_at + 1.5 > frame_t && !bullet.collided);
        self.rockets
            .retain(|rocket| rocket.shot_at + 4. > frame_t && !rocket.collided);
        self.asteroids.retain(|asteroid| {
            !asteroid.collided && self.ship.pos.distance(asteroid.pos) < world_diag_length / 2.
        });
        self.asteroids.append(&mut new_asteroids);

        // You win?
        if self.asteroids.len() == 0 {
            return Some(Box::new(MenuState::Won));
        }

        let cam_pos = self.ship.pos - Vec2::new(screen_width(), -screen_height()) / 2.;
        let rect = Rect::new(cam_pos.x, cam_pos.y, screen_width(), -screen_height());
        let camera = Camera2D::from_display_rect(rect);
        set_camera(&camera);
        clear_background(LIGHTGRAY);

        let in_screen = |pos: Vec2, size: f32| {
            pos.distance(self.ship.pos) < screen_diag_length / 2. + SHIP_HEIGHT + size
        };

        for bullet in self.bullets.iter() {
            if in_screen(bullet.pos, 2.) {
                draw_circle(bullet.pos.x, bullet.pos.y, 2., BLACK);
            }
        }

        for rocket in self.rockets.iter() {
            if in_screen(rocket.pos, ROCKET_SIZE) {
                let rr = rocket.rot.to_radians();
                let rv = vec_from_rot(rr) * ROCKET_SIZE / 2.;
                let p0 = rocket.pos + rv;
                let p1 = rocket.pos - rv;
                draw_line(p0.x, p0.y, p1.x, p1.y, 2., BLACK);
            }
        }

        for asteroid in self.asteroids.iter() {
            if in_screen(asteroid.pos, asteroid.size) {
                let shape = &self.asteroid_shapes[asteroid.shape_idx];

                shape.draw(
                    asteroid.pos.x,
                    asteroid.pos.y,
                    asteroid.size,
                    asteroid.rot,
                    2.,
                    BLACK,
                )
            }
        }

        let v1 = Vec2::new(
            self.ship.pos.x + rotation.sin() * SHIP_HEIGHT / 2.,
            self.ship.pos.y - rotation.cos() * SHIP_HEIGHT / 2.,
        );
        let v2 = Vec2::new(
            self.ship.pos.x - rotation.cos() * SHIP_BASE / 2. - rotation.sin() * SHIP_HEIGHT / 2.,
            self.ship.pos.y - rotation.sin() * SHIP_BASE / 2. + rotation.cos() * SHIP_HEIGHT / 2.,
        );
        let v3 = Vec2::new(
            self.ship.pos.x + rotation.cos() * SHIP_BASE / 2. - rotation.sin() * SHIP_HEIGHT / 2.,
            self.ship.pos.y + rotation.sin() * SHIP_BASE / 2. + rotation.cos() * SHIP_HEIGHT / 2.,
        );
        draw_triangle_lines(v1, v2, v3, 2., BLACK);

        set_default_camera();

        draw_text(
            &format!(
                "Asteroids: {} ({}), Bullets: {}, Rockets: {}",
                self.asteroids.len(),
                self.generated_asteroids,
                self.bullets.len(),
                self.rockets.len()
            ),
            30.,
            30.,
            30.,
            BLACK,
        );

        None
    }
}

struct AsteroidShape {
    corners: Vec<(f32, f32)>,
}

impl AsteroidShape {
    fn new() -> Self {
        let mut corners = Vec::new();

        for _ in 0..rand::gen_range(6, 12) {
            let arc_offset = rand::gen_range(-0.3, 0.3);
            let radius_factor = rand::gen_range(0.9, 1.1);
            corners.push((arc_offset, radius_factor));
        }

        Self { corners }
    }

    fn draw(&self, x: f32, y: f32, radius: f32, rotation: f32, thickness: f32, color: Color) {
        let rot = rotation.to_radians();
        let sides = self.corners.len();

        fn p(x: f32, y: f32, rot: f32, arc: f32, radius: f32) -> Vec2 {
            let rx = (arc * std::f32::consts::PI * 2. + rot).cos();
            let ry = (arc * std::f32::consts::PI * 2. + rot).sin();

            vec2(x + radius * rx, y + radius * ry)
        }

        for i in 0..sides {
            let (ao, rf) = self.corners[i];
            let p0 = p(x, y, rot, (ao + i as f32) / sides as f32, radius * rf);

            let (ao, ro) = self.corners[(i + 1) % sides];
            let p1 = p(x, y, rot, (ao + (i + 1) as f32) / sides as f32, radius * ro);

            draw_line(p0.x, p0.y, p1.x, p1.y, thickness, color);
        }
    }
}
