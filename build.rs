fn main() {
    println!("cargo:rustc-link-lib=dylib=readline");
    println!("cargo:rustc-link-lib=dylib=ncurses"); // Sometimes required for readline
}
