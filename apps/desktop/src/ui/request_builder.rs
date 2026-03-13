use crate::engine::model::*;
use crate::ui::text_input::TextInput;
use gpui::prelude::*;
use gpui::*;

pub struct KeyValueRow {
    pub key_input: Entity<TextInput>,
    pub value_input: Entity<TextInput>,
    pub enabled: bool,
}

pub struct KeyValueEditor {
    pub rows: Vec<KeyValueRow>,
    focus_handle: FocusHandle,
}

impl KeyValueEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            rows: Vec::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn from_key_values(kvs: &[KeyValue], cx: &mut Context<Self>) -> Self {
        let rows = kvs
            .iter()
            .map(|kv| {
                let key_input = cx.new(|cx| {
                    let mut input = TextInput::new(cx, "Key");
                    input.set_text(&kv.key, cx);
                    input
                });
                let value_input = cx.new(|cx| {
                    let mut input = TextInput::new(cx, "Value");
                    input.set_text(&kv.value, cx);
                    input
                });
                KeyValueRow {
                    key_input,
                    value_input,
                    enabled: kv.enabled,
                }
            })
            .collect();
        Self {
            rows,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn add_row(&mut self, cx: &mut Context<Self>) {
        let key_input = cx.new(|cx| TextInput::new(cx, "Key"));
        let value_input = cx.new(|cx| TextInput::new(cx, "Value"));
        self.rows.push(KeyValueRow {
            key_input,
            value_input,
            enabled: true,
        });
        cx.notify();
    }

    pub fn remove_row(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.rows.len() {
            self.rows.remove(index);
            cx.notify();
        }
    }

    pub fn to_key_values(&self, cx: &App) -> Vec<KeyValue> {
        self.rows
            .iter()
            .map(|row| {
                let key = row.key_input.read(cx).text().to_string();
                let value = row.value_input.read(cx).text().to_string();
                KeyValue {
                    key,
                    value,
                    enabled: row.enabled,
                }
            })
            .filter(|kv| !kv.key.is_empty() || !kv.value.is_empty())
            .collect()
    }
}

impl Render for KeyValueEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_count = self.rows.len();
        let mut col = div().flex().flex_col().gap(px(4.)).w_full();

        for i in 0..row_count {
            let row = &self.rows[i];
            let row_idx = i;
            col = col.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(4.))
                    .items_center()
                    .w_full()
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(16.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .rounded(px(2.))
                            .bg(if row.enabled {
                                rgb(0x89b4fa)
                            } else {
                                rgb(0x45475a)
                            })
                            .child(if row.enabled { "✓" } else { "" })
                            .text_size(px(10.))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.rows[row_idx].enabled = !this.rows[row_idx].enabled;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(div().flex_1().child(row.key_input.clone()))
                    .child(div().flex_1().child(row.value_input.clone()))
                    .child(
                        div()
                            .w(px(24.))
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .rounded(px(4.))
                            .hover(|s| s.bg(rgb(0xf38ba8)))
                            .child("×")
                            .text_size(px(14.))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.remove_row(row_idx, cx);
                                }),
                            ),
                    ),
            );
        }

        col = col.child(
            div()
                .flex()
                .cursor_pointer()
                .px(px(8.))
                .py(px(4.))
                .rounded(px(4.))
                .hover(|s| s.bg(rgb(0x313244)))
                .text_color(rgb(0x89b4fa))
                .text_size(px(12.))
                .child("+ Add Row")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.add_row(cx);
                    }),
                ),
        );

        div().track_focus(&self.focus_handle).w_full().child(col)
    }
}

impl Focusable for KeyValueEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestTab {
    Headers,
    Body,
    Params,
    Cookies,
}

pub struct RequestBuilder {
    pub active_tab: RequestTab,
    pub headers_editor: Entity<KeyValueEditor>,
    pub params_editor: Entity<KeyValueEditor>,
    pub cookies_editor: Entity<KeyValueEditor>,
    pub body_input: Entity<TextInput>,
    focus_handle: FocusHandle,
}

impl RequestBuilder {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            active_tab: RequestTab::Headers,
            headers_editor: cx.new(|cx| KeyValueEditor::new(cx)),
            params_editor: cx.new(|cx| KeyValueEditor::new(cx)),
            cookies_editor: cx.new(|cx| KeyValueEditor::new(cx)),
            body_input: cx.new(|cx| TextInput::new(cx, "Request body (JSON)")),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn from_request(request: &Request, cx: &mut Context<Self>) -> Self {
        let headers_editor = cx.new(|cx| KeyValueEditor::from_key_values(&request.headers, cx));
        let params_editor = cx.new(|cx| KeyValueEditor::from_key_values(&request.query_params, cx));
        let cookies_editor = cx.new(|cx| KeyValueEditor::from_key_values(&request.cookies, cx));
        let body_input = cx.new(|cx| {
            let mut input = TextInput::new(cx, "Request body (JSON)");
            if let Some(body) = &request.body {
                let text = match body {
                    Body::Json(s) | Body::Text(s) | Body::Xml(s) => s.clone(),
                    Body::File(p) => format!("file:{}", p),
                    Body::Base64(b) => format!("base64:{}", b),
                };
                input.set_text(&text, cx);
            }
            input
        });
        Self {
            active_tab: RequestTab::Headers,
            headers_editor,
            params_editor,
            cookies_editor,
            body_input,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn to_request_parts(
        &self,
        cx: &App,
    ) -> (Vec<KeyValue>, Vec<KeyValue>, Vec<KeyValue>, Option<Body>) {
        let headers = self.headers_editor.read(cx).to_key_values(cx);
        let params = self.params_editor.read(cx).to_key_values(cx);
        let cookies = self.cookies_editor.read(cx).to_key_values(cx);
        let body_text = self.body_input.read(cx).text().to_string();
        let body = if body_text.is_empty() {
            None
        } else {
            Some(Body::Json(body_text))
        };
        (headers, params, cookies, body)
    }
}

impl Render for RequestBuilder {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.active_tab;

        let tab_button = |label: &str, tab: RequestTab, active: RequestTab| {
            div()
                .px(px(12.))
                .py(px(6.))
                .cursor_pointer()
                .text_size(px(12.))
                .rounded_t(px(4.))
                .when(active == tab, |d| {
                    d.bg(rgb(0x313244))
                        .text_color(rgb(0x89b4fa))
                        .border_b_2()
                        .border_color(rgb(0x89b4fa))
                })
                .when(active != tab, |d| {
                    d.text_color(rgb(0x6c7086))
                        .hover(|s| s.text_color(rgb(0xcdd6f4)))
                })
                .child(label.to_string())
        };

        div()
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .child(
                        tab_button("Headers", RequestTab::Headers, active_tab).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.active_tab = RequestTab::Headers;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        tab_button("Body", RequestTab::Body, active_tab).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.active_tab = RequestTab::Body;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        tab_button("Params", RequestTab::Params, active_tab).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.active_tab = RequestTab::Params;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        tab_button("Cookies", RequestTab::Cookies, active_tab).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.active_tab = RequestTab::Cookies;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .id("request-content")
                    .flex_1()
                    .p(px(8.))
                    .overflow_y_scroll()
                    .when(active_tab == RequestTab::Headers, |d| {
                        d.child(self.headers_editor.clone())
                    })
                    .when(active_tab == RequestTab::Body, |d| {
                        d.child(self.body_input.clone())
                    })
                    .when(active_tab == RequestTab::Params, |d| {
                        d.child(self.params_editor.clone())
                    })
                    .when(active_tab == RequestTab::Cookies, |d| {
                        d.child(self.cookies_editor.clone())
                    }),
            )
    }
}

impl Focusable for RequestBuilder {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
