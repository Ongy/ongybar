extern crate x11;
extern crate xcb;
extern crate byteorder;
extern crate hostname;

extern crate mio;

extern crate gl;
extern crate libc;
extern crate graphics;
extern crate opengl_graphics;

mod xorg;

use graphics::Transformed;
use graphics::character::CharacterCache;


fn draw_text(graphics: &mut opengl_graphics::GlGraphics, height: u32,
             cache: &mut opengl_graphics::glyph_cache::GlyphCache,
             trans: &graphics::math::Matrix2d, msg: &str) -> f64 {
    let text_height = (height - 2) / 3 * 2;
    graphics::text([0.8f32, 0.8f32, 0.8f32, 1.0f32], text_height,
                   msg, cache, trans.trans(0f64, text_height as f64 + 2f64),
                   graphics);

    return cache.width(text_height, msg);
}

fn draw_text_right(graphics: &mut opengl_graphics::GlGraphics, height: u32,
                   cache: &mut opengl_graphics::glyph_cache::GlyphCache,
                   trans: &graphics::math::Matrix2d, msg: &str) -> f64 {
    let text_height = (height - 2) / 3 * 2;
    let text_width = cache.width(text_height, msg);

    return draw_text(graphics, height, cache, &trans.trans(-text_width, 0f64), msg);
}

fn draw_seperator_right(graphics: &mut opengl_graphics::GlGraphics, height: u32,
                        trans: graphics::math::Matrix2d) {
    graphics::line([0.8f32, 0.8f32, 0.8f32, 1.0f32], 0.2f64,
                   [0f64, 3f64, 0f64, height as f64 - 3f64],
                   trans, graphics);
}

fn draw_window(mut glyphs: &mut opengl_graphics::glyph_cache::GlyphCache,
               graphics : &mut opengl_graphics::GlGraphics,
               width: u32, height: u32) {

    let viewport = graphics::Viewport { rect: [0, 0, width as i32, height as i32],
                                        draw_size: [width, height],
                                        window_size: [width, height] };
    graphics.draw(viewport, |c, g| {
        graphics::clear(graphics::color::BLACK, g);
        let border = width as f64 - 5f64;
        let nd = border - draw_text_right(g, height, &mut glyphs, &c.transform.trans(border, 0f64), "26.06 10:56:30");
        draw_seperator_right(g, height, c.transform.trans(nd - 4f64, 0f64));
        draw_text_right(g, height, &mut glyphs, &c.transform.trans(nd - 9f64, 0f64), "52%  1:14");
    });
}

fn main() {
    let mut glyphs = opengl_graphics::glyph_cache::GlyphCache::new("/usr/share/fonts/TTF/DejaVuSans.ttf").unwrap();
    xorg::do_x11main(|g, w, h| draw_window(&mut glyphs, g, w, h));
}
