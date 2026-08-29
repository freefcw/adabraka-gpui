use gpui::{App, AppResourceProfile, Application, ApplicationHandle, Platform, QuitMode};
use std::{rc::Rc, sync::Arc};

#[allow(dead_code)]
fn compile_application_surface(
    application: Application,
    platform: Rc<dyn Platform>,
    http_client: Arc<dyn gpui::http_client::HttpClient>,
) {
    let _new: fn() -> Application = Application::new;
    let _headless: fn() -> Application = Application::headless;
    let _background_executor: fn() -> gpui::BackgroundExecutor = gpui::background_executor;
    let _current_platform: fn(bool) -> Rc<dyn Platform> = gpui::current_platform;

    let application = application
        .inaccessible()
        .with_assets(())
        .with_http_client(http_client)
        .with_resource_profile(AppResourceProfile::default())
        .with_quit_mode(QuitMode::Explicit);

    application.on_open_urls(|_| {});
    application.on_reopen(|cx: &mut App| {
        cx.set_window_appearance(Some(gpui::WindowAppearance::Dark));
        let _ = cx.window_appearance();
        cx.set_window_appearance(None);
    });
    let _ = application.background_executor();
    let _ = application.foreground_executor();
    let _ = application.text_system();
    let _ = application.path_for_auxiliary_executable("helper");

    let _: ApplicationHandle = application.run_embedded(|_: &mut App| {});
    Application::with_platform(platform).run(|_: &mut App| {});
}

#[test]
fn application_wrapper_exposes_the_compatibility_surface() {
    let _: fn(Application, Rc<dyn Platform>, Arc<dyn gpui::http_client::HttpClient>) =
        compile_application_surface;
}

#[cfg(feature = "test-support")]
#[allow(dead_code)]
fn compile_real_visual_test_surface(context: gpui::RealVisualTestContext) {
    let _new: fn() -> gpui::RealVisualTestContext = gpui::RealVisualTestContext::new;
    let _new_if_supported: fn() -> Option<gpui::RealVisualTestContext> =
        gpui::RealVisualTestContext::new_if_supported;
    let _with_assets: fn(Arc<dyn gpui::AssetSource>) -> gpui::RealVisualTestContext =
        gpui::RealVisualTestContext::with_asset_source;

    let _: &Rc<gpui::AppCell> = &context.app;
    context.run_until_parked();
    context.advance_clock(std::time::Duration::ZERO);
    context.run(|_: &mut gpui::RealVisualTestContext| {});
}

#[cfg(feature = "test-support")]
#[test]
fn real_visual_wrapper_exposes_the_compatibility_surface() {
    let _: fn(gpui::RealVisualTestContext) = compile_real_visual_test_surface;
}
