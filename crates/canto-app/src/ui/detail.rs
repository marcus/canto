//! The detail panel for the selected song.

use canto_core::{Song, SongStatus};
use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use super::widgets::*;
use crate::app::CantoApp;
use crate::palette::*;

impl CantoApp {
    pub fn render_detail(&mut self, song: Song, cx: &mut Context<Self>) -> impl IntoElement {
        let id = song.id.clone();
        let is_current = self.player.song_id.as_deref() == Some(&song.id);
        let playing = is_current && self.player.is_playing();
        let confirm = self.confirm_delete.as_deref() == Some(&song.id);

        let mut facts = Vec::new();
        if let Some(d) = song.duration_secs {
            facts.push(fmt_time(d as f32));
        }
        facts.push(format!("seed {}", song.seed));
        facts.push(if song.is_draft() { "draft".into() } else { format!("{} steps", song.steps) });
        if let Some(s) = song.seconds_to_generate {
            facts.push(format!("made in {}", fmt_time(s as f32)));
        }
        facts.push(fmt_ago(song.created));

        div()
            .id("detail-scroll")
            .w(px(360.))
            .flex_none()
            .h_full()
            .overflow_y_scroll()
            .bg(panel())
            .border_l_1()
            .border_color(line())
            .px(px(24.))
            .pt(px(42.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .relative()
                    .child(cover(&song.id, px(312.)))
                    .when(song.audio.is_some(), |d| {
                        d.child(
                            div()
                                .id("detail-play")
                                .absolute()
                                .right(px(14.))
                                .bottom(px(14.))
                                .size(px(56.))
                                .rounded_full()
                                .bg(text())
                                .shadow_lg()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.9))
                                .child(icon(if playing { "pause-fill" } else { "play-fill" }, px(24.), bg()))
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, _, cx| this.play(&id, cx)
                                })),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(div().text_xl().font_weight(FontWeight::BOLD).child(song.title.clone()))
                    .child(div().text_xs().text_color(text_faint()).child(facts.join(" · "))),
            )
            .child(
                div().flex().flex_wrap().gap(px(5.)).children(
                    song.style
                        .split(',')
                        .map(str::trim)
                        .filter(|t| !t.is_empty())
                        .map(|t| {
                            div()
                                .px(px(8.))
                                .py(px(3.))
                                .rounded_full()
                                .bg(raised())
                                .text_size(px(11.))
                                .text_color(text_dim())
                                .child(t.to_string())
                        }),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.))
                    .child(
                        ghost_button("d-fav", Some(if song.favorite { "heart-fill" } else { "heart" }), if song.favorite { "Favorited" } else { "Favorite" })
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.toggle_favorite(&id, cx)
                            })),
                    )
                    .child(ghost_button("d-remix", Some("refresh"), "Remix").on_click(cx.listener({
                        let id = id.clone();
                        move |this, _, _, cx| this.remix(&id, cx)
                    })))
                    .child(ghost_button("d-reuse", Some("pencil"), "Reuse prompt").on_click(cx.listener({
                        let id = id.clone();
                        move |this, _, window, cx| this.reuse(&id, window, cx)
                    })))
                    .when(song.audio.is_some(), |d| {
                        d.child(ghost_button("d-mp3", Some("download"), "MP3").on_click(cx.listener({
                            let id = id.clone();
                            move |this, _, _, cx| this.export_mp3(&id, cx)
                        })))
                    })
                    .child(ghost_button("d-reveal", Some("folder-open"), "Reveal").on_click(cx.listener({
                        let id = id.clone();
                        move |this, _, _, cx| this.reveal(&id, cx)
                    })))
                    .child(
                        ghost_button("d-delete", Some("trash"), if confirm { "Really delete?" } else { "Delete" })
                            .when(confirm, |d| d.bg(danger().opacity(0.2)).text_color(danger()))
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.delete(&id, cx)
                            })),
                    ),
            )
            .when(song.status == SongStatus::Failed || song.status == SongStatus::Cancelled, |d| {
                d.child(
                    div()
                        .p(px(12.))
                        .rounded(px(10.))
                        .bg(danger().opacity(0.08))
                        .border_1()
                        .border_color(danger().opacity(0.3))
                        .text_xs()
                        .text_color(danger())
                        .child(match (&song.error, song.status) {
                            (Some(e), _) => e.clone(),
                            (None, SongStatus::Cancelled) => "Cancelled before it finished.".into(),
                            _ => "Generation failed. The engine log is in the song's folder (canto.log).".into(),
                        }),
                )
            })
            .when_some(song.prompt_source.clone(), |d, idea| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(section_label("From the idea"))
                        .child(div().text_sm().italic().text_color(text_dim()).child(idea)),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(section_label("Lyrics"))
                            .child(icon_button("d-copy", "copy", text_faint()).on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.copy_lyrics(&id, cx)
                            }))),
                    )
                    .child(render_lyrics(&song.lyrics)),
            )
    }
}

fn render_lyrics(lyrics: &str) -> impl IntoElement {
    div().flex().flex_col().children(lyrics.lines().enumerate().map(|(i, line)| {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            div()
                .when(i > 0, |d| d.pt(px(12.)))
                .pb(px(2.))
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .text_color(accent().opacity(0.8))
                .child(line.trim_matches(['[', ']']).to_uppercase())
        } else if line.is_empty() {
            div()
        } else {
            div().text_sm().line_height(px(22.)).text_color(text()).child(line.to_string())
        }
    }))
}
