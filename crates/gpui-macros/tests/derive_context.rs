#[test]
fn test_derive_context() {
    use adabraka_gpui::{App, Window};
    use adabraka_gpui_macros::{AppContext, VisualContext};

    #[derive(AppContext, VisualContext)]
    struct _MyCustomContext<'a, 'b> {
        #[app]
        app: &'a mut App,
        #[window]
        window: &'b mut Window,
    }
}
