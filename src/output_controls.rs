use super::*;
impl DownloadApp {
    pub(super) fn output_control(
        &self,
        inline: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let value = self.output.read(cx).value().to_string();
        let control = if self.session_paths.is_empty() && self.preferences.recent_paths.is_empty() {
            Input::new(&self.output)
                .bg(cx.theme().button)
                .aria_label(tr("輸出資料夾"))
                .h(self.form_control_height(window))
                .into_any_element()
        } else {
            self.dropdown_control(menu_view::MenuKind::Output, value, window, cx)
                .into_any_element()
        };
        div()
            .flex()
            .items_end()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(if self.page == Page::Download { 4. } else { 8. }))
                    .when(inline, |d| d.flex_row().items_center().gap_2())
                    .child(
                        div()
                            .flex_shrink_0()
                            .child(tr(if self.page == Page::Settings {
                                "預設下載資料夾"
                            } else {
                                "輸出資料夾"
                            })),
                    )
                    .child(div().min_w_0().when(inline, |d| d.flex_1()).child(control)),
            )
            .child(
                Button::new("add-output-folder")
                    .border_0()
                    .h(self.form_control_height(window))
                    .px(px(12.))
                    .icon(IconName::Plus)
                    .label(tr("新增"))
                    .on_click(cx.listener(|this, _, w, cx| this.folder(w, cx))),
            )
    }
}
