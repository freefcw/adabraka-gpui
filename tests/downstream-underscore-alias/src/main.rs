#[derive(adabraka_gpui::Render)]
struct DerivedView;

#[derive(Clone, PartialEq, adabraka_gpui::Action)]
struct AliasedAction;

#[derive(Clone, PartialEq, adabraka_gpui::Action)]
#[action(no_register)]
struct ExplicitlyRegisteredAction;

adabraka_gpui::register_action!(ExplicitlyRegisteredAction);

#[derive(adabraka_gpui::IntoElement)]
struct DerivedElement;

impl adabraka_gpui::RenderOnce for DerivedElement {
    fn render(
        self,
        _window: &mut adabraka_gpui::Window,
        _cx: &mut adabraka_gpui::App,
    ) -> impl adabraka_gpui::IntoElement {
        adabraka_gpui::Empty
    }
}

#[derive(adabraka_gpui::AppContext, adabraka_gpui::VisualContext)]
#[allow(dead_code)]
struct AliasedContext<'a, 'b> {
    #[app]
    app: &'a mut adabraka_gpui::App,
    #[window]
    window: &'b mut adabraka_gpui::Window,
}

#[adabraka_gpui::test]
fn aliased_test_macro(_cx: &mut adabraka_gpui::TestAppContext) {}

fn main() {
    let _ = DerivedView;
    let _ = AliasedAction;
    let _ = DerivedElement;
}
