use super::*;
use gpui_kit::component::WindowExt as _;
impl DownloadApp {
    pub(super) fn close_library_player(&mut self, cx: &mut Context<Self>) {
        if let Some(player) = self.library_preview.take() {
            player.update(cx, |p, cx| p.hide(cx));
        }
        self.library_preparing = None;
        self.library_subscription = None;
    }
    pub(super) fn prepare_library_player(&mut self, id: u128, cx: &mut Context<Self>) {
        let Some(entry) = self.library.entries.iter().find(|e| e.result.id == id) else {
            return;
        };
        let Some(path) = entry.result.files.last().cloned() else {
            self.error = Some(tr("此下載尚無可播放檔案"));
            return;
        };
        self.close_library_player(cx);
        let ffmpeg = Tools::default().ffmpeg;
        let (tx, rx) = mpsc::channel();
        self.library_preparing = Some(rx);
        self.error = None;
        std::thread::spawn(move || {
            let _ = tx.send(flowmoor_rust::library::prepare_preview(&path, &ffmpeg));
        });
        cx.notify();
    }
    pub(super) fn show_library_player(
        &mut self,
        path: PathBuf,
        duration: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page != Page::Library {
            return;
        }
        let source = url::Url::from_file_path(path).unwrap().to_string();
        match Preview::new(&source, duration, false, None, window, cx) {
            Ok(player) => {
                self.library_subscription =
                    Some(cx.subscribe_in(&player, window, |this, _, event, w, cx| {
                        if matches!(event, PreviewEvent::Expand) {
                            if let Some(player) = &this.library_preview {
                                player.update(cx, |p, cx| {
                                    p.expanded = !p.expanded;
                                    p.focus(w, cx);
                                    cx.notify();
                                });
                            }
                            cx.notify();
                        }
                        if matches!(event, PreviewEvent::Close) {
                            this.close_library_player(cx);
                            cx.notify();
                        }
                    }));
                self.library_preview = Some(player);
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn library_actions(
        &self,
        entry: &flowmoor_rust::library::Entry,
        cx: &mut Context<Self>,
    ) -> Div {
        let id = entry.result.id;
        let path = entry
            .result
            .files
            .last()
            .cloned()
            .unwrap_or_else(|| entry.result.directory.clone());
        let bounds = std::rc::Rc::new(std::cell::Cell::new(Bounds::default()));
        let measured = bounds.clone();
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                Button::new(("library-folder", id as usize))
                    .ghost()
                    .size_8()
                    .rounded(px(8.))
                    .icon(IconName::FolderOpen)
                    .tooltip(tr("開啟資料夾"))
                    .on_click(move |_, _, cx| cx.reveal_path(&path)),
            )
            .when(!entry.result.files.is_empty(), |d| {
                let path = entry.result.files.last().unwrap().clone();
                d.child(
                    Button::new(("library-file", id as usize))
                        .outline()
                        .size_11()
                        .rounded(px(8.))
                        .icon(IconName::ExternalLink)
                        .tooltip(tr("開啟檔案"))
                        .on_click(move |_, _, cx| {
                            if let Ok(url) = url::Url::from_file_path(&path) {
                                cx.open_url(url.as_str());
                            }
                        }),
                )
            })
            .child(
                div()
                    .relative()
                    .child(
                        canvas(move |b, _, _| measured.set(b), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    )
                    .child(
                        Button::new(("library-more", id as usize))
                            .ghost()
                            .size_8()
                            .rounded(px(8.))
                            .icon(IconName::EllipsisVertical)
                            .tooltip(tr("媒體操作"))
                            .on_click(cx.listener(move |this, _, w, cx| {
                                this.library_menu_id = Some(id);
                                this.menu_anchors
                                    .borrow_mut()
                                    .insert(menu_view::MenuKind::LibraryActions, bounds.get());
                                this.toggle_dropdown(menu_view::MenuKind::LibraryActions, w, cx);
                            })),
                    ),
            )
    }
    pub(super) fn load_download_request(
        &mut self,
        request: Request,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page == Page::Download && !self.input_checkpoint {
            self.remember_navigation(cx);
        }
        self.navigate_to(Page::Download, cx);
        self.input_checkpoint = true;
        self.url
            .update(cx, |v, cx| v.set_value(request.url.clone(), w, cx));
        let (a, b) = request
            .range
            .map_or((String::new(), String::new()), |(a, b)| {
                (
                    flowmoor_rust::time_controls::clock(a),
                    flowmoor_rust::time_controls::clock(b),
                )
            });
        self.filename.update(cx, |v, cx| {
            v.set_value(request.format.filename.clone(), w, cx)
        });
        self.contents = vec![request.format.content];
        self.output.update(cx, |v, cx| {
            v.set_value(request.output.to_string_lossy().into_owned(), w, cx)
        });
        self.retries
            .update(cx, |v, cx| v.set_value(request.retries.to_string(), w, cx));
        self.cookies.update(cx, |v, cx| {
            v.set_value(
                request
                    .cookies
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                w,
                cx,
            )
        });
        self.presets
            .update(cx, |v, cx| v.set_selected_index(None, w, cx));
        self.inspect_video(request.url.clone(), w, cx);
        self.start.update(cx, |v, cx| v.set_value(a, w, cx));
        self.end.update(cx, |v, cx| v.set_value(b, w, cx));
        self.library_edit_request = Some(request);
    }
    pub(super) fn library_action(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.library_menu_id else {
            return;
        };
        let Some(entry) = self
            .library
            .entries
            .iter()
            .find(|e| e.result.id == id)
            .cloned()
        else {
            return;
        };
        match index {
            0 => {
                self.remember_navigation(cx);
                self.library_selected = Some(id);
                self.prepare_library_player(id, cx);
            }
            1 => {
                self.error = self
                    .manager
                    .enqueue(entry.request.clone())
                    .map(|job| {
                        job.set_title(entry.result.title.clone());
                        self.job_metadata.insert(
                            job.snapshot().id,
                            (entry.thumbnail.clone(), entry.uploader.clone()),
                        );
                    })
                    .err();
                if self.error.is_none() {
                    self.navigate_to(Page::Queue, cx);
                }
            }
            2 => self.load_download_request(entry.request, w, cx),
            3 => match serde_json::to_string_pretty(&entry.request) {
                Ok(text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    w.push_notification(tr("已複製下載參數"), cx);
                }
                Err(error) => self.error = Some(error.to_string()),
            },
            4 => {
                if let Some(path) = entry.result.files.last()
                    && let Ok(url) = url::Url::from_file_path(path)
                {
                    cx.open_url(url.as_str());
                }
            }
            5 => cx.reveal_path(entry.result.files.last().unwrap_or(&entry.result.directory)),
            6 => {
                if let Some(path) = entry.result.files.last() {
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        path.to_string_lossy().into_owned(),
                    ));
                    w.push_notification(tr("已複製檔案路徑"), cx);
                }
            }
            7 => match serde_json::to_string_pretty(&entry.result) {
                Ok(text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    w.push_notification(tr("已複製診斷報告"), cx);
                }
                Err(error) => self.error = Some(error.to_string()),
            },
            _ => {
                self.library.entries.retain(|e| e.result.id != id);
                self.error = self
                    .library
                    .save(&Preferences::path().with_file_name("library.json"))
                    .err();
                if self.library_selected == Some(id) {
                    self.library_selected = None;
                    self.close_library_player(cx);
                }
            }
        }
        cx.notify();
    }
    pub(super) fn library_view(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut view = div()
            .w_full()
            .max_w(px(1152.))
            .mx_auto()
            .px_8()
            .pt_6()
            .pb_6()
            .flex()
            .flex_col();
        if let Some(error) = &self.library_load_error {
            return view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(error.clone()),
            );
        }
        if let Some(id) = self.library_selected
            && let Some(entry) = self.library.entries.iter().find(|e| e.result.id == id)
        {
            view = view
                .child(
                    div()
                        .mb_6()
                        .flex()
                        .items_start()
                        .gap_6()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(24.))
                                .line_height(px(32.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(entry.result.title.clone()),
                        )
                        .child(self.library_actions(entry, cx)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(entry.result.phase.label())),
                );
            if let Some(preview) = &self.library_preview {
                view = view.child(if preview.read(cx).expanded {
                    div().mt_6().h(preview.read(cx).inline_height.get())
                } else {
                    div().mt_6().child(preview.clone())
                });
            } else if self.library_preparing.is_some() {
                view = view.child(
                    div()
                        .py_10()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("正在準備預覽…")),
                );
            }
            return view;
        }
        view = view
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr("媒體庫")),
            )
            .child(
                div()
                    .mt_6()
                    .mb_6()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_3()
                    .child(
                        div()
                            .flex_grow_1()
                            .flex_shrink_1()
                            .flex_basis(px(320.))
                            .min_w_0()
                            .child(
                                Input::new(&self.library_search)
                                    .h_12()
                                    .prefix(Icon::new(IconName::Search).size_5())
                                    .aria_label(tr("搜尋標題或頻道")),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_grow_1()
                            .flex_shrink_1()
                            .flex_basis(px(224.))
                            .child(self.dropdown_control(
                                menu_view::MenuKind::LibrarySort,
                                tr(if self.library_sort_title {
                                    "依標題排序"
                                } else {
                                    "依日期排序"
                                }),
                                cx,
                            )),
                    ),
            );
        let search = self.library_search.read(cx).value().to_lowercase();
        let mut entries: Vec<_> = self
            .library
            .entries
            .iter()
            .filter(|e| {
                format!(
                    "{} {}",
                    e.result.title,
                    e.uploader.as_deref().unwrap_or_default()
                )
                .to_lowercase()
                .contains(&search)
            })
            .collect();
        if self.library_sort_title {
            entries.sort_by(|a, b| {
                flowmoor_rust::i18n::compare_titles(
                    &a.result.title,
                    &b.result.title,
                    &self.preferences.language,
                )
            });
        } else {
            entries.sort_by_key(|e| std::cmp::Reverse(e.completed_at.unwrap_or(e.result.id)));
        }
        if entries.is_empty() {
            let empty = div()
                .py_10()
                .text_center()
                .text_color(cx.theme().muted_foreground);
            return view.child(if self.library.entries.is_empty() {
                empty
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
                    .child(tr("尚無來源影片"))
            } else {
                empty.text_size(px(14.)).child(tr("沒有符合的項目"))
            });
        }
        let mut list = div()
            .id("library-list")
            .relative()
            .flex()
            .flex_col()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.library_glow.hide();
                    cx.notify();
                }
            }))
            .child(
                self.library_glow
                    .view("library-hover", cx.theme().accent.opacity(0.6)),
            );
        for entry in entries {
            let id = entry.result.id;
            let measure = self.library_glow.clone();
            let mut image = div()
                .w(px(96.))
                .h(px(56.))
                .flex_shrink_0()
                .rounded(px(10.))
                .overflow_hidden()
                .bg(cx.theme().muted)
                .flex()
                .items_center()
                .justify_center();
            image = if let Some(path) = &entry.thumbnail {
                image.child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
            } else {
                image.child(
                    Icon::new(IconName::Play)
                        .size_5()
                        .text_color(cx.theme().muted_foreground),
                )
            };
            let date = time::OffsetDateTime::from_unix_timestamp(
                (entry.completed_at.unwrap_or(id) / 1000) as i64,
            )
            .ok()
            .map(|d| {
                let d = d
                    .to_offset(time::UtcOffset::local_offset_at(d).unwrap_or(time::UtcOffset::UTC));
                let hour = if d.hour() % 12 == 0 {
                    12
                } else {
                    d.hour() % 12
                };
                if self.preferences.language == "en" {
                    format!(
                        "{}/{}/{}, {}:{:02}:{:02} {}",
                        d.month() as u8,
                        d.day(),
                        d.year(),
                        hour,
                        d.minute(),
                        d.second(),
                        if d.hour() < 12 { "AM" } else { "PM" }
                    )
                } else {
                    format!(
                        "{}/{}/{} {}{}:{:02}:{:02}",
                        d.year(),
                        d.month() as u8,
                        d.day(),
                        if d.hour() < 12 { "上午" } else { "下午" },
                        hour,
                        d.minute(),
                        d.second()
                    )
                }
            })
            .unwrap_or_default();
            list = list.child(
                div()
                    .id(("library-record", id as usize))
                    .rounded(px(16.))
                    .border_1()
                    .border_color(transparent_black())
                    .px_2()
                    .py_3()
                    .mb_1()
                    .flex()
                    .items_center()
                    .gap_3()
                    .relative()
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered {
                            this.library_glow.activate(id as usize);
                            cx.notify();
                        }
                    }))
                    .child(
                        canvas(
                            move |bounds, w, _| {
                                if measure.measure(id as usize, bounds) {
                                    w.on_next_frame(|w, _| w.refresh());
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .id(("library-select", id as usize))
                            .role(Role::Button)
                            .aria_label(entry.result.title.clone())
                            .tab_index(0)
                            .cursor_pointer()
                            .rounded(px(8.))
                            .focus_visible(|d| d.bg(cx.theme().accent))
                            .px_2()
                            .py_1()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_4()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remember_navigation(cx);
                                this.library_selected = Some(id);
                                this.prepare_library_player(id, cx);
                            }))
                            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    this.remember_navigation(cx);
                                    this.library_selected = Some(id);
                                    this.prepare_library_player(id, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child(image)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_base()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(entry.result.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_3()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(tr("下載"))
                                            .child(tr(entry.result.phase.label()))
                                            .child(date),
                                    ),
                            ),
                    )
                    .child(self.library_actions(entry, cx)),
            );
        }
        view.child(list)
    }
}
