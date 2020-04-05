// #![windows_subsystem = "windows"]

mod interface;

use crate::interface::*;

fn main() {
  unsafe {
    println!("init");
    assert_eq!(cef_init(), 0);

    println!("free");
    assert_eq!(cef_free(), 0);
  }
}
