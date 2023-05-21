use crate::mm::translated_byte_buffer;
use crate::print;
use crate::sbi::console_getchar;
use crate::task::{get_current_token, yield_};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
  match fd {
    FD_STDIN => {
      assert_eq!(len, 1, "Only support len = 1 in sys_read!");
      let mut c: usize;
      loop {
        c = console_getchar();
        if c == 0 {
          yield_();
          continue;
        } else {
          break;
        }
      }
      let ch = c as u8;
      let mut buffers = translated_byte_buffer(get_current_token(), buf, len);
      unsafe {
        buffers[0].as_mut_ptr().write_volatile(ch);
      }
      len as isize
    }
    _ => {
      panic!("Unsupported fd in sys_read: {}", fd);
    }
  }
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
  match fd {
    FD_STDOUT => {
      let buffers = translated_byte_buffer(get_current_token(), buf, len);
      for buffer in buffers {
        // TODO: fix malicious input
        print!("{}", core::str::from_utf8(buffer).unwrap());
      }
      len as isize
    }
    _ => {
      panic!("Unsupported fd in sys_write: {}", fd);
    }
  }
}
