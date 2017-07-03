extern crate graphics;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;
use graphics::Transformed;

pub struct Separator;

impl<G, C> Renderable<G, C> for Separator
    where G: graphics::Graphics {

    fn get_size(&self, _: &mut C, _: u32, _: &mut OngybarState) -> f64
    { return 1f64; }

    fn do_render(&self, g: &mut G, height: u32, _: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, _: &mut C,
                 c: graphics::types::Color) -> f64 {
        graphics::line(c, 0.5f64, [0f64, 3f64, 0f64, height as f64 - 3f64],
                       trans.trans(0f64, 0f64), g);

        return 1f64;
    }
}
