extern crate graphics;
extern crate opengl_graphics;
extern crate num_traits;

use modules::renderable::Renderable;
use modules::ongystr::OngyStr;
use modules::colored::Colored;
use modules::ongypos::OngyPos;
use modules::ongyrect::OngyRect;
use modules::ongyimage::OngyImage;

use std;
use std::vec::Vec;
use std::boxed::Box;
use std::io::Write;
use self::num_traits::Num;

struct DzenIter<G, C> {
    text: String,
    g: std::marker::PhantomData<G>,
    c: std::marker::PhantomData<C>,
}

impl<G, C> DzenIter<G, C> {
    fn new(text: &str) -> Self {
        return DzenIter{ text: String::from(text),
                         g: std::marker::PhantomData,
                         c: std::marker::PhantomData  };
    }
}

fn parse_pos(text: &str) -> Option<OngyPos> {
    let x = match text.find(';') {
            Some(x) => &text[..x],
            None => text,
        };

    /* There's a few special values that we wan't to discard, but don't error
     * when they appear.
     * dzen has all of them starting with a '_'
     */
    if x.starts_with('_') {
        /* This exactly removes the offset from having an element in the trait
         * Renderable implementation
         */
        return Some(OngyPos(-4f64));
    }

    return match f64::from_str_radix(x, 10) {
        Ok(val) => Some(OngyPos(val)),
        Err(_) => None
    }
}

fn parse_image(text: &str) -> Option<OngyImage> {
    return Some(OngyImage(String::from(text).replace(".xbm", ".bmp")));
}

fn parse_rect(text: &str) -> Option<OngyRect> {
    match text.find('x') {
        Some(i) => {
            let w_str = &text[..i];
            let h_str = &text[i + 1..];

            //TODO: Remove unwrap()
            let w = f64::from_str_radix(w_str, 10).unwrap();
            let h = f64::from_str_radix(h_str, 10).unwrap();

            return Some(OngyRect{width: w, height: h});
        }

        None => return None
    }
}

fn find_str(text: &String) -> Option<(OngyStr, String)> {
    match text.find('^') {
        None => {
            return Some((OngyStr(text.clone()), String::new()));
        }
        Some(i) => {
            let left  = &text.as_str()[..i];
            let right = &text.as_str()[i + 1..];

            if right.chars().nth(0) == Some('^') {
                match find_str(&String::from(&right[1..])) {
                    Some((OngyStr(x), ret)) => {
                        let mut tmp = String::from(&text.as_str()[.. i + 1]);
                        tmp.push_str(x.as_str());
                        return Some((OngyStr(tmp), ret));
                    },
                    None => {
                        return Some((OngyStr(String::from(&text.as_str()[.. i + 1])), String::from(right)));
                    }
                }
            }

            if i == 0 {
                return None;
            }

            return Some((OngyStr(String::from(left)), String::from(&text.as_str()[i ..])));
        }
    }

}

fn parse_color(hex_str: &str) -> Option<[f32; 4]> {
    if hex_str.chars().nth(0) != Some('#') {
        std::io::stderr().write(b"Sorry, ongybar currently only supports colors as #Hexstr").unwrap();
        return None;
    }
    // TODO: Detect smaller/longer strs!
    let red   = &hex_str[1..3];
    let green = &hex_str[3..5];
    let blue  = &hex_str[5..7];
    // TODO: Remove the unwrap()...
    return Some([f32::from_str_radix(red, 16).unwrap(),
                 f32::from_str_radix(green, 16).unwrap(),
                 f32::from_str_radix(blue, 16).unwrap(),
                 1.0]);
}

impl<G, C> Iterator for DzenIter<G, C>
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static, {
    type Item=Box<Renderable<G, C>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.len() == 0 {
            return None;
        }

        match find_str(&self.text) {
            Some ((ret, nxt)) => {
                self.text = nxt;
                return Some(Box::new(ret) as Box<Renderable<G, C>>);
            }
            None => {
                if self.text.starts_with("^fg(") {
                    if self.text.starts_with("^fg()") {
                        self.text = String::from(&self.text.as_str()[5 .. ]);
                        return Some(Box::new(OngyPos(-4.0)));
                    }
                    return match self.text.find(')') {
                        Some(i) => {
                            match parse_color(&self.text.as_str()[4 .. i]) {
                                Some(c) =>  {
                                     match self.text.as_str()[i + 1..].find("^fg(") {
                                        Some (j) => {
                                            let tmp = dzen_parse(&self.text.as_str()[i + 1 .. i + 1 + j]);
                                            self.text = String::from(&self.text.as_str()[i + 1 + j ..]);
                                            return Some(Box::new(Colored{ color: c, elem: Box::new(tmp)}));
                                        }
                                        None => {
                                            let tmp = dzen_parse(&self.text.as_str()[i + 1 ..]);
                                            self.text.clear();
                                            return Some(Box::new(Colored{ color: c, elem: Box::new(tmp)}));
                                        }
                                    }
                                }
                                None => None
                            }
                        }
                        None => {
                            std::io::stderr().write(b"Found '^fg(', but couldn't find closing parens!").unwrap();
                            return None;
                        }
                    }
                }
                if self.text.starts_with("^p(") {
                    return match self.text.find(')') {
                        Some(i) => {
                            match parse_pos(&self.text.as_str()[3 .. i]) {
                                Some(r) =>  {
                                    self.text = String::from(&self.text.as_str()[i + 1 ..]);
                                    return Some(Box::new(r));
                                }
                                None => None
                                }
                            }
                        None => {
                            std::io::stderr().write(b"Found '^fg(', but couldn't find closing parens!").unwrap();
                            return None;
                        }
                    }
                }
                if self.text.starts_with("^r(") {
                    return match self.text.find(')') {
                        Some(i) => {
                            match parse_rect(&self.text.as_str()[3 .. i]) {
                                Some(r) =>  {
                                    self.text = String::from(&self.text.as_str()[i + 1 ..]);
                                    return Some(Box::new(r));
                                }
                                None => None
                                }
                            }
                        None => {
                            std::io::stderr().write(b"Found '^fg(', but couldn't find closing parens!").unwrap();
                            return None;
                        }
                    }
                }
                if self.text.starts_with("^i") {
                    return match self.text.find(')') {
                        Some(i) => {
                            match parse_image(&self.text.as_str()[3 .. i]) {
                                Some(r) =>  {
                                    self.text = String::from(&self.text.as_str()[i + 1 ..]);
                                    return Some(Box::new(r));
                                }
                                None => None
                                }
                            }
                        None => {
                            std::io::stderr().write(b"Found '^i(', but couldn't find closing parens!").unwrap();
                            return None;
                        }
                    }
                }
                if self.text.starts_with("^bg(") {
                    //std::io::stderr().write(b"Sorry, ongybar currently ignores background colours\n").unwrap();
                    match self.text.find(')') {
                        None => {
                            std::io::stderr().write(b"Found ^bg(, but not closing parens").unwrap();
                            return None;
                        }
                        Some(i) => {
                            self.text = String::from(&self.text.as_str()[i + 1..]);
                            return Some(Box::new(OngyPos(-4.0)));
                        }
                    }
                }
                if self.text.starts_with("^pa(") {
                    //std::io::stderr().write(b"Sorry, ongybar currently ignores total positions\n").unwrap();
                    match self.text.find(')') {
                        None => {
                            std::io::stderr().write(b"Found ^pa(, but not closing parens").unwrap();
                            return None;
                        }
                        Some(i) => {
                            self.text = String::from(&self.text.as_str()[i + 1..]);
                            return Some(Box::new(OngyPos(-4.0)));
                        }
                    }
                }
                println!("Currently can't handle \"{}\"", self.text);
                return None;
            }
        }
    }
}

pub fn dzen_parse<G, C>(arg: &str) -> Vec<Box<Renderable<G, C>>>
    where C: graphics::character::CharacterCache<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture>  + 'static,
          G: graphics::Graphics<Texture = <opengl_graphics::GlGraphics as graphics::Graphics>::Texture> + 'static, {
    return DzenIter::new(arg).collect();
}
