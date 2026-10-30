//! Fill-option resolution: sizing a request from the provider's declared limits.
//!
//! Split out of `handlers.rs` to keep both files inside the 250-line law. The rule is `GB-215`: the
//! API exposes no context window, so the ceiling is declared in config, and the output budget slides
//! down as the prompt grows rather than sitting at a constant.

use crate::args::FillArgs;
use joiner_core::function_fill::{DEFAULT_FILL_MAX_TOKENS, FillOptions, VerifyMode};
use sawhorse_core::{SawhorseConfig, config::schema::ProviderConfig, types::ModelRole};

/// Estimated prompt cost for the file about to be sent, plus room for the instructions.
///
/// A char-per-four estimate is deliberately crude: it only has to keep the budget under the window,
/// and the provider reports the real count once the call returns.
fn estimated_prompt_tokens(file: &std::path::Path) -> u32 {
    std::fs::read_to_string(file)
        .map(|source| (source.chars().count() / 4) as u32 + 512)
        .unwrap_or(512)
}

/// Resolves the fill options for `args` against the hole-filler provider's declared limits.
#[must_use]
pub fn resolve(config: &SawhorseConfig, args: &FillArgs) -> FillOptions {
    let limits = config
        .roles
        .get(&ModelRole::HoleFiller)
        .and_then(|role| config.providers.get(&role.provider))
        .map(ProviderConfig::limits)
        .unwrap_or_default();

    // An explicit `--max-tokens` wins, then the declared budget, then the crate default: a model with
    // no declared window keeps its own default rather than an invented ceiling.
    let max_tokens = args.max_tokens.unwrap_or_else(|| {
        limits
            .output_budget(estimated_prompt_tokens(&args.file))
            .unwrap_or(DEFAULT_FILL_MAX_TOKENS)
    });

    FillOptions {
        max_tokens,
        verify: match &args.verify_crate {
            Some(dir) => VerifyMode::Crate(dir.clone()),
            None => VerifyMode::Syntax,
        },
        apply: args.apply,
        reasoning_effort: limits.reasoning_effort,
    }
}
