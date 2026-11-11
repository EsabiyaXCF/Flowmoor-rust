use super::*;
use flowmoor_rust::time_controls::{clock, precision, range, scrub};
pub(super) struct TimeDrag {
    pub start: bool,
    pub origin: Point<Pixels>,
    pub value: u64,
    pub moved: bool,
    pub step: u64,
    pub started: std::time::Instant,
}
impl DownloadApp {
    fn begin_time_edit(&mut self, start: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_original = [
            self.start.read(cx).value().to_string(),
            self.end.read(cx).value().to_string(),
        ];
        self.edit_time = Some(start);
        let input = if start { &self.start } else { &self.end };
        input.read(cx).focus_handle(cx).focus(window, cx);
        cx.notify();
    }
    fn duration_limit(&self) -> Option<u64> {
        self.info
            .as_ref()
            .filter(|i| i.seekable())
            .and_then(|i| i.duration)
            .map(|n| n.floor() as u64)
    }
    pub(super) fn adjust_time(
        &mut self,
        start_field: bool,
        next: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let a = parse_time(&self.start.read(cx).value()).unwrap_or(0);
        let b = parse_time(&self.end.read(cx).value()).ok();
        let (a, b) = range(start_field, next, a, b, self.duration_limit());
        self.start
            .update(cx, |s, cx| s.set_value(clock(a), window, cx));
        if let Some(b) = b {
            self.end
                .update(cx, |s, cx| s.set_value(clock(b), window, cx));
        }
        self.sync_preview_range(window, cx);
        cx.notify();
    }
    pub(super) fn time_pointer_events(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let on_move = cx.listener(|this, event: &MouseMoveEvent, window, cx| {
            let Some(drag) = &this.time_drag else {
                return;
            };
            let dx = f32::from(event.position.x - drag.origin.x);
            let up = f32::from(drag.origin.y - event.position.y);
            if !drag.moved && dx.hypot(up) < 6. {
                return;
            }
            let next = scrub(drag.value, dx, up, this.duration_limit());
            let field = drag.start;
            let step = precision(this.duration_limit(), up);
            if let Some(drag) = &mut this.time_drag {
                drag.moved = true;
                drag.step = step;
            }
            this.adjust_time(field, next, window, cx);
        });
        let on_up = cx.listener(|this, e: &MouseUpEvent, window, cx| {
            if let Some(drag) = this.time_drag.take() {
                let dx = f32::from(e.position.x - drag.origin.x);
                let up = f32::from(drag.origin.y - e.position.y);
                if drag.moved || dx.hypot(up) >= 6. {
                    let value = scrub(drag.value, dx, up, this.duration_limit());
                    this.adjust_time(drag.start, value, window, cx);
                } else if drag.started.elapsed() < std::time::Duration::from_millis(220) {
                    this.begin_time_edit(drag.start, window, cx);
                }
                cx.notify();
            }
        });
        canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                window.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase == DispatchPhase::Capture {
                        on_move(e, w, cx);
                    }
                });
                window.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase == DispatchPhase::Capture {
                        on_up(e, w, cx);
                    }
                });
            },
        )
        .absolute()
        .size_full()
        .into_any_element()
    }
    pub(super) fn time_field(&self, start: bool, cx: &mut Context<Self>) -> Div {
        let label = if start {
            "開始時間"
        } else {
            "結束時間"
        };
        let input = if start { &self.start } else { &self.end };
        let value = input.read(cx).value();
        let seconds = parse_time(&value).unwrap_or(0);
        let mut shell = div()
            .flex_1()
            .relative()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            );
        if self.edit_time == Some(start) {
            shell = shell.child(Input::new(input).aria_label(label).h_11());
        } else {
            shell = shell.child(
                div()
                    .id(if start { "start-scrub" } else { "end-scrub" })
                    .role(Role::Button)
                    .aria_label(label)
                    .on_a11y_action(AccessibleAction::Click, {
                        let entity = cx.entity();
                        move |_, w, cx| {
                            entity.update(cx, |this, cx| {
                                this.begin_time_edit(start, w, cx);
                            })
                        }
                    })
                    .font_family("Menlo")
                    .track_focus(&self.time_focus[usize::from(!start)])
                    .tab_index(0)
                    .h_11()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(10.))
                    .cursor(CursorStyle::ResizeLeftRight)
                    .hover(|s| s.bg(cx.theme().muted))
                    .child(clock(seconds))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            window.focus(&this.time_focus[usize::from(!start)], cx);
                            this.time_drag = Some(TimeDrag {
                                start,
                                origin: e.position,
                                value: seconds,
                                moved: false,
                                step: precision(this.duration_limit(), 0.),
                                started: std::time::Instant::now(),
                            });
                            cx.notify();
                        }),
                    ),
            );
        }
        if let Some(drag) = &self.time_drag
            && drag.start == start
        {
            shell = shell.child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .text_xs()
                    .text_color(cx.theme().primary)
                    .child(format!("{} 秒", drag.step)),
            );
        }
        shell.capture_key_down(cx.listener(move |this, e: &KeyDownEvent, w, cx| {
            let input = if start { &this.start } else { &this.end };
            let seconds = parse_time(&input.read(cx).value()).unwrap_or(0);
            let step = if e.keystroke.modifiers.shift { 10 } else { 1 };
            let next = match e.keystroke.key.as_str() {
                "up" => Some(seconds.saturating_add(step)),
                "down" => Some(seconds.saturating_sub(step)),
                "pageup" => Some(seconds.saturating_add(10)),
                "pagedown" => Some(seconds.saturating_sub(10)),
                "escape" if this.edit_time == Some(start) => {
                    let original = this.edit_original.clone();
                    this.start
                        .update(cx, |s, cx| s.set_value(original[0].clone(), w, cx));
                    this.end
                        .update(cx, |s, cx| s.set_value(original[1].clone(), w, cx));
                    this.edit_time = None;
                    this.sync_preview_range(w, cx);
                    cx.stop_propagation();
                    cx.notify();
                    None
                }
                "enter" | "space" if this.edit_time.is_none() => {
                    this.begin_time_edit(start, w, cx);
                    cx.stop_propagation();
                    cx.notify();
                    None
                }
                _ => None,
            };
            if let Some(next) = next {
                cx.stop_propagation();
                this.adjust_time(start, next, w, cx);
            }
        }))
    }
}
