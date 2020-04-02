//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use serde::{Deserialize, Serialize};
use crate::{game, deck, table, actions, repr};

pub use table::PlayerTpos;

/// Server <-> client interaction

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateReqDebug {
    pub hand_s: String,
    pub table_s: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateReq {
    pub nplayers: u8,
    pub debug: Option<CreateReqDebug>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// player id (0 is admin of the game)
    pub admin: bool,
    /// table position
    pub tpos: PlayerTpos,
    /// player name
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerId(pub usize); // Player (internal) identifier

/// Lobby state info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub players: Vec<PlayerInfo>,
    pub self_id: PlayerId, // self id in the vector (so that the player knows who they are)
    pub nplayers: u8,   // total number of players for the game
}

/**
 * In game state
 */



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo(pub game::PlayerGameView);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEndInfo { }


/**
 * Message types
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    InLobby(LobbyInfo),
    GameUpdate(game::PlayerGameView),
    InvalidAction(String),
    GameEnd(GameEndInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    StartGame,
    PlayerAction(actions::PlayerAction),
}

impl CreateReq {
    pub fn verify_debug(&mut self) {
        let valid = self.get_debug_hand().is_some() && self.get_debug_table().is_some();
        if !valid {
            self.debug = None
        }
    }

    pub fn get_debug_hand(&self) -> Option<deck::Deck> {
        if let Some(debug) = &self.debug {
            if debug.hand_s.len() == 0 {
                None
            } else {
                repr::DeckRepr::new(&debug.hand_s).parse()
            }
        } else { None }
    }

    pub fn get_debug_table(&self) -> Option<table::Table> {
        if let Some(debug) = &self.debug {
            if debug.table_s.len() == 0 {
                None
            } else {
                repr::TableRepr::new(&debug.table_s).parse()
            }
        } else { None }
    }
}

impl LobbyInfo {
    pub fn player_from_tpos(&self, tpos: PlayerTpos) -> Option<&PlayerInfo> {
       self.players.iter().find( |pi| pi.tpos == tpos)
    }
}
