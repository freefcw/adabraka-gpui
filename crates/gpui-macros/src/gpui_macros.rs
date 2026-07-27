#![recursion_limit = "512"]

mod derive_action;
mod derive_app_context;
mod derive_into_element;
mod derive_render;
mod derive_visual_context;
mod register_action;
mod styles;
mod test;

#[cfg(any(feature = "inspector", debug_assertions))]
mod derive_inspector_reflection;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use std::{env, fs, path::PathBuf, sync::OnceLock};
use syn::{DeriveInput, Ident};
use toml_edit::DocumentMut;

/// `Action` derive macro - see the trait documentation for details.
#[proc_macro_derive(Action, attributes(action))]
pub fn derive_action(input: TokenStream) -> TokenStream {
    derive_action::derive_action(input)
}

/// This can be used to register an action with the GPUI runtime when you want to manually implement
/// the `Action` trait. Typically you should use the `Action` derive macro or `actions!` macro
/// instead.
#[proc_macro]
pub fn register_action(ident: TokenStream) -> TokenStream {
    register_action::register_action(ident)
}

/// #[derive(IntoElement)] is used to create a Component out of anything that implements
/// the `RenderOnce` trait.
#[proc_macro_derive(IntoElement)]
pub fn derive_into_element(input: TokenStream) -> TokenStream {
    derive_into_element::derive_into_element(input)
}

#[proc_macro_derive(Render)]
#[doc(hidden)]
pub fn derive_render(input: TokenStream) -> TokenStream {
    derive_render::derive_render(input)
}

/// #[derive(AppContext)] is used to create a context out of anything that holds a `&mut App`
/// Note that a `#[app]` attribute is required to identify the variable holding the &mut App.
///
/// Failure to add the attribute causes a compile error:
///
/// ```compile_fail
/// # #[macro_use] extern crate gpui_macros;
/// # #[macro_use] extern crate gpui;
/// #[derive(AppContext)]
/// struct MyContext<'a> {
///     app: &'a mut gpui::App
/// }
/// ```
#[proc_macro_derive(AppContext, attributes(app))]
pub fn derive_app_context(input: TokenStream) -> TokenStream {
    derive_app_context::derive_app_context(input)
}

/// #[derive(VisualContext)] is used to create a visual context out of anything that holds a `&mut Window` and
/// implements `AppContext`
/// Note that a `#[app]` and a `#[window]` attribute are required to identify the variables holding the &mut App,
/// and &mut Window respectively.
///
/// Failure to add both attributes causes a compile error:
///
/// ```compile_fail
/// # #[macro_use] extern crate gpui_macros;
/// # #[macro_use] extern crate gpui;
/// #[derive(VisualContext)]
/// struct MyContext<'a, 'b> {
///     #[app]
///     app: &'a mut gpui::App,
///     window: &'b mut gpui::Window
/// }
/// ```
///
/// ```compile_fail
/// # #[macro_use] extern crate gpui_macros;
/// # #[macro_use] extern crate gpui;
/// #[derive(VisualContext)]
/// struct MyContext<'a, 'b> {
///     app: &'a mut gpui::App,
///     #[window]
///     window: &'b mut gpui::Window
/// }
/// ```
#[proc_macro_derive(VisualContext, attributes(window, app))]
pub fn derive_visual_context(input: TokenStream) -> TokenStream {
    derive_visual_context::derive_visual_context(input)
}

/// Used by GPUI to generate the style helpers.
#[proc_macro]
#[doc(hidden)]
pub fn style_helpers(input: TokenStream) -> TokenStream {
    styles::style_helpers(input)
}

/// Generates methods for visibility styles.
#[doc(hidden)]
#[proc_macro]
pub fn visibility_style_methods(input: TokenStream) -> TokenStream {
    styles::visibility_style_methods(input)
}

/// Generates methods for margin styles.
#[doc(hidden)]
#[proc_macro]
pub fn margin_style_methods(input: TokenStream) -> TokenStream {
    styles::margin_style_methods(input)
}

/// Generates methods for padding styles.
#[doc(hidden)]
#[proc_macro]
pub fn padding_style_methods(input: TokenStream) -> TokenStream {
    styles::padding_style_methods(input)
}

/// Generates methods for position styles.
#[doc(hidden)]
#[proc_macro]
pub fn position_style_methods(input: TokenStream) -> TokenStream {
    styles::position_style_methods(input)
}

/// Generates methods for overflow styles.
#[doc(hidden)]
#[proc_macro]
pub fn overflow_style_methods(input: TokenStream) -> TokenStream {
    styles::overflow_style_methods(input)
}

/// Generates methods for cursor styles.
#[doc(hidden)]
#[proc_macro]
pub fn cursor_style_methods(input: TokenStream) -> TokenStream {
    styles::cursor_style_methods(input)
}

/// Generates methods for border styles.
#[doc(hidden)]
#[proc_macro]
pub fn border_style_methods(input: TokenStream) -> TokenStream {
    styles::border_style_methods(input)
}

/// Generates methods for box shadow styles.
#[doc(hidden)]
#[proc_macro]
pub fn box_shadow_style_methods(input: TokenStream) -> TokenStream {
    styles::box_shadow_style_methods(input)
}

/// `#[gpui::test]` can be used to annotate test functions that run with GPUI support.
///
/// It supports both synchronous and asynchronous tests, and can provide you with
/// as many `TestAppContext` instances as you need.
/// The output contains a `#[test]` annotation so this can be used with any existing
/// test harness (`cargo test` or `cargo-nextest`).
///
/// ```
/// #[gpui::test]
/// async fn test_foo(mut cx: &TestAppContext) { }
/// ```
///
/// In addition to passing a TestAppContext, you can also ask for a `StdRnd` instance.
/// this will be seeded with the `SEED` environment variable and is used internally by
/// the ForegroundExecutor and BackgroundExecutor to run tasks deterministically in tests.
/// Using the same `StdRng` for behavior in your test will allow you to exercise a wide
/// variety of scenarios and interleavings just by changing the seed.
///
/// # Arguments
///
/// - `#[gpui::test]` with no arguments runs once with the seed `0` or `SEED` env var if set.
/// - `#[gpui::test(seed = 10)]` runs once with the seed `10`.
/// - `#[gpui::test(seeds(10, 20, 30))]` runs three times with seeds `10`, `20`, and `30`.
/// - `#[gpui::test(iterations = 5)]` runs five times, providing as seed the values in the range `0..5`.
/// - `#[gpui::test(retries = 3)]` runs up to four times if it fails to try and make it pass.
/// - `#[gpui::test(on_failure = "crate::test::report_failure")]` will call the specified function after the
///   tests fail so that you can write out more detail about the failure.
///
/// You can combine `iterations = ...` with `seeds(...)`:
/// - `#[gpui::test(iterations = 5, seed = 10)]` is equivalent to `#[gpui::test(seeds(0, 1, 2, 3, 4, 10))]`.
/// - `#[gpui::test(iterations = 5, seeds(10, 20, 30)]` is equivalent to `#[gpui::test(seeds(0, 1, 2, 3, 4, 10, 20, 30))]`.
/// - `#[gpui::test(seeds(10, 20, 30), iterations = 5]` is equivalent to `#[gpui::test(seeds(0, 1, 2, 3, 4, 10, 20, 30))]`.
///
/// # Environment Variables
///
/// - `SEED`: sets a seed for the first run
/// - `ITERATIONS`: forces the value of the `iterations` argument
#[proc_macro_attribute]
pub fn test(args: TokenStream, function: TokenStream) -> TokenStream {
    test::test(args, function)
}

/// When added to a trait, `#[derive_inspector_reflection]` generates a module which provides
/// enumeration and lookup by name of all methods that have the shape `fn method(self) -> Self`.
/// This is used by the inspector so that it can use the builder methods in `Styled` and
/// `StyledExt`.
///
/// The generated module will have the name `<snake_case_trait_name>_reflection` and contain the
/// following functions:
///
/// ```ignore
/// pub fn methods::<T: TheTrait + 'static>() -> Vec<gpui::inspector_reflection::FunctionReflection<T>>;
///
/// pub fn find_method::<T: TheTrait + 'static>() -> Option<gpui::inspector_reflection::FunctionReflection<T>>;
/// ```
///
/// The `invoke` method on `FunctionReflection` will run the method. `FunctionReflection` also
/// provides the method's documentation.
#[cfg(any(feature = "inspector", debug_assertions))]
#[proc_macro_attribute]
pub fn derive_inspector_reflection(_args: TokenStream, input: TokenStream) -> TokenStream {
    derive_inspector_reflection::derive_inspector_reflection(_args, input)
}

pub(crate) fn gpui_crate_path() -> proc_macro2::TokenStream {
    if let Some(path) = configured_gpui_crate_path() {
        return path;
    }

    let facade = crate_name("adabraka-gpui").ok();
    let core = crate_name("adabraka-gpui-core").ok();

    select_gpui_crate_path(facade, core)
}

fn select_gpui_crate_path(
    facade: Option<FoundCrate>,
    core: Option<FoundCrate>,
) -> proc_macro2::TokenStream {
    match (facade, core) {
        (Some(_), Some(_)) => panic!(
            "both adabraka-gpui and adabraka-gpui-core are direct dependencies; \
             set [package.metadata.gpui-macros] crate = \"<dependency identifier>\" \
             in Cargo.toml to select the crate used by GPUI macros"
        ),
        (Some(found), None) => crate_path(found, "adabraka_gpui", "gpui"),
        (None, Some(found)) => crate_path(found, "adabraka_gpui_core", "gpui_core"),
        (None, None) => quote!(::gpui),
    }
}

fn crate_path(
    found: FoundCrate,
    package_identifier: &str,
    library_identifier: &str,
) -> proc_macro2::TokenStream {
    match found {
        FoundCrate::Itself => quote!(::gpui),
        FoundCrate::Name(name) => {
            // Cargo reports the dependency key, but the default library target can differ.
            let name = if name == package_identifier {
                library_identifier
            } else {
                &name
            };
            let ident = Ident::new(&name.replace('-', "_"), proc_macro2::Span::call_site());
            quote!(::#ident)
        }
    }
}

fn configured_gpui_crate_path() -> Option<proc_macro2::TokenStream> {
    static CONFIGURED_CRATE: OnceLock<Option<String>> = OnceLock::new();

    let crate_name = CONFIGURED_CRATE.get_or_init(|| {
        let manifest_path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?).join("Cargo.toml");
        let manifest = fs::read_to_string(manifest_path).ok()?;
        let document = manifest.parse::<DocumentMut>().ok()?;
        document
            .get("package")?
            .get("metadata")?
            .get("gpui-macros")?
            .get("crate")?
            .as_str()
            .map(str::to_owned)
    });
    let ident = syn::parse_str::<Ident>(crate_name.as_deref()?).unwrap_or_else(|_| {
        panic!(
            "package.metadata.gpui-macros.crate must be a Rust crate identifier, got {crate_name:?}"
        )
    });

    Some(quote!(::#ident))
}

pub(crate) fn get_simple_attribute_field(ast: &DeriveInput, name: &'static str) -> Option<Ident> {
    match &ast.data {
        syn::Data::Struct(data_struct) => data_struct
            .fields
            .iter()
            .find(|field| field.attrs.iter().any(|attr| attr.path().is_ident(name)))
            .map(|field| field.ident.clone().unwrap()),
        syn::Data::Enum(_) => None,
        syn::Data::Union(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{FoundCrate, select_gpui_crate_path};

    #[test]
    #[should_panic(expected = "both adabraka-gpui and adabraka-gpui-core are direct dependencies")]
    fn ambiguous_direct_dependencies_require_an_explicit_crate() {
        select_gpui_crate_path(
            Some(FoundCrate::Name("facade_ui".into())),
            Some(FoundCrate::Name("core_ui".into())),
        );
    }
}
