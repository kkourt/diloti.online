//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use serde::{Deserialize, Serialize};
use crate::{game, deck, table, actions, repr};

pub use game::PlayerGameView;
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
    /// connected: is player connected to the server?
    pub connected: bool,
}

/// PlayerId identifies a player in the lobby.
///
/// Specifically, it's value can be used to index players in the lobby to get information such as
/// name and table position.
//
// NB: note that this means that the server want to swap PlayerIds, then it should update LobbyInfo
// by sending a LobbyUpdate message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerId(pub usize);

/// Lobby state info
// NB: rename LobbyInfo to PlayersInfo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub players: Vec<PlayerInfo>,
    pub self_id: PlayerId, // self id in the vector (so that the player knows who they are)
    pub nplayers: u8,   // total number of players for the game
}

/**
 * In game state
 */


/**
 * Message types
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    LobbyUpdate(LobbyInfo),
    GameUpdate(PlayerGameView),
    InvalidAction(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    StartGame,
    SwapTpos(PlayerTpos, PlayerTpos),
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

    pub fn iter_players<'a>(&'a self) -> impl Iterator<Item=(PlayerId, &PlayerInfo)> + 'a {
        self.players.iter().enumerate().map( |(i,p)| (PlayerId(i), p))
    }

    pub fn all_ready(&self) -> bool {
        self.players.iter().filter(|p| p.connected).count() == self.nplayers as usize
    }

    pub fn is_self(&self, pid: PlayerId) -> bool {
        self.self_id == pid
    }

    pub fn is_self_from_tpos(&self, tpos: PlayerTpos) -> bool {
        self.player_id_from_tpos(tpos).map_or(false, |pid| self.self_id == pid)
    }

    pub fn is_admin(&self, pid: PlayerId) -> bool {
        self.players.get(pid.0).map_or(false, |x| x.admin)
    }

    pub fn am_i_admin(&self) -> bool {
        self.players[self.self_id.0].admin
    }

    // iterate players in tpos order
    pub fn iter_players_tpos<'a>(&'a self) -> impl Iterator<Item=(PlayerTpos, &PlayerInfo)> + 'a {
        (0..self.nplayers)
            .filter_map(move |i| {
                let tpos = PlayerTpos(i);
                match self.player_from_tpos(tpos.clone()) {
                    None => None,
                    Some(p) => Some((tpos, p)),
                }
            })
    }

    pub fn my_tpos(&self) -> PlayerTpos {
        self.players[self.self_id.0].tpos
    }

    pub fn get_player(&self, pid: PlayerId) -> Option<&PlayerInfo> {
        self.players.get(pid.0)
    }

    pub fn player_id_from_tpos(&self, tpos: PlayerTpos) -> Option<PlayerId> {
        self.players
            .iter()
            .enumerate()
            .find( |(_, pi)| pi.tpos == tpos)
            .map( |(i,_)| PlayerId(i) )
    }

    pub fn nteams(&self) -> usize {
        match self.nplayers {
            1 => 1,
            2 | 4 => 2,
            _ => panic!("invalid number of players"),
        }
    }

    pub fn team_tpos(&self, i: usize) -> Vec<PlayerTpos> {
        match (self.nplayers, i) {
            (1, 0) => vec![PlayerTpos(0)],

            (2, 0) => vec![PlayerTpos(0)],
            (2, 1) => vec![PlayerTpos(1)],

            (4, 0) => vec![PlayerTpos(0), PlayerTpos(2)],
            (4, 1) => vec![PlayerTpos(1), PlayerTpos(3)],
            _ => panic!("Unexpected number of players/teams"),
        }
    }

    pub fn disconnected_players(&self) -> Vec<PlayerId> {
        self.players
            .iter()
            .enumerate()
            .filter_map(|(i,p)| match p.connected { true => None, false => Some(PlayerId(i)) })
            .collect()
    }
}
