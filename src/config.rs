use rs_config;
use rs_config::ConfigAble;


#[derive(Debug, Clone, ConfigAble)]
pub enum Size {
    Pixels(i32),
    Percent(i32),
    Font(i32),
}

impl Size {
    pub fn get_height(&self, mon: i32) -> i32 {
        match self {
            &Size::Pixels(ref size) => *size,
            &Size::Percent(ref size) => mon * *size / 100,
            &Size::Font(ref size) => *size * 3 /2,
        }
    }
}

#[derive(Debug, Clone, ConfigAble)]
pub enum Parser {
    Dzen,
    Ongybar,
    Plain,
}

#[derive(Debug, Clone, ConfigAble)]
pub enum Anchor {
    Left,
    Right,
    Middle
}

#[derive(Debug, ConfigAble)]
pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, ConfigAble)]
pub enum Position {
    Global(Direction),
    Monitor(String, Direction),
}

#[derive(Debug, ConfigAble)]
pub enum InputSource {
    Stdin,
    Pipe(i32),
    Named(String),
    Spawn(String),
}

#[derive(Debug, ConfigAble)]
pub struct Input {
    pub source: InputSource,
    #[ConfigAttrs(default = "0")]
    pub layer: i32,
    #[ConfigAttrs(default = "\"\".to_string()")]
    pub name: String,
    #[ConfigAttrs(default = "Parser::Plain")]
    pub parser: Parser,
    pub position: Anchor,
}

#[derive(Debug, ConfigAble)]
pub struct Config {
    #[ConfigAttrs(default = "Position::Global(Direction::Top)")]
    pub position: Position,
    #[ConfigAttrs(default = "vec![Input {source: InputSource::Stdin, layer: 0, name: \"\".into(), parser: Parser::Plain, position: Anchor::Middle}]")]
    pub inputs: Vec<Input>,
    #[ConfigAttrs(default = "\"ongybar\".to_string()")]
    pub title: String,
    #[ConfigAttrs(default = "Size::Pixels(16)")]
    pub size: Size
}

/// Get the default config
pub fn get_default() -> Config {
    println!("Running default config");
    /* This unwrap Is fine. It's guaranteed to exist */
    return Config::get_default().unwrap();
}
