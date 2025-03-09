use std::ffi::CString;

use libc::{
    __errno_location, close, dup2, execvp, exit, fork, getpgrp, getpid, ioctl, open, pipe, setpgid,
    signal, tcsetpgrp, waitpid,
};
use libc::{c_char, c_int};
use libc::{
    EINTR, O_APPEND, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, SIGINT, SIGQUIT, SIG_DFL, TIOCSPGRP,
};
use libc::{WEXITSTATUS, WIFEXITED};

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Semicolon,  // `;`
    Background, // `&`
    And,        // `&&`
    Or,         // `||`
    Pipe,       // `|`
}

#[derive(Debug, Clone)]
pub struct Redirection {
    pub fd: Option<u32>,
    pub operator: RedirectOperator,
    pub target: RedirectTarget,
}

#[derive(Debug, Clone)]
pub enum RedirectTarget {
    File(String),        // e.g., `> file.txt`
    FileDescriptor(u32), // e.g., `2>&1`
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectOperator {
    Overwrite,    // `>`
    Append,       // `>>`
    Input,        // `<`
    HereDoc,      // `<<`
    DuplicateIn,  // `<&`
    DuplicateOut, // `>&`
}

#[derive(Debug, Clone)]
pub enum Command {
    Simple {
        executable: String,
        args: Vec<String>,
        redirects: Vec<Redirection>,
    },

    Binary {
        left: Box<Command>,
        right: Box<Command>,
        operator: Operator,
    },

    Group {
        group: Box<Command>,
    },
}

impl Command {
    fn redirect(&self) -> Result<(), String> {
        if let Command::Simple { redirects, .. } = self {
            for redirection in redirects {
                let fd = redirection.fd.unwrap_or(match redirection.operator {
                    RedirectOperator::Input | RedirectOperator::DuplicateIn => 0,
                    _ => 1,
                });

                match &redirection.target {
                    RedirectTarget::File(path) => {
                        let c_path = CString::new(path.as_str()).unwrap();
                        let mode = match redirection.operator {
                            RedirectOperator::Overwrite => O_WRONLY | O_CREAT | O_TRUNC,
                            RedirectOperator::Append => O_WRONLY | O_CREAT | O_APPEND,
                            RedirectOperator::Input => O_RDONLY,
                            _ => return Err("Unsupported redirection type".into()),
                        };

                        let target_fd = unsafe { open(c_path.as_ptr(), mode, 0o644) };
                        if target_fd < 0 {
                            return Err("Failed to open file".into());
                        }

                        unsafe { dup2(target_fd, fd as c_int) };
                        unsafe { close(target_fd) };
                    }
                    RedirectTarget::FileDescriptor(target_fd) => {
                        unsafe { dup2(*target_fd as c_int, fd as c_int) };
                    }
                }
            }
        }

        Ok(())
    }

    pub fn execute(&self) -> i32 {
        match self {
            Command::Simple {
                executable, args, ..
            } => {
                let c_exec = CString::new(executable.as_str()).unwrap();
                let mut c_args: Vec<CString> = args
                    .iter()
                    .map(|a| CString::new(a.as_str()).unwrap())
                    .collect();
                c_args.insert(0, c_exec.clone());

                let mut ptr_args: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
                ptr_args.push(std::ptr::null());

                unsafe {
                    let pid = fork();
                    if pid == 0 {
                        signal(SIGINT, SIG_DFL);
                        signal(SIGQUIT, SIG_DFL);

                        setpgid(0, 0);

                        tcsetpgrp(0, getpid());

                        if let Err(e) = self.redirect() {
                            eprintln!("Redirection error: {}", e);
                            exit(1);
                        }

                        execvp(c_exec.as_ptr(), ptr_args.as_ptr());
                        eprintln!("Execution failed");
                        exit(1);
                    } else if pid < 0 {
                        eprintln!("Fork failed");
                        return 1;
                    }

                    let shell_pgrp = getpgrp();

                    setpgid(pid, pid);
                    tcsetpgrp(0, pid);

                    let mut status = 0;
                    while waitpid(pid, &mut status, 0) < 0 {
                        if *__errno_location() != EINTR {
                            break;
                        }
                    }

                    let _ = tcsetpgrp(0, shell_pgrp);
                    ioctl(0, TIOCSPGRP, &shell_pgrp);

                    if WIFEXITED(status) {
                        WEXITSTATUS(status) as i32
                    } else {
                        1
                    }
                }
            }

            Command::Binary {
                left,
                right,
                operator,
            } => match operator {
                Operator::Pipe => {
                    let mut fds = [0; 2];
                    unsafe {
                        if pipe(fds.as_mut_ptr()) != 0 {
                            eprintln!("Pipe creation failed");
                            return 1;
                        }
                    }

                    let (read_end, write_end) = (fds[0], fds[1]);
                    let left_pid = unsafe { fork() };
                    if left_pid == 0 {
                        unsafe {
                            close(read_end);
                            dup2(write_end, 1);
                            close(write_end);
                            exit(left.execute());
                        }
                    }

                    let right_pid = unsafe { fork() };
                    if right_pid == 0 {
                        unsafe {
                            close(write_end);
                            dup2(read_end, 0);
                            close(read_end);
                            exit(right.execute());
                        }
                    }

                    unsafe {
                        close(read_end);
                        close(write_end);

                        let mut status = 0;
                        waitpid(left_pid, &mut status, 0);
                        waitpid(right_pid, &mut status, 0);

                        WEXITSTATUS(status) as i32
                    }
                }
                Operator::And => {
                    let left_code = left.execute();
                    if left_code == 0 {
                        right.execute()
                    } else {
                        left_code
                    }
                }
                Operator::Or => {
                    let left_code = left.execute();
                    if left_code == 0 {
                        left_code
                    } else {
                        right.execute()
                    }
                }
                Operator::Semicolon => {
                    let _ = left.execute();
                    right.execute()
                }
                Operator::Background => unsafe {
                    let pid = fork();

                    if pid < 0 {
                        eprintln!("Fork failed for background process");
                        1
                    } else if pid == 0 {
                        let exit_code = left.execute();
                        exit(exit_code);
                    } else {
                        right.execute()
                    }
                },
            },

            Command::Group { group } => group.execute(),
        }
    }
}
