#![no_std]
#![no_main]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("unimp", options(noreturn));
    }
}

fn pre_mul(a: u32, b: u32) -> u32 {
    if a > b {
        a * b
    } else  if b != 0 {
        a / b
    } else {
        a - b
    }
}

// Call the host function
#[polkavm_derive::polkavm_import]
extern "C" {
    fn get_third_number() -> u32;
}

#[polkavm_derive::polkavm_export]
extern "C" fn add_numbers(a: u32, b: u32) -> u32 {
    let c = pre_mul(a, b);
    a + b + c + unsafe { get_third_number() }
}

#[polkavm_derive::polkavm_export]
extern "C" fn sub_numbers(a: u32, b: u32) -> u32 {
    a - b
}

#[polkavm_derive::polkavm_export]
extern "C" fn mul_numbers(a: u32, b: u32) -> u32 {
    a * b
}
