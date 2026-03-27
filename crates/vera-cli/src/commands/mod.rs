//! CLI command implementations.
//!
//! Each subcommand is implemented in its own module to keep files focused
//! and under the 500-line budget.

pub mod agent;
pub mod config;
pub mod doctor;
pub mod index;
pub mod mcp;
pub mod repair;
pub mod search;
pub mod setup;
pub mod stats;
pub mod uninstall;
pub mod update;
