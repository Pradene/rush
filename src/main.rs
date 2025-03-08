use rush::input::input_read;
use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::prompt::prompt;

use std::ffi::CString;

use libc::c_int;
use libc::{exit, getpid, getsid, setsid, signal, write};
use libc::{SIGINT, SIGPIPE, SIGQUIT, SIGTTIN, SIGTTOU, SIG_DFL, SIG_IGN, STDOUT_FILENO};

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
        if getsid(0) != getpid() {
            setsid();
        }

        signal(SIGTTOU, SIG_IGN);
        signal(SIGTTIN, SIG_IGN);

        rl_catch_signals = 0;
        signal(SIGINT, sigint_handler as usize);
        signal(SIGPIPE, SIG_DFL);
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
