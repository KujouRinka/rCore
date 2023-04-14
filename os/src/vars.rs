extern "C" {
  pub fn stext();
  pub fn etext();
  pub fn srodata();
  pub fn erodata();
  pub fn sdata();
  pub fn edata();
  pub fn boot_stack_top();
  pub fn boot_stack_lower_bound();
  pub fn sbss_with_stack();
  pub fn sbss();
  pub fn ebss();
  pub fn ekernel();
  pub fn strampoline();
}
