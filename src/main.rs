use rush::input::*;
use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::prompt::*;

fn main() {
    loop {
        prompt_print();
        let input = input_read();

        // println!("input: {}", input);

        let lexer = Lexer::new(input);
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
