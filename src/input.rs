use libc::c_char;
use std::ffi::{CStr, CString};

extern "C" {
    fn readline(prompt: *const c_char) -> *mut c_char;
    fn add_history(line: *const c_char);
    fn free(ptr: *mut c_char);
}

pub fn input_read(prompt: String) -> Option<String> {
    let prompt = CString::new(prompt).unwrap();

    unsafe {
        let input = readline(prompt.as_ptr());

        if input.is_null() {
            None
        } else {
            add_history(input);
            let command = CStr::from_ptr(input).to_string_lossy().into_owned();
            free(input);
            Some(command)
        }
    }
}
