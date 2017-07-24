use rs_config;
use rs_config::ConfigAble;

#[derive(Debug, Clone, ConfigAble)]
pub enum Parser {
    Dzen,
    Ongybar,
    Plain,
}

#[derive(Debug, Clone, ConfigAble)]
pub enum StaticPosition {
    Left,
    Right,
    Middle,
    MiddleLeft,
    MiddleRight,
}

#[derive(Debug, Clone, ConfigAble)]
pub enum Anchor {
    Static(StaticPosition),
    LeftOf(String),
    RightOf(String),
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
    #[ConfigAttrs(default = "vec![Input {source: InputSource::Stdin, layer: 0, name: \"\".into(), parser: Parser::Plain, position: Anchor::Static(StaticPosition::Middle)}]")]
    pub inputs: Vec<Input>,
    #[ConfigAttrs(default = "\"ongybar\".to_string()")]
    pub title: String,
}
