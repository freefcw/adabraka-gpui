#[cfg(all(target_os = "linux", feature = "wayland"))]
mod example {
    use gpui::{
        App, Application, Bounds, Context, Window, WindowBackgroundAppearance, WindowBounds,
        WindowKind, WindowOptions, div,
        layer_shell::{Anchor, KeyboardInteractivity, LayerShellOptions},
        point,
        prelude::*,
        px, size,
    };

    struct LayerShellExample;

    impl Render for LayerShellExample {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::black())
                .text_color(gpui::white())
                .child("GPUI layer-shell")
        }
    }

    pub fn run() {
        Application::new().run(|cx: &mut App| {
            cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.0), px(0.0)),
                        size(px(500.0), px(200.0)),
                    ))),
                    app_id: Some("gpui-layer-shell-example".to_string()),
                    window_background: WindowBackgroundAppearance::Transparent,
                    kind: WindowKind::LayerShell(LayerShellOptions {
                        namespace: "gpui".to_string(),
                        anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM,
                        margin: Some((px(0.0), px(0.0), px(40.0), px(0.0))),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| LayerShellExample),
            )
            .unwrap();
        });
    }
}

fn main() {
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    example::run();

    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires Linux and the `wayland` feature.");
}
