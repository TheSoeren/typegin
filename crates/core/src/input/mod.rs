pub mod action;
pub mod direction;
pub mod lexer;
pub mod tokenizer;

pub use action::{Action, DropResult, MoveResult, TakeResult};
pub use direction::{Direction, DirectionResolution};
use lexer::lex;
use tokenizer::tokenize;

pub fn parse_input(input: &str) -> Action {
    let tokens = tokenize(input);
    let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    lex(&token_refs)
}
