mod xar;

use std::fs::File;
use std::path::Path;

fn main() {
    println!("Hello, world!");
    let path = Path::new("archive.xar");
    let file = File::open(path).unwrap();
    let mut archive = xar::Archive::open(file).unwrap();
    match archive.verify() {
        Ok(_) => { println!("Archive verified") },
        Err(err) => { println!("Archive verification failed: {}", err) }
    }
}
