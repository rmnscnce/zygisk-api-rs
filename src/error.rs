#[derive(Clone, Debug, thiserror::Error)]
pub enum ZygiskError {
    #[error("Unable to connect to the companion process")]
    ConnectCompanionError,
    #[error("Unrecognized state flag returned by Zygisk")]
    UnrecognizedStateFlag,
    #[error("Encountered an error while committing PLT hooks")]
    PltHookCommitError,
}
