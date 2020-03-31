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
    hand: Vec<core::Card>,
    table: Vec<core::Card>,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub nplayers: u8,
    pub debug: Option<GameDebug>,
}

struct Player {
    player_info: srvcli::PlayerInfo,
    tx: PlayerTaskTx,
}

#[derive(Debug)]
pub struct GameScore;

enum State {
    InLobby,
    InGame,
    GameEnd,
    Error(String),
}

struct Game {
    players: Vec<Player>, // player players[0] is admin
    self_rx: GameTaskRx,
    dir_tx: DirTaskTx,
    gid: GameId,
    state: State,
    curr_game: core::Game<Rng>,
    nplayers: u8,
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
            (x, _)          => panic!("Debuging mode allowed only for single player (for now)."),
        };

        Game {
            gid: gid,
            players: vec![],
            self_rx: self_rx,
            dir_tx: dir_tx,
            state: State::InLobby,
            curr_game: game,
            nplayers: nplayers,
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

            _ => unimplemented!(),
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

    fn is_players_turn(&self, pid: srvcli::PlayerId) -> bool {
        unimplemented!()
    }

    fn players_ready(&self) -> bool {
        self.players.len() == self.nplayers as usize
    }

    async fn handle_clireq(&mut self, pid: srvcli::PlayerId, climsg: srvcli::ClientMsg) {
        match self.state {

            State::InLobby => match climsg {
                srvcli::ClientMsg::StartGame => {
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
                }

                srvcli::ClientMsg::PlayerAction(action) => unimplemented!(),
            },

            State::InGame => match climsg {
                srvcli::ClientMsg::StartGame => unimplemented!(),
                srvcli::ClientMsg::PlayerAction(action) => {
                    let tpos = self.player_tpos(pid);
                    let pview = self.curr_game.get_player_game_view(tpos);
                    let player = self.get_player_mut(pid);

                    if let Err(errmsg) = pview.validate_action(&action) {
                        let msg = srvcli::ServerMsg::InvalidAction(errmsg);
                        self.send_msg_to_pid(pid, msg).await;
                    } else {
                        let res = self.curr_game.apply_action(tpos, action);
                        match res {
                            Err(x) => unimplemented!(), // TODO: send error message to clients
                            Ok(g) => self.curr_game = g,
                        }

                        self.send_info_to_players().await;
                    }

                }
            }

            _ => unimplemented!(),
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
        let debug_hand = req.get_debug_hand().map(|x| x.to_inner());
        let debug_table = req.get_debug_table().map(|x| x.to_inner());
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
