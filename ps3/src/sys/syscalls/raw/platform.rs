//! Public GPIO calls.

pub const SYS_GPIO_LED_DEVICE_ID: u64 = 1;
pub const SYS_GPIO_DIP_SWITCH_DEVICE_ID: u64 = 2;

define_syscalls! {

    pub unsafe fn sys_gpio_set(device: u64, mask: u64, value: u64) -> i32 = SYS_GPIO_SET;
    pub unsafe fn sys_gpio_get(device: u64, value: *mut u64) -> i32 = SYS_GPIO_GET;
}
