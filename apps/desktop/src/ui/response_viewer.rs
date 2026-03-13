use crate::engine::model::ActualResponse;
use gpui::prelude::*;
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseTab {
    Body,
    Headers,
}

pub struct ResponseViewer {
    pub response: Option<ActualResponse>,
    pub active_tab: ResponseTab,
    pub error: Option<String>,
    pub loading: bool,
    focus_handle: FocusHandle,
}

impl ResponseViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            response: None,
            active_tab: ResponseTab::Body,
            error: None,
            loading: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_response(&mut self, response: ActualResponse, cx: &mut Context<Self>) {
        self.response = Some(response);
        self.error = None;
        self.loading = false;
        cx.notify();
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        self.response = None;
        self.loading = false;
        cx.notify();
    }

    pub fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.error = None;
        cx.notify();
    }

    fn status_color(status: u16) -> Hsla {
        match status {
            200..=299 => hsla(0.33, 0.8, 0.5, 1.0), // green
            300..=399 => hsla(0.58, 0.8, 0.5, 1.0), // blue
            400..=499 => hsla(0.12, 0.8, 0.5, 1.0), // yellow
            500..=599 => hsla(0.0, 0.8, 0.5, 1.0),  // red
            _ => hsla(0.0, 0.0, 0.5, 1.0),          // gray
        }
    }
}

impl Render for ResponseViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.active_tab;

        let mut root = div()
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .border_t_1()
            .border_color(rgb(0x313244));

        if self.loading {
            return root.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .text_color(rgb(0x89b4fa))
                    .text_size(px(14.))
                    .child("Sending request..."),
            );
        }

        if let Some(error) = &self.error {
            return root.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .text_color(rgb(0xf38ba8))
                    .text_size(px(13.))
                    .p(px(16.))
                    .child(error.clone()),
            );
        }

        if let Some(resp) = &self.response {
            let status = resp.status;
            let time = resp.time_ms;
            let size = resp.size_bytes;
            let body = resp.body.clone();
            let headers = resp.headers.clone();

            root = root
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(12.))
                        .px(px(12.))
                        .py(px(8.))
                        .items_center()
                        .border_b_1()
                        .border_color(rgb(0x313244))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(2.))
                                .rounded(px(4.))
                                .bg(Self::status_color(status))
                                .text_color(rgb(0x1e1e2e))
                                .text_size(px(13.))
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{}", status)),
                        )
                        .child(
                            div()
                                .text_color(rgb(0x6c7086))
                                .text_size(px(12.))
                                .child(format!("{:.0}ms", time)),
                        )
                        .child(
                            div()
                                .text_color(rgb(0x6c7086))
                                .text_size(px(12.))
                                .child(format_size(size)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .border_b_1()
                        .border_color(rgb(0x313244))
                        .child(
                            div()
                                .px(px(12.))
                                .py(px(6.))
                                .cursor_pointer()
                                .text_size(px(12.))
                                .when(active_tab == ResponseTab::Body, |d| {
                                    d.text_color(rgb(0x89b4fa))
                                        .border_b_2()
                                        .border_color(rgb(0x89b4fa))
                                })
                                .when(active_tab != ResponseTab::Body, |d| {
                                    d.text_color(rgb(0x6c7086))
                                })
                                .child("Body")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.active_tab = ResponseTab::Body;
                                        cx.notify();
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .px(px(12.))
                                .py(px(6.))
                                .cursor_pointer()
                                .text_size(px(12.))
                                .when(active_tab == ResponseTab::Headers, |d| {
                                    d.text_color(rgb(0x89b4fa))
                                        .border_b_2()
                                        .border_color(rgb(0x89b4fa))
                                })
                                .when(active_tab != ResponseTab::Headers, |d| {
                                    d.text_color(rgb(0x6c7086))
                                })
                                .child("Headers")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.active_tab = ResponseTab::Headers;
                                        cx.notify();
                                    }),
                                ),
                        ),
                );

            if active_tab == ResponseTab::Body {
                root = root.child(
                    div()
                        .id("response-body-scroll")
                        .flex_1()
                        .p(px(12.))
                        .overflow_y_scroll()
                        .text_size(px(13.))
                        .text_color(rgb(0xcdd6f4))
                        .child(
                            div()
                                .font_family("monospace")
                                .child(pretty_print_body(&body)),
                        ),
                );
            } else {
                let mut header_list = div().flex().flex_col().gap(px(2.)).w_full();
                for h in &headers {
                    header_list = header_list.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.))
                            .px(px(4.))
                            .py(px(2.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .text_color(rgb(0x89b4fa))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("{}:", h.key)),
                            )
                            .child(div().text_color(rgb(0xcdd6f4)).child(h.value.clone())),
                    );
                }
                root = root.child(
                    div()
                        .id("response-headers-scroll")
                        .flex_1()
                        .p(px(12.))
                        .overflow_y_scroll()
                        .child(header_list),
                );
            }
        } else {
            root = root.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .text_color(rgb(0x6c7086))
                    .text_size(px(13.))
                    .child("Send a request to see the response"),
            );
        }

        root
    }
}

impl Focusable for ResponseViewer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn pretty_print_body(body: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string())
    } else {
        body.to_string()
    }
}
