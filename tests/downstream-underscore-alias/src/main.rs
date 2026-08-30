#[derive(fc_gpui::Render)]
struct DerivedView;

#[derive(Clone, PartialEq, fc_gpui::Action)]
struct AliasedAction;

#[derive(Clone, PartialEq, fc_gpui::Action)]
#[action(no_register)]
struct ExplicitlyRegisteredAction;

fc_gpui::register_action!(ExplicitlyRegisteredAction);

#[derive(fc_gpui::IntoElement)]
struct DerivedElement;

impl fc_gpui::RenderOnce for DerivedElement {
    fn render(
        self,
        _window: &mut fc_gpui::Window,
        _cx: &mut fc_gpui::App,
    ) -> impl fc_gpui::IntoElement {
        fc_gpui::Empty
    }
}

#[derive(fc_gpui::AppContext, fc_gpui::VisualContext)]
#[allow(dead_code)]
struct AliasedContext<'a, 'b> {
    #[app]
    app: &'a mut fc_gpui::App,
    #[window]
    window: &'b mut fc_gpui::Window,
}

#[fc_gpui::test]
fn aliased_test_macro(_cx: &mut fc_gpui::TestAppContext) {}

fn main() {
    let _ = DerivedView;
    let _ = AliasedAction;
    let _ = DerivedElement;
}
