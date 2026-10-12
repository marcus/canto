//! The Create panel: Simple (describe it) and Custom (write it) modes.

use canto_core::{Mode, RefinerKind};
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use super::widgets::*;
use crate::app::{CantoApp, Refining, Tab};
use crate::palette::*;

const IDEAS: &[&str] = &[
    "A rainy-day lo-fi song about missing the bus",
    "An 80s synthwave anthem about driving at night",
    "A gentle folk lullaby for a sleepy cat",
    "A stomping bluegrass song about a runaway tractor",
];

const STYLE_TAGS: &[&str] = &[
    "indie pop", "synthwave", "lo-fi hip hop", "folk", "surf rock", "soul", "drum and bass", "country",
    "female vocal", "male vocal", "acoustic guitar", "analog synths", "piano", "string section", "upbeat",
    "melancholic", "dreamy", "90 BPM", "120 BPM",
];

impl CantoApp {
    pub fn render_create(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy_idea = matches!(self.refining, Some(Refining::Idea { .. }));
        let create_label: SharedString = match (self.tab, busy_idea) {
            (_, true) => format!("{} is writing…", self.refiner.label()).into(),
            (Tab::Simple, false) => "Write & create".into(),
            (Tab::Custom, false) => "Create".into(),
        };

        div()
            .w(px(420.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(panel())
            .border_r_1()
            .border_color(line())
            // Title bar area: traffic lights sit on the left.
            .child(
                div()
                    .pt(px(42.))
                    .px(px(24.))
                    .pb(px(14.))
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(8.))
                            .child(div().text_2xl().font_weight(FontWeight::BOLD).text_color(text()).child("Canto"))
                            .child(div().text_xs().text_color(text_faint()).child("YuE2 · on this Mac")),
                    )
                    .child(self.render_refiner_picker(cx)),
            )
            .child(
                div()
                    .px(px(24.))
                    .pb(px(12.))
                    .flex()
                    .gap(px(6.))
                    .child(chip("tab-simple", "Simple", self.tab == Tab::Simple).on_click(
                        cx.listener(|this, _, _, cx| this.set_tab(Tab::Simple, cx)),
                    ))
                    .child(chip("tab-custom", "Custom", self.tab == Tab::Custom).on_click(
                        cx.listener(|this, _, _, cx| this.set_tab(Tab::Custom, cx)),
                    )),
            )
            .child(
                div()
                    .id("create-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(24.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .map(|d| match self.tab {
                        Tab::Simple => d.children(self.render_simple(cx)),
                        Tab::Custom => d.children(self.render_custom(cx)),
                    })
                    .child(self.render_options(cx)),
            )
            .child(
                div()
                    .px(px(24.))
                    .pt(px(12.))
                    .pb(px(20.))
                    .border_t_1()
                    .border_color(line())
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .when_some(self.form_error.clone(), |d, e| {
                        d.child(div().text_xs().text_color(danger()).child(e))
                    })
                    .child(
                        div()
                            .id("create")
                            .h(px(48.))
                            .rounded_full()
                            .bg(accent_gradient())
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(8.))
                            .text_color(rgb(0x1a0b12))
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .hover(|s| s.opacity(0.92))
                            .active(|s| s.opacity(0.8))
                            .when(busy_idea, |d| d.child(spinner("create-spin", px(16.), rgb(0x1a0b12).into())))
                            .when(!busy_idea, |d| d.child(icon("sparkles", px(16.), rgb(0x1a0b12).into())))
                            .child(create_label)
                            .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
                    ),
            )
    }

    fn render_refiner_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(2.))
            .rounded_full()
            .bg(raised())
            .children(RefinerKind::ALL.iter().map(|&kind| {
                let active = self.refiner == kind;
                div()
                    .id(SharedString::from(format!("refiner-{}", kind.label())))
                    .px(px(9.))
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .text_size(px(11.))
                    .cursor_pointer()
                    .when(active, |d| d.bg(hover()).text_color(text()).font_weight(FontWeight::SEMIBOLD))
                    .when(!active, |d| d.text_color(text_faint()).hover(|s| s.text_color(text_dim())))
                    .child(kind.label())
                    .on_click(cx.listener(move |this, _, _, cx| this.set_refiner(kind, cx)))
            }))
    }

    fn render_simple(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let busy = self.refining.is_some();
        vec![
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(section_label("Song description"))
                .child(Textarea::new(&self.idea))
                .child(
                    div()
                        .text_xs()
                        .text_color(text_faint())
                        .child(format!("{} writes the title, style and lyrics, then YuE2 sings it.", self.refiner.label())),
                )
                .into_any_element(),
            div()
                .flex()
                .flex_wrap()
                .gap(px(6.))
                .children(IDEAS.iter().enumerate().map(|(i, idea)| {
                    chip(("idea", i), *idea, false).on_click(cx.listener(move |this, _, window, cx| {
                        this.idea.update(cx, |s, cx| s.set_value(*idea, window, cx));
                    }))
                }))
                .into_any_element(),
            ghost_button("write-only", Some("pencil"), "Write it, but let me edit first")
                .when(busy, |d| d.opacity(0.5))
                .on_click(cx.listener(|this, _, window, cx| this.refine(Refining::Idea { then_create: false }, window, cx)))
                .into_any_element(),
        ]
    }

    fn render_custom(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(self.field_header("Lyrics", Refining::Lyrics, cx))
                .child(Textarea::new(&self.lyrics).h(px(280.)))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(self.field_header("Style of music", Refining::Style, cx))
                .child(Textarea::new(&self.style))
                .child(div().flex().flex_wrap().gap(px(5.)).children(STYLE_TAGS.iter().enumerate().map(|(i, tag)| {
                    div()
                        .id(("tag", i))
                        .px(px(8.))
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .rounded_full()
                        .bg(raised())
                        .text_size(px(11.))
                        .text_color(text_dim())
                        .cursor_pointer()
                        .hover(|s| s.bg(hover()).text_color(text()))
                        .child(format!("+ {tag}"))
                        .on_click(cx.listener(move |this, _, window, cx| this.append_style_tag(tag, window, cx)))
                })))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(section_label("Title"))
                .child(Input::new(&self.title))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(12.))
                .rounded(px(12.))
                .bg(field())
                .border_1()
                .border_color(line())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(icon("sparkles", px(14.), accent()))
                        .child(section_label(format!("Refine with {}", self.refiner.label()))),
                )
                .child(Input::new(&self.direction))
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(self.refine_button("refine-all", "Refine everything", Refining::All, cx)),
                )
                .when_some(self.refine_notes.clone(), |d, notes| {
                    d.child(div().text_xs().italic().text_color(text_dim()).child(notes))
                })
                .into_any_element(),
        ]
    }

    fn field_header(&self, label: &str, what: Refining, cx: &mut Context<Self>) -> impl IntoElement {
        let id = SharedString::from(format!("refine-{label}"));
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(section_label(label.to_string()))
            .child(self.refine_button(id, "Refine", what, cx))
    }

    fn refine_button(
        &self,
        id: impl Into<SharedString>,
        label: &str,
        what: Refining,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id: SharedString = id.into();
        let spinning = self.refining == Some(what);
        let disabled = self.refining.is_some() && !spinning;
        div()
            .id(ElementId::Name(id.clone()))
            .h(px(26.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(5.))
            .rounded_full()
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(accent())
            .bg(accent().opacity(0.12))
            .cursor_pointer()
            .hover(|s| s.bg(accent().opacity(0.2)))
            .when(disabled, |d| d.opacity(0.4))
            .map(|d| {
                if spinning {
                    d.child(spinner(ElementId::Name(format!("{id}-spin").into()), px(12.), accent()))
                        .child("Refining…")
                } else {
                    d.child(icon("sparkles", px(12.), accent())).child(label.to_string())
                }
            })
            .on_click(cx.listener(move |this, _, window, cx| this.refine(what, window, cx)))
    }

    fn render_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let draft = self.draft;
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .pt(px(4.))
            .child(section_label("Options"))
            .child(
                div()
                    .id("draft-toggle")
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_sm().text_color(text()).child("Quick draft"))
                            .child(div().text_xs().text_color(text_faint()).child("8 steps, about a minute. Rougher sound.")),
                    )
                    .child(toggle(draft))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.draft = !this.draft;
                        cx.notify();
                    })),
            )
            .when(self.tab == Tab::Custom, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_sm().text_color(text()).child("Structure"))
                        .child(div().flex().gap(px(4.)).children(
                            [(Mode::Full, "Melody + chords"), (Mode::Melody, "Melody"), (Mode::Off, "Free")].into_iter().map(
                                |(mode, label)| {
                                    chip(SharedString::from(format!("mode-{}", mode.as_str())), label, self.mode == mode)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.mode = mode;
                                            cx.notify();
                                        }))
                                },
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(div().text_sm().text_color(text()).child("Seed"))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .w(px(190.))
                                .child(div().flex_1().child(Input::new(&self.seed)))
                                .child(
                                    icon_button("dice", "dice", text_dim())
                                        .on_click(cx.listener(|this, _, window, cx| this.randomize_seed(window, cx))),
                                ),
                        ),
                )
            })
    }
}

fn toggle(on: bool) -> impl IntoElement {
    div()
        .w(px(36.))
        .h(px(20.))
        .flex_none()
        .rounded_full()
        .p(px(2.))
        .flex()
        .when(on, |d| d.bg(accent()).justify_end())
        .when(!on, |d| d.bg(line()))
        .child(div().size(px(16.)).rounded_full().bg(text()))
}
