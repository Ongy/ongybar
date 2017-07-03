extern crate graphics;
extern crate opengl_graphics;

use std::collections::HashMap;
use std;
use graphics::Transformed;

pub struct OngybarState {
    images: HashMap<String, <opengl_graphics::GlGraphics as graphics::Graphics>::Texture>,
}

impl OngybarState {
    pub fn new() -> Self {
        return OngybarState { images: HashMap::new() };
    }

    pub fn get_image(&mut self, path: &String) -> Option<&opengl_graphics::Texture> {
        /* Load the image, insert it into the cache, and then get it */
        if !self.images.contains_key(path) {
            match  opengl_graphics::Texture::from_path(path) {
                Ok(image) =>  {
                    self.images.insert(path.clone(), image);
                }
                Err(_) => return None
            }
        }

        return self.images.get(path);
    }
}


pub trait Renderable<G, C> {
    fn get_size(&self, cache: &mut C, height: u32, &mut OngybarState) -> f64;
    fn do_render(&self, g: &mut G, height: u32, o: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 color: graphics::types::Color) -> f64;
}

impl<G, C, I> Renderable<G, C> for I
    where for<'a> &'a I: std::iter::IntoIterator<Item=&'a Box<Renderable<G, C>>> {
    fn get_size(&self, c: &mut C, h: u32, o: &mut OngybarState) -> f64 {
        let mut first = true;
        let mut ret = 0f64;

        for ref x in self.into_iter() {
            if first {
                first = false;
            } else {
                ret += 4f64;
            }

            ret += x.get_size(c, h, o);
        }
        return ret;
    }

    fn do_render(&self, g: &mut G, h: u32, o: &mut OngybarState,
                 trans: &graphics::math::Matrix2d, cache: &mut C,
                 c: graphics::types::Color) -> f64 {
        let mut first = true;
        let mut total_offset = 0f64;
        let mut cur_trans = trans.trans(0f64, 0f64);

        for ref x in self.into_iter() {
            let offset = x.do_render(g, h, o, &cur_trans, cache, c);
            cur_trans = cur_trans.trans(offset + 4f64, 0f64);
            total_offset += offset;
            if !first {
                total_offset += 4.0;
            } else {
                first = false;
            }
        }
        return total_offset;
    }
}
