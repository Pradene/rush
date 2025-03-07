use rush::input::*;
use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::prompt::*;

fn main() {
    loop {
        prompt_print();
        let input = input_read();

        let lexer = Lexer::new(input);
        let command = Parser::new(lexer).parse();

        // println!("input: {}", input);
        // println!("{:#?}", command);

        match command {
            Ok(command) => match command.execute() {
                Ok(_) => continue,
                Err(e) => eprintln!("{}", e),
            },
            Err(e) => eprintln!("Parsing error: {}", e),
        }
    }
}
