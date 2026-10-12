//! Layout: create panel | library | detail, with the player along the bottom.

mod create;
mod detail;
mod library;
mod player_bar;
pub mod widgets;

use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use crate::app::CantoApp;
use crate::palette::*;
use widgets::icon;

impl Render for CantoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.clone().and_then(|id| self.song(&id).cloned());
        if let Some(song) = &selected {
            self.ensure_waveform(song, cx);
        }
        let toast = self.toast.as_ref().map(|t| (t.message.clone(), t.is_error));

        div()
            .size_full()
            .relative()
            .bg(bg())
            .text_color(text())
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_create(window, cx))
                    .child(self.render_library(window, cx))
                    .when_some(selected, |d, song| d.child(self.render_detail(song, cx))),
            )
            .child(self.render_player_bar(cx))
            .when_some(toast, |d, (message, is_error)| {
                d.child(
                    div()
                        .absolute()
                        .bottom(px(100.))
                        .right(px(24.))
                        .max_w(px(460.))
                        .child(
                            div()
                                .id("toast")
                                .flex()
                                .items_start()
                                .gap(px(10.))
                                .px(px(14.))
                                .py(px(10.))
                                .rounded(px(12.))
                                .bg(raised())
                                .border_1()
                                .border_color(if is_error { danger().opacity(0.5) } else { line() })
                                .shadow_lg()
                                .text_sm()
                                .cursor_pointer()
                                .child(icon(if is_error { "x" } else { "check" }, px(16.), if is_error { danger() } else { success() }))
                                .child(div().flex_1().child(message))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_toast(cx))),
                        ),
                )
            })
    }
}
