extern crate graphics;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;

pub struct OngyPos(pub f64);

impl<G, C> Renderable<G, C> for OngyPos {
    fn get_size(&self, _: &mut C, _: u32, _: &mut OngybarState) -> f64
    { return self.0; }

    fn do_render(&self, _: &mut G, _: u32, _: &mut OngybarState,
                 _: &graphics::math::Matrix2d, _: &mut C,
                 _: graphics::types::Color) -> f64 {
        return self.0;
    }
}

