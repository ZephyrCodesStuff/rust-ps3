/// LV2 start/stop command block. `entry` and `entry2` are guest addresses
/// encoded as 64-bit values. An absent callback is returned as `u64::MAX`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct SysPrxStartStopModuleOption {
    pub size: u64,
    pub command: u64,
    pub entry: u64,
    pub result: u64,
    pub entry2: u64,
}

impl SysPrxStartStopModuleOption {
    /// Prepares the first phase of a module start or stop operation.
    pub const fn begin() -> Self {
        Self {
            size: core::mem::size_of::<Self>() as u64,
            command: 1,
            entry: u64::MAX,
            result: 0,
            entry2: u64::MAX,
        }
    }
}

assert_layout!(SysPrxStartStopModuleOption, 40, 8, entry = 16, entry2 = 32);

define_syscalls! {
    pub(crate) unsafe fn _sys_prx_start_module(id: u32, flags: u64, option: *mut SysPrxStartStopModuleOption) -> i32 = _SYS_PRX_START_MODULE;
    pub(crate) unsafe fn _sys_prx_stop_module(id: u32, flags: u64, option: *mut SysPrxStartStopModuleOption) -> i32 = _SYS_PRX_STOP_MODULE;
}
