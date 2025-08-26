use crate::{
    rhythm::euclidean::euclidean, rhythm::RhythmEventIterator, BeatTimeBase, Event, ParameterSet,
    Pulse, Rhythm, RhythmEvent,
};

// -------------------------------------------------------------------------------------------------

/// A rhythm which endlessly emits pulses by stepping through a static pulse rhythm.
#[derive(Clone, Debug)]
pub struct FixedRhythm {
    pulses: Vec<Pulse>,
    pulse_index: usize,
    pulse_iter: Option<RhythmEventIterator>,
    repeat_count_option: Option<usize>,
    repeat_count: usize,
}

impl Default for FixedRhythm {
    fn default() -> Self {
        Self::from_pulses(vec![Pulse::Pulse(1.0)])
    }
}

impl FixedRhythm {
    /// Create from a vector of pulses or a vector of values which can be
    /// converted to pulses (boolean, u32, f32).
    pub fn from_pulses<T>(pulses: Vec<T>) -> Self
    where
        Pulse: std::convert::From<T> + Sized,
    {
        let pulses = pulses
            .into_iter()
            .map(|v| Pulse::from(v))
            .collect::<Vec<_>>();
        let pulse_index = 0;
        let pulse_iter = pulses.first().map(|pulse| pulse.clone().into_iter());
        let repeat_count_option = None;
        let repeat_count = 0;
        FixedRhythm {
            pulses,
            pulse_index,
            pulse_iter,
            repeat_count_option,
            repeat_count,
        }
    }

    /// Create from an euclidean rhythm.
    pub fn from_euclidean(steps: u32, pulses: u32, offset: i32) -> Self {
        Self::from_pulses(euclidean(steps, pulses, offset))
    }
}

impl Rhythm for FixedRhythm {
    fn len(&self) -> usize {
        self.pulses.iter().fold(0, |sum, pulse| sum + pulse.len())
    }

    fn run(&mut self) -> Option<RhythmEvent> {
        // when we have no pulses there's nothing to run
        if self.is_empty() {
            return None;
        }
        // if we have a pulse iterator, consume it
        if let Some(pulse_iter) = &mut self.pulse_iter {
            if let Some(pulse) = pulse_iter.next() {
                return Some(pulse);
            }
        }
        // check if we finished playback
        if self
            .repeat_count_option
            .is_some_and(|option| self.repeat_count > option)
        {
            return None;
        }
        // else move on to the next pulse
        self.pulse_index += 1;
        if self.pulse_index >= self.pulses.len() {
            self.pulse_index = 0;
            self.repeat_count += 1;
            if self
                .repeat_count_option
                .is_some_and(|option| self.repeat_count > option)
            {
                return None;
            }
        }
        // reset pulse iter and fetch first pulse from it
        let mut pulse_iter = self.pulses[self.pulse_index].clone().into_iter();
        let pulse = pulse_iter.next().unwrap_or_default();
        self.pulse_iter = Some(pulse_iter);
        Some(pulse)
    }

    fn set_time_base(&mut self, _time_base: &BeatTimeBase) {
        // nothing to do
    }

    fn set_trigger_event(&mut self, _event: &Event) {
        // nothing to do
    }

    fn set_parameters(&mut self, _parameters: ParameterSet) {
        // nothing to do
    }

    fn set_repeat_count(&mut self, count: Option<usize>) {
        self.repeat_count_option = count;
    }

    fn duplicate(&self) -> Box<dyn Rhythm> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        self.repeat_count = 0;
        self.pulse_index = 0;
        if self.pulses.is_empty() {
            self.pulse_iter = None;
        } else {
            self.pulse_iter = Some(self.pulses[self.pulse_index].clone().into_iter());
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// Create [`FixedRhythm`]s from convertible types.
pub trait ToFixedRhythm {
    fn to_rhythm(self) -> FixedRhythm;
}

impl<T> ToFixedRhythm for Vec<T>
where
    Pulse: std::convert::From<T>,
{
    /// Create a vector of pulses, numbers or booleans to a new [`FixedRhythm`].
    fn to_rhythm(self) -> FixedRhythm {
        FixedRhythm::from_pulses(self)
    }
}

impl<const N: usize, T> ToFixedRhythm for [T; N]
where
    Pulse: std::convert::From<T>,
{
    /// Create a static array of pulses, numbers or booleans to a new [`FixedRhythm`].
    fn to_rhythm(self) -> FixedRhythm {
        FixedRhythm::from_pulses(self.into_iter().collect::<Vec<_>>())
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run() {
        let mut rhythm = [1.0, 0.0, 1.0, 0.0].to_rhythm();
        assert_eq!(
            vec![rhythm.run(), rhythm.run(), rhythm.run(), rhythm.run()],
            vec![
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                })
            ]
        );

        rhythm = [
            Pulse::from(1.0),
            Pulse::from(0.0),
            Pulse::from(vec![Pulse::from(vec![0.0, 1.0]), Pulse::from(1.0)]),
            Pulse::from(0.0),
        ]
        .to_rhythm();
        assert_eq!(
            vec![
                rhythm.run(),
                rhythm.run(),
                rhythm.run(),
                rhythm.run(),
                rhythm.run(),
                rhythm.run()
            ],
            vec![
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 0.25,
                }),
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 0.25,
                }),
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 0.5,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                })
            ]
        );
    }

    #[test]
    fn repeat() {
        let mut rhythm = [1.0, 0.0].to_rhythm();
        rhythm.set_repeat_count(Some(1));
        assert_eq!(
            vec![rhythm.run(), rhythm.run(), rhythm.run(), rhythm.run()],
            vec![
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                })
            ]
        );
        assert_eq!(rhythm.run(), None);
    }
}
