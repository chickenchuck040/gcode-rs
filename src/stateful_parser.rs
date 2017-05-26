#![allow(dead_code, missing_docs)]

use arrayvec::ArrayVec;

use lexer::{Token, TokenKind, Span};
use errors::*;

type ArgBuffer = ArrayVec<[Argument; 10]>;


#[derive(Copy, Clone, Hash, PartialEq, Debug)]
enum State {
    Start,
    End,
    G,
    M,
    ProgramNumber,
}

impl Default for State {
    fn default() -> Self {
        State::Start
    }
}


#[derive(Debug, )]
pub struct Parser<I>
    where I: Iterator<Item = Token>
{
    tokens: I,
    span: Span,
    state: State,
}


impl<I> Parser<I>
    where I: Iterator<Item = Token>
{
    pub fn new(tokens: I) -> Parser<I> {
        Parser {
            tokens: tokens,
            span: Span::default(),
            state: State::default(),
        }
    }

    fn step(&mut self) -> Result<Option<Line>> {
        if let Some(next) = self.tokens.next() {
            self.span = next.span();

            match self.state {
                State::Start => self.step_start(next),
                State::ProgramNumber => self.step_program_number(next),
                State::M => self.step_m(next),

                _ => unimplemented!(),
            }
        } else {
            Err(Error::UnexpectedEOF)
        }
    }

    /// We're at the initial state and want to parse a valid command type.
    fn step_start(&mut self, tok: Token) -> Result<Option<Line>> {
        match tok.kind() {
            TokenKind::O => {
                self.state = State::ProgramNumber;
                Ok(None)
            }
            TokenKind::M => {
                self.state = State::M;
                Ok(None)
            }

            _ => unimplemented!(),
        }
    }

    /// We've just parsed an `O` command and now need to get the program
    /// number.
    fn step_program_number(&mut self, tok: Token) -> Result<Option<Line>> {
        if let TokenKind::Number(n) = tok.kind() {
            // We've finished parsing a program number, go back to the start
            self.state = State::Start;

            Ok(Some(Line::ProgramNumber(n as u32)))
        } else {
            Err(Error::SyntaxError("'O' token should be followed by a program number",
                                   self.span))
        }
    }

    fn step_m(&mut self, tok: Token) -> Result<Option<Line>> {
        unimplemented!()
    }
}


#[derive(Copy, Clone, Hash, PartialEq, Debug)]
pub enum Line {
    ProgramNumber(u32),
    M(u32),
}

#[derive(Copy, Clone, Hash, PartialEq, Debug)]
struct Command {
    number: u32,
    // args: ArgBuffer,
}

#[derive(Copy, Clone, Hash, PartialEq, Debug)]
enum Argument {}


#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! tokens {
        ($src:expr) => {
            {
                use std::vec::Vec;
                use ::lexer::Tokenizer;
                let tokens: Result<Vec<_>> = Tokenizer::new($src.chars()).collect();
                tokens.unwrap().into_iter()
            }
        }
    }

    #[test]
    fn start_and_step_into_program_number() {
        let src = tokens!("O1000");
        let mut parser = Parser::new(src);

        assert_eq!(parser.state, State::Start);
        let got = parser.step().unwrap();
        assert_eq!(parser.state, State::ProgramNumber);

        assert_eq!(got, None);
    }

    #[test]
    fn parse_full_program_number() {
        let src = tokens!("O1000");
        let mut parser = Parser::new(src);

        assert_eq!(parser.state, State::Start);
        assert_eq!(parser.step().unwrap(), None);
        assert_eq!(parser.state, State::ProgramNumber);
        let got = parser.step().unwrap();
        assert_eq!(parser.state, State::Start);

        assert_eq!(got, Some(Line::ProgramNumber(1000)));
    }

    #[test]
    fn start_and_step_into_m_code() {
        let src = tokens!("M50\n");
        let mut parser = Parser::new(src);

        assert_eq!(parser.step().unwrap(), None);
        assert_eq!(parser.state, State::M);
        let got = parser.step().unwrap();
        assert_eq!(parser.state, State::Start);

        assert_eq!(got, Some(Line::M(50)));
    }
}
