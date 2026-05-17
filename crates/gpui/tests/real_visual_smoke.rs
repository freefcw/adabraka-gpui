#[cfg(not(target_os = "macos"))]
fn main() {
    println!("real_visual_smoke is currently macOS-only; skipping");
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn run() -> anyhow::Result<()> {
    use gpui::{
        AppContext as _, Context, IntoElement, RealVisualTestContext, Render, Styled as _, Window,
        black, div, px, size,
    };
    use std::{cell::RefCell, rc::Rc};

    struct PaintedView;

    impl Render for PaintedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(20.0)).h(px(20.0)).bg(black())
        }
    }

    let Some(cx) = RealVisualTestContext::new_if_supported() else {
        println!("real visual renderer is not available; skipping");
        return Ok(());
    };
    let outcome = Rc::new(RefCell::new(None));
    let outcome_in_run = outcome.clone();

    cx.run(move |cx| {
        let result = (|| -> anyhow::Result<()> {
            let window = cx.open_offscreen_window(size(px(64.0), px(64.0)), |_, app| {
                app.new(|_| PaintedView)
            })?;
            let bounds = cx.update_window(window.into(), |_, window, app| {
                let clear = window.draw(app);
                window.present_for_visual_test();
                clear.clear();
                window.bounds()
            })?;
            let expected_origin = gpui::point(px(-10000.0), px(-10000.0));
            let expected_size = size(px(64.0), px(64.0));

            anyhow::ensure!(
                bounds.origin == expected_origin,
                "expected origin {:?}, got {:?}",
                expected_origin,
                bounds.origin
            );
            anyhow::ensure!(
                bounds.size == expected_size,
                "expected size {:?}, got {:?}",
                expected_size,
                bounds.size
            );
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
