//! A container query element, in the spirit of CSS container queries.
//! The element's own size is determined solely by its style and the space
//! offered by its parent.

use refineable::Refineable as _;

use crate::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, StyleRefinement, Styled,
    Window, relative,
};

/// Construct a container query element with the given render callback.
/// The callback receives the size the element was assigned during layout and
/// returns the contents to display within it.
///
/// By default the element fills its parent (equivalent to `.size_full()`);
/// use the [`Styled`] methods to size it differently. Because the contents
/// don't exist until after layout, they cannot influence the element's size.
///
/// # Example
///
/// ```
/// # use gpui::{container_query, div, px, IntoElement, ParentElement};
/// container_query(|size, _window, _cx| {
///     if size.width < px(240.) {
///         div().child("Narrow layout")
///     } else {
///         div().child("Wide layout")
///     }
/// });
/// ```
pub fn container_query<E>(
    render: impl 'static + FnOnce(Size<Pixels>, &mut Window, &mut App) -> E,
) -> ContainerQuery
where
    E: IntoElement,
{
    let mut base_style = StyleRefinement::default();
    base_style.size.width = Some(relative(1.).into());
    base_style.size.height = Some(relative(1.).into());

    ContainerQuery {
        render: Some(Box::new(|size, window, cx| {
            render(size, window, cx).into_any_element()
        })),
        style: base_style,
    }
}

/// A container query element, created with [`container_query`].
pub struct ContainerQuery {
    render: Option<Box<dyn FnOnce(Size<Pixels>, &mut Window, &mut App) -> AnyElement>>,
    style: StyleRefinement,
}

impl Element for ContainerQuery {
    type RequestLayoutState = ();
    type PrepaintState = Option<AnyElement>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let render = self.render.take()?;
        let mut child = render(bounds.size, window, cx);
        child.layout_as_root(bounds.size.map(AvailableSpace::Definite), window, cx);
        child.prepaint_at(bounds.origin, window, cx);
        Some(child)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(child) = prepaint {
            child.paint(window, cx);
        }
    }
}

impl IntoElement for ContainerQuery {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for ContainerQuery {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Context, IntoElement, ParentElement, Pixels, Render, Size, Styled, TestAppContext, Window,
        container_query, div, px, size,
    };
    use std::cell::Cell;
    use std::rc::Rc;

    struct ContainerQueryView {
        last_size: Rc<Cell<Option<Size<Pixels>>>>,
    }

    impl Render for ContainerQueryView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let last_size = self.last_size.clone();
            container_query(move |container_size, _window, _cx| {
                last_size.set(Some(container_size));
                if container_size.width < px(400.) {
                    div().size_full().child("narrow")
                } else {
                    div().size_full().child("wide")
                }
            })
        }
    }

    #[gpui::test]
    fn container_query_builds_children_from_measured_size(cx: &mut TestAppContext) {
        let last_size = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let last_size = last_size.clone();
            move |_, _| ContainerQueryView { last_size }
        });

        cx.simulate_resize(size(px(640.), px(480.)));
        let wide = last_size
            .get()
            .expect("container_query should run after the wide resize");
        assert!(
            wide.width >= px(400.),
            "wide window should offer a container at least 400px wide, got {}",
            wide.width
        );

        cx.simulate_resize(size(px(320.), px(480.)));
        let narrow = last_size
            .get()
            .expect("container_query should run after the narrow resize");
        assert!(
            narrow.width < px(400.),
            "narrow window should offer a container under 400px wide, got {}",
            narrow.width
        );
        assert_ne!(
            wide, narrow,
            "resizing the window should rebuild children from a new measured size"
        );
    }

    #[gpui::test]
    fn container_query_honors_explicit_size(cx: &mut TestAppContext) {
        let last_size = Rc::new(Cell::new(None));
        struct SizedQuery {
            last_size: Rc<Cell<Option<Size<Pixels>>>>,
        }

        impl Render for SizedQuery {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let last_size = self.last_size.clone();
                container_query(move |container_size, _window, _cx| {
                    last_size.set(Some(container_size));
                    div().size_full()
                })
                .w(px(240.))
                .h(px(80.))
            }
        }

        let (_, cx) = cx.add_window_view({
            let last_size = last_size.clone();
            move |_, _| SizedQuery { last_size }
        });
        cx.simulate_resize(size(px(800.), px(600.)));
        assert_eq!(
            last_size.get(),
            Some(size(px(240.), px(80.))),
            "container_query should measure from its own style, not only the parent offer"
        );
    }
}
