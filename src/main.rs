use rush::input::input_read;
use rush::prompt::prompt;
use rush::lexer::Lexer;
use rush::parser::Parser;

use std::ffi::CString;

use libc::{exit, signal, write};
use libc::c_int;
use libc::{SIGPIPE, SIG_DFL, SIG_IGN, SIGINT, SIGQUIT, STDOUT_FILENO};

extern "C" {
    static mut rl_catch_signals: c_int;

    fn rl_on_new_line();
    fn rl_replace_line(text: *const i8, clear_undo: c_int);
    fn rl_redisplay();
}

extern "C" fn sigint_handler(_signum: c_int) {
    unsafe {
        write(STDOUT_FILENO, "\n".as_ptr() as *const _, 1);

        let s = CString::new("").unwrap();                
        rl_on_new_line();
        rl_replace_line(s.as_ptr(), 0);
        rl_redisplay();
    }
}

fn main() {
    unsafe {
        rl_catch_signals = 0;
        signal(SIGPIPE, SIG_DFL);
        signal(SIGINT, sigint_handler as usize);
        signal(SIGQUIT, SIG_IGN);
    }

    loop {
        let input = input_read(prompt());
        
        if input.is_none() {
            unsafe { exit(0) };
        }

        let input = input.unwrap();
        if input.trim().is_empty() {
            continue;
        }

        let lexer = Lexer::new(input);
        let command = Parser::new(lexer).parse();

        match command {
            Ok(command) => {
                let _ = command.execute();
            }
            Err(e) => eprintln!("Parsing error: {}", e),
        }
    }
}
