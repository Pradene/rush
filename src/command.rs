use std::fs::{File, OpenOptions};
use std::os::unix::io::FromRawFd;
use std::process::{Command as StdCommand, Stdio};

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
    pub direction: RedirectType,
    pub target: RedirectTarget,
}

#[derive(Debug, Clone)]
pub enum RedirectTarget {
    File(String),        // e.g., `> file.txt`
    FileDescriptor(u32), // e.g., `2>&1`
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectType {
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
}

impl Command {
    fn build_std_command(&self) -> Result<StdCommand, String> {
        match self {
            Command::Simple {
                executable,
                args,
                redirects,
            } => {
                let mut cmd = StdCommand::new(executable);
                cmd.args(args);

                for redir in redirects {
                    let fd = redir.fd.unwrap_or(match redir.direction {
                        RedirectType::Input | RedirectType::DuplicateIn => 0,
                        _ => 1,
                    });

                    match &redir.target {
                        RedirectTarget::File(path) => {
                            let file = match redir.direction {
                                RedirectType::Overwrite => {
                                    File::create(path).map_err(|e| e.to_string())?
                                }
                                RedirectType::Append => OpenOptions::new()
                                    .append(true)
                                    .create(true)
                                    .open(path)
                                    .map_err(|e| e.to_string())?,
                                RedirectType::Input => {
                                    File::open(path).map_err(|e| e.to_string())?
                                }
                                _ => {
                                    return Err(format!(
                                        "Unsupported redirection: {:?}",
                                        redir.direction
                                    ))
                                }
                            };

                            match fd {
                                0 => cmd.stdin(file),
                                1 => cmd.stdout(file),
                                2 => cmd.stderr(file),
                                _ => return Err(format!("Unsupported file descriptor: {}", fd)),
                            };
                        }
                        RedirectTarget::FileDescriptor(target_fd) => {
                            match fd {
                                0 => cmd.stdin(unsafe { Stdio::from_raw_fd(*target_fd as i32) }),
                                1 => cmd.stdout(unsafe { Stdio::from_raw_fd(*target_fd as i32) }),
                                2 => cmd.stderr(unsafe { Stdio::from_raw_fd(*target_fd as i32) }),
                                _ => return Err(format!("Unsupported FD: {}", fd)),
                            };
                        }
                    }
                }

                Ok(cmd)
            }
            _ => Err("Complex commands need special handling".to_string()),
        }
    }

    pub fn execute(&self) -> i32 {
        match self {
            Command::Simple { .. } => {
                let mut cmd = match self.build_std_command() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return 127;
                    }
                };

                match cmd.status() {
                    Ok(status) => status.code().unwrap_or(1),
                    Err(e) => {
                        eprintln!("Execution error: {}", e);
                        return 1;
                    }
                }
            }

            Command::Binary {
                left,
                right,
                operator,
            } => match operator {
                Operator::Pipe => {
                    let mut pipe_fds = [-1; 2];
                    unsafe {
                        if libc::pipe(pipe_fds.as_mut_ptr()) != 0 {
                            eprintln!("Failed to create pipe");
                            return 1;
                        }
                    }

                    let (read_fd, write_fd) = (pipe_fds[0], pipe_fds[1]);

                    unsafe {
                        let left_pid = libc::fork();

                        if left_pid < 0 {
                            eprintln!("Fork failed");
                            libc::close(read_fd);
                            libc::close(write_fd);
                            return 1;
                        } else if left_pid == 0 {
                            libc::close(read_fd);

                            if libc::dup2(write_fd, 1) < 0 {
                                eprintln!("dup2 failed for stdout");
                                libc::exit(1);
                            }
                            libc::close(write_fd);

                            let exit_code = left.execute();
                            libc::exit(exit_code);
                        }

                        libc::close(write_fd);

                        let right_pid = libc::fork();

                        if right_pid < 0 {
                            eprintln!("Fork failed for right command");
                            libc::close(read_fd);
                            libc::waitpid(left_pid, std::ptr::null_mut(), 0);
                            return 1;
                        } else if right_pid == 0 {
                            if libc::dup2(read_fd, 0) < 0 {
                                eprintln!("dup2 failed for stdin");
                                libc::exit(1);
                            }
                            libc::close(read_fd);

                            let exit_code = right.execute();
                            libc::exit(exit_code);
                        }

                        libc::close(read_fd);

                        let mut status = 0;
                        let mut exit_code = 0;

                        libc::waitpid(left_pid, &mut status, 0);

                        libc::waitpid(right_pid, &mut status, 0);
                        if libc::WIFEXITED(status) {
                            exit_code = libc::WEXITSTATUS(status);
                        }

                        exit_code
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
                    let pid = libc::fork();

                    if pid < 0 {
                        eprintln!("Fork failed for background process");
                        1
                    } else if pid == 0 {
                        let exit_code = left.execute();
                        libc::exit(exit_code);
                    } else {
                        right.execute()
                    }
                },
            },
        }
    }
}
