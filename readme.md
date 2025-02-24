# fortranc-rs

A library for [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html) 
to compile a set of Fortran/assembly files into a static archive for cargo 
to link into the crate being built. This crate does not compile code itself;
it calls out to the default compiler for the platform.

This is not ready for usage yet, use for hacking only.

The code is pretty much copied from [cc-rs project](https://github.com/rust-lang/cc-rs).

## License

This project is licensed under either of

* Apache License, Version 2.0, ([license-apache](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([license-mit](LICENSE-MIT) or
  https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in cc-rs by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
