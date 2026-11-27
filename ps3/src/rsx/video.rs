//! Configures the video output to scan out RSX display buffers.

use super::Error;

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
