use macroquad::{
    prelude::Color,
    text::{draw_text, get_text_center},
};

pub fn draw_centered_text(text: &str, x: f32, y: f32, font_size: f32, color: Color) {
    let center = get_text_center(text, None, font_size as u16, 1.0, 0.);
    draw_text(text, x - center.x, y - center.y, font_size, color)
}
