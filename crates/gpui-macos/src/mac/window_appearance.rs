use crate::WindowAppearance;
use objc::runtime::Object;
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSAppearanceNameVibrantDark,
    NSAppearanceNameVibrantLight,
};

pub(crate) unsafe fn from_native(appearance: *mut Object) -> WindowAppearance {
    unsafe {
        let Some(appearance) = appearance.cast::<NSAppearance>().as_ref() else {
            return WindowAppearance::Light;
        };
        let name = appearance.name();
        if &*name == NSAppearanceNameVibrantLight {
            WindowAppearance::VibrantLight
        } else if &*name == NSAppearanceNameVibrantDark {
            WindowAppearance::VibrantDark
        } else if &*name == NSAppearanceNameAqua {
            WindowAppearance::Light
        } else if &*name == NSAppearanceNameDarkAqua {
            WindowAppearance::Dark
        } else {
            println!("unknown appearance: {}", name);
            WindowAppearance::Light
        }
    }
}
