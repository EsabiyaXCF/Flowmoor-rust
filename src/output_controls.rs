use super::*;
impl DownloadApp {
    pub(super) fn output_control(&self, cx: &mut Context<Self>) -> Div {
        let value = self.output.read(cx).value().to_string();
        let control = if self.session_paths.is_empty() && self.preferences.recent_paths.is_empty() {
            Input::new(&self.output)
                .aria_label(tr("輸出資料夾"))
                .h_11()
                .into_any_element()
        } else {
            self.dropdown_control(menu_view::MenuKind::Output, value, cx)
                .into_any_element()
        };
        div()
            .flex()
            .items_end()
            .gap(px(12.))
            .when(self.page == Page::Download, |d| d.pt_3())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(tr(if self.page == Page::Settings {
                        "預設下載資料夾"
                    } else {
                        "輸出資料夾"
                    }))
                    .child(control),
            )
            .child(
                Button::new("add-output-folder")
                    .outline()
                    .h(px(44.))
                    .px(px(12.))
                    .icon(IconName::Plus)
                    .label(tr("新增"))
                    .on_click(cx.listener(|this, _, w, cx| this.folder(w, cx))),
            )
    }
}
