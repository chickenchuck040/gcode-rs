//! The first stage of parsing which turns tokens into a basic gcode
//! representation.

use core::iter::Peekable;
use core::fmt::{self, Formatter, Display};
use arrayvec::ArrayVec;

use lexer::{Token, Span, TokenKind};
use errors::*;


/// An argument buffer containing up to 10 Arguments.
pub type ArgBuffer = ArrayVec<[Argument; 10]>;


/// A parser which takes a stream of characters and parses them as gcode
/// instructions.
///
///
/// # Grammar
///
/// The grammar used has one token of lookahead and is roughly as follows:
///
/// ```text
/// line ::= command
///        | program_number
///
/// program_number ::= O number
///
/// command ::= line_number command_name args
///
/// command_name ::= command_type number
///
/// command_type ::= G
///                | M
///
/// args ::= args arg
///
/// arg ::= arg_kind number
///       | <epsilon>
///
/// arg_kind ::= X
///            | Y
///            | Z
///            | F
///            | R
///
/// line_number ::= N number
///               | <epsilon>
///
/// number ::= MINUS NUMBER
///          | NUMBER
/// ```
#[derive(Debug)]
#[deprecated]
pub struct BasicParser<I>
    where I: Iterator<Item = Token>
{
    stream: Peekable<I>,
}

/// Peek at the next token, if its kind isn't one of the specified `$pattern`s,
/// return a `Error::SyntaxError` with the provided message.
macro_rules! lookahead {
    ($self:expr, $err_msg:expr, $( $pattern:pat )|*) => {
        match $self.peek() {
            $( Some($pattern) )|* => {},
            Some(_) => {
                let next = $self.stream.peek().unwrap();
                return Err(Error::SyntaxError($err_msg, next.span()));
            }
            None => return Err(Error::UnexpectedEOF),
        }
    }
}

impl<I> BasicParser<I>
    where I: Iterator<Item = Token>
{
    /// Create a new `BasicParser` from a token stream.
    pub fn new(stream: I) -> BasicParser<I> {
        BasicParser { stream: stream.peekable() }
    }

    /// Parse the input and get the next line.
    pub fn parse(&mut self) -> Result<Line> {
        let next_span = self.next_span();

        if let Ok(n) = self.program_number() {
            return Ok(Line::ProgramNumber(n));
        }

        self.command()
            .map(|mut c| {
                     if let Some(span) = next_span {
                         c.span = span;
                     }
                     Line::Cmd(c)
                 })
    }

    fn program_number(&mut self) -> Result<u32> {
        lookahead!(self, "Expected a 'O'", TokenKind::O);
        let _ = self.stream.next();

        self.number().map(|n| n as u32)
    }

    fn number(&mut self) -> Result<f32> {
        // Check for a negative sign, consuming it if we find one
        let is_negative = match self.peek() {
            Some(TokenKind::Minus) => {
                let _ = self.stream.next();
                true
            }
            _ => false,
        };

        lookahead!(self, "Expected a number", TokenKind::Number(_));

        let n = match self.stream.next().unwrap().kind() {
            TokenKind::Number(n) => n,
            _ => unreachable!(),
        };

        if is_negative { Ok(-1.0 * n) } else { Ok(n) }
    }

    fn command(&mut self) -> Result<Command> {
        let span = match self.next_span() {
            Some(span) => span,
            None => return Err(Error::UnexpectedEOF),
        };

        let line_number = self.line_number()?;
        let (command_type, command_number) = self.command_name()?;
        let args = self.args()?;

        let cmd = Command {
            span,
            line_number,
            command_type,
            args,
            command_number,
        };
        Ok(cmd)
    }

    fn command_name(&mut self) -> Result<(CommandType, u32)> {
        let ty = self.command_type()?;
        let n = self.number()?;

        Ok((ty, n as u32))
    }

    fn command_type(&mut self) -> Result<CommandType> {
        lookahead!(self, "Expected a command type", TokenKind::G | TokenKind::M | TokenKind::T);

        match self.stream.next().unwrap().kind() {
            TokenKind::G => Ok(CommandType::G),
            TokenKind::M => Ok(CommandType::M),
            TokenKind::T => Ok(CommandType::T),
            _ => unreachable!(),
        }
    }

    fn line_number(&mut self) -> Result<Option<u32>> {
        if self.peek() != Some(TokenKind::N) {
            return Ok(None);
        }

        let _ = self.stream.next();

        if let Ok(n) = self.number() {
            Ok(Some(n as u32))
        } else {
            Ok(None)
        }
    }

    fn arg_kind(&mut self) -> Result<ArgumentKind> {
        lookahead!(self,
                   "Expected an argument kind",
                   TokenKind::X | TokenKind::Y | TokenKind::Z |
                   TokenKind::R | TokenKind::S |
                   TokenKind::H | TokenKind::P | TokenKind::I |
                   TokenKind::J | TokenKind::E |
                   TokenKind::FeedRate);

        match self.stream.next().unwrap().kind() {
            TokenKind::X => Ok(ArgumentKind::X),
            TokenKind::Y => Ok(ArgumentKind::Y),
            TokenKind::Z => Ok(ArgumentKind::Z),
            TokenKind::R => Ok(ArgumentKind::R),
            TokenKind::S => Ok(ArgumentKind::S),
            TokenKind::H => Ok(ArgumentKind::H),
            TokenKind::P => Ok(ArgumentKind::P),
            TokenKind::I => Ok(ArgumentKind::I),
            TokenKind::J => Ok(ArgumentKind::J),
            TokenKind::E => Ok(ArgumentKind::E),
            TokenKind::FeedRate => Ok(ArgumentKind::FeedRate),
            _ => unreachable!(),
        }
    }

    fn arg(&mut self) -> Result<Option<Argument>> {
        if let Ok(kind) = self.arg_kind() {
            let n = self.number()?;
            Ok(Some(Argument {
                        kind: kind,
                        value: n,
                    }))

        } else {
            Ok(None)
        }
    }

    fn args(&mut self) -> Result<ArgBuffer> {
        let mut buffer = ArgBuffer::new();

        while let Ok(Some(arg)) = self.arg() {
            buffer.push(arg);
        }

        Ok(buffer)
    }

    fn peek(&mut self) -> Option<TokenKind> {
        self.stream.peek().map(|t| t.kind())
    }

    fn next_span(&mut self) -> Option<Span> {
        self.stream.peek().map(|t| t.span())
    }
}

impl<I> Iterator for BasicParser<I>
    where I: Iterator<Item = Token>
{
    type Item = Result<Line>;

    fn next(&mut self) -> Option<Self::Item> {
        let got = self.parse();

        if got == Err(Error::UnexpectedEOF) {
            None
        } else {
            Some(got)
        }
    }
}

/// A gcode command.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    span: Span,
    line_number: Option<u32>,
    command_type: CommandType,
    command_number: u32,
    args: ArgBuffer,
}

impl Command {
    /// Get the location of the `Command` in source code.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The line number as declared with `N123` (if provided).
    pub fn line_number(&self) -> Option<u32> {
        self.line_number
    }

    /// Loosely-typed representation of the command (e.g. `(G, 90)`).
    pub fn command(&self) -> (CommandType, u32) {
        (self.command_type, self.command_number)
    }

    /// Get the arguments this command was invoked with.
    pub fn args(&self) -> &[Argument] {
        &self.args
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(n) = self.line_number {
            write!(f, "N{} ", n)?;
        }

        write!(f, "{}{}", self.command_type, self.command_number)?;

        for arg in &self.args {
            write!(f, " {}", arg)?;
        }

        write!(f, "\t(line: {}, column: {})", self.span.line, self.span.column)
    }
}

impl From<(CommandType, u32)> for Command {
    fn from(other: (CommandType, u32)) -> Self {
        Command {
            span: Span::default(),
            line_number: None,
            command_type: other.0,
            command_number: other.1,
            args: ArgBuffer::default(),
        }
    }
}

/// An argument for a gcode command.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Argument {
    /// What type of argument this is.
    pub kind: ArgumentKind,
    /// Its value.
    pub value: f32,
}

impl Argument {
    /// Create a new argument.
    pub fn new(kind: ArgumentKind, value: f32) -> Argument {
        Argument { kind, value }
    }
}

impl Display for Argument {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", self.kind, self.value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum ArgumentKind {
    X,
    Y,
    Z,

    R,
    S,
    H,
    FeedRate,
    P,
    I,
    J,
    E,
}

impl Display for ArgumentKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ArgumentKind::X | ArgumentKind::Y | ArgumentKind::Z | ArgumentKind::R |
            ArgumentKind::S | ArgumentKind::H | ArgumentKind::P | ArgumentKind::I |
            ArgumentKind::E | ArgumentKind::J => write!(f, "{:?}", self),
            ArgumentKind::FeedRate => write!(f, "F"),
        }
    }
}

/// An enum representing the command type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandType {
    /// A general G code.
    G,
    /// A M code.
    M,
    /// An instruction to change tools.
    T,
}


impl Display for CommandType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}



/// A line of gcode.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// A gcode command.
    Cmd(Command),
    /// The program number.
    ProgramNumber(u32),
}


impl Display for Line {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Line::Cmd(ref cmd) => write!(f, "{}", cmd),
            Line::ProgramNumber(n) => write!(f, "O{}", n),
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use lexer::TokenKind;

    #[test]
    fn parse_no_line_number() {
        let src = vec![];
        let should_be = None;

        let mut parser = BasicParser::new(src.into_iter());

        let got = parser.line_number().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_line_number() {
        let src = [TokenKind::N, TokenKind::Number(10.0)];
        let should_be = Some(10);

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.line_number().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_empty_arg() {
        let src = vec![];
        let mut parser = BasicParser::new(src.into_iter());
        let got = parser.arg().unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn parse_x_arg() {
        let src = vec![TokenKind::X, TokenKind::Number(3.14)];
        let should_be = Argument {
            kind: ArgumentKind::X,
            value: 3.14,
        };

        let tokens = src.iter().map(|&k| k.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.arg().unwrap().unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_empty_args() {
        let src = vec![];
        let mut parser = BasicParser::new(src.into_iter());
        let got = parser.args().unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn parse_single_args() {
        let src = vec![TokenKind::X, TokenKind::Number(3.14)];
        let should_be = Argument {
            kind: ArgumentKind::X,
            value: 3.14,
        };

        let tokens = src.iter().map(|&k| k.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.args().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0], should_be);
    }

    #[test]
    fn parse_multiple_args() {
        let src = vec![TokenKind::X,
                       TokenKind::Number(3.14),
                       TokenKind::Y,
                       TokenKind::Number(2.1828),
                       TokenKind::Z,
                       TokenKind::Number(6.0)];

        let mut should_be = ArgBuffer::new();
        should_be.push(Argument {
                           kind: ArgumentKind::X,
                           value: 3.14,
                       });
        should_be.push(Argument {
                           kind: ArgumentKind::Y,
                           value: 2.1828,
                       });
        should_be.push(Argument {
                           kind: ArgumentKind::Z,
                           value: 6.0,
                       });

        let tokens = src.iter().map(|&k| k.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.args().unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_basic_command() {
        let src = vec![TokenKind::G, TokenKind::Number(90.0)];
        let should_be = Command {
            span: (0, 0).into(),
            command_type: CommandType::G,
            command_number: 90,
            args: ArgBuffer::new(),
            line_number: None,
        };

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.command().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_normal_g01() {
        let src = vec![TokenKind::N,
                       TokenKind::Number(10.0),
                       TokenKind::G,
                       TokenKind::Number(91.0),
                       TokenKind::X,
                       TokenKind::Number(1.0),
                       TokenKind::Y,
                       TokenKind::Number(3.1415),
                       TokenKind::Z,
                       TokenKind::Number(-20.0)];
        let mut should_be = Command {
            span: (0, 0).into(),
            command_type: CommandType::G,
            command_number: 91,
            args: ArgBuffer::new(),
            line_number: Some(10),
        };

        should_be
            .args
            .push(Argument {
                      kind: ArgumentKind::X,
                      value: 1.0,
                  });
        should_be
            .args
            .push(Argument {
                      kind: ArgumentKind::Y,
                      value: 3.1415,
                  });
        should_be
            .args
            .push(Argument {
                      kind: ArgumentKind::Z,
                      value: -20.0,
                  });

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.command().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_command_and_name() {
        let src = [TokenKind::G, TokenKind::Number(0.0)];
        let should_be = (CommandType::G, 0);

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.command_name().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_program_number() {
        let src = [TokenKind::O, TokenKind::Number(50.0)];
        let should_be = 50;

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.program_number().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn tool_change_line() {
        let src = [TokenKind::T, TokenKind::Number(1.0)];
        let should_be = Command {
            span: (0, 0).into(),
            line_number: None,
            command_type: CommandType::T,
            command_number: 1,
            args: ArgBuffer::new(),
        };

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.command().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_negative_arg() {
        let src = [TokenKind::X, TokenKind::Minus, TokenKind::Number(6.0)];
        let should_be = Argument {
            kind: ArgumentKind::X,
            value: -6.0,
        };

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.arg().unwrap().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn spindle_speed() {
        let src = [TokenKind::S, TokenKind::Number(600.0)];
        let should_be = Argument {
            kind: ArgumentKind::S,
            value: 600.0,
        };

        let tokens = src.iter().map(|&t| t.into());
        let mut parser = BasicParser::new(tokens);

        let got = parser.arg().unwrap().unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn argument_kinds() {
        let inputs = vec![(TokenKind::X, ArgumentKind::X),
                          (TokenKind::Y, ArgumentKind::Y),
                          (TokenKind::Z, ArgumentKind::Z),

                          (TokenKind::R, ArgumentKind::R),
                          (TokenKind::S, ArgumentKind::S),
                          (TokenKind::H, ArgumentKind::H),
                          (TokenKind::P, ArgumentKind::P),
                          (TokenKind::I, ArgumentKind::I),
                          (TokenKind::J, ArgumentKind::J),
                          (TokenKind::E, ArgumentKind::E),
                          (TokenKind::FeedRate, ArgumentKind::FeedRate)];

        for (input, should_be) in inputs.into_iter() {
            println!("{:?} => {:?}", input, should_be);

            let src = [input];
            let tokens = src.iter().map(|&t| t.into());

            let got = BasicParser::new(tokens).arg_kind().unwrap();
            assert_eq!(got, should_be);
        }
    }

    /// This test makes sure we don't get regressions on issue #5
    /// link: https://github.com/Michael-F-Bryan/gcode-rs/issues/5
    #[test]
    fn m_is_not_an_argument() {
        let input = vec![Token::from(TokenKind::M)];

        let mut parser = BasicParser::new(input.clone().into_iter());
        let got = parser.arg_kind();
        assert!(got.is_err());

        let mut parser = BasicParser::new(input.into_iter());
        let got = parser.command_type();
        assert!(got.is_ok());
    }

    #[allow(trivial_casts)]
    mod qc {
        use super::*;
        use std::prelude::v1::*;

        macro_rules! quick_parser_quickcheck {
            ($method:ident) => (
                quickcheck!{
                    fn $method(tokens: Vec<Token>) -> () {
                    let mut parser = BasicParser::new(tokens.into_iter());
                    let _ = parser.$method();
                    }
                }
            )
        }

        quick_parser_quickcheck!(parse);

        quick_parser_quickcheck!(command);
        quick_parser_quickcheck!(command_name);
        quick_parser_quickcheck!(command_type);
        quick_parser_quickcheck!(number);
        quick_parser_quickcheck!(arg);
        quick_parser_quickcheck!(arg_kind);
        quick_parser_quickcheck!(program_number);
        quick_parser_quickcheck!(line_number);
    }
}
