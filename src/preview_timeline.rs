use super::preview::*;
use crate::tr;
use flowmoor_rust::time_controls::{clock, range};
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
                    move |bounds, _, _| geometry.set(bounds),
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
                    .rounded(px(10.))
                    .bg(cx.theme().muted.opacity(0.7))
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
                        primary.opacity(0.55)
                    }),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(37.))
                    .w(relative(position))
                    .h(px(5.))
                    .bg(primary),
            );
        let step = [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 1800., 3600.,
        ]
        .into_iter()
        .find(|s| duration / s <= 8.)
        .unwrap_or(7200.);
        let first = (offset / step).ceil() as u64;
        for tick in first..=((offset + duration) / step).floor() as u64 {
            let x = fraction(tick as f64 * step);
            ruler = ruler
                .child(
                    div()
                        .absolute()
                        .left(relative(x))
                        .top(px(2.))
                        .when(x > 0.9, |d| d.ml(-px(40.)).w(px(40.)).text_right())
                        .text_size(px(10.))
                        .text_color(cx.theme().foreground)
                        .child(
                            clock((tick as f64 * step) as u64)
                                .strip_prefix("00:")
                                .unwrap_or(&clock((tick as f64 * step) as u64))
                                .to_string(),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left(relative(x))
                        .top(px(14.))
                        .w(px(1.))
                        .h(px(18.))
                        .bg(primary),
                );
        }
        for tick in 0..=40 {
            ruler = ruler.child(
                div()
                    .absolute()
                    .left(relative(tick as f32 / 40.))
                    .top(px(23.))
                    .w(px(1.))
                    .h(px(9.))
                    .bg(line),
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
                        .top(px(32.))
                        .w(px(6.))
                        .h(px(21.))
                        .rounded(px(2.))
                        .bg(primary.opacity(0.7))
                        .border_1()
                        .border_color(line),
                );
            }
        }
        ruler
            .child(
                div()
                    .absolute()
                    .left(relative(position))
                    .top(px(33.))
                    .w(px(2.))
                    .h(px(15.))
                    .bg(primary),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .text_size(px(10.))
                    .child(clock((offset + duration) as u64)),
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
                if dx.abs() > dy.abs() {
                    this.viewport.0 = (this.viewport.0 - dx / 800. * this.viewport.1)
                        .clamp(0., (this.state.duration - this.viewport.1).max(0.));
                } else {
                    let bounds = this.timeline_bounds.get();
                    let anchor = (f32::from(e.position.x - bounds.origin.x)
                        / f32::from(bounds.size.width).max(1.))
                    .clamp(0., 1.) as f64;
                    let span = (this.viewport.1 * (dy * 0.002).exp()).clamp(
                        5f64.min(this.state.duration),
                        this.state.duration.max(0.001),
                    );
                    this.viewport.0 = (this.viewport.0 + anchor * (this.viewport.1 - span))
                        .clamp(0., (this.state.duration - span).max(0.));
                    this.viewport.1 = span;
                }
                cx.stop_propagation();
                cx.notify();
            }))
    }
}
