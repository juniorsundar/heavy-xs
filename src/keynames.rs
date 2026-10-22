//! Key names for `keypressed`/`keyreleased`, without SDL: the editor's `get_key_name` rules over
//! SDL scancodes and keycodes. The native input path converts to those codes first.
//!
//! Reviewed difference: SDL named a letter key by its shifted character in the current layout
//! (so the Turkish dotless-i key read "İ"); a-z keys are now always their plain letter.

/// `SDLK_SCANCODE_MASK`: keycodes for keys without a character carry their scancode.
pub const SCANCODE_MASK: u32 = 1 << 30;
/// `SDL_SCANCODE_KP_1`; KP_1..KP_0 and KP_PERIOD follow it.
const KP_1: i32 = 89;

const NUMPAD: [&str; 11] = [
    "end", "down", "pagedown", "left", "", "right", "home", "up", "pageup", "ins", "delete",
];

/// `SDL_GetScancodeName`: `""` for unnamed or out-of-range scancodes.
fn scancode_name(scancode: i64) -> &'static str {
    usize::try_from(scancode).ok().and_then(|i| SCANCODE_NAMES.get(i)).copied().unwrap_or("")
}

/// `get_key_name`: layout-aware names for ASCII and special keys, the QWERTY scancode name
/// otherwise; keypad keys become their navigation name while NumLock is off. ASCII-lowercased.
pub fn key_name(scancode: i32, key: u32, numlock: bool) -> String {
    if (KP_1..=KP_1 + 10).contains(&scancode) && !numlock {
        return NUMPAD[(scancode - KP_1) as usize].to_string();
    }
    let name = if key & SCANCODE_MASK != 0 {
        scancode_name((key & !SCANCODE_MASK) as i64).to_string()
    } else if key < 128 {
        // SDL_GetKeyName: these six by scancode name, other ASCII as the character itself
        // (key 0 is the empty string).
        match key {
            0x0D => "Return".to_string(),
            0x1B => "Escape".to_string(),
            0x08 => "Backspace".to_string(),
            0x09 => "Tab".to_string(),
            0x20 => "Space".to_string(),
            0x7F => "Delete".to_string(),
            0 => String::new(),
            _ => char::from(key as u8).to_string(),
        }
    } else {
        scancode_name(scancode as i64).to_string()
    };
    name.to_ascii_lowercase()
}

/// SDL 3.4.18's `SDL_scancode_names` (src/events/SDL_keymap.c), indices 0..=290; NULL is "".
const SCANCODE_NAMES: [&str; 291] = [
    "", // 0
    "", // 1
    "", // 2
    "", // 3
    "A", // 4
    "B", // 5
    "C", // 6
    "D", // 7
    "E", // 8
    "F", // 9
    "G", // 10
    "H", // 11
    "I", // 12
    "J", // 13
    "K", // 14
    "L", // 15
    "M", // 16
    "N", // 17
    "O", // 18
    "P", // 19
    "Q", // 20
    "R", // 21
    "S", // 22
    "T", // 23
    "U", // 24
    "V", // 25
    "W", // 26
    "X", // 27
    "Y", // 28
    "Z", // 29
    "1", // 30
    "2", // 31
    "3", // 32
    "4", // 33
    "5", // 34
    "6", // 35
    "7", // 36
    "8", // 37
    "9", // 38
    "0", // 39
    "Return", // 40
    "Escape", // 41
    "Backspace", // 42
    "Tab", // 43
    "Space", // 44
    "-", // 45
    "=", // 46
    "[", // 47
    "]", // 48
    "\\", // 49
    "#", // 50
    ";", // 51
    "'", // 52
    "`", // 53
    ",", // 54
    ".", // 55
    "/", // 56
    "CapsLock", // 57
    "F1", // 58
    "F2", // 59
    "F3", // 60
    "F4", // 61
    "F5", // 62
    "F6", // 63
    "F7", // 64
    "F8", // 65
    "F9", // 66
    "F10", // 67
    "F11", // 68
    "F12", // 69
    "PrintScreen", // 70
    "ScrollLock", // 71
    "Pause", // 72
    "Insert", // 73
    "Home", // 74
    "PageUp", // 75
    "Delete", // 76
    "End", // 77
    "PageDown", // 78
    "Right", // 79
    "Left", // 80
    "Down", // 81
    "Up", // 82
    "Numlock", // 83
    "Keypad /", // 84
    "Keypad *", // 85
    "Keypad -", // 86
    "Keypad +", // 87
    "Keypad Enter", // 88
    "Keypad 1", // 89
    "Keypad 2", // 90
    "Keypad 3", // 91
    "Keypad 4", // 92
    "Keypad 5", // 93
    "Keypad 6", // 94
    "Keypad 7", // 95
    "Keypad 8", // 96
    "Keypad 9", // 97
    "Keypad 0", // 98
    "Keypad .", // 99
    "NonUSBackslash", // 100
    "Application", // 101
    "Power", // 102
    "Keypad =", // 103
    "F13", // 104
    "F14", // 105
    "F15", // 106
    "F16", // 107
    "F17", // 108
    "F18", // 109
    "F19", // 110
    "F20", // 111
    "F21", // 112
    "F22", // 113
    "F23", // 114
    "F24", // 115
    "Execute", // 116
    "Help", // 117
    "Menu", // 118
    "Select", // 119
    "Stop", // 120
    "Again", // 121
    "Undo", // 122
    "Cut", // 123
    "Copy", // 124
    "Paste", // 125
    "Find", // 126
    "Mute", // 127
    "VolumeUp", // 128
    "VolumeDown", // 129
    "", // 130
    "", // 131
    "", // 132
    "Keypad ,", // 133
    "Keypad = (AS400)", // 134
    "International 1", // 135
    "International 2", // 136
    "International 3", // 137
    "International 4", // 138
    "International 5", // 139
    "International 6", // 140
    "International 7", // 141
    "International 8", // 142
    "International 9", // 143
    "Language 1", // 144
    "Language 2", // 145
    "Language 3", // 146
    "Language 4", // 147
    "Language 5", // 148
    "Language 6", // 149
    "Language 7", // 150
    "Language 8", // 151
    "Language 9", // 152
    "AltErase", // 153
    "SysReq", // 154
    "Cancel", // 155
    "Clear", // 156
    "Prior", // 157
    "Return", // 158
    "Separator", // 159
    "Out", // 160
    "Oper", // 161
    "Clear / Again", // 162
    "CrSel", // 163
    "ExSel", // 164
    "", // 165 (3.4.18 names it "Front"; the linked 3.4.14 does not)
    "", // 166
    "", // 167
    "", // 168
    "", // 169
    "", // 170
    "", // 171
    "", // 172
    "", // 173
    "", // 174
    "", // 175
    "Keypad 00", // 176
    "Keypad 000", // 177
    "ThousandsSeparator", // 178
    "DecimalSeparator", // 179
    "CurrencyUnit", // 180
    "CurrencySubUnit", // 181
    "Keypad (", // 182
    "Keypad )", // 183
    "Keypad {", // 184
    "Keypad }", // 185
    "Keypad Tab", // 186
    "Keypad Backspace", // 187
    "Keypad A", // 188
    "Keypad B", // 189
    "Keypad C", // 190
    "Keypad D", // 191
    "Keypad E", // 192
    "Keypad F", // 193
    "Keypad XOR", // 194
    "Keypad ^", // 195
    "Keypad %", // 196
    "Keypad <", // 197
    "Keypad >", // 198
    "Keypad &", // 199
    "Keypad &&", // 200
    "Keypad |", // 201
    "Keypad ||", // 202
    "Keypad :", // 203
    "Keypad #", // 204
    "Keypad Space", // 205
    "Keypad @", // 206
    "Keypad !", // 207
    "Keypad MemStore", // 208
    "Keypad MemRecall", // 209
    "Keypad MemClear", // 210
    "Keypad MemAdd", // 211
    "Keypad MemSubtract", // 212
    "Keypad MemMultiply", // 213
    "Keypad MemDivide", // 214
    "Keypad +/-", // 215
    "Keypad Clear", // 216
    "Keypad ClearEntry", // 217
    "Keypad Binary", // 218
    "Keypad Octal", // 219
    "Keypad Decimal", // 220
    "Keypad Hexadecimal", // 221
    "", // 222
    "", // 223
    "Left Ctrl", // 224
    "Left Shift", // 225
    "Left Alt", // 226
    "Left GUI", // 227
    "Right Ctrl", // 228
    "Right Shift", // 229
    "Right Alt", // 230
    "Right GUI", // 231
    "", // 232
    "", // 233
    "", // 234
    "", // 235
    "", // 236
    "", // 237
    "", // 238
    "", // 239
    "", // 240
    "", // 241
    "", // 242
    "", // 243
    "", // 244
    "", // 245
    "", // 246
    "", // 247
    "", // 248
    "", // 249
    "", // 250
    "", // 251
    "", // 252
    "", // 253
    "", // 254
    "", // 255
    "", // 256
    "ModeSwitch", // 257
    "Sleep", // 258
    "Wake", // 259
    "ChannelUp", // 260
    "ChannelDown", // 261
    "MediaPlay", // 262
    "MediaPause", // 263
    "MediaRecord", // 264
    "MediaFastForward", // 265
    "MediaRewind", // 266
    "MediaTrackNext", // 267
    "MediaTrackPrevious", // 268
    "MediaStop", // 269
    "Eject", // 270
    "MediaPlayPause", // 271
    "MediaSelect", // 272
    "AC New", // 273
    "AC Open", // 274
    "AC Close", // 275
    "AC Exit", // 276
    "AC Save", // 277
    "AC Print", // 278
    "AC Properties", // 279
    "AC Search", // 280
    "AC Home", // 281
    "AC Back", // 282
    "AC Forward", // 283
    "AC Stop", // 284
    "AC Refresh", // 285
    "AC Bookmarks", // 286
    "SoftLeft", // 287
    "SoftRight", // 288
    "Call", // 289
    "EndCall", // 290
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_the_historical_contract() {
        let cases: &[(i32, u32, bool, &str)] = &[
            (4, 97, true, "a"),
            (4, 65, true, "a"),
            (44, 32, true, "space"),
            (40, 0x0D, true, "return"),
            (58, 0x4000003A, true, "f1"),
            (224, 0x400000E0, true, "left ctrl"),
            (4, 0x0444, true, "a"),
            (49, 92, true, "\\"),
            (53, 126, true, "~"),
            (208, 0x400000D0, true, "keypad memstore"),
            (215, 0x400000D7, true, "keypad +/-"),
            (1000, 1000, true, ""),
            (4, 0, true, ""),
            (89, 0x40000059, false, "end"),
            (89, 0x40000059, true, "keypad 1"),
            (93, 0x4000005D, false, ""),
            (88, 0x40000058, false, "keypad enter"),
            (165, 0x400000A5, true, ""),
            (-1, 0x80, true, ""),
        ];
        for &(scancode, key, numlock, want) in cases {
            assert_eq!(key_name(scancode, key, numlock), want, "{scancode} {key:#x} {numlock}");
        }
    }
}
