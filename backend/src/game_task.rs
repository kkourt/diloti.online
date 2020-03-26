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

/// Game task requests
#[derive(Debug)]
pub enum GameReq {
    // Player name
    RegisterPlayer(PlayerTaskTx, String),
    ClientReq(srvcli::PlayerId, srvcli::ClientMsg),
}

/// Channel for {<player_tasks>, ???} -> <game_task> communication
pub type GameTaskTx = tokio::sync::mpsc::Sender<GameReq>;
pub type GameTaskRx = tokio::sync::mpsc::Receiver<GameReq>;
