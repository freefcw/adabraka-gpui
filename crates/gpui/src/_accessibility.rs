//! Accessibility guide.
//!
//! GPUI exposes accessibility through [AccessKit](https://accesskit.dev/).
//! Elements opt into the accessibility tree by providing both a stable
//! [`ElementId`](crate::ElementId) and an accessible role.
//!
//! The usual entry point for application code is the fluent API on
//! [`StatefulInteractiveElement`](crate::StatefulInteractiveElement):
//!
//! ```ignore
//! div()
//!     .id("save-button")
//!     .role(gpui::Role::Button)
//!     .aria_label("Save")
//!     .aria_description("Save the active document")
//!     .aria_keyshortcuts("Ctrl+S")
//!     .on_click(|_, window, _| {
//!         window.refresh();
//!     })
//! ```
//!
//! `aria_keyshortcuts` only reports existing shortcut metadata to assistive
//! technology; it does not register a GPUI key binding.
//!
//! For development diagnostics, [`Window::debug_a11y_tree_json`](crate::Window::debug_a11y_tree_json)
//! returns the latest completed accessibility frame as JSON once accessibility
//! is active for the window.
//!
//! This core crate builds the AccessKit tree, maps focused GPUI elements to
//! AccessKit nodes, and dispatches accessibility actions back into GPUI.
//! Platform-specific adapter wiring is provided separately by GPUI's internal
//! platform window implementations.
