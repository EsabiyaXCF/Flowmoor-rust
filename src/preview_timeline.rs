use super::preview::*;
use crate::tr;
use flowmoor_rust::time_controls::{compact_clock, range};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{component::ActiveTheme, *};
impl Preview {
    fn timeline_at(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if !self.state.ready || self.state.duration <= 0. {
            return;
        }
        let bounds = self.timeline_bounds.get();
        let fraction = (f32::from(x - bounds.origin.x) / f32::from(bounds.size.width).max(1.))
            .clamp(0., 1.) as f64;
        let seconds = (self.viewport.0 + fraction * self.viewport.1).min(self.state.duration);
        match self.timeline_drag {
            Some(1 | 2) => {
                let (a, b) = range(
                    self.timeline_drag == Some(1),
                    seconds.round() as u64,
                    self.selection.0,
                    Some(self.selection.1),
                    Some(self.state.duration.floor() as u64),
                );
                self.selection = (a, b.unwrap_or(a));
                cx.emit(PreviewEvent::Range(self.selection.0, self.selection.1));
            }
            _ => {
                if self
                    .last_seek
                    .is_none_or(|t| t.elapsed() > std::time::Duration::from_millis(45))
                {
                    self.seek_to(seconds, cx);
                }
            }
        }
        cx.notify();
    }
    pub(super) fn timeline(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let moving = cx.listener(|this, e: &MouseMoveEvent, _, cx| {
            if this.timeline_drag.is_some() {
                this.timeline_at(e.position.x, cx)
            }
        });
        let up = cx.listener(|this, e: &MouseUpEvent, _, cx| {
            if this.timeline_drag.is_some() {
                this.last_seek = None;
                this.timeline_at(e.position.x, cx);
            }
            this.timeline_drag = None;
            cx.notify();
        });
        if self.viewport.1 <= 0. || self.viewport.1 > self.state.duration {
            self.viewport = (0., self.state.duration);
        }
        let duration = self.viewport.1.max(1.);
        let offset = self.viewport.0;
        let fraction = |second: f64| ((second - offset) / duration).clamp(0., 1.) as f32;
        let start = fraction(self.selection.0 as f64);
        let end = fraction(self.selection.1 as f64);
        let position = fraction(self.state.time);
        let geometry = self.timeline_bounds.clone();
        let primary = cx.theme().primary;
        let line = cx.theme().border;
        let mut ruler = div()
            .id("preview-timeline")
            .role(Role::Slider)
            .aria_label(tr("預覽播放位置"))
            .aria_numeric_value(self.state.time)
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(self.state.duration)
            .aria_numeric_value_step(5.)
            .on_a11y_action(AccessibleAction::Increment, {
                let entity = cx.entity();
                move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.seek_to((this.state.time + 5.).min(this.state.duration), cx)
                    })
                }
            })
            .on_a11y_action(AccessibleAction::Decrement, {
                let entity = cx.entity();
                move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.seek_to((this.state.time - 5.).max(0.), cx)
                    })
                }
            })
            .track_focus(&self.timeline_focus)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(68.))
            .flex_shrink_0()
            .cursor_pointer()
            .child(
                canvas(
                    move |bounds, window, _| {
                        if geometry.get().size.width != bounds.size.width {
                            window.on_next_frame(|w, _| w.refresh());
                        }
                        geometry.set(bounds);
                    },
                    move |_, _, window, _| {
                        window.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if phase == DispatchPhase::Capture {
                                moving(e, w, cx)
                            }
                        });
                        window.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                            if phase == DispatchPhase::Capture {
                                up(e, w, cx)
                            }
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top_0()
                    .h_8()
                    .rounded(px(14.))
                    .bg(cx.theme().sidebar.opacity(0.9))
                    .border_1()
                    .border_color(line.opacity(0.3)),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(37.))
                    .h(px(5.))
                    .bg(line),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(start))
                    .top(px(37.))
                    .w(relative((end - start).max(0.)))
                    .h(px(5.))
                    .bg(if self.local {
                        transparent_black()
                    } else {
                        primary.opacity(0.24)
                    }),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(37.))
                    .w(relative(position))
                    .h(px(5.))
                    .bg(primary.opacity(0.32)),
            );
        if !self.local && self.state.time > self.selection.0 as f64 {
            let played = fraction(self.state.time.min(self.selection.1 as f64));
            ruler = ruler.child(
                div()
                    .absolute()
                    .left(relative(start))
                    .top(px(37.))
                    .h(px(5.))
                    .w(relative((played - start).max(0.)))
                    .bg(primary.opacity(0.9)),
            );
        }
        let width = f32::from(self.timeline_bounds.get().size.width).max(1.) as f64;
        let (minor, step) = timeline_scale(duration, width);
        let first = (offset / step).ceil() as u64;
        for tick in first..=((offset + duration) / step).floor() as u64 {
            let x = fraction(tick as f64 * step);
            let label = timeline_label(tick as f64 * step, step);
            let label_width = label.len() as f32 * 6. + 8.;
            ruler = ruler
                .child(
                    div()
                        .absolute()
                        .left(relative(x))
                        .top(px(2.))
                        .w(px(label_width))
                        .ml(-px(if x <= 0.06 {
                            0.
                        } else if x >= 0.94 {
                            label_width
                        } else {
                            label_width / 2.
                        }))
                        .font_family("Menlo")
                        .text_size(px(10.))
                        .text_color(cx.theme().foreground)
                        .child(label),
                )
                .child(
                    div()
                        .absolute()
                        .left(relative(x))
                        .top(px(14.))
                        .w(px(2.))
                        .h(px(18.))
                        .bg(primary),
                );
        }
        for tick in (offset / minor).ceil() as u64..=((offset + duration) / minor).floor() as u64 {
            let seconds = tick as f64 * minor;
            if (seconds / step - (seconds / step).round()).abs() < 0.000001 {
                continue;
            }
            ruler = ruler.child(
                div()
                    .absolute()
                    .left(relative(fraction(seconds)))
                    .top(px(23.))
                    .w(px(2.))
                    .h(px(9.))
                    .rounded_full()
                    .bg(cx.theme().muted_foreground.opacity(0.3)),
            );
        }
        for (index, (value, x)) in [(self.selection.0, start), (self.selection.1, end)]
            .into_iter()
            .enumerate()
        {
            if !self.local && (value as f64) >= offset && (value as f64) <= offset + duration {
                ruler = ruler.child(
                    div()
                        .id(("range-handle", index))
                        .tab_index(0)
                        .role(Role::Slider)
                        .aria_label(if index == 0 {
                            "下載起點"
                        } else {
                            "下載終點"
                        })
                        .aria_numeric_value(value as f64)
                        .aria_min_numeric_value(0.)
                        .aria_max_numeric_value(self.state.duration)
                        .aria_numeric_value_step(1.)
                        .on_key_down(cx.listener(move |_, e: &KeyDownEvent, _, cx| {
                            let step = if e.keystroke.modifiers.shift { 10 } else { 1 };
                            let next = match e.keystroke.key.as_str() {
                                "left" | "down" => value.saturating_sub(step),
                                "right" | "up" => value.saturating_add(step),
                                _ => return,
                            };
                            cx.emit(if index == 0 {
                                PreviewEvent::Start(next)
                            } else {
                                PreviewEvent::End(next)
                            });
                            cx.stop_propagation();
                        }))
                        .on_a11y_action(AccessibleAction::Increment, {
                            let entity = cx.entity();
                            move |_, _, cx| {
                                entity.update(cx, |_, cx| {
                                    cx.emit(if index == 0 {
                                        PreviewEvent::Start(value.saturating_add(1))
                                    } else {
                                        PreviewEvent::End(value.saturating_add(1))
                                    })
                                })
                            }
                        })
                        .on_a11y_action(AccessibleAction::Decrement, {
                            let entity = cx.entity();
                            move |_, _, cx| {
                                entity.update(cx, |_, cx| {
                                    cx.emit(if index == 0 {
                                        PreviewEvent::Start(value.saturating_sub(1))
                                    } else {
                                        PreviewEvent::End(value.saturating_sub(1))
                                    })
                                })
                            }
                        })
                        .absolute()
                        .left(relative(x))
                        .ml(-px(6.))
                        .top(px(31.5))
                        .w(px(12.))
                        .h(px(28.))
                        .cursor(CursorStyle::ResizeLeftRight)
                        .rounded_full()
                        .bg(cx.theme().sidebar.opacity(0.75))
                        .border_1()
                        .border_color(line.opacity(0.7))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().w(px(7.)).h(px(20.)).rounded(px(3.)).bg(
                            if self.timeline_drag == Some(index as u8 + 1) {
                                primary
                            } else {
                                cx.theme().muted_foreground.opacity(0.7)
                            },
                        )),
                );
            }
        }
        ruler
            .child(
                div()
                    .absolute()
                    .left(relative(position))
                    .ml(-px(4.5))
                    .top(px(35.))
                    .size(px(9.))
                    .rounded_full()
                    .bg(
                        if self.state.ready
                            && self.state.time >= offset
                            && self.state.time <= offset + duration
                        {
                            primary
                        } else {
                            transparent_black()
                        },
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(52.))
                    .font_family("Menlo")
                    .text_size(px(10.))
                    .when(self.state.ready, |d| {
                        d.child(compact_clock(self.state.time.round() as u64))
                    }),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(0.25))
                    .w(relative(0.5))
                    .text_center()
                    .top(px(52.))
                    .font_family("Menlo")
                    .text_size(px(10.))
                    .text_color(cx.theme().muted_foreground)
                    .when(duration < self.state.duration - 0.001, |d| {
                        d.child(format!(
                            "{} – {}",
                            timeline_label(offset, step),
                            timeline_label(offset + duration, step)
                        ))
                    }),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top(px(52.))
                    .font_family("Menlo")
                    .text_size(px(10.))
                    .child(compact_clock(self.state.duration as u64)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, w, cx| {
                    if !this.state.ready {
                        return;
                    }
                    cx.stop_propagation();
                    this.timeline_focus.focus(w, cx);
                    let bounds = this.timeline_bounds.get();
                    let x = f32::from(e.position.x - bounds.origin.x);
                    let width = f32::from(bounds.size.width);
                    this.timeline_drag = Some(
                        if !this.local
                            && this.selection.0 as f64 >= offset
                            && (this.selection.0 as f64) <= offset + duration
                            && (x - start * width).abs() < 10.
                        {
                            1
                        } else if !this.local
                            && this.selection.1 as f64 >= offset
                            && (this.selection.1 as f64) <= offset + duration
                            && (x - end * width).abs() < 10.
                        {
                            2
                        } else {
                            0
                        },
                    );
                    this.timeline_at(e.position.x, cx);
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                let step = if e.keystroke.modifiers.shift { 10. } else { 5. };
                match e.keystroke.key.as_str() {
                    "left" => this.seek_to((this.state.time - step).max(0.), cx),
                    "right" => this.seek_to((this.state.time + step).min(this.state.duration), cx),
                    "home" => this.seek_to(this.selection.0 as f64, cx),
                    "end" => this.seek_to(this.selection.1 as f64, cx),
                    _ => return,
                };
                cx.stop_propagation();
            }))
            .on_scroll(cx.listener(|this, e: &ScrollEvent, _, cx| {
                let delta = e.delta.pixel_delta(px(16.));
                let dx = f32::from(delta.x) as f64;
                let dy = f32::from(delta.y) as f64;
                if e.modifiers.alt {
                    let bounds = this.timeline_bounds.get();
                    let anchor = (f32::from(e.position.x - bounds.origin.x)
                        / f32::from(bounds.size.width).max(1.))
                    .clamp(0., 1.) as f64;
                    let span = (this.viewport.1 * (dy * 0.0025).exp()).clamp(
                        if this.state.duration <= 15. {
                            this.state.duration
                        } else if this.state.duration <= 120. {
                            3.
                        } else if this.state.duration <= 1800. {
                            5.
                        } else {
                            10.
                        },
                        this.state.duration.max(0.001),
                    );
                    this.viewport.0 = (this.viewport.0 + anchor * (this.viewport.1 - span))
                        .clamp(0., (this.state.duration - span).max(0.));
                    this.viewport.1 = span;
                } else if this.viewport.1 < this.state.duration - 0.001 {
                    let delta = if dx.abs() > dy.abs() { dx } else { dy };
                    this.viewport.0 = (this.viewport.0
                        - delta / f32::from(this.timeline_bounds.get().size.width).max(1.) as f64
                            * this.viewport.1)
                        .clamp(0., (this.state.duration - this.viewport.1).max(0.));
                } else {
                    return;
                }
                cx.stop_propagation();
                cx.notify();
            }))
    }
}

fn timeline_scale(duration: f64, width: f64) -> (f64, f64) {
    const INTERVALS: [f64; 16] = [
        0.1, 0.2, 0.5, 1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600.,
    ];
    let seconds_per_pixel = duration / width;
    let minor = INTERVALS
        .into_iter()
        .find(|step| step / seconds_per_pixel >= 6.)
        .unwrap_or(3600.);
    let candidates: Vec<_> = INTERVALS
        .into_iter()
        .filter(|step| *step >= minor && (step / minor - (step / minor).round()).abs() < 0.000001)
        .collect();
    let has_readable = candidates
        .iter()
        .any(|step| step / seconds_per_pixel >= 56.);
    let major = candidates
        .into_iter()
        .filter(|step| !has_readable || step / seconds_per_pixel >= 56.)
        .min_by(|a, b| {
            (a / seconds_per_pixel - 120.)
                .abs()
                .total_cmp(&(b / seconds_per_pixel - 120.).abs())
        })
        .unwrap();
    (minor, major)
}
fn timeline_label(seconds: f64, interval: f64) -> String {
    if interval >= 1. {
        compact_clock(seconds.round() as u64)
    } else {
        let tenths = (seconds * 10.).round() as u64;
        format!("{}.{:01}", compact_clock(tenths / 10), tenths % 10)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn scale_follows_width_and_fractional_zoom() {
        assert_eq!(super::timeline_scale(19., 720.), (0.2, 2.));
        assert_eq!(super::timeline_scale(19., 320.), (0.5, 5.));
        assert_eq!(super::timeline_scale(3., 720.), (0.1, 0.5));
        assert_eq!(super::timeline_label(5., 1.), "0:05");
        assert_eq!(super::timeline_label(1.999, 0.1), "0:02.0");
    }
}
