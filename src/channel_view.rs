use super::*;
impl DownloadApp {
    pub(super) fn fetch_channel_images(&mut self) {
        let Some(page) = &self.channel_page else {
            return;
        };
        let mut sources: Vec<_> = page
            .entries
            .iter()
            .filter(|e| e.image.is_none())
            .filter_map(|e| e.thumbnail.clone().map(|url| (Some(e.id.clone()), url)))
            .collect();
        if page.avatar.is_none()
            && let Some(url) = &page.avatar_url
        {
            sources.push((None, url.clone()));
        }
        let (tx, rx) = mpsc::channel();
        self.channel_images = Some(rx);
        std::thread::spawn(move || {
            std::thread::scope(|scope| {
                for chunk in sources.chunks(6) {
                    let tx = tx.clone();
                    scope.spawn(move || {
                        for (id, url) in chunk {
                            if let Ok(path) = metadata::fetch_thumbnail(url) {
                                let _ = tx.send((id.clone(), path));
                            }
                        }
                    });
                }
            });
        });
    }

    pub(super) fn load_channel(&mut self, append: bool, cx: &mut Context<Self>) {
        if self.pending_channel.is_some() {
            return;
        }
        self.channel_load_failed = false;
        let Some(source) = &self.channel else {
            return;
        };
        let cookies = self.cookies.read(cx).value();
        let request = Request {
            url: source.listing_url(self.channel_tab),
            output: std::env::temp_dir(),
            range: None,
            live_from_start: false,
            is_live: false,
            retries: 0,
            cookies: (!cookies.trim().is_empty()).then(|| cookies.trim().into()),
            format: Default::default(),
        };
        let start = if append {
            self.channel_page.as_ref().map_or(1, |p| p.next)
        } else {
            1
        };
        self.close_preview(cx);
        self.info = None;
        self.pending = None;
        self.error = None;
        self.showing_channel = true;
        self.channel_glow.hide();
        self.channel_hovered = None;
        self.channel_append = append;
        if !append {
            self.channel_page = None;
        }
        let (tx, rx) = mpsc::channel();
        self.pending_channel = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(channel::inspect(&request, &Tools::default(), start));
        });
        cx.notify();
    }
    pub(super) fn channel_view(&self, cx: &mut Context<Self>) -> Div {
        let Some(source) = &self.channel else {
            return div();
        };
        if !source.youtube {
            return self.twitch_channel_view(cx);
        }
        let page = self.channel_page.as_ref();
        let loading = self.pending_channel.is_some();
        let shorts = self.channel_tab == 1;
        let muted = cx.theme().muted_foreground;
        let mut avatar = div()
            .size_7()
            .flex_shrink_0()
            .rounded_full()
            .overflow_hidden()
            .bg(cx.theme().muted)
            .flex()
            .items_center()
            .justify_center();
        avatar = if let Some(path) = page.and_then(|p| p.avatar.as_ref()) {
            avatar.child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
        } else {
            avatar.child(
                svg()
                    .data(include_bytes!("../assets/youtube.svg"))
                    .size(px(14.))
                    .text_color(cx.theme().danger),
            )
        };
        let title = page
            .map(|p| p.title.clone())
            .unwrap_or_else(|| source.base.clone());
        let mut view = div().w_full().min_w_0().flex().flex_col().gap_3().child(
            div()
                .flex()
                .items_center()
                .gap_2p5()
                .child(avatar)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(title),
                )
                .child(
                    Button::new("channel-refresh")
                        .size_11()
                        .border_0()
                        .icon(IconName::RotateCcw)
                        .tooltip(tr("重新整理頻道"))
                        .disabled(loading)
                        .on_click(cx.listener(|this, _, _, cx| this.load_channel(false, cx))),
                ),
        );
        let mut tabs = div().flex().w_full();
        for (index, label) in [(0, "Videos"), (1, "Shorts"), (2, "Streams")] {
            tabs = tabs.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .relative()
                    .child(
                        Button::new(("channel-tab", index))
                            .ghost()
                            .w_full()
                            .h_10()
                            .label(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.channel_tab != index {
                                    this.remember_navigation(cx);
                                    this.channel_tab = index;
                                    this.load_channel(false, cx);
                                }
                            })),
                    )
                    .when(self.channel_tab == index, |d| {
                        d.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .w_full()
                                .h(px(1.5))
                                .rounded_full()
                                .bg(cx.theme().primary),
                        )
                    }),
            );
        }
        view = view.child(tabs);
        let mut list = div()
            .id("youtube-channel-list")
            .relative()
            .flex()
            .flex_col()
            .gap(px(2.))
            .when(shorts, |d| d.grid().grid_cols(3).gap_2())
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.channel_glow.hide();
                    this.channel_hovered = None;
                    cx.notify();
                }
            }))
            .child(
                self.channel_glow
                    .view("youtube-channel-highlight", cx.theme().accent),
            );
        if let Some(page) = page {
            for (index, entry) in page.entries.iter().enumerate() {
                let url = entry.url.clone();
                let key_url = url.clone();
                let a11y_url = url.clone();
                let owner = cx.entity().downgrade();
                let measure = self.channel_glow.clone();
                let mut thumbnail = div()
                    .relative()
                    .flex_shrink_0()
                    .rounded(px(14.))
                    .overflow_hidden()
                    .bg(cx.theme().muted)
                    .when(shorts, |d| d.w_full().aspect_ratio(9. / 14.))
                    .when(!shorts, |d| d.w(px(128.)).h(px(72.)));
                if let Some(path) = &entry.image {
                    thumbnail =
                        thumbnail.child(img(path.clone()).size_full().object_fit(ObjectFit::Cover));
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
                            .right_1()
                            .bottom_1()
                            .px_1p5()
                            .py(px(2.))
                            .rounded(px(4.))
                            .bg(black().opacity(0.7))
                            .text_color(white())
                            .text_size(px(10.))
                            .child(flowmoor_rust::time_controls::compact_clock(duration as u64)),
                    );
                }
                if entry.live_status.as_deref() == Some("is_live") {
                    thumbnail = thumbnail.child(
                        div()
                            .absolute()
                            .left_1()
                            .top_1()
                            .px_1p5()
                            .py(px(2.))
                            .rounded(px(6.))
                            .bg(cx.theme().danger.opacity(0.8))
                            .text_color(white())
                            .text_size(px(9.))
                            .child("LIVE"),
                    );
                }
                thumbnail = thumbnail.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(black().opacity(0.4))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(IconName::Download).size_5().text_color(white()))
                        .opacity(if self.channel_hovered == Some(index) {
                            1.
                        } else {
                            0.
                        }),
                );
                let mut meta = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2p5()
                    .text_size(px(11.))
                    .text_color(muted);
                if let Some(views) = entry.view_count.filter(|views| *views > 0) {
                    let label = if views >= 1_000_000 {
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
                            .child(label),
                    );
                }
                if !shorts && let Some(date) = &entry.upload_date {
                    let format =
                        time::format_description::parse_borrowed::<2>("[year][month][day]")
                            .unwrap();
                    let label = time::Date::parse(date, &format)
                        .map(|d| format!("{}/{}/{}", d.year(), u8::from(d.month()), d.day()))
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
                        .id(("youtube-video", index))
                        .relative()
                        .min_w_0()
                        .rounded(px(14.))
                        .cursor_pointer()
                        .role(Role::Button)
                        .aria_label(entry.title.clone())
                        .tab_index(0)
                        .flex()
                        .gap_3()
                        .p_2()
                        .when(shorts, |d| {
                            d.flex_col().border_1().border_color(cx.theme().border)
                        })
                        .when(!shorts, |d| d.items_center())
                        .focus_visible(|d| d.border_1().border_color(cx.theme().ring))
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            if *hovered {
                                this.channel_glow.activate(index);
                                this.channel_hovered = Some(index);
                            } else if this.channel_hovered == Some(index) {
                                this.channel_hovered = None;
                            }
                            cx.notify();
                        }))
                        .on_click(
                            cx.listener(move |this, _, w, cx| {
                                this.inspect_video(url.clone(), w, cx)
                            }),
                        )
                        .on_key_down(cx.listener(move |this, e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                this.inspect_video(key_url.clone(), w, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_a11y_action(AccessibleAction::Click, move |_, w, cx| {
                            let _ = owner
                                .update(cx, |this, cx| this.inspect_video(a11y_url.clone(), w, cx));
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
                                .justify_center()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_size(px(if shorts { 12. } else { 13. }))
                                        .font_weight(FontWeight::MEDIUM)
                                        .line_height(px(18.))
                                        .line_clamp(2)
                                        .child(entry.title.clone()),
                                )
                                .child(meta),
                        ),
                );
            }
            if page.entries.is_empty() && !loading {
                list = list.child(
                    div()
                        .py_8()
                        .text_sm()
                        .text_color(muted)
                        .child(tr("沒有可顯示的影片")),
                );
            }
        }
        view = view.child(list);
        if page.is_some_and(|p| p.has_more) {
            view = view.child(self.channel_more(loading, cx));
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
    pub(super) fn channel_more(&self, loading: bool, cx: &mut Context<Self>) -> Div {
        let owner = cx.entity().downgrade();
        let enabled = !loading && !self.channel_load_failed;
        let more = div().child(
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
                                        && this.channel_page.as_ref().is_some_and(|p| p.has_more)
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
        more.when(self.channel_load_failed, |d| {
            d.child(
                Button::new("retry-channel")
                    .ghost()
                    .label(tr("重試"))
                    .on_click(cx.listener(|this, _, _, cx| this.load_channel(true, cx))),
            )
        })
    }
}
