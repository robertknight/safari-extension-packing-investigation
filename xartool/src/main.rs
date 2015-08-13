mod xar;

use std::fs::File;
use std::path::Path;

fn main() {
    println!("Hello, world!");
    let path = Path::new("archive.xar");
    let file = File::open(path).unwrap();
    let archive = xar::Archive::open(file);
    println!("archive header: {:?}", archive.unwrap())
}
