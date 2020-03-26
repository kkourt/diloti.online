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


#[derive(Debug)]
pub struct GameConfig {
    pub nplayers: u8,
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
    cfg: GameConfig,
    state: State,

    curr_game: core::Game<Rng>,
}

impl Game {
    pub fn new(gid: GameId, cfg: GameConfig, self_rx: GameTaskRx, dir_tx: DirTaskTx) -> Game {

        let rng = Rng::from_rng(rand::rngs::OsRng).expect("unable to initalize RNG");
        let game = match cfg.nplayers {
            1 => unimplemented!(),
            2 => core::Game::new_2p(rng),
            4 => core::Game::new_4p(rng),
            _ => panic!("Incorrect number of players"),
        };

        Game {
            gid: gid,
            players: vec![],
            self_rx: self_rx,
            dir_tx: dir_tx,
            cfg: cfg,
            state: State::InLobby,
            curr_game: game,
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
        if len >= self.cfg.nplayers as usize {
            assert_eq!(len, self.cfg.nplayers as usize);
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
    pub async fn send_info_to_players(&mut self) {
        match self.state {
            State::InLobby => {
                let players : Vec<srvcli::PlayerInfo>
                    = self.players.iter().map(|x: &Player| x.player_info.clone()).collect();
                for pid in 0..self.players.len() {
                    let player = &mut self.players[pid];
                    let climsg = srvcli::ServerMsg::InLobby(srvcli::LobbyInfo {
                        nplayers: self.cfg.nplayers,
                        players: players.clone(),
                        self_id: srvcli::PlayerId(pid),
                    });
                    let msg = PlayerTaskMsg::ForwardToClient(climsg);
                    if let Err(x) = player.tx.send(msg).await {
                        eprintln!("Error sending msg to player {:?}", x);
                        // TODO: remove player or retry
                        unimplemented!();
                    }
                }
            },

            State::InGame => {
                for player in self.players.iter_mut() {
                    let tpos = player.player_info.tpos;
                    let player_view = self.curr_game.get_player_game_view(tpos);
                    let climsg = srvcli::ServerMsg::InGame(srvcli::GameInfo(player_view));
                    let msg = PlayerTaskMsg::ForwardToClient(climsg);
                    if let Err(x) = player.tx.send(msg).await {
                        eprintln!("Error sending msg to player {:?}", x);
                        // TODO: remove player or retry
                        unimplemented!();
                    }
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

    fn players_ready(&self) -> bool {
        self.players.len() == self.cfg.nplayers as usize
    }

    async fn handle_clireq(&mut self, pid: srvcli::PlayerId, climsg: srvcli::ClientMsg) {
        match self.state {
            State::InLobby => match climsg {
                srvcli::ClientMsg::InLobby(srvcli::LobbyReq::StartGame) => {
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
            },

            _ => unimplemented!(),
        }
    }

    async fn task_init(&mut self, rep_tx: oneshot::Sender<srvcli::CreateRep>) {
        // initialization: create the first player and send ther reply
        let game_id  = self.gid.to_string();
        let reply = srvcli::CreateRep { game_id: game_id };

        if let Err(x) = rep_tx.send(reply) {
            eprintln!("Error sending CreateRep reply: {:?}", x);
            // TODO: self destruct or soemthing?
            unimplemented!()
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
