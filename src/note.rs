//! Raw note events triggered by an `Event`.

use std::{
    fmt::Display,
    mem,
    ops::{Add, Sub},
};

// -------------------------------------------------------------------------------------------------

pub mod chord;
pub mod scale;

// -------------------------------------------------------------------------------------------------

/// A note representable in a 7 bit unsigned int.
///
/// The subscript 'S' to a note means sharp. Because it only uses the least significant 7 bits,
/// any value can be interpreted as either an i8 or a u8 for free.
///
/// Note implements `From\<u8\>`, `Into\<u8\>` as well as `From\<i8\>`, `Into\<i8\>` and
/// `From\<&str\>` so the enum values usually can be completely ignored.
///
/// In `From<&str>` conversions, the following notation is supported:
/// `C4` (plain), `C#1` (sharps), `Db1` (flats),
/// `D_2` (using _ as separator), `G 5` (using space as separator)
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
#[allow(non_camel_case_types)]
pub enum Note {
    C0 = 0x00,
    Cs0 = 0x01,
    D0 = 0x02,
    Ds0 = 0x03,
    E0 = 0x04,
    F0 = 0x05,
    Fs0 = 0x06,
    G0 = 0x07,
    Gs0 = 0x08,
    A0 = 0x09,
    As0 = 0x0A,
    B0 = 0x0B,
    C1 = 0x0C,
    Cs1 = 0x0D,
    D1 = 0x0E,
    Ds1 = 0x0F,
    E1 = 0x10,
    F1 = 0x11,
    Fs1 = 0x12,
    G1 = 0x13,
    Gs1 = 0x14,
    A1 = 0x15,
    As1 = 0x16,
    B1 = 0x17,
    C2 = 0x18,
    Cs2 = 0x19,
    D2 = 0x1A,
    Ds2 = 0x1B,
    E2 = 0x1C,
    F2 = 0x1D,
    Fs2 = 0x1E,
    G2 = 0x1F,
    Gs2 = 0x20,
    A2 = 0x21,
    As2 = 0x22,
    B2 = 0x23,
    C3 = 0x24,
    Cs3 = 0x25,
    D3 = 0x26,
    Ds3 = 0x27,
    E3 = 0x28,
    F3 = 0x29,
    Fs3 = 0x2A,
    G3 = 0x2B,
    Gs3 = 0x2C,
    A3 = 0x2D,
    As3 = 0x2E,
    B3 = 0x2F,
    C4 = 0x30,
    Cs4 = 0x31,
    D4 = 0x32,
    Ds4 = 0x33,
    E4 = 0x34,
    F4 = 0x35,
    Fs4 = 0x36,
    G4 = 0x37,
    Gs4 = 0x38,
    A4 = 0x39,
    As4 = 0x3A,
    B4 = 0x3B,
    C5 = 0x3C,
    Cs5 = 0x3D,
    D5 = 0x3E,
    Ds5 = 0x3F,
    E5 = 0x40,
    F5 = 0x41,
    Fs5 = 0x42,
    G5 = 0x43,
    Gs5 = 0x44,
    A5 = 0x45,
    As5 = 0x46,
    B5 = 0x47,
    C6 = 0x48,
    Cs6 = 0x49,
    D6 = 0x4A,
    Ds6 = 0x4B,
    E6 = 0x4C,
    F6 = 0x4D,
    Fs6 = 0x4E,
    G6 = 0x4F,
    Gs6 = 0x50,
    A6 = 0x51,
    As6 = 0x52,
    B6 = 0x53,
    C7 = 0x54,
    Cs7 = 0x55,
    D7 = 0x56,
    Ds7 = 0x57,
    E7 = 0x58,
    F7 = 0x59,
    Fs7 = 0x5A,
    G7 = 0x5B,
    Gs7 = 0x5C,
    A7 = 0x5D,
    As7 = 0x5E,
    B7 = 0x5F,
    C8 = 0x60,
    Cs8 = 0x61,
    D8 = 0x62,
    Ds8 = 0x63,
    E8 = 0x64,
    F8 = 0x65,
    Fs8 = 0x66,
    G8 = 0x67,
    Gs8 = 0x68,
    A8 = 0x69,
    As8 = 0x6A,
    B8 = 0x6B,
    C9 = 0x6C,
    Cs9 = 0x6D,
    D9 = 0x6E,
    Ds9 = 0x6F,
    E9 = 0x70,
    F9 = 0x71,
    Fs9 = 0x72,
    G9 = 0x73,
    Gs9 = 0x74,
    A9 = 0x75,
    As9 = 0x76,
    B9 = 0x77,
    C10 = 0x78,
    Cs10 = 0x79,
    D10 = 0x7A,
    Ds10 = 0x7B,
    E10 = 0x7C,
    F10 = 0x7D,
    Fs10 = 0x7E,
    G10 = 0x7F,
    // Following notes are NOT valid MIDI notes, but only internally used
    EMPTY = 0xFE,
    OFF = 0xFF,
}

impl Note {
    /// returns if this note value is a note-on.
    pub fn is_note_on(&self) -> bool {
        *self != Note::OFF && *self != Note::EMPTY
    }
    /// returns if this note value is a note-off.
    pub fn is_note_off(&self) -> bool {
        *self == Note::OFF
    }

    /// Get root key of the note as number: 0 = C, 1 = C# ...
    /// Panics when the note is not a note on value.
    pub fn key(&self) -> u8 {
        assert!(self.is_note_on());
        *self as u8 % 12
    }
    /// Get note's octave value.
    /// Panics when the note is not a note on value.
    pub fn octave(&self) -> u8 {
        assert!(self.is_note_on());
        *self as u8 / 12
    }

    /// return a new transposed note with the given offset.
    #[must_use]
    pub fn transposed(&self, offset: i32) -> Self {
        match self {
            Self::OFF | Self::EMPTY => *self,
            _ => Note::from((*self as i32 + offset).clamp(0, 0x7f) as u8),
        }
    }
}

impl TryFrom<&str> for Note {
    type Error = String;

    /// Try converting the given string to a Note value
    fn try_from(s: &str) -> Result<Self, String> {
        fn is_empty_note(s: &str) -> bool {
            s.is_empty() || s.trim_matches(|c| c == '-').is_empty()
        }

        fn is_note_off(s: &str) -> bool {
            s.eq_ignore_ascii_case("off") || s == "~"
        }

        fn is_sharp_symbol(s: &str, index: usize) -> bool {
            if let Some(c) = s.chars().nth(index) {
                if c == 'S' || c == 's' || c == '#' || c == '♮' {
                    return true;
                }
            }
            false
        }
        fn is_white_space_symbol(s: &str, index: usize) -> bool {
            if let Some(c) = s.chars().nth(index) {
                if c == ' ' || c == '\t' {
                    return true;
                }
            }
            false
        }
        fn is_flat_symbol(s: &str, index: usize) -> bool {
            if let Some(c) = s.chars().nth(index) {
                if c == 'B' || c == 'b' || c == '♭' {
                    return true;
                }
            }
            false
        }
        fn is_empty_symbol(s: &str, index: usize) -> bool {
            if let Some(c) = s.chars().nth(index) {
                if c == ' ' || c == '_' || c == '-' {
                    return true;
                }
            }
            false
        }
        fn octave_value_at(s: &str, index: usize) -> Result<i32, String> {
            let str = &s[index..];
            str.parse::<i32>()
                .map_err(|e| format!("invalid note str '{}': {}", s, e))
        }
        fn note_value_at(s: &str, index: usize) -> Result<i8, String> {
            if let Some(c) = s.chars().nth(index) {
                let note = match c {
                    'c' | 'C' => 0,
                    'd' | 'D' => 2,
                    'e' | 'E' => 4,
                    'f' | 'F' => 5,
                    'g' | 'G' => 7,
                    'a' | 'A' => 9,
                    'b' | 'B' => 11,
                    _ => {
                        return Err(format!(
                            "invalid note str '{}' - note character '{}' is invalid.",
                            s, c
                        ))
                    }
                };
                if is_sharp_symbol(s, 1) {
                    Ok(note + 1)
                } else if is_flat_symbol(s, 1) {
                    Ok(note - 1)
                } else {
                    Ok(note)
                }
            } else {
                Err(format!("invalid note str '{}' - string is too short.", s))
            }
        }

        // Empty Note
        if is_empty_note(s) {
            return Ok(Note::EMPTY);
        }

        // Note-Off
        if is_note_off(s) {
            return Ok(Note::OFF);
        }

        // Note-On
        let note = note_value_at(s, 0)? as i32;
        let octave = if is_sharp_symbol(s, 1) || is_flat_symbol(s, 1) || is_empty_symbol(s, 1) {
            if s.len() > 2 && !is_white_space_symbol(s, 2) {
                octave_value_at(s, 2)?
            } else {
                4
            }
        } else if s.len() > 1 && !is_white_space_symbol(s, 1) {
            octave_value_at(s, 1)?
        } else {
            4
        };
        if !(0..=10).contains(&octave) {
            return Err(format!(
                "invalid note str '{}' - octave '{}' is out of range.",
                s, octave
            ));
        }
        Ok(Self::from((octave * 12 + note) as u8))
    }
}

impl From<u8> for Note {
    fn from(n: u8) -> Note {
        match n {
            0xFE => Self::EMPTY,
            0xFF => Self::OFF,
            _ => unsafe { mem::transmute::<u8, Self>(n & 0x7f) },
        }
    }
}

impl From<Note> for u8 {
    fn from(note: Note) -> u8 {
        note as u8
    }
}

impl From<i8> for Note {
    fn from(n: i8) -> Note {
        unsafe { mem::transmute(n & 0x7f) }
    }
}

impl From<Note> for i8 {
    fn from(note: Note) -> Self {
        note as i8
    }
}

impl Add<u8> for Note {
    type Output = Self;
    fn add(self, rhs: u8) -> Self {
        Note::from(((self as u8) as i32 + rhs as i32).min(0x7f) as u8)
    }
}

impl Sub<u8> for Note {
    type Output = Self;
    fn sub(self, rhs: u8) -> Self {
        Note::from(((self as u8) as i32 - rhs as i32).max(0) as u8)
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const NOTE_NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        match self {
            Self::EMPTY => write!(f, "---"),
            Self::OFF => write!(f, "OFF"),
            _ => {
                let octave = (*self as u8 / 12) as i32;
                let note = (*self as u8 % 12) as usize;
                write!(f, "{}{}", NOTE_NAMES[note], octave)
            }
        }
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::Note;

    #[test]
    fn note_number_conversion() {
        assert_eq!(Note::from(0x0_u8), Note::C0);
        assert_eq!(Note::from(0x24_u8), Note::C3);
        assert_eq!(Note::from(0x80_u8), Note::C0); // wraps, probably should not be allowed
        assert_eq!(Note::from(0xFE_u8), Note::EMPTY);
        assert_eq!(Note::from(-1i32 as u8), Note::OFF);
        assert_eq!(Note::from(0xFF_u8), Note::OFF);
        assert_eq!(i8::from(Note::C4), 48);
        assert_eq!(u8::from(Note::C3), 0x24);
        assert_eq!(u8::from(Note::EMPTY), 0xFE);
        assert_eq!(u8::from(Note::OFF), 0xFF);
    }

    #[test]
    fn note_serialization() {
        assert_eq!(Note::C4.to_string(), "C4");
        assert_eq!(Note::Cs0.to_string(), "C#0");
        assert_eq!(Note::G9.to_string(), "G9");
        assert_eq!(Note::Fs10.to_string(), "F#10");
        assert_eq!(Note::OFF.to_string(), "OFF");
    }

    #[test]
    fn note_deserialization() -> Result<(), String> {
        assert!(Note::try_from("x4").is_err());
        assert!(Note::try_from("c.2").is_err());
        assert!(Note::try_from("cc2").is_err());
        assert!(Note::try_from("cbb2").is_err());
        assert!(Note::try_from("c##2").is_err());

        assert_eq!(Note::try_from("C4")?, Note::C4);
        assert_eq!(Note::try_from("Cb4")?, Note::B3);
        assert_eq!(Note::try_from("C#3")?, Note::Cs3);
        assert_eq!(Note::try_from("D#10")?, Note::Ds10);
        assert_eq!(Note::try_from("E_7")?, Note::E7);
        assert_eq!(Note::try_from("f5")?, Note::F5);
        assert_eq!(Note::try_from("g 9")?, Note::G9);
        assert_eq!(Note::try_from("A 8")?, Note::A8);
        assert_eq!(Note::try_from("bb2")?, Note::As2);

        assert_eq!(Note::try_from("OFF")?, Note::OFF);
        assert_eq!(Note::try_from("off")?, Note::OFF);
        assert_eq!(Note::try_from("~")?, Note::OFF);

        assert_eq!(Note::try_from("")?, Note::EMPTY);
        assert_eq!(Note::try_from("-")?, Note::EMPTY);
        assert_eq!(Note::try_from("---")?, Note::EMPTY);
        Ok(())
    }
}
