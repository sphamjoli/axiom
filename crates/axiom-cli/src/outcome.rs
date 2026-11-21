//! How a run of `axiom` ended, and the exit code that says so. The codes are
//! part of the command line's contract and do not change.

/// How a run of `axiom` ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The command succeeded and, for `check`, every claim was accepted.
    Accepted,
    /// The checker rejected at least one claim.
    Rejected,
    /// The model or the certificate was refused as invalid or unsupported.
    Refused,
    /// The command could not do its work: a file could not be read or
    /// written, or the checker could not run, ran too long or misbehaved.
    Failed,
    /// The command line is not a valid command.
    Usage,
}

impl Outcome {
    /// Every outcome, in exit code order.
    pub const ALL: [Self; 5] = [
        Self::Accepted,
        Self::Rejected,
        Self::Refused,
        Self::Failed,
        Self::Usage,
    ];

    /// The process exit code for the outcome.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Accepted => 0,
            Self::Rejected => 1,
            Self::Refused => 2,
            Self::Failed => 3,
            Self::Usage => 64,
        }
    }

    /// What the outcome means, for the help text.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Accepted => "success, every claim accepted",
            Self::Rejected => "some claim rejected",
            Self::Refused => "model or certificate refused",
            Self::Failed => "operational failure: file, checker or time limit",
            Self::Usage => "usage error",
        }
    }
}
