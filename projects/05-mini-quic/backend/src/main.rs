use mini_quic::connection::QuicConnection;
use mini_quic::wire::packet::ConnectionId;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Mini-QUIC Transport Engine v0.1.0 (RFC 9000 / RFC 9002)...");

    let client_cid = ConnectionId::random(8);
    let server_cid = ConnectionId::random(8);

    let server = QuicConnection::new_server(client_cid.clone(), server_cid.clone());
    let client = QuicConnection::new_client(server_cid);

    tracing::info!(
        "Initialized Client (SCID={:?}) and Server (SCID={:?})",
        client.scid.0,
        server.scid.0
    );
    tracing::info!("Mini-QUIC Engine ready for UDP ingestion.");
}
