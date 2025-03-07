use std::io;

pub fn input_read() -> String {
    let mut command = String::new();
    io::stdin()
        .read_line(&mut command)
        .expect("Read line failed");

    command
}
