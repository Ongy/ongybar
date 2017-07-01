extern crate x11;
extern crate xcb;
extern crate byteorder;
extern crate hostname;

extern crate mio;

extern crate gl;
extern crate libc;
extern crate graphics;
extern crate opengl_graphics;

extern crate num_traits;

mod xorg;

use num_traits::Num;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::io::BufRead;
use std::io::BufReader;
use std::iter::IntoIterator;
use graphics::Transformed;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::linked_list::LinkedList;
use std::ops::DerefMut;
use std::os::raw::*;
use std::rc::Rc;


trait Renderable<G, C> {
    fn get_size(&self, cache: &mut C, height: u32) -> f64;
    fn do_render(&self, g: &mut G, height: u32,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 color: graphics::types::Color) -> f64;
}

struct Window<G, C> {
    right_list: LinkedList<Box<Renderable<G, C>>>,
    left_list:  LinkedList<Box<Renderable<G, C>>>,
}

struct Colored<G, C> {
    color: graphics::types::Color,
    elem: Box<Renderable<G, C>>,
}

impl<G, C> Renderable<G, C> for Colored<G, C> {
    fn get_size(&self, cache: &mut C, height: u32) -> f64 {
        return self.elem.get_size(cache, height);
    }

    fn do_render(&self, g: &mut G, height: u32,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 _: graphics::types::Color) -> f64 {
        return self.elem.do_render(g, height, trans, cache, self.color);
    }
}

struct OngyStr(String);
struct Separator;

impl<G, C> Renderable<G, C> for OngyStr
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> {

    fn get_size(&self, cache: &mut C, height: u32) -> f64 {
        let text_height = (height - 2) * 2 / 3;
        cache.width(text_height, self.0.as_str())
    }

    fn do_render(&self, g: &mut G, height: u32,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 c: graphics::types::Color) -> f64 {
        let text_height = (height - 2) * 2 / 3;
        graphics::text(c, text_height, self.0.as_str(), cache,
                       trans.trans(0f64, text_height as f64 + 2f64), g);
        return cache.width(text_height, self.0.as_str());
    }
}

impl<G, C> Renderable<G, C> for Separator
    where G: graphics::Graphics {

    fn get_size(&self, _: &mut C, _: u32) -> f64
    { return 1f64; }

    fn do_render(&self, g: &mut G, height: u32,
                 trans: &graphics::math::Matrix2d, _: &mut C,
                 c: graphics::types::Color) -> f64 {
        graphics::line(c, 0.5f64, [0f64, 3f64, 0f64, height as f64 - 3f64],
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
            if first {
                first = false;
            } else {
                ret += 4f64;
            }

            ret += x.get_size(c, h);
        }

        return ret;
    }

    fn do_render(&self, g: &mut G, h: u32,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 c: graphics::types::Color) -> f64 {
        let mut total_offset = 0f64;
        let mut cur_trans = trans.trans(0f64, 0f64);

        for ref x in self.into_iter() {
            let offset = x.do_render(g, h, &cur_trans, cache, c);
            cur_trans = cur_trans.trans(offset + 4f64, 0f64);
            total_offset += offset + 4f64;
        }

        return total_offset;
    }
}

fn render_right<G, C, R>(g: &mut G, obj: &R, c : &mut C,
                         trans: &graphics::math::Matrix2d, height: u32)
    where R: Renderable<G, C> {
    let size = obj.get_size(c, height);
    obj.do_render(g, height, &trans.trans(-size, 0f64), c, [0.8, 0.8, 0.8, 1.0]);
}

fn draw_window<'a>(glyphs: &mut opengl_graphics::glyph_cache::GlyphCache<'a>,
                   win: &Window<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache<'a>>,
                   graphics : &mut opengl_graphics::GlGraphics,
                   width: u32, height: u32) {
    let viewport = graphics::Viewport { rect: [0, 0, width as i32, height as i32],
                                        draw_size: [width, height],
                                        window_size: [width, height] };
    graphics.draw(viewport, |c, g| {
        graphics::clear(graphics::color::BLACK, g);
        win.left_list.do_render(g, height, &c.transform, glyphs, [0.8, 0.8, 0.8, 1.0]);

        render_right(g, &win.right_list, glyphs, &c.transform.trans(width as f64, 0f64), height);
    });
}

struct DzenIter<G, C> {
    text: String,
    g: std::marker::PhantomData<G>,
    c: std::marker::PhantomData<C>,
}

impl<G, C> DzenIter<G, C> {
    fn new(text: &str) -> Self {
        return DzenIter{ text: String::from(text),
                         g: std::marker::PhantomData,
                         c: std::marker::PhantomData  };
    }
}

fn find_str(text: &String) -> Option<(OngyStr, String)> {
    match text.find('^') {
        None => {
            return Some((OngyStr(text.clone()), String::new()));
        }
        Some(i) => {
            let left  = &text.as_str()[..i];
            let right = &text.as_str()[i + 1..];

            if right.chars().nth(0) == Some('^') {
                match find_str(&String::from(&right[1..])) {
                    Some((OngyStr(x), ret)) => {
                        let mut tmp = String::from(&text.as_str()[.. i + 1]);
                        tmp.push_str(x.as_str());
                        return Some((OngyStr(tmp), ret));
                    },
                    None => {
                        return Some((OngyStr(String::from(&text.as_str()[.. i + 1])), String::from(right)));
                    }
                }
            }

            if i == 0 {
                return None;
            }

            return Some((OngyStr(String::from(left)), String::from(&text.as_str()[i ..])));
        }
    }

}

fn parse_color(hex_str: &str) -> Option<[f32; 4]> {
    if hex_str.chars().nth(0) != Some('#') {
        std::io::stderr().write(b"Sorry, ongybar currently only supports colors as #Hexstr").unwrap();
        return None;
    }
    // TODO: Detect smaller/longer strs!
    let red   = &hex_str[1..3];
    let green = &hex_str[3..5];
    let blue  = &hex_str[5..7];
    // TODO: Remove the unwrap()...
    return Some([f32::from_str_radix(red, 16).unwrap(),
                 f32::from_str_radix(green, 16).unwrap(),
                 f32::from_str_radix(blue, 16).unwrap(),
                 1.0]);
}

impl<G, C> Iterator for DzenIter<G, C>
    where C: graphics::character::CharacterCache + 'static,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> + 'static, {
    type Item=Box<Renderable<G, C>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.len() == 0 {
            return None;
        }

        match find_str(&self.text) {
            Some ((ret, nxt)) => {
                self.text = nxt;
                return Some(Box::new(ret) as Box<Renderable<G, C>>);
            }
            None => {
                if self.text.starts_with("^fg(") {
                    return match self.text.find(')') {
                        Some(i) => {
                            match parse_color(&self.text.as_str()[4 .. i]) {
                                Some(c) =>  {
                                    let tmp = dzen_parse(&self.text.as_str()[i + 1 ..]);
                                    self.text.clear();
                                    return Some(Box::new(Colored{ color: c, elem: Box::new(tmp)}));
                                }
                                None => None
                            }
                        }
                        None => {
                            std::io::stderr().write(b"Found '^fg(', but couldn't find closing parens!").unwrap();
                            return None;
                        }
                    }
                }
                println!("Currently can't handle \"{}\"", self.text);
                return None;
            }
        }
    }
}

fn dzen_parse<G, C>(arg: &str) -> Vec<Box<Renderable<G, C>>>
    where C: graphics::character::CharacterCache + 'static,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> + 'static, {
    return DzenIter::new(arg).collect();
}

fn read_pipe<G, C, R>(reader: &mut R, str_list: &mut LinkedList<Box<Renderable<G, C>>>) -> bool
    where C: graphics::character::CharacterCache + 'static,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> + 'static,
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
                             draw_window(&mut glyphs, l.deref_mut(), g, w, h); },
                         || opengl_graphics::GlGraphics::new(opengl_graphics::OpenGL::V3_0),
                         fun_list.into_iter());
    }
}
