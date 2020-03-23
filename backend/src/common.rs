//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

/// Common datastructures for server and client
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateGameReq {
    pub nplayers: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateGameRep {
    pub game_id: String,
}
