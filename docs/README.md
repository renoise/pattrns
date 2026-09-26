## pattrns Book


### Requirements

The documentations are generated with [mdBook](https://github.com/rust-lang/mdBook). To preview the pages locally, you need [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) to install mdbook:

```sh
cargo install mdbook@^0.5
```

Or use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) to avoid compiling the binaries.


### Building

Then you can serve the docs to `localhost:3000` using mdbook, this will automatically update the browser tab when you change the markdown files.

```sh
mdbook serve --open
```


### Generate API reference

See [generate/README.md](./generate/README.md)
