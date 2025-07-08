# pattrns

***pattrns*** is an experimental imperative-style music sequence generator engine. 

It allows you to programmatically create music sequences either in plain [Rust](https://www.rust-lang.org/) as library (*static, compiled*) or in [Lua](https://www.lua.org/) as a scripting engine (*dynamic, interpreted*). So it's also suitable for [live coding music](https://github.com/pjagielski/awesome-live-coding-music).

In addition to its imperative event generator approach, it also supports the creation of musical events using [tidalcycle](https://tidalcycles.org/)'s mini-notation.

This crate only deals with the *generation of raw musical events*. It does not generate audio. You must use an application with built-in support for pattrns to use it.


## Conceptional Overview

pattrns generates musical sequences using three distinct components, stages:

- **Rhythm**: (`pulse` in scripts) dynamic pulse generator to define a rhythmical pulse train.
- **Gate**: (`gate` in scripts) optional pulse filter between rhythm and event emitter.
- **Emitter**: (`event` in scripts), dynamic note or parameter event generator which gets triggered by the pulse train.

By separating the rhythmical from the tonal part of a musical sequence, each part can be freely modified, composed and (re)combined as it fits. 


## Documentation & Guides

Read the [Scripting Book](https://renoise.github.io/pattrns/).
It contains an introduction, guides, full Lua API documentation and bunch of script examples.

The Rust backend uses standard Rust documentation features. The docs are currently not hosted online (yet), but you can generate them locally via `cargo doc --open`.

## Applications

- [`Online Playground`](https://pattrns.renoise.com) is a simple browser based playground app. It allows you to learn and test how pattrns works.

- [`Renoise`](https://www.renoise.com) uses pattrns in its [phrase editor](https://tutorials.renoise.com/wiki/Phrase_Editor).

## Integration Examples

- [`examples/play.rs`](./examples/play.rs) demonstrates how to use patterns using only Rust: it defines and plays a little music thing. The content can only be changed at compile time.

- [`examples/play-script.rs`](./examples/play-script.rs) is an example using the Lua API: it also defines and plays a little music thing, but its content can be added/removed and changed on the fly to do some basic live music hacking.  


## Acknowledgements

Thanks to [unlessgames](https://github.com/unlessgames) for adding the Tidal Cycles mini-notation support.


## Contribute

Patches are welcome! Please fork the latest git repository and create a feature or bugfix branch.


## Licence

pattrns is distributed under the terms of the [GNU Affero General Public License V3](https://www.gnu.org/licenses/agpl-3.0.html).
