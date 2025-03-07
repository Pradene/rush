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

    pub fn execute(&self) -> Result<bool, String> {
        match self {
            Command::Simple { .. } => {
                let mut cmd = self.build_std_command()?;
                let status = cmd.status().map_err(|e| e.to_string())?;
                Ok(status.success())
            }

            Command::Binary {
                left,
                right,
                operator,
            } => match operator {
                Operator::Pipe => {
                    let mut left_cmd = left
                        .build_std_command()?
                        .stdout(Stdio::piped())
                        .spawn()
                        .map_err(|e| e.to_string())?;

                    let left_output = left_cmd
                        .stdout
                        .take()
                        .ok_or("Failed to capture left command output")?;

                    let mut right_cmd = right
                        .build_std_command()?
                        .stdin(Stdio::from(left_output))
                        .spawn()
                        .map_err(|e| e.to_string())?;

                    let left_status = left_cmd.wait().map_err(|e| e.to_string())?;
                    let right_status = right_cmd.wait().map_err(|e| e.to_string())?;

                    Ok(left_status.success() && right_status.success())
                }
                Operator::And => {
                    if left.execute()? {
                        right.execute()
                    } else {
                        Ok(false)
                    }
                }
                Operator::Or => {
                    if left.execute()? {
                        Ok(true)
                    } else {
                        right.execute()
                    }
                }
                Operator::Semicolon => {
                    let _ = left.execute()?;
                    right.execute()
                }
                Operator::Background => {
                    left.build_std_command()?
                        .spawn()
                        .map_err(|e| e.to_string())?;
                    right.execute()
                }
            },
        }
    }
}
