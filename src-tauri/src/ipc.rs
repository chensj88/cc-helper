use helper_protocol::{Request, Response, PROTOCOL_VERSION};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

const SOCKET_PATH: &str = "/tmp/cc-helper.sock";

#[derive(Debug)]
pub enum IpcMessage {
    Request {
        request: Request,
        response_tx: tokio::sync::oneshot::Sender<Response>,
    },
}

fn cleanup_stale_socket() {
    let path = Path::new(SOCKET_PATH);
    if !path.exists() {
        return;
    }
    if std::os::unix::net::UnixStream::connect(SOCKET_PATH).is_ok() {
        eprintln!("ipc: another daemon is already running");
        std::process::exit(1);
    }
    std::fs::remove_file(SOCKET_PATH).ok();
}

pub fn start_ipc_listener() -> tokio::sync::mpsc::Receiver<IpcMessage> {
    cleanup_stale_socket();
    let (tx, rx) = tokio::sync::mpsc::channel::<IpcMessage>(32);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .expect("Failed to create tokio runtime for IPC");
        rt.block_on(async move {
            let listener = match UnixListener::bind(SOCKET_PATH) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("ipc: failed to bind: {}", e);
                    return;
                }
            };
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let tx = tx.clone();
                        tokio::spawn(handle_connection(stream, tx));
                    }
                    Err(e) => eprintln!("ipc: accept error: {}", e),
                }
            }
        });
    });
    rx
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    tx: tokio::sync::mpsc::Sender<IpcMessage>,
) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => return,
        Err(_) => return,
        Ok(_) => {}
    }
    let request: Request = match serde_json::from_str::<Request>(&line) {
        Ok(r) => r,
        Err(e) => {
            let err = Response::error(&format!("parse error: {}", e));
            let _ = writer.write_all(err.to_wire().as_bytes()).await;
            return;
        }
    };
    if request.version != PROTOCOL_VERSION {
        let err = Response::error("version mismatch");
        let _ = writer.write_all(err.to_wire().as_bytes()).await;
        return;
    }
    // Non-blocking events: send ack immediately, forward with dummy channel
    if !matches!(
        request.event,
        helper_protocol::HookEvent::PermissionRequest
    ) {
        let ack = Response::ack();
        let _ = writer.write_all(ack.to_wire().as_bytes()).await;
        let (dummy_tx, _dummy_rx) = tokio::sync::oneshot::channel::<Response>();
        let _ = tx.try_send(IpcMessage::Request {
            request,
            response_tx: dummy_tx,
        });
        return;
    }
    // Blocking: PermissionRequest
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<Response>();
    if tx
        .try_send(IpcMessage::Request {
            request,
            response_tx: resp_tx,
        })
        .is_err()
    {
        return;
    }
    match resp_rx.await {
        Ok(response) => {
            let _ = writer.write_all(response.to_wire().as_bytes()).await;
        }
        Err(_) => {}
    }
}
