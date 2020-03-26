//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use tokio::sync::{oneshot, mpsc};

use core::srvcli;

use crate::game::{GameConfig, GameId};
use crate::game_task::GameTaskTx;


/// Directory requests (includes oneshot channels for replies as needed)
#[derive(Debug)]
pub enum DirReq {
    /// Create a new game, return the ID
    CreateGame(GameConfig, oneshot::Sender<srvcli::CreateRep>),
    /// Request the game task for a given game
    GetGameHandle(GameId, oneshot::Sender<Option<GameTaskTx>>),
}

/// A channel to send requests to the directory task
pub type DirTaskTx = mpsc::Sender<DirReq>;
/// A channel to receive directory requests
pub type DirTaskRx = mpsc::Receiver<DirReq>;
