# Sources

The desktop look comes from two upstream projects, adapted for the
Keraunos shell. This file carries the attribution; nothing upstream is
reproduced in code headers.

- [ubuntu/yaru](https://github.com/ubuntu/yaru) — `assets/icons/Keraunos`
  is the default Yaru icon theme with the artwork untouched and the theme
  renamed; `assets/sounds/Keraunos` is the Yaru sound theme laid out under
  freedesktop event names; `assets/theme` holds the shell theme artwork
  (start art, calendar, dash and workspace placeholders, pad OSD css).
  Icon artwork is CC-BY-SA 4.0, scripts GPL-3.0; license files ship inside
  each asset directory. Imported from yaru at commit `7f18973e`.
- [GNOME/gnome-shell](https://gitlab.gnome.org/GNOME/gnome-shell) — the
  shell layout (panel, overview, message tray, sound events) is a trimmed
  Rust port of `js/ui`, GPL-2.0-or-later, imported at commit `f9cd9aa`.
