//! Small building blocks shared across panels.

use std::time::Duration;

use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use crate::assets::icon as icon_path;
use crate::palette::*;

pub fn icon(name: &str, size: Pixels, color: Hsla) -> Svg {
    svg().path(icon_path(name)).size(size).flex_none().text_color(color)
}

pub fn spinner(id: impl Into<ElementId>, size: Pixels, color: Hsla) -> impl IntoElement {
    icon("spinner", size, color).with_animation(
        id,
        Animation::new(Duration::from_millis(900)).repeat(),
        |svg, delta| svg.with_transformation(Transformation::rotate(percentage(delta))),
    )
}

/// Song artwork: a gradient keyed to the id, with a soft glyph.
pub fn cover(id: &str, size: Pixels) -> Div {
    let glyph = size * 0.34;
    div()
        .size(size)
        .flex_none()
        .rounded(size * 0.14)
        .bg(cover_gradient(id))
        .flex()
        .items_center()
        .justify_center()
        .child(icon("waves", glyph, hsla(0., 0., 1., 0.55)))
}

/// A round, quiet icon button.
pub fn icon_button(id: impl Into<ElementId>, name: &str, color: Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(30.))
        .flex_none()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(hover()))
        .child(icon(name, px(16.), color))
}

/// A pill used for tabs, filters and options.
pub fn chip(id: impl Into<ElementId>, label: impl Into<SharedString>, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(12.))
        .h(px(28.))
        .flex()
        .items_center()
        .rounded_full()
        .text_xs()
        .cursor_pointer()
        .border_1()
        .when(active, |d| d.bg(text()).text_color(bg()).border_color(text()).font_weight(FontWeight::SEMIBOLD))
        .when(!active, |d| d.border_color(line()).text_color(text_dim()).hover(|s| s.bg(hover()).text_color(text())))
        .child(label.into())
}

/// A secondary action with an optional leading icon.
pub fn ghost_button(id: impl Into<ElementId>, icon_name: Option<&str>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(30.))
        .px(px(12.))
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .rounded_full()
        .bg(raised())
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(text())
        .cursor_pointer()
        .hover(|s| s.bg(hover()))
        .when_some(icon_name, |d, name| d.child(icon(name, px(14.), text_dim())))
        .child(label.into())
}

pub fn section_label(text_: impl Into<SharedString>) -> Div {
    div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(text_dim()).child(text_.into())
}

pub fn badge(label: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .px(px(6.))
        .py(px(1.))
        .rounded(px(4.))
        .border_1()
        .border_color(color.opacity(0.5))
        .text_color(color)
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.into())
}

pub fn progress_bar(fraction: f32) -> Div {
    div()
        .h(px(4.))
        .w_full()
        .rounded_full()
        .bg(line())
        .child(div().h_full().rounded_full().w(relative(fraction.clamp(0.02, 1.0))).bg(accent_gradient()))
}

pub fn fmt_time(secs: f32) -> String {
    let s = secs.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn fmt_ago(created: u64) -> String {
    let now = canto_core::library::now();
    let d = now.saturating_sub(created);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", d / 60),
        3600..=86_399 => format!("{}h ago", d / 3600),
        _ => format!("{}d ago", d / 86_400),
    }
}
