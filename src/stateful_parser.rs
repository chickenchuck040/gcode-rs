#![allow(dead_code, missing_docs)]

use lexer::{Token, Span};
use errors::*;


#[derive(Copy, Clone, Hash, PartialEq, Debug)]
enum CurrentCommand {
    Uninitialized,
    G,
    M,
    ProgramNumber,
}

impl Default for CurrentCommand {
    fn default() -> Self {
        CurrentCommand::Uninitialized
    }
}


#[derive(Debug, )]
pub struct Parser<I>
    where I: Iterator<Item = Token>
{
    tokens: I,
    span: Span,
    state: CurrentCommand,
}


impl<I> Parser<I>
    where I: Iterator<Item = Token>
{
    pub fn new(tokens: I) -> Parser<I> {
        Parser {
            tokens: tokens,
            span: Span::default(),
            state: CurrentCommand::default(),
        }
    }

    fn step(&mut self) -> Result<Option<Line>> {
        if let Some(next) = self.tokens.next() {
            unimplemented!()
        } else {
            Err(Error::UnexpectedEOF)
        }
    }
}


#[derive(Copy, Clone, Hash, PartialEq, Debug)]
pub enum Line {
    ProgramNumber(u32),
}


#[cfg(test)]
mod tests {
    use super::*;
    use lexer::TokenKind;

    #[test]
    fn parse_program_number() {
        let src = [TokenKind::O, TokenKind::Number(1000.0)];
        let tokens = src.iter().map(|&t| t.into());

        let mut parser = Parser::new(tokens);

        assert_eq!(parser.state, CurrentCommand::Uninitialized);
        let got = parser.step().unwrap();
        assert_eq!(parser.state, CurrentCommand::ProgramNumber);

        assert_eq!(got, None);
    }
}
