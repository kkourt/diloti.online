//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use tokio::sync::{oneshot, mpsc};

use core::srvcli;

use crate::{
    game_task::{GameReq, GameTaskRx, GameTaskTx},
    player_task::{PlayerTaskMsg, PlayerTaskTx},
    directory_task::DirTaskTx,
};
use rand::SeedableRng;
type Rng = rand_pcg::Pcg64;

/**
 * Backend-side game structures
 */

pub use crate::chararr_id::GameId;

#[derive(Debug, Clone)]
pub struct GameDebug {
    hand: core::Deck,
    table: core::Table,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub nplayers: u8,
    pub debug: Option<GameDebug>,
}


struct Player {
    player_info: srvcli::PlayerInfo, // players referenced by player id
    tx: PlayerTaskTx,
}

#[derive(Debug, Clone)]
enum State {
    InLobby,
    InGame,
    Error(String),
}

// NB:
// - backend::game::Game players are referenced by PlayerId
// - core::game::Game players are referenced by PlayerTpos
//
// PlayerId is assigned by the game task when a client joins, whule playertops is their location on
// the table that determines teams, player order, etc.


struct Game {
    players: Vec<Player>, // player players[0] is admin
    self_rx: GameTaskRx,
    dir_tx: DirTaskTx,
    gid: GameId,
    state: State,
    curr_game: core::Game<Rng>,
    nplayers: u8,

    nteams: u8,
}

impl Game {
    pub fn new(gid: GameId, cfg: GameConfig, self_rx: GameTaskRx, dir_tx: DirTaskTx) -> Game {

        let rng = Rng::from_rng(rand::rngs::OsRng).expect("unable to initalize RNG");
        let nplayers = cfg.nplayers;
        let game = match (cfg.nplayers, cfg.debug) {
            (1, None)       => core::Game::new_1p(rng),
            (2, None)       => core::Game::new_2p(rng),
            (4, None)       => core::Game::new_4p(rng),
            (x, None)       => panic!("Incorrect number of players: {:?}", x),
            (1, Some(dbg))  => core::Game::new_1p_debug(rng, dbg.table, dbg.hand),
            (x, _)          => panic!("Debugging mode allowed only for single player (for now)."),
        };

        let nteams = match cfg.nplayers {
            2 | 4 => 2,
            1     => 1,
            _ => panic!("Unexepcted number of players"),
        };

        Game {
            gid: gid,
            players: vec![],
            self_rx: self_rx,
            dir_tx: dir_tx,
            state: State::InLobby,
            curr_game: game,
            nplayers: nplayers,
            nteams: nteams,
        }
    }


    /// add a new player, and return its reference
    /// Fails if we 've already reached the maximum number of players.
    fn new_player(
        &mut self,
        ptx: &PlayerTaskTx,
        player_name: String
    ) -> Result<srvcli::PlayerId, String> {

        let len = self.players.len();

        // no more players allowed
        if len >= self.nplayers as usize {
            assert_eq!(len, self.nplayers as usize);
            return Err(String::from(format!("Game is already full ({} players)", len)));
        }

        let p = Player {
            player_info: srvcli::PlayerInfo {
                tpos: srvcli::PlayerTpos(len as u8),
                admin: len == 0,
                name: player_name,
            },
            tx: ptx.clone(),
        };
        self.players.push(p);

        Ok(srvcli::PlayerId(len))
    }

    fn get_player(&self, pid: srvcli::PlayerId) -> &Player {
        self.players.get(pid.0).unwrap()
    }

    fn get_player_mut(&mut self, pid: srvcli::PlayerId) -> &mut Player {
        self.players.get_mut(pid.0).unwrap()
    }

    pub async fn send_msg_to_pid(&mut self, pid: srvcli::PlayerId, srvmsg: srvcli::ServerMsg) {
        let player = self.get_player_mut(pid);
        let msg = PlayerTaskMsg::ForwardToClient(srvmsg);
        if let Err(x) = player.tx.send(msg).await {
            eprintln!("Error sending msg to player {:?}", x);
            // TODO: remove player or retry
            unimplemented!();
        }
    }

    pub fn iter_player_ids(&self) -> impl Iterator<Item=srvcli::PlayerId> {
        (0..self.players.len()).map(|i| srvcli::PlayerId(i))
    }

    pub async fn send_info_to_players(&mut self) {
        match self.state {
            State::InLobby => {
                let players : Vec<srvcli::PlayerInfo>
                    = self.players.iter().map(|x: &Player| x.player_info.clone()).collect();

                for pid in self.iter_player_ids() {
                    let player = self.get_player_mut(pid);
                    let srvmsg = srvcli::ServerMsg::InLobby(srvcli::LobbyInfo {
                        nplayers: self.nplayers,
                        players: players.clone(),
                        self_id: pid,
                    });
                    self.send_msg_to_pid(pid, srvmsg).await;
                }
            },

            State::InGame => {
                for i in 0..self.players.len() {
                    let pid = srvcli::PlayerId(i);
                    let tpos = self.get_player(pid).player_info.tpos;
                    let player_view = self.curr_game.get_player_game_view(tpos);
                    let srvmsg = srvcli::ServerMsg::GameUpdate(player_view);
                    self.send_msg_to_pid(pid, srvmsg).await;
                }
            },

            State::Error(_) => unimplemented!(),
        }
    }

    async fn task(mut self, rep_tx: oneshot::Sender<srvcli::CreateRep>) {
        self.task_init(rep_tx).await;

        while let Some(cmd) = self.self_rx.recv().await {
            match cmd {
                GameReq::RegisterPlayer(mut pl_tx, name) => {
                    // Send a registration result to the player task
                    let ret = self.new_player(&pl_tx, name);
                    let rep = PlayerTaskMsg::RegistrationResult(ret.clone());
                    if let Err(x) = pl_tx.send(rep).await {
                        eprintln!("Error sending RegisterPlayer reply: {:?}", x);
                        // TODO: remove player if they were registered (?)
                        unimplemented!()
                    }

                    if let Ok(pid) = ret {
                        // If registration was succesful, send player game info to everyone.
                        self.send_info_to_players().await;
                    }
                },

                GameReq::ClientReq(pid, climsg) => {
                    self.handle_clireq(pid, climsg).await
                }
            }
        }

        // TODO: remove self from directory
        unimplemented!()
    }

    fn is_player_admin(&self, pid: srvcli::PlayerId) -> bool {
        self.get_player(pid).player_info.admin
    }

    fn player_tpos(&self, pid: srvcli::PlayerId) -> srvcli::PlayerTpos {
        self.get_player(pid).player_info.tpos
    }

    fn swap_player_tpos(&mut self, tpos1: srvcli::PlayerTpos, tpos2: srvcli::PlayerTpos) -> Result<(), String> {
        let p1 = self.players.iter().position(|x| x.player_info.tpos == tpos1).ok_or("Failed to find tpos1")?;
        let p2 = self.players.iter().position(|x| x.player_info.tpos == tpos2).ok_or("Failed to find tpos2")?;
        assert_eq!(self.players[p1].player_info.tpos, tpos1);
        assert_eq!(self.players[p2].player_info.tpos, tpos2);
        self.players[p1].player_info.tpos = tpos2;
        self.players[p2].player_info.tpos = tpos1;
        Ok(())
    }

    fn players_ready(&self) -> bool {
        self.players.len() == self.nplayers as usize
    }

    async fn apply_action(&mut self, pid: srvcli::PlayerId, action: core::PlayerAction) {
        let tpos = self.player_tpos(pid);
        let pview = self.curr_game.get_player_game_view(tpos);
        let player = self.get_player_mut(pid);

        // validate action
        if let Err(errmsg) = action.validate(&pview) {
            let msg = srvcli::ServerMsg::InvalidAction(errmsg);
            self.send_msg_to_pid(pid, msg).await;
            return;
        }

        let res = self.curr_game.apply_action(tpos, action);
        if let Err(errmsg) = res {
            log::error!("Error applying action evern after validation succeeded: {}", errmsg);
            let msg = srvcli::ServerMsg::InvalidAction(errmsg);
            self.send_msg_to_pid(pid, msg).await;
            return;
        }

        // action was applied successfully
        let newgame = res.unwrap();
        self.curr_game = newgame;

        match self.curr_game.state() {
            core::GameState::NextTurn(_) => (),
            core::GameState::GameDone(_) => (),
            core::GameState::RoundDone => self.curr_game.new_round(),
        }

        self.send_info_to_players().await;
    }

    async fn handle_clireq(&mut self, pid: srvcli::PlayerId, climsg: srvcli::ClientMsg) {
        use State::{InLobby, InGame};
        use srvcli::ClientMsg::{StartGame, SwapTpos, PlayerAction};

        match (self.state.clone(), climsg) {
            (InLobby, StartGame) => {
                if !self.is_player_admin(pid) {
                    log::error!("Non-admin player attempted to start game");
                    return;
                }

                if !self.players_ready() {
                    log::error!("admin attempted to start game but players are not ready");
                    return;
                }

                self.state = State::InGame;
                self.send_info_to_players().await;
            },

            (InLobby, SwapTpos(tpos1, tpos2)) => {
                if !self.is_player_admin(pid) {
                    log::error!("Non-admin player attempted to start game");
                    return;
                }

                if !self.players_ready() {
                    log::error!("admin attempted to swap positions while players are not ready");
                    return;
                }

                match self.swap_player_tpos(tpos1.clone(), tpos2.clone()) {
                    Ok(()) => (),
                    Err(x) => log::error!("Failed to switch player tpos tpos1:{:?} tpos2:{:?} err:{}", tpos1, tpos2, x),
                }
                self.send_info_to_players().await;
            },

            (InGame, PlayerAction(action)) => {
                self.apply_action(pid, action).await;
            },

            (st, msg) => log::error!("Received message: {:?} from client while state is {:?}. Ignoring.", msg, st),
        }

    }

    async fn task_init(&mut self, rep_tx: oneshot::Sender<srvcli::CreateRep>) {
        // initialization: create the first player and send ther reply
        let game_id  = self.gid.to_string();
        let reply = srvcli::CreateRep { game_id: game_id };

        if let Err(x) = rep_tx.send(reply) {
            eprintln!("Error sending CreateRep reply: {:?}", x);
            // TODO: self destruct or something?
            unimplemented!()
        }
    }

}

impl From<srvcli::CreateReq> for GameConfig {
    fn from(req: srvcli::CreateReq) -> Self {
        let debug_hand = req.get_debug_hand();
        let debug_table = req.get_debug_table();
        if debug_hand.is_none() || debug_table.is_none() {
            return GameConfig {
                nplayers: req.nplayers,
                debug: None,
            }
        }

        GameConfig {
            nplayers: req.nplayers,
            debug: Some(GameDebug {
                hand: debug_hand.unwrap(),
                table: debug_table.unwrap(),
            }),
        }
    }
}


pub fn spawn_game_task(
    gid: GameId,
    cfg: GameConfig,
    dir_tx: DirTaskTx,
    rep_tx: oneshot::Sender<srvcli::CreateRep>,
) -> GameTaskTx {
    let (game_tx, game_rx) = mpsc::channel::<GameReq>(1024);
    let game = Game::new(gid, cfg, game_rx, dir_tx);
    // NB: we are detaching the game task by dropping its handle
    let game_task = tokio::spawn(game.task(rep_tx));
    game_tx
}
