use crate::command::{Command, Operator, RedirectTarget, RedirectType, Redirection};
use crate::lexer::{Lexer, Token};

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Parser {
        let current_token = lexer.next_token();

        Parser {
            lexer,
            current_token,
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    pub fn parse(&mut self) -> Result<Command, String> {
        self.parse_with_min_precedence(0)
    }

    fn parse_with_min_precedence(&mut self, min_precedence: u8) -> Result<Command, String> {
        let mut left = self.parse_command()?;

        loop {
            let (operator, precedence) = match self.current_token {
                Token::Pipe => (Operator::Pipe, 4),
                Token::And => (Operator::And, 3),
                Token::Or => (Operator::Or, 2),
                Token::Semicolon => (Operator::Semicolon, 1),
                Token::Background => (Operator::Background, 1),
                _ => break,
            };

            if precedence < min_precedence {
                break;
            }

            self.advance();
            let right = self.parse_with_min_precedence(precedence + 1)?;
            left = Command::Binary {
                left: Box::new(left),
                right: Box::new(right),
                operator,
            };
        }

        Ok(left)
    }

    fn parse_command(&mut self) -> Result<Command, String> {
        let mut words = vec![];
        let mut redirects = vec![];

        loop {
            match &self.current_token {
                Token::Word(w) => {
                    words.push(w.clone());
                    self.advance();
                }
                Token::SingleQuoted(s) | Token::DoubleQuoted(s) => {
                    words.push(s.clone());
                    self.advance();
                }
                Token::RedirectType(_) => {
                    redirects.push(self.parse_redirection()?);
                }
                _ => break,
            }
        }

        if words.is_empty() {
            return Err("Empty command".to_string());
        }

        Ok(Command::Simple {
            executable: words.remove(0),
            args: words,
            redirects,
        })
    }

    fn parse_redirection(&mut self) -> Result<Redirection, String> {
        let rt = match &self.current_token {
            Token::RedirectType(t) => t.clone(),
            _ => return Err("Expected redirect operator".to_string()),
        };
        self.advance();

        let (mut fd, direction) = match rt {
            RedirectType::Overwrite => (Some(1), RedirectType::Overwrite),
            RedirectType::Append => (Some(1), RedirectType::Append),
            RedirectType::DuplicateOut => (Some(1), RedirectType::DuplicateOut),
            RedirectType::Input => (Some(0), RedirectType::Input),
            RedirectType::DuplicateIn => (Some(0), RedirectType::DuplicateIn),
            RedirectType::HereDoc => (Some(0), RedirectType::HereDoc),
        };

        if let Token::Word(n) = &self.current_token {
            if let Ok(num) = n.parse::<u32>() {
                fd = Some(num);
                self.advance();
            }
        }

        let target = match &self.current_token {
            Token::Word(filename) => {
                let t = filename.clone();
                self.advance();
                RedirectTarget::File(t)
            }
            Token::SingleQuoted(s) | Token::DoubleQuoted(s) => {
                let t = s.clone();
                self.advance();
                RedirectTarget::File(t)
            }
            _ => return Err("Invalid redirect target".to_string()),
        };

        Ok(Redirection {
            fd,
            direction,
            target,
        })
    }
}
