use super::*;
use crate::menu_view::{FlowmoorMenu, MenuEvent, MenuItem, MenuKind};
impl DownloadApp {
    pub(super) fn dropdown_control(
        &self,
        kind: MenuKind,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = label.into();
        let disabled = self.dropdown_disabled(kind);
        let measure = self.menu_anchors.clone();
        let focus = self
            .menu_focus
            .borrow_mut()
            .entry(kind)
            .or_insert_with(|| cx.focus_handle())
            .clone();
        let name = match kind {
            MenuKind::Quality => "影片品質",
            MenuKind::Presets => "下載預設",
            MenuKind::VideoCodec => "視訊編碼",
            MenuKind::AudioCodec => "音訊編碼",
            MenuKind::Content => "內容類型",
            MenuKind::Output => {
                if self.page == Page::Settings {
                    "預設下載資料夾"
                } else {
                    "輸出資料夾"
                }
            }
            MenuKind::Language => "應用程式語言",
            MenuKind::LibrarySort => "排序",
            MenuKind::LibraryActions => "媒體操作",
        };
        div()
            .id(name)
            .track_focus(&focus)
            .tab_index(0)
            .tab_stop(!disabled)
            .rounded(px(12.))
            .focus_visible(|s| {
                s.shadow(vec![BoxShadow {
                    color: cx.theme().ring,
                    spread_radius: px(2.),
                    inset: false,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                }])
            })
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, w, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space" | "down" | "up") {
                    this.toggle_dropdown(kind, w, cx);
                    cx.stop_propagation();
                }
            }))
            .relative()
            .w_full()
            .min_w_0()
            .child(
                canvas(
                    move |b, _, _| {
                        measure.borrow_mut().insert(kind, b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                Button::new(name)
                    .outline()
                    .disabled(disabled)
                    .w_full()
                    .min_w_0()
                    .h(px(if kind == MenuKind::LibrarySort {
                        48.
                    } else {
                        44.
                    }))
                    .px(px(12.))
                    .py(px(8.))
                    .dropdown_caret(true)
                    .accessibility_label(tr(name))
                    .label(tr(label.as_ref()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.toggle_dropdown(kind, window, cx)
                    })),
            )
    }
    fn dropdown_disabled(&self, kind: MenuKind) -> bool {
        match kind {
            MenuKind::Quality | MenuKind::VideoCodec => {
                !self.contents.iter().any(|c| *c != Content::Audio)
            }
            MenuKind::AudioCodec => !self.contents.iter().any(|c| *c != Content::Video),
            _ => false,
        }
    }
    pub(super) fn toggle_dropdown(
        &mut self,
        kind: MenuKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dropdown_disabled(kind) {
            return;
        }
        let this = self;
        let anchors = this.menu_anchors.clone();

        if let Some(menu) = this.open_menu.as_ref()
            && menu.read(cx).kind == kind
        {
            menu.update(cx, |m, cx| m.close(cx));
            return;
        }
        let Some(bounds) = anchors.borrow().get(&kind).copied() else {
            return;
        };
        let selected = match kind {
            MenuKind::Quality => this.quality.read(cx).selected_value(),
            MenuKind::Presets => this.presets.read(cx).selected_value(),
            MenuKind::VideoCodec => this.video_codec.read(cx).selected_value(),
            MenuKind::AudioCodec => this.audio_codec.read(cx).selected_value(),
            _ => None,
        }
        .cloned();
        let labels: Vec<SharedString> = match kind {
            MenuKind::Quality => {
                let mut heights: Vec<_> = this
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
                heights.sort_unstable_by(|a, b| b.cmp(a));
                heights.dedup();
                std::iter::once("最佳可用畫質".into())
                    .chain(heights.into_iter().map(|h| format!("{h}p").into()))
                    .collect()
            }
            MenuKind::LibraryActions => [
                "播放影片",
                "重新執行下載",
                "編輯下載參數",
                "複製下載參數",
                "開啟檔案",
                "開啟資料夾",
                "複製檔案路徑",
                "複製診斷報告",
                "從記錄中移除",
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
            MenuKind::Language => ["繁體中文", "English"]
                .into_iter()
                .map(Into::into)
                .collect(),
            MenuKind::LibrarySort => ["依日期排序", "依標題排序"]
                .into_iter()
                .map(Into::into)
                .collect(),
            MenuKind::Presets => this
                .preferences
                .presets
                .iter()
                .map(|p| p.name.clone().into())
                .collect(),
            MenuKind::VideoCodec => ["自動", "H.264", "VP9", "AV1"]
                .into_iter()
                .map(Into::into)
                .collect(),
            MenuKind::AudioCodec => ["自動", "MP3", "AAC", "Opus"]
                .into_iter()
                .map(Into::into)
                .collect(),
            MenuKind::Content => ["影片＋音訊", "僅影片", "僅音訊"]
                .into_iter()
                .map(Into::into)
                .collect(),
            MenuKind::Output => this
                .session_paths
                .iter()
                .chain(
                    this.preferences
                        .recent_paths
                        .iter()
                        .filter(|p| !this.session_paths.contains(p)),
                )
                .cloned()
                .map(Into::into)
                .collect(),
        };
        let library_entry = this
            .library
            .entries
            .iter()
            .find(|e| Some(e.result.id) == this.library_menu_id);
        let library_completed = library_entry.is_some_and(|e| e.result.phase == Phase::Completed);
        let library_files = library_entry.is_some_and(|e| !e.result.files.is_empty());
        let library_failed = library_entry.is_some_and(|e| e.result.phase == Phase::Failed);
        let items = labels
            .into_iter()
            .enumerate()
            .filter(|(index, _)| {
                kind != MenuKind::LibraryActions
                    || match index {
                        0 | 4 => library_completed && library_files,
                        6 => library_files,
                        7 => library_failed,
                        _ => true,
                    }
            })
            .map(|(index, label)| MenuItem {
                key: index,
                icon: (kind == MenuKind::LibraryActions).then_some(match index {
                    0 => IconName::Play,
                    1 => IconName::RotateCcw,
                    2 | 4 => IconName::ExternalLink,
                    5 => IconName::FolderOpen,
                    8 => IconName::Trash,
                    _ => IconName::Clipboard,
                }),
                checked: match kind {
                    MenuKind::Content => this
                        .contents
                        .contains(&[Content::VideoAudio, Content::Video, Content::Audio][index]),
                    MenuKind::Output => label.as_ref() == this.output.read(cx).value().as_ref(),
                    MenuKind::Language => index == usize::from(this.preferences.language == "en"),
                    MenuKind::LibrarySort => index == usize::from(this.library_sort_title),
                    _ => selected.as_ref() == Some(&label),
                },
                temporary: kind == MenuKind::Output
                    && !this
                        .preferences
                        .recent_paths
                        .iter()
                        .any(|p| p == label.as_ref()),
                label,
            })
            .collect();
        let menu = cx.new(|cx| FlowmoorMenu::new(kind, items, bounds, window, cx));
        this.menu_subscription =
            Some(
                cx.subscribe_in(&menu, window, move |this, menu, event, window, cx| {
                    if matches!(event, MenuEvent::Dismiss) {
                        if this.open_menu.as_ref() == Some(menu) {
                            if menu.read(cx).restore_focus
                                && let Some(focus) = this.menu_focus.borrow().get(&kind).cloned()
                            {
                                focus.focus(window, cx);
                            }
                            this.open_menu = None;
                            this.menu_subscription = None;
                        }
                        cx.notify();
                        return;
                    }
                    let index = match event {
                        MenuEvent::Choose(i) | MenuEvent::Save(i) | MenuEvent::Remove(i) => *i,
                        MenuEvent::Dismiss => unreachable!(),
                    };
                    let value = menu.read(cx).items[index].label.clone();
                    match event {
                        MenuEvent::Choose(_) => match kind {
                            MenuKind::Content => {
                                let value =
                                    [Content::VideoAudio, Content::Video, Content::Audio][index];
                                if this.contents.contains(&value) {
                                    this.contents.retain(|v| *v != value);
                                } else {
                                    this.contents.push(value);
                                }
                                this.presets
                                    .update(cx, |s, cx| s.set_selected_index(None, window, cx));
                            }
                            MenuKind::LibraryActions => {
                                let key = menu.read(cx).items[index].key;
                                this.library_action(key, window, cx);
                            }
                            MenuKind::Language => {
                                this.preferences.language =
                                    if index == 0 { "zh-TW" } else { "en" }.into();
                                this.apply_language(window, cx);
                            }
                            MenuKind::LibrarySort => {
                                this.library_sort_title = index == 1;
                            }
                            MenuKind::Output => this
                                .output
                                .update(cx, |s, cx| s.set_value(value, window, cx)),
                            _ => {
                                let state = match kind {
                                    MenuKind::Quality => &this.quality,
                                    MenuKind::Presets => &this.presets,
                                    MenuKind::VideoCodec => &this.video_codec,
                                    MenuKind::AudioCodec => &this.audio_codec,
                                    _ => unreachable!(),
                                };
                                state.update(cx, |s, cx| {
                                    s.set_selected_value(&value, window, cx);
                                    cx.emit(SelectEvent::<Vec<SharedString>>::Confirm(Some(value)));
                                    cx.notify();
                                });
                            }
                        },
                        MenuEvent::Save(_) => {
                            this.session_paths.retain(|p| p != value.as_ref());
                            this.preferences.recent_paths.insert(0, value.to_string());
                            if this.page != Page::Settings {
                                this.error = this.preferences.save(&Preferences::path()).err();
                            }
                        }
                        MenuEvent::Remove(_) => {
                            this.session_paths.retain(|p| p != value.as_ref());
                            this.preferences
                                .recent_paths
                                .retain(|p| p != value.as_ref());
                            if this.output.read(cx).value().as_ref() == value.as_ref() {
                                let next = this
                                    .session_paths
                                    .first()
                                    .or(this.preferences.recent_paths.first())
                                    .cloned()
                                    .unwrap_or_default();
                                this.output
                                    .update(cx, |s, cx| s.set_value(next, window, cx));
                            }
                            this.preferences.output = this.output.read(cx).value().to_string();
                            if this.page != Page::Settings {
                                this.error = this.preferences.save(&Preferences::path()).err();
                            }
                        }
                        MenuEvent::Dismiss => unreachable!(),
                    }
                    cx.notify();
                }),
            );
        menu.update(cx, |m, cx| m.focus(window, cx));
        this.open_menu = Some(menu);
        cx.notify();
    }
}
