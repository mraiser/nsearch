use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs::*;
use std::io::Read;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use bitvec::prelude::*;
use std::time::SystemTime;

use crate::hashmask::*;
use crate::mime_type::mime_type;

pub type FileFilter = fn(PathBuf) -> bool;
pub type FileFound = dyn FnMut(PathBuf);

const DIRNAME: &str = "8cee109e-8684-43a1-ada5-eca55e4ba55d";

pub struct DirectoryIndex {
  dir: PathBuf,
  work_dir: PathBuf,
  hash_mask: HashMask,
  filter: FileFilter,
  index_content: bool,
  max_file_size: usize,
  excluded: HashMap<PathBuf, bool>,
}

impl DirectoryIndex {
  pub fn new( dir: PathBuf,
              work_dir: PathBuf,
              sequence_length: u8,
              compression: f32,
              ok_chars: String,
              filter: FileFilter,
              index_content: bool,
              max_file_size: i32,
              excluded: HashMap<PathBuf, bool>) -> DirectoryIndex {
    let s = dir.to_str().unwrap();
    let h = calculate_hash(&s);
    let wd = "x".to_string() + &h.to_string();
    let mfs;
    if max_file_size == -1 { mfs = std::usize::MAX; }
    else { mfs = max_file_size as usize; }
    let mask = HashMask::new(&ok_chars, sequence_length, compression);
    DirectoryIndex {
      dir: dir,
      work_dir: work_dir.join(wd),
      hash_mask: mask,
      filter: filter,
      index_content: index_content,
      max_file_size: mfs,
      excluded: excluded,
    }
  }
  
  pub fn index(&self, rebuild: bool) -> bool {
    if rebuild { remove_dir_all(self.work_dir.to_owned()).unwrap(); }
    return self.index_dir(self.dir.to_owned());
  }
  
  pub fn index_dir(&self, f: PathBuf) -> bool {
    let f2 = self.get_work_file(f.to_owned());
    let mut bs = self.hash_mask.empty_bit_array();
    return self.index_file(f, f2, &mut bs);
  }
  
  fn get_work_file(&self, f: PathBuf) -> PathBuf {
    let s1: String = f.canonicalize().unwrap().to_str().unwrap().to_string();
    let s2: String = self.dir.canonicalize().unwrap().to_str().unwrap().to_string();
    if !s1.starts_with(&s2) { panic!("File {} is not inside the indexed directory {}", s1, s2); }
    
    let s3 = &s1[s2.len()..];
    self.work_dir.join(s3)
  }
  
  fn index_file(&self, f: PathBuf, w: PathBuf, ba: &mut BitVec<u8>) -> bool {
    if symlink_metadata(&f).unwrap().file_type().is_symlink() {
      return false;
    }
    
    let dw = w.join(DIRNAME);
    let m = metadata(&f).unwrap();
    let isdir = m.is_dir();
    let exists = w.exists();
    let mut changed;
    if exists {
      let lm;
      if isdir {
        if dw.exists() {
          lm = metadata(&dw).unwrap().modified().unwrap();
        }
        else {
          lm = SystemTime::UNIX_EPOCH;
        }
      }
      else {
        lm = metadata(&w).unwrap().modified().unwrap();
      }
      changed = lm < m.modified().unwrap();
    }
    else {
      changed = true;
    }
    
    if !isdir && exists && !changed {
      let ba2 = read_file(w);
      *ba |= ba2;
      return false;
    }
    
    let name = f.file_name().unwrap().to_str().unwrap();
    let mut ba3 = self.hash_mask.empty_bit_array();
    self.hash_mask.evaluate(&mut ba3, name);
    
    let parent = w.parent().unwrap();
    create_dir_all(parent).unwrap();
    
    if isdir {
      let cp = f.canonicalize().unwrap();
      if self.excluded.contains_key(&cp) {
        if dw.exists() {
          let ba2 = read_file(dw);
          *ba |= ba2;
        }
        return false;
      }
      
      create_dir_all(&w).unwrap();
      for file in read_dir(&f).unwrap() {
        let path = file.unwrap().path();
        if (self.filter)(path.to_owned()) {
          let name2 = (&path).file_name().unwrap().to_str().unwrap().to_owned();
          changed |= self.index_file(path, (&w).join(name2).to_owned(), &mut ba3);
        }
      }
      
      if changed {
        let x:Vec<u8> = ba3.to_owned().into_vec();
        let mut file = File::create(dw.to_owned()).unwrap();
        file.write_all(&x).unwrap();
      }
    }
    else {
      let typ = mime_type(name.to_owned());
      let n = m.len();
      if self.index_content && 
        n<self.max_file_size as u64 && (
          typ == "application/x-javascript" ||
          typ == "application/json" || (
            !typ.starts_with("audio") &&
            !typ.starts_with("video") &&
            !typ.starts_with("image") &&
            !typ.starts_with("application"))) 
      {
        self.hash_mask.evaluate_file(&mut ba3, f);
        let x:Vec<u8> = ba3.to_owned().into_vec();
        let mut file = File::create(w.to_owned()).unwrap();
        file.write_all(&x).unwrap();
        changed = true;
      }
      else {
        changed = false;
      }
    }
    
    *ba |= ba3;
    return changed
  }
  
  pub fn search(&self, query: &str, v: &mut FileFound, search_content: bool) {
    let mut bitset = self.hash_mask.empty_bit_array();
    self.hash_mask.evaluate(&mut bitset, query);
    self.search_file(self.dir.to_owned(), self.work_dir.to_owned(), query, &mut bitset, v, search_content);
  }
  
  #[allow(dead_code)]
  pub fn search_dir(&self, f: PathBuf, query: &str, v: &mut FileFound, search_content: bool) {
    let mut bitset = self.hash_mask.empty_bit_array();
    self.hash_mask.evaluate(&mut bitset, query);
    self.search_file(f.to_owned(), self.get_work_file(f), query, &mut bitset, v, search_content);
  }
  
  pub fn search_file(&self, f: PathBuf, w: PathBuf, query: &str, bs: &mut BitVec<u8>, v: &mut FileFound, search_content: bool) {
    let m = metadata(&f).unwrap();
    let isdir = m.is_dir();
    let fname = f.file_name().unwrap().to_str().unwrap();
    let sa:Vec<&str> = query.split(" ").collect();
    let n = sa.len();
    let mut i = 0;
    let mut m = 0;
    while i<n {
      if fname.contains(sa[i]) {
        m += 1;
      }
      i += 1;
    }
    if n == m {
      (v)(f.to_owned());
      if !isdir {
        return;
      }
    }
    
    let dw = w.join(DIRNAME);
    if (isdir && dw.exists()) || (search_content && w.exists()) {
      let p;
      if isdir { p = dw.to_owned(); }
      else { p = w.to_owned(); }
      let bs2 = read_file(p);
      if HashMask::and_equals(bs, &bs2) {
        if isdir {
          for file in read_dir(&f).unwrap() {
            let path = file.unwrap().path();
            if (self.filter)(path.to_owned()) {
              let name2 = (&path).file_name().unwrap().to_str().unwrap().to_owned();
              self.search_file(path, w.join(name2), query, bs, v, search_content);
            }
          }
        }
        else {
          let mut hits = HashMap::<&str, bool>::new();
          let mut i = 0;
          while i<n {
            hits.insert(sa[i], false);
            i += 1;
          }
          let n = hits.len();
          let mut m = 0;
          let file = File::open(f.to_owned()).unwrap();
          let lines = BufReader::new(file).lines();
          for line in lines {
            if let Ok(ip) = line {
              let ip = ip.to_lowercase();
              let mut i = 0;
              while i<n {
                if ip.contains(sa[i]) {
                  let b = hits.get(sa[i]).unwrap();
                  if !b {
                    hits.insert(sa[i], true);
                    m += 1;
                    if m == n { break; }
                  }
                }
                i += 1;
              }
              
              if m == n {
                (v)(f);
                break;
              }
            }
          }
          
          // if (m != n) v.visitFileFailed(f, null); //System.out.println("False positive: "+f);
          
        }
      }
    }
  }
}

fn read_file(f: PathBuf) -> BitVec<u8> {
  let mut buf = Vec::<u8>::new();
  let mut ff = File::open(f).unwrap();
  let _n = ff.read_to_end(&mut buf).unwrap();
  buf.view_bits::<Lsb0>().to_owned()
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
  let mut s = DefaultHasher::new();
  t.hash(&mut s);
  s.finish()
}
