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
mod modules;
mod parsers;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;
use modules::separator::Separator;

use parsers::dzen::dzen_parse;

use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::io::BufRead;
use std::io::BufReader;
use graphics::Transformed;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::linked_list::LinkedList;
use std::ops::DerefMut;
use std::os::raw::*;
use std::rc::Rc;

struct Window<G, C> {
    right_list: LinkedList<Box<Renderable<G, C>>>,
    left_list:  LinkedList<Box<Renderable<G, C>>>,
}

fn render_right<G, C, R>(g: &mut G, obj: &R, o: &mut OngybarState, c : &mut C,
                         trans: &graphics::math::Matrix2d, height: u32)
    where R: Renderable<G, C> {
    let size = obj.get_size(c, height, o);
    obj.do_render(g, height, o, &trans.trans(-size, 0f64), c, [0.8, 0.8, 0.8, 1.0]);
}

fn draw_window<'a>(glyphs: &mut opengl_graphics::glyph_cache::GlyphCache<'a>, o: &mut OngybarState,
                   win: &Window<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache<'a>>,
                   graphics : &mut opengl_graphics::GlGraphics,
                   width: u32, height: u32) {
    let viewport = graphics::Viewport { rect: [0, 0, width as i32, height as i32],
                                        draw_size: [width, height],
                                        window_size: [width, height] };
    graphics.draw(viewport, |c, g| {
        graphics::clear(graphics::color::BLACK, g);
        win.left_list.do_render(g, height, o, &c.transform, glyphs, [0.8, 0.8, 0.8, 1.0]);

        render_right(g, &win.right_list, o, glyphs, &c.transform.trans(width as f64, 0f64), height);
    });
}

fn read_pipe<G, C, R>(reader: &mut R, str_list: &mut LinkedList<Box<Renderable<G, C>>>) -> bool
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture>  + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          R: BufRead {
    let mut first = true;
    let mut buffer = String::new();
    let _ = reader.read_line(&mut buffer);


    let new_list = buffer.trim().split("|").map(|x| dzen_parse(x));
    //let new_list = buffer.trim().split("|").map(|x| Box::new(OngyStr(String::from(x))));

    let mut list = str_list;

    list.clear();
    for b in new_list {
        if first {
            first = false;
        } else {
            list.push_back(Box::new(Separator));
        }

        list.push_back(Box::new(b));
    }

    return true;
}



fn main() {
    let mut glyphs = opengl_graphics::glyph_cache::GlyphCache::new("/usr/share/fonts/TTF/DejaVuSans.ttf").unwrap();
    let list = Window { left_list: LinkedList::new(), right_list: LinkedList::new() };
    let mut state = OngybarState::new();

    let list_cell = Rc::new(RefCell::new(list));

    {
        let mut fun_list = LinkedList::new();

        let std_cpy = list_cell.clone();
        let mut std_reader = BufReader::new(std::io::stdin());
        fun_list.push_back((0 as c_int, Box::new(move || read_pipe(&mut std_reader, &mut std_cpy.borrow_mut().left_list)) as Box<FnMut() -> bool>));

        let child = Command::new("monky")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped()).spawn().unwrap();
        let stdout = child.stdout.unwrap();
        let fd = stdout.as_raw_fd();
        let mut pipe_reader = BufReader::new(stdout);
        let pipe_cpy = list_cell.clone();
        fun_list.push_back((fd, Box::new(move || read_pipe(&mut pipe_reader, &mut pipe_cpy.borrow_mut().right_list)) as Box<FnMut() -> bool>));

        xorg::do_x11main(|g, w, h| {
                             let mut l = list_cell.borrow_mut();
                             draw_window(&mut glyphs, &mut state, l.deref_mut(), g, w, h); },
                         || opengl_graphics::GlGraphics::new(opengl_graphics::OpenGL::V3_0),
                         fun_list.into_iter());
    }
}
