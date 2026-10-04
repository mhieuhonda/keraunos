//! Icon lookups into the Keraunos icon theme (`assets/icons/Keraunos`).
//!
//! Every chrome element names the theme asset it stands for and the bitmap
//! size it draws at once the framebuffer lands; text surfaces render the
//! code page glyph instead. Sizes follow the shell convention: 16px for
//! panel rows, 24px for header badges, 48/256px for the brand mark.

/// One shell badge: the theme asset, its bitmap size and the code page 437
/// glyph that stands in on text surfaces.
#[derive(Debug, Clone, Copy)]
pub struct Badge {
    /// Asset path inside the theme, context included (e.g. `apps/...`).
    pub icon: &'static str,
    /// Bitmap size the asset is taken at.
    pub size: u16,
    /// Glyph drawn on text surfaces.
    pub glyph: u8,
}

impl Badge {
    /// The glyph this surface actually draws.
    pub const fn glyph(&self) -> u8 {
        self.glyph
    }
}

// Standard shell sizes, as used by the GNOME Shell status area (16) and
// window headers (24); the brand mark ships at 48 and 256.
pub const PANEL: u16 = 16;
pub const BADGE: u16 = 24;
pub const BRAND: u16 = 48;
pub const BRAND_LARGE: u16 = 256;

// Panel rows: 16px status assets from `16x16/panel`.
pub const BADGE_VOLUME: Badge = Badge {
    icon: "panel/audio-volume-high",
    size: PANEL,
    glyph: 0x0e,
};

// Window header badges: 24px assets.
pub const BADGE_SYSTEM: Badge = Badge {
    icon: "apps/applications-system",
    size: BADGE,
    glyph: 0x0f,
};
pub const BADGE_MEMORY: Badge = Badge {
    icon: "apps/gnome-system-monitor",
    size: BADGE,
    glyph: 0xdb,
};
pub const BADGE_LOG: Badge = Badge {
    icon: "apps/gnome-logs",
    size: BADGE,
    glyph: 0x10,
};
pub const BADGE_STATUS: Badge = Badge {
    icon: "apps/preferences-system",
    size: BADGE,
    glyph: 0xf0,
};
pub const BADGE_SESSION: Badge = Badge {
    icon: "status/dialog-information",
    size: BADGE,
    glyph: 0x07,
};

// Brand mark: `places/distributor-logo`, 48px inline and 256px for dialogs.
pub const BADGE_BRAND: Badge = Badge {
    icon: "places/distributor-logo",
    size: BRAND,
    glyph: 0x0f,
};
