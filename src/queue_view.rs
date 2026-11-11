use super::*;
impl DownloadApp {
    pub(super) fn queue_view(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        use gpui_kit::component::resizable::{resizable_panel, v_resizable};
        let height = (window.viewport_size().height - px(76.)).max(px(300.));
        div().px_4().py_2().h(height + px(16.)).child(
            v_resizable("download-queue-sections")
                .child(
                    resizable_panel()
                        .size(height * 0.45)
                        .child(self.queue_section(false, cx)),
                )
                .child(resizable_panel().child(self.queue_section(true, cx))),
        )
    }
    fn queue_section(&self, pending: bool, cx: &mut Context<Self>) -> Div {
        let mut view = div().flex().flex_col().gap_2();
        let count = self
            .manager
            .jobs
            .iter()
            .filter(|j| {
                matches!(
                    j.snapshot().phase,
                    Phase::Pending | Phase::Queued | Phase::Paused
                ) == pending
            })
            .count();
        let mut header = div()
            .h_11()
            .px_2()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(if pending { "待處理" } else { "處理中" })),
            );
        if pending {
            header = header.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("start-pending")
                            .ghost()
                            .h_11()
                            .px_3()
                            .icon(IconName::Play)
                            .label(tr("全部開始"))
                            .disabled(count == 0)
                            .on_click(cx.listener(|this, _, _, cx| {
                                for job in &this.manager.jobs {
                                    if matches!(
                                        job.snapshot().phase,
                                        Phase::Pending | Phase::Queued | Phase::Paused
                                    ) {
                                        this.manager.resume(job);
                                    }
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("remove-pending")
                            .ghost()
                            .h_11()
                            .px_3()
                            .icon(IconName::Trash)
                            .label(tr("全部移除"))
                            .disabled(count == 0)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.manager.jobs.retain(|job| {
                                    if matches!(
                                        job.snapshot().phase,
                                        Phase::Pending | Phase::Queued | Phase::Paused
                                    ) {
                                        job.cancel();
                                        false
                                    } else {
                                        true
                                    }
                                });
                                cx.notify();
                            })),
                    ),
            );
        }
        if !pending
            && self
                .manager
                .jobs
                .iter()
                .any(|j| j.snapshot().phase.terminal())
        {
            header = header.child(
                Button::new("clear-finished")
                    .ghost()
                    .h_11()
                    .px_3()
                    .label(tr("清除已結束"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.manager.jobs.retain(|j| !j.snapshot().phase.terminal());
                        cx.notify();
                    })),
            );
        }
        for job in &self.manager.jobs {
            let s = job.snapshot();
            if matches!(s.phase, Phase::Pending | Phase::Queued | Phase::Paused) != pending {
                continue;
            }
            let id = s.id;
            let tone = match s.phase {
                Phase::Completed => rgb(if self.dark {
                    theme::SUCCESS_DARK
                } else {
                    theme::SUCCESS_LIGHT
                })
                .into(),
                Phase::Failed => cx.theme().danger,
                Phase::Cancelled | Phase::Stopped => rgb(if self.dark {
                    theme::WARNING_DARK
                } else {
                    theme::WARNING_LIGHT
                })
                .into(),
                _ => cx.theme().primary,
            };
            let progress = if s.phase == Phase::Completed {
                100.
            } else {
                s.total
                    .filter(|n| *n > 0)
                    .map_or(0., |n| s.downloaded as f32 / n as f32 * 100.)
                    .clamp(0., 100.)
            };
            let mut actions = div().flex().items_center().gap_1().flex_shrink_0();
            if !s.phase.terminal() {
                if matches!(s.phase, Phase::Pending | Phase::Paused) {
                    let job = job.clone();
                    actions = actions.child(
                        Button::new(("resume", id as usize))
                            .ghost()
                            .size_11()
                            .rounded_full()
                            .icon(IconName::Play)
                            .tooltip(tr("開始"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.manager.resume(&job);
                                cx.notify();
                            })),
                    );
                } else if !job.request.is_live {
                    let job = job.clone();
                    actions = actions.child(
                        Button::new(("pause", id as usize))
                            .ghost()
                            .size_11()
                            .rounded_full()
                            .icon(IconName::Pause)
                            .tooltip(tr("暫停"))
                            .on_click(move |_, _, _| job.pause()),
                    );
                }
                if matches!(s.phase, Phase::Pending | Phase::Queued | Phase::Paused) {
                    for (direction, icon, label) in [
                        (-1, IconName::ChevronUp, "上移"),
                        (1, IconName::ChevronDown, "下移"),
                    ] {
                        actions = actions.child(
                            Button::new(SharedString::from(format!("order-{id}-{direction}")))
                                .ghost()
                                .size_8()
                                .rounded_full()
                                .icon(icon)
                                .tooltip(tr(label))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.manager.move_queued(id, direction);
                                    cx.notify();
                                })),
                        );
                    }
                }
                if !matches!(s.phase, Phase::Pending | Phase::Paused | Phase::Queued) {
                    let job = job.clone();
                    actions = actions.child(
                        Button::new(("stop", id as usize))
                            .ghost()
                            .size_11()
                            .rounded_full()
                            .icon(IconName::Square)
                            .tooltip(tr("停止並保留輸出"))
                            .on_click(move |_, _, _| job.stop()),
                    );
                }
                let job = job.clone();
                actions = actions.child(
                    Button::new(("cancel", id as usize))
                        .ghost()
                        .size_11()
                        .rounded_full()
                        .icon(IconName::X)
                        .tooltip(tr("取消"))
                        .on_click(move |_, _, _| job.cancel()),
                );
            } else {
                if !s.files.is_empty() {
                    let path = s.files[0].clone();
                    actions = actions.child(
                        Button::new(("open", id as usize))
                            .ghost()
                            .size_11()
                            .rounded_full()
                            .icon(IconName::FolderOpen)
                            .tooltip(tr("開啟資料夾"))
                            .on_click(move |_, _, cx| cx.reveal_path(&path)),
                    );
                }
                if s.phase != Phase::Completed {
                    let request = job.request.clone();
                    actions = actions.child(
                        Button::new(("retry", id as usize))
                            .ghost()
                            .size_11()
                            .rounded_full()
                            .icon(IconName::RotateCcw)
                            .tooltip(tr("重新下載"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.error = this.manager.enqueue(request.clone()).err();
                                cx.notify();
                            })),
                    );
                }
                actions = actions.child(
                    Button::new(("dismiss", id as usize))
                        .ghost()
                        .size_11()
                        .rounded_full()
                        .icon(IconName::X)
                        .tooltip(tr("關閉"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.manager.jobs.retain(|j| j.snapshot().id != id);
                            cx.notify();
                        })),
                );
            }
            let mut stats = div()
                .flex()
                .items_center()
                .gap_3()
                .text_size(px(11.))
                .text_color(cx.theme().muted_foreground)
                .child(format!("{:.1} MB", s.downloaded as f64 / 1e6));
            if let Some(speed) = s.speed {
                stats = stats.child(format!("{:.1} MB/s", speed / 1e6));
            }
            if let Some(eta) = s.eta {
                stats = stats.child(format!("{} {eta}s", tr("剩餘")));
            }
            if s.total.is_some() {
                stats = stats.child(format!("{progress:.0}%"));
            }
            let row = div()
                .id(("download-job", id as usize))
                .rounded(px(12.))
                .border_1()
                .border_color(transparent_black())
                .px_4()
                .py_3()
                .hover(|d| d.bg(cx.theme().accent.opacity(0.5)))
                .flex()
                .gap_3()
                .items_start()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().truncate().text_sm().child(s.title.clone()))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .h_5()
                                                        .px_2()
                                                        .rounded_full()
                                                        .border_1()
                                                        .border_color(cx.theme().border)
                                                        .text_size(px(11.))
                                                        .text_color(tone)
                                                        .child(tr(s.phase.label())),
                                                )
                                                .child(stats),
                                        ),
                                )
                                .child(actions),
                        )
                        .child(
                            div()
                                .h(px(6.))
                                .w_full()
                                .rounded_full()
                                .overflow_hidden()
                                .bg(cx.theme().border.opacity(0.6))
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(progress / 100.))
                                        .rounded_full()
                                        .bg(tone),
                                ),
                        )
                        .when_some(s.error.clone(), |d, e| {
                            d.child(div().text_xs().text_color(cx.theme().danger).child(e))
                        }),
                );
            view = view.child(row);
        }
        if count == 0 {
            view =
                view.child(
                    div()
                        .flex_1()
                        .min_h(px(160.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .child(
                            img(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/boat.png"))
                                .h(px(64.))
                                .w(px(112.)),
                        )
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(tr(
                            if pending {
                                "目前沒有等待中的任務"
                            } else {
                                "目前沒有處理中的任務"
                            },
                        ))),
                );
        }
        div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id(if pending {
                        "pending-jobs"
                    } else {
                        "active-jobs"
                    })
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(view),
            )
    }
}
