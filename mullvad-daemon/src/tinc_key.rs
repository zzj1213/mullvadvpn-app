use crate::InternalDaemonEvent;

use std::thread;
use std::sync::mpsc;

use futures::{future::Executor, sync::oneshot, Async, Future, Poll};
use jsonrpc_client_core::Error as JsonRpcError;
use tokio_core::reactor::Remote;

use mullvad_types::account::AccountToken;
use tinc_plugin::{TincOperator, TincRunMode};
use std::path::PathBuf;

#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "Failed to generate private key")]
    GenerationError(#[error(cause)] rand::Error),
    #[error(display = "Failed to spawn future")]
    ExectuionError,
    #[error(display = "Unexpected RPC error")]
    RpcError(#[error(cause)] jsonrpc_client_core::Error),
    #[error(display = "Account already has maximum number of keys")]
    TooManyKeys,
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct KeyManager {
    tokio_remote: Remote,
    daemon_tx: mpsc::Sender<InternalDaemonEvent>,
    http_handle: mullvad_rpc::HttpHandle,
    resource_dir: String,
}

impl KeyManager {
    pub(crate) fn new(
        resource_dir:   &PathBuf,
        daemon_tx:      mpsc::Sender<InternalDaemonEvent>,
        http_handle:    mullvad_rpc::HttpHandle,
        tokio_remote:   Remote,
    ) -> Self {
        let resource_dir_str = resource_dir.to_str().unwrap().to_string();
        Self {
            resource_dir: resource_dir_str,
            daemon_tx,
            http_handle,
            tokio_remote,
        }
    }

    /// Generate a new private key
    pub fn generate_key_sync(&mut self, account: &AccountToken) -> Result<()> {
        let account = account.to_string();

        let (tx, rx) = oneshot::channel();
        let fut = self.push_future_generator(account)().then(|result| {
            let _ = tx.send(result);
            Ok(())
        });
        self.tokio_remote
            .execute(fut)
            .map_err(|_e| Error::ExectuionError)?;

        let server_pubkey = rx.wait()
            .map_err(|_| Error::ExectuionError)?
            .map_err(Self::map_rpc_error)?;

        let _ = TincOperator::instance().add_hosts("vpnserver", &server_pubkey);
        Ok(())
    }

    fn push_future_generator(
        &self,
        account: AccountToken,
    ) -> Box<dyn FnMut() -> Box<dyn Future<Item = String, Error = JsonRpcError> + Send> + Send>
    {
        if !TincOperator::is_inited() {
            TincOperator::new(&self.resource_dir, TincRunMode::Client);
        }
        let pubkey = TincOperator::instance().get_pub_key().unwrap();
        let mut rpc = mullvad_rpc::TincKeyProxy::new(self.http_handle.clone());

        let push_future =
            move || -> Box<dyn Future<Item = String, Error = JsonRpcError> + Send> {
                Box::new(rpc.push_tinc_key(account.clone(), pubkey.clone()))
            };
        Box::new(push_future)
    }

    fn map_rpc_error(err: jsonrpc_client_core::Error) -> Error {
        match err.kind() {
            // TODO: Consider handling the invalid account case too.
            jsonrpc_client_core::ErrorKind::JsonRpcError(err) if err.code.code() == -703 => {
                Error::TooManyKeys
            }
            _ => Error::RpcError(err),
        }
    }
}