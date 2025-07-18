# Bindings

C++ bindings for the Rust `pattrns` library.

This project creates a shared library with a thin FFI layer, allowing `pattrns` to be used from C++ applications.

See [`pattrns.h`](./includes/pattrns.h) for the exposed API.


## Building

To build `pattrns` with the default interpreter (LuaJIT), run:

```bash
cargo build --release
```

See [Cargo.toml](./Cargo.toml) for optional feature flags.


## Using a Custom `pattrns` Library in Renoise

If you want to build and use your own version of `pattrns` in Renoise, build the shared library as explained above and place it into Renoise's preferences folder. Renoise will then skip loading its bundled `pattrns` library and load the user-provided one instead.

To locate your Renoise preferences folder, open Renoise and click on the `Help` -> `Show Preferences Folder...` menu item.

To verify that the library is loaded as intended, check the Renoise log file. It will contain a line similar to this:

```
Player: Loading pattrns library from: '/SOME/PATH/TO/pattrns.so/dylib/dll'
```

## Usage in other C++ Applications

To use `pattrns` in your application, you can link against the created shared library.

The provided [`relay.cpp`](./src/relay.cpp) can also be used to load the library dynamically at runtime. This is how [Renoise](https://www.renoise.com) uses `pattrns`. To use it, simply add, compile, and link the `relay.cpp` file with your C++ application, and then call `pattrns::load_library("/SOME/PATH/TO/pattrns.so/dylib/dll")`.

Note: The FFI layer is, in theory, C-language compatible, but `cbindgen` is configured via [`cbindgen.toml`](./cbindgen.toml) to produce a C++ header file by default. Tweak this file if you require a plain C header.


## License

`pattrns` is distributed under the terms of the [GNU Affero General Public License V3](https://www.gnu.org/licenses/agpl-3.0.html).
