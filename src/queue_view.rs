use super::*;
#[derive(Clone)]
struct QueueDrag {
    id: u128,
    title: String,
}
impl Render for QueueDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .rounded(px(14.))
            .bg(cx.theme().popover)
            .text_sm()
            .child(self.title.clone())
    }
}
fn bytes(value: f64) -> String {
    let (amount, unit) = if value >= 1_073_741_824. {
        (value / 1_073_741_824., "GB")
    } else if value >= 1_048_576. {
        (value / 1_048_576., "MB")
    } else if value >= 1024. {
        (value / 1024., "KB")
    } else {
        (value, "B")
    };
    if unit == "B" {
        format!("{amount:.0} {unit}")
    } else {
        format!("{amount:.2} {unit}")
    }
}
impl DownloadApp {
    pub(super) fn queue_view(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        use gpui_kit::component::resizable::{resizable_panel, v_resizable};
        let height = (window.viewport_size().height - px(76.)).max(px(300.));
        if self.queue_collapsed.iter().any(|collapsed| *collapsed) {
            let mut view = div().px_4().py_2().h(height + px(16.)).flex().flex_col();
            for pending in [false, true] {
                view = view.child(
                    div()
                        .min_h_0()
                        .when(self.queue_collapsed[usize::from(pending)], |d| {
                            d.h_11().flex_shrink_0()
                        })
                        .when(!self.queue_collapsed[usize::from(pending)], |d| d.flex_1())
                        .child(self.queue_section(pending, cx)),
                );
            }
            return view;
        }
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
            .flex_shrink_0()
            .px_2()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new(if pending {
                    "toggle-pending"
                } else {
                    "toggle-active"
                })
                .ghost()
                .h_11()
                .px_0()
                .label(tr(if pending { "待處理" } else { "處理中" }))
                .icon(if self.queue_collapsed[usize::from(pending)] {
                    IconName::ChevronRight
                } else {
                    IconName::ArrowDown
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.queue_collapsed[usize::from(pending)] =
                        !this.queue_collapsed[usize::from(pending)];
                    cx.notify();
                })),
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
        let mut order = 0;
        for job in &self.manager.jobs {
            let s = job.snapshot();
            if matches!(s.phase, Phase::Pending | Phase::Queued | Phase::Paused) != pending {
                continue;
            }
            order += 1;
            let id = s.id;
            let expanded = self.queue_expanded.contains(&id);
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
                .child(s.total.map_or_else(
                    || bytes(s.downloaded as f64),
                    |total| format!("{} / {}", bytes(s.downloaded as f64), bytes(total as f64)),
                ));
            if let Some(speed) = s.speed {
                stats = stats.child(format!("{}/s", bytes(speed)));
            }
            if let Some(eta) = s.eta {
                stats = stats.child(format!(
                    "{} {}",
                    tr("剩餘"),
                    flowmoor_rust::time_controls::compact_clock(eta)
                ));
            }
            if s.total.is_some() {
                stats = stats.child(format!("{progress:.0}%"));
            }
            let edit_request = job.request.clone();
            let title = div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .text_left()
                .text_sm()
                .child(div().flex_1().min_w_0().truncate().child(s.title.clone()))
                .when(
                    pending && job.request.format.content != Content::VideoAudio,
                    |d| {
                        d.child(
                            div()
                                .flex_shrink_0()
                                .rounded(px(8.))
                                .px_1p5()
                                .py_px()
                                .text_size(px(10.))
                                .bg(cx.theme().accent)
                                .child(match job.request.format.content {
                                    Content::Audio => "audio-only",
                                    Content::Video => "video-only",
                                    Content::VideoAudio => "video+audio",
                                }),
                        )
                    },
                );
            let mut row = div()
                .id(("download-job", id as usize))
                .rounded(px(12.))
                .border_1()
                .border_color(transparent_black())
                .px_4()
                .py_3()
                .when(pending, |d| d.p_0())
                .when(!pending, |d| {
                    d.hover(|d| d.bg(cx.theme().accent.opacity(0.5)))
                })
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
                                        .child(
                                            Button::new(("job-details", id as usize))
                                                .ghost()
                                                .p_0()
                                                .h_auto()
                                                .min_w_0()
                                                .w_full()
                                                .accessibility_label(format!(
                                                    "{} · {}",
                                                    tr(if pending {
                                                        "編輯下載參數"
                                                    } else if expanded {
                                                        "收合詳情"
                                                    } else {
                                                        "展開詳情"
                                                    }),
                                                    s.title
                                                ))
                                                .child(title)
                                                .on_click(cx.listener(move |this, _, w, cx| {
                                                    if pending {
                                                        this.load_download_request(
                                                            edit_request.clone(),
                                                            w,
                                                            cx,
                                                        );
                                                    } else if !this.queue_expanded.remove(&id) {
                                                        this.queue_expanded.insert(id);
                                                    }
                                                    cx.notify();
                                                })),
                                        )
                                        .when(!pending, |d| {
                                            d.child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .flex_wrap()
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
                                            )
                                        }),
                                )
                                .child(actions),
                        )
                        .when(!pending, |d| {
                            d.child(
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
                        })
                        .when_some(s.error.clone(), |d, e| {
                            d.child(div().text_xs().text_color(cx.theme().danger).child(e))
                        }),
                );
            if pending {
                let metadata = self.job_metadata.get(&id);
                let mut avatar = div()
                    .size_10()
                    .flex_shrink_0()
                    .rounded_full()
                    .overflow_hidden()
                    .bg(cx.theme().accent)
                    .flex()
                    .items_center()
                    .justify_center();
                avatar = if let Some(path) = metadata.and_then(|m| m.avatar.as_ref()) {
                    avatar.child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
                } else {
                    avatar.child(Icon::new(IconName::User).size_5())
                };
                row = div()
                    .id(("pending-row", id as usize))
                    .rounded(px(12.))
                    .px_4()
                    .py(px(10.))
                    .hover(|d| d.bg(cx.theme().accent.opacity(0.5)))
                    .flex()
                    .items_center()
                    .gap_3()
                    .on_drop(cx.listener(move |this, drag: &QueueDrag, _, cx| {
                        let ids: Vec<_> = this
                            .manager
                            .jobs
                            .iter()
                            .map(|job| job.snapshot())
                            .filter(|s| {
                                matches!(s.phase, Phase::Pending | Phase::Queued | Phase::Paused)
                            })
                            .map(|s| s.id)
                            .collect();
                        if let (Some(from), Some(to)) = (
                            ids.iter().position(|value| *value == drag.id),
                            ids.iter().position(|value| *value == id),
                        ) && from != to
                        {
                            this.manager
                                .move_queued(drag.id, to as isize - from as isize);
                            cx.notify();
                        }
                    }))
                    .child(
                        div()
                            .id(("queue-order", id as usize))
                            .size_10()
                            .flex_shrink_0()
                            .rounded_full()
                            .border_1()
                            .border_color(cx.theme().border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .cursor(CursorStyle::OpenHand)
                            .role(Role::Button)
                            .aria_label(tr("拖曳調整順序"))
                            .tab_index(0)
                            .focus_visible(|d| d.border_color(cx.theme().ring))
                            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                                let direction = match e.keystroke.key.as_str() {
                                    "up" => -1,
                                    "down" => 1,
                                    _ => return,
                                };
                                this.manager.move_queued(id, direction);
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .child(order.to_string())
                            .on_drag(
                                QueueDrag {
                                    id,
                                    title: s.title.clone(),
                                },
                                |drag, _, _, cx| cx.new(|_| drag.clone()),
                            ),
                    )
                    .child(avatar)
                    .child(row.flex_1().min_w_0());
            }
            let details = div()
                .px_4()
                .pb_3()
                .flex()
                .flex_col()
                .gap_2()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(s.url.clone())
                .child(job.request.output.to_string_lossy().into_owned())
                .child(format!(
                    "{} · {} · {}",
                    tr(match job.request.format.content {
                        Content::VideoAudio => "影片＋音訊",
                        Content::Video => "僅影片",
                        Content::Audio => "僅音訊",
                    }),
                    job.request
                        .format
                        .max_height
                        .map_or_else(|| tr("最佳可用畫質"), |height| format!("{height}p")),
                    job.request.range.map_or_else(
                        || tr("完整影片"),
                        |(start, end)| format!(
                            "{} – {}",
                            flowmoor_rust::time_controls::clock(start),
                            flowmoor_rust::time_controls::clock(end)
                        )
                    )
                ))
                .children(
                    s.files
                        .iter()
                        .map(|path| div().child(path.to_string_lossy().into_owned())),
                );
            view = view.child(
                div()
                    .child(row)
                    .when(expanded && !pending, |d| d.child(details)),
            );
        }
        if count == 0 {
            view = div()
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .child(
                    svg()
                        .data(include_bytes!("../assets/queue.svg"))
                        .w(px(160.))
                        .h(px(107.))
                        .min_h_0()
                        .text_color(cx.theme().primary),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(tr(if pending {
                            "目前沒有等待中的任務"
                        } else {
                            "目前沒有處理中的任務"
                        })),
                );
        }
        div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .when(!self.queue_collapsed[usize::from(pending)], |d| {
                d.child(
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
            })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn queue_sizes_match_flowmoor_units() {
        assert_eq!(super::bytes(0.), "0 B");
        assert_eq!(super::bytes(1024.), "1.00 KB");
        assert_eq!(super::bytes(1_572_864.), "1.50 MB");
        assert_eq!(super::bytes(1_073_741_824.), "1.00 GB");
    }
}
