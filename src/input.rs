use std::io;

pub fn input_read() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Read line failed");

    input
}
