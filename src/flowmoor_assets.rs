use gpui_kit::*;
use std::borrow::Cow;
pub(super) struct FlowmoorAssets;
impl AssetSource for FlowmoorAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let data = match path.rsplit('/').next().unwrap_or(path) {
            "arrow-down.svg" => {
                Some(include_bytes!("../assets/phosphor/arrow-down.svg") as &'static [u8])
            }
            "arrow-down-left.svg" => {
                Some(include_bytes!("../assets/phosphor/arrow-down-left.svg") as &'static [u8])
            }
            "arrow-left.svg" => {
                Some(include_bytes!("../assets/phosphor/arrow-left.svg") as &'static [u8])
            }
            "arrow-right.svg" => {
                Some(include_bytes!("../assets/phosphor/arrow-right.svg") as &'static [u8])
            }
            "arrow-up-right.svg" => {
                Some(include_bytes!("../assets/phosphor/arrow-up-right.svg") as &'static [u8])
            }
            "calendar.svg" => {
                Some(include_bytes!("../assets/phosphor/calendar.svg") as &'static [u8])
            }
            "check.svg" => Some(include_bytes!("../assets/phosphor/check.svg") as &'static [u8]),
            "chevron-down.svg" => {
                Some(include_bytes!("../assets/phosphor/chevron-down.svg") as &'static [u8])
            }
            "chevron-right.svg" => {
                Some(include_bytes!("../assets/phosphor/chevron-right.svg") as &'static [u8])
            }
            "chevron-up.svg" => {
                Some(include_bytes!("../assets/phosphor/chevron-up.svg") as &'static [u8])
            }
            "clipboard.svg" => {
                Some(include_bytes!("../assets/phosphor/clipboard.svg") as &'static [u8])
            }
            "clock.svg" => Some(include_bytes!("../assets/phosphor/clock.svg") as &'static [u8]),
            "download.svg" => {
                Some(include_bytes!("../assets/phosphor/download.svg") as &'static [u8])
            }
            "ellipsis-vertical.svg" => {
                Some(include_bytes!("../assets/phosphor/ellipsis-vertical.svg") as &'static [u8])
            }
            "external-link.svg" => {
                Some(include_bytes!("../assets/phosphor/external-link.svg") as &'static [u8])
            }
            "eye.svg" => Some(include_bytes!("../assets/phosphor/eye.svg") as &'static [u8]),
            "film.svg" => Some(include_bytes!("../assets/phosphor/film.svg") as &'static [u8]),
            "flag.svg" => Some(include_bytes!("../assets/phosphor/flag.svg") as &'static [u8]),
            "flag-off.svg" => {
                Some(include_bytes!("../assets/phosphor/flag-off.svg") as &'static [u8])
            }
            "folder-open.svg" => {
                Some(include_bytes!("../assets/phosphor/folder-open.svg") as &'static [u8])
            }
            "folder-plus.svg" => {
                Some(include_bytes!("../assets/phosphor/folder-plus.svg") as &'static [u8])
            }
            "info.svg" => Some(include_bytes!("../assets/phosphor/info.svg") as &'static [u8]),
            "languages.svg" => {
                Some(include_bytes!("../assets/phosphor/languages.svg") as &'static [u8])
            }
            "list-video.svg" => {
                Some(include_bytes!("../assets/phosphor/list-video.svg") as &'static [u8])
            }
            "maximize2.svg" => {
                Some(include_bytes!("../assets/phosphor/maximize2.svg") as &'static [u8])
            }
            "moon.svg" => Some(include_bytes!("../assets/phosphor/moon.svg") as &'static [u8]),
            "palette.svg" => {
                Some(include_bytes!("../assets/phosphor/palette.svg") as &'static [u8])
            }
            "panel-left-close.svg" => {
                Some(include_bytes!("../assets/phosphor/panel-left-close.svg") as &'static [u8])
            }
            "panel-left-open.svg" => {
                Some(include_bytes!("../assets/phosphor/panel-left-open.svg") as &'static [u8])
            }
            "pause.svg" => Some(include_bytes!("../assets/phosphor/pause.svg") as &'static [u8]),
            "play.svg" => Some(include_bytes!("../assets/phosphor/play.svg") as &'static [u8]),
            "plus.svg" => Some(include_bytes!("../assets/phosphor/plus.svg") as &'static [u8]),
            "radio.svg" => Some(include_bytes!("../assets/phosphor/radio.svg") as &'static [u8]),
            "rotate-ccw.svg" => {
                Some(include_bytes!("../assets/phosphor/rotate-ccw.svg") as &'static [u8])
            }
            "save.svg" => Some(include_bytes!("../assets/phosphor/save.svg") as &'static [u8]),
            "search.svg" => Some(include_bytes!("../assets/phosphor/search.svg") as &'static [u8]),
            "settings.svg" => {
                Some(include_bytes!("../assets/phosphor/settings.svg") as &'static [u8])
            }
            "skip-back.svg" => {
                Some(include_bytes!("../assets/phosphor/skip-back.svg") as &'static [u8])
            }
            "skip-forward.svg" => {
                Some(include_bytes!("../assets/phosphor/skip-forward.svg") as &'static [u8])
            }
            "sliders-horizontal.svg" => {
                Some(include_bytes!("../assets/phosphor/sliders-horizontal.svg") as &'static [u8])
            }
            "square.svg" => Some(include_bytes!("../assets/phosphor/square.svg") as &'static [u8]),
            "sun.svg" => Some(include_bytes!("../assets/phosphor/sun.svg") as &'static [u8]),
            "trash.svg" => Some(include_bytes!("../assets/phosphor/trash.svg") as &'static [u8]),
            "user.svg" => Some(include_bytes!("../assets/phosphor/user.svg") as &'static [u8]),
            "volume2.svg" => {
                Some(include_bytes!("../assets/phosphor/volume2.svg") as &'static [u8])
            }
            "volume-x.svg" => {
                Some(include_bytes!("../assets/phosphor/volume-x.svg") as &'static [u8])
            }
            "x.svg" => Some(include_bytes!("../assets/phosphor/x.svg") as &'static [u8]),
            _ => None,
        };
        if let Some(data) = data {
            Ok(Some(Cow::Borrowed(data)))
        } else {
            gpui_kit::assets::AllAssets.load(path)
        }
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::AllAssets.list(path)
    }
}
