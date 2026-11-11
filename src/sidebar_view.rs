use super::*;
impl DownloadApp {
    fn toggle_download_group(&mut self, cx: &mut Context<Self>) {
        if self.collapsed {
            self.collapsed = false;
            self.download_group_open = true;
        } else {
            self.download_group_open = !self.download_group_open;
        }
        cx.notify();
    }

    pub(super) fn sidebar_width(&self, window: &Window) -> f32 {
        if self.collapsed {
            48.
        } else {
            (f32::from(window.viewport_size().width) * 0.18).clamp(180., 200.)
        }
    }

    fn nav_button(
        &self,
        id: &'static str,
        label: &'static str,
        icon: IconName,
        page: Page,
        nested: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let owner = cx.entity().downgrade();
        let row = div()
            .relative()
            .size_full()
            .flex()
            .items_center()
            .child(Icon::new(icon).size(px(18.)))
            .when(!self.collapsed, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(if nested { 25. } else { 29. }))
                        .right_0()
                        .text_size(px(if nested { 12. } else { 13. }))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(tr(label)),
                )
            });
        let glow = if page == Page::Settings {
            self.settings_glow.clone()
        } else {
            self.sidebar_glow.clone()
        };
        let measure = glow.clone();
        let leave = glow.clone();
        div()
            .id(id)
            .role(Role::Button)
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                let _ = owner.update(cx, |this, cx| this.navigate_to(page, cx));
            })
            .aria_label(tr(label))
            .tab_index(0)
            .cursor_pointer()
            .rounded(px(8.))
            .focus_visible(|s| {
                s.shadow(vec![BoxShadow {
                    color: cx.theme().ring,
                    spread_radius: px(2.),
                    inset: false,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                }])
            })
            .on_hover(cx.listener(move |_, hovered, _, cx| {
                if *hovered {
                    glow.activate(page as usize + 1);
                } else if page == Page::Settings {
                    leave.hide();
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    this.navigate_to(page, cx);
                    cx.stop_propagation();
                }
            }))
            .relative()
            .child(
                canvas(
                    move |b, w, _| {
                        if measure.measure(page as usize + 1, b) {
                            w.on_next_frame(|w, _| w.refresh());
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .w_full()
            .h(px(40.))
            .px(px(9.))
            .py_0()
            .border_1()
            .border_color(if self.page == page {
                cx.theme().primary.opacity(0.45)
            } else {
                transparent_black()
            })
            .bg(if self.page == page {
                cx.theme().accent
            } else {
                transparent_black()
            })
            .tooltip(move |w, cx| gpui_kit::component::tooltip::Tooltip::new(label).build(w, cx))
            .child(row)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.navigate_to(page, cx);
                cx.notify();
            }))
    }
    pub(super) fn sidebar_view(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let width = self.sidebar_width(window);
        let header = div()
            .relative()
            .h(px(56.))
            .flex_shrink_0()
            .pt_4()
            .pb_2()
            .when(!self.collapsed, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .pl_12()
                        .pr_3()
                        .h_full()
                        .child(
                            svg()
                                .data(include_bytes!("../assets/flowmoor-mark.svg"))
                                .size_5()
                                .text_color(cx.theme().foreground),
                        )
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Flowmoor"),
                        ),
                )
            })
            .child(
                Button::new("collapse")
                    .ghost()
                    .absolute()
                    .left(px(6.))
                    .top_4()
                    .w_9()
                    .h_9()
                    .icon(if self.collapsed {
                        IconName::PanelLeftOpen
                    } else {
                        IconName::PanelLeftClose
                    })
                    .tooltip(tr("收合／展開側邊欄"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.collapsed = !this.collapsed;
                        cx.notify();
                    })),
            );
        let measure = self.sidebar_glow.clone();
        let owner = cx.entity().downgrade();
        let group = div()
            .id("download-group")
            .role(Role::Button)
            .aria_label(tr("下載"))
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                let _ = owner.update(cx, |this, cx| this.toggle_download_group(cx));
            })
            .tab_index(0)
            .cursor_pointer()
            .relative()
            .rounded(px(8.))
            .focus_visible(|s| {
                s.shadow(vec![BoxShadow {
                    color: cx.theme().ring,
                    spread_radius: px(2.),
                    inset: false,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                }])
            })
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if *hovered {
                    this.sidebar_glow.activate(0);
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_download_group(cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        if measure.measure(0, b) {
                            w.on_next_frame(|w, _| w.refresh());
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .w_full()
            .h_10()
            .px(px(9.))
            .py_0()
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .items_center()
                    .child(Icon::new(IconName::Download).size(px(18.)))
                    .when(!self.collapsed, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(px(29.))
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(tr("下載")),
                        )
                        .child(
                            div().absolute().right_0().child(
                                Icon::new(if self.download_group_open {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .size(px(14.)),
                            ),
                        )
                    }),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_download_group(cx);
                cx.notify();
            }));
        let nav = div()
            .id("sidebar-navigation")
            .relative()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.sidebar_glow.hide();
                    cx.notify();
                }
            }))
            .child(
                self.sidebar_glow
                    .view("sidebar-highlight", cx.theme().accent),
            )
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(2.))
            .px(px(6.))
            .py_3()
            .child(group)
            .when(!self.collapsed && self.download_group_open, |d| {
                d.child(
                    div()
                        .ml(px(15.))
                        .pl(px(6.))
                        .pr(px(2.))
                        .border_l_1()
                        .border_color(cx.theme().border.opacity(0.7))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(self.nav_button(
                            "media",
                            "媒體下載",
                            IconName::Download,
                            Page::Download,
                            true,
                            cx,
                        ))
                        .child(self.nav_button(
                            "queue",
                            "下載進度與佇列",
                            IconName::ListVideo,
                            Page::Queue,
                            true,
                            cx,
                        )),
                )
            });
        let nav = nav.child(self.nav_button(
            "library",
            "媒體庫",
            IconName::Film,
            Page::Library,
            false,
            cx,
        ));
        div()
            .relative()
            .child(
                self.settings_glow
                    .view("settings-highlight", cx.theme().accent),
            )
            .w(px(width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(header)
            .child(nav)
            .child(div().px(px(6.)).pb_3().child(self.nav_button(
                "settings",
                "設定",
                IconName::Settings,
                Page::Settings,
                false,
                cx,
            )))
    }
}
