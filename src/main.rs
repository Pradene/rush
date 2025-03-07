use rush::input::*;
use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::prompt::*;

fn main() {
    prompt_print();

    let input = input_read();
    println!("input: {}", input);

    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let command = parser.parse().unwrap();

    println!("{:#?}", command);

    let _ = command.execute();
}
