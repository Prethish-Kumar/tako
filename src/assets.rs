//! Bundled assets: the mock's own SVG icon set and fonts, falling back to
//! gpui-kit's Lucide icons for anything gpui-component asks for.

use gpui_kit::{App, AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets"]
struct Local;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(f) = Local::get(path) {
            return Ok(Some(f.data));
        }
        gpui_kit::assets::Assets.load(path)
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v: Vec<SharedString> = Local::iter().filter(|p| p.starts_with(path)).map(|p| p.to_string().into()).collect();
        v.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(v)
    }
}

pub fn load_fonts(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = Local::iter()
        .filter(|p| p.starts_with("fonts/") && p.ends_with(".ttf"))
        .filter_map(|p| Local::get(&p).map(|f| f.data))
        .collect();
    cx.text_system().add_fonts(fonts).expect("load fonts");
}
