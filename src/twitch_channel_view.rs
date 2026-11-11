use super::*;
use flowmoor_rust::time_controls::compact_clock as clock;
impl DownloadApp {
    pub(super) fn twitch_channel_view(&self, cx: &mut Context<Self>) -> Div {
        let page = self.channel_page.as_ref();
        let loading = self.pending_channel.is_some();
        let title = page
            .map(|p| p.title.clone())
            .or_else(|| {
                self.channel
                    .as_ref()
                    .map(|s| s.base.rsplit('/').next().unwrap_or_default().into())
            })
            .unwrap_or_default();
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;
        let twitch: Hsla = rgb(theme::TWITCH_COLOR).into();
        let mut view = div().relative().w_full().flex().flex_col().gap_4().child(
            div()
                .flex()
                .items_center()
                .gap_2p5()
                .py_1()
                .child(
                    div()
                        .size_7()
                        .rounded_full()
                        .bg(twitch.opacity(0.15))
                        .border_1()
                        .border_color(twitch.opacity(0.2))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            svg()
                                .data(include_bytes!("../assets/twitch.svg"))
                                .size(px(14.))
                                .text_color(twitch),
                        ),
                )
                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title)),
        );
        if let Some(live) = page.and_then(|p| p.live.as_ref()) {
            let url = self.channel.as_ref().unwrap().live_url();
            view = view.child(
                div()
                    .id("twitch-live-settings")
                    .role(Role::Button)
                    .aria_label(tr("開啟直播下載設定"))
                    .tab_index(0)
                    .cursor_pointer()
                    .rounded(px(14.))
                    .border_1()
                    .border_color(danger.opacity(0.3))
                    .bg(danger.opacity(0.04))
                    .px_5()
                    .py_4()
                    .focus_visible(|s| s.border_color(danger))
                    .hover(|s| s.bg(danger.opacity(0.1)))
                    .on_click(
                        cx.listener(move |this, _, w, cx| this.inspect_video(url.clone(), w, cx)),
                    )
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            if let Some(source) = &this.channel {
                                let url = source.live_url();
                                this.inspect_video(url, w, cx);
                            }
                            cx.stop_propagation();
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(div().size_2().rounded_full().bg(danger))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(danger)
                                                    .child("L I V E"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(muted)
                                                    .child(clock(live.elapsed())),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .line_height(px(24.))
                                            .child(live.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .line_height(px(20.))
                                            .text_color(muted)
                                            .child(tr(
                                                "使用一般下載頁面設定畫質、編解碼與時間範圍。",
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(Icon::new(IconName::Download).size_4())
                                    .child(tr("開啟下載設定")),
                            ),
                    ),
            );
        }
        let count = page.map_or(0, |p| p.entries.len());
        view = view.child(
            div()
                .border_t_1()
                .border_color(cx.theme().border)
                .pt_4()
                .child(
                    div().flex().items_center().h_11().child(
                        Button::new("twitch-vods-toggle")
                            .ghost()
                            .h_8()
                            .px_0()
                            .accessibility_label(format!("VOD {count}"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Icon::new(if self.twitch_vods_open {
                                            IconName::ChevronDown
                                        } else {
                                            IconName::ChevronRight
                                        })
                                        .size(px(14.)),
                                    )
                                    .child(Icon::new(IconName::Film).size(px(14.)))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("VOD"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(muted.opacity(0.65))
                                            .child(count.to_string()),
                                    ),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.twitch_vods_open = !this.twitch_vods_open;
                                cx.notify();
                            })),
                    ),
                ),
        );
        if self.twitch_vods_open {
            let mut list = div()
                .id("twitch-vod-list")
                .mt(-px(4.))
                .relative()
                .flex()
                .flex_col()
                .gap_1()
                .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                    if !*hovered {
                        this.channel_glow.hide();
                        this.channel_hovered = None;
                        cx.notify();
                    }
                }))
                .child(
                    self.channel_glow
                        .view("channel-highlight", cx.theme().accent),
                );
            if let Some(page) = page {
                for (index, entry) in page.entries.iter().enumerate() {
                    let url = entry.url.clone();
                    let keyboard_url = url.clone();
                    let measure = self.channel_glow.clone();
                    let owner = cx.entity().downgrade();
                    let a11y_url = url.clone();
                    let mut thumbnail = div()
                        .relative()
                        .w(px(112.))
                        .h(px(63.))
                        .flex_shrink_0()
                        .rounded(px(14.))
                        .overflow_hidden()
                        .bg(cx.theme().accent);
                    if let Some(image) = &entry.image {
                        thumbnail = thumbnail
                            .child(img(image.clone()).size_full().object_fit(ObjectFit::Cover));
                    } else {
                        thumbnail = thumbnail.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Icon::new(IconName::Film).size_5().text_color(muted)),
                        );
                    }
                    if let Some(duration) = entry.duration {
                        thumbnail = thumbnail.child(
                            div()
                                .absolute()
                                .bottom_1()
                                .right_1()
                                .px_1p5()
                                .py_0p5()
                                .rounded(px(4.))
                                .bg(black().opacity(0.7))
                                .text_color(white())
                                .text_size(px(10.))
                                .child(clock(duration as u64)),
                        );
                    }
                    let hovered = self.channel_hovered == Some(index);
                    thumbnail = thumbnail.child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(black().opacity(0.4))
                            .child(Icon::new(IconName::Download).size_5().text_color(white()))
                            .with_spring(
                                ("vod-overlay", index),
                                SpringAnimation::new(SpringConfig::new(1400., 30., 0.16))
                                    .to(if hovered { 1f32 } else { 0. })
                                    .from(0.),
                                |layer, v| layer.opacity(v),
                            ),
                    );
                    let mut meta = div()
                        .flex()
                        .items_center()
                        .gap_2p5()
                        .text_size(px(11.))
                        .text_color(muted);
                    if let Some(views) = entry.view_count {
                        let value = if views >= 1_000_000 {
                            format!("{:.1}M", views as f64 / 1_000_000.)
                        } else if views >= 1000 {
                            format!("{:.1}K", views as f64 / 1000.)
                        } else {
                            views.to_string()
                        };
                        meta = meta.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(Icon::new(IconName::Eye).size_3())
                                .child(value),
                        );
                    }
                    if let Some(date) = &entry.upload_date {
                        let label = time::OffsetDateTime::parse(
                            date,
                            &time::format_description::well_known::Rfc3339,
                        )
                        .map(|d| format!("{}/{}/{}", u8::from(d.month()), d.day(), d.year()))
                        .unwrap_or_else(|_| date.clone());
                        meta = meta.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(Icon::new(IconName::Calendar).size_3())
                                .child(label),
                        );
                    }
                    list = list.child(
                        div()
                            .id(("twitch-vod", index))
                            .w_full()
                            .mx(-px(8.))
                            .relative()
                            .role(Role::Button)
                            .aria_label(entry.title.clone())
                            .tab_index(0)
                            .cursor_pointer()
                            .rounded(px(14.))
                            .p_2()
                            .flex()
                            .items_center()
                            .gap_3()
                            .focus_visible(|s| {
                                s.bg(cx.theme().accent)
                                    .border_1()
                                    .border_color(cx.theme().ring)
                            })
                            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                                if *hovered {
                                    this.channel_glow.activate(index);
                                    this.channel_hovered = Some(index);
                                } else if this.channel_hovered == Some(index) {
                                    this.channel_hovered = None;
                                }
                                cx.notify();
                            }))
                            .on_click(cx.listener(move |this, _, w, cx| {
                                this.inspect_video(url.clone(), w, cx)
                            }))
                            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    this.inspect_video(keyboard_url.clone(), w, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .on_a11y_action(AccessibleAction::Click, move |_, w, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.inspect_video(a11y_url.clone(), w, cx)
                                });
                            })
                            .child(
                                canvas(
                                    move |b, w, _| {
                                        if measure.measure(index, b) {
                                            w.on_next_frame(|w, _| w.refresh());
                                        }
                                    },
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .inset_0(),
                            )
                            .child(thumbnail)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .line_height(px(18.))
                                            .child(entry.title.clone()),
                                    )
                                    .child(meta),
                            ),
                    );
                }
            }
            if !loading && count == 0 {
                list = list.child(
                    div()
                        .py_6()
                        .text_xs()
                        .text_color(muted)
                        .child(tr("沒有可下載的 VOD")),
                );
            }
            view = view.child(list);
            if page.is_some_and(|p| p.has_more) {
                let owner = cx.entity().downgrade();
                let enabled = !loading && !self.channel_load_failed;
                view = view.child(
                    div().h_11().w_full().child(
                        canvas(
                            move |bounds, w, _| {
                                if enabled && bounds.top() < w.viewport_size().height + px(120.) {
                                    let owner = owner.clone();
                                    w.on_next_frame(move |_, cx| {
                                        let _ = owner.update(cx, |this, cx| {
                                            if this.showing_channel
                                                && this.pending_channel.is_none()
                                                && !this.channel_load_failed
                                                && this
                                                    .channel_page
                                                    .as_ref()
                                                    .is_some_and(|p| p.has_more)
                                            {
                                                this.load_channel(true, cx);
                                            }
                                        });
                                    });
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    ),
                );
                if self.channel_load_failed {
                    view = view.child(
                        Button::new("retry-vods")
                            .ghost()
                            .label(tr("重試"))
                            .on_click(cx.listener(|this, _, _, cx| this.load_channel(true, cx))),
                    );
                }
            }
        }
        if loading {
            view = view.child(
                div()
                    .py_4()
                    .text_sm()
                    .text_color(muted)
                    .child(tr("正在取得頻道影片…")),
            );
        }
        view
    }
}
