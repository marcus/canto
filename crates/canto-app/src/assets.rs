//! Canto's own icons (from roc) layered over GPUI Kit's bundled assets.

use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$((concat!("canto/icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*]
    };
}

const ICONS: &[(&str, &[u8])] = icons![
    "check", "chevron-down", "copy", "dice", "download", "folder-open", "heart", "heart-fill", "more-horizontal",
    "music", "pause", "pause-fill", "pencil", "play", "play-fill", "plus", "refresh", "search", "sparkles", "spinner",
    "stop", "trash", "volume-high", "volume-off", "waves", "x",
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out: Vec<SharedString> =
            ICONS.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| SharedString::from(*p)).collect();
        out.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(out)
    }
}

/// Path for one of Canto's icons.
pub fn icon(name: &str) -> SharedString {
    format!("canto/icons/{name}.svg").into()
}
