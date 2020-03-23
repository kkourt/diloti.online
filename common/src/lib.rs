//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

/// Common datastructures for server and client
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateReq {
    pub nplayers: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateRep {
    pub game_id: String,
}

/**
 * Pre-game state
 */

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Empty for now
pub struct PreReq {
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// player id (0 is admin of the game)
    pub id: usize,
    /// table position
    pub tpos: u8,
}

/// Pre-game state info
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreInfo {
    pub players: Vec<PlayerInfo>,
    pub nplayers: u8, // total number of players for the game
}

impl PlayerInfo {
    // NB: if we implement player remove, this would have to change
    pub fn is_admin(&self) -> bool { self.id == 0 }
}
