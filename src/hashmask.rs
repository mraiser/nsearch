use bitvec::prelude::*;
use std::path::PathBuf;
use std::io::Read;
use std::fs::*;


pub struct HashMask {
  ok_chars: [u8; 256],
  sequence_length: u8,
  compression: f32,
  num_chars: u8,
}

impl HashMask {
  pub fn new(chars: &str, len: u8, compression: f32) -> HashMask {
    let chars = chars.as_bytes();
    let n = chars.len();
    let mut numchars = compression as u8;
    let mut i = 0;
    let mut oc = [0; 256];
    while i < n {
      let c = chars[i];
      oc[c as usize] = (numchars as f32 / compression) as u8;
      numchars += 1;
      i += 1;
    }
    
    let oval:u8 = numchars;
    numchars = (numchars as f32 / compression) as u8;
    if ((numchars as f32 * compression) as u8) < oval { numchars += 1; }
    
    HashMask {
      ok_chars: oc,
      sequence_length: len,
      compression: compression,
      num_chars: numchars,
    }
  }
  
  pub fn empty_bit_array(&self) -> BitVec<u8> {
    let v = vec![0u8; self.get_number_of_bytes()];
    v.view_bits::<Lsb0>().to_owned()
  }
  
  pub fn evaluate(&self, bitset: &mut BitVec<u8>, s:&str) {
    let sa:Vec<&str> = s.split(" ").collect();
    let n = sa.len();
    let mut i = 0;
    while i<n {
      if sa[i].len() as u8 >= self.sequence_length {
        self.set(bitset, sa[i]);
      }
      i += 1;
    }
  }
  
  pub fn evaluate_file(&self, bitset: &mut BitVec<u8>, path:PathBuf) {
    let n = metadata(&path).unwrap().len();
    let mut f = File::open(&path).unwrap();
    let mut buffer = [0; 1024];
    let mut i = 0;
    let mut remainder = "".to_string();
    while i < n {
      let nn = f.read(&mut buffer).unwrap();
      i += nn as u64;
      let s = remainder.to_owned() + std::str::from_utf8(&buffer[0..nn]).unwrap();
      let len = s.len();
      let ssl = self.sequence_length as usize;
      if len > ssl {
        self.evaluate(bitset, &s);
        remainder = s[len-(ssl-1)..].to_string();
      }
      else {
        remainder += &s;
      }
    }
  }
  
  pub fn set(&self, bitset: &mut BitVec<u8>, s:&str) {
    let s = s.to_lowercase();
    let s = s.as_bytes();
    let mut n = s.len();
    if n < self.sequence_length as usize { 
      panic!("Query string must be {} or more characters long", self.sequence_length); 
    }
    n -= (self.sequence_length-1) as usize;
    let mut i = 0;
    let ssl = self.sequence_length as usize;
    while i < n {
      self.set_bit(bitset, &s[i..i+ssl]);
      i += 1;
    }
  }
  
  pub fn set_bit(&self, bitset: &mut BitVec<u8>, ba:&[u8]) {
    let mut val:usize = 0;
    let mut i:usize = 0;
    let ssl = self.sequence_length as usize;
    let snc = self.num_chars as usize;
    while i < ssl {
      let b = ba[i];
      val += (self.ok_chars[b as usize] as usize) * snc.pow((ssl-1-i) as u32);
      i += 1;
    }
    bitset.set(val, true);
  }
  
  #[allow(dead_code)]
  pub fn get_compression(&self) -> f32 {
    self.compression
  }
  
  #[allow(dead_code)]
  pub fn get_sequence_length(&self) -> u8 {
    self.sequence_length
  }
  
  pub fn get_number_of_bits(&self) -> usize {
    (self.num_chars as usize).pow(self.sequence_length as u32)
  }
  
  pub fn get_number_of_bytes(&self) -> usize {
    let numbits = self.get_number_of_bits() as usize;
    let mut numbytes: usize = numbits / 8;
    if numbytes * 8 < numbits { numbytes += 1 }
    numbytes
  }
  
  pub fn and_equals(bs1: &BitVec<u8>, bs2: &BitVec<u8>) -> bool {
    for i in bs1.iter_ones() {
      if !bs2.get(i).unwrap() { return false; };
    }
    true
  }
}
