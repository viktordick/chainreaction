use std::str;
use std::time::Duration;

use sdl2::EventPump;
use sdl2::VideoSubsystem;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{Window,WindowContext};
use sdl2::render::{Canvas,Texture,TextureCreator};
use sdl2::surface::Surface;
use sdl2::rect::Rect;
use sdl2::pixels::{Color,PixelFormatEnum};
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::ttf;

use crate::grid::{Point, PointIter, DIRECTIONS};
use crate::game::Game;

// Create a canvas, allow the given CanvasDrawer function to fill it, and convert to a texture.
pub fn create_texture<'a, CanvasDrawer>(
    creator: &'a TextureCreator<WindowContext>,
    width: u32,
    height: u32,
    draw: CanvasDrawer
) -> Result<Texture, String>
where CanvasDrawer: Fn(&mut Canvas<Surface>) -> Result<(), String>
{
    let mut canvas = Surface::new(width, height, PixelFormatEnum::RGBA8888)
        ?.into_canvas()?;
    draw(&mut canvas)?;
    Ok(creator
        .create_texture_from_surface(canvas.into_surface())
        .map_err(|e| e.to_string())?)
}

pub fn gradient(canvas: &Canvas<Surface>, cx: i16, cy: i16, color: Color) -> Result<(), String> {
    for i in 0..31 {
        let mut color = color;
        color.a = (256 - (((31-i) as u32 * 140)/32) as u16) as u8;
        let halflength = ((15*15-(i-15)*(i-15)) as f64).sqrt() as i16;
        canvas.hline(cx-halflength, cx+halflength, cy-15+i, Color::RGB(200, 200, 200))?;
        canvas.hline(cx-halflength, cx+halflength, cy-15+i, color)?;
    }
    Ok(())
}

// Rendering helper. This pre-renders all required textures and copies them to the board
// accordingly.
pub struct Renderer<'a> {
    dim: Point,
    background: Texture<'a>,
    marbles: Vec<Texture<'a>>,
    active_marker: Texture<'a>,
    dead_marker: Texture<'a>,
    selected: Texture<'a>,
}
impl<'a> Renderer<'a> {

    fn add_coords(background: &mut Canvas<Surface>, dim: Point) -> Result<(), String> {
        let fontcontext = ttf::init().map_err(|e| e.to_string())?;
        let font = fontcontext.load_font("/usr/share/fonts/liberation/LiberationMono-Regular.ttf", 24)?;
        let creator = background.texture_creator();
        let mut render = |character: u8, posx: i32, posy: i32| -> Result<(), String> {
            let bytes: [u8; 1] = [character];
            let s = str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            let rendered = font.render(&s).blended(Color::RGB(0,0,0))
                .map_err(|e| e.to_string())?;
            let texture = rendered.as_texture(&creator)
                .map_err(|e| e.to_string())?;
            background.copy(
                &texture,
                None,
                Some(
                    Rect::new(
                        posx - rendered.width() as i32/2,
                        posy - rendered.height() as i32/2,
                        rendered.width(),
                        rendered.height()
                    )
                )
            )?;
            Ok(())
        };
        for i in 0..dim.re {
            render(65+i as u8, 100*i as i32 + 50, 20)?;
        };
        for i in 0..dim.im{
            render(49+i as u8, 15, 100*i as i32+50)?;
        }
        Ok(())
    }

    pub fn new(creator: &'a TextureCreator<WindowContext>, game: &Game)
        -> Result<Renderer<'a>, String>
    {
        let black = Color::RGB(0, 0, 0);

        // Marbles
        let mut marbles = Vec::with_capacity(game.num_players());
        for player in game.players() {
            marbles.push(
                create_texture(creator, 31, 31, |canvas| {
                    gradient(&canvas, 15, 15, player.color())?;
                    Ok(())
                })?
            );
        }

        let dim = game.dim();

        Ok(Renderer{
            dim: dim,
            background: create_texture(
                creator, 100*dim.re as u32 + 100, 100*dim.im as u32,
                |mut canvas| {
                    canvas.set_draw_color(Color::RGB(200, 200, 200));
                    canvas.clear();
                    Renderer::add_coords(&mut canvas, dim)?;
                    for x in 0..=dim.re {
                        canvas.vline((x*100) as i16, 0, 100*dim.im as i16, black)?;
                    }
                    for y in 0..dim.im {
                        canvas.hline(0, (100*dim.re) as i16, (y*100) as i16, black)?;
                    }
                    for coord in PointIter::new(dim) {
                        let cell = game.grid().cell(coord);
                        let center = coord*100 + Point::new(50, 50);
                        for direction in 0..4 {
                            if !cell.has_neighbor(direction) {
                                continue
                            }
                            let pos = center + 25*DIRECTIONS[direction];
                            let cx = pos.re as i16;
                            let cy = pos.im as i16;
                            gradient(&canvas, cx, cy, Color::RGB(255, 255, 255))?;
                        }
                    }

                    for (idx, player) in game.players().enumerate() {
                        let x = (dim.re * 100 + 50) as i16;
                        let y = (30 + idx * 40) as i16;
                        gradient(&canvas, x, y, player.color())?;
                    }
                    Ok(())
                },
            )?,
            marbles: marbles,
            active_marker: create_texture(
                creator, 31, 31, |canvas| {
                    canvas.filled_pie(25, 15, 20, 160, 200, black)?;
                    Ok(())
                },
            )?,
            dead_marker: create_texture(
                creator, 31, 31, |canvas| {
                    canvas.thick_line(0, 0, 30, 30, 3, black)?;
                    canvas.thick_line(0, 30, 30, 0, 3, black)?;
                    Ok(())
                },
            )?,
            selected: create_texture(
                creator, 100, 100, |canvas| {
                    canvas.thick_line(1, 1, 100, 1, 2, black)?;
                    canvas.thick_line(1, 1, 1, 100, 2, black)?;
                    canvas.thick_line(100, 1, 100, 100, 2, black)?;
                    canvas.thick_line(1, 100, 100, 100, 2, black)?;
                    Ok(())
                },
            )?,
        })
    }

    pub fn update(&self, canvas: &mut Canvas<Window>, game: &Game) -> Result<(), String>{
        let grid = game.grid();
        canvas.copy(&self.background, None, None)?;
        for marble in grid.marbles() {
            let rect = Rect::new(marble.get_pos().re-15, marble.get_pos().im-15, 31, 31);
            canvas.copy(
                &self.marbles[marble.get_owner()],
                None,
                Some(rect),
            )?
        }
        let rect = Rect::new(self.dim.re as i32*100 + 5, game.cur_player() as i32*40 + 15, 30, 31);
        canvas.copy(
            &self.active_marker,
            None,
            Some(rect),
        )?;
        for (idx, player) in game.players().enumerate() {
            if player.alive {
                continue
            }
            let rect = Rect::new(self.dim.re as i32*100+35, 15+idx as i32*40, 31, 31);
            canvas.copy(
                &self.dead_marker,
                None,
                Some(rect),
            )?;
        }
        let x = game.selected().re as i32;
        let y = game.selected().im as i32;
        canvas.copy(
            &self.selected,
            None,
            Some(Rect::new(x*100, y*100, 100, 100)),
        )?;

        Ok(())
    }
}

pub fn run_game(video: &VideoSubsystem, event_pump: &mut EventPump, game: &mut Game) -> Result<(), String> {
    let dim = game.dim();
    let mut canvas = video
        .window("Chain reaction", 100*dim.re as u32 + 100, 100*dim.im as u32)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?
        .into_canvas()
        .present_vsync()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let renderer = Renderer::new(&texture_creator, &game)?;

    'running: loop {
        canvas.set_draw_color(Color::RGB(90, 90, 90));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode, .. } => game.keydown(keycode.unwrap()),
                Event::MouseButtonDown {x, y, .. } => {
                    let x = x/100;
                    let y = y/100;
                    if x < dim.re && y < dim.im {
                        game.click(Point::new(x, y));
                    }
                },
                _ => {}
            }
        }
        game.step();
        renderer.update(&mut canvas, &game)?;
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
