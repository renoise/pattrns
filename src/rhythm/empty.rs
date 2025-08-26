use crate::{BeatTimeBase, Event, ParameterSet, Rhythm, RhythmEvent};

// -------------------------------------------------------------------------------------------------

/// An empty rhythm which continuously emits None pulse values.
#[derive(Clone, Debug, Default)]
pub struct EmptyRhythm {}

impl EmptyRhythm {
    pub fn new() -> Self {
        Self {}
    }
}

impl Rhythm for EmptyRhythm {
    fn is_empty(&self) -> bool {
        true
    }

    fn len(&self) -> usize {
        0
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

    fn set_repeat_count(&mut self, _count: Option<usize>) {
        // nothing to do
    }

    fn run(&mut self) -> Option<RhythmEvent> {
        None
    }

    fn duplicate(&self) -> Box<dyn Rhythm> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        // nothing to do
    }
}
