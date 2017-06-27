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

use std::collections::linked_list::LinkedList;
use graphics::Transformed;
use graphics::character::CharacterCache;

trait Renderable<G, C> {
    fn get_size(&self, cache: &mut C, height: u32) -> f64;
    fn do_render(&self, g: &mut G, height: u32, trans: &graphics::math::Matrix2d, cache: &mut C);
}

impl<G, C> Renderable<G, C> for String
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> {

    fn get_size(&self, cache: &mut C, height: u32) -> f64 {
        let text_height = (height - 2) / 3 * 2;
        cache.width(text_height, self.as_str())
    }

    fn do_render(&self, g: &mut G, height: u32, trans: &graphics::math::Matrix2d, cache: &mut C) {
        let text_height = (height - 2) / 3 * 2;
        graphics::text([0.8f32, 0.8f32, 0.8f32, 1.0f32], text_height,
                       self.as_str(), cache, trans.trans(0f64, text_height as f64 + 2f64),
                       g);
    }
}


//fn draw_text(graphics: &mut opengl_graphics::GlGraphics, height: u32,
//             cache: &mut opengl_graphics::glyph_cache::GlyphCache,
//             trans: &graphics::math::Matrix2d, msg: &str) -> f64 {
//    let text_height = (height - 2) / 3 * 2;
//    graphics::text([0.8f32, 0.8f32, 0.8f32, 1.0f32], text_height,
//                   msg, cache, trans.trans(0f64, text_height as f64 + 2f64),
//                   graphics);
//
//    return cache.width(text_height, msg);
//}

//fn draw_text_right(graphics: &mut opengl_graphics::GlGraphics, height: u32,
//                   cache: &mut opengl_graphics::glyph_cache::GlyphCache,
//                   trans: &graphics::math::Matrix2d, msg: &str) -> f64 {
//    let text_height = (height - 2) / 3 * 2;
//    let text_width = cache.width(text_height, msg);
//
//    return draw_text(graphics, height, cache, &trans.trans(-text_width, 0f64), msg);
//}

//fn draw_seperator_right(graphics: &mut opengl_graphics::GlGraphics, height: u32,
//                        trans: &graphics::math::Matrix2d) {
//    graphics::line([0.8f32, 0.8f32, 0.8f32, 1.0f32], 0.2f64,
//                   [0f64, 3f64, 0f64, height as f64 - 3f64],
//                   trans.trans(-4f64, 0f64), graphics);
//}

fn draw_seperator<G>(graphics: &mut G, height: u32,
                     trans: &graphics::math::Matrix2d)
    where G: graphics::Graphics {
    graphics::line([0.8f32, 0.8f32, 0.8f32, 1.0f32], 0.5f64,
                   [0f64, 3f64, 0f64, height as f64 - 3f64],
                   trans.trans(0f64, 0f64), graphics);
}

fn draw_text_list<'a, C, G, I>(graphics: &mut G, height: u32, cache: &mut C,
                               trans: &graphics::math::Matrix2d, strs: I) -> f64
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture>,
          I: std::iter::Iterator<Item=&'a String> {
    let mut total_offset = 0f64;
    let mut cur_trans = trans.trans(0f64, 0f64);
    let mut first = true;
    for x in strs {
        if !first {
            draw_seperator(graphics, height, &cur_trans.trans(5f64, 0f64));
            cur_trans = cur_trans.trans(9f64, 0f64);
        } else {
            first = false;
        }

        x.do_render(graphics, height, &cur_trans, cache); //draw_text(graphics, height, cache, &cur_trans, x.as_str());
        let offset = <String as Renderable<G, C>>::get_size(x, cache, height);
        cur_trans = cur_trans.trans(offset, 0f64);
        total_offset += offset;
    }

    return total_offset;
}

fn draw_window(glyphs: &mut opengl_graphics::glyph_cache::GlyphCache,
               source: &LinkedList<String>,
               graphics : &mut opengl_graphics::GlGraphics,
               width: u32, height: u32) {

    println!("Going to draw the window");

    let viewport = graphics::Viewport { rect: [0, 0, width as i32, height as i32],
                                        draw_size: [width, height],
                                        window_size: [width, height] };
    graphics.draw(viewport, |c, g| {
        graphics::clear(graphics::color::BLACK, g);
        draw_text_list(g, height, glyphs, &c.transform, source.iter());
    });
}

fn main() {
    let mut glyphs = opengl_graphics::glyph_cache::GlyphCache::new("/usr/share/fonts/TTF/DejaVuSans.ttf").unwrap();
    let mut list :LinkedList<String> = LinkedList::new();

    list.push_back(String::from("This is date"));
    list.push_back(String::from("This is ram usage"));

    xorg::do_x11main(|g, w, h| draw_window(&mut glyphs, &list, g, w, h));
}
