// This is the parser for the format defined by this
//
// TODO: Do this :)

extern crate graphics;
extern crate opengl_graphics;
extern crate num_traits;
extern crate byteorder;

use self::byteorder::ReadBytesExt;

use modules::renderable::Renderable;
use modules::ongystr::OngyStr;
use modules::ongyimage::OngyImage;
use modules::ongyrect::OngyRectR;
use modules::colored::Colored;
use modules::ongydraw::*;

use std;
use std::vec::Vec;

struct CustomIter<'a, G, C, R: 'a> {
    r: &'a mut R,
    num: Option<u8>,
    g: std::marker::PhantomData<G>,
    c: std::marker::PhantomData<C>,
}

struct DrawIter<'a, R: 'a> {
    r: &'a mut R,
    num: u8,
}

impl<'a, R> DrawIter<'a, R> {
    fn new(read: &'a mut R, num: u8) -> Self {
        DrawIter {r: read, num: num}
    }
}

impl<'a, G, C, R> CustomIter<'a, G, C, R> {
    fn new(reader: &'a mut R) -> Self {
        return CustomIter{ r: reader,
                           num: None,
                           g: std::marker::PhantomData,
                           c: std::marker::PhantomData  };
    }
}

fn parse_text<R> (r: &mut R) -> OngyStr
    where R: std::io::Read {

    let len = r.read_u16::<byteorder::NativeEndian>().unwrap();

    let mut read_buf = Vec::new();
    read_buf.resize(len as usize + 1, 0);

    r.read_exact(read_buf.as_mut_slice()).unwrap();
    let _ = read_buf.pop();

    match String::from_utf8(read_buf) {
        Ok(x) => return OngyStr(x),
        Err(x) => {
            println!("Error while decoding string from custom format: {:?}", x);
            return OngyStr(String::from("ERR"));
        }
    }
}

fn parse_image<R> (r: &mut R) -> OngyImage
    where R: std::io::Read {
    let OngyStr(x) = parse_text(r);
    return OngyImage(x);
}

fn parse_colorfrag<R> (r: &mut R) -> graphics::types::Color
    where R: std::io::Read {
    let mut ret = [0;4];
    let _ = r.read_exact(&mut ret);

    /* This is a bit stupid, but eh */
    return [ret[0] as f32 / 255.0,
            ret[1] as f32 / 255.0,
            ret[2] as f32 / 255.0,
            ret[3] as f32 / 255.0];
}

fn parse_color<R, G, C> (r: &mut R) -> Colored<G, C>
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          R: std::io::Read {
    let fg = parse_colorfrag(r);
    /* We don't do anything for background currently, ignore this */
    let _ = parse_colorfrag(r);

    match parse_elem(r) {
        Some(x) => Colored { color: fg, elem: x },
        None => {
            println!("Couldn't decode child element of color in custom format :(");
            return Colored { color: fg, elem: Box::new(OngyStr(String::from("ERR"))) };
        }
    }
}

fn parse_rect<R> (r: &mut R) -> OngyRectR
    where R: std::io::Read {
    let width = r.read_u16::<byteorder::NativeEndian>().unwrap();
    let height = r.read_u16::<byteorder::NativeEndian>().unwrap();

    return OngyRectR { width: width as f64, height: height as f64 }
}

fn parse_draw_elem<R> (r: &mut R) -> DrawCommand
    where R: std::io::Read {

    let mut type_enum = [0xff;1];
    let _ = r.read(&mut type_enum);
    match type_enum[0] {
        1 => { /* 1 is the Rectangle */
            let x1 = r.read_u16::<byteorder::NativeEndian>().unwrap();
            let y1 = r.read_u16::<byteorder::NativeEndian>().unwrap();
            let x2 = r.read_u16::<byteorder::NativeEndian>().unwrap();
            let y2 = r.read_u16::<byteorder::NativeEndian>().unwrap();

            return DrawCommand::Rect(
                    DrawRect::new(
                        x1 as f64,
                        y1 as f64,
                        x2 as f64,
                        y2 as f64));
        }
        3 => { /* 3 is a coloured element */
            let color = parse_colorfrag(r);
            let val = parse_draw_elem(r);
            return DrawCommand::Col(DrawCol::new(val, color));
        }
        x => {
            println!("Found something I can't work with :( {}", x);
            return DrawCommand::Rect(DrawRect::new(0.0, 0.0, 0.0, 0.0));
        }
    }
}

impl<'a, R> Iterator for DrawIter<'a, R>
    where R: std::io::Read {
    type Item=DrawCommand;

    fn next(&mut self) -> Option<Self::Item> {
        if self.num < 1 {
            return None;
        }
        self.num = self.num - 1;

        return Some(parse_draw_elem(self.r));
    }
}

fn parse_draw<R> (r: &mut R) -> OngyDraw
    where R: std::io::Read {
    let mut buffer = [1];
    let _ = r.read(&mut buffer);

    let coords = match buffer[0] {
        0 => Coordtype::Absolute,
        1 => Coordtype::Relative,
        2 => Coordtype::SemiRelative,
        x => {
                println!("Found unused coord type while parsing OngyDraw: {}", x);
                println!("Defaulting to relative");
                Coordtype::Relative
            },
    };

    let _ = r.read(&mut buffer);

    return OngyDraw::new(coords, DrawIter::new(r, buffer[0]));
}

fn parse_elem<G, C, R> (r: &mut R) -> Option<Box<Renderable<G, C>>>
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          R: std::io::Read {
    /* Ok, we will read one element at a time. So we first read in the type enum value */
    let mut type_enum = [0;1];
    let _ = r.read(&mut type_enum);
    match type_enum[0] {
        0 => return Some(Box::new(custom_parse(r))),
        1 => return Some(Box::new(parse_text(r))),
        2 => return Some(Box::new(parse_image(r))),
        3 => return Some(Box::new(parse_color(r))),
        4 => return Some(Box::new(parse_rect(r))),
        5 => return Some(Box::new(parse_draw(r))),
        x => {
            println!("Found a type I couldn't interpret while parsing custom format: {}", x);
            return None
        },
    }
}

impl<'a, G, C, R> Iterator for CustomIter<'a, G, C, R>
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          R: std::io::Read {
    type Item=Box<Renderable<G, C>>;


    fn next(&mut self) -> Option<Self::Item> {
        /* Ok, we are starting up. First read in the number of elements in the list */
        if let None = self.num {
            let mut buffer = [1];
            let _ = self.r.read(&mut buffer);
            self.num = Some(buffer[0]);
        }

        /* We are guaranteed to have *something* in here, so we just unwrap the value */
        let remaining = self.num.unwrap();
        if remaining < 1 {
            /* We read all elements of the list. End now */
            return None;
        } else {
            /* We will read another element from the list, so we decrement by 1 */
            self.num = Some(remaining - 1);
        }

        return parse_elem(self.r);
    }
}

/* This should be called with buffered reader for performance reasons! */
pub fn custom_parse<G, C, R>(arg: &mut R) -> Vec<Box<Renderable<G, C>>>
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture>  + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          R: std::io::Read {
    return CustomIter::new(arg).collect();
}
