mod engine;
mod ui;

use hurl_builder::{
    BuildOptions, ResponseSnapshot, SuggestedAssert, SuggestedPredicate, build_response_spec,
};
use engine::model::*;
use engine::{hurl_parser, hurl_runner, hurl_serializer};
use ui::hurl_preview::HurlPreview;
use ui::request_builder::RequestBuilder;
use ui::response_viewer::ResponseViewer;
use ui::text_input::TextInput;

use gpui::{prelude::*, *};
use std::path::PathBuf;

actions!(aranshi_desktop, [Quit, SendRequest, SaveFile, OpenFile]);

struct AranshiApp {
    method: Method,
    url_input: Entity<TextInput>,
    request_builder: Entity<RequestBuilder>,
    response_viewer: Entity<ResponseViewer>,
    hurl_preview: Entity<HurlPreview>,
    file_path: Option<PathBuf>,
    focus_handle: FocusHandle,
    show_method_dropdown: bool,
}

impl AranshiApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let url_input = cx.new(|cx| TextInput::new(cx, "Enter URL..."));
        let request_builder = cx.new(|cx| RequestBuilder::new(cx));
        let response_viewer = cx.new(|cx| ResponseViewer::new(cx));
        let hurl_preview = cx.new(|cx| HurlPreview::new(cx));

        Self {
            method: Method::Get,
            url_input,
            request_builder,
            response_viewer,
            hurl_preview,
            file_path: None,
            focus_handle: cx.focus_handle(),
            show_method_dropdown: false,
        }
    }

    fn build_hurl_file(&self, cx: &App, include_response_status: bool) -> HurlFile {
        let url = self.url_input.read(cx).text().to_string();
        let (headers, params, cookies, body) = self.request_builder.read(cx).to_request_parts(cx);
        let response_spec = if include_response_status {
            self.build_response_spec_from_last_response(cx)
        } else {
            None
        };

        HurlFile {
            path: self.file_path.clone(),
            entries: vec![Entry {
                comment: None,
                request: Request {
                    method: self.method.clone(),
                    url,
                    headers,
                    query_params: params,
                    cookies,
                    body,
                    form_params: Vec::new(),
                    options: Vec::new(),
                },
                response_spec,
            }],
        }
    }

    fn build_response_spec_from_last_response(&self, cx: &App) -> Option<ResponseSpec> {
        let response = self.response_viewer.read(cx).response.clone()?;
        let snapshot = ResponseSnapshot {
            status: response.status,
            headers: response
                .headers
                .iter()
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect(),
            body: response.body.clone(),
        };
        let suggested = build_response_spec(&snapshot, &BuildOptions::default());
        let asserts = suggested
            .asserts
            .into_iter()
            .map(|assert| match assert {
                SuggestedAssert::HeaderContains { header, value } => Assert {
                    query: Query::Header(header),
                    predicate: Predicate::Contains(Value::String(value)),
                },
                SuggestedAssert::JsonPath {
                    expression,
                    predicate,
                } => Assert {
                    query: Query::JsonPath(expression),
                    predicate: match predicate {
                        SuggestedPredicate::Exists => Predicate::Exists,
                        SuggestedPredicate::IsInteger => Predicate::IsInteger,
                        SuggestedPredicate::IsFloat => Predicate::IsFloat,
                        SuggestedPredicate::IsBoolean => Predicate::IsBoolean,
                        SuggestedPredicate::IsString => Predicate::IsString,
                        SuggestedPredicate::IsCollection => Predicate::IsCollection,
                        SuggestedPredicate::IsEmpty => Predicate::IsEmpty,
                    },
                },
            })
            .collect();

        Some(ResponseSpec {
            status: Some(suggested.status),
            headers: Vec::new(),
            captures: Vec::new(),
            asserts,
        })
    }

    fn update_preview(&mut self, cx: &mut Context<Self>) {
        let file = self.build_hurl_file(cx, false);
        let content = hurl_serializer::serialize(&file);
        self.hurl_preview.update(cx, |preview, cx| {
            preview.set_content(content, cx);
        });
    }

    fn send_request(&mut self, cx: &mut Context<Self>) {
        let file = self.build_hurl_file(cx, false);
        let content = hurl_serializer::serialize(&file);

        self.response_viewer.update(cx, |viewer, cx| {
            viewer.set_loading(cx);
        });

        let response_viewer = self.response_viewer.clone();

        cx.spawn(|_, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = std::thread::spawn(move || hurl_runner::run_hurl_content(&content))
                    .join()
                    .unwrap_or_else(|_| Err("runner thread panicked".to_string()));

                response_viewer
                    .update(&mut cx, |viewer, cx| match result {
                        Ok(responses) => {
                            if let Some(resp) = responses.into_iter().next() {
                                viewer.set_response(resp, cx);
                            } else {
                                viewer.set_error("No response received".to_string(), cx);
                            }
                        }
                        Err(e) => {
                            viewer.set_error(e, cx);
                        }
                    })
                    .ok();
            }
        })
        .detach();
    }

    fn save_file(&mut self, cx: &mut Context<Self>) {
        let file = self.build_hurl_file(cx, true);
        let content = hurl_serializer::serialize(&file);

        if let Some(path) = &self.file_path {
            if let Err(e) = std::fs::write(path, &content) {
                eprintln!("Failed to save: {}", e);
            }
        } else {
            let path = std::env::current_dir()
                .unwrap_or_default()
                .join("untitled.hurl");
            if let Err(e) = std::fs::write(&path, &content) {
                eprintln!("Failed to save: {}", e);
            } else {
                self.file_path = Some(path);
            }
        }
    }

    fn open_file(&mut self, cx: &mut Context<Self>) {
        let prompts = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });

        let url_input = self.url_input.clone();
        let hurl_preview = self.hurl_preview.clone();

        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(Ok(Some(paths))) = prompts.await {
                    if let Some(path) = paths.into_iter().next() {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let hurl_file = hurl_parser::parse(&content);

                            this.update(&mut cx, |app, cx: &mut Context<Self>| {
                                app.file_path = Some(path);

                                if let Some(entry) = hurl_file.entries.first() {
                                    app.method = entry.request.method.clone();

                                    url_input.update(cx, |input, cx| {
                                        input.set_text(&entry.request.url, cx);
                                    });

                                    let request = entry.request.clone();
                                    let new_builder = cx.new(|cx| {
                                        RequestBuilder::from_request(&request, cx)
                                    });
                                    app.request_builder = new_builder;
                                }

                                let preview_content = hurl_serializer::serialize(&hurl_file);
                                hurl_preview.update(cx, |preview, cx| {
                                    preview.set_content(preview_content, cx);
                                });

                                cx.notify();
                            })
                            .ok();
                        }
                    }
                }
            }
        })
        .detach();
    }

    fn method_display(&self) -> &str {
        match self.method {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Options => "OPTIONS",
            Method::Head => "HEAD",
        }
    }

    fn method_color(&self) -> u32 {
        match self.method {
            Method::Get => 0xa6e3a1,
            Method::Post => 0xf9e2af,
            Method::Put => 0x89b4fa,
            Method::Delete => 0xf38ba8,
            Method::Patch => 0xcba6f7,
            Method::Options => 0x94e2d5,
            Method::Head => 0xf5c2e7,
        }
    }
}

impl Render for AranshiApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_preview(cx);

        let method_text = self.method_display().to_string();
        let method_color = self.method_color();
        let show_dropdown = self.show_method_dropdown;

        div()
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .on_action(cx.listener(|this, _: &Quit, _window, cx| {
                cx.quit();
                let _ = this;
            }))
            .on_action(cx.listener(|this, _: &SendRequest, _window, cx| {
                this.send_request(cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFile, _window, cx| {
                this.save_file(cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFile, _window, cx| {
                this.open_file(cx);
            }))
            // Toolbar
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(16.))
                    .py(px(8.))
                    .bg(rgb(0x181825))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x89b4fa))
                            .child("Aranshi"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .text_size(px(11.))
                                    .bg(rgb(0x313244))
                                    .hover(|s| s.bg(rgb(0x45475a)))
                                    .child("Open")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.open_file(cx);
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .text_size(px(11.))
                                    .bg(rgb(0x313244))
                                    .hover(|s| s.bg(rgb(0x45475a)))
                                    .child("Save")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.save_file(cx);
                                        }),
                                    ),
                            ),
                    ),
            )
            // URL Bar
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .px(px(16.))
                    .py(px(8.))
                    .bg(rgb(0x1e1e2e))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    // Method dropdown
                    .child(
                        div()
                            .relative()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(4.))
                                    .px(px(10.))
                                    .py(px(6.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .bg(rgb(0x313244))
                                    .hover(|s| s.bg(rgb(0x45475a)))
                                    .text_color(rgb(method_color))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(method_text)
                                    .child(
                                        div()
                                            .text_size(px(8.))
                                            .text_color(rgb(0x6c7086))
                                            .child("▼"),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.show_method_dropdown = !this.show_method_dropdown;
                                            cx.notify();
                                        }),
                                    ),
                            )
                            .when(show_dropdown, |d| {
                                let mut dropdown = div()
                                    .absolute()
                                    .top(px(36.))
                                    .left_0()
                                    .bg(rgb(0x313244))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(rgb(0x45475a))
                                    .py(px(4.))
                                    .min_w(px(100.));

                                for method in Method::all() {
                                    let m = method.clone();
                                    let color = match m {
                                        Method::Get => 0xa6e3a1,
                                        Method::Post => 0xf9e2af,
                                        Method::Put => 0x89b4fa,
                                        Method::Delete => 0xf38ba8,
                                        Method::Patch => 0xcba6f7,
                                        Method::Options => 0x94e2d5,
                                        Method::Head => 0xf5c2e7,
                                    };
                                    dropdown = dropdown.child(
                                        div()
                                            .px(px(10.))
                                            .py(px(4.))
                                            .cursor_pointer()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(color))
                                            .hover(|s| s.bg(rgb(0x45475a)))
                                            .child(format!("{}", m))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.method = m.clone();
                                                    this.show_method_dropdown = false;
                                                    cx.notify();
                                                }),
                                            ),
                                    );
                                }
                                d.child(dropdown)
                            }),
                    )
                    // URL input
                    .child(div().flex_1().child(self.url_input.clone()))
                    // Send button
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(6.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .bg(rgb(0x89b4fa))
                            .text_color(rgb(0x1e1e2e))
                            .text_size(px(13.))
                            .font_weight(FontWeight::BOLD)
                            .hover(|s| s.bg(rgb(0xb4d0fb)))
                            .child("Send")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.send_request(cx);
                                }),
                            ),
                    ),
            )
            // Main content area: request builder + hurl preview | response viewer
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    // Left: Request builder + Response viewer stacked
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(400.))
                            .child(self.request_builder.clone())
                            .child(self.response_viewer.clone()),
                    )
                    // Right: Hurl preview
                    .child(
                        div()
                            .w(px(300.))
                            .border_l_1()
                            .border_color(rgb(0x313244))
                            .child(self.hurl_preview.clone()),
                    ),
            )
            // Status bar
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(12.))
                    .py(px(4.))
                    .bg(rgb(0x181825))
                    .border_t_1()
                    .border_color(rgb(0x313244))
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .child(if let Some(path) = &self.file_path {
                        format!("{}", path.display())
                    } else {
                        "No file open".to_string()
                    }),
            )
    }
}

impl Focusable for AranshiApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-enter", SendRequest, None),
            KeyBinding::new("cmd-s", SaveFile, None),
            KeyBinding::new("cmd-o", OpenFile, None),
            KeyBinding::new("backspace", ui::text_input::Backspace, Some("TextInput")),
            KeyBinding::new("delete", ui::text_input::Delete, Some("TextInput")),
            KeyBinding::new("left", ui::text_input::Left, Some("TextInput")),
            KeyBinding::new("right", ui::text_input::Right, Some("TextInput")),
            KeyBinding::new("shift-left", ui::text_input::SelectLeft, Some("TextInput")),
            KeyBinding::new(
                "shift-right",
                ui::text_input::SelectRight,
                Some("TextInput"),
            ),
            KeyBinding::new("cmd-a", ui::text_input::SelectAll, Some("TextInput")),
            KeyBinding::new("cmd-v", ui::text_input::Paste, Some("TextInput")),
            KeyBinding::new("cmd-c", ui::text_input::Copy, Some("TextInput")),
            KeyBinding::new("cmd-x", ui::text_input::Cut, Some("TextInput")),
            KeyBinding::new("home", ui::text_input::Home, Some("TextInput")),
            KeyBinding::new("end", ui::text_input::End, Some("TextInput")),
            KeyBinding::new(
                "ctrl-cmd-space",
                ui::text_input::ShowCharacterPalette,
                Some("TextInput"),
            ),
        ]);

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| AranshiApp::new(cx)),
        )
        .unwrap();
    });
}
