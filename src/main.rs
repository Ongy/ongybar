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
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::linked_list::LinkedList;
use std::ops::DerefMut;
use std::os::raw::*;
use std::rc::Rc;

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

struct Separator;

impl<G, C> Renderable<G, C> for Separator
    where G: graphics::Graphics {

    fn get_size(&self, _: &mut C, _: u32) -> f64
    { return 9f64; }


    fn do_render(&self, g: &mut G, height: u32, trans: &graphics::math::Matrix2d, cache: &mut C) {
        graphics::line([0.8f32, 0.8f32, 0.8f32, 1.0f32], 0.5f64,
                       [0f64, 3f64, 0f64, height as f64 - 3f64],
                       trans.trans(0f64, 0f64), g);
    }

}

fn draw_seperator<G>(graphics: &mut G, height: u32,
                     trans: &graphics::math::Matrix2d)
    where G: graphics::Graphics {
    graphics::line([0.8f32, 0.8f32, 0.8f32, 1.0f32], 0.5f64,
                   [0f64, 3f64, 0f64, height as f64 - 3f64],
                   trans.trans(0f64, 0f64), graphics);
}

fn draw_text_list<'a, C, G, I, R>(graphics: &mut G, height: u32, cache: &mut C,
                                  trans: &graphics::math::Matrix2d, strs: I) -> f64
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture>,
          R: 'a + Renderable<G, C>,
          I: std::iter::Iterator<Item=&'a Box<R>> {
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

        x.as_ref().do_render(graphics, height, &cur_trans, cache);
        let offset = <R as Renderable<G, C>>::get_size(x.as_ref(), cache, height);
        cur_trans = cur_trans.trans(offset, 0f64);
        total_offset += offset;
    }

    return total_offset;
}

fn draw_window<R>(glyphs: &mut opengl_graphics::glyph_cache::GlyphCache,
                  source: &LinkedList<Box<R>>,
                  graphics : &mut opengl_graphics::GlGraphics,
                  width: u32, height: u32)
    where for<'a> R: Renderable<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache<'a>> {

    println!("Going to draw the window");

    let viewport = graphics::Viewport { rect: [0, 0, width as i32, height as i32],
                                        draw_size: [width, height],
                                        window_size: [width, height] };
    graphics.draw(viewport, |c, g| {
        graphics::clear(graphics::color::BLACK, g);
        draw_text_list(g, height, glyphs, &c.transform, source.iter());
    });
}

fn read_stdin<R>(str_list: &RefCell<LinkedList<Box<R>>>) -> bool
    where for<'a> R: Renderable<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache<'a>> {

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer);
    let tmp = Box::new(String::from(buffer.trim()));
    str_list.borrow_mut().deref_mut().push_back(tmp); // as Box<R>);

    return true;
}

fn main() {
    let mut glyphs = opengl_graphics::glyph_cache::GlyphCache::new("/usr/share/fonts/TTF/DejaVuSans.ttf").unwrap();
    let mut list :LinkedList<Box<_>> = LinkedList::new();

    list.push_back(Box::new(String::from("This is date")) as Box<_>);
    list.push_back(Box::new(String::from("This is ram usage")) as Box<_>);

    let list_cell = Rc::new(RefCell::new(list));

    {
        let mut fun_list = LinkedList::new();
        let cell_cpy = list_cell.clone();
        fun_list.push_back((0 as c_int, Box::new(move || read_stdin(&cell_cpy)) as Box<FnMut() -> bool>));

        xorg::do_x11main(|g, w, h| {
                             let mut l = list_cell.borrow_mut();
                             draw_window(&mut glyphs, l.deref_mut(), g, w, h); },
                         fun_list.into_iter());
    }
}
