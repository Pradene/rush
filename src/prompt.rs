use std::io::{self, Write};

pub fn prompt_print() -> () {
    print!("> ");
    io::stdout().flush().expect("Flush failed");
}
