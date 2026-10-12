//! The persistent player along the bottom: transport, waveform scrubber, volume.

use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use super::widgets::*;
use crate::app::CantoApp;
use crate::palette::*;

impl CantoApp {
    pub fn render_player_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.player.song_id.clone().and_then(|id| self.song(&id).cloned());
        let playing = self.player.is_playing();
        let position = self.player.position();
        let duration = self.player.duration.max(0.001);
        let fraction = if self.player.is_loaded() { position / duration } else { 0.0 };
        let peaks = current.as_ref().and_then(|s| self.waveforms.get(&s.id).cloned());
        let wave_bounds = self.wave_bounds.clone();
        let volume = self.player.volume;
        let volume_bounds = self.volume_bounds.clone();

        div()
            .h(px(84.))
            .flex_none()
            .w_full()
            .bg(panel())
            .border_t_1()
            .border_color(line())
            .px(px(20.))
            .flex()
            .items_center()
            .gap(px(20.))
            // Now playing
            .child(
                div()
                    .w(px(300.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .map(|d| match &current {
                        Some(song) => d
                            .child(cover(&song.id, px(52.)))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(div().truncate().text_sm().font_weight(FontWeight::SEMIBOLD).child(song.title.clone()))
                                    .child(div().truncate().text_xs().text_color(text_dim()).child(song.style.clone())),
                            ),
                        None => d.child(div().text_sm().text_color(text_faint()).child("Nothing playing")),
                    }),
            )
            // Transport + waveform
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        div()
                            .id("transport")
                            .size(px(40.))
                            .flex_none()
                            .rounded_full()
                            .bg(text())
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.opacity(0.9))
                            .child(icon(if playing { "pause-fill" } else { "play-fill" }, px(18.), bg()))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_playback(cx))),
                    )
                    .child(div().w(px(40.)).text_xs().text_color(text_dim()).text_right().child(fmt_time(position)))
                    .child(
                        div()
                            .id("waveform")
                            .flex_1()
                            .h(px(44.))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, e: &MouseDownEvent, _, cx| {
                                let f = fraction_in(this.wave_bounds.get(), e.position);
                                this.seek(f, cx);
                            }))
                            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                                if e.pressed_button == Some(MouseButton::Left) {
                                    let f = fraction_in(this.wave_bounds.get(), e.position);
                                    this.seek(f, cx);
                                }
                            }))
                            .child(
                                canvas(
                                    move |bounds, _, _| wave_bounds.set(bounds),
                                    move |bounds, _, window, _| paint_waveform(window, bounds, peaks.as_deref(), fraction),
                                )
                                .size_full(),
                            ),
                    )
                    .child(div().w(px(40.)).text_xs().text_color(text_dim()).child(fmt_time(if self.player.is_loaded() { duration } else { 0.0 }))),
            )
            // Volume
            .child(
                div()
                    .w(px(170.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        icon_button("mute", if volume <= 0.001 { "volume-off" } else { "volume-high" }, text_dim())
                            .on_click(cx.listener(|this, _, _, cx| {
                                let v = if this.player.volume > 0.001 { 0.0 } else { 0.8 };
                                this.set_volume(v, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("volume")
                            .flex_1()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, e: &MouseDownEvent, _, cx| {
                                let f = fraction_in(this.volume_bounds.get(), e.position);
                                this.set_volume(f, cx);
                            }))
                            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                                if e.pressed_button == Some(MouseButton::Left) {
                                    let f = fraction_in(this.volume_bounds.get(), e.position);
                                    this.set_volume(f, cx);
                                }
                            }))
                            .child(
                                div()
                                    .relative()
                                    .w_full()
                                    .h(px(4.))
                                    .rounded_full()
                                    .bg(line())
                                    .child(div().h_full().rounded_full().w(relative(volume)).bg(text_dim()))
                                    .child(canvas(move |b, _, _| volume_bounds.set(b), |_, _, _, _| {}).absolute().size_full()),
                            ),
                    ),
            )
    }
}

fn fraction_in(bounds: Bounds<Pixels>, position: Point<Pixels>) -> f32 {
    let width: f32 = bounds.size.width.into();
    if width <= 0.0 {
        return 0.0;
    }
    let x: f32 = (position.x - bounds.origin.x).into();
    (x / width).clamp(0.0, 1.0)
}

fn paint_waveform(window: &mut Window, bounds: Bounds<Pixels>, peaks: Option<&Vec<f32>>, played: f32) {
    let height: f32 = bounds.size.height.into();
    let width: f32 = bounds.size.width.into();
    let bars = peaks.filter(|p| !p.is_empty()).map(|p| p.len()).unwrap_or(120);
    let step = width / bars as f32;
    let bar_w = (step * 0.62).max(1.0);
    let played_color: Hsla = accent();
    let rest_color: Hsla = text_faint().opacity(0.55);
    for i in 0..bars {
        let amp = peaks.filter(|p| !p.is_empty()).map(|p| p[i]).unwrap_or(0.08);
        let h = (height * amp).max(2.0);
        let x = f32::from(bounds.origin.x) + step * i as f32;
        let y = f32::from(bounds.origin.y) + (height - h) / 2.0;
        let color = if (i as f32 + 0.5) / bars as f32 <= played { played_color } else { rest_color };
        window.paint_quad(
            fill(Bounds::new(point(px(x), px(y)), size(px(bar_w), px(h))), color).corner_radii(px(bar_w / 2.0)),
        );
    }
}
