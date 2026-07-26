use gpui::{
    App, AppProfile, Application, Context, Empty, IntoElement, Render, Window, WindowKind,
    WindowOptions,
};

#[derive(gpui::Render)]
struct DerivedView;

struct ManualView;

impl Render for ManualView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

fn compile_startup_and_desktop_contracts() {
    let _headless = Application::headless();
    let _overlay = WindowOptions {
        kind: WindowKind::Overlay,
        mouse_passthrough: true,
        ..WindowOptions::default()
    };

    Application::new()
        .with_resource_profile(AppProfile::Minimal)
        .run(|cx: &mut App| {
            cx.set_keep_alive_without_windows(true);
            cx.set_tray_tooltip("Adabraka GPUI");
            let _ = cx.show_notification("Ready", "Compatibility fixture");
        });
}

fn main() {
    let _ = DerivedView;
    let _ = ManualView;
    let _ = compile_startup_and_desktop_contracts;
}
