use std::{
    cell::Cell,
    collections::hash_map::DefaultHasher,
    f32::consts::PI,
    hash::{Hash, Hasher},
    rc::Rc,
};

use macroquad::prelude::*;

use crate::{utils::draw_centered_text, GameState};

use super::menu_state::MenuState;

const SHIP_HEIGHT: f32 = 25.;
const SHIP_BASE: f32 = 22.;
const ROCKET_SIZE: f32 = 8.;

const BULLET_LIFETIME: f32 = 1.5; // sec
const ROCKET_LIFETIME: f32 = 4.0; // sec

const ASTEROID_DENSITY: usize = 4;

const SHIP_ROTATION_SPEED: f32 = 4.; // deg/frame

trait BlackHoleEffected {
    fn pos(&self) -> Vec2;
    fn vel(&mut self) -> &mut Vec2;
    fn radius(&self) -> f32;
    fn collide(&mut self);
}

struct Ship {
    pos: Vec2,
    rot: f32,
    vel: Vec2,
}

impl BlackHoleEffected for Ship {
    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn vel(&mut self) -> &mut Vec2 {
        &mut self.vel
    }

    fn radius(&self) -> f32 {
        SHIP_HEIGHT / 3.
    }

    fn collide(&mut self) {}
}

struct Bullet {
    pos: Vec2,
    vel: Vec2,
    shot_at: f32,
    collided: bool,
}

impl BlackHoleEffected for Bullet {
    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn vel(&mut self) -> &mut Vec2 {
        &mut self.vel
    }

    fn radius(&self) -> f32 {
        2.
    }

    fn collide(&mut self) {
        self.collided = true;
    }
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

impl BlackHoleEffected for Asteroid {
    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn vel(&mut self) -> &mut Vec2 {
        &mut self.vel
    }

    fn radius(&self) -> f32 {
        self.size
    }

    fn collide(&mut self) {
        self.collided = true;
    }
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
    shot_at: f32,
    steer: bool,
}

impl BlackHoleEffected for Rocket {
    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn vel(&mut self) -> &mut Vec2 {
        &mut self.vel
    }

    fn radius(&self) -> f32 {
        5.
    }

    fn collide(&mut self) {
        self.collided = true;
    }
}

struct BlackHole {
    pos: Cell<Vec2>,
    vel: Cell<Vec2>,
    size: f32,
    collided: Cell<bool>,
}

impl BlackHole {
    fn pos(&self) -> Vec2 {
        self.pos.get()
    }

    fn vel(&self) -> Vec2 {
        self.vel.get()
    }
}

struct Upgrade {
    desc: Box<dyn Fn(&MainState) -> String>,
    effect: Box<dyn Fn(&mut MainState) -> bool>,
}

impl Upgrade {
    fn new(
        desc: impl Fn(&MainState) -> String + 'static,
        effect: impl Fn(&mut MainState) -> bool + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            desc: Box::new(desc),
            effect: Box::new(effect),
        })
    }

    fn simple(desc: &str, effect: impl Fn(&mut MainState) -> bool + 'static) -> Rc<Self> {
        let desc = desc.to_string();
        Self::new(move |_| desc.clone(), effect)
    }
}

fn make_upgrades() -> Vec<Rc<Upgrade>> {
    vec![
        Upgrade::simple("Install brakes", |s| {
            s.has_brakes = true;
            false
        }),
        {
            let next_rockets = Rc::new(Cell::new(5));
            let next_rockets2 = next_rockets.clone();

            Upgrade::new(
                move |_| format!("+{} Missiles", next_rockets.get()),
                move |s| {
                    let new_rockets = next_rockets2.get();
                    s.rocket_stockpile += new_rockets;
                    next_rockets2.set(new_rockets + 5);
                    true
                },
            )
        },
        Upgrade::simple("-20% Missle reload time", |s| {
            s.rocket_reload_time *= 0.8;
            s.rocket_reload_time > 0.05
        }),
        Upgrade::simple("-20% Bullet reload time", |s| {
            s.bullet_reload_time *= 0.8;
            s.bullet_reload_time > 0.05
        }),
        Upgrade::simple("+0.3 Missile production/s", |s| {
            s.rocket_production_per_sec += 0.3;
            true
        }),
        Upgrade::new(
            |s| {
                if s.shield_regeneration_per_sec == 0. {
                    "Install Shields".to_string()
                } else {
                    "+0.5 Shield production/min".to_string()
                }
            },
            |s| {
                if s.shield_regeneration_per_sec == 0. {
                    s.shields = 1.;
                    s.shield_regeneration_per_sec = 0.1 / 60.;
                } else {
                    s.shield_regeneration_per_sec += 0.5 / 60.;
                }
                true
            },
        ),
    ]
}

struct LevelUp {
    selected: usize,
    upgrade_choices: Vec<Rc<Upgrade>>,
}

impl LevelUp {
    fn new(choices: usize, mut available_upgrades: Vec<Rc<Upgrade>>) -> Self {
        let mut upgrade_choices = Vec::new();

        for _ in 0..choices {
            if available_upgrades.is_empty() {
                break;
            }

            let i = rand::gen_range(0, available_upgrades.len());
            upgrade_choices.push(available_upgrades.remove(i));
        }

        Self {
            selected: 0,
            upgrade_choices,
        }
    }
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
    paused: bool,
    game_t: f32,
    ship: Ship,
    invulnerable_until: f32,
    colliding: bool,
    last_asteroid_generate_pos: Vec2,
    generated_asteroids: usize,
    bullets: Vec<Bullet>,
    last_bullet_shot: f32,
    last_rocket_shot: f32,
    asteroids: Vec<Asteroid>,
    rockets: Vec<Rocket>,
    rocket_side: RocketSide,
    asteroid_shapes: Vec<AsteroidShape>,

    black_holes: Vec<BlackHole>,

    level_up: Option<LevelUp>,
    level: usize,
    xp: usize,
    next_level_xp: usize,
    hostile_asteroids_per_second: f32,
    new_hostile_asteroids: f32,
    max_hostile_asteroid_speed: f32,

    available_upgrades: Vec<Rc<Upgrade>>,
    has_brakes: bool,

    shields: f32,
    shield_regeneration_per_sec: f32,
    rocket_stockpile: usize,
    rocket_production_progress: f32,
    rocket_production_per_sec: f32,

    bullet_reload_time: f32,
    rocket_reload_time: f32,
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
        for _ in 0..(ASTEROID_DENSITY * 5 * 5) {
            let x = rand::gen_range(SHIP_HEIGHT * 10., 2.5 * screen_width());
            let y = rand::gen_range(SHIP_HEIGHT * 10., 2.5 * screen_height());
            let pos = Vec2::new(rand_signum() * x, rand_signum() * y);
            asteroids.push(Asteroid::new(screen_center + pos, &asteroid_shapes));
        }

        Self {
            game_t: 0.,
            paused: false,
            last_asteroid_generate_pos: ship.pos,
            invulnerable_until: 0.,
            colliding: false,
            ship,
            generated_asteroids: asteroids.len(),
            bullets: Vec::new(),
            rockets: Vec::new(),
            last_bullet_shot: 0.,
            last_rocket_shot: 0.,
            rocket_side: RocketSide::Right,
            asteroids,
            asteroid_shapes,

            black_holes: Vec::new(),

            level_up: None,
            level: 1,
            xp: 0,
            next_level_xp: 3,
            hostile_asteroids_per_second: 4. / 60.,
            new_hostile_asteroids: 0.,
            max_hostile_asteroid_speed: 1.,

            available_upgrades: make_upgrades(),

            shields: 0.,
            shield_regeneration_per_sec: 0.,

            rocket_stockpile: 2,
            rocket_production_progress: 0.,
            rocket_production_per_sec: 0.,
            has_brakes: false,

            bullet_reload_time: 0.5,
            rocket_reload_time: 1.,
        }
    }

    fn update(&mut self) -> Option<Box<dyn GameState>> {
        if let Some(level_up) = &mut self.level_up {
            if is_key_pressed(KeyCode::Enter) {
                let upgrade = level_up.upgrade_choices[level_up.selected].clone();
                self.level_up = None;

                if !(upgrade.effect)(self) {
                    self.available_upgrades.retain(|u| !Rc::ptr_eq(u, &upgrade))
                }
            } else {
                if is_key_pressed(KeyCode::Down) {
                    level_up.selected += 1;
                } else if is_key_pressed(KeyCode::Up) {
                    level_up.selected = level_up.upgrade_choices.len() + level_up.selected - 1;
                }
                level_up.selected = level_up.selected % level_up.upgrade_choices.len();
                return None;
            }
        }

        if is_key_pressed(KeyCode::P) {
            self.paused = !self.paused
        }

        if self.paused {
            return None;
        }

        let frame_t: f32 = get_frame_time();
        self.game_t += frame_t;
        let game_t = self.game_t;

        let screen_size = Vec2::new(screen_width(), screen_height());
        let screen_diag_length = screen_size.length();
        let world_diag_length = screen_diag_length * 5.;
        let rotation = self.ship.rot.to_radians();
        // Forward
        let acc = if is_key_down(KeyCode::Up) {
            vec_from_rot(rotation) / 3.
        } else if is_key_down(KeyCode::Down) && self.has_brakes {
            -self.ship.vel / 20. // Break
        } else {
            -self.ship.vel / 1000. // Friction
        };

        // Shot
        if is_key_down(KeyCode::Space) && game_t - self.last_bullet_shot > self.bullet_reload_time {
            let rot_vec = vec_from_rot(rotation);
            self.bullets.push(Bullet {
                pos: self.ship.pos + rot_vec * SHIP_HEIGHT / 2.,
                vel: rot_vec * 10.,
                shot_at: game_t,
                collided: false,
            });
            self.last_bullet_shot = game_t;
        }

        // shoot rocket
        if is_key_down(KeyCode::LeftAlt)
            && game_t - self.last_rocket_shot > self.rocket_reload_time
            && self.rocket_stockpile > 0
        {
            self.rocket_stockpile -= 1;
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
                shot_at: game_t,
                collided: false,
                steer: false,
            });
            self.last_rocket_shot = game_t;
        }

        // produce rockets
        self.rocket_production_progress += self.rocket_production_per_sec * frame_t;
        if self.rocket_production_progress >= 1. {
            let new_rockets = self.rocket_production_progress as usize;
            self.rocket_production_progress -= new_rockets as f32;
            self.rocket_stockpile += new_rockets;
        }

        // regenerate shields
        self.shields += self.shield_regeneration_per_sec * frame_t;

        // Steer
        if is_key_down(KeyCode::Right) {
            self.ship.rot += SHIP_ROTATION_SPEED;
        } else if is_key_down(KeyCode::Left) {
            self.ship.rot -= SHIP_ROTATION_SPEED;
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
            if rocket.shot_at + 0.3 < game_t {
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
        self.bullets.retain(|bullet| bullet.shot_at + 2.5 > game_t);

        let mut new_asteroids = Vec::new();
        let mut colliding = false;
        for asteroid in self.asteroids.iter_mut() {
            // Asteroid/ship collision
            if (asteroid.pos - self.ship.pos).length() < asteroid.size + SHIP_HEIGHT / 3. {
                if !colliding && !self.colliding {
                    if self.shields > 1. {
                        self.shields -= 1.;
                        self.invulnerable_until = game_t + 0.3;
                    }

                    if game_t < self.invulnerable_until {
                        let collision_vec = asteroid.pos - self.ship.pos;
                        self.ship.vel -= 6. * self.ship.vel.project_onto(collision_vec);
                    } else {
                        return Some(Box::new(MenuState::Lost));
                    }
                }
                colliding = true;
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
                self.xp += 1;

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

        self.colliding = colliding;

        let mut new_black_holes = Vec::new();

        for (i, bh1) in self.black_holes.iter().enumerate() {
            for (j, bh2) in self.black_holes.iter().enumerate() {
                if i > j {
                    let dist = bh1.pos().distance(bh2.pos());
                    let dist_vec = bh2.pos() - bh1.pos();
                    let comb_size = bh1.size + bh2.size;
                    bh1.vel
                        .set(bh1.vel() + dist_vec.normalize() * (70. * comb_size / dist.powi(2)));
                    bh2.vel
                        .set(bh2.vel() - dist_vec.normalize() * (70. * comb_size / dist.powi(2)));

                    if bh1.pos().distance(bh2.pos()) < comb_size {
                        bh1.collided.set(true);
                        bh2.collided.set(true);
                        new_black_holes.push(BlackHole {
                            pos: Cell::new(bh1.pos() + (bh2.size / comb_size) * dist_vec),
                            vel: Cell::new(
                                (bh1.size / comb_size) * bh1.vel()
                                    + (bh2.size / comb_size) * bh2.vel(),
                            ),
                            collided: Cell::new(false),
                            size: comb_size.min(400.), // this is so not how physics works
                        })
                    }
                }
            }
        }

        for bh in self.black_holes.iter() {
            bh.pos.set(bh.pos() + bh.vel());

            fn affect_obj(bh: &BlackHole, obj: &mut impl BlackHoleEffected) -> bool {
                let pos = obj.pos();
                let dist = bh.pos().distance(pos);
                *obj.vel() += (bh.pos() - pos).normalize() * (70. * bh.size / dist.powi(2));

                let collided = dist < bh.size + obj.radius();

                if collided {
                    obj.collide();
                }

                collided
            }

            fn affect_objs(bh: &BlackHole, objs: &mut Vec<impl BlackHoleEffected>) {
                for obj in objs.iter_mut() {
                    affect_obj(bh, obj);
                }
            }

            affect_objs(bh, &mut self.bullets);
            affect_objs(bh, &mut self.rockets);
            affect_objs(bh, &mut self.asteroids);
            if affect_obj(bh, &mut self.ship) {
                return Some(Box::new(MenuState::Lost));
            }
        }

        // generate new asteroids
        if self.last_asteroid_generate_pos.distance(self.ship.pos) > 50. {
            let gen_vec = self.ship.pos - self.last_asteroid_generate_pos;
            let asteroid_per_pixel = ASTEROID_DENSITY as f32 / (screen_height() * screen_width());
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

        // generate hostile asteroids
        self.new_hostile_asteroids += self.hostile_asteroids_per_second * frame_t;

        while self.new_hostile_asteroids >= 1. {
            self.new_hostile_asteroids -= 1.;

            let pos = self.ship.pos
                + Vec2::from_angle(rand::gen_range(0.0_f32, 360.).to_radians())
                    * rand::gen_range(screen_diag_length, screen_diag_length * 2.);
            let mut asteroid = Asteroid::new(pos, &self.asteroid_shapes);
            asteroid.vel = (self.ship.pos - pos).normalize()
                * rand::gen_range(1., self.max_hostile_asteroid_speed);
            new_asteroids.push(asteroid);
        }

        // Remove the collided objects
        self.bullets
            .retain(|bullet| bullet.shot_at + BULLET_LIFETIME > game_t && !bullet.collided);
        self.rockets
            .retain(|rocket| rocket.shot_at + ROCKET_LIFETIME > game_t && !rocket.collided);
        self.asteroids.retain(|asteroid| {
            !asteroid.collided && self.ship.pos.distance(asteroid.pos) < world_diag_length / 2.
        });
        self.asteroids.append(&mut new_asteroids);
        self.black_holes.retain(|bh| {
            !bh.collided.get() && self.ship.pos.distance(bh.pos()) < world_diag_length / 2.
        });
        self.black_holes.append(&mut new_black_holes);

        while self.black_holes.len() < (self.level + 5) / 10 {
            // self.level / 10 {
            let pos = self.ship.pos
                + Vec2::from_angle(rand::gen_range(0.0_f32, 360.).to_radians())
                    * rand::gen_range(screen_diag_length * 0.4, screen_diag_length * 2.);
            let rand_vec = Vec2::new(
                rand::gen_range(-0.5, 0.5) * screen_width(),
                rand::gen_range(-0.5, 0.5) * screen_height(),
            );
            let bh = BlackHole {
                pos: Cell::new(pos),
                vel: Cell::new(
                    ((self.ship.pos + rand_vec) - pos).normalize() * rand::gen_range(1., 3.),
                ),
                size: rand::gen_range(5., 20.),
                collided: Cell::new(false),
            };
            self.black_holes.push(bh);
        }

        // update level
        while self.xp >= self.next_level_xp {
            self.level += 1;
            self.xp -= self.next_level_xp;
            self.next_level_xp =
                ((self.next_level_xp as f32 * 1.1) as usize).max(self.next_level_xp + 1);

            self.hostile_asteroids_per_second *= 1.2;
            self.max_hostile_asteroid_speed *= 1.08;

            self.level_up = Some(LevelUp::new(3, self.available_upgrades.clone()))
        }

        // You win?
        /*if self.asteroids.len() == 0 {
            return Some(Box::new(MenuState::Won));
        }*/

        None
    }

    fn render(&self) {
        let screen_size = Vec2::new(screen_width(), screen_height());
        let screen_diag_length = screen_size.length();
        let rotation = self.ship.rot.to_radians();

        fn make_camera(pos: Vec2) -> Camera2D {
            let cam_pos = pos - Vec2::new(screen_width(), -screen_height()) / 2.;
            let rect = Rect::new(cam_pos.x, cam_pos.y, screen_width(), -screen_height());
            Camera2D::from_display_rect(rect)
        }

        clear_background(LIGHTGRAY);

        let in_screen = |pos: Vec2, size: f32| {
            pos.distance(self.ship.pos) < screen_diag_length / 2. + SHIP_HEIGHT + size
        };

        // render stars
        let render_stars = |pos: Vec2, step: i64| {
            set_camera(&make_camera(pos));
            let start: Vec2 = pos - 0.6 * screen_size;
            let end = pos + 0.6 * screen_size;
            let c = |n: f32| -> i64 {
                let n = n as i64;
                n - n % step
            };
            for x in (c(start.x)..c(end.x)).step_by(step as usize) {
                for y in (c(start.y)..c(end.y)).step_by(step as usize) {
                    let mut hasher = DefaultHasher::new();
                    (x, y).hash(&mut hasher);
                    let result = hasher.finish();

                    let x = x + (result.wrapping_mul(11) % step as u64) as i64 - step / 2;
                    let y = y + (result.wrapping_mul(31) % step as u64) as i64 - step / 2;

                    draw_circle(x as f32, y as f32, 2., GRAY);
                }
            }
        };

        render_stars(Vec2::new(2000., 2000.) + self.ship.pos / 4., 400);
        render_stars(Vec2::new(1000., 1000.) + self.ship.pos / 2., 200);
        render_stars(self.ship.pos, 150);

        set_camera(&make_camera(self.ship.pos));

        for bh in self.black_holes.iter() {
            draw_circle(bh.pos().x, bh.pos().y, bh.size, BLACK);
        }

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
        if self.shields >= 1. {
            let mut shield_color = if self.game_t < self.invulnerable_until {
                RED
            } else {
                DARKBLUE
            };
            shield_color.a = 0.5;
            draw_circle_lines(
                self.ship.pos.x + rand::gen_range(-1., 1.),
                self.ship.pos.y + rand::gen_range(-1., 1.),
                0.9 * SHIP_HEIGHT,
                1.5,
                shield_color,
            );
        }

        set_default_camera();

        draw_text(
            &format!(
                "Fps: {}, Asteroids: {} ({}), Bullets: {}, Rockets: {}",
                get_fps(),
                self.asteroids.len(),
                self.generated_asteroids,
                self.bullets.len(),
                self.rockets.len()
            ),
            30.,
            screen_height() - 30.,
            30.,
            BLACK,
        );

        draw_text(
            &format!(
                "Level {}, XP no next Level: {}",
                self.level,
                self.next_level_xp - self.xp
            ),
            30.,
            30.,
            30.,
            BLACK,
        );

        draw_text(
            &format!(
                "Missiles: {}  Shields: {}",
                self.rocket_stockpile, self.shields as usize
            ),
            30.,
            60.,
            30.,
            BLACK,
        );

        if let Some(level_up) = &self.level_up {
            let uc = level_up.upgrade_choices.len();

            let th = 60.;
            let h = 20. + th + (80 * uc) as f32;
            let w = 600.;

            let x = screen_width() / 2. - w / 2.;
            let y = screen_height() / 2. - h / 2.;

            draw_rectangle(x, y, w, h, GRAY);

            draw_centered_text("Level Up!", screen_width() / 2., y + 20., 60., BLACK);

            for (idx, upgrade) in level_up.upgrade_choices.iter().enumerate() {
                let is_selected = idx == level_up.selected;
                let idx = idx as f32;
                draw_rectangle(x + 20., y + idx * 80. + th + 20., w - 40., 60., BLACK);
                let color = if is_selected { LIGHTGRAY } else { GRAY };
                draw_rectangle(x + 25., y + idx * 80. + th + 25., w - 50., 50., color);

                draw_centered_text(
                    &(upgrade.desc)(self),
                    screen_width() / 2.,
                    y + idx * 80. + th + 45.,
                    50.,
                    BLACK,
                )
            }
        } else if self.paused {
            draw_rectangle(
                screen_width() / 2. - 100.,
                screen_height() / 2. - 30.,
                200.,
                60.,
                LIGHTGRAY,
            );
            draw_centered_text(
                "PAUSE",
                screen_width() / 2.,
                screen_height() / 2.,
                50.,
                BLACK,
            );
        }
    }
}

impl GameState for MainState {
    fn do_frame(&mut self) -> Option<Box<dyn GameState>> {
        let new_state = self.update();

        if new_state.is_none() {
            self.render();
        }

        new_state
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
