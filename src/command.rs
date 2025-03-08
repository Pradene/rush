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
                    let mut left_cmd = match left.build_std_command() {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            eprintln!("{}", e);
                            return 127;
                        }
                    };

                    let mut left_child = match left_cmd.stdout(Stdio::piped()).spawn() {
                        Ok(child) => child,
                        Err(e) => {
                            eprintln!("Failed to execute left command: {}", e);
                            return 1;
                        }
                    };

                    let left_output = match left_child.stdout.take() {
                        Some(output) => output,
                        None => {
                            eprintln!("Failed to capture left command output");
                            return 1;
                        }
                    };

                    let mut right_cmd = match right.build_std_command() {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            eprintln!("{}", e);
                            return 127;
                        }
                    };

                    let mut right_child = match right_cmd.stdin(Stdio::from(left_output)).spawn() {
                        Ok(child) => child,
                        Err(e) => {
                            eprintln!("Failed to execute right command: {}", e);
                            return 1;
                        }
                    };

                    let _ = left_child.wait();
                    let right_status = match right_child.wait() {
                        Ok(status) => status,
                        Err(e) => {
                            eprintln!("Failed to get right command status: {}", e);
                            return 1;
                        }
                    };

                    right_status.code().unwrap_or(1)
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
                Operator::Background => {
                    let mut left_cmd = match left.build_std_command() {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            eprintln!("{}", e);
                            return 127;
                        }
                    };

                    match left_cmd.spawn() {
                        Ok(_) => right.execute(),
                        Err(e) => {
                            eprintln!("Failed to spawn background process: {}", e);
                            return 1;
                        }
                    }
                }
            },
        }
    }
}
