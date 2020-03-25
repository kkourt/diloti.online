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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JoinReq { }

/**
 * Lobby state
 */

/// External information for a player
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// player id (0 is admin of the game)
    pub admin: bool,
    /// table position
    pub tpos: u8,
}

/// Lobby state info
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub players: Vec<PlayerInfo>,
    pub self_id: usize, // self id in the vector (so that the player knows who they are)
    pub nplayers: u8,   // total number of players for the game
}



/**
 * In game state
 */


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameInfo { }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameEndInfo { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    InLobby(LobbyInfo),
    InGame(GameInfo),
    GameEnd(GameEndInfo),
}
