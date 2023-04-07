#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::get_taskinfo;

#[no_mangle]
fn main() -> i32 {
  println!("Current task id is {}", get_taskinfo());
  0
}
