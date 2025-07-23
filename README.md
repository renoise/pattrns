<img src="./docs/src/logo.png" alt="pattrns" height="50"/>

is an experimental imperative music sequence generator engine. It allows you to programmatically create music sequences either in plain [Rust](https://www.rust-lang.org/) as library (*static, compiled*) or in [Lua](https://www.lua.org/) as a scripting engine (*dynamic, interpreted*). So it's also suitable for [live coding music](https://github.com/pjagielski/awesome-live-coding-music).

In addition to its imperative event generator approach, it also supports the creation of musical events using [TidalCycles'](https://tidalcycles.org/) mini-notation.

This crate only deals with the *generation of raw musical events*. It does not generate audio. You must use an application with built-in support for `pattrns` to use it.


## Conceptual Overview

`pattrns` generates musical sequences using three distinct components/stages:

- **Rhythm**: (`pulse` in scripts) A dynamic pulse generator to define a rhythmic pulse train.
- **Gate**: (`gate` in scripts) An optional pulse filter between the rhythm and event emitter.
- **Emitter**: (`event` in scripts) A dynamic note or parameter event generator which is triggered by the pulse train.

By separating the rhythmic from the tonal part of a musical sequence, each part can be freely modified, composed, and (re)combined as needed.


## Documentation & Guides

Read the [Scripting Book](https://renoise.github.io/pattrns/).
It contains an introduction, guides, full Lua API documentation, and script examples.

The Rust backend uses standard Rust documentation features. The documentation is not yet hosted online, but you can generate it locally via `cargo doc --open`.


## Applications

- [`Online Playground`](https://pattrns.renoise.com): A simple browser-based app that lets you learn and test how `pattrns` work.
- [`Renoise`](https://www.renoise.com): Uses `pattrns` in its instrument [phrase editor](https://tutorials.renoise.com/wiki/Phrase_Editor).


## Integration Examples

- [`examples/play.rs`](./examples/play.rs): Demonstrates how to use pattrns using only Rust. It defines and plays a little music thing. The content can only be changed at compile time.
- [`examples/play-script.rs`](./examples/play-script.rs): An example using the Lua API. It also defines and plays a little music thing, but its content can be added, removed, and changed on the fly to perform some basic live music hacking.


## Repository Structure

The repository is organised as a monorepo and contains several sub-projects:

- `benches`: Rust benchmark source code to ensure performance does not regress with changes.
- `bindings`: Provides C++ bindings, an FFI layer, and a relay loader for dynamically loading the pattrns shared library.
- `src`: Contains the core Rust source code for the pattrns engine and its Lua bindings.
- `docs`: The source files for the [Scripting Book](https://renoise.github.io/pattrns/), built with mdBook.
- `examples`: Rust and WASM examples that demonstrate how to use the pattrns library.
- `types`: Contains the pattrns Lua API documentation for the Lua Language server [LuaLS](https://luals.github.io/).


## Acknowledgments

Thanks to [unlessgames](https://github.com/unlessgames) for adding the TidalCycles mini-notation support.


## Contributing

Patches are welcome! Please fork the latest git repository and create a feature or bugfix branch.


## License

`pattrns` is distributed under the terms of the [GNU Affero General Public License V3](https://www.gnu.org/licenses/agpl-3.0.html).
