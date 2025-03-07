use crate::command::RedirectType;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String),
    SingleQuoted(String),       // 'text'
    DoubleQuoted(String),       // "text"
    Semicolon,                  // ;
    DoubleSemicolon,            // ;;
    Pipe,                       // |
    And,                        // &&
    Or,                         // ||
    Background,                 // &
    RedirectType(RedirectType), // >, >>, <, etc.
    LParen,                     // (
    RParen,                     // )
    EOF,                        // End of input
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: String) -> Lexer {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.input[self.position].is_whitespace() {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<&char> {
        self.input.get(self.position)
    }

    fn consume(&mut self) {
        self.position += 1;
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        if self.position >= self.input.len() {
            return Token::EOF;
        }

        let c = self.peek();
        match c {
            Some(&';') => self.handle_semicolon(),
            Some(&'|') => self.handle_pipe(),
            Some(&'&') => self.handle_ampersand(),
            Some(&'>') => self.handle_redirect_out(),
            Some(&'<') => self.handle_redirect_in(),
            Some(&'(') => self.handle_parentheses(),
            Some(&')') => self.handle_parentheses(),
            Some(&'\'') => self.read_single_quoted(),
            Some(&'"') => self.read_double_quoted(),
            Some(_) => self.read_word(),
            _ => panic!("Wrong command"),
        }
    }

    fn handle_semicolon(&mut self) -> Token {
        self.consume();
        if self.peek() == Some(&';') {
            self.consume();
            Token::DoubleSemicolon
        } else {
            Token::Semicolon
        }
    }

    fn handle_pipe(&mut self) -> Token {
        self.consume();
        if self.peek() == Some(&'|') {
            self.consume();
            Token::Or
        } else {
            Token::Pipe
        }
    }

    fn handle_ampersand(&mut self) -> Token {
        self.consume();
        if self.peek() == Some(&'&') {
            self.consume();
            Token::And
        } else {
            Token::Background
        }
    }

    fn handle_redirect_in(&mut self) -> Token {
        self.consume();
        if self.peek() == Some(&'<') {
            self.consume();
            Token::RedirectType(RedirectType::HereDoc)
        } else {
            Token::RedirectType(RedirectType::Input)
        }
    }

    fn handle_redirect_out(&mut self) -> Token {
        self.consume();
        if self.peek() == Some(&'>') {
            self.consume();
            Token::RedirectType(RedirectType::Append)
        } else {
            Token::RedirectType(RedirectType::Overwrite)
        }
    }

    fn handle_parentheses(&mut self) -> Token {
        if self.peek() == Some(&'(') {
            self.consume();
            Token::LParen
        } else {
            self.consume();
            Token::RParen
        }
    }

    fn read_single_quoted(&mut self) -> Token {
        let content = self.read_quoted('\'');
        Token::SingleQuoted(content)
    }

    fn read_double_quoted(&mut self) -> Token {
        let content = self.read_quoted('"');
        Token::DoubleQuoted(content)
    }

    fn read_quoted(&mut self, quote: char) -> String {
        let mut content = String::new();
        self.consume(); // Skip opening quote
        while self.position < self.input.len() {
            let c = self.input[self.position];
            if c == quote {
                self.consume(); // Skip closing quote
                break;
            } else if c == '\\' && quote == '"' {
                // Handle escapes in double quotes (e.g., \", \$)
                self.consume();
                if self.position < self.input.len() {
                    content.push(self.input[self.position]);
                    self.consume();
                }
            } else {
                content.push(c);
                self.consume();
            }
        }
        content
    }

    fn read_word(&mut self) -> Token {
        let mut word = String::new();

        while self.position < self.input.len() {
            let c = self.input[self.position];
            if c.is_whitespace() || self.is_operator(c) {
                break;
            }

            word.push(c);
            self.consume();
        }

        Token::Word(word)
    }

    fn is_operator(&self, c: char) -> bool {
        matches!(c, ';' | '|' | '&' | '>' | '<' | '(' | ')')
    }
}
