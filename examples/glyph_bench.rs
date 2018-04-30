extern crate ggez;

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::graphics::{self, Point2};
use ggez::timer;
use std::env;
use std::path;

struct MainState {
    text_cached: graphics::TextCached,
    text_raw: graphics::Text,
    frames: usize,
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let font_cached = graphics::Font::new_glyph_font(ctx, "/DejaVuSerif.ttf", 40)?;
        let font_raw = graphics::Font::new(ctx, "/DejaVuSerif.ttf", 40)?;

        let text_cached = graphics::TextCached::new(ctx, "Hello", &font_cached)?;
        let text_raw = graphics::Text::new(ctx, "World!", &font_raw)?;

        Ok(MainState {
            text_cached,
            text_raw,
            frames: 0,
        })
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.frames += 1;
        if self.frames % 10 == 0 {
            println!("FPS: {}", timer::get_fps(ctx));
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);

        // With no drawing at all, runs about 880 fps in debug mode, 12,000 fps release mode
        for y in 0..50 {
            for x in 0..50 {
                let xf = x as f32 * 10.0;   
                let yf = y as f32 * 10.0;
                let dest = Point2::new(xf, yf);
                // ~2 fps debug, 265 fps release
                //graphics::draw(ctx, &self.text_cached, dest, 0.0)?;
            
                // 2 fps debug, 81 fps release
                //graphics::draw(ctx, &self.text_raw, dest, 0.0)?;

                // 30 fps debug, 9200 fps release
                self.text_cached.queue(ctx, dest);
            }
        }
        self.text_cached.draw_queued(ctx, graphics::DrawParam::default())?;
        graphics::present(ctx);

        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: u32, height: u32) {
        graphics::set_screen_coordinates(
            ctx,
            graphics::Rect::new(0.0, 0.0, width as f32, height as f32),
        ).unwrap();
    }
}

pub fn main() {
    let ctx = &mut ContextBuilder::new("text_cached", "ggez")
        .window_setup(
            WindowSetup::default()
                .title("Cached text example!")
                .resizable(true),
        )
        .window_mode(
            WindowMode::default()
                .dimensions(800, 600)
                .vsync(false)
        )
        .build()
        .unwrap();

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        ctx.filesystem.mount(&path, true);
    }

    let state = &mut MainState::new(ctx).unwrap();
    if let Err(e) = event::run(ctx, state) {
        println!("Error encountered: {}", e);
    } else {
        println!("Game exited cleanly.");
    }
}
