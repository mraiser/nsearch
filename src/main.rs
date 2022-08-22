pub mod hashmask;
pub mod directoryindex;
pub mod mime_type;

use std::path::PathBuf;
use std::collections::HashMap;

use hashmask::*;
use directoryindex::*;

fn main() {
  let ok_chars = "abcdefghijklmnopqrstuvwxyz0123456789.-_";
  let mask = HashMask::new(ok_chars, 3, 1.0);

  let s = "hello world";
  let mut ba1 = mask.empty_bit_array();
  mask.evaluate(&mut ba1, s);

  let s = "zebra world basket donkey hello magic";
  let mut ba2 = mask.empty_bit_array();
  mask.evaluate(&mut ba2, s);

  assert_eq!(true, HashMask::and_equals(&ba1, &ba2));

  let mut ba2 = mask.empty_bit_array();
  mask.evaluate_file(&mut ba2, PathBuf::from("src/main.rs"));

  assert_eq!(true, HashMask::and_equals(&ba1, &ba2));

  let mut ba2 = mask.empty_bit_array();
  mask.evaluate_file(&mut ba2, PathBuf::from("src/hashmask.rs"));

  assert_eq!(false, HashMask::and_equals(&ba1, &ba2));
  
  let dir = PathBuf::from("src");
  let work_dir = PathBuf::from("index");
  let filter = |path: PathBuf| -> bool {
    let name = path.file_name().unwrap().to_str().unwrap();
    !name.starts_with(".")
  };
  
  let di = DirectoryIndex::new(dir, work_dir, 3, 1.0, ok_chars.to_string(), filter, true, 2000000, HashMap::new());
  di.index(false);

  let x = "Found";
  let mut found = move |path: PathBuf| -> () {
    println!("{}: {:?}", x, path);
  };
  
  di.search("sapphire", &mut found, true);
}
