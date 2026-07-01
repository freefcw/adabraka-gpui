use std::{
    cell::{Ref, RefCell, RefMut},
    ffi::c_void,
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
};

use collections::HashMap;
use futures::channel::oneshot::Receiver;

use raw_window_handle as rwh;
use wayland_backend::client::ObjectId;
use wayland_client::WEnum;
use wayland_client::{
    Proxy,
    protocol::{wl_output, wl_surface},
};
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::xdg_surface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self};
use wayland_protocols::{
    wp::fractional_scale::v1::client::wp_fractional_scale_v1,
    xdg::shell::client::xdg_toplevel::XdgToplevel,
};
use wayland_protocols_plasma::blur::client::org_kde_kwin_blur;

use crate::{
    AnyWindowHandle, Bounds, Decorations, DisplayId, Globals, GpuSpecs, Modifiers, Output, Pixels,
    PlatformDisplay, PlatformInput, Point, PromptButton, PromptLevel, RequestFrameOptions,
    ResizeEdge, Size, Tiling, WaylandClientStatePtr, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowControlArea, WindowControls, WindowDecorations, WindowParams, px, size,
};
use crate::{
    Capslock,
    platform::{
        PlatformAtlas, PlatformInputHandler, PlatformWindow,
        linux::wayland::{display::WaylandDisplay, serial::SerialKind},
        wgpu::{GpuContext, WgpuRenderer, WgpuSurfaceConfig},
    },
};
use crate::{
    LayerShellAnchor, LayerShellExclusiveZone, LayerShellKeyboardInteractivity, LayerShellLayer,
    LayerShellOptions, LayerShellProtocolPreference,
    platform::linux::wayland::{
        ext_layer_shell::client::ext_layer_surface_v1,
        wlr_layer_shell::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1},
    },
};
use crate::{WindowKind, scene::Scene};

#[derive(Default)]
pub(crate) struct Callbacks {
    request_frame: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    input: Option<Box<dyn FnMut(crate::PlatformInput) -> crate::DispatchEventResult>>,
    active_status_change: Option<Box<dyn FnMut(bool)>>,
    hover_status_change: Option<Box<dyn FnMut(bool)>>,
    resize: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    moved: Option<Box<dyn FnMut()>>,
    should_close: Option<Box<dyn FnMut() -> bool>>,
    close: Option<Box<dyn FnOnce()>>,
    appearance_changed: Option<Box<dyn FnMut()>>,
}

#[derive(Clone, Debug)]
struct RawWindow {
    window: *mut c_void,
    display: *mut c_void,
}

unsafe impl Send for RawWindow {}
unsafe impl Sync for RawWindow {}

impl rwh::HasWindowHandle for RawWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let window = NonNull::new(self.window).unwrap();
        let handle = rwh::WaylandWindowHandle::new(window);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}
impl rwh::HasDisplayHandle for RawWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let display = NonNull::new(self.display).unwrap();
        let handle = rwh::WaylandDisplayHandle::new(display);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle.into()) })
    }
}

#[derive(Debug)]
struct InProgressConfigure {
    size: Option<Size<Pixels>>,
    fullscreen: bool,
    maximized: bool,
    resizing: bool,
    tiling: Tiling,
}

pub struct WaylandWindowState {
    role: WaylandWindowRole,
    acknowledged_first_configure: bool,
    pub surface: wl_surface::WlSurface,
    app_id: Option<String>,
    appearance: WindowAppearance,
    blur: Option<org_kde_kwin_blur::OrgKdeKwinBlur>,
    viewport: Option<wp_viewport::WpViewport>,
    outputs: HashMap<ObjectId, Output>,
    display: Option<(ObjectId, Output)>,
    globals: Globals,
    renderer: WgpuRenderer,
    bounds: Bounds<Pixels>,
    scale: f32,
    input_handler: Option<PlatformInputHandler>,
    decorations: WindowDecorations,
    background_appearance: WindowBackgroundAppearance,
    fullscreen: bool,
    maximized: bool,
    tiling: Tiling,
    window_bounds: Bounds<Pixels>,
    client: WaylandClientStatePtr,
    handle: AnyWindowHandle,
    active: bool,
    hovered: bool,
    force_render_after_recovery: bool,
    renderer_presented: bool,
    in_progress_configure: Option<InProgressConfigure>,
    resize_throttle: bool,
    in_progress_window_controls: Option<WindowControls>,
    window_controls: WindowControls,
    client_inset: Option<Pixels>,
    visible: bool,
}

pub enum WaylandWindowRole {
    XdgToplevel {
        xdg_surface: xdg_surface::XdgSurface,
        toplevel: xdg_toplevel::XdgToplevel,
        decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    },
    ExtLayer {
        layer_surface: ext_layer_surface_v1::ExtLayerSurfaceV1,
        options: LayerShellOptions,
        configured: bool,
    },
    WlrLayer {
        layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        options: LayerShellOptions,
        configured: bool,
    },
}

impl WaylandWindowRole {
    fn toplevel(&self) -> Option<xdg_toplevel::XdgToplevel> {
        match self {
            WaylandWindowRole::XdgToplevel { toplevel, .. } => Some(toplevel.clone()),
            WaylandWindowRole::ExtLayer { .. } => None,
            WaylandWindowRole::WlrLayer { .. } => None,
        }
    }

    fn is_layer_shell(&self) -> bool {
        matches!(
            self,
            WaylandWindowRole::ExtLayer { .. } | WaylandWindowRole::WlrLayer { .. }
        )
    }
}

#[derive(Clone)]
pub struct WaylandWindowStatePtr {
    state: Rc<RefCell<WaylandWindowState>>,
    callbacks: Rc<RefCell<Callbacks>>,
}

impl WaylandWindowState {
    pub(crate) fn new(
        handle: AnyWindowHandle,
        surface: wl_surface::WlSurface,
        role: WaylandWindowRole,
        appearance: WindowAppearance,
        viewport: Option<wp_viewport::WpViewport>,
        client: WaylandClientStatePtr,
        globals: Globals,
        gpu_context: GpuContext,
        options: WindowParams,
    ) -> anyhow::Result<Self> {
        let renderer = {
            let raw_window = RawWindow {
                window: surface.id().as_ptr().cast::<c_void>(),
                display: surface
                    .backend()
                    .upgrade()
                    .unwrap()
                    .display_ptr()
                    .cast::<c_void>(),
            };
            let config = WgpuSurfaceConfig {
                size: options.bounds.to_device_pixels(1.0).size,
                transparent: true,
                // Prefer Mailbox on Wayland to avoid blocking the event loop on FIFO stalls.
                preferred_present_mode: Some(wgpu::PresentMode::Mailbox),
            };
            WgpuRenderer::new(
                gpu_context,
                &raw_window,
                config,
                None,
                options.atlas_initial_size,
            )?
        };

        Ok(Self {
            role,
            acknowledged_first_configure: false,
            surface,
            app_id: options.app_id,
            blur: None,
            viewport,
            globals,
            outputs: HashMap::default(),
            display: None,
            renderer,
            bounds: options.bounds,
            scale: 1.0,
            input_handler: None,
            decorations: WindowDecorations::Client,
            background_appearance: WindowBackgroundAppearance::Opaque,
            fullscreen: false,
            maximized: false,
            tiling: Tiling::default(),
            window_bounds: options.bounds,
            in_progress_configure: None,
            resize_throttle: false,
            client,
            appearance,
            handle,
            active: false,
            hovered: false,
            force_render_after_recovery: false,
            renderer_presented: false,
            in_progress_window_controls: None,
            window_controls: WindowControls::default(),
            client_inset: None,
            visible: true,
        })
    }

    pub fn is_transparent(&self) -> bool {
        self.decorations == WindowDecorations::Client
            || self.background_appearance != WindowBackgroundAppearance::Opaque
    }

    pub fn primary_output_scale(&mut self) -> i32 {
        let mut scale = 1;
        let mut current_output = self.display.take();
        for (id, output) in self.outputs.iter() {
            if let Some((_, output_data)) = &current_output {
                if output.scale > output_data.scale {
                    current_output = Some((id.clone(), output.clone()));
                }
            } else {
                current_output = Some((id.clone(), output.clone()));
            }
            scale = scale.max(output.scale);
        }
        self.display = current_output;
        scale
    }

    pub fn inset(&self) -> Pixels {
        match self.decorations {
            WindowDecorations::Server => px(0.0),
            WindowDecorations::Client => self.client_inset.unwrap_or(px(0.0)),
        }
    }
}

pub(crate) struct WaylandWindow(pub WaylandWindowStatePtr);
pub enum ImeInput {
    InsertText(String),
    SetMarkedText(String),
    UnmarkText,
    DeleteText,
}

impl Drop for WaylandWindow {
    fn drop(&mut self) {
        let mut state = self.0.state.borrow_mut();
        let surface_id = state.surface.id();
        let client = state.client.clone();

        state.renderer.destroy();
        if let Some(blur) = &state.blur {
            blur.release();
        }
        match &state.role {
            WaylandWindowRole::XdgToplevel {
                xdg_surface,
                toplevel,
                decoration,
            } => {
                if let Some(decoration) = decoration {
                    decoration.destroy();
                }
                toplevel.destroy();
                xdg_surface.destroy();
            }
            WaylandWindowRole::ExtLayer { layer_surface, .. } => {
                layer_surface.destroy();
            }
            WaylandWindowRole::WlrLayer { layer_surface, .. } => {
                layer_surface.destroy();
            }
        }
        if let Some(viewport) = &state.viewport {
            viewport.destroy();
        }
        state.surface.destroy();

        let state_ptr = self.0.clone();
        state
            .globals
            .executor
            .spawn(async move {
                state_ptr.close();
                client.drop_window(&surface_id)
            })
            .detach();
        drop(state);
    }
}

impl WaylandWindow {
    fn borrow(&self) -> Ref<'_, WaylandWindowState> {
        self.0.state.borrow()
    }

    fn borrow_mut(&self) -> RefMut<'_, WaylandWindowState> {
        self.0.state.borrow_mut()
    }

    pub fn new(
        handle: AnyWindowHandle,
        globals: Globals,
        gpu_context: GpuContext,
        client: WaylandClientStatePtr,
        params: WindowParams,
        appearance: WindowAppearance,
        parent: Option<XdgToplevel>,
        outputs: Vec<Output>,
    ) -> anyhow::Result<(Self, ObjectId)> {
        let surface = globals.compositor.create_surface(&globals.qh, ());
        let role = create_window_role(&surface, &globals, &params, parent, &outputs)?;

        if let Some(fractional_scale_manager) = globals.fractional_scale_manager.as_ref() {
            fractional_scale_manager.get_fractional_scale(&surface, &globals.qh, surface.id());
        }

        let viewport = globals
            .viewporter
            .as_ref()
            .map(|viewporter| viewporter.get_viewport(&surface, &globals.qh, ()));

        let mouse_passthrough = params.mouse_passthrough;

        let this = Self(WaylandWindowStatePtr {
            state: Rc::new(RefCell::new(WaylandWindowState::new(
                handle,
                surface.clone(),
                role,
                appearance,
                viewport,
                client,
                globals,
                gpu_context,
                params,
            )?)),
            callbacks: Rc::new(RefCell::new(Callbacks::default())),
        });

        if mouse_passthrough {
            this.set_mouse_passthrough(true);
        }

        // Kick things off
        surface.commit();

        Ok((this, surface.id()))
    }
}

fn create_window_role(
    surface: &wl_surface::WlSurface,
    globals: &Globals,
    params: &WindowParams,
    parent: Option<XdgToplevel>,
    outputs: &[Output],
) -> anyhow::Result<WaylandWindowRole> {
    if let Some(options) = layer_shell_options(params, globals, outputs) {
        if let Some(role) = create_layer_shell_role(surface, globals, params, &options, outputs) {
            return Ok(role);
        }
        return Ok(create_xdg_toplevel_role(surface, globals, params, parent));
    }

    Ok(create_xdg_toplevel_role(surface, globals, params, parent))
}

fn create_layer_shell_role(
    surface: &wl_surface::WlSurface,
    globals: &Globals,
    params: &WindowParams,
    options: &LayerShellOptions,
    outputs: &[Output],
) -> Option<WaylandWindowRole> {
    match options.protocol {
        LayerShellProtocolPreference::ExtThenWlr => {
            create_ext_layer_shell_role(surface, globals, params, options, outputs)
                .or_else(|| create_wlr_layer_shell_role(surface, globals, params, options, outputs))
        }
        LayerShellProtocolPreference::Wlr => {
            create_wlr_layer_shell_role(surface, globals, params, options, outputs)
        }
    }
}

fn create_ext_layer_shell_role(
    surface: &wl_surface::WlSurface,
    globals: &Globals,
    params: &WindowParams,
    options: &LayerShellOptions,
    outputs: &[Output],
) -> Option<WaylandWindowRole> {
    let layer_shell = globals.ext_layer_shell.as_ref()?;
    let output = output_for_layer_shell(params, outputs);
    let layer_surface =
        layer_shell.get_layer_surface(surface, output.as_ref(), &globals.qh, surface.id());

    configure_ext_layer_surface(&layer_surface, options, params.bounds.size);
    if let Some(app_id) = &params.app_id {
        layer_surface.set_app_id(app_id.clone());
    }
    Some(WaylandWindowRole::ExtLayer {
        layer_surface,
        options: options.clone(),
        configured: false,
    })
}

fn create_wlr_layer_shell_role(
    surface: &wl_surface::WlSurface,
    globals: &Globals,
    params: &WindowParams,
    options: &LayerShellOptions,
    outputs: &[Output],
) -> Option<WaylandWindowRole> {
    let layer_shell = globals.wlr_layer_shell.as_ref()?;
    let output = output_for_layer_shell(params, outputs);
    let layer_surface = layer_shell.get_layer_surface(
        surface,
        output.as_ref(),
        options.layer.to_wlr(),
        options.namespace.to_string(),
        &globals.qh,
        surface.id(),
    );

    configure_wlr_layer_surface(&layer_surface, options, params.bounds.size);
    Some(WaylandWindowRole::WlrLayer {
        layer_surface,
        options: options.clone(),
        configured: false,
    })
}

fn output_for_layer_shell(
    params: &WindowParams,
    outputs: &[Output],
) -> Option<wl_output::WlOutput> {
    params.display_id.and_then(|display_id| {
        outputs
            .iter()
            .find(|output| DisplayId::from(output.id.protocol_id()) == display_id)
            .map(|output| output.output.clone())
    })
}

fn create_xdg_toplevel_role(
    surface: &wl_surface::WlSurface,
    globals: &Globals,
    params: &WindowParams,
    parent: Option<XdgToplevel>,
) -> WaylandWindowRole {
    let xdg_surface = globals
        .wm_base
        .get_xdg_surface(surface, &globals.qh, surface.id());
    let toplevel = xdg_surface.get_toplevel(&globals.qh, surface.id());

    if let Some(app_id) = &params.app_id {
        toplevel.set_app_id(app_id.clone());
    }

    if params.kind == WindowKind::Floating || params.kind == WindowKind::Overlay {
        toplevel.set_parent(parent.as_ref());
    }

    if params.kind == WindowKind::Overlay {
        log::warn!(
            "Wayland: WindowKind::Overlay does not support true always-on-top without \
             layer-shell support; falling back to xdg_toplevel."
        );
    }

    if let Some(size) = params.window_min_size {
        toplevel.set_min_size(size.width.0 as i32, size.height.0 as i32);
    }

    let decoration = globals
        .decoration_manager
        .as_ref()
        .map(|decoration_manager| {
            decoration_manager.get_toplevel_decoration(&toplevel, &globals.qh, surface.id())
        });

    WaylandWindowRole::XdgToplevel {
        xdg_surface,
        toplevel,
        decoration,
    }
}

fn layer_shell_options(
    params: &WindowParams,
    globals: &Globals,
    outputs: &[Output],
) -> Option<LayerShellOptions> {
    let mut options = params.layer_shell.clone().or_else(|| {
        (params.kind == WindowKind::Overlay).then(|| LayerShellOptions {
            layer: LayerShellLayer::Overlay,
            keyboard_interactivity: LayerShellKeyboardInteractivity::None,
            namespace: "gpui-overlay".into(),
            ..LayerShellOptions::default()
        })
    })?;

    if params.layer_shell.is_none()
        && params.kind == WindowKind::Overlay
        && let Some(output) = find_output_for_params(params, outputs)
    {
        options = LayerShellOptions::from_window_bounds(
            output.bounds.to_pixels(output.scale as f32),
            params.bounds,
        );
        options.layer = LayerShellLayer::Overlay;
        options.keyboard_interactivity = LayerShellKeyboardInteractivity::None;
        options.namespace = "gpui-overlay".into();
    }

    if !is_layer_shell_available(globals, options.protocol) {
        log::warn!(
            "Wayland: layer-shell requested but no supported layer-shell protocol is available; \
             falling back to xdg_toplevel positioning."
        );
        return None;
    }

    Some(options)
}

fn is_layer_shell_available(globals: &Globals, protocol: LayerShellProtocolPreference) -> bool {
    match protocol {
        LayerShellProtocolPreference::ExtThenWlr => {
            globals.ext_layer_shell.is_some() || globals.wlr_layer_shell.is_some()
        }
        LayerShellProtocolPreference::Wlr => globals.wlr_layer_shell.is_some(),
    }
}

fn find_output_for_params<'a>(params: &WindowParams, outputs: &'a [Output]) -> Option<&'a Output> {
    params
        .display_id
        .and_then(|display_id| {
            outputs
                .iter()
                .find(|output| DisplayId::from(output.id.protocol_id()) == display_id)
        })
        .or_else(|| {
            outputs.iter().find(|output| {
                output
                    .bounds
                    .to_pixels(output.scale as f32)
                    .contains(&params.bounds.center())
            })
        })
        .or_else(|| outputs.first())
}

fn configure_layer_surface_state(
    set_size: impl FnOnce(i32, i32),
    set_anchor: impl FnOnce(u32),
    set_margin: impl FnOnce(i32, i32, i32, i32),
    set_exclusive_zone: impl FnOnce(i32),
    set_keyboard_interactivity: impl FnOnce(LayerShellKeyboardInteractivity),
    options: &LayerShellOptions,
    size: Size<Pixels>,
) {
    set_size(size.width.0.max(1.0) as i32, size.height.0.max(1.0) as i32);
    set_anchor(options.anchor.bits());
    set_margin(
        options.margin.top.0 as i32,
        options.margin.right.0 as i32,
        options.margin.bottom.0 as i32,
        options.margin.left.0 as i32,
    );
    set_exclusive_zone(options.exclusive_zone.to_wayland());
    set_keyboard_interactivity(options.keyboard_interactivity);
}

fn configure_ext_layer_surface(
    layer_surface: &ext_layer_surface_v1::ExtLayerSurfaceV1,
    options: &LayerShellOptions,
    size: Size<Pixels>,
) {
    layer_surface.set_layer(options.layer.to_ext());
    configure_layer_surface_state(
        |width, height| layer_surface.set_size(width, height),
        |anchor| layer_surface.set_anchor(ext_layer_surface_v1::Anchor::from_bits_truncate(anchor)),
        |top, right, bottom, left| layer_surface.set_margin(top, right, bottom, left),
        |zone| layer_surface.set_exclusive_zone(zone),
        |interactivity| {
            let Some(interactivity) = interactivity.to_ext() else {
                log::warn!(
                    "Wayland: ext-layer-shell does not support exclusive keyboard \
                     interactivity; using on-demand focus."
                );
                layer_surface.set_keyboard_interactivity(
                    ext_layer_surface_v1::KeyboardInteractivity::OnDemand,
                );
                return;
            };
            layer_surface.set_keyboard_interactivity(interactivity);
        },
        options,
        size,
    );
}

fn configure_wlr_layer_surface(
    layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    options: &LayerShellOptions,
    size: Size<Pixels>,
) {
    configure_layer_surface_state(
        |width, height| layer_surface.set_size(width as u32, height as u32),
        |anchor| {
            layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::from_bits_truncate(anchor))
        },
        |top, right, bottom, left| layer_surface.set_margin(top, right, bottom, left),
        |zone| layer_surface.set_exclusive_zone(zone),
        |interactivity| layer_surface.set_keyboard_interactivity(interactivity.to_wlr()),
        options,
        size,
    );
}

trait ToWlrLayerShell {
    type Wlr;

    fn to_wlr(self) -> Self::Wlr;
}

impl ToWlrLayerShell for LayerShellLayer {
    type Wlr = zwlr_layer_shell_v1::Layer;

    fn to_wlr(self) -> Self::Wlr {
        match self {
            LayerShellLayer::Background => zwlr_layer_shell_v1::Layer::Background,
            LayerShellLayer::Bottom => zwlr_layer_shell_v1::Layer::Bottom,
            LayerShellLayer::Top => zwlr_layer_shell_v1::Layer::Top,
            LayerShellLayer::Overlay => zwlr_layer_shell_v1::Layer::Overlay,
        }
    }
}

trait ToExtLayerShell {
    type Ext;

    fn to_ext(self) -> Self::Ext;
}

impl ToExtLayerShell for LayerShellLayer {
    type Ext = ext_layer_surface_v1::Layer;

    fn to_ext(self) -> Self::Ext {
        match self {
            LayerShellLayer::Background => ext_layer_surface_v1::Layer::Background,
            LayerShellLayer::Bottom => ext_layer_surface_v1::Layer::Bottom,
            LayerShellLayer::Top => ext_layer_surface_v1::Layer::Top,
            LayerShellLayer::Overlay => ext_layer_surface_v1::Layer::Overlay,
        }
    }
}

impl ToWlrLayerShell for LayerShellAnchor {
    type Wlr = zwlr_layer_surface_v1::Anchor;

    fn to_wlr(self) -> Self::Wlr {
        zwlr_layer_surface_v1::Anchor::from_bits_truncate(self.bits())
    }
}

impl LayerShellExclusiveZone {
    fn to_wayland(self) -> i32 {
        match self {
            LayerShellExclusiveZone::None => 0,
            LayerShellExclusiveZone::Auto => -1,
            LayerShellExclusiveZone::Pixels(zone) => zone,
        }
    }
}

impl ToWlrLayerShell for LayerShellKeyboardInteractivity {
    type Wlr = zwlr_layer_surface_v1::KeyboardInteractivity;

    fn to_wlr(self) -> Self::Wlr {
        match self {
            LayerShellKeyboardInteractivity::None => {
                zwlr_layer_surface_v1::KeyboardInteractivity::None
            }
            LayerShellKeyboardInteractivity::OnDemand => {
                zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand
            }
            LayerShellKeyboardInteractivity::Exclusive => {
                zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive
            }
        }
    }
}

impl ToExtLayerShell for LayerShellKeyboardInteractivity {
    type Ext = Option<ext_layer_surface_v1::KeyboardInteractivity>;

    fn to_ext(self) -> Self::Ext {
        match self {
            LayerShellKeyboardInteractivity::None => {
                Some(ext_layer_surface_v1::KeyboardInteractivity::None)
            }
            LayerShellKeyboardInteractivity::OnDemand => {
                Some(ext_layer_surface_v1::KeyboardInteractivity::OnDemand)
            }
            LayerShellKeyboardInteractivity::Exclusive => None,
        }
    }
}

fn layer_configure_size(current_size: Size<Pixels>, width: i32, height: i32) -> Size<Pixels> {
    match (width, height) {
        (0, 0) => current_size,
        (0, height) => size(current_size.width, px(height as f32)),
        (width, 0) => size(px(width as f32), current_size.height),
        (width, height) => size(px(width as f32), px(height as f32)),
    }
}

impl WaylandWindowStatePtr {
    pub fn handle(&self) -> AnyWindowHandle {
        self.state.borrow().handle
    }

    pub fn surface(&self) -> wl_surface::WlSurface {
        self.state.borrow().surface.clone()
    }

    pub fn toplevel(&self) -> Option<xdg_toplevel::XdgToplevel> {
        self.state.borrow().role.toplevel()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }

    pub fn frame(&self) {
        let mut state = self.state.borrow_mut();
        state.surface.frame(&state.globals.qh, state.surface.id());
        state.resize_throttle = false;
        let force_render = state.force_render_after_recovery;
        state.force_render_after_recovery = false;
        drop(state);

        let mut cb = self.callbacks.borrow_mut();
        if let Some(fun) = cb.request_frame.as_mut() {
            fun(RequestFrameOptions {
                force_render,
                ..Default::default()
            });
            self.update_ime_enabled();
        }
    }

    fn update_ime_enabled(&self) {
        let mut state = self.state.borrow_mut();
        if !state.active {
            return;
        }
        let client = state.client.clone();
        let ime_enabled = state
            .input_handler
            .as_mut()
            .map(|input_handler| input_handler.query_accepts_text_input())
            .unwrap_or(true);
        drop(state);

        if Some(ime_enabled) == client.ime_enabled() {
            return;
        }

        if ime_enabled {
            client.enable_ime();
        } else {
            client.disable_ime();
        }
    }

    pub fn handle_xdg_surface_event(&self, event: xdg_surface::Event) {
        if let xdg_surface::Event::Configure { serial } = event {
            {
                let mut state = self.state.borrow_mut();
                if let Some(window_controls) = state.in_progress_window_controls.take() {
                    state.window_controls = window_controls;

                    drop(state);
                    let mut callbacks = self.callbacks.borrow_mut();
                    if let Some(appearance_changed) = callbacks.appearance_changed.as_mut() {
                        appearance_changed();
                    }
                }
            }
            {
                let mut state = self.state.borrow_mut();

                if let Some(mut configure) = state.in_progress_configure.take() {
                    let got_unmaximized = state.maximized && !configure.maximized;
                    state.fullscreen = configure.fullscreen;
                    state.maximized = configure.maximized;
                    state.tiling = configure.tiling;
                    // Limit interactive resizes to once per vblank
                    if configure.resizing && state.resize_throttle {
                        return;
                    } else if configure.resizing {
                        state.resize_throttle = true;
                    }
                    if !configure.fullscreen && !configure.maximized {
                        configure.size = if got_unmaximized {
                            Some(state.window_bounds.size)
                        } else {
                            compute_outer_size(state.inset(), configure.size, state.tiling)
                        };
                        if let Some(size) = configure.size {
                            state.window_bounds = Bounds {
                                origin: Point::default(),
                                size,
                            };
                        }
                    }
                    drop(state);
                    if let Some(size) = configure.size {
                        self.resize(size);
                    }
                }
            }
            let mut state = self.state.borrow_mut();
            let xdg_surface = match &state.role {
                WaylandWindowRole::XdgToplevel { xdg_surface, .. } => xdg_surface.clone(),
                WaylandWindowRole::ExtLayer { .. } => return,
                WaylandWindowRole::WlrLayer { .. } => return,
            };
            xdg_surface.ack_configure(serial);

            let window_geometry = inset_by_tiling(
                state.bounds.map_origin(|_| px(0.0)),
                state.inset(),
                state.tiling,
            )
            .map(|v| v.0 as i32)
            .map_size(|v| if v <= 0 { 1 } else { v });

            xdg_surface.set_window_geometry(
                window_geometry.origin.x,
                window_geometry.origin.y,
                window_geometry.size.width,
                window_geometry.size.height,
            );

            let request_frame_callback = !state.acknowledged_first_configure;
            if request_frame_callback {
                state.acknowledged_first_configure = true;
                drop(state);
                self.frame();
            }
        }
    }

    pub fn handle_toplevel_decoration_event(&self, event: zxdg_toplevel_decoration_v1::Event) {
        if let zxdg_toplevel_decoration_v1::Event::Configure { mode } = event {
            match mode {
                WEnum::Value(zxdg_toplevel_decoration_v1::Mode::ServerSide) => {
                    self.state.borrow_mut().decorations = WindowDecorations::Server;
                    if let Some(mut appearance_changed) =
                        self.callbacks.borrow_mut().appearance_changed.as_mut()
                    {
                        appearance_changed();
                    }
                }
                WEnum::Value(zxdg_toplevel_decoration_v1::Mode::ClientSide) => {
                    self.state.borrow_mut().decorations = WindowDecorations::Client;
                    // Update background to be transparent
                    if let Some(mut appearance_changed) =
                        self.callbacks.borrow_mut().appearance_changed.as_mut()
                    {
                        appearance_changed();
                    }
                }
                WEnum::Value(_) => {
                    log::warn!("Unknown decoration mode");
                }
                WEnum::Unknown(v) => {
                    log::warn!("Unknown decoration mode: {}", v);
                }
            }
        }
    }

    pub fn handle_ext_layer_surface_event(&self, event: ext_layer_surface_v1::Event) -> bool {
        match event {
            ext_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                let mut state = self.state.borrow_mut();
                let size = layer_configure_size(state.bounds.size, width, height);
                let layer_surface = match &mut state.role {
                    WaylandWindowRole::ExtLayer {
                        layer_surface,
                        configured,
                        ..
                    } => {
                        *configured = true;
                        layer_surface.clone()
                    }
                    WaylandWindowRole::XdgToplevel { .. } | WaylandWindowRole::WlrLayer { .. } => {
                        return false;
                    }
                };
                layer_surface.ack_configure(serial);
                drop(state);
                self.resize(size);
                self.frame();
                false
            }
            ext_layer_surface_v1::Event::Closed => true,
        }
    }

    pub fn handle_wlr_layer_surface_event(&self, event: zwlr_layer_surface_v1::Event) -> bool {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                let mut state = self.state.borrow_mut();
                let size = layer_configure_size(state.bounds.size, width as i32, height as i32);
                let layer_surface = match &mut state.role {
                    WaylandWindowRole::WlrLayer {
                        layer_surface,
                        configured,
                        ..
                    } => {
                        *configured = true;
                        layer_surface.clone()
                    }
                    WaylandWindowRole::XdgToplevel { .. } | WaylandWindowRole::ExtLayer { .. } => {
                        return false;
                    }
                };
                layer_surface.ack_configure(serial);
                drop(state);
                self.resize(size);
                self.frame();
                false
            }
            zwlr_layer_surface_v1::Event::Closed => true,
        }
    }

    pub fn handle_fractional_scale_event(&self, event: wp_fractional_scale_v1::Event) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            self.rescale(scale as f32 / 120.0);
        }
    }

    pub fn handle_toplevel_event(&self, event: xdg_toplevel::Event) -> bool {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                let mut size = if width == 0 || height == 0 {
                    None
                } else {
                    Some(size(px(width as f32), px(height as f32)))
                };

                let states = extract_states::<xdg_toplevel::State>(&states);

                let mut tiling = Tiling::default();
                let mut fullscreen = false;
                let mut maximized = false;
                let mut resizing = false;

                for state in states {
                    match state {
                        xdg_toplevel::State::Maximized => {
                            maximized = true;
                        }
                        xdg_toplevel::State::Fullscreen => {
                            fullscreen = true;
                        }
                        xdg_toplevel::State::Resizing => resizing = true,
                        xdg_toplevel::State::TiledTop => {
                            tiling.top = true;
                        }
                        xdg_toplevel::State::TiledLeft => {
                            tiling.left = true;
                        }
                        xdg_toplevel::State::TiledRight => {
                            tiling.right = true;
                        }
                        xdg_toplevel::State::TiledBottom => {
                            tiling.bottom = true;
                        }
                        _ => {
                            // noop
                        }
                    }
                }

                if fullscreen || maximized {
                    tiling = Tiling::tiled();
                }

                let mut state = self.state.borrow_mut();
                state.in_progress_configure = Some(InProgressConfigure {
                    size,
                    fullscreen,
                    maximized,
                    resizing,
                    tiling,
                });

                false
            }
            xdg_toplevel::Event::Close => {
                let mut cb = self.callbacks.borrow_mut();
                if let Some(mut should_close) = cb.should_close.take() {
                    let result = (should_close)();
                    cb.should_close = Some(should_close);
                    if result {
                        drop(cb);
                        self.close();
                    }
                    result
                } else {
                    true
                }
            }
            xdg_toplevel::Event::WmCapabilities { capabilities } => {
                let mut window_controls = WindowControls::default();

                let states = extract_states::<xdg_toplevel::WmCapabilities>(&capabilities);

                for state in states {
                    match state {
                        xdg_toplevel::WmCapabilities::Maximize => {
                            window_controls.maximize = true;
                        }
                        xdg_toplevel::WmCapabilities::Minimize => {
                            window_controls.minimize = true;
                        }
                        xdg_toplevel::WmCapabilities::Fullscreen => {
                            window_controls.fullscreen = true;
                        }
                        xdg_toplevel::WmCapabilities::WindowMenu => {
                            window_controls.window_menu = true;
                        }
                        _ => {}
                    }
                }

                let mut state = self.state.borrow_mut();
                state.in_progress_window_controls = Some(window_controls);
                false
            }
            _ => false,
        }
    }

    #[allow(clippy::mutable_key_type)]
    pub fn handle_surface_event(
        &self,
        event: wl_surface::Event,
        outputs: HashMap<ObjectId, Output>,
    ) {
        let mut state = self.state.borrow_mut();

        match event {
            wl_surface::Event::Enter { output } => {
                let id = output.id();

                let Some(output) = outputs.get(&id) else {
                    return;
                };

                state.outputs.insert(id, output.clone());

                let scale = state.primary_output_scale();

                // We use `PreferredBufferScale` instead to set the scale if it's available
                if state.surface.version() < wl_surface::EVT_PREFERRED_BUFFER_SCALE_SINCE {
                    state.surface.set_buffer_scale(scale);
                    drop(state);
                    self.rescale(scale as f32);
                }
            }
            wl_surface::Event::Leave { output } => {
                state.outputs.remove(&output.id());

                let scale = state.primary_output_scale();

                // We use `PreferredBufferScale` instead to set the scale if it's available
                if state.surface.version() < wl_surface::EVT_PREFERRED_BUFFER_SCALE_SINCE {
                    state.surface.set_buffer_scale(scale);
                    drop(state);
                    self.rescale(scale as f32);
                }
            }
            wl_surface::Event::PreferredBufferScale { factor } => {
                // We use `WpFractionalScale` instead to set the scale if it's available
                if state.globals.fractional_scale_manager.is_none() {
                    state.surface.set_buffer_scale(factor);
                    drop(state);
                    self.rescale(factor as f32);
                }
            }
            _ => {}
        }
    }

    pub fn handle_ime(&self, ime: ImeInput) {
        let mut state = self.state.borrow_mut();
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            match ime {
                ImeInput::InsertText(text) => {
                    input_handler.replace_text_in_range(None, &text);
                }
                ImeInput::SetMarkedText(text) => {
                    input_handler.replace_and_mark_text_in_range(None, &text, None);
                }
                ImeInput::UnmarkText => {
                    input_handler.unmark_text();
                }
                ImeInput::DeleteText => {
                    if let Some(marked) = input_handler.marked_text_range() {
                        input_handler.replace_text_in_range(Some(marked), "");
                    }
                }
            }
            self.state.borrow_mut().input_handler = Some(input_handler);
        }
    }

    pub fn get_ime_area(&self) -> Option<Bounds<Pixels>> {
        let mut state = self.state.borrow_mut();
        let mut bounds: Option<Bounds<Pixels>> = None;
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            bounds = input_handler.ime_candidate_bounds();
            self.state.borrow_mut().input_handler = Some(input_handler);
        }
        bounds
    }

    pub fn set_size_and_scale(&self, size: Option<Size<Pixels>>, scale: Option<f32>) {
        let (size, scale) = {
            let mut state = self.state.borrow_mut();
            if size.is_none_or(|size| size == state.bounds.size)
                && scale.is_none_or(|scale| scale == state.scale)
            {
                return;
            }
            if let Some(size) = size {
                state.bounds.size = size;
            }
            if let Some(scale) = scale {
                state.scale = scale;
            }
            let device_bounds = state.bounds.to_device_pixels(state.scale);
            state.renderer.update_drawable_size(device_bounds.size);
            (state.bounds.size, state.scale)
        };

        if let Some(ref mut fun) = self.callbacks.borrow_mut().resize {
            fun(size, scale);
        }

        {
            let state = self.state.borrow();
            if let Some(viewport) = &state.viewport {
                viewport.set_destination(size.width.0 as i32, size.height.0 as i32);
            }
        }
    }

    pub fn resize(&self, size: Size<Pixels>) {
        self.set_size_and_scale(Some(size), None);
    }

    pub fn rescale(&self, scale: f32) {
        self.set_size_and_scale(None, Some(scale));
    }

    pub fn close(&self) {
        let mut callbacks = self.callbacks.borrow_mut();
        if let Some(fun) = callbacks.close.take() {
            fun()
        }
    }

    pub fn handle_input(&self, input: PlatformInput) {
        if let Some(ref mut fun) = self.callbacks.borrow_mut().input
            && !fun(input.clone()).propagate
        {
            return;
        }
        if let PlatformInput::KeyDown(event) = input
            && event.keystroke.modifiers.is_subset_of(&Modifiers::shift())
            && let Some(key_char) = &event.keystroke.key_char
        {
            let mut state = self.state.borrow_mut();
            if let Some(mut input_handler) = state.input_handler.take() {
                drop(state);
                input_handler.replace_text_in_range(None, key_char);
                self.state.borrow_mut().input_handler = Some(input_handler);
            }
        }
    }

    pub fn set_focused(&self, focus: bool) {
        self.state.borrow_mut().active = focus;
        if let Some(ref mut fun) = self.callbacks.borrow_mut().active_status_change {
            fun(focus);
        }
    }

    pub fn set_hovered(&self, focus: bool) {
        if let Some(ref mut fun) = self.callbacks.borrow_mut().hover_status_change {
            fun(focus);
        }
    }

    pub fn set_appearance(&mut self, appearance: WindowAppearance) {
        self.state.borrow_mut().appearance = appearance;

        let mut callbacks = self.callbacks.borrow_mut();
        if let Some(ref mut fun) = callbacks.appearance_changed {
            (fun)()
        }
    }

    pub fn primary_output_scale(&self) -> i32 {
        self.state.borrow_mut().primary_output_scale()
    }
}

fn extract_states<'a, S: TryFrom<u32> + 'a>(states: &'a [u8]) -> impl Iterator<Item = S> + 'a
where
    <S as TryFrom<u32>>::Error: 'a,
{
    states
        .chunks_exact(4)
        .flat_map(TryInto::<[u8; 4]>::try_into)
        .map(u32::from_ne_bytes)
        .flat_map(S::try_from)
}

impl rwh::HasWindowHandle for WaylandWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let surface = self.0.surface().id().as_ptr() as *mut libc::c_void;
        let c_ptr = NonNull::new(surface).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandWindowHandle::new(c_ptr);
        let raw_handle = rwh::RawWindowHandle::Wayland(handle);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(raw_handle) })
    }
}

impl rwh::HasDisplayHandle for WaylandWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let display = self
            .0
            .surface()
            .backend()
            .upgrade()
            .ok_or(rwh::HandleError::Unavailable)?
            .display_ptr() as *mut libc::c_void;

        let c_ptr = NonNull::new(display).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandDisplayHandle::new(c_ptr);
        let raw_handle = rwh::RawDisplayHandle::Wayland(handle);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(raw_handle) })
    }
}

impl PlatformWindow for WaylandWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.borrow().bounds
    }

    fn is_maximized(&self) -> bool {
        self.borrow().maximized
    }

    fn window_bounds(&self) -> WindowBounds {
        let state = self.borrow();
        if state.fullscreen {
            WindowBounds::Fullscreen(state.window_bounds)
        } else if state.maximized {
            WindowBounds::Maximized(state.window_bounds)
        } else {
            drop(state);
            WindowBounds::Windowed(self.bounds())
        }
    }

    fn inner_window_bounds(&self) -> WindowBounds {
        let state = self.borrow();
        if state.fullscreen {
            WindowBounds::Fullscreen(state.window_bounds)
        } else if state.maximized {
            WindowBounds::Maximized(state.window_bounds)
        } else {
            let inset = state.inset();
            drop(state);
            WindowBounds::Windowed(self.bounds().inset(inset))
        }
    }

    fn content_size(&self) -> Size<Pixels> {
        self.borrow().bounds.size
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let state = self.borrow();
        let state_ptr = self.0.clone();
        let dp_size = size.to_device_pixels(self.scale_factor());

        match &state.role {
            WaylandWindowRole::XdgToplevel { xdg_surface, .. } => {
                xdg_surface.set_window_geometry(
                    state.bounds.origin.x.0 as i32,
                    state.bounds.origin.y.0 as i32,
                    dp_size.width.0,
                    dp_size.height.0,
                );
            }
            WaylandWindowRole::ExtLayer { layer_surface, .. } => {
                if state.visible {
                    layer_surface
                        .set_size(size.width.0.max(1.0) as i32, size.height.0.max(1.0) as i32);
                }
            }
            WaylandWindowRole::WlrLayer { layer_surface, .. } => {
                if state.visible {
                    layer_surface
                        .set_size(size.width.0.max(1.0) as u32, size.height.0.max(1.0) as u32);
                }
            }
        }

        state
            .globals
            .executor
            .spawn(async move { state_ptr.resize(size) })
            .detach();
    }

    fn scale_factor(&self) -> f32 {
        self.borrow().scale
    }

    fn appearance(&self) -> WindowAppearance {
        self.borrow().appearance
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        let state = self.borrow();
        state.display.as_ref().map(|(id, display)| {
            Rc::new(WaylandDisplay {
                id: id.clone(),
                name: display.name.clone(),
                bounds: display.bounds.to_pixels(state.scale),
            }) as Rc<dyn PlatformDisplay>
        })
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.borrow()
            .client
            .get_client()
            .borrow()
            .mouse_location
            .unwrap_or_default()
    }

    fn modifiers(&self) -> Modifiers {
        self.borrow().client.get_client().borrow().modifiers
    }

    fn capslock(&self) -> Capslock {
        self.borrow().client.get_client().borrow().capslock
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.borrow_mut().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.borrow_mut().input_handler.take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<Receiver<usize>> {
        None
    }

    fn activate(&self) {
        // Try to request an activation token. Even though the activation is likely going to be rejected,
        // KWin and Mutter can use the app_id to visually indicate we're requesting attention.
        let state = self.borrow();
        if let (Some(activation), Some(app_id)) = (&state.globals.activation, state.app_id.clone())
        {
            state.client.set_pending_activation(state.surface.id());
            let token = activation.get_activation_token(&state.globals.qh, ());
            // The serial isn't exactly important here, since the activation is probably going to be rejected anyway.
            let serial = state.client.get_serial(SerialKind::MousePress);
            token.set_app_id(app_id);
            token.set_serial(serial, &state.globals.seat);
            token.set_surface(&state.surface);
            token.commit();
        }
    }

    fn is_active(&self) -> bool {
        self.borrow().active
    }

    fn is_hovered(&self) -> bool {
        self.borrow().hovered
    }

    fn set_title(&mut self, title: &str) {
        let state = self.borrow();
        if let Some(toplevel) = state.role.toplevel() {
            toplevel.set_title(title.to_string());
        }
    }

    fn set_app_id(&mut self, app_id: &str) {
        let mut state = self.borrow_mut();
        match &state.role {
            WaylandWindowRole::XdgToplevel { toplevel, .. } => {
                toplevel.set_app_id(app_id.to_owned());
            }
            WaylandWindowRole::ExtLayer { layer_surface, .. } => {
                layer_surface.set_app_id(app_id.to_owned());
            }
            WaylandWindowRole::WlrLayer { .. } => {}
        }
        state.app_id = Some(app_id.to_owned());
    }

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut state = self.borrow_mut();
        state.background_appearance = background_appearance;
        update_window(state);
    }

    fn minimize(&self) {
        if let Some(toplevel) = self.borrow().role.toplevel() {
            toplevel.set_minimized();
        }
    }

    fn zoom(&self) {
        let state = self.borrow();
        let Some(toplevel) = state.role.toplevel() else {
            return;
        };
        if !state.maximized {
            toplevel.set_maximized();
        } else {
            toplevel.unset_maximized();
        }
    }

    fn toggle_fullscreen(&self) {
        let mut state = self.borrow_mut();
        let Some(toplevel) = state.role.toplevel() else {
            return;
        };
        if !state.fullscreen {
            toplevel.set_fullscreen(None);
        } else {
            toplevel.unset_fullscreen();
        }
    }

    fn is_fullscreen(&self) -> bool {
        self.borrow().fullscreen
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.callbacks.borrow_mut().request_frame = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> crate::DispatchEventResult>) {
        self.0.callbacks.borrow_mut().input = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.callbacks.borrow_mut().active_status_change = Some(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.callbacks.borrow_mut().hover_status_change = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.callbacks.borrow_mut().resize = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().moved = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.callbacks.borrow_mut().should_close = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.callbacks.borrow_mut().close = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().appearance_changed = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        let mut state = self.borrow_mut();

        if state.renderer.device_lost() {
            let raw_window = RawWindow {
                window: state.surface.id().as_ptr().cast::<c_void>(),
                display: state
                    .surface
                    .backend()
                    .upgrade()
                    .unwrap()
                    .display_ptr()
                    .cast::<c_void>(),
            };
            if let Err(err) = state.renderer.recover(&raw_window) {
                log::warn!("GPU recovery failed, will retry on next frame: {err}");
            }

            state.force_render_after_recovery = true;
            return;
        }

        state.renderer_presented = state.renderer.draw(scene);

        if state.renderer.needs_redraw() {
            state.force_render_after_recovery = true;
        }
    }

    fn completed_frame(&self) {
        let mut state = self.borrow_mut();
        if is_unconfigured_layer_shell(&state.role) {
            state.renderer_presented = false;
            return;
        }
        if !state.renderer_presented {
            state.surface.commit();
        }
        state.renderer_presented = false;
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        let state = self.borrow();
        state.renderer.sprite_atlas().clone()
    }

    fn show_window_menu(&self, position: Point<Pixels>) {
        let state = self.borrow();
        let Some(toplevel) = state.role.toplevel() else {
            return;
        };
        let serial = state.client.get_serial(SerialKind::MousePress);
        toplevel.show_window_menu(
            &state.globals.seat,
            serial,
            position.x.0 as i32,
            position.y.0 as i32,
        );
    }

    fn start_window_move(&self) {
        let state = self.borrow();
        let Some(toplevel) = state.role.toplevel() else {
            return;
        };
        let serial = state.client.get_serial(SerialKind::MousePress);
        toplevel._move(&state.globals.seat, serial);
    }

    fn start_window_resize(&self, edge: crate::ResizeEdge) {
        let state = self.borrow();
        let Some(toplevel) = state.role.toplevel() else {
            return;
        };
        toplevel.resize(
            &state.globals.seat,
            state.client.get_serial(SerialKind::MousePress),
            edge.to_xdg(),
        )
    }

    fn window_decorations(&self) -> Decorations {
        let state = self.borrow();
        if state.role.is_layer_shell() {
            return Decorations::Client {
                tiling: Tiling::default(),
            };
        }
        match state.decorations {
            WindowDecorations::Server => Decorations::Server,
            WindowDecorations::Client => Decorations::Client {
                tiling: state.tiling,
            },
        }
    }

    fn request_decorations(&self, decorations: WindowDecorations) {
        let mut state = self.borrow_mut();
        state.decorations = decorations;
        let decoration = match &state.role {
            WaylandWindowRole::XdgToplevel { decoration, .. } => decoration.clone(),
            WaylandWindowRole::ExtLayer { .. } => None,
            WaylandWindowRole::WlrLayer { .. } => None,
        };
        if let Some(decoration) = decoration {
            decoration.set_mode(decorations.to_xdg());
            update_window(state);
        }
    }

    fn window_controls(&self) -> WindowControls {
        self.borrow().window_controls
    }

    fn set_client_inset(&self, inset: Pixels) {
        let mut state = self.borrow_mut();
        if Some(inset) != state.client_inset {
            state.client_inset = Some(inset);
            update_window(state);
        }
    }

    fn update_ime_position(&self, bounds: Bounds<Pixels>) {
        let state = self.borrow();
        state.client.update_ime_position(bounds);
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.borrow().renderer.gpu_specs().into()
    }

    fn play_system_bell(&self) {
        let state = self.borrow();
        if let Some(bell) = state.globals.system_bell.as_ref() {
            bell.ring(Some(&state.surface));
        }
    }

    fn show(&self) {
        let mut state = self.borrow_mut();
        state.visible = true;
        let size = state.bounds.size;
        match &mut state.role {
            WaylandWindowRole::ExtLayer {
                configured,
                layer_surface,
                options,
            } => {
                *configured = false;
                configure_ext_layer_surface(layer_surface, options, size);
            }
            WaylandWindowRole::WlrLayer {
                configured,
                layer_surface,
                options,
            } => {
                *configured = false;
                configure_wlr_layer_surface(layer_surface, options, size);
            }
            WaylandWindowRole::XdgToplevel { .. } => {}
        }
        state.surface.frame(&state.globals.qh, state.surface.id());
        state.surface.commit();
    }

    fn hide(&self) {
        let mut state = self.borrow_mut();
        state.visible = false;
        if is_unconfigured_layer_shell(&state.role) {
            return;
        }
        state.surface.attach(None, 0, 0);
        state.surface.commit();
    }

    fn is_visible(&self) -> bool {
        self.borrow().visible
    }

    fn set_mouse_passthrough(&self, passthrough: bool) {
        let state = self.borrow();
        if passthrough {
            let region = state
                .globals
                .compositor
                .create_region(&state.globals.qh, ());
            state.surface.set_input_region(Some(&region));
            region.destroy();
        } else {
            state.surface.set_input_region(None);
        }
        if !is_unconfigured_layer_shell(&state.role) {
            state.surface.commit();
        }
    }
}

fn is_unconfigured_layer_shell(role: &WaylandWindowRole) -> bool {
    matches!(
        role,
        WaylandWindowRole::ExtLayer {
            configured: false,
            ..
        } | WaylandWindowRole::WlrLayer {
            configured: false,
            ..
        }
    )
}

fn update_window(mut state: RefMut<WaylandWindowState>) {
    let opaque = !state.is_transparent();

    state.renderer.update_transparency(!opaque);
    let mut opaque_area = state.window_bounds.map(|v| v.0 as i32);
    opaque_area.inset(state.inset().0 as i32);

    let region = state
        .globals
        .compositor
        .create_region(&state.globals.qh, ());
    region.add(
        opaque_area.origin.x,
        opaque_area.origin.y,
        opaque_area.size.width,
        opaque_area.size.height,
    );

    // Note that rounded corners make this rectangle API hard to work with.
    // As this is common when using CSD, let's just disable this API.
    if state.background_appearance == WindowBackgroundAppearance::Opaque
        && state.decorations == WindowDecorations::Server
    {
        // Promise the compositor that this region of the window surface
        // contains no transparent pixels. This allows the compositor to skip
        // updating whatever is behind the surface for better performance.
        state.surface.set_opaque_region(Some(&region));
    } else {
        state.surface.set_opaque_region(None);
    }

    if let Some(ref blur_manager) = state.globals.blur_manager {
        if state.background_appearance == WindowBackgroundAppearance::Blurred {
            if state.blur.is_none() {
                let blur = blur_manager.create(&state.surface, &state.globals.qh, ());
                state.blur = Some(blur);
            }
            state.blur.as_ref().unwrap().commit();
        } else {
            // It probably doesn't hurt to clear the blur for opaque windows
            blur_manager.unset(&state.surface);
            if let Some(b) = state.blur.take() {
                b.release()
            }
        }
    }

    region.destroy();
}

impl WindowDecorations {
    fn to_xdg(self) -> zxdg_toplevel_decoration_v1::Mode {
        match self {
            WindowDecorations::Client => zxdg_toplevel_decoration_v1::Mode::ClientSide,
            WindowDecorations::Server => zxdg_toplevel_decoration_v1::Mode::ServerSide,
        }
    }
}

impl ResizeEdge {
    fn to_xdg(self) -> xdg_toplevel::ResizeEdge {
        match self {
            ResizeEdge::Top => xdg_toplevel::ResizeEdge::Top,
            ResizeEdge::TopRight => xdg_toplevel::ResizeEdge::TopRight,
            ResizeEdge::Right => xdg_toplevel::ResizeEdge::Right,
            ResizeEdge::BottomRight => xdg_toplevel::ResizeEdge::BottomRight,
            ResizeEdge::Bottom => xdg_toplevel::ResizeEdge::Bottom,
            ResizeEdge::BottomLeft => xdg_toplevel::ResizeEdge::BottomLeft,
            ResizeEdge::Left => xdg_toplevel::ResizeEdge::Left,
            ResizeEdge::TopLeft => xdg_toplevel::ResizeEdge::TopLeft,
        }
    }
}

/// The configuration event is in terms of the window geometry, which we are constantly
/// updating to account for the client decorations. But that's not the area we want to render
/// to, due to our intrusize CSD. So, here we calculate the 'actual' size, by adding back in the insets
fn compute_outer_size(
    inset: Pixels,
    new_size: Option<Size<Pixels>>,
    tiling: Tiling,
) -> Option<Size<Pixels>> {
    new_size.map(|mut new_size| {
        if !tiling.top {
            new_size.height += inset;
        }
        if !tiling.bottom {
            new_size.height += inset;
        }
        if !tiling.left {
            new_size.width += inset;
        }
        if !tiling.right {
            new_size.width += inset;
        }

        new_size
    })
}

fn inset_by_tiling(mut bounds: Bounds<Pixels>, inset: Pixels, tiling: Tiling) -> Bounds<Pixels> {
    if !tiling.top {
        bounds.origin.y += inset;
        bounds.size.height -= inset;
    }
    if !tiling.bottom {
        bounds.size.height -= inset;
    }
    if !tiling.left {
        bounds.origin.x += inset;
        bounds.size.width -= inset;
    }
    if !tiling.right {
        bounds.size.width -= inset;
    }

    bounds
}

#[cfg(test)]
mod layer_shell_tests {
    use super::*;

    #[test]
    fn layer_configure_size_keeps_current_zero_dimensions() {
        let current_size = size(px(320.0), px(240.0));

        assert_eq!(layer_configure_size(current_size, 0, 0), current_size);
        assert_eq!(
            layer_configure_size(current_size, 640, 0),
            size(px(640.0), px(240.0))
        );
        assert_eq!(
            layer_configure_size(current_size, 0, 480),
            size(px(320.0), px(480.0))
        );
        assert_eq!(
            layer_configure_size(current_size, 640, 480),
            size(px(640.0), px(480.0))
        );
    }

    #[test]
    fn layer_exclusive_zone_maps_to_wayland_values() {
        assert_eq!(LayerShellExclusiveZone::None.to_wayland(), 0);
        assert_eq!(LayerShellExclusiveZone::Auto.to_wayland(), -1);
        assert_eq!(LayerShellExclusiveZone::Pixels(24).to_wayland(), 24);
    }

    #[test]
    fn layer_keyboard_interactivity_maps_to_wlr_protocol() {
        assert_eq!(
            LayerShellKeyboardInteractivity::None.to_wlr(),
            zwlr_layer_surface_v1::KeyboardInteractivity::None
        );
        assert_eq!(
            LayerShellKeyboardInteractivity::OnDemand.to_wlr(),
            zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand
        );
        assert_eq!(
            LayerShellKeyboardInteractivity::Exclusive.to_wlr(),
            zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive
        );
    }

    #[test]
    fn layer_keyboard_interactivity_maps_to_ext_protocol_limits() {
        assert_eq!(
            LayerShellKeyboardInteractivity::None.to_ext(),
            Some(ext_layer_surface_v1::KeyboardInteractivity::None)
        );
        assert_eq!(
            LayerShellKeyboardInteractivity::OnDemand.to_ext(),
            Some(ext_layer_surface_v1::KeyboardInteractivity::OnDemand)
        );
        assert_eq!(LayerShellKeyboardInteractivity::Exclusive.to_ext(), None);
    }

    #[test]
    fn layer_values_map_to_both_wayland_protocols() {
        assert_eq!(
            LayerShellLayer::Background.to_wlr(),
            zwlr_layer_shell_v1::Layer::Background
        );
        assert_eq!(
            LayerShellLayer::Bottom.to_ext(),
            ext_layer_surface_v1::Layer::Bottom
        );
        assert_eq!(
            LayerShellLayer::Top.to_wlr(),
            zwlr_layer_shell_v1::Layer::Top
        );
        assert_eq!(
            LayerShellLayer::Overlay.to_ext(),
            ext_layer_surface_v1::Layer::Overlay
        );
    }
}
