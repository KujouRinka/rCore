use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::println;

pub fn get_num_app() -> usize {
  extern "C" {
    fn _num_app();
  }
  unsafe { (_num_app as *const usize).read_volatile() }
}

pub fn get_app_data(app_id: usize) -> &'static [u8] {
  extern "C" { fn _num_app(); }
  let num_app_ptr = _num_app as *const usize;
  let num_app = get_num_app();
  assert!(app_id < num_app);
  let app_start = unsafe {
    core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
  };
  // Copy app bytes from src to dst
  unsafe {
    core::slice::from_raw_parts(
      app_start[app_id] as *const u8,
      app_start[app_id + 1] - app_start[app_id],
    )
  }
}

lazy_static! {
  static ref APP_NAMES: Vec<&'static str> = {
    extern "C" { fn _app_names(); }
    let mut start = _app_names as usize as *const u8;
    let app_num = get_num_app();
    let mut v = Vec::new();
    unsafe {
      for _ in 0..app_num {
        let mut end = start;
        while end.read_volatile() != b'\0' {
          end = end.add(1);
        }
        let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
        let str = core::str::from_utf8(slice).unwrap();
        v.push(str);
        start = end.add(1);
      }
    }
    v
  };
}

pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
  APP_NAMES.iter()
    .enumerate()
    .find(|(_, x)| {
      name == **x
    }).map(|(i, _)| get_app_data(i))
}

pub fn list_apps() {
  println!("/**** APPS ****");
  for app_name in APP_NAMES.iter() {
    println!("{}", app_name);
  }
  println!("**************/");
}
