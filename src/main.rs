use macroquad::prelude::*;
use states::menu_state::MenuState;

mod states;

pub trait GameState {
    fn do_frame(&mut self) -> Option<Box<dyn GameState>>;
}

struct Game {
    main: Box<dyn GameState>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            main: Box::new(MenuState::Initial),
        }
    }

    pub fn do_frame(&mut self) {
        let new_state = self.main.do_frame();

        if let Some(new_state) = new_state {
            set_default_camera();
            self.main = new_state;
        }
    }
}

#[macroquad::main("Asteroids")]
async fn main() {
    let mut game = Game::new();

    loop {
        game.do_frame();
        next_frame().await
    }
}
