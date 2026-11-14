// A global_asm! in AT&T syntax with `fnstenv`, `stmxcsr` and `call sym@PLT` (the shape of
// crash-context's getcontext): one instruction the assembler does not know leaves the whole
// template out, and its symbol undefined.
#![cfg(all(target_arch = "x86_64", target_os = "linux"))]

std::arch::global_asm! {
    ".text",
    ".global relax_test_fpenv_pid",
    ".type relax_test_fpenv_pid, @function",
    "relax_test_fpenv_pid:",
    "fnstenv (%rdi)",
    "fldenv (%rdi)",
    "stmxcsr 28(%rdi)",
    "subq $8, %rsp",
    "call getpid@PLT",
    "addq $8, %rsp",
    "ret",
    ".size relax_test_fpenv_pid, . - relax_test_fpenv_pid",
    options(att_syntax)
}

extern "C" {
    fn relax_test_fpenv_pid(env: *mut u8) -> i32;
}

#[test]
fn the_x87_environment_mxcsr_and_a_plt_call() {
    let mut env = [0xAAu8; 32];
    let pid = unsafe { relax_test_fpenv_pid(env.as_mut_ptr()) };
    assert_eq!(pid as u32, std::process::id());
    // the 28-byte protected-mode environment starts with the control word; Linux starts a
    // thread with 0x037F (all exceptions masked, 64-bit precision)
    assert_eq!(u16::from_le_bytes([env[0], env[1]]), 0x037F);
    #[allow(deprecated)]
    let mxcsr = unsafe { std::arch::x86_64::_mm_getcsr() };
    assert_eq!(u32::from_le_bytes([env[28], env[29], env[30], env[31]]) & 0xFFC0, mxcsr & 0xFFC0);
}
