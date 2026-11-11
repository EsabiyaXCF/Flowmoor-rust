use super::*;
// Thumbnail theme tokens from Flowmoor glass.css.
const LIGHT_TINT: u32 = 0xf5f9fc;
const DARK_TINT: u32 = 0x060a12;
// Source: Flowmoor glass.css --download-info-thumbnail-mask (mask alpha inverted over the panel).
const LIGHT_MASK: &[(f32, f32)] = &[
    (0., 0.),
    (0.09, 0.006),
    (0.18, 0.025),
    (0.29, 0.07),
    (0.40, 0.16),
    (0.52, 0.34),
    (0.64, 0.58),
    (0.75, 0.8),
    (0.84, 0.94),
    (0.92, 1.),
    (1., 1.),
];
const DARK_MASK: &[(f32, f32)] = &[
    (0., 0.),
    (0.05, 0.),
    (0.11, 0.03),
    (0.18, 0.1),
    (0.27, 0.24),
    (0.38, 0.46),
    (0.49, 0.7),
    (0.60, 0.88),
    (0.70, 0.97),
    (0.78, 1.),
    (1., 1.),
];
impl DownloadApp {
    pub(super) fn fetch_info_images(&mut self) {
        let Some(info) = &self.info else {
            return;
        };
        if info.thumbnail.is_none()
            && let Some(url) = info.thumbnail_url.clone()
        {
            let (tx, rx) = mpsc::channel();
            self.pending_thumbnail = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(metadata::fetch_thumbnail(&url));
            });
        }
        if info.avatar.is_none() {
            let profile = info.clone();
            let (tx, rx) = mpsc::channel();
            self.pending_avatar = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(metadata::channel_avatar(&profile, &Tools::default()));
            });
        }
    }

    pub(super) fn video_info_view(
        &self,
        info: &MediaInfo,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let content_width =
            f32::from(window.viewport_size().width) - self.sidebar_width(window) - 66.;
        let hero_height = (content_width * 0.16).clamp(120., 172.);
        let title_size = (13.44 + content_width * 0.0082).clamp(17.92, 25.92);
        let surface = cx.theme().sidebar;
        let muted = cx.theme().muted_foreground;
        let mut card = div()
            .id("video-info")
            .relative()
            .w_full()
            .min_h(px(hero_height))
            .overflow_hidden()
            .bg(surface);
        if let Some(path) = &info.thumbnail {
            card = card.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(relative(0.50))
                    .overflow_hidden()
                    .child(
                        img(path.clone())
                            .size_full()
                            .object_fit(ObjectFit::Cover)
                            .opacity(if self.dark { 0.12 } else { 0.08 }),
                    )
                    .child(div().absolute().inset_0().bg(linear_gradient(
                        90.,
                        linear_color_stop(surface, 0.08),
                        linear_color_stop(surface.opacity(0.), 1.),
                    ))),
            );
            let mut thumbnail = div()
                .id("thumbnail-preview")
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(relative(0.30))
                .overflow_hidden()
                .cursor_pointer()
                .role(Role::Button)
                .aria_label(if self.preview.is_some() {
                    "點擊縮圖收合預覽"
                } else {
                    "點擊縮圖展開預覽"
                })
                .child(img(path.clone()).size_full().object_fit(ObjectFit::Cover));
            let tint = if self.dark {
                rgb(DARK_TINT)
            } else {
                rgb(LIGHT_TINT)
            };
            thumbnail = thumbnail.child(div().absolute().inset_0().bg(linear_gradient(
                180.,
                linear_color_stop(tint.opacity(if self.dark { 0.08 } else { 0.04 }), 0.),
                linear_color_stop(tint.opacity(if self.dark { 0.72 } else { 0.22 }), 1.),
            )));
            // shortcut: Two-stop GPUI gradients approximate the CSS 100deg mask horizontally; replace when composed masks are available.
            for pair in (if self.dark { DARK_MASK } else { LIGHT_MASK }).windows(2) {
                let (x, a) = pair[0];
                let (next, b) = pair[1];
                thumbnail = thumbnail.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(x))
                        .w(relative(next - x + 0.0005))
                        .bg(linear_gradient(
                            90.,
                            linear_color_stop(surface.opacity(1. - a), 0.),
                            linear_color_stop(surface.opacity(1. - b), 1.),
                        )),
                );
            }
            thumbnail = thumbnail.on_click(cx.listener(|this, _, w, cx| this.open_preview(w, cx)));
            card = card.child(thumbnail);
        }
        let mut copy = div()
            .relative()
            .w(relative(0.72))
            .min_h(px(hero_height))
            .p(px(14.))
            .flex()
            .flex_col()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_size(px(title_size))
                    .font_weight(FontWeight::SEMIBOLD)
                    .line_clamp(2)
                    .child(info.title.clone()),
            );
        if let Some(uploader) = &info.uploader {
            let mut avatar = div()
                .size_8()
                .flex_shrink_0()
                .rounded_full()
                .overflow_hidden()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted)
                .flex()
                .items_center()
                .justify_center();
            avatar = if let Some(path) = &info.avatar {
                avatar.child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
            } else {
                avatar.child(Icon::new(IconName::User).size_4())
            };
            let source = info
                .channel_url
                .clone()
                .or_else(|| info.uploader_url.clone())
                .or_else(|| {
                    info.uploader_id
                        .as_ref()
                        .filter(|_| self.info_url.contains("twitch.tv"))
                        .map(|id| format!("https://www.twitch.tv/{id}"))
                });
            let mut author = div()
                .id("video-author")
                .flex()
                .items_center()
                .gap_2()
                .text_color(muted)
                .child(avatar)
                .child(div().text_sm().truncate().child(uploader.clone()));
            if let Some(source) = source {
                let key_source = source.clone();
                author = author
                    .role(Role::Button)
                    .aria_label(tr("前往頻道"))
                    .tab_index(0)
                    .cursor_pointer()
                    .rounded(px(8.))
                    .focus_visible(|s| s.bg(cx.theme().accent))
                    .hover(|s| s.text_color(cx.theme().foreground))
                    .on_click(cx.listener(move |this, _, w, cx| this.open_uploader(&source, w, cx)))
                    .on_key_down(cx.listener(move |this, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            this.open_uploader(&key_source, w, cx);
                            cx.stop_propagation();
                        }
                    }));
            }
            copy = copy.child(author);
        }
        let duration = if info.is_live {
            "直播中".into()
        } else {
            info.duration
                .map(|s| flowmoor_rust::time_controls::compact_clock(s as u64))
                .unwrap_or_else(|| "長度未知".into())
        };
        let mut stats = div()
            .flex()
            .items_center()
            .gap_4()
            .text_sm()
            .text_color(muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::Clock).size_4())
                    .child(tr(&duration)),
            );
        if let Some(views) = info.view_count {
            stats = stats.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::Eye).size_4())
                    .child(views.to_string()),
            );
        }
        let hero = card.child(copy.child(stats)).child(
            Button::new("open-preview")
                .ghost()
                .absolute()
                .top_2()
                .right_2()
                .w_9()
                .h_9()
                .icon(if self.preview.is_some() {
                    IconName::ArrowDownLeft
                } else {
                    IconName::ArrowUpRight
                })
                .tooltip(tr("播放器預覽"))
                .on_click(cx.listener(|this, _, w, cx| {
                    cx.stop_propagation();
                    this.open_preview(w, cx);
                })),
        );
        div()
            .id("video-info-panel")
            .w_full()
            .flex()
            .flex_col()
            .rounded(px(24.))
            .overflow_hidden()
            .border_1()
            .border_color(cx.theme().border)
            .bg(surface)
            .child(hero)
            .when_some(
                self.preview.clone().filter(|p| !p.read(cx).expanded),
                |d, p| {
                    d.child(
                        div()
                            .id("preview-anchor")
                            .anchor_scroll(Some(self.preview_anchor.clone()))
                            .w_full()
                            .px_4()
                            .pb_4()
                            .child(
                                div()
                                    .h(px(1.))
                                    .mx_4()
                                    .my_4()
                                    .bg(cx.theme().border.opacity(0.5)),
                            )
                            .child(p),
                    )
                },
            )
    }
}

impl DownloadApp {
    fn open_uploader(&mut self, url: &str, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(source) = ChannelSource::parse(url) {
            self.remember_navigation(cx);
            self.url
                .update(cx, |input, cx| input.set_value(source.base.clone(), w, cx));
            self.channel_tab = 0;
            self.channel = Some(source);
            self.channel_page = None;
            self.input_checkpoint = false;
            self.load_channel(false, cx);
        }
    }
}
