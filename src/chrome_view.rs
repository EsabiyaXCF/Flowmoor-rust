use super::*;
#[derive(Clone)]
pub(super) struct DownloadSnapshot {
    url: String,
    info_url: String,
    info: Option<MediaInfo>,
    channel: Option<ChannelSource>,
    channel_page: Option<channel::Page>,
    channel_tab: usize,
    showing_channel: bool,
    start: String,
    end: String,
    filename: String,
    contents: Vec<Content>,
    quality: Option<SharedString>,
    video_codec: Option<SharedString>,
    audio_codec: Option<SharedString>,
    preset: Option<SharedString>,
    live_from_start: bool,
    scroll: Point<Pixels>,
    metadata_loading: bool,
    channel_loading: Option<bool>,
    edit_request: Option<Request>,
}
#[derive(Clone)]
pub(super) struct NavigationSnapshot {
    page: Page,
    download: DownloadSnapshot,
    library_selected: Option<u128>,
    scroll: Point<Pixels>,
}
impl DownloadApp {
    fn snapshot(&self, cx: &App) -> DownloadSnapshot {
        DownloadSnapshot {
            url: if self.info.is_some() {
                self.info_url.clone()
            } else if self.showing_channel {
                self.channel
                    .as_ref()
                    .map(|c| c.listing_url(self.channel_tab))
                    .unwrap_or_else(|| self.url.read(cx).value().to_string())
            } else {
                self.url.read(cx).value().to_string()
            },
            info_url: self.info_url.clone(),
            info: self.info.clone(),
            channel: self.channel.clone(),
            channel_page: self.channel_page.clone(),
            channel_tab: self.channel_tab,
            showing_channel: self.showing_channel,
            start: self.start.read(cx).value().to_string(),
            end: self.end.read(cx).value().to_string(),
            filename: self.filename.read(cx).value().to_string(),
            contents: self.contents.clone(),
            quality: self.quality.read(cx).selected_value().cloned(),
            video_codec: self.video_codec.read(cx).selected_value().cloned(),
            audio_codec: self.audio_codec.read(cx).selected_value().cloned(),
            preset: self.presets.read(cx).selected_value().cloned(),
            live_from_start: self.live_from_start,
            scroll: self.workspace_scroll.offset(),
            metadata_loading: self.pending.is_some(),
            channel_loading: self.pending_channel.as_ref().map(|_| self.channel_append),
            edit_request: self.library_edit_request.clone(),
        }
    }
    fn navigation_snapshot(&self, cx: &App) -> NavigationSnapshot {
        NavigationSnapshot {
            page: self.page,
            download: self.snapshot(cx),
            library_selected: self.library_selected,
            scroll: self.workspace_scroll.offset(),
        }
    }
    pub(super) fn remember_navigation(&mut self, cx: &App) {
        self.navigation.push(self.navigation_snapshot(cx));
    }
    fn restore_snapshot(
        &mut self,
        s: DownloadSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_preview(cx);
        self.channel_glow.hide();
        self.channel_hovered = None;
        self.channel_load_failed = false;
        self.open_menu = None;
        self.menu_subscription = None;
        self.pending = None;
        self.pending_thumbnail = None;
        self.pending_avatar = None;
        self.pending_channel = None;
        self.library_edit_request = None;
        self.url.update(cx, |v, cx| v.set_value(s.url, window, cx));
        self.start
            .update(cx, |v, cx| v.set_value(s.start, window, cx));
        self.end.update(cx, |v, cx| v.set_value(s.end, window, cx));
        self.filename
            .update(cx, |v, cx| v.set_value(s.filename, window, cx));
        self.info_url = s.info_url;
        self.info = s.info;
        self.channel = s.channel;
        self.channel_page = s.channel_page;
        self.channel_tab = s.channel_tab;
        self.showing_channel = s.showing_channel;
        self.channel_images = None;
        if self.showing_channel {
            self.fetch_channel_images();
        }
        self.contents = s.contents;
        self.live_from_start = s.live_from_start;
        let mut heights: Vec<_> = self
            .info
            .as_ref()
            .into_iter()
            .flat_map(|i| &i.formats)
            .filter(|f| {
                f.vcodec
                    .as_deref()
                    .is_some_and(|v| v != "none" && v != "images")
            })
            .filter_map(|f| f.height)
            .filter(|h| *h > 0)
            .collect();
        if let Some(h) = s
            .quality
            .as_ref()
            .and_then(|v| v.strip_suffix('p'))
            .and_then(|v| v.parse::<u32>().ok())
        {
            heights.push(h);
        }
        heights.sort_unstable_by(|a, b| b.cmp(a));
        heights.dedup();
        let items = std::iter::once("最佳可用畫質".into())
            .chain(heights.into_iter().map(|h| format!("{h}p").into()))
            .collect();
        self.quality
            .update(cx, |v, cx| v.set_items(items, window, cx));
        for (state, value) in [
            (&self.quality, s.quality),
            (&self.video_codec, s.video_codec),
            (&self.audio_codec, s.audio_codec),
            (&self.presets, s.preset),
        ] {
            state.update(cx, |v, cx| {
                if let Some(value) = value {
                    v.set_selected_value(&value, window, cx);
                } else {
                    v.set_selected_index(None, window, cx);
                }
                cx.notify();
            });
        }
        if let Some(append) = s.channel_loading {
            self.load_channel(append, cx);
        } else if s.metadata_loading {
            self.input_checkpoint = true;
            self.inspect_video(self.info_url.clone(), window, cx);
            self.library_edit_request = s.edit_request;
        } else {
            self.fetch_info_images();
        }
        self.input_checkpoint = false;
        self.error = None;
        let scroll = self.workspace_scroll.clone();
        let offset = s.scroll;
        window.on_next_frame(move |w, _| {
            w.on_next_frame(move |w, _| {
                scroll.set_offset(offset);
                w.refresh();
            });
            w.refresh();
        });
        cx.notify();
    }
    pub(super) fn navigate_to(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page == page {
            if page == Page::Library && self.library_selected.is_some() {
                self.remember_navigation(cx);
                self.library_selected = None;
                self.close_library_player(cx);
                cx.notify();
            }
            return;
        }
        if page == Page::Settings {
            self.begin_settings(cx);
        }
        self.page_scroll[self.page as usize] = self.workspace_scroll.offset();
        self.remember_navigation(cx);
        self.page = page;
        self.workspace_scroll
            .set_offset(self.page_scroll[page as usize]);
        if page != Page::Download {
            self.close_preview(cx);
        }
        self.error = None;
        if page != Page::Library {
            self.close_library_player(cx);
        } else if let Some(id) = self.library_selected {
            self.prepare_library_player(id, cx);
        }
        cx.notify();
    }
    pub(super) fn navigate_history(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page == Page::Settings {
            self.discard_settings(window, cx);
        }
        let current = self.navigation_snapshot(cx);
        let target = if forward {
            self.navigation.forward(current)
        } else {
            self.navigation.back(current)
        };
        if let Some(target) = target {
            self.page_scroll[self.page as usize] = self.workspace_scroll.offset();
            self.close_preview(cx);
            self.close_library_player(cx);
            self.open_menu = None;
            self.page = target.page;
            self.library_selected = target.library_selected;
            if self.page == Page::Download {
                self.restore_snapshot(target.download, window, cx);
            } else {
                if self.page == Page::Settings {
                    self.begin_settings(cx);
                }
                if self.page == Page::Library
                    && let Some(id) = self.library_selected
                {
                    self.prepare_library_player(id, cx);
                }
                let scroll = self.workspace_scroll.clone();
                window.on_next_frame(move |w, _| {
                    scroll.set_offset(target.scroll);
                    w.refresh();
                });
            }
            self.error = None;
            cx.notify();
        }
    }
    pub(super) fn chrome_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let back = self.navigation.can_back();
        let forward = self.navigation.can_forward();
        gpui_kit::component::TitleBar::new()
            .h(px(44.))
            .pl(px(if cfg!(target_os = "macos") { 96. } else { 16. }))
            .pr_4()
            .bg(cx.theme().sidebar)
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(
                        Button::new("navigate-back")
                            .ghost()
                            .size_8()
                            .p_0()
                            .icon(IconName::ArrowLeft)
                            .tooltip(tr("返回"))
                            .disabled(!back)
                            .on_click(
                                cx.listener(|this, _, w, cx| this.navigate_history(false, w, cx)),
                            ),
                    )
                    .child(
                        Button::new("navigate-forward")
                            .ghost()
                            .size_8()
                            .p_0()
                            .icon(IconName::ArrowRight)
                            .tooltip(tr("下一頁"))
                            .disabled(!forward)
                            .on_click(
                                cx.listener(|this, _, w, cx| this.navigate_history(true, w, cx)),
                            ),
                    ),
            )
    }
}

actions!(
    flowmoor_window,
    [CloseWindow, MinimizeWindow, ToggleFullscreen, Quit]
);
pub(super) fn init(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("ctrl-cmd-f", ToggleFullscreen, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
    ]);
}
