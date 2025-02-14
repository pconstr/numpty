#[derive(Debug, PartialEq)]
pub enum InputSeq {
    Standard(String),
    Cursor(String, String),
}

pub fn seqs_to_bytes(seqs: &[InputSeq], app_mode: bool) -> Vec<u8> {
    let mut bytes = Vec::new();

    for seq in seqs {
        bytes.extend_from_slice(seq_as_bytes(seq, app_mode));
    }

    bytes
}

fn seq_as_bytes(seq: &InputSeq, app_mode: bool) -> &[u8] {
    match (seq, app_mode) {
        (InputSeq::Standard(seq), _) => seq.as_bytes(),
        (InputSeq::Cursor(seq1, _seq2), false) => seq1.as_bytes(),
        (InputSeq::Cursor(_seq1, seq2), true) => seq2.as_bytes(),
    }
}

fn standard_key<S: ToString>(seq: S) -> InputSeq {
    InputSeq::Standard(seq.to_string())
}

fn cursor_key<S: ToString>(seq1: S, seq2: S) -> InputSeq {
    InputSeq::Cursor(seq1.to_string(), seq2.to_string())
}

pub fn parse_key(key: String) -> InputSeq {
    let seq = match key.as_str() {
        "C-@" | "C-Space" | "^@" => "\x00",
        "C-[" | "Escape" | "^[" => "\x1b",
        "C-\\" | "^\\" => "\x1c",
        "C-]" | "^]" => "\x1d",
        "C-^" | "C-/" => "\x1e",
        "C--" | "C-_" => "\x1f",
        "Tab" => "\x09",   // same as C-i
        "Enter" => "\x0d", // same as C-m
        "Space" => " ",
        "Left" => return cursor_key("\x1b[D", "\x1bOD"),
        "Right" => return cursor_key("\x1b[C", "\x1bOC"),
        "Up" => return cursor_key("\x1b[A", "\x1bOA"),
        "Down" => return cursor_key("\x1b[B", "\x1bOB"),
        "C-Left" => "\x1b[1;5D",
        "C-Right" => "\x1b[1;5C",
        "S-Left" => "\x1b[1;2D",
        "S-Right" => "\x1b[1;2C",
        "C-Up" => "\x1b[1;5A",
        "C-Down" => "\x1b[1;5B",
        "S-Up" => "\x1b[1;2A",
        "S-Down" => "\x1b[1;2B",
        "A-Left" => "\x1b[1;3D",
        "A-Right" => "\x1b[1;3C",
        "A-Up" => "\x1b[1;3A",
        "A-Down" => "\x1b[1;3B",
        "C-S-Left" | "S-C-Left" => "\x1b[1;6D",
        "C-S-Right" | "S-C-Right" => "\x1b[1;6C",
        "C-S-Up" | "S-C-Up" => "\x1b[1;6A",
        "C-S-Down" | "S-C-Down" => "\x1b[1;6B",
        "C-A-Left" | "A-C-Left" => "\x1b[1;7D",
        "C-A-Right" | "A-C-Right" => "\x1b[1;7C",
        "C-A-Up" | "A-C-Up" => "\x1b[1;7A",
        "C-A-Down" | "A-C-Down" => "\x1b[1;7B",
        "A-S-Left" | "S-A-Left" => "\x1b[1;4D",
        "A-S-Right" | "S-A-Right" => "\x1b[1;4C",
        "A-S-Up" | "S-A-Up" => "\x1b[1;4A",
        "A-S-Down" | "S-A-Down" => "\x1b[1;4B",
        "C-A-S-Left" | "C-S-A-Left" | "A-C-S-Left" | "S-C-A-Left" | "A-S-C-Left" | "S-A-C-Left" => {
            "\x1b[1;8D"
        }
        "C-A-S-Right" | "C-S-A-Right" | "A-C-S-Right" | "S-C-A-Right" | "A-S-C-Right"
        | "S-A-C-Right" => "\x1b[1;8C",
        "C-A-S-Up" | "C-S-A-Up" | "A-C-S-Up" | "S-C-A-Up" | "A-S-C-Up" | "S-A-C-Up" => "\x1b[1;8A",
        "C-A-S-Down" | "C-S-A-Down" | "A-C-S-Down" | "S-C-A-Down" | "A-S-C-Down" | "S-A-C-Down" => {
            "\x1b[1;8B"
        }
        "F1" => "\x1bOP",
        "F2" => "\x1bOQ",
        "F3" => "\x1bOR",
        "F4" => "\x1bOS",
        "F5" => "\x1b[15~",
        "F6" => "\x1b[17~",
        "F7" => "\x1b[18~",
        "F8" => "\x1b[19~",
        "F9" => "\x1b[20~",
        "F10" => "\x1b[21~",
        "F11" => "\x1b[23~",
        "F12" => "\x1b[24~",
        "C-F1" => "\x1b[1;5P",
        "C-F2" => "\x1b[1;5Q",
        "C-F3" => "\x1b[1;5R",
        "C-F4" => "\x1b[1;5S",
        "C-F5" => "\x1b[15;5~",
        "C-F6" => "\x1b[17;5~",
        "C-F7" => "\x1b[18;5~",
        "C-F8" => "\x1b[19;5~",
        "C-F9" => "\x1b[20;5~",
        "C-F10" => "\x1b[21;5~",
        "C-F11" => "\x1b[23;5~",
        "C-F12" => "\x1b[24;5~",
        "S-F1" => "\x1b[1;2P",
        "S-F2" => "\x1b[1;2Q",
        "S-F3" => "\x1b[1;2R",
        "S-F4" => "\x1b[1;2S",
        "S-F5" => "\x1b[15;2~",
        "S-F6" => "\x1b[17;2~",
        "S-F7" => "\x1b[18;2~",
        "S-F8" => "\x1b[19;2~",
        "S-F9" => "\x1b[20;2~",
        "S-F10" => "\x1b[21;2~",
        "S-F11" => "\x1b[23;2~",
        "S-F12" => "\x1b[24;2~",
        "A-F1" => "\x1b[1;3P",
        "A-F2" => "\x1b[1;3Q",
        "A-F3" => "\x1b[1;3R",
        "A-F4" => "\x1b[1;3S",
        "A-F5" => "\x1b[15;3~",
        "A-F6" => "\x1b[17;3~",
        "A-F7" => "\x1b[18;3~",
        "A-F8" => "\x1b[19;3~",
        "A-F9" => "\x1b[20;3~",
        "A-F10" => "\x1b[21;3~",
        "A-F11" => "\x1b[23;3~",
        "A-F12" => "\x1b[24;3~",
        "Home" => return cursor_key("\x1b[H", "\x1bOH"),
        "C-Home" => "\x1b[1;5H",
        "S-Home" => "\x1b[1;2H",
        "A-Home" => "\x1b[1;3H",
        "End" => return cursor_key("\x1b[F", "\x1bOF"),
        "C-End" => "\x1b[1;5F",
        "S-End" => "\x1b[1;2F",
        "A-End" => "\x1b[1;3F",
        "PageUp" => "\x1b[5~",
        "C-PageUp" => "\x1b[5;5~",
        "S-PageUp" => "\x1b[5;2~",
        "A-PageUp" => "\x1b[5;3~",
        "PageDown" => "\x1b[6~",
        "C-PageDown" => "\x1b[6;5~",
        "S-PageDown" => "\x1b[6;2~",
        "A-PageDown" => "\x1b[6;3~",

        k => {
            let chars: Vec<char> = k.chars().collect();

            match chars.as_slice() {
                ['C', '-', k @ 'a'..='z'] => {
                    return standard_key((*k as u8 - 0x60) as char);
                }

                ['C', '-', k @ 'A'..='Z'] => {
                    return standard_key((*k as u8 - 0x40) as char);
                }

                ['^', k @ 'a'..='z'] => {
                    return standard_key((*k as u8 - 0x60) as char);
                }

                ['^', k @ 'A'..='Z'] => {
                    return standard_key((*k as u8 - 0x40) as char);
                }

                ['A', '-', k] => {
                    return standard_key(format!("\x1b{}", k));
                }

                _ => &key,
            }
        }
    };

    standard_key(seq)
}
