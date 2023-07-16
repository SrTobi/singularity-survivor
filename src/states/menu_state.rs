use macroquad::prelude::*;

use crate::GameState;

use super::main_state::MainState;

pub enum MenuState {
    Initial,
    Lost,
    Won,
}

impl GameState for MenuState {
    fn do_frame(&mut self) -> Option<Box<dyn GameState>> {
        clear_background(LIGHTGRAY);
        let font_size = 30.;

        let text = match self {
            MenuState::Initial => "Welcome to Asterodis. Press [enter] to play.",
            MenuState::Lost => "Game Over. Press [enter] to play again.",
            MenuState::Won => "You Win!. Press [enter] to play again.",
        };

        let text_size = measure_text(text, None, font_size as _, 1.0);
        draw_text(
            text,
            screen_width() / 2. - text_size.width / 2.,
            screen_height() / 2. - text_size.height / 2.,
            font_size,
            DARKGRAY,
        );
        if is_key_down(KeyCode::Enter) {
            Some(Box::new(MainState::new()))
        } else {
            None
        }
    }
}
