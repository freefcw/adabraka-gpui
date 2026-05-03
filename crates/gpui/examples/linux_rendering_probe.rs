use gpui::{
    App, Application, Bounds, Context, Render, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, div, linear_color_stop, multi_stop_linear_gradient,
    prelude::*, px, relative, rgb, size,
};

struct LinuxRenderingProbe {
    enabled: bool,
    ratio: f32,
}

impl LinuxRenderingProbe {
    fn new() -> Self {
        Self {
            enabled: true,
            ratio: 0.67,
        }
    }
}

impl Render for LinuxRenderingProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x0b1020))
            .text_color(rgb(0xe5e7eb))
            .font_family(".SystemUIFont")
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xl().child("Linux WGPU Rendering Probe"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x94a3b8))
                                    .child("Rounded quads, borders, gradients, and repeated quads should all stay visible."),
                            ),
                    )
                    .child(
                        div()
                            .id("toggle-probe-state")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x38bdf8))
                            .bg(rgb(0x082f49))
                            .cursor_pointer()
                            .child("Toggle / repaint")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.enabled = !this.enabled;
                                this.ratio = if this.enabled { 0.67 } else { 0.33 };
                                // 触发整窗 invalidate，复刻最初 bug「滚动一下整片刷新」的路径，
                                // 验证重新上传 instance buffer 后渲染仍然正确，而不仅仅依赖
                                // reactive 系统的 partial redraw。
                                cx.notify();
                                window.refresh();
                            })),
                    ),
            )
            .child(section(
                "1. Quad stride stress grid",
                "If only the first few cells render correctly, storage-buffer struct stride is suspect.",
                stress_grid(),
            ))
            .child(section(
                "2. Toggle track",
                "The outer rounded track and 1px border are the important primitives; the knob is a child quad.",
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(toggle_switch(self.enabled))
                    .child(toggle_switch(!self.enabled))
                    .child("Expected: two pill tracks with visible colored borders."),
            ))
            .child(section(
                "3. Progress bar",
                "A rounded clipped gradient with relative width should be stable before and after scrolling/repaint.",
                progress_bar(self.ratio),
            ))
            .child(section(
                "4. Separator and rounded cards",
                "The 1px line, card backgrounds, and borders should not disappear after moving the window.",
                div()
                    .flex()
                    .flex_col()
                    .rounded(px(18.0))
                    .border_1()
                    .border_color(rgb(0x334155))
                    .bg(rgb(0x111827))
                    .overflow_hidden()
                    .child(div().h(px(44.0)).px_4().flex().items_center().child("Header"))
                    .child(div().w_full().h(px(1.0)).bg(rgb(0xfbbf24)))
                    .child(
                        div()
                            .p_4()
                            .flex()
                            .gap_3()
                            .child(card("solid", rgb(0x1d4ed8)))
                            .child(card("rounded", rgb(0x047857)))
                            .child(card("border", rgb(0x7c2d12))),
                    ),
            ))
    }
}

fn section(title: &'static str, note: &'static str, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .rounded(px(20.0))
        .border_1()
        .border_color(rgb(0x1e293b))
        .bg(rgb(0x0f172a))
        .p_4()
        .child(div().text_lg().child(title))
        .child(div().text_sm().text_color(rgb(0x94a3b8)).child(note))
        .child(body)
}

fn stress_grid() -> impl IntoElement {
    let colors = [
        0xef4444, 0xf97316, 0xf59e0b, 0x84cc16, 0x22c55e, 0x14b8a6, 0x06b6d4, 0x3b82f6, 0x6366f1,
        0x8b5cf6, 0xa855f7, 0xd946ef, 0xec4899, 0xf43f5e, 0xffffff, 0x111827,
    ];

    let mut grid = div().flex().flex_wrap().gap_2();
    for color in colors {
        grid = grid.child(
            div()
                .flex_none()
                .w(px(36.0))
                .h(px(36.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(rgb(0xe5e7eb))
                .bg(rgb(color)),
        );
    }
    grid
}

fn toggle_switch(enabled: bool) -> impl IntoElement {
    let track_bg = if enabled { 0x2563eb } else { 0x9333ea };
    let knob_bg = if enabled { 0xffffff } else { 0x000000 };

    div()
        .flex_none()
        .w(px(44.0))
        .h(px(24.0))
        .flex()
        .items_center()
        .px(px(2.0))
        .rounded_full()
        .border_1()
        .border_color(rgb(0xfbbf24))
        .bg(rgb(track_bg))
        .when(enabled, |el| el.justify_end())
        .when(!enabled, |el| el.justify_start())
        .child(
            div()
                .flex_none()
                .w(px(18.0))
                .h(px(18.0))
                .rounded_full()
                .bg(rgb(knob_bg)),
        )
}

fn progress_bar(ratio: f32) -> impl IntoElement {
    div()
        .w_full()
        .h(px(12.0))
        .rounded_full()
        .overflow_hidden()
        .border_1()
        .border_color(rgb(0x475569))
        .bg(rgb(0x111827))
        .child(
            div()
                .h_full()
                .w(relative(ratio))
                .rounded_full()
                .bg(multi_stop_linear_gradient(
                    90.0,
                    &[
                        linear_color_stop(rgb(0x22c55e), 0.0),
                        linear_color_stop(rgb(0x06b6d4), 0.45),
                        linear_color_stop(rgb(0xf59e0b), 0.75),
                        linear_color_stop(rgb(0xef4444), 1.0),
                    ],
                )),
        )
}

fn card(label: &'static str, color: impl Into<gpui::Background>) -> impl IntoElement {
    let color = color.into();

    div()
        .flex_1()
        .h(px(72.0))
        .rounded(px(14.0))
        .border_1()
        .border_color(rgb(0xcbd5e1))
        .bg(color)
        .flex()
        .items_center()
        .justify_center()
        .child(label)
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(760.0), px(640.0)), cx);
        cx.open_window(
            WindowOptions {
                titlebar: None,
                window_background: WindowBackgroundAppearance::Opaque,
                window_decorations: Some(WindowDecorations::Client),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| LinuxRenderingProbe::new()),
        )
        .unwrap();
        cx.activate(true);
    });
}
