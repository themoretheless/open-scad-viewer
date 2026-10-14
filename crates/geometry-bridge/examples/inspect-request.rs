//! Run the native JSON geometry boundary against a request from stdin.
use std::io::{self, Read};
fn main() {
    let mut request = String::new();
    io::stdin().read_to_string(&mut request).unwrap();
    println!("{}", geometry_bridge::execute(&request));
}
