//! Session Management Subsystem
//!
//! # Responsibilities
//! - Actor-per-room isolation maintaining dedicated 60Hz tick loop tasks.
//! - Cryptographically signed reconnect tokens with 30-second grace window recovery.
//! - Bounded input queues per peer (max 64 frames) with backpressure shedding.
//! - Participant connection lifecycle (Lobby -> Active -> Disconnected -> Finished).

pub mod room {
    // Room logic
}
