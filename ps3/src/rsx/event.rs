use core::ptr::NonNull;

use crate::sys::raw::PpuFunctionDescriptor;

use super::commands::packet;
use super::{raw, Context, Error};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Flip,
    Graphics,
    Queue,
    SecondV,
    User,
    VBlank,
}

/// A Rust callback converted to the compact descriptor GCM uses.
#[derive(Clone, Copy)]
pub struct EventCallback(NonNull<PpuFunctionDescriptor>);

impl EventCallback {
    pub fn new(handler: extern "C" fn(u32)) -> Result<Self, Error> {
        // ELFv1 function pointers refer to an OPD. moldier populates the
        // final eight bytes with the 32-bit entry and TOC for firmware calls.
        let address = handler as *const () as *const [u64; 3];
        let [entry, toc, compact] = unsafe { address.read() };
        if u32::try_from(entry).is_err()
            || u32::try_from(toc).is_err()
            || entry == 0
            || compact != (entry << 32 | toc)
        {
            return Err(Error::InvalidContext);
        }
        let compact = unsafe { address.cast::<u64>().add(2).cast_mut().cast() };
        Ok(Self(NonNull::new(compact).ok_or(Error::InvalidContext)?))
    }
}

impl Context {
    /// Installs or removes a callback invoked by GCM's event thread.
    ///
    /// # Safety
    ///
    /// The application must synchronize the handler with every other
    /// thread using its resources. The handler must obey GCM's callback
    /// restrictions and stay callable until all pending events finish.
    pub unsafe fn set_event_handler(&mut self, event: Event, handler: Option<EventCallback>) {
        let descriptor = handler.map_or(core::ptr::null(), |cb| cb.0.as_ptr().cast_const());
        unsafe {
            match event {
                Event::Flip => raw::set_flip_handler(descriptor),
                Event::Graphics => raw::set_graphics_handler(descriptor),
                Event::Queue => raw::set_queue_handler(descriptor),
                Event::SecondV => raw::set_second_v_handler(descriptor),
                Event::User => raw::set_user_handler(descriptor),
                Event::VBlank => raw::set_v_blank_handler(descriptor),
            }
        }
    }

    /// Appends a command that delivers `cause` to the GCM user handler.
    pub fn user_event(&mut self, cause: u32) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0xeb00, 1), cause]]) }
    }
}
