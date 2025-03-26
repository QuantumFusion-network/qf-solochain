use relay_substrate_client::SimpleRuntimeVersion;

// Bridge-supported network definition.
///
/// Used to abstract away CLI commands.
pub trait CliChain: relay_substrate_client::Chain {
    /// Current version of the chain runtime, known to relay.
    ///
    /// can be `None` if relay is not going to submit transactions to that chain.
    const RUNTIME_VERSION: Option<SimpleRuntimeVersion>;
}