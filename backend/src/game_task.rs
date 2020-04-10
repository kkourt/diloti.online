//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

/**
 * Game task structures
 */

use core::srvcli;
use crate::player_task::PlayerTaskTx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTaskId(pub usize);

/// Game task requests
#[derive(Debug)]
pub enum GameReq {
    /// Register a player to the game
    RegisterPlayer(PlayerTaskTx, String),
    /// Forward a client request to the game task
    ClientReq(PlayerTaskId, srvcli::ClientMsg),
    /// Notify the server that the player task for handling the websocket connection has terminated
    /// (typically due to user disconnect or an error).
    PlayerTaskTerminated(PlayerTaskId),
    /// Re-join a player to the game (tx, name), and name has to match an existing disconnected
    /// player
    ReconnectPlayer(PlayerTaskTx, String),
}

/// Channel for {<player_tasks>, ???} -> <game_task> communication
pub type GameTaskTx = tokio::sync::mpsc::Sender<GameReq>;
pub type GameTaskRx = tokio::sync::mpsc::Receiver<GameReq>;
