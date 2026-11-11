use super::*;
use std::{cell::Cell, rc::Rc};
impl DownloadApp {
    pub(super) fn paste_control(&self, cx: &mut Context<Self>) -> Button {
        let busy = self.pending.is_some() || self.pending_channel.is_some();
        let mut button = Button::new("inspect")
            .primary()
            .h(px(44.))
            .min_w(px(130.))
            .px(px(20.))
            .py_0()
            .font_weight(FontWeight::SEMIBOLD)
            .accessibility_label(tr("貼上並取得"))
            .loading(busy)
            .on_hover(cx.listener(|this, hovered, _, cx| {
                if this.paste_hover != *hovered {
                    this.paste_from = this.paste_progress.get();
                    this.paste_hover = *hovered;
                    this.paste_revision += 1;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, window, cx| this.inspect(window, cx)));
        if busy {
            return button.label(tr("載入中"));
        }
        let mut viewport = div().relative().w_full().h(px(20.)).overflow_hidden();
        for icon in [false, true] {
            let shift = Rc::new(Cell::new(0.));
            let animated_shift = shift.clone();
            let progress = self.paste_progress.clone();
            let from = self.paste_from;
            let to = if self.paste_hover { 1. } else { 0. };
            let row = div()
                .w_full()
                .h(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .when(icon, |d| d.absolute().inset_0())
                .child(if icon {
                    Icon::new(IconName::Download).size_5().into_any_element()
                } else {
                    div().child(tr("貼上並取得")).into_any_element()
                });
            let row = row.with_animation(
                (
                    if icon { "paste-icon" } else { "paste-label" },
                    self.paste_revision,
                ),
                Animation::new(Duration::from_millis(180))
                    .with_easing(|t| menu_view::ease(t, 0.22, 0.36)),
                move |row, t| {
                    let p = from + (to - from) * t;
                    progress.set(p);
                    animated_shift.set(if icon { 33. * (1. - p) } else { -33. * p });
                    row
                },
            );
            viewport = viewport.child(menu_view::MotionFrame::new(row, shift, None));
        }
        button = button.child(viewport);
        button
    }
}
