//! Canto — a desktop studio for local YuE2 song generation.

mod app;
mod assets;
mod palette;
mod player;
mod ui;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

fn main() {
    gpui_kit::application().with_assets(assets::Assets).run(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        Theme::update(cx, |theme| {
            let c = &mut theme.colors;
            c.background = palette::field();
            c.input = palette::line();
            c.border = palette::line();
            c.foreground = palette::text();
            c.muted_foreground = palette::text_faint();
            c.caret = palette::accent();
            c.ring = palette::accent().opacity(0.6);
            c.selection = palette::accent().opacity(0.3);
            c.scrollbar = palette::bg().opacity(0.);
            c.scrollbar_thumb = palette::line();
            c.scrollbar_thumb_hover = palette::hover();
        });

        let bounds = Bounds::centered(None, size(px(1400.), px(900.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(1100.), px(700.))),
            titlebar: Some(TitlebarOptions {
                title: Some("Canto".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.), px(16.))),
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| app::CantoApp::new(window, cx)))
            .expect("failed to open the Canto window");
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.activate(true);
    });
}
