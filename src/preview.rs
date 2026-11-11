use crate::tr;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Icon,
        button::*,
        slider::{Slider, SliderEvent, SliderState},
    },
    *,
};
use gpui_kit::{base::Disableable, prelude::FluentBuilder};
use gpui_webview::WebView;
use serde::Deserialize;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Default, Deserialize)]
pub(super) struct Playback {
    pub(super) time: f64,
    pub(super) duration: f64,
    pub(super) playing: bool,
    pub(super) ready: bool,
    pub(super) loaded: bool,
    pub(super) muted: bool,
    volume: f32,
    error: Option<String>,
}
pub struct Preview {
    web: Entity<WebView>,
    _playlist: Option<crate::playlist_server::PlaylistServer>,
    pub(super) state: Playback,
    pub(super) selection: (u64, u64),
    pub(super) viewport: (f64, f64),
    pub(super) timeline_bounds: std::rc::Rc<std::cell::Cell<Bounds<Pixels>>>,
    pub(super) timeline_drag: Option<u8>,
    pub(super) timeline_focus: FocusHandle,
    volume: Entity<SliderState>,
    live: bool,
    pub(super) local: bool,
    pub(super) last_seek: Option<Instant>,
    pub expanded: bool,
    pub(super) inline_height: std::rc::Rc<std::cell::Cell<Pixels>>,
    volume_hover: bool,
    last_volume: f32,
    volume_focus: FocusHandle,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<PreviewEvent> for Preview {}
pub enum PreviewEvent {
    Start(u64),
    End(u64),
    Range(u64, u64),
    Close,
    Expand,
}

impl Preview {
    fn control_button(
        &self,
        id: impl Into<ElementId>,
        icon: IconName,
        label: Option<&'static str>,
        disabled: bool,
        cx: &App,
    ) -> Button {
        let foreground = if matches!(icon, IconName::Play | IconName::Pause) {
            cx.theme().foreground
        } else if cx.theme().mode.is_dark() {
            rgb(0xe8f1fc).into()
        } else {
            cx.theme().muted_foreground
        };
        let hover: Hsla = match icon {
            IconName::Flag => rgb(0x10b981).opacity(0.1).into(),
            IconName::FlagOff => rgb(0xf43f5e).opacity(0.1).into(),
            _ => rgb(if cx.theme().mode.is_dark() {
                0x0a0f18
            } else if self.local || matches!(icon, IconName::Play | IconName::Pause) {
                0xf5f9ff
            } else {
                0xfcfeff
            })
            .into(),
        };
        let hover_foreground = match icon {
            IconName::Flag => rgb(0x10b981).into(),
            IconName::FlagOff => rgb(0xf43f5e).into(),
            _ => cx.theme().foreground,
        };
        let button = Button::new(id)
            .group("preview-control")
            .custom(
                ButtonCustomVariant::new(cx)
                    .foreground(foreground)
                    .hover(hover)
                    .active(hover),
            )
            .h_11()
            .flex_shrink_0()
            .rounded(px(22.))
            .p_0()
            .on_mouse_down(MouseButton::Left, |_, window, cx| window.blur(cx))
            .disabled(disabled)
            .opacity(if disabled { 0.35 } else { 1. });
        if let Some(label) = label {
            button
                .min_w(px(96.))
                .px_3()
                .accessibility_label(tr(label))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_color(foreground)
                        .when(!disabled, |d| {
                            d.group_hover("preview-control", |d| d.text_color(hover_foreground))
                        })
                        .child(Icon::new(icon).size(px(14.)))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(tr(label)),
                        ),
                )
        } else {
            button.w_11().child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(foreground)
                    .when(!disabled, |d| {
                        d.group_hover("preview-control", |d| d.text_color(hover_foreground))
                    })
                    .child(
                        Icon::new(icon)
                            .size(px(16.))
                            .when(matches!(icon, IconName::Play), |i| i.ml(px(2.))),
                    ),
            )
        }
    }

    pub fn new(
        source: &str,
        duration: f64,
        live: bool,
        stream: Option<&flowmoor_rust::twitch::LiveStream>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<Self>, String> {
        let html = |source: &str| -> String {
            let source = serde_json::to_string(source)
                .unwrap()
                .replace('<', "\\u003c");
            format!(
                r#"<!doctype html><html><meta name="viewport" content="width=device-width"><style>html,body{{margin:0;width:100%;height:100%;overflow:hidden;background:black}}video{{width:100%;height:100%;object-fit:contain}}</style><video playsinline preload="auto"></video><script>{}</script><script>const video=document.querySelector('video');const source={source};if(!source.includes('.m3u8')||video.canPlayType('application/vnd.apple.mpegurl'))video.src=source;else if(Hls.isSupported()){{const hls=new Hls({{startPosition:0}});hls.loadSource(source);hls.attachMedia(video);}}video.addEventListener('click',()=>window.flowmoorPreview.toggle());</script></html>"#,
                include_str!("../assets/hls.light.min.js")
            )
        };
        let local = url::Url::parse(source)
            .ok()
            .filter(|u| u.scheme() == "file")
            .map(|u| u.to_file_path().map_err(|_| "無效媒體路徑"))
            .transpose()?;
        let is_local = local.is_some();
        let playlist = if let Some(path) = local {
            Some(crate::playlist_server::PlaylistServer::file(path, html)?)
        } else {
            stream
                .and_then(|s| s.preview_playlist.clone())
                .map(|body| crate::playlist_server::PlaylistServer::new(body, html))
                .transpose()?
        };
        let url = if let Some(server) = &playlist {
            server.url.clone()
        } else if stream.is_some() {
            "about:blank".into()
        } else {
            flowmoor_rust::metadata::preview_url(source)?
        };
        let allowed = url.clone();
        let (tx, rx) = mpsc::sync_channel(4);
        let mut headers = wry::http::HeaderMap::new();
        headers.insert("Referer", "https://localhost/".parse().unwrap());
        let builder = wry::WebViewBuilder::new();
        let builder = if playlist.is_some() {
            builder.with_url(url)
        } else if let Some(stream) = stream {
            builder.with_html(html(&stream.selected_url(None, false)?))
        } else {
            builder.with_url_and_headers(url, headers)
        };
        let builder = builder
            .with_autoplay(true)
            .with_navigation_handler(move |url| url == allowed || url == "about:blank")
            .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
            .with_download_started_handler(|_, _| false)
            .with_initialization_script(include_str!("../assets/preview.js"))
            .with_ipc_handler(move |request| {
                if request.body().len() <= 4096
                    && let Ok(state) = serde_json::from_str::<Playback>(request.body())
                    && state.time.is_finite()
                    && state.time >= 0.0
                    && state.duration.is_finite()
                    && state.duration >= 0.0
                {
                    let _ = tx.try_send(state);
                }
            });
        let view = WebView::build(builder, window, cx).map_err(|e| format!("無法建立預覽：{e}"))?;
        let web = cx.new(|_| view);
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.01)
                .default_value(1.)
        });
        Ok(cx.new(|cx: &mut Context<Self>| {
            let subscriptions = vec![cx.subscribe(&volume, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    this.set_volume(value.end(), cx);
                }
            })];
            cx.spawn_in(window, async move |this, cx| {
                let started = Instant::now();
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(250))
                        .await;
                    if this
                        .update_in(cx, |this, window, cx| {
                            let mut latest = None;
                            while let Ok(state) = rx.try_recv() {
                                latest = Some(state);
                            }
                            if let Some(mut state) = latest {
                                if state.duration == 0. {
                                    state.duration = this.state.duration;
                                }
                                if state.ready {
                                    this.error = None;
                                }
                                if let Some(error) = &state.error {
                                    this.error = Some(error.clone());
                                }
                                let volume = if state.muted { 0. } else { state.volume };
                                if this.volume.read(cx).value().end() != volume {
                                    this.volume
                                        .update(cx, |s, cx| s.set_value(volume, window, cx));
                                }
                                this.state = state;
                                cx.notify();
                            }
                            if !this.state.ready
                                && this.error.is_none()
                                && started.elapsed() > Duration::from_secs(30)
                            {
                                this.error =
                                    Some("預覽尚未就緒，可重新開啟；來源可能限制嵌入播放。".into());
                                cx.notify();
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
            Self {
                web,
                _playlist: playlist,
                state: Playback {
                    duration,
                    ..Default::default()
                },
                selection: (0, duration.ceil() as u64),
                viewport: (0., duration),
                timeline_bounds: Default::default(),
                timeline_drag: None,
                timeline_focus: cx.focus_handle(),
                volume,
                live,
                local: is_local,
                last_seek: None,
                expanded: false,
                inline_height: Default::default(),
                volume_hover: false,
                last_volume: 1.,
                volume_focus: cx.focus_handle(),
                error: None,
                _subscriptions: subscriptions,
            }
        }))
    }
    fn command(&mut self, script: &str, cx: &mut Context<Self>) {
        let script = [
            "(()=>{const v=document.querySelector('video');",
            script,
            "})()",
        ]
        .concat();
        if let Err(e) = self.web.read(cx).raw().evaluate_script(&script) {
            self.error = Some(e.to_string());
        }
        cx.notify();
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.timeline_focus.focus(window, cx);
    }
    pub fn hide(&self, cx: &mut App) {
        self.web.update(cx, |v, _| {
            let _ = v
                .raw()
                .evaluate_script("document.querySelector('video')?.pause()");
            v.hide();
        });
    }
    pub fn set_range(
        &mut self,
        start: u64,
        end: u64,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selection = (start, end);
        cx.notify();
    }
    pub(super) fn seek_to(&mut self, time: f64, cx: &mut Context<Self>) {
        self.last_seek = Some(Instant::now());
        self.state.time = time;
        self.command(&format!("window.flowmoorPreview.seek({time})"), cx);
    }
}
fn clock(seconds: f64) -> String {
    let s = seconds.max(0.) as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}
impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let timeline = if !self.live {
            Some(self.timeline(window, cx))
        } else {
            None
        };
        let measured = self.inline_height.clone();
        let expanded = self.expanded;
        div()
            .id("preview-root")
            .relative()
            .child(
                canvas(
                    move |bounds, _, _| {
                        if !expanded {
                            measured.set(bounds.size.height);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .capture_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && this.expanded {
                    cx.emit(PreviewEvent::Expand);
                    cx.stop_propagation();
                }
            }))
            .flex()
            .flex_col()
            .gap(px(12.))
            .w_full()
            .child(
                div()
                    .w_full()
                    .aspect_ratio(if self.expanded { 1.8 } else { 16. / 9. })
                    .when(self.expanded, |d| {
                        d.h((window.viewport_size().height - px(180.)).max(px(180.)))
                    })
                    .max_h(if self.expanded {
                        window.viewport_size().height - px(150.)
                    } else {
                        px(560.)
                    })
                    .rounded(px(12.))
                    .overflow_hidden()
                    .flex_shrink_0()
                    .child(self.web.clone()),
            )
            .when_some(timeline, |d, t| d.child(t))
            .child(if self.local {
                self.local_controls(window, cx)
            } else {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .px_2()
                    .py_1p5()
                    .rounded(px(16.))
                    .bg(rgb(if cx.theme().mode.is_dark() {
                        0x0a0f18
                    } else {
                        0xfcfeff
                    })
                    .opacity(0.8))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(self.volume_control(window, cx))
                            .child(
                                self.control_button(
                                    "preview-expand",
                                    if self.expanded {
                                        IconName::Minimize2
                                    } else {
                                        IconName::Maximize2
                                    },
                                    None,
                                    !self.state.ready
                                        || self.volume_hover
                                        || self.volume_focus.contains_focused(window, cx),
                                    cx,
                                )
                                .when(
                                    self.volume_hover
                                        || self.volume_focus.contains_focused(window, cx),
                                    |b| b.opacity(0.),
                                )
                                .accessibility_label(tr("放大／還原預覽"))
                                .tooltip(tr("放大／還原預覽"))
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(PreviewEvent::Expand))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                self.control_button(
                                    "preview-back",
                                    IconName::SkipBack,
                                    None,
                                    self.live || !self.state.ready,
                                    cx,
                                )
                                .accessibility_label(tr("前往起點"))
                                .tooltip(tr("前往起點"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| this.seek_to(this.selection.0 as f64, cx),
                                )),
                            )
                            .child(
                                self.control_button(
                                    "preview-start",
                                    IconName::Flag,
                                    Some("設為起點"),
                                    self.live || !self.state.ready,
                                    cx,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        cx.emit(PreviewEvent::Start(this.state.time.floor() as u64))
                                    },
                                )),
                            )
                            .child(
                                self.control_button(
                                    "preview-play",
                                    if self.state.playing {
                                        IconName::Pause
                                    } else {
                                        IconName::Play
                                    },
                                    None,
                                    !self.state.loaded,
                                    cx,
                                )
                                .accessibility_label(tr("播放／暫停"))
                                .tooltip(tr("播放／暫停"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.command("window.flowmoorPreview.toggle()", cx)
                                    },
                                )),
                            )
                            .child(
                                self.control_button(
                                    "preview-end",
                                    IconName::FlagOff,
                                    Some("設為終點"),
                                    self.live || !self.state.ready,
                                    cx,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        cx.emit(PreviewEvent::End(this.state.time.ceil() as u64))
                                    },
                                )),
                            )
                            .child(
                                self.control_button(
                                    "preview-forward",
                                    IconName::SkipForward,
                                    None,
                                    self.live || !self.state.ready,
                                    cx,
                                )
                                .accessibility_label(tr("前往終點"))
                                .tooltip(tr("前往終點"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| this.seek_to(this.selection.1 as f64, cx),
                                )),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_1()
                            .child(div().text_xs().child(if self.live {
                                "直播中".into()
                            } else {
                                format!(
                                    "{} / {}",
                                    clock(self.state.time),
                                    clock(self.state.duration)
                                )
                            }))
                            .child(
                                self.control_button("preview-close", IconName::X, None, false, cx)
                                    .accessibility_label(tr("關閉預覽"))
                                    .tooltip(tr("關閉預覽"))
                                    .on_click(
                                        cx.listener(|_, _, _, cx| cx.emit(PreviewEvent::Close)),
                                    ),
                            ),
                    )
            })
            .when_some(self.error.clone(), |d, e| {
                d.child(div().text_sm().text_color(cx.theme().danger).child(e))
            })
    }
}

impl Preview {
    fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        if volume > 0. {
            self.last_volume = volume;
        }
        self.command(
            &format!("if(v){{v.volume={volume};v.muted={}}}", volume == 0.),
            cx,
        );
    }
    fn volume_control(&self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let expanded = self.volume_hover || self.volume_focus.contains_focused(window, cx);
        let slider = div().w(px(160.)).pl_3().pr_4().child(
            Slider::new(&self.volume)
                .disabled(!self.state.ready)
                .accessibility_label(tr("音量")),
        );
        let slider = if expanded {
            slider
                .with_animation(
                    "volume-open",
                    Animation::new(Duration::from_millis(200)),
                    |d, p| d.opacity(p),
                )
                .into_any_element()
        } else {
            slider.opacity(0.).into_any_element()
        };
        div()
            .id("preview-volume")
            .flex()
            .items_center()
            .h(px(46.))
            .track_focus(&self.volume_focus)
            .tab_index(0)
            .rounded(px(23.))
            .border_1()
            .border_color(cx.theme().border.opacity(0.5))
            .bg(cx.theme().sidebar.opacity(0.7))
            .overflow_hidden()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.volume_hover = *hovered;
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, w, cx| {
                let step = match e.keystroke.key.as_str() {
                    "left" | "down" => -0.01,
                    "right" | "up" => 0.01,
                    _ => return,
                };
                let value = (this.volume.read(cx).value().end() + step).clamp(0., 1.);
                this.volume.update(cx, |s, cx| s.set_value(value, w, cx));
                this.set_volume(value, cx);
                cx.stop_propagation();
            }))
            .child(
                Button::new("preview-mute")
                    .custom(ButtonCustomVariant::new(cx).foreground(cx.theme().muted_foreground))
                    .group("preview-mute")
                    .size_11()
                    .rounded(px(22.))
                    .p_0()
                    .disabled(!self.state.ready)
                    .child(
                        div()
                            .size_9()
                            .rounded(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(if cx.theme().mode.is_dark() {
                                rgb(0xe8f1fc).into()
                            } else {
                                cx.theme().muted_foreground
                            })
                            .opacity(if self.state.ready { 1. } else { 0.35 })
                            .when(self.state.ready, |d| {
                                d.group_hover("preview-mute", |d| {
                                    d.bg(rgb(if cx.theme().mode.is_dark() {
                                        0x0a0f18
                                    } else {
                                        0xfcfeff
                                    }))
                                    .text_color(cx.theme().foreground)
                                })
                            })
                            .child(
                                Icon::new(if self.state.muted || self.state.volume == 0. {
                                    IconName::VolumeX
                                } else if self.state.volume < 0.5 {
                                    IconName::Volume1
                                } else {
                                    IconName::Volume2
                                })
                                .size(px(18.)),
                            ),
                    )
                    .accessibility_label(tr("靜音／取消靜音"))
                    .tooltip(tr("靜音／取消靜音"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let volume = if this.state.muted || this.state.volume == 0. {
                            this.last_volume
                        } else {
                            0.
                        };
                        this.set_volume(volume, cx);
                    })),
            )
            .child(
                div()
                    .w(px(if expanded { 160. } else { 0. }))
                    .overflow_hidden()
                    .child(slider),
            )
    }
    fn local_controls(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let expanded = self.volume_hover || self.volume_focus.contains_focused(window, cx);
        let mut playback = div().flex().items_center().gap_1p5();
        for (index, icon, label) in [
            (0, IconName::SkipBack, "前往起點"),
            (
                1,
                if self.state.playing {
                    IconName::Pause
                } else {
                    IconName::Play
                },
                "播放／暫停",
            ),
            (2, IconName::SkipForward, "前往終點"),
        ] {
            playback = playback.child(
                self.control_button(
                    ("local-playback", index as usize),
                    icon,
                    None,
                    !self.state.ready,
                    cx,
                )
                .accessibility_label(tr(label))
                .tooltip(tr(label))
                .on_click(cx.listener(move |this, _, _, cx| match index {
                    0 => this.seek_to(0., cx),
                    1 => this.command("window.flowmoorPreview.toggle()", cx),
                    _ => this.seek_to(this.state.duration, cx),
                })),
            );
        }
        div()
            .w_full()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_1p5()
            .rounded(px(16.))
            .bg(rgb(if cx.theme().mode.is_dark() {
                0x0a0f18
            } else {
                0xfcfeff
            })
            .opacity(0.8))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(self.volume_control(window, cx))
                    .child(
                        self.control_button(
                            "local-expand",
                            if self.expanded {
                                IconName::Minimize2
                            } else {
                                IconName::Maximize2
                            },
                            None,
                            !self.state.ready || expanded,
                            cx,
                        )
                        .when(expanded, |b| b.opacity(0.))
                        .accessibility_label(tr("放大／還原預覽"))
                        .tooltip(tr("放大／還原預覽"))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(PreviewEvent::Expand))),
                    ),
            )
            .child(playback)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .justify_end()
                    .text_size(px(11.))
                    .font_family("Menlo")
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "{} / {}",
                        clock(self.state.time),
                        clock(self.state.duration)
                    )),
            )
    }
}
