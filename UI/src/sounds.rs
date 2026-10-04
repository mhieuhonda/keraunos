//! Sound events, resolved against the Keraunos sound theme in
//! `assets/sounds/Keraunos`. Event names follow the freedesktop sound
//! naming spec — the same table GNOME Shell plays from — so every file in
//! the theme has exactly one desktop moment it belongs to. Until the audio
//! driver lands, each event is narrated on serial, keeping sessions
//! deterministic.

use crate::SessionInfo;

/// Stereo files of the Keraunos sound theme, relative to `UI/assets`.
pub const THEME_DIR: &str = "assets/sounds/Keraunos/stereo";

/// One desktop sound event, named after the freedesktop spec entry whose
/// file it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Session bring-up: the desktop has started.
    Login,
    /// System is up and verified — the BOOT OK moment.
    SystemReady,
    /// Session end.
    Logoff,
    /// A notification banner appeared.
    Message,
    /// A new email arrived.
    MessageNewEmail,
    /// An instant message arrived.
    MessageNewInstant,
    /// An error dialog was raised.
    DialogError,
    /// A question dialog needs an answer.
    DialogQuestion,
    /// A warning dialog was raised.
    DialogWarning,
    /// A device was plugged in.
    DeviceAdded,
    /// A device was removed.
    DeviceRemoved,
    /// External power connected.
    PowerPlug,
    /// Running on battery now.
    PowerUnplug,
    /// Battery is running low.
    BatteryLow,
    /// Volume changed (OSD feedback).
    VolumeChange,
    /// The trash was emptied.
    TrashEmpty,
    /// A long operation completed.
    Complete,
    /// The system bell (terminal beep).
    Bell,
    /// First-boot startup fanfare.
    Startup,
}

impl Event {
    /// The theme file this event plays.
    pub fn file(self) -> &'static str {
        match self {
            Event::Login => "desktop-login.oga",
            Event::SystemReady => "system-ready.oga",
            Event::Logoff => "desktop-logoff.oga",
            Event::Message => "message.oga",
            Event::MessageNewEmail => "message-new-email.oga",
            Event::MessageNewInstant => "message-new-instant.oga",
            Event::DialogError => "dialog-error.oga",
            Event::DialogQuestion => "dialog-question.oga",
            Event::DialogWarning => "dialog-warning.oga",
            Event::DeviceAdded => "device-added.oga",
            Event::DeviceRemoved => "device-removed.oga",
            Event::PowerPlug => "power-plug.oga",
            Event::PowerUnplug => "power-unplug.oga",
            Event::BatteryLow => "battery-low.oga",
            Event::VolumeChange => "audio-volume-change.oga",
            Event::TrashEmpty => "trash-empty.oga",
            Event::Complete => "complete.oga",
            Event::Bell => "bell.oga",
            Event::Startup => "warty-startup.oga",
        }
    }
}

/// Raise one sound event: narrated for now, played once audio exists.
pub fn play(event: Event, info: &SessionInfo) {
    info.say_sound(event.file());
}
