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
//!     .on_click(|_, window, _| {
//!         window.refresh();
//!     })
//! ```
//!
//! This core crate builds the AccessKit tree, maps focused GPUI elements to
//! AccessKit nodes, and dispatches accessibility actions back into GPUI.
//! Platform-specific adapter wiring is provided separately by GPUI's internal
//! platform window implementations.
