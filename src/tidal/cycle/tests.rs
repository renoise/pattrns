use rand::Rng;

type Fraction = num_rational::Rational32;

use super::*;

use pretty_assertions::assert_eq;

fn assert_cycles(input: &str, outputs: Vec<Vec<Vec<Event>>>) -> Result<(), String> {
    let mut cycle = Cycle::from(input)?;
    for out in outputs {
        assert_eq!(cycle.generate()?, out, "with input: '{}'", input);
    }
    Ok(())
}

fn assert_cycle_equality(a: &str, b: &str) -> Result<(), String> {
    let seed = rand::rng().random();
    assert_eq!(
        Cycle::from(a)?.with_seed(seed).generate()?,
        Cycle::from(b)?.with_seed(seed).generate()?,
    );
    Ok(())
}

fn assert_cycle_advancing(input: &str) -> Result<(), String> {
    let seed = rand::rng().random();
    for number_of_runs in 1..9 {
        let mut cycle1 = Cycle::from(input)?.with_seed(seed);
        let mut cycle2 = Cycle::from(input)?.with_seed(seed);
        for _ in 0..number_of_runs {
            let _ = cycle1.generate()?;
            cycle2.advance();
        }
        assert_eq!(cycle1.generate()?, cycle2.generate()?);
    }
    Ok(())
}

fn cycle_with_vars(input: &str, subcycles: Vec<(&str, &str)>) -> Result<Cycle, String> {
    let mut cycle = Cycle::from(input)?;
    for (name, input) in subcycles {
        let subcycle = SubCycle::from(input)?;
        cycle.set_var(name, subcycle);
    }
    Ok(cycle)
}

#[test]
fn span() -> Result<(), String> {
    assert!(Span::new(Fraction::new(0, 1), Fraction::new(1, 1))
        .includes(&Span::new(Fraction::new(1, 2), Fraction::new(2, 1))));
    Ok(())
}

#[test]
fn weight_and_replicate() -> Result<(), String> {
    let mut cycle = Cycle::from("a!1.5 b")?;
    let _events = cycle.generate()?;
    Ok(())
}

#[test]
fn variables() -> Result<(), String> {
    assert!(SubCycle::from("a b c d").is_ok());
    // subcycles cannot contain variables
    assert!(SubCycle::from("[a b c d]*$mult").is_err());
    assert!(SubCycle::from("$note $note $note").is_err());
    assert!(SubCycle::from("a:p=<0.5 0.2 $right>").is_err());

    assert_eq!(
        cycle_with_vars("a b $note d", vec![("note", "e4")])?.generate(),
        Cycle::from("a b e4 d")?.generate()
    );

    // unset variables convert into named
    assert_eq!(
        Cycle::from("a $note")?.generate(),
        Cycle::from("a note")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a:$index", vec![("index", "12")])?.generate(),
        Cycle::from("a:12")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a*$mult", vec![("mult", "1 2")])?.generate(),
        Cycle::from("a*[1 2]")?.generate()
    );

    assert_eq!(
        cycle_with_vars("[a b c d]:p=[$f1 $f2]", vec![("f1", "0.5"), ("f2", "0.9")])?.generate(),
        Cycle::from("[a b c d]:p=[0.5 0.9]")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a*$mult", vec![("mult", "2.0")])?.generate(),
        Cycle::from("a*2")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a@$length b", vec![("length", "3.0")])?.generate(),
        Cycle::from("a@3 b")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a:p$float", vec![("float", "0.9")])?.generate(),
        Cycle::from("a:p0.9")?.generate()
    );

    assert_eq!(
        cycle_with_vars("[a b c d]:p=[$f1 $f2]", vec![("f1", "0.5"), ("f2", "0.9")])?.generate(),
        Cycle::from("[a b c d]:p=[0.5 0.9]")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a@$length b", vec![("length", "3.0")])?.generate(),
        Cycle::from("a _ _ b")?.generate()
    );

    assert_eq!(
        cycle_with_vars("a!$repeat b", vec![("repeat", "3.0")])?.generate(),
        Cycle::from("a a a b")?.generate()
    );

    assert_eq!(
        cycle_with_vars(
            "a*[[$int1 $int2!$repeat]*$mult]",
            vec![
                ("int1", "1"),
                ("int2", "2"),
                ("repeat", "2.0"),
                ("mult", "2.5")
            ]
        )?
        .generate(),
        Cycle::from("a*[[1 2 2]*2.5]")?.generate()
    );

    let mut cycle = cycle_with_vars("a!$repeat", vec![])?;
    let outputs = ["a", "a a", "a a a", "a a a a"];
    for (i, o) in outputs.iter().enumerate() {
        cycle.set_var("repeat", SubCycle::from(&(i + 1).to_string())?);
        assert_eq!(cycle.generate(), Cycle::from(o)?.generate());
    }

    let mut cycle = cycle_with_vars("$note", vec![])?;
    let mut static_cycle = Cycle::from("<a b c d>")?;
    for note in ["a", "b", "c", "d"] {
        cycle.set_var("note", SubCycle::from(note)?);
        assert_eq!(cycle.generate(), static_cycle.generate());
    }

    Ok(())
}

#[test]
fn polymeter() -> Result<(), String> {
    assert_eq!(
        Cycle::from("{0 1 2, 0 1 2 3}")?.generate(),
        Cycle::from("{0 1 2, 0 1 2 3}%3")?.generate(),
    );

    assert_eq!(
        Cycle::from("{0 ! ! 1}%2")?.generate(),
        Cycle::from("{0 0 0 1}%2")?.generate(),
    );

    assert_eq!(
        Cycle::from("{0 1!2, 0 1 2 3}")?.generate(),
        Cycle::from("{0 1 1, 0 1 2 3}%3")?.generate(),
    );

    assert_eq!(
        Cycle::from("{a b c d}%3")?.generate(),
        Cycle::from("[a b c d]*0.75")?.generate(),
    );

    assert_cycles(
        "{-3 -2 -1 0 1 2 3}%4",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(-3),
                Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_int(-2),
                Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_int(-1),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_int(0),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(1),
                Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_int(2),
                Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_int(3),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_int(-3),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(-2),
                Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_int(-1),
                Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_int(0),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_int(1),
            ]],
        ],
    )?;

    assert_cycle_equality("{a b!2 c}%3", "{a b b c}%3")?;
    assert_cycle_equality("a b, {c d e}%2", "{a b, c d e}")?;
    assert_cycle_equality("{a b . c d . f g h}%2", "{[a b] [c d] [f g h]}%2")?;

    assert_cycles(
        "{0 1 2 3}%<2 3>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2)).with_int(0),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_int(1),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 3)).with_int(3),
                Event::at(Fraction::new(1, 3), Fraction::new(1, 3)).with_int(0),
                Event::at(Fraction::new(2, 3), Fraction::new(1, 3)).with_int(1),
            ]],
        ],
    )?;

    Ok(())
}

#[test]
fn constant_literals() -> Result<(), String> {
    assert_eq!(
        Cycle::constant_from("c3")?,
        Constant::Pitch(Pitch { note: 0, octave: 3 })
    );
    assert_eq!(
        Cycle::constant_from("v0.5")?,
        Constant::Target(Target::named_float("v", 0.5))
    );
    assert_eq!(Cycle::constant_from("1.0")?, Constant::Float(1.0));
    assert_eq!(Cycle::constant_from("3.75")?, Constant::Float(3.75));
    assert_eq!(Cycle::constant_from("8")?, Constant::Integer(8));
    assert_eq!(Cycle::constant_from("~")?, Constant::Rest);
    assert_eq!(Cycle::constant_from("_")?, Constant::Hold);
    assert_eq!(Cycle::constant_from("name")?, Constant::Name("name".into()));

    assert!(Cycle::constant_from("$param").is_err());
    assert!(Cycle::constant_from("[1 2 3]").is_err());
    assert!(Cycle::constant_from("<1 2 3>").is_err());
    assert!(Cycle::constant_from("{{a b c}}%2").is_err());
    assert!(Cycle::constant_from("2 _ 3").is_err());
    Ok(())
}

#[test]
fn parse() -> Result<(), String> {
    assert!(Cycle::from("a b c [d").is_err());
    assert!(Cycle::from("a b/ c [d").is_err());
    assert!(Cycle::from("a b--- c [d").is_err());
    assert!(Cycle::from("*a b c [d").is_err());
    assert!(Cycle::from("a {{{}}").is_err());
    assert!(Cycle::from("] a z [").is_err());
    assert!(Cycle::from("->err").is_err());
    assert!(Cycle::from("(a, b)").is_err());
    assert!(Cycle::from("#(12, 32)").is_err());
    assert!(Cycle::from("#c $").is_err());

    assert!(Cycle::from("c44'mode").is_err());
    assert!(Cycle::from("c4'!mode").is_err());
    assert!(Cycle::from("y'mode").is_err());
    assert!(Cycle::from("c4'mo'de").is_err());
    assert!(Cycle::from("_names_cannot_start_with_underscore").is_err());

    assert!(Cycle::from("c4'mode").is_ok());
    assert!(Cycle::from("c'm7#^-").is_ok());
    assert!(Cycle::from("[[[[[[[[]]]]]][[[[[]][[[]]]]]][[[][[[]]]]][[[[]]]]]]").is_ok());

    Ok(())
}

#[test]
fn targets() -> Result<(), String> {
    assert_eq!(
        Cycle::from("a:v=0.5")?.generate()?,
        [[Event::at(Fraction::from(0), Fraction::new(1, 1))
            .with_note(9, 4)
            .with_target(Target::named_float("v", 0.5))]]
    );

    assert_eq!(
        Cycle::from("a:1 b:target")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 2))
                .with_note(9, 4)
                .with_target(Target::from_index(1)),
            Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                .with_note(11, 4)
                .with_target(Target::from_name("target".into()))
        ]]
    );

    assert_cycles(
        "a:<1 2>",
        vec![
            vec![vec![Event::at(Fraction::from(0), Fraction::new(1, 1))
                .with_note(9, 4)
                .with_target(Target::from_index(1))]],
            vec![vec![Event::at(Fraction::from(0), Fraction::new(1, 1))
                .with_note(9, 4)
                .with_target(Target::from_index(2))]],
        ],
    )?;

    assert_cycles(
        "a:1:2:Target",
        vec![vec![vec![Event::at(
            Fraction::from(0),
            Fraction::new(1, 1),
        )
        .with_note(9, 4)
        .with_targets(vec![
            Target::from_index(1),
            Target::from_name("Target".into()),
        ])]]],
    )?;

    assert_cycles(
        "[a:1:2]:<3 4>",
        vec![
            vec![vec![Event::at(Fraction::from(0), Fraction::new(1, 1))
                .with_note(9, 4)
                .with_target(Target::from_index(1))]],
            vec![vec![Event::at(Fraction::from(0), Fraction::new(1, 1))
                .with_note(9, 4)
                .with_target(Target::from_index(1))]],
        ],
    )?;

    // target expression preserves the structure from the left side
    assert_eq!(
        Cycle::from("[a b c d]:[1 2 3]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4))
                .with_note(9, 4)
                .with_target(Target::from_index(1)),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4))
                .with_note(11, 4)
                .with_target(Target::from_index(1)),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4))
                .with_note(0, 4)
                .with_target(Target::from_index(2)),
            Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                .with_note(2, 4)
                .with_target(Target::from_index(3)),
        ]]
    );

    assert_cycles(
        "{<0 0 d#8:test> 1 <c d e>:0xB [<.5 0.95> 1.]}%3",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 3)).with_int(0),
                Event::at(Fraction::new(1, 3), Fraction::new(1, 3)).with_int(1),
                Event::at(Fraction::new(2, 3), Fraction::new(1, 3))
                    .with_note(0, 4)
                    .with_target(Target::from_index(0xB)),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 6)).with_float(0.5),
                Event::at(Fraction::new(1, 6), Fraction::new(1, 6)).with_float(1.0),
                Event::at(Fraction::new(1, 3), Fraction::new(1, 3)).with_int(0),
                Event::at(Fraction::new(2, 3), Fraction::new(1, 3)).with_int(1),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 3))
                    .with_note(2, 4)
                    .with_target(Target::from_index(0xB)),
                Event::at(Fraction::new(2, 6), Fraction::new(1, 6)).with_float(0.95),
                Event::at(Fraction::new(3, 6), Fraction::new(1, 6)).with_float(1.0),
                Event::at(Fraction::new(2, 3), Fraction::new(1, 3))
                    .with_note(3, 8)
                    .with_target(Target::from_name("test".into())),
            ]],
        ],
    )?;

    assert_cycles(
        "[1 2] [3 4,[5 6]:42]",
        vec![vec![
            vec![
                Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(1),
                Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_int(2),
                Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_int(3),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_int(4),
            ],
            vec![
                Event::at(Fraction::new(1, 2), Fraction::new(1, 4))
                    .with_int(5)
                    .with_target(Target::from_index(42)),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                    .with_int(6)
                    .with_target(Target::from_index(42)),
            ],
        ]],
    )?;

    assert_cycles(
        "[<1 10> <2 20>:a](2,5)",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 10)).with_int(1),
                Event::at(Fraction::new(1, 10), Fraction::new(1, 10))
                    .with_int(2)
                    .with_target(Target::from_name("a".into())),
                Event::at(Fraction::new(1, 5), Fraction::new(1, 5)),
                Event::at(Fraction::new(2, 5), Fraction::new(1, 10)).with_int(1),
                Event::at(Fraction::new(5, 10), Fraction::new(1, 10))
                    .with_int(2)
                    .with_target(Target::from_name("a".into())),
                Event::at(Fraction::new(3, 5), Fraction::new(2, 5)),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 10)).with_int(10),
                Event::at(Fraction::new(1, 10), Fraction::new(1, 10))
                    .with_int(20)
                    .with_target(Target::from_name("a".into())),
                Event::at(Fraction::new(1, 5), Fraction::new(1, 5)),
                Event::at(Fraction::new(2, 5), Fraction::new(1, 10)).with_int(10),
                Event::at(Fraction::new(5, 10), Fraction::new(1, 10))
                    .with_int(20)
                    .with_target(Target::from_name("a".into())),
                Event::at(Fraction::new(3, 5), Fraction::new(2, 5)),
            ]],
        ],
    )?;

    // when using ~ as a target, it's possible selectively skip overriding the outer target from within
    assert_cycles(
        "[a [b:<~ 7> b:<8 9>]]:[1 [2 3], 4]",
        vec![
            vec![
                vec![
                    Event::at(Fraction::from(0), Fraction::new(1, 2))
                        .with_note(9, 4)
                        .with_target(Target::from_index(1)),
                    Event::at(Fraction::new(1, 2), Fraction::new(1, 4))
                        .with_note(11, 4)
                        // this iteration lets the outer context set the target
                        .with_target(Target::from_index(2)),
                    Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(8)),
                ],
                vec![
                    Event::at(Fraction::from(0), Fraction::new(1, 2))
                        .with_note(9, 4)
                        .with_target(Target::from_index(4)),
                    Event::at(Fraction::new(1, 2), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(4)),
                    Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(8)),
                ],
            ],
            vec![
                vec![
                    Event::at(Fraction::from(0), Fraction::new(1, 2))
                        .with_note(9, 4)
                        .with_target(Target::from_index(1)),
                    Event::at(Fraction::new(1, 2), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(7)),
                    Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(9)),
                ],
                vec![
                    Event::at(Fraction::from(0), Fraction::new(1, 2))
                        .with_note(9, 4)
                        .with_target(Target::from_index(4)),
                    Event::at(Fraction::new(1, 2), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(7)),
                    Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                        .with_note(11, 4)
                        .with_target(Target::from_index(9)),
                ],
            ],
        ],
    )?;

    assert_cycles(
        "[a b]:<1 target>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_note(9, 4)
                    .with_target(Target::from_index(1)),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_note(11, 4)
                    .with_target(Target::from_index(1)),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_note(9, 4)
                    .with_target(Target::from_name("target".into())),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_note(11, 4)
                    .with_target(Target::from_name("target".into())),
            ]],
        ],
    )?;

    assert_cycle_equality(
        "[1 2 3 4]:v=[0.2 0.3 0.4 p.8]",
        "[1 2 3 4]:[v0.2 v0.3 v0.4 p.8]",
    )?;

    assert_cycle_equality("[1 2 3 4]:g=[0.1 10.]", "[1 2 3 4]:[g0.1 g10.]")?;

    assert_cycles(
        "[a:1 b]:<3 4>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_note(9, 4)
                    .with_target(Target::from_index(1)),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_note(11, 4)
                    .with_target(Target::from_index(3)),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_note(9, 4)
                    .with_target(Target::from_index(1)),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_note(11, 4)
                    .with_target(Target::from_index(4)),
            ]],
        ],
    )?;

    assert_eq!(
        Cycle::from("a:1 b:v0.1:v1.0:p1.0:g100.0")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 2))
                .with_note(9, 4)
                .with_target(Target::from_index(1)),
            Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                .with_note(11, 4)
                .with_targets(vec![
                    Target::named_float("v", 0.1),
                    // second v should not be applied
                    Target::named_float("p", 1.0),
                    Target::named_float("g", 100.0),
                ])
        ]]
    );

    // outer instrument values shouldn't override inner ones
    assert_eq!(
        Cycle::from("[a:#2 b]:#3")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 2))
                .with_note(9, 4)
                .with_target(Target::from_index(2)),
            Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                .with_note(11, 4)
                .with_target(Target::from_index(3)),
        ]]
    );

    assert_eq!(
        Cycle::from("a:1:#1 b:#1:1")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 2))
                .with_note(9, 4)
                .with_target(Target::from_index(1)),
            Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                .with_note(11, 4)
                .with_target(Target::Index(1)),
        ]]
    );

    Ok(())
}

#[test]
fn generate() -> Result<(), String> {
    assert_eq!(
        Cycle::from("[0x0] [0x1A] [0XA] [-0X5] [-0XA0] [-0Xaa]")?.generate()?,
        [[
            Event::at(Fraction::new(0, 6), Fraction::new(1, 6)).with_int(0x0),
            Event::at(Fraction::new(1, 6), Fraction::new(1, 6)).with_int(0x1a),
            Event::at(Fraction::new(2, 6), Fraction::new(1, 6)).with_int(0xa),
            Event::at(Fraction::new(3, 6), Fraction::new(1, 6)).with_int(-0x5),
            Event::at(Fraction::new(4, 6), Fraction::new(1, 6)).with_int(-0xa0),
            Event::at(Fraction::new(5, 6), Fraction::new(1, 6)).with_int(-0xaa),
        ]]
    );

    assert_eq!(
        Cycle::from("[0] [1] [1.01] [0.01] [0.] [.01]")?.generate()?,
        [[
            Event::at(Fraction::new(0, 6), Fraction::new(1, 6)).with_int(0),
            Event::at(Fraction::new(1, 6), Fraction::new(1, 6)).with_int(1),
            Event::at(Fraction::new(2, 6), Fraction::new(1, 6)).with_float(1.01),
            Event::at(Fraction::new(3, 6), Fraction::new(1, 6)).with_float(0.01),
            Event::at(Fraction::new(4, 6), Fraction::new(1, 6)).with_float(0.0),
            Event::at(Fraction::new(5, 6), Fraction::new(1, 6)).with_float(0.01),
        ]]
    );

    let empty_events: Vec<Vec<Event>> = vec![];
    assert_eq!(Cycle::from("a*[]")?.generate()?, empty_events);
    assert_eq!(Cycle::from("[c d]/0")?.generate()?, empty_events);
    assert_eq!(Cycle::from("[c d]*0")?.generate()?, empty_events);

    assert!(Cycle::from("[c d]/1000000000000").is_err()); // too large for fraction
    assert_eq!(
        Cycle::from("[c d]/1000000")?.generate()?,
        [[Event::at(Fraction::from(0), Fraction::new(1, 1)).with_note(0, 4)]]
    );

    assert_eq!(
        Cycle::from("a b c d")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4)).with_note(9, 4),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_note(11, 4),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_note(0, 4),
            Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_note(2, 4),
        ]]
    );
    assert_eq!(
        Cycle::from("\ta\r\n\tb\nc\n d\n\n")?.generate()?,
        Cycle::from("a b c d")?.generate()?
    );
    assert_eq!(
        Cycle::from("a b [ c d ]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 3)).with_note(9, 4),
            Event::at(Fraction::new(1, 3), Fraction::new(1, 3)).with_note(11, 4),
            Event::at(Fraction::new(2, 3), Fraction::new(1, 6)).with_note(0, 4),
            Event::at(Fraction::new(5, 6), Fraction::new(1, 6)).with_note(2, 4),
        ]]
    );
    assert_eq!(
        Cycle::from("[a a] [b4 b5 b6] [c0 d1 c2 d3]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 6)).with_note(9, 4),
            Event::at(Fraction::new(1, 6), Fraction::new(1, 6)).with_note(9, 4),
            Event::at(Fraction::new(3, 9), Fraction::new(1, 9)).with_note(11, 4),
            Event::at(Fraction::new(4, 9), Fraction::new(1, 9)).with_note(11, 5),
            Event::at(Fraction::new(5, 9), Fraction::new(1, 9)).with_note(11, 6),
            Event::at(Fraction::new(8, 12), Fraction::new(1, 12)).with_note(0, 0),
            Event::at(Fraction::new(9, 12), Fraction::new(1, 12)).with_note(2, 1),
            Event::at(Fraction::new(10, 12), Fraction::new(1, 12)).with_note(0, 2),
            Event::at(Fraction::new(11, 12), Fraction::new(1, 12)).with_note(2, 3),
        ]]
    );
    assert_eq!(
        Cycle::from("[a0 [bb1 [b2 c3]]] c#4 [[[d5 D#6] E7 ] F8]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 6)).with_note(9, 0),
            Event::at(Fraction::new(1, 6), Fraction::new(1, 12)).with_note(10, 1),
            Event::at(Fraction::new(3, 12), Fraction::new(1, 24)).with_note(11, 2),
            Event::at(Fraction::new(7, 24), Fraction::new(1, 24)).with_note(0, 3),
            Event::at(Fraction::new(1, 3), Fraction::new(1, 3)).with_note(1, 4),
            Event::at(Fraction::new(2, 3), Fraction::new(1, 24)).with_note(2, 5),
            Event::at(Fraction::new(17, 24), Fraction::new(1, 24)).with_note(3, 6),
            Event::at(Fraction::new(9, 12), Fraction::new(1, 12)).with_note(4, 7),
            Event::at(Fraction::new(5, 6), Fraction::new(1, 6)).with_note(5, 8),
        ]]
    );
    assert_eq!(
        Cycle::from("[R [e [n o]]] , [[[i s] e ] _]")?.generate()?,
        vec![
            vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2)).with_name("R"),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 4)).with_note(4, 4),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 8)).with_name("n"),
                Event::at(Fraction::new(7, 8), Fraction::new(1, 8)).with_name("o"),
            ],
            vec![
                Event::at(Fraction::from(0), Fraction::new(1, 8)).with_name("i"),
                Event::at(Fraction::new(1, 8), Fraction::new(1, 8)).with_name("s"),
                Event::at(Fraction::new(1, 4), Fraction::new(3, 4)).with_note(4, 4),
            ],
        ]
    );

    assert_cycles(
        "<a b c d>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_note(9, 4)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_note(11, 4)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_note(0, 4)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_note(2, 4)
            ]],
        ],
    )?;

    assert_cycles(
        "<a ~ ~ a0> <b <c d>>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2)).with_note(9, 4),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_note(11, 4),
            ]],
            vec![vec![
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_note(0, 4)
            ]],
            vec![vec![
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_note(11, 4)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2)).with_note(9, 0),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_note(2, 4),
            ]],
        ],
    )?;

    assert_cycles(
        "<<a a8> b,  <c [d e]>>",
        vec![
            vec![
                vec![Event::at(Fraction::from(0), Fraction::from(1)).with_note(9, 4)],
                vec![Event::at(Fraction::from(0), Fraction::from(1)).with_note(0, 4)],
            ],
            vec![
                vec![Event::at(Fraction::from(0), Fraction::from(1)).with_note(11, 4)],
                vec![
                    Event::at(Fraction::from(0), Fraction::new(1, 2)).with_note(2, 4),
                    Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_note(4, 4),
                ],
            ],
            vec![
                vec![Event::at(Fraction::from(0), Fraction::from(1)).with_note(9, 8)],
                vec![Event::at(Fraction::from(0), Fraction::from(1)).with_note(0, 4)],
            ],
        ],
    )?;

    assert_eq!(
        Cycle::from("[1 middle _] {}%42 [] <>")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 12)).with_int(1),
            Event::at(Fraction::new(1, 12), Fraction::new(1, 6)).with_name("middle"),
            Event::at(Fraction::new(1, 4), Fraction::new(3, 4)),
        ]]
    );

    assert_eq!(
        Cycle::from("[1 __ 2] 3")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(3, 8)).with_int(1),
            Event::at(Fraction::new(3, 8), Fraction::new(1, 8)).with_int(2),
            Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_int(3),
        ]]
    );

    assert_cycles(
        "<some_name another_one c4'chord c4'-^7 c6a_name>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_name("some_name")
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_name("another_one")
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_chord(0, 4, "chord")
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_chord(0, 4, "-^7")
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_name("c6a_name")
            ]],
        ],
    )?;

    assert_eq!(
        Cycle::from("1 second*2 eb3*3 [32 32]*4")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(1),
            Event::at(Fraction::new(2, 8), Fraction::new(1, 8)).with_name("second"),
            Event::at(Fraction::new(3, 8), Fraction::new(1, 8)).with_name("second"),
            Event::at(Fraction::new(6, 12), Fraction::new(1, 12)).with_note(3, 3),
            Event::at(Fraction::new(7, 12), Fraction::new(1, 12)).with_note(3, 3),
            Event::at(Fraction::new(8, 12), Fraction::new(1, 12)).with_note(3, 3),
            Event::at(Fraction::new(24, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(25, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(26, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(27, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(28, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(29, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(30, 32), Fraction::new(1, 32)).with_int(32),
            Event::at(Fraction::new(31, 32), Fraction::new(1, 32)).with_int(32),
        ]]
    );

    assert_cycles(
        "tresillo(6,8), outside(4,11)",
        vec![vec![
            vec![
                Event::at(Fraction::from(0), Fraction::new(1, 8)).with_name("tresillo"),
                Event::at(Fraction::new(1, 8), Fraction::new(1, 8)),
                Event::at(Fraction::new(2, 8), Fraction::new(1, 8)).with_name("tresillo"),
                Event::at(Fraction::new(3, 8), Fraction::new(1, 8)).with_name("tresillo"),
                Event::at(Fraction::new(4, 8), Fraction::new(1, 8)).with_name("tresillo"),
                Event::at(Fraction::new(5, 8), Fraction::new(1, 8)),
                Event::at(Fraction::new(6, 8), Fraction::new(1, 8)).with_name("tresillo"),
                Event::at(Fraction::new(7, 8), Fraction::new(1, 8)).with_name("tresillo"),
            ],
            vec![
                Event::at(Fraction::from(0), Fraction::new(1, 11)).with_name("outside"),
                Event::at(Fraction::new(1, 11), Fraction::new(2, 11)),
                Event::at(Fraction::new(3, 11), Fraction::new(1, 11)).with_name("outside"),
                Event::at(Fraction::new(4, 11), Fraction::new(2, 11)),
                Event::at(Fraction::new(6, 11), Fraction::new(1, 11)).with_name("outside"),
                Event::at(Fraction::new(7, 11), Fraction::new(2, 11)),
                Event::at(Fraction::new(9, 11), Fraction::new(1, 11)).with_name("outside"),
                Event::at(Fraction::new(10, 11), Fraction::new(1, 11)),
            ],
        ]],
    )?;

    assert_eq!(
        Cycle::from("1!2 3 [4!3 5]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(1),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_int(1),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_int(3),
            Event::at(Fraction::new(12, 16), Fraction::new(1, 16)).with_int(4),
            Event::at(Fraction::new(13, 16), Fraction::new(1, 16)).with_int(4),
            Event::at(Fraction::new(14, 16), Fraction::new(1, 16)).with_int(4),
            Event::at(Fraction::new(15, 16), Fraction::new(1, 16)).with_int(5),
        ]]
    );

    assert_cycles(
        "[0 1]!2 <a b>!2",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 8)).with_int(0),
                Event::at(Fraction::new(1, 8), Fraction::new(1, 8)).with_int(1),
                Event::at(Fraction::new(2, 8), Fraction::new(1, 8)).with_int(0),
                Event::at(Fraction::new(3, 8), Fraction::new(1, 8)).with_int(1),
                Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_note(9, 4),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_note(9, 4),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 8)).with_int(0),
                Event::at(Fraction::new(1, 8), Fraction::new(1, 8)).with_int(1),
                Event::at(Fraction::new(2, 8), Fraction::new(1, 8)).with_int(0),
                Event::at(Fraction::new(3, 8), Fraction::new(1, 8)).with_int(1),
                Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_note(11, 4),
                Event::at(Fraction::new(3, 4), Fraction::new(1, 4)).with_note(11, 4),
            ]],
        ],
    )?;

    assert_cycles(
        "[0 1]/2",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 1)).with_int(0)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 1)).with_int(1)
            ]],
        ],
    )?;

    assert_cycles(
        "[0 1]*2.5",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 5)).with_int(0),
                Event::at(Fraction::new(1, 5), Fraction::new(1, 5)).with_int(1),
                Event::at(Fraction::new(2, 5), Fraction::new(1, 5)).with_int(0),
                Event::at(Fraction::new(3, 5), Fraction::new(1, 5)).with_int(1),
                Event::at(Fraction::new(4, 5), Fraction::new(1, 5)).with_int(0),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 5)).with_int(1),
                Event::at(Fraction::new(1, 5), Fraction::new(1, 5)).with_int(0),
                Event::at(Fraction::new(2, 5), Fraction::new(1, 5)).with_int(1),
                Event::at(Fraction::new(3, 5), Fraction::new(1, 5)).with_int(0),
                Event::at(Fraction::new(4, 5), Fraction::new(1, 5)).with_int(1),
            ]],
        ],
    )?;

    assert_eq!(
        Cycle::from("c(3,8,9)")?.generate()?,
        [[
            Event::at(Fraction::new(2, 8), Fraction::new(1, 8)).with_note(0, 4),
            Event::at(Fraction::new(3, 8), Fraction::new(1, 4)),
            Event::at(Fraction::new(5, 8), Fraction::new(1, 8)).with_note(0, 4),
            Event::at(Fraction::new(6, 8), Fraction::new(1, 8)),
            Event::at(Fraction::new(7, 8), Fraction::new(1, 8)).with_note(0, 4),
        ]]
    );

    assert_cycle_equality("a? b?", "a?0.5 b?0.5")?;
    assert_cycle_equality("[a b c](3,8,9)", "[a b c](3,8,1)")?;
    assert_cycle_equality("[a b c](3,8,7)", "[a b c](3,8,-1)")?;
    assert_cycle_equality("[a a a a]", "[a ! ! !]")?;
    assert_cycle_equality("[! ! a !]", "[~ ~ a a]")?;
    assert_cycle_equality("a ~ ~ ~", "a - - -")?;
    assert_cycle_equality("[a b] ! ! <a b c> !", "[a b] [a b] [a b] <a b c> <a b c>")?;
    assert_cycle_equality("0..3", "0 1 2 3")?;
    assert_cycle_equality("-5..-8", "-5 -6 -7 -8")?;
    assert_cycle_equality("a b . c d", "[a b] [c d]")?;
    assert_cycle_equality(
        "a b . c d e . f g h i [j k . l m]",
        "[a b] [c d e] [f g h i [[j k] [l m]]]",
    )?;
    assert_cycle_equality(
        "a b . c d e , f g h i . j k, l m",
        "[a b] [c d e], [[f g h i] [j k]], [l m]",
    )?;
    assert_cycle_equality("<a b . c d . f g h>", "<[a b] [c d] [f g h]>")?;

    assert_cycles(
        "[0 1 2]/2",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(2, 3)).with_int(0),
                Event::at(Fraction::new(2, 3), Fraction::new(1, 3)).with_int(1),
            ]],
            vec![vec![
                Event::at(Fraction::new(1, 3), Fraction::new(2, 3)).with_int(2)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(2, 3)).with_int(0),
                Event::at(Fraction::new(2, 3), Fraction::new(1, 3)).with_int(1),
            ]],
        ],
    )?;

    assert_cycles(
        "0*<1 2>",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::from(1)).with_int(0)
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2)).with_int(0),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_int(0),
            ]],
        ],
    )?;

    assert_eq!(
        Cycle::from("0*[4 3]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4)).with_int(0),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4)).with_int(0),
            Event::at(Fraction::new(2, 3), Fraction::new(1, 3)).with_int(0),
        ]]
    );

    // TODO test random outputs // parse_with_debug("[a b c d]?0.5");

    Ok(())
}

#[test]
fn expression_chains() -> Result<(), String> {
    assert_cycle_equality("a*3/2", "a*1.5")?;
    assert_cycle_equality("[a b c d]*2*4", "[a b c d]*8")?;
    assert_cycle_equality("a/2/3/4/5", "a/120")?;
    assert_cycle_equality(
        "[a b c d e f]:[[v.2 v.5]*3]",
        "[a b c d e f]:[v.2 v.5 v.2 v.5 v.2 v.5]",
    )?;
    assert_cycle_equality(
        "[a:0 b:0 c:1 d:1 e:2 f:2 g:3 h:3]/2*4",
        "[a b c d e f g h]:[0 1 2 3]/2*4",
    )?;
    assert_cycle_equality("[a:0 b:0 c:1 d:1]/2", "[a b c d]/2:<0 1>")?;

    assert_eq!(
        Cycle::from("[0 1]*2:[1 2 3 4]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4))
                .with_int(0)
                .with_target(Target::from_index(1)),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4))
                .with_int(1)
                .with_target(Target::from_index(2)),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4))
                .with_int(0)
                .with_target(Target::from_index(3)),
            Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                .with_int(1)
                .with_target(Target::from_index(4)),
        ]]
    );

    assert_cycles(
        "[a b c:v.2 d]:p.5:[v.1 v.3]:v.8/4",
        vec![
            vec![vec![Event::at(Fraction::from(0), Fraction::from(1))
                .with_note(9, 4)
                .with_targets(vec![
                    Target::named_float("p", 0.5),
                    Target::named_float("v", 0.1),
                ])]],
            vec![vec![Event::at(Fraction::from(0), Fraction::from(1))
                .with_note(11, 4)
                .with_targets(vec![
                    Target::named_float("p", 0.5),
                    Target::named_float("v", 0.1),
                ])]],
            vec![vec![Event::at(Fraction::from(0), Fraction::from(1))
                .with_note(0, 4)
                .with_targets(vec![
                    Target::named_float("v", 0.2),
                    Target::named_float("p", 0.5),
                ])]],
            vec![vec![Event::at(Fraction::from(0), Fraction::from(1))
                .with_note(2, 4)
                .with_targets(vec![
                    Target::named_float("p", 0.5),
                    Target::named_float("v", 0.3),
                ])]],
        ],
    )?;
    Ok(())
}

#[test]
fn event_limit() -> Result<(), String> {
    assert!(Cycle::from("[[a b c d]*100]*100")?.generate().is_err());
    assert!(Cycle::from("[[a b c d]*100]*100")?
        .with_event_limit(0x10000)
        .generate()
        .is_ok());
    Ok(())
}

#[test]
fn stacks() -> Result<(), String> {
    assert_eq!(
        Cycle::from("bd [bd, cp], ~ hh")?.generate()?,
        [
            vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2)).with_name("bd"),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_name("bd")
            ],
            vec![Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_name("cp")],
            vec![Event::at(Fraction::new(1, 2), Fraction::new(1, 2)).with_name("hh")]
        ]
    );
    Ok(())
}

#[test]
fn advancing() -> Result<(), String> {
    assert_cycle_advancing("[a b c d]")?; // stateless
    assert_cycle_advancing("[a b], [c d]")?;
    assert_cycle_advancing("{a b}%2 {a b}*5")?; // stateful
    assert_cycle_advancing("[a b]*5 [a b]/5")?;
    assert_cycle_advancing("[a b c d]<c d>")?;
    assert_cycle_advancing("a <b c>")?;
    assert_cycle_advancing("[a b? c d]|[c? d?]")?;
    assert_cycle_advancing("[{a b}/2 c d], <c d> e? {a b}*2")?;
    Ok(())
}

#[test]
fn target_assign() -> Result<(), String> {
    assert_cycle_equality(
        "[a b c [d e f g]]:[v.5 v.3 v.2 v.1]:[p.5 p.25 p.1 p.9]",
        "[a b c [d e f g]]:v=[.5 .3 .2 .1]:p=[.5 .25 .1 .9]",
    )?;
    assert_eq!(
        Cycle::from("[1 2 3 4]:p=[.1 .2 .3 .4]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4))
                .with_int(1)
                .with_target(Target::named_float("p", 0.1)),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4))
                .with_int(2)
                .with_target(Target::named_float("p", 0.2)),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4))
                .with_int(3)
                .with_target(Target::named_float("p", 0.3)),
            Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                .with_int(4)
                .with_target(Target::named_float("p", 0.4)),
        ]]
    );
    assert_eq!(
        Cycle::from("[1 2 3 4]:#=[1 c4 3 0.2]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4))
                .with_int(1)
                .with_target(Target::Index(1)),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4))
                .with_int(2)
                .with_target(Target::Index(48)),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4))
                .with_int(3)
                .with_target(Target::Index(3)),
            Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                .with_int(4)
                .with_target(Target::Index(0)),
        ]]
    );

    assert_eq!(
        Cycle::from("[1 2 3 4]:long=[1 _ ~ 0.2]")?.generate()?,
        [[
            Event::at(Fraction::from(0), Fraction::new(1, 4))
                .with_int(1)
                .with_target(Target::named_float("long", 1.0)),
            Event::at(Fraction::new(1, 4), Fraction::new(1, 4))
                .with_int(2)
                .with_target(Target::named_float("long", 1.0)),
            Event::at(Fraction::new(2, 4), Fraction::new(1, 4)).with_int(3),
            Event::at(Fraction::new(3, 4), Fraction::new(1, 4))
                .with_int(4)
                .with_target(Target::named_float("long", 0.2)),
        ]]
    );

    assert_cycles(
        "[1 2 3 4 5 6 7 8]/4:d=<.1 .2 .3 .4>:v=[<.3 .2 .1>*2/3]",
        vec![
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_int(1)
                    .with_targets(vec![
                        Target::named_float("d", 0.1),
                        Target::named_float("v", 0.3),
                    ]),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_int(2)
                    .with_targets(vec![
                        Target::named_float("d", 0.1),
                        Target::named_float("v", 0.3),
                    ]),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_int(3)
                    .with_targets(vec![
                        Target::named_float("d", 0.2),
                        Target::named_float("v", 0.3),
                    ]),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_int(4)
                    .with_targets(vec![
                        Target::named_float("d", 0.2),
                        Target::named_float("v", 0.2),
                    ]),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_int(5)
                    .with_targets(vec![
                        Target::named_float("d", 0.3),
                        Target::named_float("v", 0.2),
                    ]),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_int(6)
                    .with_targets(vec![
                        Target::named_float("d", 0.3),
                        Target::named_float("v", 0.2),
                    ]),
            ]],
            vec![vec![
                Event::at(Fraction::from(0), Fraction::new(1, 2))
                    .with_int(7)
                    .with_targets(vec![
                        Target::named_float("d", 0.4),
                        Target::named_float("v", 0.1),
                    ]),
                Event::at(Fraction::new(1, 2), Fraction::new(1, 2))
                    .with_int(8)
                    .with_targets(vec![
                        Target::named_float("d", 0.4),
                        Target::named_float("v", 0.1),
                    ]),
            ]],
        ],
    )?;

    Ok(())
}
