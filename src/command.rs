pub enum Command {
    Simple {
        executable: String,
        args: Vec<String>,
        redirects: Vec<Redirection>,
    },

    Pipeline {
        left: Box<Command>,
        right: Box<Command>,
    },

    List {
        commands: Vec<Command>,
        separators: Vec<ListSeparator>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListSeparator {
    Semicolon,  // `;`
    Background, // `&`
    And,        // `&&`
    Or,         // `||`
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
    Overwrite,      // `>`
    Append,         // `>>`
    Input,          // `<`
    HereDoc,        // `<<`
    DuplicateIn,    // `<&`
    DuplicateOut,   // `<&`
}
