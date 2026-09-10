use super::Rules;

/// Minimal rules that reuse every default hook.
///
/// Used when no custom rules are supplied to [`GameEngine::get`](crate::GameEngine::get).
pub struct BasicRules;

impl Rules for BasicRules {}
