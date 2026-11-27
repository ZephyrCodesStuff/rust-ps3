//! Open PSID lookup.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CellSsOpenPsid {
    pub high: u64,
    pub low: u64,
}

assert_layout!(CellSsOpenPsid, 16, 8, low = 8);

define_syscalls! {

    pub unsafe fn sys_ss_get_open_psid(psid: *mut CellSsOpenPsid) -> i32 = SYS_SS_GET_OPEN_PSID;
}
