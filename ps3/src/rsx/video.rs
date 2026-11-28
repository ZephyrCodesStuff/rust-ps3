//! Configures the video output to scan out RSX display buffers.

#[cfg(feature = "sdk-370")]
use super::raw;
use super::Error;

/// VBlank or SecondV callback delivery frequency accepted by Cell GCM.
///
/// The SDK documents these values as 59.94 Hz, scanout frequency, and disabled.
/// They control callback frequency; they do not change the video output mode.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventFrequency {
    Hz59_94 = 1,
    Scanout = 2,
    Disabled = 3,
}

/// Position and size of one GCM display buffer, in RSX I/O address space.
#[cfg(feature = "sdk-370")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DisplayBufferInfo {
    pub offset: u32,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
}

/// A snapshot of GCM's display timing counters.
///
/// The timestamp values use the firmware's `system_time_t` units; compare
/// successive values rather than assuming a particular clock frequency.
#[cfg(feature = "sdk-370")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DisplayTelemetry {
    pub current_buffer_id: u8,
    pub current_field: u32,
    pub vblank_count: u64,
    pub last_flip_time: i64,
    pub last_second_v_time: i64,
}

/// Reads display buffer metadata and timing from the initialized GCM runtime.
///
/// The metadata array is firmware-owned and remains valid while GCM is
/// initialized. Buffer IDs are limited to the eight GCM display slots.
#[cfg(feature = "sdk-370")]
pub fn display_buffer_info(id: u8) -> Result<DisplayBufferInfo, Error> {
    if id >= 8 {
        return Err(Error::InvalidArgument);
    }
    let info = unsafe { raw::get_display_info() };
    if info.is_null() {
        return Err(Error::InvalidContext);
    }
    let info = unsafe { info.add(usize::from(id)).read() };
    Ok(DisplayBufferInfo {
        offset: info.offset,
        pitch: info.pitch,
        width: info.width,
        height: info.height,
    })
}

/// Returns the current display slot and the firmware's display timing counters.
#[cfg(feature = "sdk-370")]
pub fn display_telemetry() -> Result<DisplayTelemetry, Error> {
    let mut current_buffer_id = 0;
    Error::status(unsafe { raw::get_current_display_buffer_id(&mut current_buffer_id) })?;
    Ok(DisplayTelemetry {
        current_buffer_id,
        current_field: unsafe { raw::get_current_field() },
        vblank_count: unsafe { raw::get_v_blank_count() },
        last_flip_time: unsafe { raw::get_last_flip_time() },
        last_second_v_time: unsafe { raw::get_last_second_v_time() },
    })
}

/// Controls the hardware cursor managed by Cell GCM (SDK 3.70 and later).
///
/// Cursor operations affect process-wide GCM state. Serialize them with other
/// cursor users and with any code that changes the display output.
/// Initialize first, select an aligned image, then enable the cursor. Apply
/// position and image changes before calling [`Cursor::update`]; the SDK's
/// cursor sample performs that update from its VBlank handler.
#[cfg(feature = "sdk-370")]
pub struct Cursor;

#[cfg(feature = "sdk-370")]
impl Cursor {
    /// Alignment required for cursor images by the Cell GCM hardware.
    pub const IMAGE_ALIGNMENT: u32 = 2048;
    /// Byte length of the fixed 64 by 64 ARGB cursor image.
    pub const IMAGE_BYTES: u32 = 64 * 64 * 4;

    /// Initializes GCM's cursor support. Call before selecting the image.
    pub fn initialize() -> Result<(), Error> {
        Error::status(unsafe { raw::init_cursor() })
    }

    /// Enables the cursor after initialization and image setup.
    pub fn enable() -> Result<(), Error> {
        Error::status(unsafe { raw::set_cursor_enable() })
    }

    /// Hides the cursor.
    pub fn disable() -> Result<(), Error> {
        Error::status(unsafe { raw::set_cursor_disable() })
    }

    /// Sets the cursor's top-left position in display coordinates.
    pub fn set_position(x: i32, y: i32) -> Result<(), Error> {
        Error::status(unsafe { raw::set_cursor_position(x, y) })
    }

    /// Selects the cursor image in RSX I/O address space.
    ///
    /// # Safety
    ///
    /// `offset` must designate a `IMAGE_BYTES` ARGB image in RSX local memory.
    /// Keep that allocation reserved and unchanged while the cursor can
    /// display it. The SDK requires `IMAGE_ALIGNMENT` alignment.
    pub unsafe fn set_image_offset(offset: u32) -> Result<(), Error> {
        if offset % Self::IMAGE_ALIGNMENT != 0 {
            return Err(Error::InvalidArgument);
        }
        Error::status(unsafe { raw::set_cursor_image_offset(offset) })
    }

    /// Publishes changes to the cursor image or position.
    ///
    /// The Cell SDK cursor sample calls this from its VBlank handler after
    /// applying image or position changes.
    pub fn update() -> Result<(), Error> {
        Error::status(unsafe { raw::update_cursor() })
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Output {
    Primary = 0,
    Secondary = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Xrgb = 0,
    Xbgr = 1,
    Rgba16Float = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DisplayMode {
    pub resolution_id: u8,
    pub scan_mode: u8,
    pub conversion: u8,
    pub aspect: u8,
    pub reserved: [u8; 2],
    pub refresh_rates: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub state: u8,
    pub color_space: u8,
    pub reserved: [u8; 6],
    pub display_mode: DisplayMode,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Resolution {
    pub width: u16,
    pub height: u16,
}

#[repr(C)]
struct Configuration {
    resolution_id: u8,
    format: u8,
    aspect: u8,
    reserved: [u8; 9],
    pitch: u32,
}

const _: () = {
    assert!(core::mem::size_of::<State>() == 16);
    assert!(core::mem::size_of::<Configuration>() == 16);
    assert!(core::mem::offset_of!(Configuration, pitch) == 12);
};

extern "C" {
    #[link_name = "cellVideoOutGetState"]
    fn get_state(output: u32, device_index: u32, state: *mut State) -> i32;
    #[link_name = "cellVideoOutGetResolution"]
    fn get_resolution(resolution_id: u32, resolution: *mut Resolution) -> i32;
    #[link_name = "cellVideoOutGetResolutionAvailability"]
    fn get_resolution_availability(
        output: u32,
        resolution_id: u32,
        aspect: u32,
        option: u32,
    ) -> i32;
    #[link_name = "cellVideoOutConfigure"]
    fn configure_output(
        output: u32,
        config: *mut Configuration,
        option: *mut u32,
        wait_for_event: u32,
    ) -> i32;
}

impl Output {
    pub fn state(self, device_index: u32) -> Result<State, Error> {
        let mut state = State::default();
        Error::status(unsafe { get_state(self as u32, device_index, &mut state) })?;
        Ok(state)
    }

    pub fn supports(self, resolution_id: u8, aspect: u8) -> Result<bool, Error> {
        let status = unsafe {
            get_resolution_availability(self as u32, resolution_id.into(), aspect.into(), 0)
        };
        if status < 0 {
            Err(Error::Firmware(status))
        } else {
            Ok(status != 0)
        }
    }

    /// Selects the output mode. The pitch is the byte stride of display buffers.
    pub fn configure(
        self,
        resolution_id: u8,
        format: Format,
        aspect: u8,
        pitch: u32,
        wait_for_event: bool,
    ) -> Result<(), Error> {
        let mut config = Configuration {
            resolution_id,
            format: format as u8,
            aspect,
            reserved: [0; 9],
            pitch,
        };
        let status = unsafe {
            configure_output(
                self as u32,
                &mut config,
                core::ptr::null_mut(),
                u32::from(wait_for_event),
            )
        };
        Error::status(status)
    }
}

pub fn resolution(id: u8) -> Result<Resolution, Error> {
    let mut resolution = Resolution::default();
    Error::status(unsafe { get_resolution(id.into(), &mut resolution) })?;
    Ok(resolution)
}
