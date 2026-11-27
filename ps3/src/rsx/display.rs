use super::{video, Buffer, Context, Error, LocalArena, Surface};

/// A linear A8R8G8B8 buffer that can be rendered to and scanned out.
pub struct Framebuffer {
    buffer: Buffer,
    id: u8,
    size: video::Resolution,
    pitch: u32,
}

impl Framebuffer {
    pub fn storage(&self) -> &Buffer {
        &self.buffer
    }

    pub fn resolution(&self) -> video::Resolution {
        self.size
    }

    pub fn pitch(&self) -> u32 {
        self.pitch
    }

    /// Builds a render target using a matching Z24S8 depth allocation.
    pub fn surface(&self, depth: &Buffer) -> Result<Surface, Error> {
        if depth.size() < self.buffer.size() || depth.offset() % 64 != 0 {
            return Err(Error::InvalidArgument);
        }
        Ok(Surface {
            kind: 1,
            color_format: 8,
            color_target: 1,
            color_location: [self.buffer.location() as u8; 4],
            color_offset: [self.buffer.offset(), 0, 0, 0],
            color_pitch: [self.pitch, 64, 64, 64],
            depth_format: 2,
            depth_location: depth.location() as u8,
            depth_offset: depth.offset(),
            depth_pitch: self.pitch,
            width: self.size.width,
            height: self.size.height,
            ..Default::default()
        })
    }
}

/// Two framebuffer slots. A submitted frame is kept out of the render
/// path until the firmware confirms its flip.
pub struct Swapchain {
    buffers: [Framebuffer; 2],
    rendering: usize,
    front: Option<usize>,
    pending: bool,
}

impl Swapchain {
    /// Registers two local-memory buffers at the selected video mode.
    ///
    /// # Safety
    ///
    /// Own the display output and both buffer IDs exclusively. No other
    /// code may submit flips or reset the flip status while this swapchain
    /// is in use. Keep the local allocations reserved while scanout can
    /// reach them, including after dropping the swapchain.
    pub unsafe fn new(
        context: &mut Context,
        arena: &mut LocalArena,
        output: video::Output,
        mode: video::DisplayMode,
        ids: [u8; 2],
    ) -> Result<Self, Error> {
        if ids[0] == ids[1] || ids.iter().any(|id| *id >= 8) {
            return Err(Error::InvalidArgument);
        }
        let size = video::resolution(mode.resolution_id)?;
        if size.width == 0 || size.height == 0 {
            return Err(Error::InvalidArgument);
        }
        let pitch = (u32::from(size.width) * 4).next_multiple_of(64);
        let bytes = pitch
            .checked_mul(u32::from(size.height))
            .ok_or(Error::InvalidArgument)?;
        let [first, second] = ids;
        let mut allocate = |id| -> Result<Framebuffer, Error> {
            Ok(Framebuffer {
                buffer: arena.allocate(bytes, 64)?,
                id,
                size,
                pitch,
            })
        };
        let buffers = [allocate(first)?, allocate(second)?];
        output.configure(
            mode.resolution_id,
            video::Format::Xrgb,
            mode.aspect,
            pitch,
            false,
        )?;
        for buffer in &buffers {
            unsafe {
                context.set_display_buffer(
                    buffer.id,
                    buffer.buffer.offset(),
                    pitch,
                    size.width.into(),
                    size.height.into(),
                )?;
            }
        }
        Ok(Self {
            buffers,
            rendering: 0,
            front: None,
            pending: false,
        })
    }

    /// Returns the buffer available for drawing. Wait for the pending flip
    /// before starting another frame when this returns `None`.
    pub fn back_buffer(&self) -> Option<&Framebuffer> {
        if self.pending {
            None
        } else {
            Some(&self.buffers[self.rendering])
        }
    }

    pub fn front_buffer(&self) -> Option<&Framebuffer> {
        self.front.map(|index| &self.buffers[index])
    }

    /// Queues and submits the current frame.
    ///
    /// # Safety
    ///
    /// All pending commands, including the draw into the back buffer, must
    /// satisfy the requirements of [`Context::flush`]. If submitting the FIFO
    /// fails after the flip was queued, retry `flush` before waiting.
    pub unsafe fn present(&mut self, context: &mut Context) -> Result<(), Error> {
        if self.pending {
            return Err(Error::InvalidContext);
        }
        context.reset_flip_status();
        unsafe { context.flip(self.buffers[self.rendering].id)? };
        self.pending = true;
        unsafe { context.flush() }
    }

    /// Waits for scanout and releases the other buffer for drawing. A
    /// timeout leaves the queued frame pending so the wait can be retried.
    pub fn wait(&mut self, context: &Context, polls: u32) -> Result<(), Error> {
        if !self.pending {
            return Err(Error::InvalidContext);
        }
        context.wait_for_display_flip(polls)?;
        self.front = Some(self.rendering);
        self.rendering ^= 1;
        self.pending = false;
        Ok(())
    }
}
