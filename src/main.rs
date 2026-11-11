#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use gpui_kit::{base::Disableable, prelude::FluentBuilder};
mod channel_view;
mod library_view;
mod queue_view;
mod settings_view;
use flowmoor_rust::i18n::text as tr;
mod playlist_server;
mod preview;
mod preview_timeline;
mod twitch_channel_view;
use flowmoor_rust::channel::{self, ChannelSource};
mod chrome_view;
mod dropdown_controls;
mod flowmoor_assets;
mod hover_glow;
mod menu_view;
mod output_controls;
mod paste_control;
mod sidebar_view;
mod theme;
mod time_fields;
mod video_info_view;
use flowmoor_rust::preferences::{Preferences, Preset};
use flowmoor_rust::{
    download::{
        AudioCodec, Content, FormatOptions, Manager, Phase, Request, Tools, VideoCodec, parse_time,
    },
    metadata::{self, MediaInfo},
};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Icon, IndexPath, Selectable, WindowExt,
        button::*,
        input::{Input, InputEvent, InputState},
        select::{SelectEvent, SelectState},
    },
    *,
};
use preview::{Preview, PreviewEvent};
use std::{path::PathBuf, sync::mpsc, time::Duration};
use time_fields::TimeDrag;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Download,
    Queue,
    Settings,
    Library,
}
struct DownloadApp {
    manager: Manager,
    url: Entity<InputState>,
    output: Entity<InputState>,
    open_menu: Option<Entity<menu_view::FlowmoorMenu>>,
    menu_subscription: Option<Subscription>,
    menu_focus: std::cell::RefCell<std::collections::HashMap<menu_view::MenuKind, FocusHandle>>,
    menu_anchors: std::rc::Rc<
        std::cell::RefCell<std::collections::HashMap<menu_view::MenuKind, Bounds<Pixels>>>,
    >,
    start: Entity<InputState>,
    end: Entity<InputState>,
    retries: Entity<InputState>,
    cookies: Entity<InputState>,
    contents: Vec<Content>,
    video_codec: Entity<SelectState<Vec<SharedString>>>,
    audio_codec: Entity<SelectState<Vec<SharedString>>>,
    presets: Entity<SelectState<Vec<SharedString>>>,
    preset_name: Entity<InputState>,
    session_paths: Vec<String>,
    preferences: Preferences,
    preview: Option<Entity<Preview>>,
    preview_subscription: Option<Subscription>,
    quality: Entity<SelectState<Vec<SharedString>>>,
    filename: Entity<InputState>,
    live_from_start: bool,
    time_drag: Option<TimeDrag>,
    edit_time: Option<bool>,
    edit_original: [String; 2],
    time_focus: [FocusHandle; 2],
    workspace_scroll: ScrollHandle,
    preview_anchor: ScrollAnchor,
    error: Option<String>,
    page: Page,
    navigation: flowmoor_rust::navigation::History<chrome_view::NavigationSnapshot>,
    page_scroll: [Point<Pixels>; 4],
    input_checkpoint: bool,
    sidebar_glow: hover_glow::HoverGlow,
    settings_glow: hover_glow::HoverGlow,
    settings_nav_glow: hover_glow::HoverGlow,
    library_glow: hover_glow::HoverGlow,
    collapsed: bool,
    download_group_open: bool,
    dark: bool,
    paste_progress: std::rc::Rc<std::cell::Cell<f32>>,
    paste_from: f32,
    paste_hover: bool,
    paste_revision: usize,
    info: Option<MediaInfo>,
    info_url: String,
    pending: Option<mpsc::Receiver<Result<MediaInfo, String>>>,
    pending_thumbnail: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    library: flowmoor_rust::library::Library,
    library_load_error: Option<String>,
    library_search: Entity<InputState>,
    library_sort_title: bool,
    library_selected: Option<u128>,
    library_menu_id: Option<u128>,
    archived_ids: std::collections::HashSet<u128>,
    library_edit_request: Option<Request>,
    library_preparing: Option<mpsc::Receiver<Result<(PathBuf, f64), String>>>,
    library_preview: Option<Entity<Preview>>,
    library_subscription: Option<Subscription>,
    job_metadata: std::collections::HashMap<u128, (Option<PathBuf>, Option<String>)>,
    settings_tab: usize,
    settings_original: Option<(Preferences, Vec<String>)>,
    pending_avatar: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    channel_load_failed: bool,
    channel: Option<ChannelSource>,
    channel_page: Option<channel::Page>,
    channel_tab: usize,
    pending_channel: Option<mpsc::Receiver<Result<channel::Page, String>>>,
    channel_append: bool,
    showing_channel: bool,
    twitch_vods_open: bool,
    channel_glow: hover_glow::HoverGlow,
    channel_hovered: Option<usize>,
    channel_images: Option<mpsc::Receiver<(Option<String>, PathBuf)>>,
    _subscriptions: Vec<Subscription>,
}
fn field(label: &'static str, input: &Entity<InputState>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_2()
        .child(tr(label))
        .child(Input::new(input).aria_label(tr(label)).h_11())
}
impl DownloadApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = Preferences::load(&Preferences::path());
        let mut load_error = settings.as_ref().err().cloned();
        let preferences = settings.unwrap_or_else(|_| Preferences {
            retries: 3,
            ..Default::default()
        });
        flowmoor_rust::i18n::set_language(&preferences.language);
        let dark = preferences.dark;
        theme::apply(dark, cx);
        let url = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("貼上 YouTube 或 Twitch 影片連結..."))
        });
        let output = cx.new(|cx| InputState::new(window, cx));
        let path = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| ".".into())
            .join("Downloads");
        output.update(cx, |s, cx| {
            s.set_value(path.to_string_lossy().into_owned(), window, cx)
        });
        let start = cx.new(|cx| InputState::new(window, cx).placeholder("00:00:00"));
        let end = cx.new(|cx| InputState::new(window, cx).placeholder(tr("完整影片")));
        let retries = cx.new(|cx| InputState::new(window, cx).placeholder("0–10"));
        retries.update(cx, |s, cx| s.set_value("3", window, cx));
        let cookies =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("選填 Netscape cookies.txt")));
        if !preferences.output.is_empty() {
            output.update(cx, |s, cx| {
                s.set_value(preferences.output.clone(), window, cx)
            });
        }
        cookies.update(cx, |s, cx| {
            s.set_value(preferences.cookies.clone(), window, cx)
        });
        retries.update(cx, |s, cx| {
            s.set_value(preferences.retries.to_string(), window, cx)
        });
        let video_codec = cx.new(|cx| {
            SelectState::new(
                vec![
                    SharedString::from("自動"),
                    "H.264".into(),
                    "VP9".into(),
                    "AV1".into(),
                ],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let audio_codec = cx.new(|cx| {
            SelectState::new(
                vec![
                    SharedString::from("自動"),
                    "MP3".into(),
                    "AAC".into(),
                    "Opus".into(),
                ],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let presets = cx.new(|cx| {
            SelectState::new(
                preferences
                    .presets
                    .iter()
                    .map(|p| SharedString::from(p.name.clone()))
                    .collect::<Vec<_>>(),
                None,
                window,
                cx,
            )
        });
        let preset_name = cx.new(|cx| InputState::new(window, cx).placeholder(tr("預設名稱")));
        let filename = cx.new(|cx| InputState::new(window, cx).placeholder(tr("自動產生")));
        let quality = cx.new(|cx| {
            SelectState::new(
                vec![SharedString::from("最佳可用畫質")],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let mut subscriptions = vec![
            cx.subscribe_in(
                &presets,
                window,
                |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                    if let SelectEvent::Confirm(Some(name)) = event {
                        this.apply_preset(name, window, cx);
                    }
                },
            ),
            cx.subscribe_in(&start, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_preview_range(window, cx);
                }
            }),
            cx.subscribe_in(&end, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_preview_range(window, cx);
                }
            }),
            cx.subscribe_in(&url, window, |this, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => this.inspect(window, cx),
                InputEvent::Change => {
                    if this.info.is_some() || this.channel_page.is_some() {
                        this.remember_navigation(cx);
                        this.input_checkpoint = true;
                    }
                    this.close_preview(cx);
                    this.live_from_start = false;
                    this.start.update(cx, |s, cx| s.set_value("", window, cx));
                    this.end.update(cx, |s, cx| s.set_value("", window, cx));
                    this.info = None;
                    this.library_edit_request = None;
                    this.pending_avatar = None;
                    this.pending = None;
                    this.pending_thumbnail = None;
                    this.pending_channel = None;
                    this.channel = None;
                    this.channel_page = None;
                    this.showing_channel = false;
                    this.error = None;
                    cx.notify();
                }
                _ => {}
            }),
        ];
        for (input, start_field) in [(&start, true), (&end, false)] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |this, input, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. })
                        && this.edit_time == Some(start_field)
                    {
                        let value = input.read(cx).value().trim().to_string();
                        if value.is_empty() {
                            this.edit_time = None;
                            this.error = None;
                            this.sync_preview_range(window, cx);
                        } else if let Ok(value) = parse_time(&value) {
                            this.adjust_time(start_field, value, window, cx);
                            this.edit_time = None;
                            this.error = None;
                        } else {
                            this.error = Some("時間格式為 HH:MM:SS".into());
                        }
                        cx.notify();
                    }
                },
            ));
        }
        for selector in [&quality, &video_codec, &audio_codec] {
            subscriptions.push(cx.subscribe_in(
                selector,
                window,
                |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                    if matches!(event, SelectEvent::Confirm(_)) {
                        this.presets
                            .update(cx, |s, cx| s.set_selected_index(None, window, cx));
                        cx.notify();
                    }
                },
            ));
        }
        cx.spawn_in(window, async move |this, cx| {
            let mut previous = Vec::new();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| {
                        if let Some(result) = this
                            .pending_thumbnail
                            .as_ref()
                            .and_then(|r| r.try_recv().ok())
                        {
                            this.pending_thumbnail = None;
                            if let Ok(path) = result
                                && let Some(info) = this.info.as_mut()
                            {
                                info.thumbnail = Some(path);
                                cx.notify();
                            }
                        }
                        if let Some(result) =
                            this.pending_avatar.as_ref().and_then(|r| r.try_recv().ok())
                        {
                            this.pending_avatar = None;
                            if let Ok(path) = result
                                && let Some(info) = this.info.as_mut()
                            {
                                info.avatar = Some(path);
                                cx.notify();
                            }
                        }
                        if let Some(result) = this
                            .library_preparing
                            .as_ref()
                            .and_then(|r| r.try_recv().ok())
                        {
                            this.library_preparing = None;
                            match result {
                                Ok((path, duration)) => {
                                    this.show_library_player(path, duration, window, cx)
                                }
                                Err(error) => this.error = Some(error),
                            }
                            cx.notify();
                        }
                        if let Some(result) = this.pending.as_ref().and_then(|r| r.try_recv().ok())
                        {
                            this.pending = None;
                            match result {
                                Ok(info) => {
                                    let mut heights: Vec<_> = info
                                        .formats
                                        .iter()
                                        .filter(|f| {
                                            f.vcodec
                                                .as_deref()
                                                .is_some_and(|c| c != "none" && c != "images")
                                        })
                                        .filter_map(|f| f.height)
                                        .filter(|h| *h > 0)
                                        .collect();
                                    heights.sort_unstable_by(|a, b| b.cmp(a));
                                    heights.dedup();
                                    let mut items = vec![SharedString::from("最佳可用畫質")];
                                    items.extend(
                                        heights
                                            .into_iter()
                                            .map(|h| SharedString::from(format!("{h}p"))),
                                    );
                                    this.quality.update(cx, |s, cx| {
                                        s.set_items(items, window, cx);
                                        s.set_selected_index(Some(IndexPath::new(0)), window, cx);
                                    });
                                    if let Some(url) = &info.webpage_url {
                                        this.info_url = url.clone();
                                    }
                                    this.info = Some(info);
                                    this.fetch_info_images();
                                    if let Some(request) = this.library_edit_request.take() {
                                        if let Some(height) = request.format.max_height {
                                            let value = SharedString::from(format!("{height}p"));
                                            this.quality.update(cx, |v, cx| {
                                                v.set_selected_value(&value, window, cx)
                                            });
                                        }
                                        for (state, value) in [
                                            (
                                                &this.video_codec,
                                                match request.format.video_codec {
                                                    VideoCodec::Auto => "自動",
                                                    VideoCodec::H264 => "H.264",
                                                    VideoCodec::Vp9 => "VP9",
                                                    VideoCodec::Av1 => "AV1",
                                                },
                                            ),
                                            (
                                                &this.audio_codec,
                                                match request.format.audio_codec {
                                                    AudioCodec::Auto => "自動",
                                                    AudioCodec::Mp3 => "MP3",
                                                    AudioCodec::Aac => "AAC",
                                                    AudioCodec::Opus => "Opus",
                                                },
                                            ),
                                        ] {
                                            state.update(cx, |v, cx| {
                                                v.set_selected_value(
                                                    &SharedString::from(value),
                                                    window,
                                                    cx,
                                                )
                                            });
                                        }
                                        this.live_from_start = request.live_from_start;
                                    }

                                    if let Some(name) =
                                        this.presets.read(cx).selected_value().cloned()
                                    {
                                        this.apply_preset(&name, window, cx);
                                    }
                                    this.error = None;
                                }
                                Err(e) => this.error = Some(e),
                            }
                            cx.notify();
                        }
                        if let Some(result) = this
                            .pending_channel
                            .as_ref()
                            .and_then(|r| r.try_recv().ok())
                        {
                            this.pending_channel = None;
                            match result {
                                Ok(page) => {
                                    if this.channel_append
                                        && let Some(current) = &mut this.channel_page
                                    {
                                        current.entries.extend(
                                            page.entries
                                                .into_iter()
                                                .filter(|e| {
                                                    !current
                                                        .entries
                                                        .iter()
                                                        .any(|old| old.id == e.id)
                                                })
                                                .collect::<Vec<_>>(),
                                        );
                                        current.next = page.next;
                                        current.has_more = page.has_more;
                                        current.live = page.live;
                                    } else {
                                        this.channel_page = Some(page);
                                    }
                                    this.error = None;
                                    this.fetch_channel_images();
                                }
                                Err(e) => {
                                    this.error = Some(e);
                                    this.channel_load_failed = true;
                                }
                            }
                            cx.notify();
                        }
                        while let Some((id, path)) =
                            this.channel_images.as_ref().and_then(|r| r.try_recv().ok())
                        {
                            if let Some(page) = &mut this.channel_page {
                                if let Some(id) = id {
                                    if let Some(entry) =
                                        page.entries.iter_mut().find(|e| e.id == id)
                                    {
                                        entry.image = Some(path);
                                    }
                                } else {
                                    page.avatar = Some(path);
                                }
                                cx.notify();
                            }
                        }
                        let current: Vec<_> =
                            this.manager.jobs.iter().map(|j| j.snapshot()).collect();
                        if current != previous {
                            let mut changed = false;
                            for job in &this.manager.jobs {
                                let result = job.snapshot();
                                if result.phase.terminal()
                                    && result.attempt > 0
                                    && this.archived_ids.insert(result.id)
                                {
                                    let metadata = this
                                        .job_metadata
                                        .get(&result.id)
                                        .cloned()
                                        .unwrap_or_default();
                                    this.library.remember(flowmoor_rust::library::Entry {
                                        request: job.request.clone(),
                                        completed_at: Some(
                                            (time::OffsetDateTime::now_utc().unix_timestamp_nanos()
                                                / 1_000_000)
                                                as u128,
                                        ),
                                        result,
                                        thumbnail: metadata.0,
                                        uploader: metadata.1,
                                    });
                                    changed = true;
                                }
                            }
                            if changed && this.library_load_error.is_none() {
                                this.error = this
                                    .library
                                    .save(&Preferences::path().with_file_name("library.json"))
                                    .err();
                            }
                            previous = current;
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
        let library_search =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("搜尋標題或頻道")));
        subscriptions.push(cx.subscribe(&library_search, |_, _, _: &InputEvent, cx| cx.notify()));
        let workspace_scroll = ScrollHandle::new();
        let mut library_load_error = None;
        let library = flowmoor_rust::library::Library::load(
            &Preferences::path().with_file_name("library.json"),
        )
        .unwrap_or_else(|e| {
            library_load_error = Some(e.clone());
            load_error = Some(e);
            Default::default()
        });
        Self {
            contents: vec![Content::VideoAudio],
            video_codec,
            audio_codec,
            presets,
            preset_name,
            session_paths: if preferences
                .recent_paths
                .contains(&output.read(cx).value().to_string())
            {
                vec![]
            } else {
                vec![output.read(cx).value().to_string()]
            },
            preferences,
            preview: None,
            preview_subscription: None,
            quality,
            filename,
            manager: Manager::new(Tools::default()),
            url,
            output,
            open_menu: None,
            menu_subscription: None,
            menu_focus: Default::default(),
            menu_anchors: Default::default(),
            start,
            end,
            retries,
            cookies,
            live_from_start: false,
            time_drag: None,
            edit_time: None,
            edit_original: [String::new(), String::new()],
            time_focus: [cx.focus_handle(), cx.focus_handle()],
            preview_anchor: ScrollAnchor::for_handle(workspace_scroll.clone()),
            workspace_scroll,
            error: load_error,
            page: Page::Download,
            navigation: Default::default(),
            page_scroll: Default::default(),
            input_checkpoint: false,
            sidebar_glow: Default::default(),
            settings_glow: Default::default(),
            settings_nav_glow: Default::default(),
            library_glow: Default::default(),
            collapsed: false,
            download_group_open: true,
            dark,
            paste_progress: Default::default(),
            paste_from: 0.,
            paste_hover: false,
            paste_revision: 0,
            info: None,
            info_url: String::new(),
            pending: None,
            pending_thumbnail: None,
            library,
            library_load_error,
            library_search,
            library_sort_title: false,
            library_selected: None,
            library_menu_id: None,
            archived_ids: Default::default(),
            library_edit_request: None,
            library_preparing: None,
            library_preview: None,
            library_subscription: None,
            job_metadata: Default::default(),
            settings_tab: 0,
            settings_original: None,
            pending_avatar: None,
            channel_load_failed: false,
            channel: None,
            channel_page: None,
            channel_tab: 0,
            pending_channel: None,
            channel_append: false,
            showing_channel: false,
            twitch_vods_open: true,
            channel_glow: Default::default(),
            channel_hovered: None,
            channel_images: None,
            _subscriptions: subscriptions,
        }
    }
    fn request(&self, cx: &App) -> Result<Request, String> {
        let a = self.start.read(cx).value();
        let b = self.end.read(cx).value();
        let range = if a.trim().is_empty() && b.trim().is_empty() {
            None
        } else {
            Some((
                if a.trim().is_empty() {
                    0
                } else {
                    parse_time(&a)?
                },
                if b.trim().is_empty() {
                    self.info
                        .as_ref()
                        .and_then(|i| i.duration)
                        .filter(|_| self.info.as_ref().is_some_and(MediaInfo::seekable))
                        .map(|d| d.ceil() as u64)
                        .ok_or("請填寫結束時間")?
                } else {
                    parse_time(&b)?
                },
            ))
        };
        let cookies = self.cookies.read(cx).value();
        let content = *self.contents.first().ok_or("請至少選擇一種內容")?;
        let max_height = self
            .quality
            .read(cx)
            .selected_value()
            .and_then(|v| v.strip_suffix('p'))
            .and_then(|v| v.parse().ok());
        Ok(Request {
            format: FormatOptions {
                content,
                max_height,
                filename: self.filename.read(cx).value().to_string(),
                video_codec: match self
                    .video_codec
                    .read(cx)
                    .selected_value()
                    .map(|v| v.as_ref())
                {
                    Some("H.264") => VideoCodec::H264,
                    Some("VP9") => VideoCodec::Vp9,
                    Some("AV1") => VideoCodec::Av1,
                    _ => VideoCodec::Auto,
                },
                audio_codec: match self
                    .audio_codec
                    .read(cx)
                    .selected_value()
                    .map(|v| v.as_ref())
                {
                    Some("MP3") => AudioCodec::Mp3,
                    Some("AAC") => AudioCodec::Aac,
                    Some("Opus") => AudioCodec::Opus,
                    _ => AudioCodec::Auto,
                },
            },
            url: self.info_url.clone(),
            output: PathBuf::from(self.output.read(cx).value().trim()),
            range: range.or_else(|| {
                self.info
                    .as_ref()
                    .filter(|i| i.twitch_stream.as_ref().is_some_and(|s| s.dvr.is_some()))
                    .and_then(|i| i.duration)
                    .map(|d| (0, d.floor() as u64))
            }),
            live_from_start: self.live_from_start,
            is_live: self.info.as_ref().is_some_and(|i| i.is_live),
            retries: self
                .retries
                .read(cx)
                .value()
                .trim()
                .parse()
                .map_err(|_| "重試次數必須介於 0–10")?,
            cookies: (!cookies.trim().is_empty()).then(|| cookies.trim().into()),
        })
    }
    fn inspect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.pending_channel.is_some() {
            return;
        }
        if self.url.read(cx).value().trim().is_empty()
            && let Some(text) = cx.read_from_clipboard().and_then(|c| c.text())
        {
            self.url.update(cx, |s, cx| s.set_value(text, window, cx));
        }
        let url = self.url.read(cx).value().trim().to_string();
        if url.starts_with('{') {
            match serde_json::from_str::<Request>(&url)
                .map_err(|e| e.to_string())
                .and_then(|request| {
                    request.validate()?;
                    Ok(request)
                }) {
                Ok(request) => self.load_download_request(request, window, cx),
                Err(error) => {
                    self.error = Some(format!("{}：{error}", tr("無效的下載參數")));
                    cx.notify();
                }
            }
            return;
        }
        if let Some(source) = ChannelSource::parse(&url) {
            if !self.input_checkpoint {
                self.remember_navigation(cx);
            }
            self.input_checkpoint = false;
            self.channel_tab = source.tab;
            self.channel = Some(source);
            self.channel_page = None;
            self.load_channel(false, cx);
            return;
        }
        self.channel = None;
        self.channel_page = None;
        self.pending_channel = None;
        self.inspect_video(url, window, cx);
    }
    fn inspect_video(&mut self, url: String, window: &mut Window, cx: &mut Context<Self>) {
        self.library_edit_request = None;
        self.pending_channel = None;
        // Metadata fetch does not depend on a partially edited download range.
        let cookies = self.cookies.read(cx).value();
        let request = Request {
            format: Default::default(),
            url,
            output: std::env::temp_dir(),
            range: None,
            live_from_start: false,
            is_live: false,
            retries: 0,
            cookies: (!cookies.trim().is_empty()).then(|| cookies.trim().into()),
        };
        if let Err(e) = request.validate() {
            self.error = Some(e);
            cx.notify();
            return;
        }
        if !self.input_checkpoint && !(self.info.is_some() && self.info_url == request.url) {
            self.remember_navigation(cx);
        }
        self.input_checkpoint = false;
        self.showing_channel = false;
        self.close_preview(cx);
        if self.info_url != request.url {
            self.start.update(cx, |s, cx| s.set_value("", window, cx));
            self.end.update(cx, |s, cx| s.set_value("", window, cx));
            self.live_from_start = false;
        }
        self.info_url = request.url.clone();
        self.info = None;
        self.pending_thumbnail = None;
        self.pending_avatar = None;
        self.error = None;
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(metadata::inspect(&request, &Tools::default()));
        });
        cx.notify();
    }
    fn add(&mut self, start_now: bool, cx: &mut Context<Self>) {
        self.error = self
            .request(cx)
            .and_then(|r| {
                r.validate()?;
                for content in &self.contents {
                    let mut r = r.clone();
                    r.format.content = *content;
                    let job = if start_now {
                        self.manager.enqueue(r)?
                    } else {
                        self.manager.enqueue_pending(r)?
                    };
                    if let Some(info) = &self.info {
                        self.job_metadata.insert(
                            job.snapshot().id,
                            (info.thumbnail.clone(), info.uploader.clone()),
                        );
                        job.set_title(info.title.clone());
                    }
                }
                Ok(())
            })
            .err();
        if self.error.is_none() {
            self.close_preview(cx);
            self.navigate_to(Page::Queue, cx);
        }
        cx.notify();
    }
    fn preset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let input = self.preset_name.clone();
        let entity = cx.entity();
        self.preset_name
            .update(cx, |s, cx| s.set_value("", window, cx));
        window.open_dialog(cx, move |dialog, window, cx| {
            let validation = entity.read(cx).error.clone();
            let entity = entity.clone();
            let save =
                std::rc::Rc::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| {
                        let name = this.preset_name.read(cx).value().trim().to_string();
                        this.error = if name.is_empty() {
                            Some("請填寫預設名稱".into())
                        } else {
                            this.request(cx)
                                .and_then(|r| {
                                    this.preferences.presets.retain(|p| p.name != name);
                                    this.preferences.presets.push(Preset {
                                        name,
                                        contents: this.contents.clone(),
                                        format: FormatOptions {
                                            filename: String::new(),
                                            ..r.format
                                        },
                                    });
                                    this.save_preferences(window, cx)
                                })
                                .err()
                        };
                        cx.notify();
                        this.error.is_none()
                    })
                });
            input.read(cx).focus_handle(cx).focus(window, cx);
            dialog
                .title("儲存下載預設")
                .child(Input::new(&input).aria_label(tr("預設名稱")).h_11())
                .when_some(validation, |d, e| {
                    d.child(div().text_sm().text_color(cx.theme().danger).child(e))
                })
                .on_ok({
                    let save = save.clone();
                    move |e, w, cx| save(e, w, cx)
                })
                .footer(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("cancel-preset")
                                .ghost()
                                .label(tr("取消"))
                                .on_click(|_, w, cx| w.close_dialog(cx)),
                        )
                        .child(
                            Button::new("confirm-preset")
                                .primary()
                                .label(tr("儲存"))
                                .on_click(move |e, w, cx| {
                                    if save(e, w, cx) {
                                        w.close_dialog(cx)
                                    }
                                }),
                        ),
                )
        });
    }
    fn scroll_preview(&self, window: &mut Window, _cx: &mut App) {
        let anchor = self.preview_anchor.clone();
        window.on_next_frame(move |window, cx| {
            anchor.scroll_to(window, cx);
            window.on_next_frame(|window, _| window.refresh());
            window.refresh();
        });
    }
    fn close_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = self.preview.take() {
            preview.update(cx, |p, cx| p.hide(cx));
        }
        self.preview_subscription = None;
    }
    fn sync_preview_range(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(preview) = &self.preview {
            let start = parse_time(&self.start.read(cx).value()).unwrap_or(0);
            let end = parse_time(&self.end.read(cx).value()).unwrap_or_else(|_| {
                self.info
                    .as_ref()
                    .and_then(|i| i.duration)
                    .unwrap_or(0.)
                    .floor() as u64
            });
            preview.update(cx, |p, cx| p.set_range(start, end, window, cx));
        }
    }
    fn open_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.preview.is_some() {
            self.close_preview(cx);
            cx.notify();
            return;
        }
        let Some(info) = &self.info else {
            return;
        };
        match Preview::new(
            &self.info_url,
            info.duration.unwrap_or(0.),
            !info.seekable(),
            info.twitch_stream.as_ref(),
            window,
            cx,
        ) {
            Ok(preview) => {
                self.preview_subscription = Some(cx.subscribe_in(
                    &preview,
                    window,
                    |this, preview, event, window, cx| {
                        match event {
                            PreviewEvent::Start(time) => this.adjust_time(true, *time, window, cx),
                            PreviewEvent::End(time) => this.adjust_time(false, *time, window, cx),
                            PreviewEvent::Range(a, b) => {
                                this.start.update(cx, |s, cx| {
                                    s.set_value(flowmoor_rust::time_controls::clock(*a), window, cx)
                                });
                                this.end.update(cx, |s, cx| {
                                    s.set_value(flowmoor_rust::time_controls::clock(*b), window, cx)
                                });
                            }
                            PreviewEvent::Close => this.close_preview(cx),
                            PreviewEvent::Expand => {
                                preview.update(cx, |p, cx| {
                                    p.expanded = !p.expanded;
                                    p.focus(window, cx);
                                    cx.notify();
                                });
                                if !preview.read(cx).expanded {
                                    this.scroll_preview(window, cx);
                                }
                            }
                        }
                        this.sync_preview_range(window, cx);
                        cx.notify();
                    },
                ));
                preview.update(cx, |p, cx| p.focus(window, cx));
                self.preview = Some(preview);
                self.sync_preview_range(window, cx);
                self.scroll_preview(window, cx);
            }
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }
    fn save_preferences(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let retries = self
            .retries
            .read(cx)
            .value()
            .parse::<u8>()
            .map_err(|_| "重試次數必須介於 0–10")?;
        if retries > 10 {
            return Err("重試次數必須介於 0–10".into());
        }
        self.preferences.retries = retries;
        self.preferences.output = self.output.read(cx).value().trim().into();
        self.preferences.cookies = self.cookies.read(cx).value().trim().into();
        self.preferences.save(&Preferences::path())?;
        self.presets.update(cx, |s, cx| {
            s.set_items(
                self.preferences
                    .presets
                    .iter()
                    .map(|p| SharedString::from(p.name.clone()))
                    .collect(),
                window,
                cx,
            )
        });
        Ok(())
    }
    fn apply_preset(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self
            .preferences
            .presets
            .iter()
            .find(|p| p.name == name)
            .cloned()
        else {
            return;
        };
        self.contents = p.contents;
        self.video_codec.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match p.format.video_codec {
                    VideoCodec::Auto => 0,
                    VideoCodec::H264 => 1,
                    VideoCodec::Vp9 => 2,
                    VideoCodec::Av1 => 3,
                })),
                window,
                cx,
            )
        });
        self.audio_codec.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match p.format.audio_codec {
                    AudioCodec::Auto => 0,
                    AudioCodec::Mp3 => 1,
                    AudioCodec::Aac => 2,
                    AudioCodec::Opus => 3,
                })),
                window,
                cx,
            )
        });
        let mut heights: Vec<_> = self
            .info
            .iter()
            .flat_map(|i| i.formats.iter())
            .filter_map(|f| f.height)
            .filter(|h| *h > 0)
            .collect();
        if let Some(h) = p.format.max_height {
            heights.push(h);
        }
        heights.sort_unstable_by(|a, b| b.cmp(a));
        heights.dedup();
        let selected = p
            .format
            .max_height
            .and_then(|h| heights.iter().position(|v| *v == h))
            .map(|i| i + 1)
            .unwrap_or(0);
        let items = std::iter::once(SharedString::from("最佳可用畫質"))
            .chain(heights.iter().map(|h| SharedString::from(format!("{h}p"))))
            .collect();
        self.quality.update(cx, |s, cx| {
            s.set_items(items, window, cx);
            s.set_selected_index(Some(IndexPath::new(selected)), window, cx);
        });
        cx.notify();
    }
    fn folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("選擇輸出資料夾".into()),
        });
        cx.spawn_in(window, async move |this, cx| match result.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.first() {
                    let _ = this.update_in(cx, |this, window, cx| {
                        let path = path.to_string_lossy().into_owned();
                        if !this.preferences.recent_paths.contains(&path)
                            && !this.session_paths.contains(&path)
                        {
                            this.session_paths.insert(0, path.clone());
                        }
                        this.output
                            .update(cx, |s, cx| s.set_value(path, window, cx));
                        cx.notify();
                    });
                }
            }
            Ok(Err(e)) => {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some(e.to_string());
                    cx.notify();
                });
            }
            _ => {}
        })
        .detach();
    }
}
impl Render for DownloadApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let time_events = self.time_pointer_events(window, cx);
        let t = cx.theme();
        let bg = t.background;
        let surface = t.sidebar;
        let ink = t.foreground;
        let muted = t.muted_foreground;
        let line = t.border;
        let danger = t.danger;
        let sidebar = self.sidebar_view(window, cx);
        let mut body = div()
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(12.));
        if self.page == Page::Download {
            body = body.child(
                div()
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.url).aria_label(tr("影片連結")).h_11()),
                    )
                    .child(self.paste_control(cx)),
            );
            if self.showing_channel {
                body = body.child(self.channel_view(cx));
            } else if let Some(info) = &self.info {
                let summary = self.video_info_view(info, window, cx);
                body = body
                    .child(summary)
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(tr("內容類型"))
                                    .child(self.dropdown_control(
                                        menu_view::MenuKind::Content,
                                        if self.contents.is_empty() {
                                            "選擇內容".into()
                                        } else {
                                            self.contents
                                                .iter()
                                                .map(|c| match c {
                                                    Content::VideoAudio => "影片＋音訊",
                                                    Content::Video => "僅影片",
                                                    Content::Audio => "僅音訊",
                                                })
                                                .collect::<Vec<_>>()
                                                .join("、")
                                        },
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(tr("影片品質"))
                                    .child(
                                        self.dropdown_control(
                                            menu_view::MenuKind::Quality,
                                            self.quality
                                                .read(cx)
                                                .selected_value()
                                                .cloned()
                                                .unwrap_or_else(|| "最佳可用畫質".into()),
                                            cx,
                                        )
                                        .h_11()
                                        .w_full(),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(field("檔案名稱", &self.filename))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(tr("下載預設"))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div().flex_1().min_w_0().child(
                                                    self.dropdown_control(
                                                        menu_view::MenuKind::Presets,
                                                        self.presets
                                                            .read(cx)
                                                            .selected_value()
                                                            .cloned()
                                                            .unwrap_or_else(|| {
                                                                "選擇已儲存預設".into()
                                                            }),
                                                        cx,
                                                    ),
                                                ),
                                            )
                                            .child(
                                                Button::new("save-preset")
                                                    .ghost()
                                                    .h_11()
                                                    .icon(IconName::Plus)
                                                    .tooltip(tr("儲存為預設"))
                                                    .on_click(cx.listener(|this, _, w, cx| {
                                                        this.preset_dialog(w, cx)
                                                    })),
                                            )
                                            .child(
                                                Button::new("delete-preset")
                                                    .ghost()
                                                    .h_11()
                                                    .icon(IconName::Trash)
                                                    .tooltip(tr("刪除選取的預設"))
                                                    .disabled(
                                                        self.presets
                                                            .read(cx)
                                                            .selected_value()
                                                            .is_none(),
                                                    )
                                                    .on_click(cx.listener(|this, _, w, cx| {
                                                        if let Some(name) = this
                                                            .presets
                                                            .read(cx)
                                                            .selected_value()
                                                            .cloned()
                                                        {
                                                            this.preferences.presets.retain(|p| {
                                                                p.name != name.as_ref()
                                                            });
                                                            this.error =
                                                                this.save_preferences(w, cx).err();
                                                        }
                                                        cx.notify();
                                                    })),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(tr("影片編解碼器"))
                                    .child(
                                        self.dropdown_control(
                                            menu_view::MenuKind::VideoCodec,
                                            self.video_codec
                                                .read(cx)
                                                .selected_value()
                                                .cloned()
                                                .unwrap_or_else(|| "自動".into()),
                                            cx,
                                        )
                                        .h_11()
                                        .w_full(),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(tr("音訊編解碼器"))
                                    .child(
                                        self.dropdown_control(
                                            menu_view::MenuKind::AudioCodec,
                                            self.audio_codec
                                                .read(cx)
                                                .selected_value()
                                                .cloned()
                                                .unwrap_or_else(|| "自動".into()),
                                            cx,
                                        )
                                        .h_11()
                                        .w_full(),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .pt_2()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Icon::new(IconName::Clock).size_4())
                                    .child(tr("時間範圍")),
                            )
                            .child(
                                Button::new("reset-range")
                                    .ghost()
                                    .icon(IconName::RotateCcw)
                                    .label(tr("重置"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.start.update(cx, |s, cx| s.set_value("", window, cx));
                                        this.end.update(cx, |s, cx| s.set_value("", window, cx));
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(self.time_field(true, cx))
                            .child(self.time_field(false, cx)),
                    )
                    .child(self.output_control(cx))
                    .when(info.is_live && self.info_url.contains("youtu"), |d| {
                        d.child(
                            Button::new("live-mode")
                                .ghost()
                                .label(if self.live_from_start {
                                    "YouTube 直播：從開頭"
                                } else {
                                    "直播：從目前開始"
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.live_from_start = !this.live_from_start;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        div()
                            .mt_1()
                            .flex()
                            .items_center()
                            .w_full()
                            .rounded_full()
                            .overflow_hidden()
                            .bg(cx.theme().button_primary)
                            .child(
                                Button::new("download")
                                    .ghost()
                                    .rounded_none()
                                    .flex_1()
                                    .h(px(52.))
                                    .icon(IconName::ArrowDown)
                                    .label(tr("開始下載"))
                                    .disabled(self.contents.is_empty())
                                    .on_click(cx.listener(|this, _, w, cx| {
                                        this.add(true, cx);
                                        if this.error.is_none() {
                                            this.error = this.save_preferences(w, cx).err();
                                        }
                                    })),
                            )
                            .when(!info.is_live, |d| {
                                d.child(div().w(px(1.)).h(px(36.)).bg(line)).child(
                                    Button::new("enqueue")
                                        .ghost()
                                        .rounded_none()
                                        .w(relative(0.24))
                                        .min_w(px(96.))
                                        .h(px(52.))
                                        .icon(IconName::Plus)
                                        .label(tr("佇列"))
                                        .tooltip(tr("加入佇列"))
                                        .disabled(self.contents.is_empty())
                                        .on_click(cx.listener(|this, _, w, cx| {
                                            this.add(false, cx);
                                            if this.error.is_none() {
                                                this.error = this.save_preferences(w, cx).err();
                                            }
                                        })),
                                )
                            }),
                    );
            } else {
                body = body.child(
                    div()
                        .flex_1()
                        .min_h(px(280.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_4()
                        .child(
                            img(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/boat.png"))
                                .w_48()
                                .h_32(),
                        )
                        .child(div().text_color(muted).child(tr(
                            if self.pending.is_some() || self.pending_channel.is_some() {
                                "正在取得影片資訊…"
                            } else {
                                "貼上影片連結開始下載"
                            },
                        ))),
                );
            }
        } else if self.page == Page::Queue {
            body = body.child(self.queue_view(window, cx));
        } else if self.page == Page::Library {
            body = body.child(self.library_view(window, cx));
        } else {
            body = body.child(self.settings_view(window, cx));
        }
        if let Some(error) = &self.error {
            body = body.child(div().text_color(danger).text_sm().child(error.clone()));
        }
        div()
            .id("flowmoor-app")
            .on_action(
                cx.listener(|_, _: &chrome_view::ToggleFullscreen, w, _| w.toggle_fullscreen()),
            )
            .on_action(cx.listener(|_, _: &chrome_view::CloseWindow, w, _| w.remove_window()))
            .on_action(cx.listener(|_, _: &chrome_view::MinimizeWindow, w, _| w.minimize_window()))
            .size_full()
            .flex()
            .bg(bg)
            .text_color(ink)
            .font_family(cx.theme().font_family.clone())
            .text_size(px(14.))
            .flex_col()
            .child(self.chrome_view(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(time_events)
                    .when(self.page != Page::Settings, |d| d.child(sidebar))
                    .child(
                        div().flex_1().min_w_0().h_full().flex().flex_col().child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .relative()
                                .when(self.page == Page::Download, |d| d.m_4())
                                .child(
                                    div()
                                        .id("workspace")
                                        .track_scroll(&self.workspace_scroll)
                                        .size_full()
                                        .overflow_y_scroll()
                                        .when(self.page == Page::Download, |d| {
                                            d.p_4()
                                                .rounded(px(20.))
                                                .border_1()
                                                .border_color(line)
                                                .bg(surface)
                                        })
                                        .when(self.page == Page::Library, |d| d.px_4().py_6())
                                        .child(body),
                                )
                                .child(div().absolute().inset_0().child(
                                    gpui_kit::component::scroll::Scrollbar::vertical(
                                        &self.workspace_scroll,
                                    ),
                                )),
                        ),
                    ),
            )
            .when_some(self.open_menu.clone(), |d, menu| {
                d.child(deferred(menu).with_priority(gpui_kit::base::POPUP_PRIORITY))
            })
            .when_some(
                self.preview
                    .clone()
                    .filter(|p| p.read(cx).expanded)
                    .or_else(|| self.library_preview.clone().filter(|p| p.read(cx).expanded)),
                |d, p| {
                    d.child(
                        div()
                            .absolute()
                            .inset_0()
                            .p_4()
                            .bg(surface)
                            .occlude()
                            .child(p),
                    )
                },
            )
    }
}
fn main() {
    flowmoor_rust::handle_process_signal();
    gpui_kit::application()
        .with_assets(flowmoor_assets::FlowmoorAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            chrome_view::init(cx);
            theme::apply(true, cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::centered(size(px(1100.), px(800.)), cx)),
                    window_min_size: Some(size(px(760.), px(620.))),
                    titlebar: Some(TitlebarOptions {
                        traffic_light_position: Some(point(px(14.), px(16.))),
                        ..gpui_kit::component::TitleBar::title_bar_options()
                    }),
                    ..gpui_kit::component::TitleBar::window_options()
                },
                cx,
                |window, cx| {
                    window.set_window_title("Flowmoor Rust");
                    cx.new(|cx| DownloadApp::new(window, cx))
                },
            )
            .expect("無法建立視窗");
            cx.activate(true);
        });
}
