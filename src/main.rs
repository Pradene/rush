use rush::input::*;
use rush::lexer::{Lexer, Token};
use rush::prompt::*;

fn main() {
    prompt_print();

    let command = input_read();
    println!("command: {}", command);

    let mut lexer = Lexer::new(command);

    loop {
        let token = lexer.next_token();
        println!("{:?}", token);
        if token == Token::EOF {
            break;
        }
    }
}
