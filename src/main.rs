use rush::input::*;
use rush::lexer::Lexer;
use rush::parser::Parser;

fn main() {
    loop {
        let prompt = String::from("> ");
        let input = input_read(prompt);

        if input.is_none() {
            continue;
        }

        // println!("input: {}", input);

        let lexer = Lexer::new(input.unwrap());
        let command = Parser::new(lexer).parse();

        // println!("{:#?}", command);

        match command {
            Ok(command) => {
                let _ = command.execute();
            }
            Err(e) => eprintln!("Parsing error: {}", e),
        }
    }
}
