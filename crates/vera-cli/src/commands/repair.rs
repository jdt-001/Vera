//! `vera repair` — repair the configured backend by re-fetching missing assets
//! or re-persisting API configuration from the current environment.

use vera_core::config::InferenceBackend;

use crate::commands::setup;

pub fn run(backend: Option<InferenceBackend>, api: bool, json_output: bool) -> anyhow::Result<()> {
    let effective_backend = if api {
        InferenceBackend::Api
    } else {
        backend.unwrap_or_else(|| vera_core::config::resolve_backend(None))
    };

    setup::configure_backend(
        effective_backend,
        None,
        json_output,
        "Vera repair complete.",
    )
}
