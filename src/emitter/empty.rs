use crate::{BeatTimeBase, Emitter, EmitterEvent, Event, ParameterSet, RhythmEvent};

// -------------------------------------------------------------------------------------------------

/// Emitter which continuously emits nothing (`None` events).
#[derive(Clone, Debug)]
pub struct EmptyEmitter {}

impl Emitter for EmptyEmitter {
    fn set_time_base(&mut self, _time_base: &BeatTimeBase) {
        // nothing to do
    }

    fn set_trigger_event(&mut self, _event: &Event) {
        // nothing to do
    }

    fn set_parameters(&mut self, _parameters: ParameterSet) {
        // nothing to do
    }

    fn run(&mut self, _pulse: RhythmEvent, _emit_event: bool) -> Option<Vec<EmitterEvent>> {
        None
    }

    fn duplicate(&self) -> Box<dyn Emitter> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        // nothing to do
    }
}
