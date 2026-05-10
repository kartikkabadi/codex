// codex-auth/src/lib.rs

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod api_key;
pub mod aws_sigv4;
pub mod factory;
pub mod gcp;
pub mod oauth2;

pub use factory::create_auth_provider;
