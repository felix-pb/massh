//! This library is a simple wrapper around the [`ssh2`] crate
//! to run SSH/SCP commands on a "mass" of hosts in parallel.
//!
//! The `massh` *library* is mainly intended to support the `massh` *binary*:
//! a Rust version of the parallel SSH program [`pssh(1)`].
//!
//! If you want to try the CLI app, you can check it on [GitHub] and install it with Cargo:
//!
//! ```no_run
//! cargo install massh
//! ```
//!
//! The rest of this documentation focuses on the library crate,
//! which offers two types of SSH client: [`MasshClient`] and [`SshClient`].
//!
//! Check their respective documentation for the details of their public API with examples.
//!
//! [`ssh2`]: https://docs.rs/ssh2
//! [`pssh(1)`]: https://linux.die.net/man/1/pssh
//! [GitHub]: https://github.com/felix-pb/massh

mod config;
mod massh_client;
mod ssh_client;

pub use config::{MasshConfig, MasshHostConfig};
pub use massh_client::{MasshClient, MasshHost, MasshReceiver};
pub use ssh_client::{SshAuth, SshClient, SshOutput};
