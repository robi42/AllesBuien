//! Why the application could not start: the one error [`AppBuilder::boot`]
//! reports, for each step of Qt's startup that can fail.
//!
//! [`AppBuilder::boot`]: crate::AppBuilder::boot

use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub enum BootstrapError {
    ApplicationCreation,
    EngineCreation,
    RuntimeCreation(std::io::Error),
    ControllerCreation,
    RootWindowCreation,
}

impl Display for BootstrapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApplicationCreation => formatter.write_str("failed to create QGuiApplication"),
            Self::EngineCreation => formatter.write_str("failed to create QQmlApplicationEngine"),
            Self::RuntimeCreation(error) => write!(formatter, "failed to create runtime: {error}"),
            Self::ControllerCreation => {
                formatter.write_str("failed to create application controller")
            }
            Self::RootWindowCreation => formatter.write_str("failed to create root QML window"),
        }
    }
}

impl Error for BootstrapError {}

#[cfg(test)]
mod tests {
    use std::io;

    use super::BootstrapError;

    #[test]
    fn each_failure_names_the_step_that_failed() {
        // Given / When / Then
        assert_eq!(
            BootstrapError::ApplicationCreation.to_string(),
            "failed to create QGuiApplication"
        );
        assert_eq!(
            BootstrapError::EngineCreation.to_string(),
            "failed to create QQmlApplicationEngine"
        );
        assert_eq!(
            BootstrapError::RuntimeCreation(io::Error::other("no threads")).to_string(),
            "failed to create runtime: no threads"
        );
        assert_eq!(
            BootstrapError::ControllerCreation.to_string(),
            "failed to create application controller"
        );
        assert_eq!(
            BootstrapError::RootWindowCreation.to_string(),
            "failed to create root QML window"
        );
    }
}
