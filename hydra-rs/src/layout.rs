pub struct KeyDef {
    pub id: &'static str,
    pub label: &'static str,
    pub led: usize,
    pub row: u8,
    pub col: f32,
    pub width: f32,
}

pub const LED_COUNT: usize = 96;

pub const KEYS: &[KeyDef] = &[
    KeyDef { id: "esc", label: "Esc", led: 1, row: 0, col: 0.0, width: 1.0 },
    KeyDef { id: "1", label: "1", led: 7, row: 0, col: 1.0, width: 1.0 },
    KeyDef { id: "2", label: "2", led: 13, row: 0, col: 2.0, width: 1.0 },
    KeyDef { id: "3", label: "3", led: 19, row: 0, col: 3.0, width: 1.0 },
    KeyDef { id: "4", label: "4", led: 25, row: 0, col: 4.0, width: 1.0 },
    KeyDef { id: "5", label: "5", led: 31, row: 0, col: 5.0, width: 1.0 },
    KeyDef { id: "6", label: "6", led: 37, row: 0, col: 6.0, width: 1.0 },
    KeyDef { id: "7", label: "7", led: 43, row: 0, col: 7.0, width: 1.0 },
    KeyDef { id: "8", label: "8", led: 49, row: 0, col: 8.0, width: 1.0 },
    KeyDef { id: "9", label: "9", led: 55, row: 0, col: 9.0, width: 1.0 },
    KeyDef { id: "0", label: "0", led: 61, row: 0, col: 10.0, width: 1.0 },
    KeyDef { id: "minus", label: "-", led: 67, row: 0, col: 11.0, width: 1.0 },
    KeyDef { id: "equal", label: "=", led: 73, row: 0, col: 12.0, width: 1.0 },
    KeyDef { id: "bksp", label: "Bksp", led: 79, row: 0, col: 13.0, width: 2.0 },
    KeyDef { id: "home", label: "Home", led: 91, row: 0, col: 14.0, width: 1.0 },
    KeyDef { id: "tab", label: "Tab", led: 2, row: 1, col: 0.0, width: 1.5 },
    KeyDef { id: "q", label: "Q", led: 8, row: 1, col: 1.0, width: 1.0 },
    KeyDef { id: "w", label: "W", led: 14, row: 1, col: 2.0, width: 1.0 },
    KeyDef { id: "e", label: "E", led: 20, row: 1, col: 3.0, width: 1.0 },
    KeyDef { id: "r", label: "R", led: 26, row: 1, col: 4.0, width: 1.0 },
    KeyDef { id: "t", label: "T", led: 32, row: 1, col: 5.0, width: 1.0 },
    KeyDef { id: "y", label: "Y", led: 38, row: 1, col: 6.0, width: 1.0 },
    KeyDef { id: "u", label: "U", led: 44, row: 1, col: 7.0, width: 1.0 },
    KeyDef { id: "i", label: "I", led: 50, row: 1, col: 8.0, width: 1.0 },
    KeyDef { id: "o", label: "O", led: 56, row: 1, col: 9.0, width: 1.0 },
    KeyDef { id: "p", label: "P", led: 62, row: 1, col: 10.0, width: 1.0 },
    KeyDef { id: "lb", label: "[", led: 68, row: 1, col: 11.0, width: 1.0 },
    KeyDef { id: "rb", label: "]", led: 74, row: 1, col: 12.0, width: 1.0 },
    KeyDef { id: "bslash", label: "\\", led: 80, row: 1, col: 13.0, width: 1.5 },
    KeyDef { id: "del", label: "Del", led: 92, row: 1, col: 14.0, width: 1.0 },
    KeyDef { id: "caps", label: "Caps", led: 3, row: 2, col: 0.0, width: 1.75 },
    KeyDef { id: "a", label: "A", led: 9, row: 2, col: 1.0, width: 1.0 },
    KeyDef { id: "s", label: "S", led: 15, row: 2, col: 2.0, width: 1.0 },
    KeyDef { id: "d", label: "D", led: 21, row: 2, col: 3.0, width: 1.0 },
    KeyDef { id: "f", label: "F", led: 27, row: 2, col: 4.0, width: 1.0 },
    KeyDef { id: "g", label: "G", led: 33, row: 2, col: 5.0, width: 1.0 },
    KeyDef { id: "h", label: "H", led: 39, row: 2, col: 6.0, width: 1.0 },
    KeyDef { id: "j", label: "J", led: 45, row: 2, col: 7.0, width: 1.0 },
    KeyDef { id: "k", label: "K", led: 51, row: 2, col: 8.0, width: 1.0 },
    KeyDef { id: "l", label: "L", led: 57, row: 2, col: 9.0, width: 1.0 },
    KeyDef { id: "semi", label: ";", led: 63, row: 2, col: 10.0, width: 1.0 },
    KeyDef { id: "quote", label: "'", led: 69, row: 2, col: 11.0, width: 1.0 },
    KeyDef { id: "enter", label: "Enter", led: 81, row: 2, col: 13.0, width: 2.25 },
    KeyDef { id: "pgup", label: "PgUp", led: 93, row: 2, col: 14.0, width: 1.0 },
    KeyDef { id: "lshift", label: "Shift", led: 4, row: 3, col: 0.0, width: 2.25 },
    KeyDef { id: "z", label: "Z", led: 10, row: 3, col: 1.0, width: 1.0 },
    KeyDef { id: "x", label: "X", led: 16, row: 3, col: 2.0, width: 1.0 },
    KeyDef { id: "c", label: "C", led: 22, row: 3, col: 3.0, width: 1.0 },
    KeyDef { id: "v", label: "V", led: 28, row: 3, col: 4.0, width: 1.0 },
    KeyDef { id: "b", label: "B", led: 34, row: 3, col: 5.0, width: 1.0 },
    KeyDef { id: "n", label: "N", led: 40, row: 3, col: 6.0, width: 1.0 },
    KeyDef { id: "m", label: "M", led: 46, row: 3, col: 7.0, width: 1.0 },
    KeyDef { id: "comma", label: ",", led: 52, row: 3, col: 8.0, width: 1.0 },
    KeyDef { id: "dot", label: ".", led: 58, row: 3, col: 9.0, width: 1.0 },
    KeyDef { id: "slash", label: "/", led: 64, row: 3, col: 10.0, width: 1.0 },
    KeyDef { id: "rshift", label: "Shift", led: 82, row: 3, col: 12.0, width: 1.75 },
    KeyDef { id: "up", label: "↑", led: 88, row: 3, col: 13.0, width: 1.0 },
    KeyDef { id: "pgdn", label: "PgDn", led: 94, row: 3, col: 14.0, width: 1.0 },
    KeyDef { id: "lctrl", label: "Ctrl", led: 5, row: 4, col: 0.0, width: 1.25 },
    KeyDef { id: "lwin", label: "Win", led: 11, row: 4, col: 1.0, width: 1.25 },
    KeyDef { id: "lalt", label: "Alt", led: 17, row: 4, col: 2.0, width: 1.25 },
    KeyDef { id: "space", label: "", led: 35, row: 4, col: 6.0, width: 6.25 },
    KeyDef { id: "ralt", label: "Alt", led: 53, row: 4, col: 9.0, width: 1.25 },
    KeyDef { id: "fn", label: "Fn", led: 59, row: 4, col: 10.0, width: 1.25 },
    KeyDef { id: "rctrl", label: "Ctrl", led: 65, row: 4, col: 11.0, width: 1.25 },
    KeyDef { id: "left", label: "←", led: 83, row: 4, col: 12.0, width: 1.0 },
    KeyDef { id: "down", label: "↓", led: 89, row: 4, col: 13.0, width: 1.0 },
    KeyDef { id: "right", label: "→", led: 95, row: 4, col: 14.0, width: 1.0 },
];

pub fn key_by_id(id: &str) -> Option<&'static KeyDef> {
    KEYS.iter().find(|key| key.id == id)
}

/// Left edge and width in key units, on a 16-unit grid.
/// Wide keys include the gaps they span, so the nav column lines up.
pub fn visual_span(id: &str, width: f32) -> (f32, f32) {
    let mut cursor = [0.0_f32; 5];
    let mut found = (0.0, width);
    for key in KEYS {
        let row = key.row as usize;
        let span = match key.id {
            "ralt" | "fn" | "rctrl" => 1.0,
            _ => key.width,
        };
        if key.id == id {
            found = (cursor[row], span);
            break;
        }
        cursor[row] += span;
    }
    found
}
