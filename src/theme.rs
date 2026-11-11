// Hallmark v2: Flowmoor retained system; native GPUI app screen.
use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};

pub const SUCCESS_DARK: u32 = 0x64c197;
pub const SUCCESS_LIGHT: u32 = 0x168553;
pub const WARNING_DARK: u32 = 0xddaa5c;
pub const WARNING_LIGHT: u32 = 0xb06f1c;
pub const TWITCH_COLOR: u32 = 0x9146ff;

pub fn apply(dark: bool, cx: &mut App) {
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, |t| {
        t.font_family = ".SystemUIFont".into();
        // Match Flowmoor's browser rem so the same spacing and size classes agree.
        t.font_size = px(16.);
        t.radius = px(10.);
        let (background, surface, foreground, muted, line, accent, selected) = if dark {
            (
                0x020408, 0x05080e, 0xf8fcff, 0xaec2db, 0x223048, 0xb0c6e8, 0x0a0f18,
            )
        } else {
            (
                0xf9fbff, 0xffffff, 0x101a2b, 0x334560, 0xdae4f2, 0x306be0, 0xebf4ff,
            )
        };
        let c = &mut t.colors;
        c.background = rgb(background).into();
        c.foreground = rgb(foreground).into();
        c.sidebar = rgb(surface).into();
        c.border = rgb(line).into();
        c.input = rgb(line).into();
        c.muted = rgb(selected).into();
        c.muted_foreground = rgb(muted).into();
        c.accent = rgb(selected).into();
        c.accent_foreground = rgb(foreground).into();
        c.primary = rgb(accent).into();
        c.primary_foreground = rgb(background).into();
        c.ring = rgb(accent).into();
        c.button = rgb(surface).into();
        c.button_foreground = rgb(foreground).into();
        c.button_hover = rgb(selected).into();
        c.button_primary = rgb(selected).into();
        c.button_primary_foreground = rgb(foreground).into();
        c.button_primary_hover = rgb(line).into();
    });
}
