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

use std::iter::IntoIterator;
use graphics::Transformed;
use std::vec::Vec;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::linked_list::LinkedList;
use std::ops::DerefMut;
use std::os::raw::*;
use std::rc::Rc;

trait Renderable<G, C> {
    fn get_size(&self, cache: &mut C, height: u32) -> f64;
    fn do_render(&self, g: &mut G, height: u32, trans: &graphics::math::Matrix2d, cache: &mut C) -> f64;
}

struct OngyStr(String);

impl<G, C> Renderable<G, C> for OngyStr
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> {

    fn get_size(&self, cache: &mut C, height: u32) -> f64 {
        let text_height = (height - 2) / 3 * 2;
        cache.width(text_height, self.0.as_str())
    }

    fn do_render(&self, g: &mut G, height: u32, trans: &graphics::math::Matrix2d, cache: &mut C) -> f64 {
        let text_height = (height - 2) / 3 * 2;
        graphics::text([0.8f32, 0.8f32, 0.8f32, 1.0f32], text_height,
                       self.0.as_str(), cache, trans.trans(0f64, text_height as f64 + 2f64),
                       g);
        return cache.width(text_height, self.0.as_str());
    }
}

struct Separator;

impl<G, C> Renderable<G, C> for Separator
    where G: graphics::Graphics {

    fn get_size(&self, _: &mut C, _: u32) -> f64
    { return 1f64; }

    fn do_render(&self, g: &mut G, height: u32, trans: &graphics::math::Matrix2d, _: &mut C) -> f64 {
        graphics::line([0.8f32, 0.8f32, 0.8f32, 1.0f32], 0.5f64,
                       [0f64, 3f64, 0f64, height as f64 - 3f64],
                       trans.trans(0f64, 0f64), g);

        return 1f64;
    }
}

impl<G, C, I> Renderable<G, C> for I
    where for<'a> &'a I: std::iter::IntoIterator<Item=&'a Box<Renderable<G, C>>> {
    fn get_size(&self, c: &mut C, h: u32) -> f64 {
        let mut first = true;
        let mut ret = 0f64;

        for ref x in self.into_iter() {
            ret += x.get_size(c, h);

            if first {
                first = false;
            } else {
                ret += 2f64;
            }
        }

        return ret;
    }

    fn do_render(&self, g: &mut G, h: u32, trans: &graphics::math::Matrix2d, c: &mut C) -> f64 {
        let mut total_offset = 0f64;
        let mut cur_trans = trans.trans(0f64, 0f64);

        for ref x in self.into_iter() {
            let offset = x.do_render(g, h, &cur_trans, c);
            cur_trans = cur_trans.trans(offset + 4f64, 0f64);
            total_offset += offset + 4f64;
        }

        return total_offset;
    }
}

fn draw_window<'a>(glyphs: &mut opengl_graphics::glyph_cache::GlyphCache<'a>,
                  source: &LinkedList<Box<Renderable<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache<'a>>>>,
                  graphics : &mut opengl_graphics::GlGraphics,
                  width: u32, height: u32) {

    println!("Going to draw the window");

    let viewport = graphics::Viewport { rect: [0, 0, width as i32, height as i32],
                                        draw_size: [width, height],
                                        window_size: [width, height] };
    graphics.draw(viewport, |c, g| {
        graphics::clear(graphics::color::BLACK, g);
        source.do_render(g, height, &c.transform, glyphs);
    });
}

fn read_stdin<C, G>(str_list: &Rc<RefCell<LinkedList<Box<Renderable<G, C>>>>>) -> bool
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture>, {

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer);
    let tmp = Box::new(OngyStr(String::from(buffer.trim())));
    let mut list = str_list.borrow_mut();

    if !list.is_empty() {
        list.deref_mut().push_back(Box::new(Separator))
    }
    list.deref_mut().push_back(tmp);

    return true;
}

fn main() {
    let mut glyphs = opengl_graphics::glyph_cache::GlyphCache::new("/usr/share/fonts/TTF/DejaVuSans.ttf").unwrap();
    let list: LinkedList<Box<Renderable<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache>>> = LinkedList::new();

    let list_cell = Rc::new(RefCell::new(list));

    {
        let mut fun_list = LinkedList::new();
        let cell_cpy: Rc<RefCell<LinkedList<Box<_>>>> = list_cell.clone();
        fun_list.push_back((0 as c_int, Box::new(move || read_stdin(&cell_cpy)) as Box<FnMut() -> bool>));

        xorg::do_x11main(|g, w, h| {
                             let mut l = list_cell.borrow_mut();
                             draw_window(&mut glyphs, l.deref_mut(), g, w, h); },
                         || opengl_graphics::GlGraphics::new(opengl_graphics::OpenGL::V3_0),
                         fun_list.into_iter());
    }
}
