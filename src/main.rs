#[macro_use]
extern crate rs_config_derive;
extern crate rs_config;

extern crate graphics;
extern crate opengl_graphics;

extern crate xdg;

mod xorg;
mod config;
mod modules;
mod parsers;

use modules::renderable::{Renderable, OngybarState};
use modules::separator::Separator;
use modules::ongystr::OngyStr;

use parsers::dzen::dzen_parse;
use parsers::custom::custom_parse;

use graphics::Transformed;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::linked_list::LinkedList;
use std::io::{BufRead, BufReader};
use std::ops::DerefMut;
use std::os::raw::*;
use std::rc::Rc;

//use std::process::Command;
//use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::ops::Deref;

struct Output<G, C> {
    name: String,
    content: Rc<RefCell<LinkedList<Box<Renderable<G, C>>>>>,
    position: config::Anchor,
    layer: i32,
}

struct Window<G, C> {
    outputs: Vec<Output<G, C>>,
}

fn render_middle<G, C, R>(g: &mut G, obj: &R, o: &mut OngybarState, c : &mut C,
                          trans: &graphics::math::Matrix2d, height: u32)
    where R: Renderable<G, C> {
    let size = obj.get_size(c, height, o);

    obj.do_render(g, height, o, &trans.trans(-size / 2.0, 0f64), c, [0.8, 0.8, 0.8, 1.0]);
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
        for ref output in &win.outputs {
            let cell = output.content.borrow();
            let list = cell.deref();
            match &output.position {
                &config::Anchor::Static(ref x) => {
                    match x {
                        &config::StaticPosition::Left => {
                            list.do_render(g, height, o, &c.transform, glyphs, [0.8, 0.8, 0.8, 1.0]);
                        },
                        &config::StaticPosition::Right => {
                            render_right(g, list, o, glyphs, &c.transform.trans(width as f64, 0f64), height);
                        },
                        &config::StaticPosition::Middle => {
                            render_middle(g, list, o, glyphs, &c.transform.trans(width as f64 / 2.0, 0f64), height);
                        },
                        x => {
                            panic!("Sorry, can't layout {:?} yet :(", x);
                        },
                    }
                },
                x => {
                    panic!("Sorry, can't layout {:?} yet :(", x);
                },
            }
        }
    });
}

fn make_update_action<G, C>(source: &config::InputSource,
                            parser: &config::Parser,
                            list: Rc<RefCell<LinkedList<Box<Renderable<G, C>>>>>)
                            -> (c_int, Box<FnMut() -> bool>)
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture>  + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static, {

    let fd = match source {
        /* STDIN is on FD 0, by definition */
        &config::InputSource::Stdin => 0,
        /* PIPE tells us to read from a fd passed by someone else */
        &config::InputSource::Pipe(fd) => fd,
        x => {
            panic!("Sorry I didn't implement getting output for {:?} yet :(", x);
        },

    };

    // TODO: Combine the paths!
    let fun = match parser {
        &config::Parser::Plain => {
            let mut reader = BufReader::new(unsafe {std::fs::File::from_raw_fd(fd)} );

            let fun = move || {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let mut mut_list = list.borrow_mut();
                mut_list.deref_mut().clear();
                mut_list.deref_mut().push_front(Box::new(OngyStr(line.trim().into())) as Box<Renderable<G, C>>);

                return true;
            };
            Box::new(fun) as Box<FnMut() -> bool>
        },
        &config::Parser::Ongybar => {
            let mut reader = BufReader::new(unsafe {std::fs::File::from_raw_fd(fd)} );
            let mut first = true;

            let fun = move || {
                let mut mut_list = list.borrow_mut();
                mut_list.deref_mut().clear();

                let new_list = custom_parse(&mut reader);

                mut_list.clear();
                for b in new_list {
                    if first {
                        first = false;
                    } else {
                        mut_list.push_back(Box::new(Separator));
                    }

                    mut_list.push_back(b);
                }

                return true;
            };

            Box::new(fun) as Box<FnMut() -> bool>
        },
        &config::Parser::Dzen => {
            let mut reader = BufReader::new(unsafe {std::fs::File::from_raw_fd(fd)} );
            let mut first = true;

            let fun = move || {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let mut mut_list = list.borrow_mut();
                mut_list.deref_mut().clear();

                let new_list = dzen_parse(line.trim());

                mut_list.clear();
                for b in new_list {
                    if first {
                        first = false;
                    } else {
                        mut_list.push_back(Box::new(Separator));
                    }

                    mut_list.push_back(b);
                }

                return true;
            };

            Box::new(fun) as Box<FnMut() -> bool>
        },
    };

    return (fd, fun);
}

fn make_outputs<G, C>(conf: &config::Config) -> (Vec<(c_int, Box<FnMut() -> bool>)>, Vec<Output<G, C>>)
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture>  + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static, {

    let mut outs = Vec::with_capacity(conf.inputs.len());
    let mut updates = Vec::with_capacity(conf.inputs.len());

    for ref input in &conf.inputs {
        let out = Output {
            name: input.name.clone(),
            position: input.position.clone(),
            layer: input.layer,
            content: Rc::new(RefCell::new(LinkedList::new())),
        };
        let update = make_update_action(&input.source, &input.parser, out.content.clone());

        outs.push(out);
        updates.push(update);
    }

    /* This should be a noop, but eh */
    outs.shrink_to_fit();
    updates.shrink_to_fit();
    return (updates, outs);
}

fn parse_or_default_config() -> config::Config {
    let xdg_base = match xdg::BaseDirectories::with_prefix("ongybar") {
        Ok(x) => x,
        Err(x) => {
            println!("Couldn't get XDG config to search for config: {}", x);
            return config::get_default();
        },
    };

    match xdg_base.find_config_file("config") {
        Some(x) => {
            if let Some(y) = x.to_str() {
                println!("Using config: {}", y);
            } else {
                println!("Couldn't decode path into string. This sounds fancy!");
            }

            return rs_config::read_or_exit(x);
        },
        None => {
            return config::get_default();
        },
    }
}

fn main() {
    let config = parse_or_default_config();

    let (updates, outputs) = make_outputs::<opengl_graphics::GlGraphics, opengl_graphics::glyph_cache::GlyphCache>(&config);

    let mut glyphs = opengl_graphics::glyph_cache::GlyphCache::new("/usr/share/fonts/TTF/DejaVuSans.ttf").unwrap();
    let win = Window { outputs: outputs };
    let mut state = OngybarState::new();

    {
        xorg::do_x11main(|g, w, h| {
                             draw_window(&mut glyphs, &mut state, &win, g, w, h); },
                         || opengl_graphics::GlGraphics::new(opengl_graphics::OpenGL::V3_0),
                         updates.into_iter());
    }
}
