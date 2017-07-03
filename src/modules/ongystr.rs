extern crate graphics;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;
use graphics::Transformed;

pub struct OngyStr(pub String);

impl<G, C> Renderable<G, C> for OngyStr
    where C: graphics::character::CharacterCache,
          G: graphics::Graphics<Texture = <C as graphics::character::CharacterCache>::Texture> {

    fn get_size(&self, cache: &mut C, height: u32, _: &mut OngybarState) -> f64 {
        let text_height = (height - 2) * 2 / 3;

        return cache.width(text_height, self.0.as_str());
    }

    fn do_render(&self, g: &mut G, height: u32, _: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 c: graphics::types::Color) -> f64 {
        let text_height = (height - 2) * 2 / 3;
        graphics::text(c, text_height, self.0.as_str(), cache,
                       trans.trans(0f64, text_height as f64 + 2f64), g);

        return cache.width(text_height, self.0.as_str());
    }
}

