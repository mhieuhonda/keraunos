//! Display service: the post-handoff owner of the VGA text framebuffer.
//!
//! `kmain` boots with the VGA console as a log sink. When the desktop
//! session starts, the service detaches that sink, hides the hardware
//! cursor and takes exclusive ownership of the 80x25 buffer. From then on
//! frames flow one way: the UI composes a full back buffer and calls
//! [`Surface::blit`], which writes the cells straight to hardware.
//!
//! The service implements the UI crate's `Surface` trait — the single
//! contract between the kernel and the desktop.

use crate::console::vga;
use keraunos_ui::{Cell, Surface};

/// Columns and rows of the standard VGA text mode this service drives.
const COLUMNS: u16 = 80;
const ROWS: u16 = 25;

/// Exclusive handle to the text framebuffer; created once per boot.
pub struct TextDisplay {
    cols: u16,
    rows: u16,
}

impl TextDisplay {
    /// Detach the VGA console sink and take over the framebuffer.
    pub fn take_over() -> Self {
        crate::console::detach_vga();
        vga::hide_cursor();
        Self {
            cols: COLUMNS,
            rows: ROWS,
        }
    }
}

impl Surface for TextDisplay {
    fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    fn blit(&mut self, cells: &[Cell], stride: u16, w: u16, h: u16) {
        for y in 0..h as usize {
            for x in 0..w as usize {
                let cell = cells[y * stride as usize + x];
                let index = y * self.cols as usize + x;
                vga::write_entry(index, cell.ch, cell.attr());
            }
        }
    }
}
