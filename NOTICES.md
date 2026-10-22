# Third-party notices

This inventory covers the 65 registry packages in the workspace dependency graph, including target-specific and development dependencies, as reported by `cargo metadata --locked --offline` for the committed `Cargo.lock` at the Issue #33 slice base (`b234beb`). Re-audit this file whenever `Cargo.lock` changes.

The two Limivra workspace packages (`limivra-core` and `limivra-cli`) are original project code licensed under MIT; see [`LICENSE`](LICENSE). The table lists third-party Cargo packages only. `adb` and `scrcpy` are invoked as separately installed host tools and are not redistributed by this tarball.

## License inventory

| Crate | Locked version | Declared SPDX license expression |
|---|---:|---|
| [`aho-corasick`](https://crates.io/crates/aho-corasick/1.1.5) | `1.1.5` | `Unlicense OR MIT` |
| [`anstream`](https://crates.io/crates/anstream/1.0.0) | `1.0.0` | `MIT OR Apache-2.0` |
| [`anstyle`](https://crates.io/crates/anstyle/1.0.14) | `1.0.14` | `MIT OR Apache-2.0` |
| [`anstyle-parse`](https://crates.io/crates/anstyle-parse/1.0.0) | `1.0.0` | `MIT OR Apache-2.0` |
| [`anstyle-query`](https://crates.io/crates/anstyle-query/1.1.5) | `1.1.5` | `MIT OR Apache-2.0` |
| [`anstyle-wincon`](https://crates.io/crates/anstyle-wincon/3.0.11) | `3.0.11` | `MIT OR Apache-2.0` |
| [`bitflags`](https://crates.io/crates/bitflags/2.13.2) | `2.13.2` | `MIT OR Apache-2.0` |
| [`bytes`](https://crates.io/crates/bytes/1.12.1) | `1.12.1` | `MIT` |
| [`cfg-if`](https://crates.io/crates/cfg-if/1.0.5) | `1.0.5` | `MIT OR Apache-2.0` |
| [`clap`](https://crates.io/crates/clap/4.6.7) | `4.6.7` | `MIT OR Apache-2.0` |
| [`clap_builder`](https://crates.io/crates/clap_builder/4.6.7) | `4.6.7` | `MIT OR Apache-2.0` |
| [`clap_derive`](https://crates.io/crates/clap_derive/4.6.7) | `4.6.7` | `MIT OR Apache-2.0` |
| [`clap_lex`](https://crates.io/crates/clap_lex/1.1.1) | `1.1.1` | `MIT OR Apache-2.0` |
| [`colorchoice`](https://crates.io/crates/colorchoice/1.0.5) | `1.0.5` | `MIT OR Apache-2.0` |
| [`errno`](https://crates.io/crates/errno/0.3.14) | `0.3.14` | `MIT OR Apache-2.0` |
| [`heck`](https://crates.io/crates/heck/0.5.0) | `0.5.0` | `MIT OR Apache-2.0` |
| [`is_terminal_polyfill`](https://crates.io/crates/is_terminal_polyfill/1.70.2) | `1.70.2` | `MIT OR Apache-2.0` |
| [`itoa`](https://crates.io/crates/itoa/1.0.18) | `1.0.18` | `MIT OR Apache-2.0` |
| [`lazy_static`](https://crates.io/crates/lazy_static/1.5.1) | `1.5.1` | `MIT OR Apache-2.0` |
| [`libc`](https://crates.io/crates/libc/0.2.191) | `0.2.191` | `MIT OR Apache-2.0` |
| [`lock_api`](https://crates.io/crates/lock_api/0.4.14) | `0.4.14` | `MIT OR Apache-2.0` |
| [`log`](https://crates.io/crates/log/0.4.34) | `0.4.34` | `MIT OR Apache-2.0` |
| [`matchers`](https://crates.io/crates/matchers/0.2.0) | `0.2.0` | `MIT` |
| [`memchr`](https://crates.io/crates/memchr/2.8.3) | `2.8.3` | `Unlicense OR MIT` |
| [`mio`](https://crates.io/crates/mio/1.2.4) | `1.2.4` | `MIT` |
| [`nu-ansi-term`](https://crates.io/crates/nu-ansi-term/0.50.3) | `0.50.3` | `MIT` |
| [`once_cell`](https://crates.io/crates/once_cell/1.21.4) | `1.21.4` | `MIT OR Apache-2.0` |
| [`once_cell_polyfill`](https://crates.io/crates/once_cell_polyfill/1.70.2) | `1.70.2` | `MIT OR Apache-2.0` |
| [`parking_lot`](https://crates.io/crates/parking_lot/0.12.5) | `0.12.5` | `MIT OR Apache-2.0` |
| [`parking_lot_core`](https://crates.io/crates/parking_lot_core/0.9.12) | `0.9.12` | `MIT OR Apache-2.0` |
| [`pin-project-lite`](https://crates.io/crates/pin-project-lite/0.2.17) | `0.2.17` | `Apache-2.0 OR MIT` |
| [`proc-macro2`](https://crates.io/crates/proc-macro2/1.0.107) | `1.0.107` | `MIT OR Apache-2.0` |
| [`quote`](https://crates.io/crates/quote/1.0.47) | `1.0.47` | `MIT OR Apache-2.0` |
| [`redox_syscall`](https://crates.io/crates/redox_syscall/0.5.18) | `0.5.18` | `MIT` |
| [`regex-automata`](https://crates.io/crates/regex-automata/0.4.18) | `0.4.18` | `MIT OR Apache-2.0` |
| [`regex-syntax`](https://crates.io/crates/regex-syntax/0.8.11) | `0.8.11` | `MIT OR Apache-2.0` |
| [`scopeguard`](https://crates.io/crates/scopeguard/1.2.0) | `1.2.0` | `MIT OR Apache-2.0` |
| [`serde`](https://crates.io/crates/serde/1.0.229) | `1.0.229` | `MIT OR Apache-2.0` |
| [`serde_core`](https://crates.io/crates/serde_core/1.0.229) | `1.0.229` | `MIT OR Apache-2.0` |
| [`serde_derive`](https://crates.io/crates/serde_derive/1.0.229) | `1.0.229` | `MIT OR Apache-2.0` |
| [`serde_json`](https://crates.io/crates/serde_json/1.0.154) | `1.0.154` | `MIT OR Apache-2.0` |
| [`sharded-slab`](https://crates.io/crates/sharded-slab/0.1.7) | `0.1.7` | `MIT` |
| [`signal-hook-registry`](https://crates.io/crates/signal-hook-registry/1.4.8) | `1.4.8` | `MIT OR Apache-2.0` |
| [`smallvec`](https://crates.io/crates/smallvec/1.16.3) | `1.16.3` | `MIT OR Apache-2.0` |
| [`socket2`](https://crates.io/crates/socket2/0.6.5) | `0.6.5` | `MIT OR Apache-2.0` |
| [`strsim`](https://crates.io/crates/strsim/0.11.1) | `0.11.1` | `MIT` |
| [`syn`](https://crates.io/crates/syn/2.0.119) | `2.0.119` | `MIT OR Apache-2.0` |
| [`syn`](https://crates.io/crates/syn/3.0.9) | `3.0.9` | `MIT OR Apache-2.0` |
| [`thiserror`](https://crates.io/crates/thiserror/2.0.21) | `2.0.21` | `MIT OR Apache-2.0` |
| [`thiserror-impl`](https://crates.io/crates/thiserror-impl/2.0.21) | `2.0.21` | `MIT OR Apache-2.0` |
| [`thread_local`](https://crates.io/crates/thread_local/1.1.10) | `1.1.10` | `MIT OR Apache-2.0` |
| [`tokio`](https://crates.io/crates/tokio/1.54.0) | `1.54.0` | `MIT` |
| [`tokio-macros`](https://crates.io/crates/tokio-macros/2.7.2) | `2.7.2` | `MIT` |
| [`tracing`](https://crates.io/crates/tracing/0.1.44) | `0.1.44` | `MIT` |
| [`tracing-attributes`](https://crates.io/crates/tracing-attributes/0.1.31) | `0.1.31` | `MIT` |
| [`tracing-core`](https://crates.io/crates/tracing-core/0.1.36) | `0.1.36` | `MIT` |
| [`tracing-log`](https://crates.io/crates/tracing-log/0.2.0) | `0.2.0` | `MIT` |
| [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber/0.3.23) | `0.3.23` | `MIT` |
| [`unicode-ident`](https://crates.io/crates/unicode-ident/1.0.27) | `1.0.27` | `(MIT OR Apache-2.0) AND Unicode-3.0` |
| [`utf8parse`](https://crates.io/crates/utf8parse/0.2.2) | `0.2.2` | `Apache-2.0 OR MIT` |
| [`valuable`](https://crates.io/crates/valuable/0.1.1) | `0.1.1` | `MIT` |
| [`wasi`](https://crates.io/crates/wasi/0.11.1+wasi-snapshot-preview1) | `0.11.1+wasi-snapshot-preview1` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| [`windows-link`](https://crates.io/crates/windows-link/0.2.1) | `0.2.1` | `MIT OR Apache-2.0` |
| [`windows-sys`](https://crates.io/crates/windows-sys/0.61.2) | `0.61.2` | `MIT OR Apache-2.0` |
| [`zmij`](https://crates.io/crates/zmij/1.0.23) | `1.0.23` | `MIT` |

## Audit notes

- All 65 locked registry package entries declare a license expression. The metadata inventory contains no GPL, LGPL, AGPL, MPL, or SSPL terms; it therefore found no strong-copyleft dependency in this workspace lockfile snapshot.
- No locked package currently declares `BSD-3-Clause`; the actual expressions are listed above rather than inferred from the examples in the packaging task.
- Dual-license `OR` expressions provide alternatives; `AND` means all listed conditions apply. In particular, `unicode-ident` declares an additional `Unicode-3.0` term, and `wasi` lists an LLVM exception among its alternatives. Preserve these expressions when updating the inventory.
- SPDX expressions come from upstream Cargo package metadata and are an engineering inventory, not legal advice or a substitute for reviewing each crate's shipped copyright and license files before a public release. The checksum manifest is not a digital signature and does not establish publisher authenticity.
