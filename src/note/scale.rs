//! Musical scales based on `Note` and custom intervals or common scale names.

use crate::Note;

// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Mode {
    name: &'static str,
    alt_names: &'static str,
    degrees: [usize; 12],
}

impl TryFrom<&str> for Mode {
    type Error = String;

    /// Try converting the given string to a known scale.
    fn try_from(mode: &str) -> Result<Self, String> {
        let norm_scale = Mode::resolve_synonyms(mode)
            .to_ascii_lowercase()
            .trim()
            .to_string();
        SCALE_MODES
            .iter()
            .find(|v| {
                v.name.eq_ignore_ascii_case(&norm_scale)
                    || v.alt_names
                        .split(';')
                        .any(|v| v.eq_ignore_ascii_case(&norm_scale))
            })
            .cloned()
            .ok_or_else(|| "Unknown scale".to_string())
    }
}

impl TryFrom<&Vec<i32>> for Mode {
    type Error = String;

    /// Try converting the given interval list to a custom scale.
    fn try_from(intervals: &Vec<i32>) -> Result<Self, String> {
        if intervals.is_empty() {
            Err("interval list can not be empty".to_string())
        } else if intervals.len() > 11 {
            Err("interval list may contain up to 11 elements only".to_string())
        } else if intervals.windows(2).any(|f| f[0] > f[1]) {
            Err("interval list must be sorted in ascending order".to_string())
        } else {
            let mut degrees = [0; 12];
            for (degree_count, i) in intervals.iter().enumerate() {
                if !(0..12).contains(i) {
                    return Err(format!(
                        "intervals must be in range [0..12] but one is '{}'",
                        i
                    ));
                }
                degrees[*i as usize] = degree_count + 1;
            }
            Ok(Self {
                name: "custom scale",
                alt_names: "",
                degrees,
            })
        }
    }
}

impl Mode {
    fn steps(&self) -> Vec<usize> {
        self.degrees
            .iter()
            .copied()
            .enumerate()
            .filter(|(_s, d)| *d != 0)
            .map(|(s, _d)| s)
            .collect()
    }

    fn resolve_synonyms(scale: &str) -> String {
        scale
            .split(' ')
            .filter(|v| !v.is_empty())
            .map(|v| match v.to_ascii_lowercase().as_str() {
                "8-tone" => "eight-tone",
                "9-tone" => "nine-tone",
                "aug" => "augmented",
                "dim" => "diminished",
                "dom" => "Dominant",
                "egypt" => "egyptian",
                "harm" => "harmonic",
                "hungary" => "hungarian",
                "roman" => "romanian",
                "min" => "minor",
                "maj" => "major",
                "nat" => "natural",
                "penta" => "pentatonic",
                "span" => "spanish",
                _ => v,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

const SCALE_MODES: [Mode; 36] = [
    Mode {
        name: "chromatic",
        alt_names: "all",
        degrees: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    },
    Mode {
        name: "natural major",
        alt_names: "major;ionian",
        degrees: [1, 0, 2, 0, 3, 4, 0, 5, 0, 6, 0, 7],
        // [0,2,4,5,7,9,10]
    },
    Mode {
        name: "natural minor",
        alt_names: "minor;aeolian",
        degrees: [1, 0, 2, 3, 0, 4, 0, 5, 6, 0, 7, 0],
    },
    Mode {
        name: "pentatonic major",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 0, 0, 4, 0, 5, 0, 0],
    },
    Mode {
        name: "pentatonic minor",
        alt_names: "",
        degrees: [1, 0, 0, 2, 0, 3, 0, 4, 0, 0, 5, 0],
    },
    Mode {
        name: "pentatonic egyptian",
        alt_names: "",
        degrees: [1, 0, 2, 0, 0, 3, 0, 4, 0, 0, 5, 0],
    },
    Mode {
        name: "blues major",
        alt_names: "",
        degrees: [1, 0, 2, 3, 4, 0, 0, 5, 0, 6, 0, 0],
    },
    Mode {
        name: "blues minor",
        alt_names: "",
        degrees: [1, 0, 0, 2, 0, 3, 4, 5, 0, 0, 6, 0],
    },
    Mode {
        name: "whole tone",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0],
    },
    Mode {
        name: "augmented",
        alt_names: "",
        degrees: [1, 0, 0, 2, 3, 0, 0, 4, 5, 0, 0, 6],
    },
    Mode {
        name: "prometheus",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 0, 4, 0, 0, 5, 6, 0],
    },
    Mode {
        name: "tritone",
        alt_names: "",
        degrees: [1, 2, 0, 0, 3, 0, 4, 5, 0, 0, 6, 0],
    },
    Mode {
        name: "harmonic major",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 4, 0, 5, 6, 0, 0, 7],
    },
    Mode {
        name: "harmonic minor",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 4, 0, 5, 6, 0, 0, 7],
    },
    Mode {
        name: "melodic minor",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 4, 0, 5, 0, 6, 0, 7],
    },
    Mode {
        name: "all minor",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 4, 0, 5, 6, 6, 7, 7],
    },
    Mode {
        name: "dorian",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 4, 0, 5, 0, 6, 7, 0],
    },
    Mode {
        name: "phrygian",
        alt_names: "",
        degrees: [1, 2, 0, 3, 0, 4, 0, 5, 6, 0, 7, 0],
    },
    Mode {
        name: "phrygian dominant",
        alt_names: "",
        degrees: [1, 2, 0, 0, 3, 4, 0, 5, 6, 0, 7, 0],
    },
    Mode {
        name: "lydian",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 0, 4, 5, 0, 6, 0, 7],
    },
    Mode {
        name: "lydian augmented",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 0, 4, 0, 5, 6, 0, 7],
    },
    Mode {
        name: "mixolydian",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 4, 0, 5, 0, 6, 7, 0],
    },
    Mode {
        name: "locrian",
        alt_names: "",
        degrees: [1, 2, 0, 3, 0, 4, 5, 0, 6, 0, 7, 0],
    },
    Mode {
        name: "locrian major",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 4, 5, 0, 6, 0, 7, 0],
    },
    Mode {
        name: "super locrian",
        alt_names: "",
        degrees: [1, 2, 0, 3, 4, 0, 5, 0, 6, 0, 7, 0],
    },
    Mode {
        name: "neapolitan major",
        alt_names: "",
        degrees: [1, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7],
    },
    Mode {
        name: "neapolitan minor",
        alt_names: "",
        degrees: [1, 2, 0, 3, 0, 4, 0, 5, 6, 0, 0, 7],
    },
    Mode {
        name: "romanian minor",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 0, 4, 5, 0, 6, 7, 0],
    },
    Mode {
        name: "spanish gypsy",
        alt_names: "",
        degrees: [1, 2, 0, 0, 3, 4, 0, 5, 6, 0, 0, 7],
    },
    Mode {
        name: "hungarian gypsy",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 0, 4, 5, 6, 0, 0, 7],
    },
    Mode {
        name: "enigmatic",
        alt_names: "",
        degrees: [1, 2, 0, 0, 3, 0, 4, 0, 5, 0, 6, 7],
    },
    Mode {
        name: "overtone",
        alt_names: "",
        degrees: [1, 0, 2, 0, 3, 0, 4, 5, 0, 6, 7, 0],
    },
    Mode {
        name: "diminished half",
        alt_names: "",
        degrees: [1, 2, 0, 3, 4, 0, 5, 6, 0, 7, 8, 0],
    },
    Mode {
        name: "diminished whole",
        alt_names: "",
        degrees: [1, 0, 2, 3, 0, 4, 5, 0, 6, 7, 0, 8],
    },
    Mode {
        name: "spanish eight-tone",
        alt_names: "eight-tone",
        degrees: [1, 2, 0, 3, 4, 5, 6, 0, 7, 0, 8, 0],
    },
    Mode {
        name: "nine-tone",
        alt_names: "",
        degrees: [1, 0, 2, 3, 4, 0, 5, 6, 7, 8, 0, 9],
    },
];

// -------------------------------------------------------------------------------------------------

#[repr(u8)]
#[derive(PartialEq, Copy, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum TransposeStrictness {
    ForceSecondaryInScaleNotesWhenRootInScale = 0,
    ForceSecondaryInScaleNotes = 1,
    ForceAllInScaleNotes = 2,
    ForceAllNotes = 3,
}

// -------------------------------------------------------------------------------------------------

/// Note iterator for notes in a [`Scale`].
#[derive(Debug, Clone)]
pub struct NoteScaleIterator {
    root: u8,
    octave: u8,
    steps: Vec<usize>,
    step_index: usize,
}

impl NoteScaleIterator {
    fn new(root: u8, octave: u8, steps: Vec<usize>) -> Self {
        let step_index = 0;
        Self {
            root,
            octave,
            steps,
            step_index,
        }
    }
}

impl Iterator for NoteScaleIterator {
    type Item = Note;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.root as usize + 12 * self.octave as usize + self.steps[self.step_index];
        self.step_index += 1;
        if self.step_index >= self.steps.len() {
            self.octave += 1;
            self.step_index = 0;
        }
        if key <= 127 {
            Some(Note::from(key as u8))
        } else {
            None
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// A musical scale / mode.
#[derive(Debug, Clone)]
pub struct Scale {
    key: u8,    // 0..12
    octave: u8, // 0..10
    mode: Mode,
}

impl TryFrom<(Note, &str)> for Scale {
    type Error = String;

    fn try_from((note, mode): (Note, &str)) -> Result<Self, String> {
        Ok(Self {
            key: note.key(),
            octave: note.octave(),
            mode: Mode::try_from(mode)?,
        })
    }
}

impl TryFrom<(Note, &Vec<i32>)> for Scale {
    type Error = String;

    fn try_from((note, intervals): (Note, &Vec<i32>)) -> Result<Self, String> {
        Ok(Self {
            key: note.key(),
            octave: note.octave(),
            mode: Mode::try_from(intervals)?,
        })
    }
}

impl Scale {
    /// Known mode/scaling names.
    pub fn mode_names() -> Vec<&'static str> {
        SCALE_MODES.iter().map(|mode| mode.name).collect()
    }

    #[allow(dead_code)] // used in tests only
    fn new(note: Note, mode: Mode) -> Self {
        let key = note.key();
        let octave = note.octave();
        Self { key, octave, mode }
    }

    /// Generate a chord from a given degree with the given number of notes.
    /// `degree` must be in range `[1..=7]`.
    /// `count` must be in range `[1..=5]`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use pattrns::{Note, Scale};
    /// let scale = Scale::try_from((Note::C4, "major")).unwrap();
    /// let cmaj = scale.chord_from_degree(1, 3);
    /// let gmaj7 = scale.chord_from_degree(5, 4);
    /// ```
    ///
    /// ### Panics
    ///
    /// Panics if `degree` or `count` is out of range.
    pub fn chord_from_degree(&self, degree: usize, count: usize) -> Vec<Note> {
        assert!((1..=7).contains(&degree));
        assert!((1..=5).contains(&count));
        self.notes_iter()
            .skip(degree - 1)
            .enumerate()
            .filter(|(index, _)| index % 2 == 0)
            .take(count)
            .map(|(_, note)| note)
            .collect()
    }

    /// Key note as number [0..12].
    pub fn key(&self) -> u8 {
        self.key
    }

    /// List of raw degrees where 0 indicates no step.
    pub fn degrees(&self) -> Vec<usize> {
        self.mode.degrees.to_vec()
    }

    /// List of steps / intervals.
    pub fn steps(&self) -> Vec<usize> {
        self.mode.steps()
    }

    /// Iterator with ascending list of notes in the scale
    pub fn notes_iter(&self) -> NoteScaleIterator {
        NoteScaleIterator::new(self.key, self.octave, self.steps())
    }

    /// Generate an ascending list of notes in the scale, using the Note passed in the
    /// constructor as root note. Note that unlike `note_iter`, this will clamp notes
    /// outside of the valid range to 0x7F
    pub fn notes(&self) -> Vec<Note> {
        self.steps()
            .into_iter()
            .map(|d| d + (self.key as usize) + (12 * self.octave as usize))
            .map(|n| Note::from(n.min(0x7f) as u8))
            .collect()
    }

    /// Transpose the given note into this scale, using the most strict strictness level.
    pub fn transpose(&self, note: Note, offset: i32) -> Note {
        self.transpose_with_strictness(note, offset, TransposeStrictness::ForceAllNotes)
    }

    /// Transpose the given note into the scale, using the specified strictness level.
    pub fn transpose_with_strictness(
        &self,
        note: Note,
        offset: i32,
        strictness: TransposeStrictness,
    ) -> Note {
        let mut transposed_note = note as i32 + offset;
        let transposed_scale_step = self.transposed_note_to_step(transposed_note);
        if self.mode.degrees[transposed_scale_step] == 0 {
            let original_note_scale_step = self.transposed_note_to_step(note as i32);
            let original_note_degree = self.mode.degrees[original_note_scale_step];
            let original_note_in_scale = original_note_degree != 0;
            let original_is_root = original_note_scale_step == 0;
            if !original_note_in_scale {
                if strictness == TransposeStrictness::ForceAllNotes {
                    transposed_note = self.quantize_note(transposed_note);
                }
            } else if original_is_root {
                if strictness == TransposeStrictness::ForceAllNotes
                    || strictness == TransposeStrictness::ForceAllInScaleNotes
                {
                    transposed_note = self.quantize_note(transposed_note);
                }
            } else {
                let transposed_root_degree =
                    self.mode.degrees[self.transposed_note_to_step(self.key as i32 + offset)];
                let transposed_root_in_scale = transposed_root_degree != 0;
                if transposed_root_in_scale
                    || strictness != TransposeStrictness::ForceSecondaryInScaleNotesWhenRootInScale
                {
                    transposed_note = self.transpose_impl(
                        note,
                        original_note_scale_step,
                        original_note_degree,
                        offset,
                    );
                }
            }
        }
        Note::from(transposed_note.clamp(0, 0x7F) as u8)
    }

    fn degree_to_step(&self, degree: usize) -> usize {
        assert!((1..=12).contains(&degree), "Degree out of bounds");
        if let Some(position) = self.mode.degrees.iter().position(|d| *d == degree) {
            return position;
        }
        unreachable!("Invalid degree value in scale.")
    }

    fn transposed_note_to_step(&self, note: i32) -> usize {
        let offset = note - self.key as i32;
        let mut offset = offset;
        while offset < 0 {
            offset += 12;
        }
        (offset % 12) as usize
    }

    fn quantize_note(&self, note: i32) -> i32 {
        self.quantize_offset(note - self.key as i32) + self.key as i32
    }

    fn quantize_offset(&self, offset: i32) -> i32 {
        let step = self.transposed_note_to_step(self.key as i32 + offset);
        let mut qstep = step;
        while self.mode.degrees[qstep] == 0 {
            qstep = (qstep + 12 - 1) % 12;
        }
        offset - step as i32 + qstep as i32
    }

    fn transpose_impl(&self, note: Note, scale_step: usize, degree: usize, offset: i32) -> i32 {
        assert!(
            degree != 0,
            "Original must have been in-scale to transpose."
        );
        let quantized_offset = self.quantize_offset(offset);

        // count distance root moved in degrees
        let num_degrees = self.mode.steps().len();
        let mut degree_diff = 0;
        if quantized_offset < 0 {
            let whole_octaves = -quantized_offset / 12;
            degree_diff -= whole_octaves * num_degrees as i32;
            let mut remainder = -quantized_offset % 12;
            for i in (0..12).rev() {
                if self.mode.degrees[i] != 0 {
                    remainder -= 1;
                    if remainder > 0 {
                        degree_diff -= 1;
                    }
                }
            }
        } else {
            let whole_octaves = quantized_offset / 12;
            degree_diff += whole_octaves * num_degrees as i32;
            let mut remainder = quantized_offset % 12;
            for degree in &self.mode.degrees[1..] {
                if *degree != 0 {
                    remainder -= 1;
                    if remainder > 0 {
                        degree_diff += 1;
                    }
                }
            }
        }

        // add the degree_diff to the original note's degree and convert to scale step
        // to find the transposed note
        let mut transposed_degree = degree as i32 + degree_diff;

        // keep track of octaves
        let mut octave_offset = 0;
        while transposed_degree > num_degrees as i32 {
            transposed_degree -= num_degrees as i32;
            octave_offset += 1;
        }
        while transposed_degree < 1 {
            transposed_degree += num_degrees as i32;
            octave_offset -= 1;
        }

        let transposed_note_scale_step = self.degree_to_step(transposed_degree as usize);
        note as i32 - scale_step as i32 + transposed_note_scale_step as i32 + octave_offset * 12
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scale_from_string() {
        // debug: print all scale names
        /*
        let names = Scale::mode_names()
            .into_iter()
            .map(|e| format!("\"{}\"", e))
            .collect::<Vec<_>>()
            .join("|");
        println!("{}", names);
        */

        assert!(Mode::try_from("mel min").is_err());
        assert!(Mode::try_from("wurst").is_err());
        assert!(Mode::try_from("Lydian Augmented Major").is_err());

        assert!(Mode::try_from("min").is_ok());
        assert!(Mode::try_from("melodic minor").is_ok());
        assert!(Mode::try_from(" Melodic  Min ").is_ok());
        assert!(Mode::try_from("phrygian dom").is_ok());
        assert!(Mode::try_from("Lydian Aug").is_ok());
        assert!(Mode::try_from("Lydian").is_ok());
    }

    #[test]
    fn scale_from_interval() -> Result<(), String> {
        assert!(Mode::try_from(&vec![0, 12]).is_err());
        assert!(Mode::try_from(&vec![0, 4, 2, 5, 7, 9, 11]).is_err());

        assert!(Mode::try_from(&vec![0, 2, 4, 5, 7, 9, 11]).is_ok());
        assert_eq!(
            Mode::try_from(&vec![0, 2, 4, 5, 7, 9, 11])?.degrees,
            Mode::try_from("major")?.degrees
        );

        Ok(())
    }

    #[test]
    fn notes() -> Result<(), String> {
        assert_eq!(
            Scale::new(Note::C4, Mode::try_from("natural minor")?).notes(),
            vec![
                Note::C4,
                Note::D4,
                Note::Ds4,
                Note::F4,
                Note::G4,
                Note::Gs4,
                Note::As4
            ]
        );
        assert_eq!(
            Scale::new(Note::C4, Mode::try_from("melodic minor")?).notes(),
            vec![
                Note::C4,
                Note::D4,
                Note::Ds4,
                Note::F4,
                Note::G4,
                Note::A4,
                Note::B4
            ]
        );
        Ok(())
    }

    #[test]
    fn transpose() {
        assert_eq!(
            Scale::new(Note::C4, SCALE_MODES[1].clone()).transpose_with_strictness(
                Note::C4,
                2,
                TransposeStrictness::ForceAllInScaleNotes,
            ),
            Note::D4
        );
    }

    #[test]
    fn chord() -> Result<(), String> {
        let scale = Scale::new(Note::C4, Mode::try_from("major")?);
        let cmaj = scale.chord_from_degree(1, 3);
        let gmaj7 = scale.chord_from_degree(5, 4);
        assert!(cmaj == vec![Note::C4, Note::E4, Note::G4]);
        assert!(gmaj7 == vec![Note::G4, Note::B4, Note::D5, Note::F5]);
        Ok(())
    }
}
