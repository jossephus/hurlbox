use gpui::prelude::*;
use gpui::*;

pub struct HurlPreview {
    pub content: String,
    focus_handle: FocusHandle,
}

impl HurlPreview {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            content: String::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
    }
}

impl Render for HurlPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(rgb(0x181825))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(12.))
                    .py(px(6.))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .child("Hurl Preview"),
            )
            .child(
                div()
                    .id("hurl-preview-scroll")
                    .flex_1()
                    .p(px(12.))
                    .overflow_y_scroll()
                    .font_family("monospace")
                    .text_size(px(12.))
                    .text_color(rgb(0xa6e3a1))
                    .child(self.content.clone()),
            )
    }
}

impl Focusable for HurlPreview {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
