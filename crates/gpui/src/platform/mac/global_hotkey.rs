use crate::{Keystroke, Modifiers};
use anyhow::{Result, anyhow};
use core_graphics::event::CGKeyCode;

use super::events::{
    CMD_MOD, NO_MOD, SHIFT_MOD, always_use_command_layout, chars_for_modified_key,
};

const CMD_KEY: u32 = 1 << 8;
const SHIFT_KEY: u32 = 1 << 9;
const OPTION_KEY: u32 = 1 << 11;
const CONTROL_KEY: u32 = 1 << 12;
const FN_KEY: u32 = 1 << 17;
const MAX_VIRTUAL_KEY_CODE: u16 = 0x7e;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeHotkey {
    pub(crate) key_code: u32,
    pub(crate) modifiers: u32,
}

pub(crate) fn hotkey_to_native(keystroke: &Keystroke) -> Result<NativeHotkey> {
    let shift_candidates: &[bool] = if keystroke.modifiers.shift {
        &[true]
    } else {
        &[false, true]
    };

    for &actual_shift in shift_candidates {
        let actual_modifiers = Modifiers {
            shift: actual_shift,
            ..keystroke.modifiers
        };

        for key_code in 0..=MAX_VIRTUAL_KEY_CODE {
            let candidate = semantic_hotkey_for_key_code(key_code, actual_modifiers);
            if hotkeys_match(&candidate, keystroke) {
                return Ok(NativeHotkey {
                    key_code: key_code as u32,
                    modifiers: carbon_modifiers(actual_modifiers),
                });
            }
        }
    }

    Err(anyhow!(
        "unsupported global hotkey key {} on macOS keyboard layout",
        keystroke.key
    ))
}

fn hotkeys_match(candidate: &Keystroke, desired: &Keystroke) -> bool {
    candidate.modifiers == desired.modifiers && candidate.key == desired.key
}

fn carbon_modifiers(modifiers: Modifiers) -> u32 {
    let mut result = 0;
    if modifiers.platform {
        result |= CMD_KEY;
    }
    if modifiers.control {
        result |= CONTROL_KEY;
    }
    if modifiers.alt {
        result |= OPTION_KEY;
    }
    if modifiers.shift {
        result |= SHIFT_KEY;
    }
    if modifiers.function {
        result |= FN_KEY;
    }
    result
}

fn special_key_for_key_code(key_code: CGKeyCode) -> Option<&'static str> {
    // Virtual key codes from Carbon/HIToolbox.
    match key_code {
        0x24 | 0x4c => Some("enter"),
        0x30 => Some("tab"),
        0x31 => Some("space"),
        0x33 => Some("backspace"),
        0x35 => Some("escape"),
        0x40 => Some("f17"),
        0x4f => Some("f18"),
        0x50 => Some("f19"),
        0x5a => Some("f20"),
        0x60 => Some("f5"),
        0x61 => Some("f6"),
        0x62 => Some("f7"),
        0x63 => Some("f3"),
        0x64 => Some("f8"),
        0x65 => Some("f9"),
        0x67 => Some("f11"),
        0x69 => Some("f13"),
        0x6a => Some("f16"),
        0x6b => Some("f14"),
        0x6d => Some("f10"),
        0x6f => Some("f12"),
        0x71 => Some("f15"),
        0x72 => Some("insert"),
        0x73 => Some("home"),
        0x74 => Some("pageup"),
        0x75 => Some("delete"),
        0x76 => Some("f4"),
        0x77 => Some("end"),
        0x78 => Some("f2"),
        0x79 => Some("pagedown"),
        0x7a => Some("f1"),
        0x7b => Some("left"),
        0x7c => Some("right"),
        0x7d => Some("down"),
        0x7e => Some("up"),
        _ => None,
    }
}

fn suppress_function_modifier_for_key_code(key_code: CGKeyCode) -> bool {
    matches!(
        key_code,
        0x40 | 0x4f
            | 0x50
            | 0x5a
            | 0x60
            | 0x61
            | 0x62
            | 0x63
            | 0x64
            | 0x65
            | 0x67
            | 0x69
            | 0x6a
            | 0x6b
            | 0x6d
            | 0x6f
            | 0x71
            | 0x72
            | 0x73
            | 0x74
            | 0x75
            | 0x76
            | 0x77
            | 0x78
            | 0x79
            | 0x7a
            | 0x7b
            | 0x7c
            | 0x7d
            | 0x7e
    )
}

fn semantic_hotkey_for_key_code(key_code: CGKeyCode, actual_modifiers: Modifiers) -> Keystroke {
    use cocoa::appkit::*;

    let special_key = special_key_for_key_code(key_code);
    let first_char = if special_key.is_none() {
        chars_for_modified_key(key_code, NO_MOD)
            .chars()
            .next()
            .map(|ch| ch as u16)
    } else {
        None
    };

    let control = actual_modifiers.control;
    let alt = actual_modifiers.alt;
    let mut shift = actual_modifiers.shift;
    let command = actual_modifiers.platform;
    let function = actual_modifiers.function && !suppress_function_modifier_for_key_code(key_code);

    #[allow(non_upper_case_globals)]
    let key = match special_key {
        Some(key) => key.to_string(),
        None => match first_char {
            Some(0x20) => "space".to_string(),
            Some(0x09) => "tab".to_string(),
            Some(0x0d) | Some(0x03) => "enter".to_string(),
            Some(0x7f) => "backspace".to_string(),
            Some(0x1b) => "escape".to_string(),
            Some(0x19) => "tab".to_string(),
            Some(NSUpArrowFunctionKey) => "up".to_string(),
            Some(NSDownArrowFunctionKey) => "down".to_string(),
            Some(NSLeftArrowFunctionKey) => "left".to_string(),
            Some(NSRightArrowFunctionKey) => "right".to_string(),
            Some(NSPageUpFunctionKey) => "pageup".to_string(),
            Some(NSPageDownFunctionKey) => "pagedown".to_string(),
            Some(NSHomeFunctionKey) => "home".to_string(),
            Some(NSEndFunctionKey) => "end".to_string(),
            Some(NSDeleteFunctionKey) => "delete".to_string(),
            Some(NSHelpFunctionKey) => "insert".to_string(),
            Some(NSF1FunctionKey) => "f1".to_string(),
            Some(NSF2FunctionKey) => "f2".to_string(),
            Some(NSF3FunctionKey) => "f3".to_string(),
            Some(NSF4FunctionKey) => "f4".to_string(),
            Some(NSF5FunctionKey) => "f5".to_string(),
            Some(NSF6FunctionKey) => "f6".to_string(),
            Some(NSF7FunctionKey) => "f7".to_string(),
            Some(NSF8FunctionKey) => "f8".to_string(),
            Some(NSF9FunctionKey) => "f9".to_string(),
            Some(NSF10FunctionKey) => "f10".to_string(),
            Some(NSF11FunctionKey) => "f11".to_string(),
            Some(NSF12FunctionKey) => "f12".to_string(),
            Some(NSF13FunctionKey) => "f13".to_string(),
            Some(NSF14FunctionKey) => "f14".to_string(),
            Some(NSF15FunctionKey) => "f15".to_string(),
            Some(NSF16FunctionKey) => "f16".to_string(),
            Some(NSF17FunctionKey) => "f17".to_string(),
            Some(NSF18FunctionKey) => "f18".to_string(),
            Some(NSF19FunctionKey) => "f19".to_string(),
            Some(NSF20FunctionKey) => "f20".to_string(),
            Some(NSF21FunctionKey) => "f21".to_string(),
            Some(NSF22FunctionKey) => "f22".to_string(),
            Some(NSF23FunctionKey) => "f23".to_string(),
            Some(NSF24FunctionKey) => "f24".to_string(),
            Some(NSF25FunctionKey) => "f25".to_string(),
            Some(NSF26FunctionKey) => "f26".to_string(),
            Some(NSF27FunctionKey) => "f27".to_string(),
            Some(NSF28FunctionKey) => "f28".to_string(),
            Some(NSF29FunctionKey) => "f29".to_string(),
            Some(NSF30FunctionKey) => "f30".to_string(),
            Some(NSF31FunctionKey) => "f31".to_string(),
            Some(NSF32FunctionKey) => "f32".to_string(),
            Some(NSF33FunctionKey) => "f33".to_string(),
            Some(NSF34FunctionKey) => "f34".to_string(),
            Some(NSF35FunctionKey) => "f35".to_string(),
            _ => {
                let mut chars_ignoring_modifiers = chars_for_modified_key(key_code, NO_MOD);
                let mut chars_with_shift = chars_for_modified_key(key_code, SHIFT_MOD);

                if command || always_use_command_layout() {
                    let chars_with_cmd = chars_for_modified_key(key_code, CMD_MOD);
                    let chars_with_both = chars_for_modified_key(key_code, CMD_MOD | SHIFT_MOD);

                    if chars_with_both != chars_with_cmd {
                        chars_with_shift = chars_with_both;
                    } else if chars_with_cmd.to_ascii_uppercase() != chars_with_cmd {
                        chars_with_shift = chars_with_cmd.to_ascii_uppercase();
                    }
                    chars_ignoring_modifiers = chars_with_cmd;
                }

                if shift
                    && chars_ignoring_modifiers
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase())
                {
                    chars_ignoring_modifiers
                } else if shift {
                    shift = false;
                    chars_with_shift
                } else {
                    chars_ignoring_modifiers
                }
            }
        },
    };

    Keystroke {
        modifiers: Modifiers {
            control,
            alt,
            shift,
            platform: command,
            function,
        },
        key,
        key_char: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_non_printable_key_codes_without_character_translation() {
        assert_eq!(
            semantic_hotkey_for_key_code(0x33, Modifiers::default()).key,
            "backspace"
        );
        assert_eq!(
            semantic_hotkey_for_key_code(0x72, Modifiers::default()).key,
            "insert"
        );
        assert_eq!(
            semantic_hotkey_for_key_code(0x73, Modifiers::default()).key,
            "home"
        );
        assert_eq!(
            semantic_hotkey_for_key_code(0x75, Modifiers::default()).key,
            "delete"
        );
        assert_eq!(
            semantic_hotkey_for_key_code(0x7b, Modifiers::default()).key,
            "left"
        );
        assert_eq!(
            semantic_hotkey_for_key_code(0x7a, Modifiers::default()).key,
            "f1"
        );
    }

    #[test]
    fn only_suppresses_function_modifier_for_standalone_function_keys() {
        let function = Modifiers {
            function: true,
            ..Modifiers::default()
        };

        assert!(
            !semantic_hotkey_for_key_code(0x7b, function)
                .modifiers
                .function
        );
        assert!(
            !semantic_hotkey_for_key_code(0x7a, function)
                .modifiers
                .function
        );
        assert!(
            semantic_hotkey_for_key_code(0x33, function)
                .modifiers
                .function
        );
    }
}
