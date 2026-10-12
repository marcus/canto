//! The library: songs in progress on top, then finished songs.

use canto_core::{Song, SongStatus, Stage};
use gpui_kit::component::input::Input;
use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use super::widgets::*;
use crate::app::{CantoApp, Filter, Job};
use crate::palette::*;

impl CantoApp {
    pub fn render_library(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let songs: Vec<Song> = self.visible_songs(cx).into_iter().cloned().collect();
        let total = self.songs.len();
        let empty = songs.is_empty() && self.jobs.is_empty();

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .pt(px(38.))
                    .px(px(28.))
                    .pb(px(12.))
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(10.))
                            .child(div().text_xl().font_weight(FontWeight::BOLD).child("Library"))
                            .child(div().text_sm().text_color(text_faint()).child(format!(
                                "{total} song{}",
                                if total == 1 { "" } else { "s" }
                            ))),
                    )
                    .child(div().w(px(240.)).child(Input::new(&self.search))),
            )
            .child(
                div()
                    .px(px(28.))
                    .pb(px(10.))
                    .flex()
                    .gap(px(6.))
                    .child(chip("filter-all", "All", self.filter == Filter::All).on_click(cx.listener(|this, _, _, cx| {
                        this.filter = Filter::All;
                        cx.notify();
                    })))
                    .child(chip("filter-fav", "Favorites", self.filter == Filter::Favorites).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.filter = Filter::Favorites;
                            cx.notify();
                        },
                    ))),
            )
            .child(
                div()
                    .id("library-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(16.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(self.jobs.iter().map(|job| render_job(job, cx).into_any_element()))
                    .children(songs.iter().map(|song| self.render_song_row(song, cx).into_any_element()))
                    .when(empty, |d| d.child(render_empty(self.filter))),
            )
    }

    fn render_song_row(&self, song: &Song, cx: &mut Context<Self>) -> impl IntoElement {
        let id = song.id.clone();
        let selected = self.selected.as_deref() == Some(&song.id);
        let is_current = self.player.song_id.as_deref() == Some(&song.id);
        let playing = is_current && self.player.is_playing();
        let playable = song.audio.is_some();
        let group = SharedString::from(format!("row-{}", song.id));

        div()
            .id(SharedString::from(format!("song-{}", song.id)))
            .group(group.clone())
            .flex()
            .items_center()
            .gap(px(14.))
            .p(px(8.))
            .rounded(px(12.))
            .cursor_pointer()
            .when(selected, |d| d.bg(raised()))
            .when(!selected, |d| d.hover(|s| s.bg(panel())))
            .on_click(cx.listener({
                let id = id.clone();
                move |this, event: &ClickEvent, _, cx| {
                    this.select(&id, cx);
                    if event.click_count() >= 2 {
                        this.play(&id, cx);
                    }
                }
            }))
            .child(
                div()
                    .id(SharedString::from(format!("cover-{}", song.id)))
                    .relative()
                    .child(cover(&song.id, px(52.)))
                    .when(playable, |d| {
                        d.child(
                            div()
                                .absolute()
                                .inset_0()
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(hsla(0., 0., 0., 0.45))
                                .when(!is_current, |d| d.invisible().group_hover(group.clone(), |s| s.visible()))
                                .child(icon(if playing { "pause-fill" } else { "play-fill" }, px(20.), text())),
                        )
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.select(&id, cx);
                                this.play(&id, cx);
                            }
                        }))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if is_current { accent() } else { text() })
                                    .child(song.title.clone()),
                            )
                            .when(song.is_draft() && song.status == SongStatus::Complete, |d| {
                                d.child(badge("DRAFT", text_faint()))
                            })
                            .when(song.status == SongStatus::Failed, |d| d.child(badge("FAILED", danger())))
                            .when(song.status == SongStatus::Incomplete, |d| d.child(badge("INCOMPLETE", text_faint()))),
                    )
                    .child(div().truncate().text_xs().text_color(text_dim()).child(song.style.clone())),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(48.))
                    .text_xs()
                    .text_color(text_faint())
                    .text_right()
                    .child(song.duration_secs.map(|d| fmt_time(d as f32)).unwrap_or_default()),
            )
            .child(
                icon_button(
                    SharedString::from(format!("fav-{}", song.id)),
                    if song.favorite { "heart-fill" } else { "heart" },
                    if song.favorite { accent() } else { text_faint() },
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.toggle_favorite(&id, cx);
                })),
            )
    }
}

fn render_job(job: &Job, cx: &mut Context<CantoApp>) -> impl IntoElement {
    let progress = job.shared.lock().unwrap().progress.clone();
    let fraction = progress.as_ref().map(|p| p.fraction).unwrap_or(0.0);
    let stage = progress.as_ref().map(|p| p.stage);
    let label = match stage {
        None => "Starting".to_string(),
        Some(Stage::Waiting) => "Queued — waiting for the current song".to_string(),
        Some(s) => s.label().to_string(),
    };
    let elapsed = job.started.elapsed().as_secs_f32();
    let title = if job.request.title.is_empty() { "Untitled".to_string() } else { job.request.title.clone() };
    let id = job.id.clone();

    div()
        .flex()
        .items_center()
        .gap(px(14.))
        .p(px(8.))
        .rounded(px(12.))
        .bg(panel())
        .child(
            div()
                .relative()
                .child(cover(&job.id, px(52.)).opacity(0.55))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(spinner(SharedString::from(format!("job-spin-{}", job.id)), px(20.), text())),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().truncate().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
                        .when(job.request.is_draft(), |d| d.child(badge("DRAFT", text_faint()))),
                )
                .child(progress_bar(fraction))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_xs()
                        .text_color(text_dim())
                        .child(format!("{label} · {}%", (fraction * 100.0).round()))
                        .child(fmt_time(elapsed)),
                ),
        )
        .child(
            icon_button(SharedString::from(format!("cancel-{}", job.id)), "x", text_faint())
                .on_click(cx.listener(move |this, _, _, cx| this.cancel_job(&id, cx))),
        )
}

fn render_empty(filter: Filter) -> impl IntoElement {
    let (title, body) = match filter {
        Filter::All => ("No songs yet", "Describe a song on the left and press Create."),
        Filter::Favorites => ("No favorites yet", "Tap the heart on a song to keep it here."),
    };
    div()
        .pt(px(80.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .child(icon("music", px(32.), text_faint()))
        .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(div().text_sm().text_color(text_dim()).child(body))
}
