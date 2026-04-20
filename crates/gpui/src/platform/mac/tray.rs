use super::screen_frame_to_tray_anchor;
use crate::platform::TrayMenuItem;
use crate::{Bounds, Pixels, TrayAnchor, TrayIconRenderingMode};
use objc::{msg_send, runtime::Object, sel, sel_impl};
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSImage, NSMenu, NSMenuItem,
    NSStatusBar, NSStatusItem,
};
use objc2_foundation::{NSData, NSSize, NSString};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    ptr,
};

type ObjcId = *mut Object;

pub(crate) struct MacTray {
    status_item: Retained<NSStatusItem>,
    panel_mode: Cell<bool>,
    stored_menu: RefCell<Option<Retained<NSMenu>>>,
}

impl MacTray {
    #[allow(unused_unsafe)]
    pub fn new() -> Self {
        unsafe {
            let status_bar = NSStatusBar::systemStatusBar();
            let length: f64 = -1.0;
            let status_item = status_bar.statusItemWithLength(length);
            status_item.setVisible(true);

            if let Some(button) = status_item.button(main_thread_marker()) {
                let default_title = NSString::from_str("App");
                button.setTitle(&default_title);
            }

            Self {
                status_item,
                panel_mode: Cell::new(false),
                stored_menu: RefCell::new(None),
            }
        }
    }

    pub fn set_icon_rendering_mode(&self, rendering_mode: TrayIconRenderingMode) {
        unsafe {
            if let Some(button) = self.status_item.button(main_thread_marker()) {
                if let Some(image) = button.image() {
                    Self::apply_icon_rendering_mode(&image, rendering_mode);
                }
            }
        }
    }

    pub fn set_icon(&self, icon_data: Option<&[u8]>, rendering_mode: TrayIconRenderingMode) {
        unsafe {
            let Some(button) = self.status_item.button(main_thread_marker()) else {
                return;
            };
            match icon_data {
                Some(data) => {
                    let ns_data = NSData::with_bytes(data);
                    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &ns_data) {
                        image.setSize(NSSize {
                            width: 18.0,
                            height: 18.0,
                        });
                        Self::apply_icon_rendering_mode(&image, rendering_mode);
                        button.setImage(Some(&image));
                        let empty = NSString::from_str("");
                        button.setTitle(&empty);
                    }
                }
                None => {
                    button.setImage(None);
                }
            }
        }
    }

    #[allow(unused_unsafe)]
    unsafe fn apply_icon_rendering_mode(image: &NSImage, rendering_mode: TrayIconRenderingMode) {
        let is_template = matches!(rendering_mode, TrayIconRenderingMode::Adaptive);
        unsafe { image.setTemplate(is_template) };
    }

    #[allow(dead_code, unused_unsafe)]
    pub fn set_title(&self, title: &str) {
        unsafe {
            if let Some(button) = self.status_item.button(main_thread_marker()) {
                let ns_title = NSString::from_str(title);
                button.setTitle(&ns_title);
            }
        }
    }

    #[allow(unused_unsafe)]
    pub fn set_tooltip(&self, tooltip: &str) {
        unsafe {
            if let Some(button) = self.status_item.button(main_thread_marker()) {
                let ns_tooltip = NSString::from_str(tooltip);
                button.setToolTip(Some(&ns_tooltip));
            }
        }
    }

    pub fn set_menu(&self, items: Vec<TrayMenuItem>) {
        unsafe {
            let menu = NSMenu::new(main_thread_marker());
            menu.setAutoenablesItems(false);
            build_menu_with_selector(
                Retained::as_ptr(&menu) as ObjcId,
                &items,
                sel!(handleTrayMenuItem:),
            );

            self.stored_menu.replace(Some(menu));

            if !self.panel_mode.get() {
                let stored_menu = self.stored_menu.borrow();
                self.status_item.setMenu(stored_menu.as_deref());
            }
        }
    }

    pub fn set_panel_mode(&self, enabled: bool) {
        self.panel_mode.set(enabled);
        unsafe {
            if enabled {
                self.status_item.setMenu(None);

                if let Some(button) = self.status_item.button(main_thread_marker()) {
                    let delegate = get_app_delegate();
                    if !delegate.is_null() {
                        let button = Retained::as_ptr(&button) as ObjcId;
                        let _: () = msg_send![button, setTarget: delegate];
                        let _: () = msg_send![button, setAction: sel!(handleTrayPanelClick:)];
                    }
                }
            } else {
                if let Some(button) = self.status_item.button(main_thread_marker()) {
                    let button = Retained::as_ptr(&button) as ObjcId;
                    let null_sel: *const c_void = ptr::null();
                    let _: () = msg_send![button, setTarget: ptr::null_mut::<Object>()];
                    let _: () = msg_send![button, setAction: null_sel];
                }

                let stored = self.stored_menu.borrow();
                self.status_item.setMenu(stored.as_deref());
            }
        }
    }

    pub fn get_icon_anchor(&self) -> Option<TrayAnchor> {
        unsafe {
            let button = self.status_item.button(main_thread_marker())?;
            let button_window = button.window()?;
            let frame = button_window.frame();
            let screen = button_window.screen()?;
            screen_frame_to_tray_anchor(Retained::as_ptr(&screen) as *mut Object, frame)
        }
    }

    pub fn get_icon_bounds(&self) -> Option<Bounds<Pixels>> {
        self.get_icon_anchor().map(|anchor| anchor.bounds)
    }
}

impl Drop for MacTray {
    #[allow(unused_unsafe)]
    fn drop(&mut self) {
        unsafe {
            let status_bar = NSStatusBar::systemStatusBar();
            status_bar.removeStatusItem(&self.status_item);
        }
    }
}

unsafe fn get_app_delegate() -> ObjcId {
    let app = NSApplication::sharedApplication(main_thread_marker());
    let app = Retained::as_ptr(&app) as ObjcId;
    msg_send![app, delegate]
}

pub(crate) unsafe fn configure_actionable_item_with_selector(
    menu_item: ObjcId,
    item_id: &str,
    selector: objc::runtime::Sel,
) {
    unsafe {
        let delegate = get_app_delegate();
        if !delegate.is_null() {
            let menu_item_ref = &*menu_item.cast::<NSMenuItem>();
            let _: () = msg_send![menu_item, setTarget: delegate];
            let _: () = msg_send![menu_item, setAction: selector];
            let represented = NSString::from_str(item_id);
            let represented = Retained::as_ptr(&represented) as ObjcId;
            let _: () = msg_send![menu_item, setRepresentedObject: represented];
            menu_item_ref.setEnabled(true);
        }
    }
}

pub(crate) unsafe fn build_menu_with_selector(
    menu: ObjcId,
    items: &[TrayMenuItem],
    selector: objc::runtime::Sel,
) {
    unsafe {
        let menu = &*menu.cast::<NSMenu>();
        for item in items {
            match item {
                TrayMenuItem::Action { label, id } => {
                    let title = NSString::from_str(label.as_ref());
                    let empty = NSString::from_str("");
                    let menu_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(main_thread_marker()),
                        &title,
                        None,
                        &empty,
                    );
                    configure_actionable_item_with_selector(
                        Retained::as_ptr(&menu_item) as ObjcId,
                        id.as_ref(),
                        selector,
                    );
                    menu.addItem(&menu_item);
                }
                TrayMenuItem::Separator => {
                    let separator = NSMenuItem::separatorItem(main_thread_marker());
                    menu.addItem(&separator);
                }
                TrayMenuItem::Submenu {
                    label,
                    items: sub_items,
                } => {
                    let title = NSString::from_str(label.as_ref());
                    let empty = NSString::from_str("");
                    let menu_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(main_thread_marker()),
                        &title,
                        None,
                        &empty,
                    );
                    let submenu = NSMenu::new(main_thread_marker());
                    build_menu_with_selector(
                        Retained::as_ptr(&submenu) as ObjcId,
                        sub_items,
                        selector,
                    );
                    menu_item.setSubmenu(Some(&submenu));
                    menu.addItem(&menu_item);
                }
                TrayMenuItem::Toggle { label, checked, id } => {
                    let title = NSString::from_str(label.as_ref());
                    let empty = NSString::from_str("");
                    let menu_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(main_thread_marker()),
                        &title,
                        None,
                        &empty,
                    );
                    configure_actionable_item_with_selector(
                        Retained::as_ptr(&menu_item) as ObjcId,
                        id.as_ref(),
                        selector,
                    );
                    menu_item.setState(if *checked {
                        NSControlStateValueOn
                    } else {
                        NSControlStateValueOff
                    });
                    menu.addItem(&menu_item);
                }
            }
        }
    }
}

fn main_thread_marker() -> MainThreadMarker {
    unsafe { MainThreadMarker::new_unchecked() }
}
