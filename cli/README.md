# rbook CLI (Experimental)

[![Crates.io](https://img.shields.io/crates/v/rbook-cli.svg?logo=rust&style=flat-square)](https://crates.io/crates/rbook-cli)
[![License](https://img.shields.io/badge/license-Apache%202.0-maroon?logo=apache&style=flat-square)](https://github.com/DevinSterling/rbook/blob/master/LICENSE)

This is an unstable experimental command-line interface for [rbook](https://crates.io/crates/rbook).

There is currently only support for outputting the debugged contents of `rbook::Epub`:

- `rbook debug my.epub --metadata --manifest --spine`
