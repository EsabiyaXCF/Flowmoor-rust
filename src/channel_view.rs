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
        let loading = self.pending_channel.is_some();
        let mut header = div().flex().items_center().justify_between().gap_3().child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(if source.youtube {
                    self.channel_page
                        .as_ref()
                        .map(|p| p.title.clone())
                        .unwrap_or_else(|| source.base.clone())
                } else {
                    source
                        .base
                        .rsplit('/')
                        .next()
                        .unwrap_or_default()
                        .to_string()
                }),
        );
        let live = source.live_url();
        header = header.child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("channel-live")
                        .ghost()
                        .icon(IconName::Radio)
                        .label(tr("直播"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.inspect_video(live.clone(), window, cx)
                        })),
                )
                .child(
                    Button::new("channel-refresh")
                        .ghost()
                        .icon(IconName::RotateCcw)
                        .tooltip(tr("重新整理頻道"))
                        .disabled(loading)
                        .on_click(cx.listener(|this, _, _, cx| this.load_channel(false, cx))),
                ),
        );
        let mut view = div().w_full().flex().flex_col().gap_3().child(header);
        let mut tabs = div()
            .flex()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .pb_3();
        for (index, label) in if source.youtube {
            vec![(0, "影片"), (1, "Shorts"), (2, "直播回放")]
        } else {
            vec![(0, "VOD")]
        } {
            tabs = tabs.child(
                Button::new(("channel-tab", index))
                    .ghost()
                    .selected(self.channel_tab == index)
                    .label(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.channel_tab == index {
                            return;
                        }
                        this.remember_navigation(cx);
                        this.channel_tab = index;
                        this.load_channel(false, cx);
                    })),
            )
        }
        view = view.child(tabs);
        if let Some(page) = &self.channel_page {
            if page.entries.is_empty() {
                view = view.child(
                    div()
                        .py_8()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("沒有可顯示的影片")),
                );
            }
            for entry in &page.entries {
                let url = entry.url.clone();
                let row_url = url.clone();
                let mut row = div()
                    .id(SharedString::from(format!("channel-entry-{}", entry.id)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.inspect_video(row_url.clone(), window, cx)
                    }));
                if let Some(image) = &entry.image {
                    row = row.child(
                        img(image.clone())
                            .w_32()
                            .h_20()
                            .object_fit(ObjectFit::Cover)
                            .rounded_lg(),
                    );
                }
                let mut details = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(entry.title.clone());
                if entry.live_status.as_deref() == Some("is_live") {
                    details = details.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().primary)
                            .child(tr("直播中")),
                    );
                } else if let Some(duration) = entry.duration {
                    details = details.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{}:{:02}:{:02}",
                                duration as u64 / 3600,
                                duration as u64 / 60 % 60,
                                duration as u64 % 60
                            )),
                    );
                }
                view = view.child(
                    row.child(details).child(
                        Button::new(SharedString::from(format!("select-{}", entry.id)))
                            .ghost()
                            .icon(IconName::Download)
                            .label(tr("選取"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.inspect_video(url.clone(), window, cx)
                            })),
                    ),
                );
            }
            if page.has_more {
                view = view.child(
                    Button::new("channel-more")
                        .ghost()
                        .label(tr("載入更多"))
                        .loading(loading)
                        .on_click(cx.listener(|this, _, _, cx| this.load_channel(true, cx))),
                );
            }
        }
        if loading {
            view = view.child(
                div()
                    .py_6()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("正在取得頻道影片…")),
            );
        }
        view
    }
}
