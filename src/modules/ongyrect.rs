extern crate graphics;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;
use graphics::Transformed;

pub struct OngyRect {
    pub width: f64,
    pub height: f64,
}

impl<G, C> Renderable<G, C> for OngyRect
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> {

    fn get_size(&self, _: &mut C, _: u32, _: &mut OngybarState) -> f64 {
        return self.width;
    }

    fn do_render(&self, g: &mut G, height: u32, _: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, _: &mut C,
                 c: graphics::types::Color) -> f64 {
        // This will be in pixels for now. Percent will probably go into another type
        let dims = [0.0, height as f64 / 2.0 - self.height / 2.0, self.width, height as f64];
        graphics::rectangle(c, dims, trans.trans(0f64, 0f64), g);
        return self.width;
    }
}

pub struct OngyRectR {
    pub width: f64,
    pub height: f64,
}

impl<G, C> Renderable<G, C> for OngyRectR
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> {

    fn get_size(&self, _: &mut C, height: u32, _: &mut OngybarState) -> f64 {
        return self.width / 100.0 * height as f64;
    }

    fn do_render(&self, g: &mut G, height: u32, _: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, _: &mut C,
                 c: graphics::types::Color) -> f64 {
        // This will be in pixels for now. Percent will probably go into another type
        let real_height = self.height / 100.0 * height as f64;
        let real_width = self.width / 100.0 * height as f64;
        let dims = [0.0, height as f64 / 2.0 - real_height / 2.0, real_width, real_height];
        graphics::rectangle(c, dims, trans.trans(0f64, 0f64), g);
        return real_width;
    }
}
