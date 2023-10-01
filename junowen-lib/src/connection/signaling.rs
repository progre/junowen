pub mod socket;
pub mod stdio_signaling_interface;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CompressedSessionDesc(pub String);
