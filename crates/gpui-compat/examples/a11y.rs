//! Accessibility (AccessKit) demo app.
//!
//! Run with: `cargo run -p adabraka-gpui --example a11y`
//!
//! On Linux: `cargo run -p adabraka-gpui --features wayland,x11 --example a11y`

use gpui::{
    AccessibleAction, App, Application, Bounds, Context, FocusHandle, KeyBinding, Role,
    SharedString, Toggled, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb,
    size,
};

actions!(a11y_example, [Tab, TabPrev]);

struct A11yDemo {
    focus_handle: FocusHandle,
    count: i32,
    enabled: bool,
}

impl A11yDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);
        Self {
            focus_handle,
            count: 0,
            enabled: false,
        }
    }
}

impl Render for A11yDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .role(Role::Application)
            .aria_label("Accessibility Demo")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|_, _: &Tab, window, _| window.focus_next()))
            .on_action(cx.listener(|_, _: &TabPrev, window, _| window.focus_prev()))
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .id("heading")
                    .role(Role::Heading)
                    .aria_level(1)
                    .aria_label("Accessibility Demo")
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Accessibility Demo"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("counter")
                            .focusable()
                            .tab_stop(true)
                            .role(Role::SpinButton)
                            .aria_label(SharedString::from(format!("Counter: {}", self.count)))
                            .aria_numeric_value(self.count as f64)
                            .aria_min_numeric_value(0.0)
                            .on_a11y_action(AccessibleAction::Increment, {
                                let this = cx.entity().downgrade();
                                move |_, _, cx| {
                                    this.update(cx, |this, cx| {
                                        this.count += 1;
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            })
                            .on_a11y_action(AccessibleAction::Decrement, {
                                let this = cx.entity().downgrade();
                                move |_, _, cx| {
                                    this.update(cx, |this, cx| {
                                        this.count = (this.count - 1).max(0);
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            }))
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x89b4fa))
                            .text_color(rgb(0x1e1e2e))
                            .cursor_pointer()
                            .child(format!("Count: {}", self.count)),
                    )
                    .child(
                        div()
                            .id("reset")
                            .focusable()
                            .tab_stop(true)
                            .role(Role::Button)
                            .aria_label("Reset counter")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x585b70))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count = 0;
                                cx.notify();
                            }))
                            .child("Reset"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("toggle")
                            .focusable()
                            .tab_stop(true)
                            .role(Role::Switch)
                            .aria_label("Enable feature")
                            .aria_toggled(if self.enabled {
                                Toggled::True
                            } else {
                                Toggled::False
                            })
                            .w(px(44.0))
                            .h(px(24.0))
                            .rounded_full()
                            .cursor_pointer()
                            .when(self.enabled, |element| element.bg(rgb(0x89b4fa)))
                            .when(!self.enabled, |element| element.bg(rgb(0x585b70)))
                            .child(
                                div()
                                    .size(px(20.0))
                                    .rounded_full()
                                    .bg(gpui::white())
                                    .mt(px(2.0))
                                    .when(self.enabled, |element| element.ml(px(22.0)))
                                    .when(!self.enabled, |element| element.ml(px(2.0))),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.enabled = !this.enabled;
                                cx.notify();
                            })),
                    )
                    .child("Enable feature"),
            )
            .child(
                div()
                    .id("task-list")
                    .role(Role::List)
                    .aria_label("Tasks")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(
                        ["Write code", "Run tests", "Ship it"]
                            .iter()
                            .enumerate()
                            .map(|(index, label)| {
                                div()
                                    .id(("task", index))
                                    .role(Role::ListItem)
                                    .aria_label(SharedString::from(*label))
                                    .aria_position_in_set(index + 1)
                                    .aria_size_of_set(3)
                                    .py_1()
                                    .px_2()
                                    .child(format!("{}. {}", index + 1, label))
                            }),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrev, None),
        ]);

        let bounds = Bounds::centered(None, size(px(500.0), px(400.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("GPUI Accessibility Demo".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| A11yDemo::new(window, cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}
