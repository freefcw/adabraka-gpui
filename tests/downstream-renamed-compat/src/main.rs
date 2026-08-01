#[derive(ui::Render)]
struct DerivedView;

#[derive(Clone, PartialEq, ui::Action)]
struct RenamedAction;

#[derive(Clone, PartialEq, ui::Action)]
#[action(no_register)]
struct ExplicitlyRegisteredAction;

ui::register_action!(ExplicitlyRegisteredAction);

#[derive(ui::IntoElement)]
struct DerivedElement;

impl ui::RenderOnce for DerivedElement {
    fn render(self, _window: &mut ui::Window, _cx: &mut ui::App) -> impl ui::IntoElement {
        ui::Empty
    }
}

#[derive(ui::AppContext, ui::VisualContext)]
#[allow(dead_code)]
struct RenamedContext<'a, 'b> {
    #[app]
    app: &'a mut ui::App,
    #[window]
    window: &'b mut ui::Window,
}

#[ui::test]
fn renamed_test_macro(_cx: &mut ui::TestAppContext) {}

fn main() {
    let _ = DerivedView;
    let _ = RenamedAction;
    let _ = DerivedElement;
}
