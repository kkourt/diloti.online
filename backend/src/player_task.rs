//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

/**
 * Player task structures
 */

use crate::game::PlayerId;

/// Information passed from the game task to the player task
#[derive(Debug)]
pub enum PlayerTaskMsg {
    /// This is the first message passed after registration.
    /// If ther registration is successful, it includes a player id that will never change.
    RegistrationResult(Result<PlayerId, String>),
    ForwardToClient(common::ServerMsg),
}

/// Channel for <game_task> -> <player_task> communication
pub type PlayerTaskTx = tokio::sync::mpsc::Sender<PlayerTaskMsg>;
pub type PlayerTaskRx = tokio::sync::mpsc::Receiver<PlayerTaskMsg>;
