use crate::lexer::{Lexer, Token};
use crate::command::{
    Command,
    ListSeparator,
    RedirectTarget,
    RedirectType,
    Redirection
};

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
        self.parse_list()
    }

    fn parse_list(&mut self) -> Result<Command, String> {
        let mut commands = vec![];
        let mut separators = vec![];

        commands.push(self.parse_pipeline()?);

        while matches!(&self.current_token, 
            Token::Semicolon | Token::And | Token::Or | Token::Background
        ) {
            let sep = match self.current_token {
                Token::Semicolon => ListSeparator::Semicolon,
                Token::And => ListSeparator::And,
                Token::Or => ListSeparator::Or,
                Token::Background => ListSeparator::Background,
                _ => unreachable!(),
            };
            separators.push(sep);
            self.advance();
            
            commands.push(self.parse_pipeline()?);
        }

        if commands.len() == 1 {
            Ok(commands.remove(0))
        } else {
            Ok(Command::List { commands, separators })
        }
    }

    fn parse_pipeline(&mut self) -> Result<Command, String> {
        let mut left = self.parse_command()?;

        while self.current_token == Token::Pipe {
            self.advance(); // Consume '|'
            let right = self.parse_command()?;
            left = Command::Pipeline {
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_command(&mut self) -> Result<Command, String> {
        let mut words = vec![];
        let mut redirects = vec![];

        // Parse command words and redirections
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

        // Default file descriptors based on redirect type
        let (mut fd, direction) = match rt {
            RedirectType::Overwrite => (Some(1), RedirectType::Overwrite),
            RedirectType::Append => (Some(1), RedirectType::Append),
            RedirectType::DuplicateOut => (Some(1), RedirectType::DuplicateOut),
            RedirectType::Input => (Some(0), RedirectType::Input),
            RedirectType::DuplicateIn => (Some(0), RedirectType::DuplicateIn),
            RedirectType::HereDoc => (Some(0), RedirectType::HereDoc),
        };

        // Handle explicit file descriptors (e.g., 2>&1)
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
