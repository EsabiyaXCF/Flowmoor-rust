use super::*;
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum MenuKind {
    Quality,
    Presets,
    VideoCodec,
    AudioCodec,
    Content,
    Output,
    Language,
    LibrarySort,
    LibraryActions,
}
#[derive(Clone)]
pub(super) struct MenuItem {
    pub label: SharedString,
    pub key: usize,
    pub icon: Option<IconName>,
    pub checked: bool,
    pub temporary: bool,
}
pub(super) enum MenuEvent {
    Choose(usize),
    Save(usize),
    Remove(usize),
    Dismiss,
}
pub(super) struct FlowmoorMenu {
    pub kind: MenuKind,
    pub items: Vec<MenuItem>,
    anchor: Bounds<Pixels>,
    above: bool,
    focus: FocusHandle,
    active: usize,
    closing: bool,
    pub restore_focus: bool,
    scroll: ScrollHandle,
    glow: crate::hover_glow::HoverGlow,
}
impl EventEmitter<MenuEvent> for FlowmoorMenu {}
impl FlowmoorMenu {
    pub fn new(
        kind: MenuKind,
        items: Vec<MenuItem>,
        anchor: Bounds<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let height = (items.len() as f32 * 50. + 14.).min(320.);
        let above = kind == MenuKind::Output
            || anchor.bottom() + px(height + 16.) > window.viewport_size().height;
        let active = items.iter().position(|i| i.checked).unwrap_or(0);
        Self {
            kind,
            items,
            anchor,
            above,
            focus: cx.focus_handle(),
            active,
            closing: false,
            restore_focus: true,
            scroll: ScrollHandle::new(),
            glow: Default::default(),
        }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if self.closing {
            return;
        }
        self.closing = true;
        let delay = if cx.reduce_motion() {
            0
        } else if matches!(self.kind, MenuKind::Content | MenuKind::Output) {
            40
        } else {
            120
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(delay))
                .await;
            let _ = this.update(cx, |_, cx| cx.emit(MenuEvent::Dismiss));
        })
        .detach();
        cx.notify();
    }
    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.closing {
            return;
        }
        self.active = index;
        if self.kind == MenuKind::Content {
            self.items[index].checked = !self.items[index].checked;
        }
        cx.emit(MenuEvent::Choose(index));
        if self.kind != MenuKind::Content {
            self.close(cx);
        }
        cx.notify();
    }
}
impl Render for FlowmoorMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dropdown = matches!(self.kind, MenuKind::Content | MenuKind::Output);
        let closing = self.closing;
        let above = self.above;
        let active = self.active;
        let mut list = div()
            .id("flowmoor-menu-items")
            .flex()
            .flex_col()
            .gap(px(6.))
            .max_h(px(320.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .relative()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.glow.hide();
                    cx.notify();
                }
            }))
            .child(self.glow.view("menu-highlight", cx.theme().accent));
        for (index, item) in self.items.iter().enumerate() {
            let checked = item.checked;
            let owner = cx.entity().downgrade();
            let measure = self.glow.clone();
            let mut row = div()
                .id(("option", index))
                .role(Role::MenuItem)
                .aria_label(tr(item.label.as_ref()))
                .aria_selected(checked)
                .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                    let _ = owner.update(cx, |this, cx| this.choose(index, cx));
                })
                .relative()
                .child(
                    canvas(
                        move |b, w, _| {
                            if measure.measure(index, b) {
                                w.on_next_frame(|w, _| w.refresh());
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .inset_0(),
                )
                .min_h(px(44.))
                .flex_shrink_0()
                .w_full()
                .min_w_0()
                .px(px(10.))
                .py(px(10.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .gap_3()
                .text_sm()
                .text_color(if checked || active == index {
                    cx.theme().foreground
                } else {
                    cx.theme().muted_foreground
                })
                .bg(if checked {
                    cx.theme().accent
                } else {
                    transparent_black()
                })
                .hover(|s| s.text_color(cx.theme().foreground))
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if *hovered {
                        this.active = index;
                        this.glow.activate(index);
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.choose(index, cx);
                }));
            if self.kind == MenuKind::Content {
                row = row.child(
                    div()
                        .size_5()
                        .flex_shrink_0()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(cx.theme().border)
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(checked, |d| d.child(Icon::new(IconName::Check).size_4())),
                );
            }
            if let Some(icon) = item.icon {
                row = row.child(Icon::new(icon).size_4());
            }
            row = row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(tr(item.label.as_ref())),
            );
            if self.kind == MenuKind::Output {
                if item.temporary {
                    row = row.child(div().text_xs().child(tr("暫存"))).child(
                        Button::new(("remember", index))
                            .ghost()
                            .size_10()
                            .p_0()
                            .icon(IconName::FolderPlus)
                            .tooltip(tr("儲存資料夾"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(MenuEvent::Save(index));
                                this.close(cx);
                            })),
                    );
                }
                row = row.child(
                    Button::new(("remove", index))
                        .ghost()
                        .size_10()
                        .p_0()
                        .icon(IconName::Trash)
                        .tooltip(tr("移除資料夾"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            cx.emit(MenuEvent::Remove(index));
                            this.close(cx);
                        })),
                );
            } else if checked && self.kind != MenuKind::Content {
                row = row.child(Icon::new(IconName::Check).size_4());
            }
            list = list.child(row);
        }
        if self.items.is_empty() {
            list = list.child(
                div()
                    .px_3()
                    .py_3()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("沒有可用項目")),
            );
        }
        let width = self.anchor.size.width;
        let surface = div()
            .id("flowmoor-menu")
            .role(Role::Menu)
            .track_focus(&self.focus)
            .occlude()
            .capture_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if this.closing {
                    return;
                }
                match e.keystroke.key.as_str() {
                    "escape" => this.close(cx),
                    "tab" => {
                        this.restore_focus = false;
                        this.close(cx);
                    }
                    "up" | "down" if !this.items.is_empty() => {
                        let n = this.items.len();
                        this.active = if e.keystroke.key == "up" {
                            (this.active + n - 1) % n
                        } else {
                            (this.active + 1) % n
                        };
                        this.scroll.scroll_to_item(this.active + 1);
                        this.glow.activate(this.active);
                        cx.notify();
                    }
                    "home" => {
                        this.active = 0;
                        this.scroll.scroll_to_item(this.active + 1);
                        this.glow.activate(this.active);
                        cx.notify();
                    }
                    "end" if !this.items.is_empty() => {
                        this.active = this.items.len() - 1;
                        this.scroll.scroll_to_item(this.active + 1);
                        this.glow.activate(this.active);
                        cx.notify();
                    }
                    "enter" | "space" if !this.items.is_empty() => this.choose(this.active, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.restore_focus = false;
                this.close(cx);
            }))
            .w(width)
            .min_w_0()
            .p(px(6.))
            .rounded(px(18.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(list)
            .when(self.items.len() > 6, |d| {
                d.child(
                    div()
                        .h(px(28.))
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(IconName::ArrowDown).size(px(14.))),
                )
            });
        let shadow = BoxShadow {
            inset: false,
            color: cx.theme().overlay.opacity(0.18),
            offset: point(px(0.), px(12.)),
            blur_radius: px(32.),
            spread_radius: px(-8.),
        };
        let phase = Rc::new(Cell::new(1.));
        let frame = phase.clone();
        let shift = Rc::new(Cell::new(0.));
        let animated_shift = shift.clone();
        let duration = if closing {
            if is_dropdown { 40 } else { 120 }
        } else if is_dropdown {
            80
        } else {
            240
        };
        let surface = surface.with_animation(
            if closing { "close" } else { "open" },
            Animation::new(Duration::from_millis(duration)).with_easing(move |t| {
                ease(
                    t,
                    if is_dropdown { 0.16 } else { 0.22 },
                    if is_dropdown { 0.3 } else { 0.36 },
                )
            }),
            move |surface, d| {
                frame.set(d);
                animated_shift.set(if closing {
                    0.
                } else {
                    (1. - d) * if is_dropdown { 3. } else { 4. } * if above { 1. } else { -1. }
                });
                let opacity = if closing {
                    1. - d
                } else if is_dropdown {
                    0.5 + 0.5 * d
                } else {
                    d
                };
                surface.opacity(opacity).shadow(vec![BoxShadow {
                    color: shadow.color.opacity(opacity.powi(3)),
                    ..shadow.clone()
                }])
            },
        );
        gpui_kit::base::Positioner::side(self.anchor)
            .placement(if above {
                gpui_kit::base::Placement::Top
            } else {
                gpui_kit::base::Placement::Bottom
            })
            .align(gpui_kit::base::Align::Start)
            .offset(px(if self.kind == MenuKind::Output {
                8.
            } else {
                5.
            }))
            .margin(px(16.))
            .child(MotionFrame::new(
                surface,
                shift,
                if closing || is_dropdown {
                    None
                } else {
                    Some((phase, above))
                },
            ))
    }
}
// Paint translation and clipping keep the menu's layout stable while it reveals.
pub(super) struct MotionFrame {
    child: AnyElement,
    shift: Rc<Cell<f32>>,
    clip: Option<(Rc<Cell<f32>>, bool)>,
}
impl MotionFrame {
    pub fn new(
        child: impl IntoElement,
        shift: Rc<Cell<f32>>,
        clip: Option<(Rc<Cell<f32>>, bool)>,
    ) -> Self {
        Self {
            child: child.into_any_element(),
            shift,
            clip,
        }
    }
    fn mask(&self, mut bounds: Bounds<Pixels>) -> Option<ContentMask<Pixels>> {
        let (phase, above) = self.clip.as_ref()?;
        if phase.get() >= 1. {
            return None;
        }
        let height = bounds.size.height * phase.get();
        if *above {
            bounds.origin.y += bounds.size.height - height;
        }
        bounds.size.height = height;
        Some(ContentMask { bounds })
    }
}
impl IntoElement for MotionFrame {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for MotionFrame {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(w, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        b: Bounds<Pixels>,
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        let y = self.shift.get();
        let mask = self.mask(b);
        w.with_content_mask(mask, |w| {
            w.with_element_offset(point(px(0.), px(y)), |w| {
                self.child.prepaint(w, cx);
            });
        });
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        b: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        let mask = self.mask(b);
        w.with_content_mask(mask, |w| self.child.paint(w, cx));
    }
}
pub(super) fn ease(t: f32, x1: f32, x2: f32) -> f32 {
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..16 {
        let x = (lo + hi) * 0.5;
        let a = 1. - x;
        let value = 3. * a * a * x * x1 + 3. * a * x * x * x2 + x * x * x;
        if value < t {
            lo = x;
        } else {
            hi = x;
        }
    }
    let x = (lo + hi) * 0.5;
    1. - (1. - x).powi(3)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn source_motion_curve_is_monotonic_and_reaches_endpoints() {
        for (x1, x2) in [(0.22, 0.36), (0.16, 0.3)] {
            assert!(ease(0., x1, x2) < 0.001);
            assert!(ease(1., x1, x2) > 0.999);
            let mut previous = 0.;
            for i in 0..=100 {
                let next = ease(i as f32 / 100., x1, x2);
                assert!(next >= previous);
                previous = next;
            }
        }
    }
}
