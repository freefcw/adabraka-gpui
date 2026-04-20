use crate::WindowAppearance;
use objc::runtime::Object;
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSAppearanceNameVibrantDark,
    NSAppearanceNameVibrantLight,
};

impl WindowAppearance {
    pub(crate) unsafe fn from_native(appearance: *mut Object) -> Self {
        unsafe {
            let Some(appearance) = appearance.cast::<NSAppearance>().as_ref() else {
                return Self::Light;
            };
            let name = appearance.name();
            if &*name == NSAppearanceNameVibrantLight {
                Self::VibrantLight
            } else if &*name == NSAppearanceNameVibrantDark {
                Self::VibrantDark
            } else if &*name == NSAppearanceNameAqua {
                Self::Light
            } else if &*name == NSAppearanceNameDarkAqua {
                Self::Dark
            } else {
                println!("unknown appearance: {}", name);
                Self::Light
            }
        }
    }
}
