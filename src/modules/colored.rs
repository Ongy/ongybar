extern crate graphics;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;

pub struct Colored<G, C> {
    pub color: graphics::types::Color,
    pub elem: Box<Renderable<G, C>>,
}

impl<G, C> Renderable<G, C> for Colored<G, C> {
    fn get_size(&self, cache: &mut C, height: u32, o: &mut OngybarState) -> f64 {
        return self.elem.get_size(cache, height, o);
    }

    fn do_render(&self, g: &mut G, height: u32, o: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 _: graphics::types::Color) -> f64 {
        return self.elem.do_render(g, height, o, trans, cache, self.color);
    }
}

