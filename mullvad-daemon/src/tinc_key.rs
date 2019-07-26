use crate::InternalDaemonEvent;

use std::sync::mpsc;

use futures::{future::Executor, sync::oneshot, Future};
use jsonrpc_client_core::Error as JsonRpcError;
use tokio_core::reactor::Remote;

use mullvad_types::account::AccountToken;
use tinc_plugin::TincOperator;

#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "Failed to generate pubkey")]
    GenerationError,
    #[error(display = "Failed to spawn future")]
    ExectuionError,
    #[error(display = "Unexpected RPC error")]
    RpcError(#[error(cause)] jsonrpc_client_core::Error),
    #[error(display = "Account already has maximum number of keys")]
    TooManyKeys,
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct KeyManager {
    tokio_remote:   Remote,
    daemon_tx:      mpsc::Sender<InternalDaemonEvent>,
    http_handle:    mullvad_rpc::HttpHandle,
    remote_pubkey:  String,
    local_pubkey:   String,
}

impl KeyManager {
    pub(crate) fn new(
        daemon_tx:      mpsc::Sender<InternalDaemonEvent>,
        http_handle:    mullvad_rpc::HttpHandle,
        tokio_remote:   Remote,
    ) -> Self {
        Self {
            daemon_tx,
            http_handle,
            tokio_remote,
            remote_pubkey: String::new(),
            local_pubkey: String::new(),
        }
    }

    pub fn get_local_pubkey(&self) -> String {
        self.local_pubkey.clone()
    }

    pub fn get_remote_pubkey(&self) -> String {
        self.remote_pubkey.clone()
    }

    /// Generate a new private key
    pub fn generate_key_sync(&mut self, account: &AccountToken) -> Result<()> {
        let account = account.to_string();

        let local_pubkey = match TincOperator::instance().get_local_pub_key() {
            Ok(x) => x,
            Err(_) => {
                TincOperator::instance().create_pub_key()
                    .map_err(|_|Error::GenerationError)?;
                TincOperator::instance().get_local_pub_key()
                    .map_err(|_|Error::GenerationError)?
            }
        };

        self.local_pubkey = local_pubkey.clone();

        let (tx, rx) = oneshot::channel();
        let fut = self.push_future_generator(account, local_pubkey)().then(|result| {
            let _ = tx.send(result);
            Ok(())
        });
        self.tokio_remote
            .execute(fut)
            .map_err(|_e| Error::ExectuionError)?;

        let server_pubkey = rx.wait()
            .map_err(|_| Error::ExectuionError)?
            .map_err(Self::map_rpc_error)?;

        self.remote_pubkey = server_pubkey;
        Ok(())
    }

    fn push_future_generator(
        &self,
        account: AccountToken,
        local_pubkey: String,
    ) -> Box<dyn FnMut() -> Box<dyn Future<Item = String, Error = JsonRpcError> + Send> + Send>
    {
        let mut rpc = mullvad_rpc::TincKeyProxy::new(self.http_handle.clone());

        let push_future =
            move || -> Box<dyn Future<Item = String, Error = JsonRpcError> + Send> {
                Box::new(rpc.push_tinc_key(account.clone(), local_pubkey.clone()))
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
