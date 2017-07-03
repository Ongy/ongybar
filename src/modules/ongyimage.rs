extern crate graphics;
extern crate opengl_graphics;

use modules::renderable::Renderable;
use modules::renderable::OngybarState;
use graphics::Transformed;
use graphics::ImageSize;

// This will be the Path string
pub struct OngyImage(pub String);

impl <G, C> Renderable<G, C> for OngyImage
    where G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> {
    fn get_size(&self, _: &mut C, _: u32, o: &mut OngybarState) -> f64 {
        return match o.get_image(&self.0) {
            Some(i) => i.get_width() as f64,
            None => 0.0,
        }
    }

    fn do_render(&self, g: &mut G, height: u32,
                 o: &mut OngybarState, trans: &graphics::math::Matrix2d,
                 _: &mut C, _: graphics::types::Color) -> f64 {
        match o.get_image(&self.0) {
            Some(i) => {
                let i_height = i.get_height() as f64;
                let offset = height as f64 / 2.0 - i_height / 2.0;
                graphics::image(i, trans.trans(0.0, offset), g);
                return i.get_width() as f64;
            }
            None => return 0.0,
        }
    }
}
