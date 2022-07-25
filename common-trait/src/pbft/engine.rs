
use crate::pbft::service::Service;
use std::sync::mpsc::Receiver;
use types::pbft::consensus::{
    Update,
    StartupState,
    Error,
};
pub trait Engine {
    /// Called after the engine is initialized, when a connection to the validator has been
    /// established. Notifications from the validator are sent along `updates`. `service` is used
    /// to send requests to the validator.
    fn start(
        &mut self,
        updates: Receiver<Update>,
        service: Box<dyn Service>,
        startup_state: StartupState,
    ) -> Result<(), Error>;

    /// Get the version of this engine
    fn version(&self) -> String;

    /// Get the name of the engine, typically the algorithm being implemented
    fn name(&self) -> String;

    /// Any additional name/version pairs this engine supports
    fn additional_protocols(&self) -> Vec<(String, String)>;
}