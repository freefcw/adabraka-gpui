#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
)))]
fn main() {
    println!("real_visual_smoke is not supported on this platform; skipping");
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
))]
fn main() {
    if !std::env::args().any(|arg| arg == "--ignored" || arg == "--include-ignored") {
        println!("real_visual_smoke is ignored by default; pass `-- --ignored` to run it");
        return;
    }

    if let Err(err) = run() {
        eprintln!("real_visual_smoke failed: {err:?}");
        std::process::exit(1);
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
))]
fn run() -> anyhow::Result<()> {
    use gpui::{
        AppContext as _, Context, IntoElement, RealVisualTestContext, Render, Styled as _,
        VisualTestCapabilities, Window, div, px, red, size,
    };
    use std::{cell::RefCell, rc::Rc};

    struct PaintedView;

    impl Render for PaintedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(20.0)).h(px(20.0)).bg(red())
        }
    }

    let capabilities = VisualTestCapabilities::detect();
    let Some(cx) = RealVisualTestContext::new_if_supported() else {
        println!("real visual renderer is not available; skipping");
        return Ok(());
    };
    let outcome = Rc::new(RefCell::new(None));
    let outcome_in_run = outcome.clone();

    cx.run(move |cx| {
        let result = (|| -> anyhow::Result<()> {
            let requested_size = size(px(64.0), px(64.0));
            let window =
                cx.open_offscreen_window(requested_size, |_, app| app.new(|_| PaintedView))?;
            let window = window.into();
            let (bounds, scale_factor) = cx.update_window(window, |_, window, app| {
                let clear = window.draw(app);
                window.request_attention();
                window.present_for_visual_test();
                clear.clear();
                (window.bounds(), window.scale_factor())
            })?;
            let image = cx.capture_screenshot(window)?;
            let expected_origin = gpui::point(px(-10000.0), px(-10000.0));
            let expected_width = (f32::from(bounds.size.width) * scale_factor).round() as u32;
            let expected_height = (f32::from(bounds.size.height) * scale_factor).round() as u32;

            if capabilities.offscreen_positioned_window {
                anyhow::ensure!(
                    bounds.origin == expected_origin,
                    "expected origin {:?}, got {:?}",
                    expected_origin,
                    bounds.origin
                );
            }
            anyhow::ensure!(
                bounds.size.width >= requested_size.width
                    && bounds.size.height >= requested_size.height,
                "expected size at least {:?}, got {:?}",
                requested_size,
                bounds.size
            );
            anyhow::ensure!(
                image.width() == expected_width,
                "expected image width {}, got {}",
                expected_width,
                image.width()
            );
            anyhow::ensure!(
                image.height() == expected_height,
                "expected image height {}, got {}",
                expected_height,
                image.height()
            );
            anyhow::ensure!(
                image.pixels().any(|pixel| pixel[3] > 0),
                "screenshot is fully transparent"
            );
            let Some(first_pixel) = image.pixels().next().copied() else {
                anyhow::bail!("screenshot is empty");
            };
            anyhow::ensure!(
                image.pixels().any(|pixel| *pixel != first_pixel),
                "screenshot is a solid color"
            );
            run_layer_shell_smoke(cx)?;
            Ok(())
        })();
        *outcome_in_run.borrow_mut() = Some(result);
        cx.quit();
    });

    outcome.borrow_mut().take().unwrap_or_else(|| {
        Err(anyhow::anyhow!(
            "real visual smoke did not report an outcome"
        ))
    })
}

#[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "wayland"))]
fn run_layer_shell_smoke(cx: &mut gpui::RealVisualTestContext) -> anyhow::Result<()> {
    if std::env::var_os("GPUI_TEST_LAYER_SHELL").is_none() {
        return Ok(());
    }

    use gpui::{
        AppContext as _, Context, IntoElement, Render, Styled as _, Window, WindowBounds,
        WindowKind, WindowOptions, div,
        layer_shell::{Anchor, LayerShellOptions},
        px, red, size,
    };

    struct LayerShellView;

    impl Render for LayerShellView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().bg(red())
        }
    }

    let bounds = gpui::Bounds::new(gpui::Point::default(), size(px(64.0), px(64.0)));
    let window = cx.app.borrow_mut().open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: false,
            show: true,
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "gpui-real-visual-smoke".to_string(),
                anchor: Anchor::TOP | Anchor::LEFT,
                ..Default::default()
            }),
            ..Default::default()
        },
        |_, app| app.new(|_| LayerShellView),
    )?;
    let window = window.into();
    cx.update_window(window, |_, window, app| {
        window.set_exclusive_zone(px(8.0));
        window.set_exclusive_edge(Anchor::TOP);
        let clear = window.draw(app);
        window.present_for_visual_test();
        clear.clear();
    })?;
    let image = cx.capture_screenshot(window)?;
    anyhow::ensure!(image.width() > 0 && image.height() > 0);
    anyhow::ensure!(image.pixels().any(|pixel| pixel[3] > 0));
    Ok(())
}

#[cfg(not(all(any(target_os = "linux", target_os = "freebsd"), feature = "wayland")))]
fn run_layer_shell_smoke(_cx: &mut gpui::RealVisualTestContext) -> anyhow::Result<()> {
    Ok(())
}
