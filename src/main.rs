use rush::input::*;
use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::prompt::*;

fn main() {
    prompt_print();

    let command = input_read();
    println!("command: {}", command);

    let lexer = Lexer::new(command);
    let mut parser = Parser::new(lexer);
    let command = parser.parse();

    println!("{:#?}", command);
}
