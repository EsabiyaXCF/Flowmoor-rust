use super::*;
impl DownloadApp {
    pub(super) fn settings_view(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let tabs = [
            ("一般設定", IconName::SlidersHorizontal),
            ("下載設定", IconName::Download),
            ("語言設定", IconName::Languages),
            ("外觀設定", IconName::Palette),
            ("關於", IconName::Info),
        ];
        let mut nav = div()
            .id("settings-navigation")
            .relative()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.settings_nav_glow.hide();
                    cx.notify();
                }
            }))
            .child(
                self.settings_nav_glow
                    .view("settings-hover", cx.theme().accent),
            )
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .w(px(224.))
            .flex_shrink_0()
            .rounded(px(16.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar);
        for (index, (label, icon)) in tabs.iter().enumerate() {
            let selected = self.settings_tab == index;
            let measure = self.settings_nav_glow.clone();
            nav = nav.child(
                div()
                    .id(("settings-tab", index))
                    .role(Role::Button)
                    .aria_label(tr(label))
                    .tab_index(0)
                    .cursor_pointer()
                    .w_full()
                    .h(px(46.))
                    .px_4()
                    .py_3()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(if selected {
                        cx.theme().border
                    } else {
                        transparent_black()
                    })
                    .bg(if selected {
                        cx.theme().accent
                    } else {
                        transparent_black()
                    })
                    .relative()
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered {
                            this.settings_nav_glow.activate(index);
                            cx.notify();
                        }
                    }))
                    .child(
                        canvas(
                            move |bounds, w, _| {
                                if measure.measure(index, bounds) {
                                    w.on_next_frame(|w, _| w.refresh());
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .focus_visible(|s| s.border_color(cx.theme().ring))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(Icon::new(*icon).size_5())
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_sm()
                            .child(tr(label)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings_tab = index;
                        this.open_menu = None;
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            this.settings_tab = index;
                            this.open_menu = None;
                            cx.notify();
                            cx.stop_propagation();
                        }
                    })),
            );
        }
        let mut content = div().p_6().flex().flex_col().gap_6().min_h(px(420.));
        match self.settings_tab {
            0 => {
                content = content.child(div().max_w(px(576.)).child(self.output_control(cx)));
            }
            1 => {
                content = content
                    .child(
                        div()
                            .max_w(px(576.))
                            .child(field("錯誤重試次數", &self.retries)),
                    )
                    .child(
                        div()
                            .max_w(px(576.))
                            .child(field("Cookie 檔案（選填）", &self.cookies)),
                    );
            }
            3 => {
                content = content.child(
                    div()
                        .max_w(px(576.))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(tr("外觀模式"))
                        .child(
                            div().flex().gap_2().children(
                                [
                                    (false, "淺色", IconName::Sun),
                                    (true, "深色", IconName::Moon),
                                ]
                                .map(|(dark, label, icon)| {
                                    Button::new(label)
                                        .outline()
                                        .h_11()
                                        .px_4()
                                        .icon(icon)
                                        .label(tr(label))
                                        .selected(self.dark == dark)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.dark = dark;
                                            this.preferences.dark = dark;
                                            theme::apply(dark, cx);
                                            cx.notify();
                                        }))
                                }),
                            ),
                        ),
                );
            }
            2 => {
                content = content.child(
                    div()
                        .max_w(px(576.))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(tr("應用程式語言"))
                        .child(self.dropdown_control(
                            menu_view::MenuKind::Language,
                            if self.preferences.language == "en" {
                                "English"
                            } else {
                                "繁體中文"
                            },
                            cx,
                        )),
                );
            }
            _ => {
                content = content
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(
                                svg()
                                    .data(include_bytes!("../assets/flowmoor-mark.svg"))
                                    .size(px(64.))
                                    .text_color(cx.theme().foreground),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xl()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Flowmoor Rust"),
                                    )
                                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("GPUI Kit · Rust · yt-dlp · FFmpeg"),
                    )
                    .child(
                        div().child(
                            Button::new("third-party-notices")
                                .outline()
                                .h_11()
                                .px_4()
                                .icon(IconName::ExternalLink)
                                .label(tr("第三方授權與資源"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let open = || -> Result<(), String> {
                                        let path = Preferences::path()
                                            .with_file_name("third-party-notices.html");
                                        std::fs::create_dir_all(path.parent().unwrap())
                                            .map_err(|e| e.to_string())?;
                                        std::fs::write(
                                            &path,
                                            include_str!("../third-party/THIRD_PARTY_NOTICES.html"),
                                        )
                                        .map_err(|e| e.to_string())?;
                                        let url = url::Url::from_file_path(&path)
                                            .map_err(|_| "無效文件路徑".to_string())?;
                                        cx.open_url(url.as_str());
                                        Ok(())
                                    };
                                    this.error = open().err().map(|error| {
                                        format!("{}：{error}", tr("無法開啟第三方授權文件"))
                                    });
                                    cx.notify();
                                })),
                        ),
                    );
            }
        }
        let panel = div()
            .flex_1()
            .min_w_0()
            .rounded(px(16.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr(tabs[self.settings_tab].0)),
            )
            .child(content.flex_1());
        div()
            .h((window.viewport_size().height - px(44.)).max(px(480.)))
            .flex()
            .flex_col()
            .child(
                div()
                    .p_4()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap_4()
                    .child(nav)
                    .child(panel),
            )
            .child(
                div()
                    .p_6()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().sidebar)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_3()
                    .child(
                        Button::new("cancel-settings")
                            .ghost()
                            .h_11()
                            .px_4()
                            .label(tr("取消"))
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.navigate_history(false, w, cx);
                            })),
                    )
                    .child(
                        Button::new("save-settings")
                            .h_11()
                            .px_4()
                            .icon(IconName::Save)
                            .label(tr("保存設定"))
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.error = this.save_preferences(w, cx).err();
                                if this.error.is_none() {
                                    this.settings_original = None;
                                    this.navigate_history(false, w, cx);
                                }
                                cx.notify();
                            })),
                    ),
            )
    }
    pub(super) fn begin_settings(&mut self, cx: &Context<Self>) {
        self.settings_nav_glow.hide();
        self.settings_glow.hide();
        self.sidebar_glow.hide();
        let mut original = self.preferences.clone();
        original.output = self.output.read(cx).value().to_string();
        original.cookies = self.cookies.read(cx).value().to_string();
        self.settings_original = Some((original, self.session_paths.clone()));
    }
    pub(super) fn discard_settings(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some((original, paths)) = self.settings_original.take() {
            self.session_paths = paths;
            self.preferences = original;
            self.dark = self.preferences.dark;
            theme::apply(self.dark, cx);
            self.apply_language(w, cx);
            for (state, value) in [
                (&self.output, self.preferences.output.clone()),
                (&self.cookies, self.preferences.cookies.clone()),
                (&self.retries, self.preferences.retries.to_string()),
            ] {
                state.update(cx, |v, cx| v.set_value(value, w, cx));
            }
        }
    }
}

impl DownloadApp {
    pub(super) fn apply_language(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        flowmoor_rust::i18n::set_language(&self.preferences.language);
        for (state, label) in [
            (&self.url, "貼上 YouTube 或 Twitch 影片連結..."),
            (&self.filename, "自動產生"),
            (&self.end, "完整影片"),
            (&self.cookies, "選填 Netscape cookies.txt"),
            (&self.library_search, "搜尋標題或頻道"),
        ] {
            state.update(cx, |s, cx| s.set_placeholder(tr(label), w, cx));
        }
    }
}
