mod game;
mod grid;
mod render;
mod menu;

use crate::game::Game;
use crate::render::run_game;
use crate::menu::show_menu;

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let mut event_pump = sdl_context.event_pump()?;
 
    let config = show_menu(&video_subsystem, &mut event_pump)?;
    if config.players.len() == 0 {
        return Ok(());
    }

    let mut game = Game::new(config);
    run_game(&video_subsystem, &mut event_pump, &mut game)?;

    Ok(())
}
