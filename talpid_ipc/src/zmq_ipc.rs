extern crate zmq;

use super::{OnMessage, ErrorKind, Result, ResultExt, IpcServerId};
use std::thread;

/// Starts listening to incoming IPC connections on a random port.
/// Messages are sent to the `on_message` callback. If anything went wrong
/// when reading the message, the message will be an `Err`.
/// NOTE that this does not apply to errors regarding whether the server
/// could start or not, those are returned directly by this function.
///
/// This function is non-blocking and thus spawns a thread where it
/// listens to messages.
///
/// The value returned from this function should be used by the clients to
/// the server.
pub fn start_new_server(on_message: Box<OnMessage<Vec<u8>>>) -> Result<IpcServerId> {

    for port in 5000..5010 {
        let connection_string = format!("tcp://127.0.0.1:{}", port);
        if let Ok(socket) = start_zmq_server(&connection_string) {
            let _ = start_receive_loop(socket, on_message);
            return Ok(connection_string);
        }
    }

    return Err(ErrorKind::CouldNotStartServer.into());
}

fn start_zmq_server(connection_string: &str) -> zmq::Result<zmq::Socket> {
    let ctx = zmq::Context::new();

    let socket = ctx.socket(zmq::PULL)?;
    socket.bind(connection_string)?;

    Ok(socket)
}

fn start_receive_loop(socket: zmq::Socket,
                      mut on_message: Box<OnMessage<Vec<u8>>>)
                      -> thread::JoinHandle<()> {

    thread::spawn(move || loop {
        let read_res = socket.recv_bytes(0).chain_err(|| ErrorKind::ReadFailure);
        on_message(read_res);
    })
}