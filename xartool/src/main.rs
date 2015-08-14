mod xar;

use std::fs::File;
use std::path::Path;

fn main() {
    let path_str = std::env::args().nth(1).unwrap();
    let path = Path::new(&path_str);

    let file = File::open(path).unwrap();
    let mut archive = xar::Archive::open(file).unwrap();
    match archive.verify() {
        Ok(_) => { println!("Archive verified") },
        Err(err) => { println!("Archive verification failed: {}", err) }
    }
}
