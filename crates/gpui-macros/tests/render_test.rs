#[test]
fn test_derive_render() {
    use adabraka_gpui_macros::Render;

    #[derive(Render)]
    struct _Element;
}
