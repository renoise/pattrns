use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[cfg(test)]
use std::fmt::Display;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use rand::{rng, Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

type Fraction = num_rational::Rational32;
use num_traits::{FromPrimitive, ToPrimitive};

use crate::rhythm::euclidean::euclidean;

// -------------------------------------------------------------------------------------------------

const OVERFLOW_ERROR: &str = "Internal error: integer overflow in cycle";

// -------------------------------------------------------------------------------------------------

type Vars = HashMap<Rc<str>, Step>;

/// public wrapper over a constant Step containing no variables
/// to be used as a variable for the main cycle
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubCycle {
    step: Step,
}

impl SubCycle {
    pub fn from(input: &str) -> Result<SubCycle, String> {
        let cycle = Cycle::from(input)?;
        let vars = cycle.root.collect_vars();
        if vars.is_empty() {
            Ok(SubCycle { step: cycle.root })
        } else {
            Err(format!("cycle contains variables\n{vars:?}"))
        }
    }
    pub fn float(f: f64) -> Self {
        Self::new(Step::constant(Constant::Float(f), None))
    }
    pub fn integer(i: i32) -> Self {
        Self::new(Step::constant(Constant::Integer(i), None))
    }
    fn new(step: Step) -> Self {
        Self { step }
    }
}

/// Tidal cycle mini notation parser and event generator.
#[derive(Debug, Clone, PartialEq)]
pub struct Cycle {
    root: Step,
    event_limit: usize,
    input: String,
    source: Option<String>,
    seed: Option<u64>,
    state: CycleState,
    vars: Option<Vars>,
}

impl Cycle {
    /// Default value for the cycle's event limit option.
    const EVENT_LIMIT_DEFAULT: usize = 0x1000;

    /// Create a Cycle from a mini-notation string, using an unseeded random number generator
    /// and the default event limit setting.
    ///
    /// Returns a parse error, when the given string is not a valid mini notation expression.
    pub fn from(input: &str) -> Result<Self, String> {
        CycleParser::parse_from_rule(Rule::mini, input).and_then(|root_pair| {
            let root = CycleParser::step(root_pair)?;
            let input = input.to_string();
            let state = CycleState {
                events: 0,
                iteration: 0,
                rng: Xoshiro256PlusPlus::from_seed(rng().random()),
            };
            let seed = None;
            let source = None;
            let event_limit = Self::EVENT_LIMIT_DEFAULT;
            let vars = None;
            let cycle = Self {
                input,
                seed,
                source,
                root,
                state,
                event_limit,
                vars,
            };
            #[cfg(test)]
            {
                println!("\nCYCLE");
                cycle.print();
            }
            Ok(cycle)
        })
    }

    #[cfg(test)]
    fn constant_from(input: &str) -> Result<Constant, String> {
        CycleParser::parse_from_rule(Rule::constant_literal, input).and_then(|root| {
            let string = root.as_str();
            match CycleParser::single(root)? {
                Step::Single(single) => Ok(single.value),
                _ => Err(format!("single constant expected, found {}", string)),
            }
        })
    }

    /// Rebuild/configure a newly created cycle to use the given custom seed.
    pub fn with_seed(self, seed: u64) -> Self {
        debug_assert!(
            self.state.iteration == 0,
            "Should not reconfigure seed of running cycle"
        );
        Self {
            seed: Some(seed),
            ..self
        }
    }

    /// Rebuild/configure a newly created cycle with the given source hint.
    pub fn with_source(self, source: &str) -> Self {
        debug_assert!(
            self.state.iteration == 0,
            "Should not reconfigure seed of running cycle"
        );
        Self {
            source: Some(source.to_string()),
            ..self
        }
    }

    /// Rebuild/configure cycle to use the given custom event count limit.
    pub fn with_event_limit(self, event_limit: usize) -> Self {
        Self {
            event_limit,
            ..self
        }
    }

    pub fn set_var(&mut self, name: &str, subcycle: SubCycle) {
        if let Some(vars) = self.vars.as_mut() {
            vars.insert(name.into(), subcycle.step);
        } else {
            let mut vars = HashMap::new();
            vars.insert(name.into(), subcycle.step);
            self.vars = Some(vars);
        }
    }

    #[cfg(test)]
    fn set_var_constant(&mut self, name: &str, constant: Constant) {
        if let Some(vars) = self.vars.as_mut() {
            vars.insert(name.into(), Step::constant(constant, Some(name)));
        } else {
            let mut vars = HashMap::new();
            vars.insert(name.into(), Step::constant(constant, Some(name)));
            self.vars = Some(vars);
        }
    }

    /// Check if a cycle may give different outputs between cycles.
    pub fn is_stateful(&self) -> bool {
        // TODO improve: * and / can change the output, <1> does not etc..
        self.input.contains(['<', '{', '|', '?', '/', '*'])
    }

    /// When the cycle got created from a script source,
    /// return source string indentifier (maybe a path), else None.
    pub fn source(&self) -> &Option<String> {
        &self.source
    }

    /// Query for the next iteration of output.
    ///
    /// Returns error when the number of generated events exceed the configured event limit.
    pub fn generate(&mut self) -> Result<Vec<Vec<Event>>, String> {
        let cycle = self.state.iteration;
        self.state.events = 0;
        if let Some(seed) = self.seed {
            self.state.rng = Xoshiro256PlusPlus::seed_from_u64(seed.wrapping_add(cycle as u64));
        }
        let mut events = Self::output(
            &self.root,
            &mut self.state,
            cycle,
            self.event_limit,
            false,
            self.vars.as_ref(),
        )?;
        self.state.iteration += 1;
        events.transform_spans(&Span::default());
        Ok(events.export())
    }

    /// Move cycle iteration without generating any events.
    pub fn advance(&mut self) {
        self.state.iteration += 1;
    }

    /// reset state to initial state
    pub fn reset(&mut self) {
        self.state.iteration = 0;
        self.state.events = 0;
    }
}

/// Musical event with timing and value information within a [`Cycle`].
#[derive(Debug, Clone)]
pub struct Event {
    length: Fraction,
    span: Span,
    value: Constant,
    string: Rc<str>,
    targets: Vec<Target>,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            length: Fraction::default(),
            span: Span::default(),
            value: Constant::default(),
            string: Rc::from("~"),
            targets: vec![],
        }
    }
}

impl Event {
    /// The step's value as string representation: Either the value name's original string value
    /// or the original value string, unparsed as fallback.
    pub fn as_str(&self) -> Rc<str> {
        match &self.value {
            // prefer `Name` over self.string, as the string may contain unresolved variables
            Constant::Name(name) => Rc::clone(name),
            _ => Rc::clone(&self.string),
        }
    }

    /// The step's original parsed value.
    pub fn value(&self) -> &Constant {
        &self.value
    }

    /// The step's time span.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// The step's length.
    pub fn length(&self) -> &Fraction {
        &self.length
    }

    /// The step's optional targets.
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }
}

/// Time span for musical events within Cycles.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    start: Fraction,
    end: Fraction,
}

#[cfg(test)]
impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:.3} -> {:.3}", self.start, self.end)
    }
}

impl Span {
    pub fn start(&self) -> Fraction {
        self.start
    }

    pub fn end(&self) -> Fraction {
        self.end
    }

    pub fn length(&self) -> Fraction {
        self.end - self.start
    }
}

/// Possible types of values that are emitted by a [`Cycle`].
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Constant {
    Null,
    #[default]
    Rest,
    Hold,
    Float(f64),
    Integer(i32),
    Pitch(Pitch),
    Chord(Pitch, Rc<str>),
    Target(Target),
    Name(Rc<str>),
}

/// Sample/instrument target information for cycle events.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Index(i32),
    NamedFloat(Rc<str>, f64),
    Named(Rc<str>),
}

/// The kind of target used for target assignments
#[derive(Clone, Debug, PartialEq)]
pub enum TargetKind {
    Index,
    Named(Rc<str>),
}

impl Target {
    pub fn equal_key(&self, other: &Self) -> bool {
        match (self, other) {
            // both are indices: compare index values
            (Self::Index(_), Self::Index(_)) => true,
            // both are names: compare names only
            (Self::NamedFloat(a, _), Self::NamedFloat(b, _)) => a == b,
            _ => false,
        }
    }

    pub fn to_integer(&self) -> Option<i32> {
        match self {
            Target::Index(i) => Some(*i),
            _ => None,
        }
    }

    pub fn named_float(name: &str, float: f64) -> Self {
        Self::NamedFloat(Rc::from(name), float)
    }
}

/// Pitch with note and octave information for cycle events.
#[derive(Clone, Debug, PartialEq)]
pub struct Pitch {
    note: u8,
    octave: u8,
}

impl Pitch {
    pub fn midi_note(&self) -> u8 {
        (self.octave as u32 * 12 + self.note as u32).min(0x7f) as u8
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Step {
    Var(Rc<str>),
    Single(Single),
    Alternating(Alternating),
    Subdivision(Subdivision),
    Polymeter(Polymeter),
    Stack(Stack),
    Choices(Choices),
    SpeedExpression(SpeedExpression),
    ReplicateExpression(ReplicateExpression),
    WeightExpression(WeightExpression),
    TargetExpression(TargetExpression),
    Degrade(Degrade),
    Bjorklund(Bjorklund),
    Static(Static),
}

impl Default for Step {
    fn default() -> Self {
        Self::Single(Single::default())
    }
}

impl Step {
    pub fn constant(constant: Constant, string: Option<&str>) -> Self {
        Self::Single(Single {
            value: constant,
            string: Rc::from(string.unwrap_or_default()),
        })
    }

    #[cfg(test)]
    fn inner_steps(&self) -> Vec<&Step> {
        match self {
            Step::Var(_) => vec![],
            Step::Single(_s) => vec![],
            Step::Alternating(a) => a.steps.iter().collect(),
            Step::Polymeter(pm) => pm.steps.as_ref().inner_steps(),
            Step::Subdivision(sd) => sd.steps.iter().collect(),
            Step::Choices(cs) => cs.choices.iter().collect(),
            Step::Stack(st) => st.stack.iter().collect(),
            Step::SpeedExpression(e) => vec![&e.step, &e.mult],
            Step::WeightExpression(e) => vec![&e.step, &e.weight],
            Step::ReplicateExpression(e) => vec![&e.step, &e.count],
            Step::Degrade(e) => vec![&e.step, &e.chance],
            Step::TargetExpression(e) => vec![&e.step, &e.target],
            Step::Bjorklund(b) => {
                if let Some(rotation) = &b.rotation {
                    vec![&b.left, &b.steps, &b.pulses, &**rotation]
                } else {
                    vec![&b.left, &b.steps, &b.pulses]
                }
            }
            Step::Static(s) => match s {
                Static::Repeat => vec![],
                Static::Range(_) => vec![],
            },
        }
    }

    fn get_vars(&self, vars: &mut HashSet<Rc<str>>) {
        match self {
            Step::Var(name) => {
                vars.insert(Rc::clone(name));
            }
            Step::Single(_s) => (),
            Step::Static(_s) => (),
            Step::Alternating(a) => a.steps.iter().for_each(|s| s.get_vars(vars)),

            Step::Polymeter(pm) => {
                pm.count.get_vars(vars);
                pm.steps.get_vars(vars);
            }
            Step::Subdivision(sd) => sd.steps.iter().for_each(|s| s.get_vars(vars)),
            Step::Choices(cs) => cs.choices.iter().for_each(|s| s.get_vars(vars)),
            Step::Stack(st) => st.stack.iter().for_each(|s| s.get_vars(vars)),
            Step::SpeedExpression(e) => {
                e.step.get_vars(vars);
                e.mult.get_vars(vars);
            }
            Step::WeightExpression(e) => {
                e.step.get_vars(vars);
                e.weight.get_vars(vars);
            }
            Step::ReplicateExpression(e) => {
                e.step.get_vars(vars);
                e.count.get_vars(vars);
            }
            Step::Degrade(e) => {
                e.step.get_vars(vars);
                e.chance.get_vars(vars);
            }
            Step::TargetExpression(e) => {
                e.step.get_vars(vars);
                e.target.get_vars(vars);
            }
            Step::Bjorklund(b) => {
                b.steps.get_vars(vars);
                b.pulses.get_vars(vars);
                if let Some(rotation) = &b.rotation {
                    rotation.get_vars(vars);
                }
            }
        }
    }

    fn collect_vars(&self) -> HashSet<Rc<str>> {
        let mut vars = HashSet::new();
        self.get_vars(&mut vars);
        vars
    }

    fn rest() -> Self {
        Self::Single(Single {
            value: Constant::Rest,
            string: Rc::from("~"),
        })
    }

    fn subdivision(steps: Vec<Step>) -> Self {
        Self::Subdivision(Subdivision { steps })
    }
    fn alternating(steps: Vec<Step>) -> Self {
        Self::Alternating(Alternating { steps })
    }
    fn polymeter(steps: Vec<Step>, count: Step) -> Self {
        Step::Polymeter(Polymeter {
            steps: Box::new(Step::subdivision(steps)),
            count: Box::new(count),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Static {
    Range(Range),
    Repeat,
}

#[derive(Clone, Debug, PartialEq)]
struct Single {
    value: Constant,
    string: Rc<str>,
}

impl Default for Single {
    fn default() -> Self {
        Single {
            value: Constant::Null,
            string: Rc::from(""),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Alternating {
    steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Subdivision {
    steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Polymeter {
    count: Box<Step>,
    steps: Box<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Choices {
    choices: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Stack {
    stack: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
enum SpeedOp {
    Fast(), // *
    Slow(), // /
}

#[derive(Clone, Debug, PartialEq)]
enum Operator {
    Speed(SpeedOp),
    Replicate(), // !
    Weight(),    // @
    Target(),    // :
    Bjorklund(), // (p,s,r)
    Degrade(),   // ?
}

impl Operator {
    fn parse(pair: Pair<Rule>) -> Result<Self, String> {
        match pair.as_rule() {
            Rule::op_degrade => Ok(Self::Degrade()),
            Rule::op_replicate => Ok(Self::Replicate()),
            Rule::op_weight => Ok(Self::Weight()),
            Rule::op_fast => Ok(Self::Speed(SpeedOp::Fast())),
            Rule::op_slow => Ok(Self::Speed(SpeedOp::Slow())),
            Rule::op_target => Ok(Self::Target()),
            Rule::op_bjorklund => Ok(Self::Bjorklund()),
            _ => Err(format!("unsupported operator: {:?}", pair.as_rule())),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SpeedExpression {
    op: SpeedOp,
    step: Box<Step>,
    mult: Box<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct WeightExpression {
    step: Box<Step>,
    weight: Box<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct ReplicateExpression {
    step: Box<Step>,
    count: Box<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Degrade {
    step: Box<Step>,
    chance: Box<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct TargetExpression {
    step: Box<Step>,
    kind: Option<TargetKind>,
    target: Box<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Bjorklund {
    left: Box<Step>,
    steps: Box<Step>,
    pulses: Box<Step>,
    rotation: Option<Box<Step>>,
}

#[derive(Clone, Debug, PartialEq)]
struct Range {
    start: i32,
    end: i32,
}

// -------------------------------------------------------------------------------------------------

impl Target {
    fn from_index(index: i32) -> Self {
        Self::Index(index)
    }

    fn from_name(str: Rc<str>) -> Self {
        Self::Named(str)
    }
}

impl Pitch {
    fn parse(pair: Pair<Rule>) -> Pitch {
        let mut pitch = Pitch { note: 0, octave: 4 };
        let mut mark: i8 = 0;
        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::note => {
                    if let Some(c) = String::from(p.as_str()).to_ascii_lowercase().chars().next() {
                        pitch.note = Self::as_note_value(c).unwrap_or(pitch.note)
                    }
                }
                Rule::octave => pitch.octave = p.as_str().parse::<u8>().unwrap_or(pitch.octave),
                Rule::mark => match p.as_str() {
                    "#" => mark = 1,
                    "b" => mark = -1,
                    _ => (),
                },
                _ => (),
            }
        }
        // maybe an error should be thrown instead of a silent clamp
        if pitch.note == 0 && mark == -1 {
            if pitch.octave > 0 {
                pitch.octave -= 1;
                pitch.note = 11;
            }
        } else if pitch.note == 11 && mark == 1 {
            if pitch.octave < 10 {
                pitch.note = 0;
                pitch.octave += 1;
            }
        } else {
            pitch.note = ((pitch.note as i8) + mark) as u8;
        }
        // pitch.note = pitch.note.clamp(0, 127);
        pitch
    }

    fn as_note_value(note: char) -> Option<u8> {
        match note {
            'c' => Some(0),
            'd' => Some(2),
            'e' => Some(4),
            'f' => Some(5),
            'g' => Some(7),
            'a' => Some(9),
            'b' => Some(11),
            _ => None,
        }
    }
}

#[cfg(test)]
impl Display for Pitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = match self.note {
            0 => "c",
            1 => "c#",
            2 => "d",
            3 => "d#",
            4 => "e",
            5 => "f",
            6 => "f#",
            7 => "g",
            8 => "g#",
            9 => "a",
            10 => "a#",
            11 => "b",
            _ => "",
        };
        if self.octave == 4 {
            f.write_str(n)
        } else {
            f.write_fmt(format_args!("{}{}", n, self.octave))
        }
    }
}

impl Constant {
    fn parse_integer(str: &str) -> Result<i32, String> {
        if let Some(hex) = str.strip_prefix("0x").or(str.strip_prefix("0X")) {
            i32::from_str_radix(hex, 16).map_err(|err| err.to_string())
        } else if let Some(hex) = str.strip_prefix("-0x").or(str.strip_prefix("-0X")) {
            i32::from_str_radix(hex, 16)
                .map(|v| -v)
                .map_err(|err| err.to_string())
        } else {
            str.parse::<i32>().map_err(|err| err.to_string())
        }
    }

    fn parse_float(str: &str) -> Result<f64, String> {
        str.parse::<f64>().map_err(|err| err.to_string())
    }

    fn from_float(str: &str) -> Result<Self, String> {
        Self::parse_float(str).map(Self::Float)
    }

    fn from_integer(str: &str) -> Result<Self, String> {
        Self::parse_integer(str).map(Self::Integer)
    }

    fn to_integer(&self) -> Option<i32> {
        match &self {
            Self::Null => None,
            Self::Rest => None,
            Self::Hold => None,
            Self::Integer(i) => Some(*i),
            Self::Float(f) => Some(*f as i32),
            Self::Pitch(n) => Some(n.midi_note() as i32),
            Self::Chord(p, _m) => Some(p.midi_note() as i32),
            Self::Target(t) => t.to_integer(),
            Self::Name(_n) => None,
        }
    }

    fn to_float(&self) -> Option<f64> {
        match &self {
            Self::Null => None,
            Self::Float(f) => Some(*f),
            Self::Integer(i) => Some(*i as f64),
            Self::Pitch(n) => Some(n.midi_note() as f64),
            Self::Chord(n, _m) => Some(n.midi_note() as f64),
            Self::Target(t) => match t {
                Target::Index(i) => Some(*i as f64),
                Target::NamedFloat(_, f) => Some(*f),
                Target::Named(_) => None,
            },

            Self::Rest => None,
            Self::Hold => None,
            Self::Name(_n) => None,
        }
    }

    fn to_chance(&self) -> Option<f64> {
        match &self {
            Self::Null => None,
            Self::Rest => None,
            Self::Hold => None,
            Self::Integer(i) => Some((*i as f64).clamp(0.0, 100.0) / 100.0),
            Self::Float(f) => Some(f.clamp(0.0, 1.0)),
            Self::Pitch(p) => Some((p.midi_note() as f64).clamp(0.0, 128.0) / 128.0),
            Self::Chord(p, _m) => Some((p.midi_note() as f64).clamp(0.0, 128.0) / 128.0),
            Self::Target(t) => match t {
                Target::Index(i) => Some(*i as f64),
                Target::NamedFloat(_, f) => Some(f.clamp(0.0, 1.0)),
                Target::Named(_) => None,
            },
            Self::Name(_n) => None,
        }
    }

    fn to_fraction(&self) -> Option<Fraction> {
        self.to_float().and_then(Fraction::from_f64)
    }

    fn to_target_without_kind(&self, string: &Rc<str>) -> Option<Target> {
        match self {
            Self::Null => None,
            Self::Target(t) => Some(t.clone()),
            Self::Integer(i) => Some(Target::from_index(*i)),
            Self::Name(name) => Some(Target::from_name(Rc::clone(name))),
            Self::Float(_) | Self::Pitch(_) | Self::Chord(_, _) => {
                // pass unexpected values as raw string and let clients deal with conversions or errors
                Some(Target::from_name(Rc::clone(string)))
            }
            Self::Rest | Self::Hold => None,
        }
    }

    fn to_target_with_kind(&self, kind: &TargetKind) -> Option<Target> {
        match self {
            // inner targets override outer target
            Self::Target(t) => Some(t.clone()),
            // // TODO allow string values for target outputs as per #94
            // Self::Name(_name) => None,
            _ => match kind {
                TargetKind::Index => self.to_integer().map(Target::from_index),
                TargetKind::Named(name) => self
                    .to_float()
                    .map(|f| Target::NamedFloat(Rc::clone(name), f)),
            },
        }
    }
    fn to_target(&self, string: &Rc<str>, kind: Option<&TargetKind>) -> Option<Target> {
        kind.and_then(|kind| self.to_target_with_kind(kind))
            .or(self.to_target_without_kind(string))
    }
}

impl Span {
    fn new(start: Fraction, end: Fraction) -> Self {
        Self { start, end }
    }

    /// transforms the span relative to an outer span.
    fn transform(&mut self, outer: &Span) {
        let outer_length = outer.length();
        let previous_length = self.length();
        self.start = outer.start + outer_length * self.start;
        self.end = self.start + outer_length * previous_length;
    }

    /// transforms the span to 0..1 based on an outer span
    /// assumes self is inside outer
    fn normalize(&mut self, outer: &Span) {
        let outer_length = outer.length();
        if outer_length != Fraction::ZERO {
            self.start = (self.start - outer.start) / outer_length;
            self.end = (self.end - outer.start) / outer_length;
        } else {
            self.start = Fraction::ZERO;
            self.end = Fraction::ZERO;
        }
    }

    fn whole_range(&self) -> std::ops::Range<u32> {
        let start = self.start.floor().to_u32().unwrap_or_default();
        let end = self.end.ceil().to_u32().unwrap_or_default();
        start..end
    }

    fn overlaps(&self, span: &Span) -> bool {
        self.start < span.end && span.start < self.end
    }

    fn includes(&self, span: &Span) -> bool {
        self.start <= span.start && span.start < self.end
    }

    /// Limit self to not extend beyond the target span
    /// this function assumes self.overlaps(span) is true
    fn crop(&mut self, span: &Span) {
        if self.start < span.start {
            self.start = span.start;
        }
        if self.end > span.end {
            self.end = span.end
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Span {
            start: Fraction::ZERO,
            end: Fraction::ONE,
        }
    }
}

impl Event {
    #[cfg(test)]
    fn at(start: Fraction, length: Fraction) -> Self {
        Self {
            length,
            span: Span {
                start,
                end: start + length,
            },
            string: Rc::from("~"),
            value: Constant::Rest,
            targets: vec![],
        }
    }

    #[cfg(test)]
    fn with_note(&self, note: u8, octave: u8) -> Self {
        let pitch = Pitch { note, octave };
        Self {
            value: Constant::Pitch(pitch.clone()),
            string: Rc::from(pitch.to_string()),
            ..self.clone()
        }
    }

    #[cfg(test)]
    fn with_chord(&self, note: u8, octave: u8, mode: &str) -> Self {
        let pitch = Pitch { note, octave };
        Self {
            value: Constant::Chord(pitch.clone(), Rc::from(mode)),
            string: Rc::from(format!("{}'{}", pitch, mode)),
            ..self.clone()
        }
    }

    #[cfg(test)]
    fn with_int(&self, i: i32) -> Self {
        Self {
            value: Constant::Integer(i),
            string: Rc::from(i.to_string()),
            ..self.clone()
        }
    }

    #[cfg(test)]
    fn with_name(&self, n: &'static str) -> Self {
        Self {
            value: Constant::Name(Rc::from(n)),
            string: Rc::from(n.to_string()),
            ..self.clone()
        }
    }

    #[cfg(test)]
    fn with_float(&self, f: f64) -> Self {
        Self {
            value: Constant::Float(f),
            string: Rc::from(f.to_string()),
            ..self.clone()
        }
    }

    #[cfg(test)]
    fn with_target(&self, target: Target) -> Self {
        Self {
            targets: vec![target],
            ..self.clone()
        }
    }

    #[cfg(test)]
    fn with_targets(&self, targets: Vec<Target>) -> Self {
        Self {
            targets,
            ..self.clone()
        }
    }

    fn extend(&mut self, next: &Event) {
        self.length += next.length;
        self.span.end = next.span.end
    }
}

impl PartialEq<Event> for Event {
    fn eq(&self, other: &Event) -> bool {
        // Don't compare self.string: compare interpreted values and target only.
        self.length == other.length
            && self.span == other.span
            && self.value == other.value
            && self.targets == other.targets
    }
}

#[cfg(test)]
impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} | {:?} {:?}",
            self.span, self.value, self.targets
        ))
    }
}

#[derive(Debug, Clone)]
struct MultiEvents {
    length: Fraction,
    span: Span,
    events: Vec<Events>,
}

#[derive(Debug, Clone)]
struct PolyEvents {
    length: Fraction,
    span: Span,
    channels: Vec<Events>,
}

#[derive(Debug, Clone)]
enum Events {
    Single(Event),
    Multi(MultiEvents),
    Poly(PolyEvents),
}

impl Events {
    fn empty() -> Events {
        Events::Single(Event {
            length: Fraction::ONE,
            span: Span::default(),
            string: Rc::from("~"),
            value: Constant::Rest,
            targets: vec![],
        })
    }

    fn named(name: &str) -> Self {
        Events::Single(Event {
            length: Fraction::ONE,
            span: Span::default(),
            string: Rc::from(name),
            value: Constant::Name(Rc::from(name)),
            targets: vec![],
        })
    }

    fn maybe_poly(poly: PolyEvents) -> Self {
        if poly.channels.len() == 1 {
            poly.channels.into_iter().next().expect("len is 1")
        } else {
            Self::Poly(poly)
        }
    }

    fn get_length(&self) -> Fraction {
        match self {
            Events::Single(s) => s.length,
            Events::Multi(m) => m.length,
            Events::Poly(p) => p.length,
        }
    }

    fn set_length(&mut self, length: Fraction) {
        match self {
            Events::Single(s) => s.length = length,
            Events::Multi(m) => m.length = length,
            Events::Poly(p) => p.length = length,
        }
    }

    fn first(&self) -> Option<Event> {
        match self {
            Events::Single(s) => Some(s.clone()),
            Events::Multi(m) => m.events.first().and_then(Self::first),
            Events::Poly(p) => p.channels.first().and_then(Self::first),
        }
    }

    fn get_span(&self) -> Span {
        match self {
            Events::Single(s) => s.span.clone(),
            Events::Multi(m) => m.span.clone(),
            Events::Poly(p) => p.span.clone(),
        }
    }

    /// Fits a list of events into a Span of 0..1
    fn subdivide_lengths(events: &mut Vec<Events>) {
        let mut length = Fraction::ZERO;
        for e in &mut *events {
            match e {
                Events::Single(s) => length += s.length,
                Events::Multi(m) => length += m.length,
                Events::Poly(p) => length += p.length,
            }
        }
        let step_size = if length != Fraction::ZERO {
            Fraction::ONE / length
        } else {
            Fraction::ZERO
        };
        let mut start = Fraction::ZERO;
        for e in &mut *events {
            match e {
                Events::Single(s) => {
                    s.length *= step_size;
                    s.span = Span::new(start, start + s.length);
                    start += s.length
                }
                Events::Multi(m) => {
                    m.length *= step_size;
                    m.span = Span::new(start, start + m.length);
                    start += m.length
                }
                Events::Poly(p) => {
                    p.length *= step_size;
                    p.span = Span::new(start, start + p.length);
                    start += p.length
                }
            }
        }
    }

    fn filter_mut<F>(&mut self, predicate: &mut F) -> bool
    where
        F: FnMut(&mut Event) -> bool,
    {
        match self {
            Events::Multi(m) => {
                let mut filtered = Vec::with_capacity(m.events.len());
                for e in &mut m.events {
                    match e {
                        Events::Single(s) => {
                            if predicate(s) {
                                filtered.push(e.clone())
                            }
                        }
                        _ => {
                            if e.filter_mut(predicate) {
                                filtered.push(e.clone())
                            }
                        }
                    }
                }
                m.events = filtered;
                !m.events.is_empty()
            }
            Events::Poly(p) => {
                let mut filtered = Vec::with_capacity(p.channels.len());
                for e in &mut p.channels {
                    if e.filter_mut(predicate) {
                        filtered.push(e.clone())
                    }
                }
                p.channels = filtered;
                !p.channels.is_empty()
            }
            Events::Single(_) => true,
        }
    }

    fn crop(&mut self, span: &Span, overlap: bool) {
        self.filter_mut(&mut |e| {
            let keep = if overlap {
                span.overlaps(&e.span)
            } else {
                span.includes(&e.span)
            };

            if keep {
                e.span.crop(span);
            }
            keep
        });
    }

    fn mutate_events<F>(&mut self, fun: &mut F)
    where
        F: FnMut(&mut Event),
    {
        match self {
            Events::Single(s) => {
                fun(s);
            }
            Events::Multi(m) => {
                for e in &mut m.events {
                    e.mutate_events(fun);
                }
            }
            Events::Poly(p) => {
                for e in &mut p.channels {
                    e.mutate_events(fun);
                }
            }
        }
    }

    /// recursively transform the spans of events from 0..1 to a given span
    fn transform_spans(&mut self, span: &Span) {
        let unit = span.length();
        match self {
            Events::Single(s) => {
                s.length *= unit;
                s.span.transform(span);
            }
            Events::Multi(m) => {
                m.length *= unit;
                m.span.transform(span);
                for e in &mut m.events {
                    e.transform_spans(&m.span);
                }
            }
            Events::Poly(p) => {
                p.length *= unit;
                p.span.transform(span);
                for e in &mut p.channels {
                    e.transform_spans(&p.span);
                }
            }
        }
    }

    /// recursively transform the spans of events to 0..1 range
    fn normalize_spans(&mut self, span: &Span) {
        match self {
            Events::Single(s) => {
                s.span.normalize(span);
                s.length = s.span.length();
            }
            Events::Multi(m) => {
                for e in &mut m.events {
                    e.normalize_spans(&m.span);
                }

                m.span.normalize(span);
                m.length = m.span.length();
            }
            Events::Poly(p) => {
                for e in &mut p.channels {
                    e.normalize_spans(&p.span);
                }
                p.span.normalize(span);
                p.length = p.span.length();
            }
        }
    }

    /// Recursively collapses Multi and Poly Events into vectors of Single Events
    fn flatten(&self, channels: &mut Vec<Vec<Event>>, channel: &mut usize) {
        if channels.len() <= *channel {
            channels.push(vec![])
        }
        match self {
            Events::Single(s) => channels[*channel].push(s.clone()),
            Events::Multi(m) => {
                for e in &m.events {
                    e.flatten(channels, channel);
                }
            }
            Events::Poly(p) => {
                for e in &p.channels {
                    e.flatten(channels, channel);
                    *channel += 1
                }
            }
        }
    }

    // filter out holds while extending preceding events
    fn merge_holds(events: &mut Vec<Event>) {
        if events.iter().any(|e| e.value == Constant::Hold) {
            let mut result: Vec<Event> = Vec::with_capacity(events.len());
            for e in events.iter() {
                match e.value {
                    Constant::Hold => {
                        if let Some(last) = result.last_mut() {
                            last.extend(e)
                        }
                    }
                    _ => result.push(e.clone()),
                }
            }
            *events = result
        }
    }

    // filter out consecutive rests
    // so any remaining rest can be converted to a note-off later
    // rests at the beginning of a pattern also get dropped
    fn merge_rests(events: &mut Vec<Event>) {
        if events.iter().any(|e| e.value == Constant::Rest) {
            let mut result: Vec<Event> = Vec::with_capacity(events.len());
            for e in events.iter() {
                match e.value {
                    Constant::Rest => {
                        if let Some(last) = result.last_mut() {
                            match last.value {
                                Constant::Rest => last.extend(e),
                                _ => result.push(e.clone()),
                            }
                        }
                    }
                    _ => result.push(e.clone()),
                }
            }
            *events = result
        }
    }

    /// Removes Holds by extending preceding events and filters out Rests
    fn merge(channels: &mut [Vec<Event>]) {
        for events in &mut *channels {
            Self::merge_holds(events);
        }
        for events in channels {
            Self::merge_rests(events);
        }
    }

    fn export(&self) -> Vec<Vec<Event>> {
        let mut channels = vec![];
        self.flatten(&mut channels, &mut 0);
        Self::merge(&mut channels);
        channels.retain(|c| !c.is_empty());

        #[cfg(test)]
        {
            self.print(0);
            println!("\nOUTPUT");
            let channel_count = channels.len();
            for (ci, channel) in channels.iter().enumerate() {
                if channel_count > 1 {
                    println!(" /{}", ci);
                }
                for (i, event) in channel.iter().enumerate() {
                    println!("  │{:02}│ {}", i, event);
                }
            }
        }

        channels
    }

    #[cfg(test)]
    fn print(&self, depth: usize) {
        let indent = " ".repeat(depth * 2);
        match self {
            Events::Single(s) => println!("{}'{}' {}", indent, s.string, s),
            Events::Multi(m) => {
                println!(
                    "{}multi {} -> {} [{}]",
                    indent, m.span.start, m.span.end, m.length
                );
                for e in &m.events {
                    e.print(depth + 1)
                }
            }
            Events::Poly(p) => {
                println!(
                    "{}poly {} -> {} [{}]",
                    indent, p.span.start, p.span.end, p.length
                );
                for e in &p.channels {
                    e.print(depth + 1)
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Parser)]
#[grammar = "tidal/cycle.pest"]
struct CycleParser {}

impl CycleParser {
    fn parse_from_rule(rule: Rule, input: &'_ str) -> Result<Pair<'_, Rule>, String> {
        match Self::parse(rule, input) {
            Ok(mut tree) => {
                if let Some(step_pair) = tree.next() {
                    #[cfg(test)]
                    {
                        println!("\nTREE");
                        Self::print_pairs(&step_pair, 0);
                    }
                    Ok(step_pair)
                } else {
                    Err("couldn't parse input".to_string())
                }
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[cfg(test)]
    fn print_pairs(pair: &Pair<Rule>, level: usize) {
        println!(
            "{} {:?} {:?}",
            indent_lines(level),
            pair.as_rule(),
            pair.as_str()
        );
        for p in pair.clone().into_inner() {
            Self::print_pairs(&p, level + 1)
        }
    }

    /// the errors here should be unreachable unless there is a bug in the pest grammar
    /// recursively parse a pair as a Step
    fn step(pair: Pair<Rule>) -> Result<Step, String> {
        match pair.as_rule() {
            Rule::variable => Self::variable(pair),
            Rule::single => Ok(Self::single(pair)?),
            Rule::repeat => Ok(Step::Static(Static::Repeat)),
            Rule::subdivision | Rule::mini => Self::group(pair, Step::subdivision),
            Rule::alternating => Self::group(pair, Step::alternating),
            Rule::polymeter => Self::polymeter(pair),
            Rule::range => Self::range(pair),
            Rule::expression => Self::expression(pair),
            _ => Err(format!(
                "unexpected rule, this is a bug in the parser\n{:?}",
                pair
            )),
        }
    }

    fn variable(pair: Pair<Rule>) -> Result<Step, String> {
        pair.into_inner()
            .next()
            .ok_or_else(|| "error in grammar, missing variable name".to_string())
            .map(|name_pair| Rc::from(name_pair.as_str()))
            .map(Step::Var)
    }

    fn variable_target(
        pair: Pair<Rule>,
        target_name: &str,
        kind: TargetKind,
    ) -> Result<Step, String> {
        let name = Rc::from(target_name);
        pair.clone()
            .into_inner()
            .next()
            .map(|variable_pair| {
                Ok(Step::TargetExpression(TargetExpression {
                    step: Box::new(Step::Single(Single {
                        value: Constant::Null,
                        string: Rc::clone(&name),
                    })),
                    kind: Some(kind),
                    target: Box::new(Self::variable(variable_pair)?),
                }))
            })
            .ok_or_else(|| format!("error in grammar, unexpected rule for variable\n{pair:?}"))?
    }

    /// parse a pair inside a single as a value
    fn single(pair: Pair<Rule>) -> Result<Step, String> {
        let pair = pair
            .clone()
            .into_inner()
            .next()
            .ok_or_else(|| format!("empty single {}", pair))?;

        let string = Rc::from(pair.as_str());

        let constant =
            match pair.as_rule() {
                // Rule::variable => Self::variable(pair),
                Rule::target => {
                    let name = pair.as_str().get(0..1).ok_or_else(|| {
                        format!("error in grammar, missing target key in pair\n{:?}", pair)
                    })?;
                    let value = pair.clone().into_inner().next().ok_or_else(|| {
                        format!("error in grammar, missing target value in pair\n{:?}", pair)
                    })?;

                    match name.as_bytes() {
                        b"#" => match value.as_rule() {
                            Rule::integer => Constant::Target(Target::Index(
                                Constant::parse_integer(value.as_str())?,
                            )),
                            Rule::variable => {
                                return Self::variable_target(pair, name, TargetKind::Index)
                            }
                            _ => {
                                return Err("error in grammar, unexpected rule for target index"
                                    .to_string())
                            }
                        },
                        _ => match value.as_rule() {
                            Rule::float => Constant::Target(Target::NamedFloat(
                                Rc::from(name),
                                Constant::parse_float(value.as_str())?,
                            )),
                            Rule::variable => {
                                return Self::variable_target(
                                    pair,
                                    name,
                                    TargetKind::Named(Rc::from(name)),
                                )
                            }
                            _ => {
                                return Err("error in grammar, unexpected rule for target float"
                                    .to_string())
                            }
                        },
                    }
                }
                _ => match pair.as_rule() {
                    Rule::integer => Constant::from_integer(pair.as_str())?,
                    Rule::float => Constant::from_float(pair.as_str())?,
                    Rule::number => {
                        if let Some(n) = pair.into_inner().next() {
                            match n.as_rule() {
                                Rule::integer => Constant::from_integer(n.as_str())?,
                                Rule::float => Constant::from_float(n.as_str())?,
                                _ => return Err(format!("unrecognized number\n{:?}", n)),
                            }
                        } else {
                            return Err("empty single".to_string());
                        }
                    }
                    Rule::hold => Constant::Hold,
                    Rule::rest => Constant::Rest,
                    Rule::pitch => Constant::Pitch(Pitch::parse(pair)),
                    Rule::chord => {
                        let mut pitch = Pitch { note: 0, octave: 4 };
                        let mut mode = "";
                        for p in pair.into_inner() {
                            match p.as_rule() {
                                Rule::pitch => {
                                    pitch = Pitch::parse(p);
                                }
                                Rule::mode => {
                                    mode = p.as_str();
                                }
                                _ => (),
                            }
                        }
                        Constant::Chord(pitch, Rc::from(mode))
                    }
                    Rule::name => Constant::Name(Rc::from(pair.as_str())),
                    _ => return Err(format!("unrecognized target value\n{:?}", pair)),
                },
            };

        Ok(Step::Single(Single {
            value: constant,
            string,
        }))
    }

    /// transform static steps into their final form and push them onto a list
    fn push_applied(steps: &mut Vec<Step>, step: Step) {
        match &step {
            Step::Static(s) => match s {
                Static::Repeat => {
                    let repeat = steps.last().cloned().unwrap_or(Step::rest());
                    steps.push(repeat)
                }
                Static::Range(r) => {
                    let range = if r.start <= r.end {
                        Box::new(r.start..=r.end) as Box<dyn Iterator<Item = i32>>
                    } else {
                        Box::new((r.end..=r.start).rev()) as Box<dyn Iterator<Item = i32>>
                    };
                    for i in range {
                        steps.push(Step::Single(Single {
                            value: Constant::Integer(i),
                            string: Rc::from(i.to_string()),
                        }))
                    }
                }
            },
            _ => steps.push(step),
        }
    }

    /// helper to split a list of pairs over a rule, used for stacks and split shorthand
    fn split_over(pairs: Vec<Pair<Rule>>, rule: Rule) -> Vec<Vec<Pair<Rule>>> {
        pairs.into_iter().fold(vec![vec![]], |mut a, p| {
            if p.as_rule() == rule {
                a.push(vec![])
            } else {
                a.last_mut()
                    .expect("we start the fold with one vec inside")
                    .push(p)
            }
            a
        })
    }

    fn with_choices(pairs: Vec<Pair<Rule>>) -> Result<Vec<Step>, String> {
        let mut choiced_pairs: Vec<Vec<Pair<Rule>>> = vec![];

        let mut is_choice = false;
        for p in pairs.into_iter().filter(|p| p.as_rule() != Rule::EOI) {
            if p.as_rule() == Rule::choice_op {
                is_choice = true;
            } else if is_choice {
                let last = choiced_pairs
                    .last_mut()
                    .ok_or("this can never happen as '|' can never start a section")?;
                last.push(p);
                is_choice = false
            } else {
                choiced_pairs.push(vec![p])
            }
        }

        choiced_pairs
            .into_iter()
            .map(|vs| {
                if let Some(first) = vs.first() {
                    if vs.len() > 1 {
                        Ok(Step::Choices(Choices {
                            choices: Self::section_vec(vs)?,
                        }))
                    } else {
                        Self::step(first.clone())
                    }
                } else {
                    Ok(Step::rest())
                }
            })
            .collect()
    }

    fn section_vec(pairs: Vec<Pair<Rule>>) -> Result<Vec<Step>, String> {
        let choiced_steps = Self::with_choices(pairs)?;
        let mut steps = Vec::with_capacity(choiced_steps.len());
        for step in choiced_steps.into_iter() {
            Self::push_applied(&mut steps, step)
        }
        Ok(steps)
    }

    fn section(pairs: Vec<Pair<Rule>>) -> Result<Vec<Step>, String> {
        let split_pairs = Self::split_over(pairs, Rule::split_op)
            .into_iter()
            .map(Self::section_vec)
            .collect::<Result<Vec<Vec<Step>>, String>>()?;

        Ok(if split_pairs.len() > 1 {
            split_pairs.into_iter().map(Step::subdivision).collect()
        } else {
            split_pairs.first().unwrap_or(&vec![]).to_owned()
        })
    }

    fn stacks(pairs: Vec<Pair<Rule>>) -> Result<Vec<Vec<Step>>, String> {
        let mut stacks = Self::split_over(pairs, Rule::stack_op)
            .into_iter()
            .map(Self::section)
            .collect::<Result<Vec<Vec<Step>>, String>>()?;
        stacks.retain(|s| !s.is_empty());
        Ok(stacks)
    }

    fn group(pair: Pair<Rule>, fun: fn(Vec<Step>) -> Step) -> Result<Step, String> {
        let stacks = Self::stacks(pair.into_inner().collect())?;

        match stacks.len() {
            0 => Ok(Step::rest()),
            1 => {
                let steps = stacks.first().unwrap();
                if steps.is_empty() {
                    Ok(Step::rest())
                } else {
                    Ok(fun(steps.to_owned()))
                }
            }
            _ => Ok(Step::Stack(Stack {
                stack: stacks.into_iter().map(fun).collect(),
            })),
        }
    }

    fn polymeter_tail(pair: Pair<Rule>) -> Result<Step, String> {
        if let Some(count) = pair.clone().into_inner().next() {
            Self::step(count)
        } else {
            Err(format!("missing polymeter count '{}'", pair.as_str()))
        }
    }

    fn polymeter(pair: Pair<Rule>) -> Result<Step, String> {
        let (stacked_pairs, count_pairs): (Vec<Pair<Rule>>, Vec<Pair<Rule>>) = pair
            .into_inner()
            .partition(|p| p.as_rule() != Rule::polymeter_tail);

        let count: Option<Step> = if let Some(pair) = count_pairs.first() {
            Some(Self::polymeter_tail(pair.to_owned())?)
        } else {
            None
        };

        let stacks = Self::stacks(stacked_pairs)?;

        let (stack, steps): (Option<Vec<Vec<Step>>>, Option<Vec<Step>>) =
            if let Some(first) = stacks.first() {
                if stacks.len() > 1 {
                    (Some(stacks), None)
                } else {
                    (None, Some(first.to_owned()))
                }
            } else {
                (None, None)
            };

        match (count, stack, steps) {
            (Some(count), None, Some(steps)) => {
                // a regular polymeter with explicit count
                Ok(Step::polymeter(steps, count))
            }
            (Some(count), Some(stack), _) => {
                // sections in a stack with explicit count will all have that
                Ok(Step::Stack(Stack {
                    stack: stack
                        .into_iter()
                        .map(|steps| Step::polymeter(steps, count.clone()))
                        .collect(),
                }))
            }
            (None, Some(stack), _) => {
                let count = stack
                    .first()
                    .map(Vec::len)
                    .ok_or_else(|| format!("empty stack {:?}", stack))?;

                if stack.len() > 1 && count > 0 {
                    let count = Step::Single(Single {
                        value: Constant::Integer(count as i32),
                        string: Rc::from(count.to_string()),
                    });
                    // if there is a stack but no count, the first section will determine the count of the rest
                    Ok(Step::Stack(Stack {
                        stack: stack
                            .into_iter()
                            .map(|steps| Step::polymeter(steps, count.clone()))
                            .collect(),
                    }))
                } else {
                    // unreachable, a stack will always have more than one sections with each having at least one item
                    Err(format!("invalid stack {:?}", stack))
                }
            }
            // if there is only one section and no count, it is treated as a subdivision
            (None, None, Some(steps)) => Ok(Step::subdivision(steps)),
            // empty polymeter like {} and {}%2 will become a single rest
            _ => Ok(Step::rest()),
        }
    }

    fn range(pair: Pair<Rule>) -> Result<Step, String> {
        let mut inner = pair.clone().into_inner();
        let start_pair = inner
            .next()
            .ok_or_else(|| format!("empty expression\n{:?}", pair))?;
        let start = start_pair.as_str().parse::<i32>().map_err(|_| {
            format!(
                "range expected integer on the left side, got '{}'",
                start_pair.as_str()
            )
        })?;

        let end_pair = inner.next().ok_or("range expression has no right side")?;
        let end = end_pair.as_str().parse::<i32>().map_err(|_| {
            format!(
                "range expected integer on the right side, got '{}'",
                end_pair.as_str()
            )
        })?;
        Ok(Step::Static(Static::Range(Range { start, end })))
    }

    fn bjorklund(left: Step, op_pair: Pair<Rule>) -> Result<Step, String> {
        let mut inner = op_pair.clone().into_inner();

        let steps = inner
            .next()
            .ok_or_else(|| format!("no steps in bjorklund\n{:?}", op_pair))
            .and_then(Self::step)?;

        let pulses = inner
            .next()
            .ok_or_else(|| format!("no pulse in bjorklund\n{:?}", op_pair))
            .and_then(Self::step)?;

        let rotate = inner.next().map(Self::step).transpose()?;

        Ok(Step::Bjorklund(Bjorklund {
            left: Box::new(left),
            pulses: Box::new(pulses),
            steps: Box::new(steps),
            rotation: rotate.map(Box::new),
        }))
    }

    fn invalid_right_hand() -> String {
        String::from("unreachable: missing right hand side from op_pair, error in grammar!")
    }

    fn optional_right_hand(op_pair: Pair<Rule>, default: fn() -> Step) -> Result<Step, String> {
        if let Some(pair) = op_pair.into_inner().next() {
            match pair.as_rule() {
                Rule::single => Self::single(pair),
                Rule::variable => Self::variable(pair),
                _ => Err("error in grammar".to_string()),
            }
        } else {
            Ok(default())
        }
    }

    // TODO allow for a pattern on the right for weight and replicate
    // with the current pest setup, this seems impossible if we want to support optional parameter here
    // at least it is impossible without major rearrangement of the grammar and parsing
    fn weight_expression(left: Step, op_pair: Pair<Rule>) -> Result<Step, String> {
        let weight = Self::optional_right_hand(op_pair, || {
            Step::constant(Constant::Float(2.0), Some("2.0"))
        })?;

        Ok(Step::WeightExpression(WeightExpression {
            step: Box::new(left),
            weight: Box::new(weight),
        }))
    }

    fn replicate_expression(left: Step, op_pair: Pair<Rule>) -> Result<Step, String> {
        let count = Self::optional_right_hand(op_pair, || {
            Step::constant(Constant::Float(2.0), Some("2.0"))
        })?;

        Ok(Step::ReplicateExpression(ReplicateExpression {
            step: Box::new(left),
            count: Box::new(count),
        }))
    }

    fn degrade_expression(step: Step, op_pair: Pair<Rule>) -> Result<Step, String> {
        let chance = Self::optional_right_hand(op_pair, || {
            Step::constant(Constant::Float(0.5), Some("0.5"))
        })?;

        Ok(Step::Degrade(Degrade {
            step: Box::new(step),
            chance: Box::new(chance),
        }))
    }

    fn speed_expression(left: Step, op: SpeedOp, op_pair: Pair<Rule>) -> Result<Step, String> {
        let right = op_pair
            .into_inner()
            .next()
            .ok_or_else(Self::invalid_right_hand)
            .and_then(Self::step)?;
        Ok(Step::SpeedExpression(SpeedExpression {
            step: Box::new(left),
            mult: Box::new(right),
            op,
        }))
    }

    fn target_expression(left: Step, op_pair: Pair<Rule>) -> Result<Step, String> {
        let right = op_pair
            .into_inner()
            .next()
            .ok_or_else(Self::invalid_right_hand)?;

        let (kind, target) = match right.as_rule() {
            Rule::target_assign => {
                let mut right = right.into_inner();

                let target_name_pair =
                    right.next().ok_or("error in grammar, missing target key")?;
                if target_name_pair.as_rule() != Rule::target_name {
                    return Err("error in grammar, expected target_name".to_string());
                }

                let p = right.next().ok_or("missing step pattern")?;
                let mut target_name = target_name_pair.into_inner();
                if let Some(target_name) = target_name.next() {
                    // target name was specified
                    (Some(TargetKind::Named(Rc::from(target_name.as_str()))), p)
                } else {
                    // # was used as indexed target
                    (Some(TargetKind::Index), p)
                }
            }
            _ => (None, right),
        };

        Ok(Step::TargetExpression(TargetExpression {
            step: Box::new(left),
            kind,
            target: Box::new(Self::step(target)?),
        }))
    }

    fn expression(pair: Pair<Rule>) -> Result<Step, String> {
        let mut inner = pair.clone().into_inner();
        // Initialize 'left' with the first step (single or group).
        let mut left = Self::step(
            inner
                .next()
                .ok_or_else(|| format!("empty expression\n{:?}", pair))?,
        )?;
        // Loop over operators and parameters, creating a nested expression if multiple pairs are present
        for op_pair in inner {
            left = match Operator::parse(op_pair.clone())? {
                Operator::Replicate() => Self::replicate_expression(left, op_pair)?,
                Operator::Weight() => Self::weight_expression(left, op_pair)?,
                Operator::Speed(op) => Self::speed_expression(left, op, op_pair)?,
                Operator::Target() => Self::target_expression(left, op_pair)?,
                Operator::Degrade() => Self::degrade_expression(left, op_pair)?,
                Operator::Bjorklund() => Self::bjorklund(left, op_pair)?,
            }
        }
        Ok(left)
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct CycleState {
    iteration: u32,
    rng: Xoshiro256PlusPlus,
    events: usize,
}

impl Cycle {
    fn output_span(
        step: &Step,
        state: &mut CycleState,
        span: &Span,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Events, String> {
        let range = span.whole_range();
        let mut cycles = Vec::with_capacity(range.clone().count());
        for cycle in range {
            let span = Span::new(
                Fraction::from_u32(cycle).ok_or(OVERFLOW_ERROR)?,
                Fraction::from_u32(cycle + 1).ok_or(OVERFLOW_ERROR)?,
            );
            let mut events = Self::output(step, state, cycle, limit, overlap, vars)?;
            events.transform_spans(&span);
            cycles.push(events)
        }
        let mut events = Events::Multi(MultiEvents {
            span: span.clone(),
            length: span.length(),
            events: cycles,
        });
        events.crop(span, overlap);
        Ok(events)
    }

    fn output_multiplied(
        step: &Step,
        state: &mut CycleState,
        cycle: u32,
        mult: Fraction,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Events, String> {
        let span = Span::new(
            Fraction::from_u32(cycle).ok_or(OVERFLOW_ERROR)? * mult,
            Fraction::from_u32(cycle + 1).ok_or(OVERFLOW_ERROR)? * mult,
        );
        let mut events = Self::output_span(step, state, &span, limit, overlap, vars)?;
        events.normalize_spans(&span);
        Ok(events)
    }

    fn step_length(
        step: &Step,
        state: &mut CycleState,
        cycle: u32,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Fraction, String> {
        let right_events = Self::output(step, state, cycle, limit, overlap, vars)?;
        Ok(right_events
            .first()
            .and_then(|e| e.value.to_fraction())
            .unwrap_or(Fraction::ONE))
    }

    // helper to calculate the right multiplier for polymeter and speed expressions
    fn step_multiplier(
        step: &Step,
        value: &Constant,
        state: &mut CycleState,
        cycle: u32,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Fraction, String> {
        let mult = match step {
            Step::Polymeter(pm) => {
                let length = if let Step::Subdivision(s) = pm.steps.as_ref() {
                    let mut a = Fraction::ZERO;
                    for v in s.steps.iter() {
                        let inner_length = match v {
                            Step::ReplicateExpression(re) => Self::step_length(
                                re.count.as_ref(),
                                state,
                                cycle,
                                limit,
                                overlap,
                                vars,
                            )?,
                            _ => Fraction::ONE,
                        };
                        a += inner_length
                    }
                    a
                } else {
                    // unreachable
                    Fraction::ONE
                };

                value.to_fraction().unwrap_or(Fraction::ZERO) / length
            }
            Step::SpeedExpression(e) => match e.op {
                SpeedOp::Fast() => value.to_fraction().unwrap_or(Fraction::ZERO),
                SpeedOp::Slow() => value
                    .to_float()
                    .and_then(|div| {
                        if div != 0.0 {
                            Fraction::from_f64(1.0 / div)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Fraction::ZERO),
            },
            _ => Fraction::from(1),
            // _ => value
            //     .to_float()
            //     .and_then(Fraction::from_f64)
            //     .unwrap_or(Fraction::ONE),
        };
        Ok(mult)
    }

    // overlay two lists of events and apply the targets from the second to the first
    fn apply_targets(events: &mut [Event], target_events: &[Event], kind: Option<&TargetKind>) {
        for target_event in target_events.iter() {
            if let Some(target) = target_event.value.to_target(&target_event.string, kind) {
                for event in events.iter_mut() {
                    if event.span.overlaps(&target_event.span)
                        && !{
                            let this = &event;
                            let target: &Target = &target;
                            this.targets.iter().any(|t| t.equal_key(target))
                        }
                    {
                        event.targets.push(target.clone());
                    }
                }
            }
            // add all targets of the target value too, if there are any
            if !target_event.targets.is_empty() {
                for event in events.iter_mut() {
                    if event.span.overlaps(&target_event.span) {
                        for target in &target_event.targets {
                            if !{
                                let this = &event;
                                this.targets.iter().any(|t| t.equal_key(target))
                            } {
                                event.targets.push(target.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // helper to output a Step as channels of flat event lists
    fn output_flat(
        step: &Step,
        state: &mut CycleState,
        cycle: u32,
        limit: usize,
        vars: Option<&Vars>,
    ) -> Result<(Vec<Vec<Event>>, Span), String> {
        let mut events = Self::output(step, state, cycle, limit, true, vars)?;
        events.transform_spans(&events.get_span());
        let mut channels = vec![];
        events.flatten(&mut channels, &mut 0);
        Events::merge(&mut channels);
        Ok((channels, events.get_span()))
    }

    // generate events from Target expressions
    fn output_with_target(
        exp: &TargetExpression,
        state: &mut CycleState,
        cycle: u32,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Events, String> {
        let (step, target_kind, target_step) =
            (exp.step.as_ref(), exp.kind.as_ref(), exp.target.as_ref());

        match target_step {
            // assign single value to avoid generating events
            Step::Single(single) => {
                let mut events = Self::output(step, state, cycle, limit, overlap, vars)?;
                if let Some(target) = single.value.to_target(&single.string, target_kind) {
                    events.mutate_events(&mut |event: &mut Event| {
                        if !{
                            let this = &event;
                            let target: &Target = &target;
                            this.targets.iter().any(|t| t.equal_key(target))
                        } {
                            event.targets.push(target.clone());
                        }
                    });
                }
                Ok(events)
            }
            _ => {
                // generate all the events as flat vecs from both the left and right side of the expression
                let (left_channels, left_span) =
                    Self::output_flat(step, state, cycle, limit, vars)?;
                let (target_channels, _) =
                    Self::output_flat(target_step, state, cycle, limit, vars)?;

                // iterate over channels from both sides to create necessary new stacks if the right side is polyphonic
                let mut channel_events: Vec<Events> = Vec::with_capacity(target_channels.len());
                for channel in target_channels.into_iter() {
                    for left_channel in left_channels.iter() {
                        let mut cloned_left = left_channel.clone();
                        Self::apply_targets(&mut cloned_left, &channel, target_kind);
                        channel_events.push(Events::Multi(MultiEvents {
                            length: left_span.length(),
                            span: left_span.clone(),
                            events: cloned_left.into_iter().map(Events::Single).collect(),
                        }));
                    }
                }
                // put all the resulting events back together
                Ok(Events::maybe_poly(PolyEvents {
                    length: left_span.length(),
                    span: left_span,
                    channels: channel_events,
                }))
            }
        }
    }

    // output a multiplied pattern expression with support for patterns on the right side
    fn output_with_speed(
        right: &Step,
        step: &Step,
        state: &mut CycleState,
        cycle: u32,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Events, String> {
        let left = match step {
            Step::Polymeter(pm) => pm.steps.as_ref(),
            Step::SpeedExpression(exp) => exp.step.as_ref(),
            _ => step,
        };
        match right {
            // multiply with single values to avoid generating events
            Step::Single(single) => {
                // apply multiplier
                let multiplier =
                    Self::step_multiplier(step, &single.value, state, cycle, limit, overlap, vars)?;
                Ok(Self::output_multiplied(
                    left, state, cycle, multiplier, limit, overlap, vars,
                )?)
            }
            _ => {
                // generate and flatten the events for the right side of the expression
                let events = Self::output(right, state, cycle, limit, overlap, vars)?;
                let channels = events.export();

                // extract a float to use as mult from each event and output the step with it
                let mut channel_events: Vec<Events> = Vec::with_capacity(channels.len());
                for channel in channels.into_iter() {
                    let mut multi_events: Vec<Events> = Vec::with_capacity(channel.len());
                    for event in channel {
                        // apply multiplier
                        let multiplier = Self::step_multiplier(
                            step,
                            &event.value,
                            state,
                            cycle,
                            limit,
                            overlap,
                            vars,
                        )?;
                        let mut partial_events = Self::output_multiplied(
                            left, state, cycle, multiplier, limit, overlap, vars,
                        )?;
                        // crop and push to multi events
                        partial_events.crop(&event.span, overlap);
                        multi_events.push(partial_events);
                    }
                    channel_events.push(Events::Multi(MultiEvents {
                        length: events.get_length(),
                        span: events.get_span(),
                        events: multi_events,
                    }));
                }

                // put all the resulting events back together
                Ok(Events::maybe_poly(PolyEvents {
                    length: events.get_length(),
                    span: events.get_span(),
                    channels: channel_events,
                }))
            }
        }
    }

    // recursively output events for the entire cycle based on some state (random seed)
    fn output(
        step: &Step,
        state: &mut CycleState,
        cycle: u32,
        limit: usize,
        overlap: bool,
        vars: Option<&Vars>,
    ) -> Result<Events, String> {
        let events = match step {
            Step::Var(name) => {
                if let Some(vars) = vars {
                    if let Some(step) = vars.get(name) {
                        Self::output(step, state, cycle, limit, overlap, Some(vars))?
                    } else {
                        Events::named(name)
                    }
                } else {
                    Events::named(name)
                }
            }
            Step::Single(s) => {
                state.events += 1;
                if state.events > limit {
                    return Err(format!(
                        "the cycle's event limit of {} was exceeded!",
                        limit
                    ));
                }
                Events::Single(Event {
                    length: Fraction::ONE,
                    span: Span::default(),
                    string: Rc::clone(&s.string),
                    value: s.value.clone(),
                    targets: vec![],
                })
            }
            Step::Subdivision(sd) => {
                if sd.steps.is_empty() {
                    Events::empty()
                } else {
                    let mut events = Vec::with_capacity(sd.steps.len());
                    for s in &sd.steps {
                        let e = Self::output(s, state, cycle, limit, overlap, vars)?;
                        events.push(e)
                    }

                    Events::subdivide_lengths(&mut events);
                    Events::Multi(MultiEvents {
                        span: Span::default(),
                        length: Fraction::ONE,
                        events,
                    })
                }
            }
            Step::WeightExpression(we) => {
                let weight = Self::output(we.weight.as_ref(), state, cycle, limit, overlap, vars)?
                    .first()
                    .and_then(|e| e.value.to_fraction())
                    .unwrap_or(Fraction::ONE);

                let mut events =
                    Self::output(we.step.as_ref(), state, cycle, limit, overlap, vars)?;
                events.set_length(weight);
                events
            }
            Step::ReplicateExpression(we) => {
                let count = Self::output(we.count.as_ref(), state, cycle, limit, overlap, vars)?
                    .first()
                    .and_then(|e| e.value.to_float())
                    .unwrap_or(2.0);

                let ceil = count.ceil();
                let len = if ceil == 0.0 { 1 } else { ceil as usize };
                let mult = count / ceil;

                let steps = vec![we.step.as_ref().clone(); len];
                let sub = Step::subdivision(steps);

                // TODO cache this if the right side is static
                let step = Step::SpeedExpression(SpeedExpression {
                    op: SpeedOp::Fast(),
                    step: Box::from(sub),
                    mult: Box::from(Step::Single(Single {
                        value: Constant::Float(mult),
                        string: Rc::from(""),
                    })),
                });

                let mut events = Self::output(&step, state, cycle, limit, overlap, vars)?;
                events.set_length(Fraction::from_f64(count).unwrap_or(Fraction::ONE));
                events
            }
            Step::Alternating(a) => {
                if a.steps.is_empty() {
                    Events::empty()
                } else {
                    let length = a.steps.len() as u32;
                    let current = cycle % length;
                    a.steps
                        .get(current as usize)
                        .map(|step| Self::output(step, state, cycle / length, limit, overlap, vars))
                        .unwrap_or(
                            Ok(Events::empty()), // unreachable
                        )?
                }
            }
            Step::Choices(cs) => {
                let choice = state.rng.random_range(0..cs.choices.len());
                Self::output(&cs.choices[choice], state, cycle, limit, overlap, vars)?
            }
            Step::Polymeter(pm) => Self::output_with_speed(
                pm.count.as_ref(),
                step,
                state,
                cycle,
                limit,
                overlap,
                vars,
            )?,
            Step::Stack(st) => {
                if st.stack.is_empty() {
                    Events::empty()
                } else {
                    let mut channels = Vec::with_capacity(st.stack.len());
                    for s in &st.stack {
                        channels.push(Self::output(s, state, cycle, limit, overlap, vars)?)
                    }
                    Events::maybe_poly(PolyEvents {
                        span: Span::default(),
                        length: Fraction::ONE,
                        channels,
                    })
                }
            }
            Step::Degrade(d) => {
                let chance = Self::output(d.chance.as_ref(), state, cycle, limit, overlap, vars)?
                    .first()
                    .and_then(|e| e.value.to_chance());

                let mut out = Self::output(d.step.as_ref(), state, cycle, limit, overlap, vars)?;
                out.mutate_events(&mut |event: &mut Event| {
                    if let Some(chance) = chance {
                        if chance < state.rng.random_range(0.0..1.0) {
                            event.value = Constant::Rest
                        }
                    }
                });
                out
            }
            Step::TargetExpression(e) => {
                Self::output_with_target(e, state, cycle, limit, overlap, vars)?
            }
            Step::SpeedExpression(e) => {
                Self::output_with_speed(e.mult.as_ref(), step, state, cycle, limit, overlap, vars)?
            }
            Step::Bjorklund(b) => {
                let mut events = vec![];

                let steps = Self::output(b.steps.as_ref(), state, cycle, limit, overlap, vars)?
                    .first()
                    .and_then(|e| e.value.to_integer())
                    .unwrap_or(0);
                let pulses = Self::output(b.pulses.as_ref(), state, cycle, limit, overlap, vars)?
                    .first()
                    .and_then(|e| e.value.to_integer())
                    .unwrap_or(0);
                let rotation = {
                    if let Some(r) = &b.rotation {
                        Self::output(r.as_ref(), state, cycle, limit, overlap, vars)?
                            .first()
                            .and_then(|e| e.value.to_integer())
                            .unwrap_or(0)
                    } else {
                        0
                    }
                };

                // TODO support something other than Step::Single as the right hand side
                events.reserve(pulses as usize);
                let out = Self::output(b.left.as_ref(), state, cycle, limit, overlap, vars)?;
                for pulse in euclidean(steps.max(0) as u32, pulses.max(0) as u32, rotation) {
                    if pulse {
                        events.push(out.clone())
                    } else {
                        events.push(Events::empty())
                    }
                }

                Events::subdivide_lengths(&mut events);
                Events::Multi(MultiEvents {
                    span: Span::default(),
                    length: Fraction::ONE,
                    events,
                })
            }

            Step::Static(_) => {
                // Repeat only makes it here if it had no preceding value
                // Range and Expression should be applied in Self::push_applied
                Events::empty()
            }
        };
        Ok(events)
    }

    #[cfg(test)]
    fn print_steps(step: &Step, level: usize) {
        let name = match step {
            Step::Var(name) => format!("Var {name:?}"),
            Step::Single(s) => match &s.value {
                Constant::Pitch(_p) => format!("{:?} {}", s.value, s.string),
                _ => format!("{:?} {:?}", s.value, s.string),
            },
            Step::Subdivision(sd) => format!("Subdivision [{}]", sd.steps.len()),
            Step::Alternating(a) => format!("Alternating <{}>", a.steps.len()),
            Step::Polymeter(pm) => format!("Polymeter {{{:?}}}", pm.count),
            Step::Choices(cs) => format!("Choices |{}|", cs.choices.len()),
            Step::Stack(st) => format!("Stack ({})", st.stack.len()),
            Step::SpeedExpression(e) => format!("Speed Expression {:?}", e.op),
            Step::WeightExpression(we) => format!("Weight Expression {:?}", we.weight),
            Step::ReplicateExpression(we) => format!("Replicate Expression {:?}", we.count),
            Step::TargetExpression(e) => format!("Target Expression {:?}", e.kind),
            Step::Static(s) => match s {
                Static::Repeat => "Repeat".to_string(),
                Static::Range(r) => format!("Range {}..{}", r.start, r.end),
            },
            Step::Degrade(d) => format!("Degrade ? {:?}", d.chance),
            Step::Bjorklund(_b) => format!("Bjorklund {}", ""),
        };
        println!("{} {}", indent_lines(level), name);
        for step in step.inner_steps() {
            Self::print_steps(step, level + 1)
        }
    }

    #[cfg(test)]
    fn print(&self) {
        Self::print_steps(&self.root, 0);
    }
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
fn indent_lines(level: usize) -> String {
    let mut lines = String::new();
    for i in 0..level {
        lines += [" │", " |"][i % 2];
    }
    lines
}

#[cfg(test)]
mod tests;
