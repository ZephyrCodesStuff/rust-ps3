//! Common CellOS status codes and their signed syscall representation.

use core::fmt;

pub const CELL_OK: i32 = 0;
pub const CELL_CANCEL: i32 = 1;

pub const CELL_EAGAIN: i32 = 0x8001_0001_u32 as i32;
pub const CELL_EINVAL: i32 = 0x8001_0002_u32 as i32;
pub const CELL_ENOSYS: i32 = 0x8001_0003_u32 as i32;
pub const CELL_ENOMEM: i32 = 0x8001_0004_u32 as i32;
pub const CELL_ESRCH: i32 = 0x8001_0005_u32 as i32;
pub const CELL_ENOENT: i32 = 0x8001_0006_u32 as i32;
pub const CELL_ENOEXEC: i32 = 0x8001_0007_u32 as i32;
pub const CELL_EDEADLK: i32 = 0x8001_0008_u32 as i32;
pub const CELL_EPERM: i32 = 0x8001_0009_u32 as i32;
pub const CELL_EBUSY: i32 = 0x8001_000a_u32 as i32;
pub const CELL_ETIMEDOUT: i32 = 0x8001_000b_u32 as i32;
pub const CELL_EABORT: i32 = 0x8001_000c_u32 as i32;
pub const CELL_EFAULT: i32 = 0x8001_000d_u32 as i32;
pub const CELL_ENOCHILD: i32 = 0x8001_000e_u32 as i32;
pub const CELL_ESTAT: i32 = 0x8001_000f_u32 as i32;
pub const CELL_EALIGN: i32 = 0x8001_0010_u32 as i32;
pub const CELL_EKRESOURCE: i32 = 0x8001_0011_u32 as i32;
pub const CELL_EISDIR: i32 = 0x8001_0012_u32 as i32;
pub const CELL_ECANCELED: i32 = 0x8001_0013_u32 as i32;
pub const CELL_EEXIST: i32 = 0x8001_0014_u32 as i32;
pub const CELL_EISCONN: i32 = 0x8001_0015_u32 as i32;
pub const CELL_ENOTCONN: i32 = 0x8001_0016_u32 as i32;
pub const CELL_EAUTHFAIL: i32 = 0x8001_0017_u32 as i32;
pub const CELL_ENOTMSELF: i32 = 0x8001_0018_u32 as i32;
pub const CELL_ESYSVER: i32 = 0x8001_0019_u32 as i32;
pub const CELL_EAUTHFATAL: i32 = 0x8001_001a_u32 as i32;
pub const CELL_EDOM: i32 = 0x8001_001b_u32 as i32;
pub const CELL_ERANGE: i32 = 0x8001_001c_u32 as i32;
pub const CELL_EILSEQ: i32 = 0x8001_001d_u32 as i32;
pub const CELL_EFPOS: i32 = 0x8001_001e_u32 as i32;
pub const CELL_EINTR: i32 = 0x8001_001f_u32 as i32;
pub const CELL_EFBIG: i32 = 0x8001_0020_u32 as i32;
pub const CELL_EMLINK: i32 = 0x8001_0021_u32 as i32;
pub const CELL_ENFILE: i32 = 0x8001_0022_u32 as i32;
pub const CELL_ENOSPC: i32 = 0x8001_0023_u32 as i32;
pub const CELL_ENOTTY: i32 = 0x8001_0024_u32 as i32;
pub const CELL_EPIPE: i32 = 0x8001_0025_u32 as i32;
pub const CELL_EROFS: i32 = 0x8001_0026_u32 as i32;
pub const CELL_ESPIPE: i32 = 0x8001_0027_u32 as i32;
pub const CELL_E2BIG: i32 = 0x8001_0028_u32 as i32;
pub const CELL_EACCES: i32 = 0x8001_0029_u32 as i32;
pub const CELL_EBADF: i32 = 0x8001_002a_u32 as i32;
pub const CELL_EIO: i32 = 0x8001_002b_u32 as i32;
pub const CELL_EMFILE: i32 = 0x8001_002c_u32 as i32;
pub const CELL_ENODEV: i32 = 0x8001_002d_u32 as i32;
pub const CELL_ENOTDIR: i32 = 0x8001_002e_u32 as i32;
pub const CELL_ENXIO: i32 = 0x8001_002f_u32 as i32;
pub const CELL_EXDEV: i32 = 0x8001_0030_u32 as i32;
pub const CELL_EBADMSG: i32 = 0x8001_0031_u32 as i32;
pub const CELL_EINPROGRESS: i32 = 0x8001_0032_u32 as i32;
pub const CELL_EMSGSIZE: i32 = 0x8001_0033_u32 as i32;
pub const CELL_ENAMETOOLONG: i32 = 0x8001_0034_u32 as i32;
pub const CELL_ENOLCK: i32 = 0x8001_0035_u32 as i32;
pub const CELL_ENOTEMPTY: i32 = 0x8001_0036_u32 as i32;
pub const CELL_ENOTSUP: i32 = 0x8001_0037_u32 as i32;
pub const CELL_EFSSPECIFIC: i32 = 0x8001_0038_u32 as i32;
pub const CELL_EOVERFLOW: i32 = 0x8001_0039_u32 as i32;
pub const CELL_ENOTMOUNTED: i32 = 0x8001_003a_u32 as i32;
pub const CELL_ENOTSDATA: i32 = 0x8001_003b_u32 as i32;
pub const CELL_ESDKVER: i32 = 0x8001_003c_u32 as i32;
pub const CELL_ENOLICDISC: i32 = 0x8001_003d_u32 as i32;
pub const CELL_ENOLICENT: i32 = 0x8001_003e_u32 as i32;

/// A negative CellOS status as returned in the low 32 bits of r3.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Lv2Error(i32);

impl Lv2Error {
    /// Converts a negative status to an error and preserves successful values.
    /// Use this with calls whose ABI encodes errors as negative status values.
    pub const fn check(status: i32) -> Result<i32, Self> {
        if status < 0 {
            Err(Self(status))
        } else {
            Ok(status)
        }
    }

    pub const fn code(self) -> i32 {
        self.0
    }
}

impl fmt::Display for Lv2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CellOS error {:#010x}", self.0 as u32)
    }
}

impl core::error::Error for Lv2Error {}
