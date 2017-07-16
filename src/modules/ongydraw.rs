extern crate graphics;

use std;
use modules::renderable::Renderable;
use modules::renderable::OngybarState;

#[derive(Clone, Copy)]
pub enum Coordtype {
    Absolute,
    Relative,
    SemiRelative,
}

impl Coordtype {
    fn transform(self, x: f64, y: f64, height: u32) -> (f64, f64) {
        return match self {
            Coordtype::Absolute =>
                (x, y),
            Coordtype::Relative =>
                (x / 100.0 * height as f64, y / 100.0 * height as f64),
            Coordtype::SemiRelative =>
                (x, y * height as f64 / 100.0),
        }
    }
}

#[derive(Debug)]
pub struct DrawRect {
    x1: f64,
    y1: f64,

    x2: f64,
    y2: f64
}

impl DrawRect {
    fn get_width(&self, height: u32, coords: Coordtype) -> f64{
        let (_, y) = coords.transform(self.x2 - self.x1, self.y2 - self.y1, height);
        return y;
    }

    fn do_render<G> (&self, g: &mut G, height: u32, coords: Coordtype,
                 trans: graphics::math::Matrix2d,
                 c: graphics::types::Color) -> f64
        where  G: graphics::Graphics {
        let (x, y) = coords.transform(self.x1, self.y1, height);
        let (width, height) = coords.transform(self.x2 - self.x1, self.y2 - self.y1, height);

        let dims = [x, y, width, height];
        graphics::rectangle(c, dims, trans, g);

        return width;
    }

    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        DrawRect { x1: x1, y1: y1, x2: x2, y2: y2 }
    }
}

#[derive(Debug)]
pub struct DrawCol {
    v: Box<DrawCommand>,
    c: graphics::types::Color,
}

impl DrawCol {
    fn get_width(&self, height: u32, coords: Coordtype) -> f64{
        return self.v.get_width(height, coords);
    }

    fn do_render<G> (&self, g: &mut G, height: u32, coords: Coordtype,
                 trans: graphics::math::Matrix2d,
                 _: graphics::types::Color) -> f64
        where  G: graphics::Graphics {
        return self.v.do_render(g, height, coords, trans, self.c);
    }

    pub fn new(v: DrawCommand, c: graphics::types::Color) -> Self {
        DrawCol {v: Box::new(v), c: c}
    }
}

#[derive(Debug)]
pub enum DrawCommand {
    Rect(DrawRect),
    Col(DrawCol),
}

impl DrawCommand {
    fn get_width(&self, height: u32, coords: Coordtype) -> f64{
        return match self {
            &DrawCommand::Rect(ref x) => x.get_width(height, coords),
            &DrawCommand::Col(ref x) => x.get_width(height, coords),
        }
    }

    fn do_render<G> (&self, g: &mut G, height: u32, coords: Coordtype,
                 trans: graphics::math::Matrix2d,
                 c: graphics::types::Color) -> f64
        where  G: graphics::Graphics {
        return match self {
            &DrawCommand::Rect(ref x) => x.do_render(g, height, coords, trans, c),
            &DrawCommand::Col(ref x) => x.do_render(g, height, coords, trans, c),
        }
    }
}

pub struct OngyDraw {
    coords: Coordtype,
    values: Vec<DrawCommand>,
}

impl OngyDraw {
    pub fn new<I> (coords: Coordtype, vals: I) -> Self
        where I: std::iter::Iterator<Item=DrawCommand> {
        OngyDraw { coords: coords, values: vals.collect() }
    }
}


impl<G, C> Renderable<G, C> for OngyDraw
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics {

    fn get_size(&self, _: &mut C, h: u32, _: &mut OngybarState) -> f64 {
        let mut sum = 0.0;
        for ref v in &self.values {
            sum += v.get_width(h, self.coords);
        }

        return sum;
    }

    fn do_render(&self, g: &mut G, height: u32, _: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, _: &mut C,
                 c: graphics::types::Color) -> f64 {
        let mut sum = 0.0;
        for ref v in &self.values {
            sum += v.do_render(g, height, self.coords, *trans, c);
        }

        return sum;
    }
}
