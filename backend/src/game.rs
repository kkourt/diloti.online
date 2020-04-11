//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

// PlayerTaskId is the id we give to the player task so that when we get a request we can identify
// who is the player that is coming from.
//
// PlayerId is the id that identifies a player in the LobbyInfo structure. If the id for a player
// changes, we need to send LobbyUpdate.
//
// As players are registered, we add them to a vector. For now, players cannot be removed, only
// disconnected. The index of a player in the vector is their PlayerId. If we choose to remove
// players, we need to update lobby info.

use std::collections::{VecDeque};

use tokio::sync::{oneshot, mpsc};
use core::srvcli;

use crate::{
    game_task::{GameReq, GameTaskRx, GameTaskTx, PlayerTaskId},
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
    //player_info: srvcli::PlayerInfo, // players referenced by player id
    tpos: srvcli::PlayerTpos,
    name: String,
    task: Option<(PlayerTaskId, PlayerTaskTx)>, // disconnected player have None
}

#[derive(Debug, Clone)]
enum State {
    InLobby,
    InGame,
}

struct Game {
    players: Vec<Player>, // player players[0] is admin
    self_rx: GameTaskRx,
    dir_tx: DirTaskTx,
    gid: GameId,
    state: State,
    curr_game: core::Game<Rng>,
    nplayers: u8,

    nteams: u8,
    next_player_task_id: usize,
    available_tpos: VecDeque<srvcli::PlayerTpos>,
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
            _ => panic!("Unexpected number of players"),
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
            next_player_task_id: 0,
            available_tpos: (0..nplayers).map(|x| srvcli::PlayerTpos(x)).collect(),

        }
    }

    fn ndisconnected(&self) -> usize {
        self.players.iter().filter(|p| p.is_disconnected()).count()
    }

    fn nconnected(&self) -> usize {
        self.players.iter().filter(|p| p.is_connected()).count()
    }

    fn all_disconnected(&self) -> bool {
        self.players.iter().all(|p| p.is_disconnected())
    }

    pub fn new_ptid(&mut self) -> PlayerTaskId {
        let ret = PlayerTaskId(self.next_player_task_id);
        self.next_player_task_id += 1;
        ret
    }

    /// Add a new player, and return its task id
    fn new_player(
        &mut self,
        ptx: PlayerTaskTx,
        mut player_name: String
    ) -> PlayerTaskId {

        while self.players.iter().find(|p| p.name == player_name).is_some() {
            player_name.push('_')
        }

        // TODO: modify player_name to be unique if it is not
        let ptid = self.new_ptid();
        let tpos = self.available_tpos.pop_front().expect("Available tpos");
        let player = Player {
            tpos: tpos,
            name: player_name,
            task: Some((ptid.clone(), ptx)),
        };
        self.players.push(player);

       ptid
    }

    fn player_by_ptid(&self, arg: PlayerTaskId) -> Option<&Player> {
        self.players.iter().position(|p: &Player|
            p.task.as_ref().map_or(false, |(ptid, _tx)| *ptid == arg)
        ).map(|idx|
            self.players.get(idx).unwrap()
        )
    }

    fn player_by_ptid_mut(&mut self, arg: PlayerTaskId) -> Option<&mut Player> {
        self.players.iter().position(|p: &Player|
            p.task.as_ref().map_or(false, |(ptid, _tx)| *ptid == arg)
        ).map(move |idx|
            self.players.get_mut(idx).unwrap()
        )
    }

    fn player_pids_iter(&self) -> impl Iterator<Item=srvcli::PlayerId> {
        (0..self.players.len()).map(|i| srvcli::PlayerId(i))
    }

    fn player_by_pid(&mut self, pid: srvcli::PlayerId) -> &Player {
        self.players.get(pid.0).expect("valid PlayerId")
    }

    fn player_by_pid_mut(&mut self, pid: srvcli::PlayerId) -> &mut Player {
        self.players.get_mut(pid.0).expect("valid PlayerId")
    }

    fn is_player_admin(&self, tpid: PlayerTaskId) -> bool {
        match self.players.get(0) {
            None => false,
            Some(p) => p.task.as_ref().map_or(false, |t| t.0 == tpid),
        }
    }

    fn mk_players_info(&self) -> Vec<srvcli::PlayerInfo> {
        self.players.iter()
            .enumerate()
            .map(|(i,p): (usize, &Player)| {
                srvcli::PlayerInfo {
                    admin: i == 0,
                    tpos: p.tpos.clone(),
                    name: p.name.clone(),
                    connected: p.is_connected(),
                }
            }).collect()
    }

    pub async fn send_lobby_update_to_players(&mut self) {
        'outer: loop {
            let players = self.mk_players_info();
            let nplayers = self.nplayers;
            'inner: for pid in self.player_pids_iter() {
                let player = self.player_by_pid_mut(pid);
                if player.is_disconnected() {
                    continue 'inner;
                }
                let linfo = srvcli::LobbyInfo {
                    nplayers: nplayers,
                    players: players.clone(),
                    self_id: pid,
                };

                // If we fail to sent, it means that another player was disconnected. So we restart
                // the loop until all updates are successful (or all clients are disconnected).
                let msg = srvcli::ServerMsg::LobbyUpdate(linfo);
                match player.send_cli_or_disconnect(msg).await {
                    Err(()) => continue 'outer,
                    Ok(()) => (),
                };
            }

            // sent all updates successfully without a disconnect, we are done
            break 'outer;
        }
    }

    async fn send_game_update_to_players(&mut self) -> Result<(), ()> {
        let mut ret = Ok(());
        // NB: can issue them concurrently and await on all of them just once?
        for player in self.players.iter_mut().filter(|p| p.is_connected()) {
            let tpos = player.tpos;
            let view = self.curr_game.get_player_game_view(tpos);
            let msg = srvcli::ServerMsg::GameUpdate(view);
            if let Err(()) = player.send_cli_or_disconnect(msg).await {
                ret = Err(());
            }
        }

        ret
    }

    fn players_ready(&self) -> bool {
        self.nconnected() == (self.nplayers as usize)
    }

    fn swap_player_tpos(&mut self, tpos1: srvcli::PlayerTpos, tpos2: srvcli::PlayerTpos) -> Result<(), String> {
        let p1 = self.players.iter().position(|x| x.tpos == tpos1).ok_or("Failed to find tpos1")?;
        let p2 = self.players.iter().position(|x| x.tpos == tpos2).ok_or("Failed to find tpos2")?;
        assert_eq!(self.players[p1].tpos, tpos1);
        assert_eq!(self.players[p2].tpos, tpos2);
        self.players[p1].tpos = tpos2;
        self.players[p2].tpos = tpos1;
        Ok(())
    }

    async fn register_player(&mut self, pl_tx: PlayerTaskTx, name: String) {
        use PlayerTaskMsg::RegistrationResult;

        let res = match self.state {
            State::InLobby => {
                let free_slots = self.players.len() < (self.nplayers as usize);
                if free_slots {
                    let ptid = self.new_player(pl_tx, name);
                    Ok(ptid)
                } else {
                    Err((pl_tx, "Too many players.".to_string()))
                }
            },

            State::InGame => {
                Err((pl_tx, "Cannot register while game in progress.".to_string()))
            }
        };

        // handle error
        let ptid = match res {
            Err((mut tx,e)) => {
                if let Err(x) = tx.send(RegistrationResult(Err(e))).await {
                    log::warn!("Error sending erroneous registration result to player task")
                }
                return;
            },
            Ok(x) => x
        };

        // success!
        self.player_by_ptid_mut(ptid.clone())
            .expect("valid ptid")
            .send_task_or_disconnect(RegistrationResult(Ok(ptid.clone())))
            .await
            .map_or((), |_| ()); // ignore error

        self.send_lobby_update_to_players().await;
    }

    async fn apply_action(&mut self, tpid: PlayerTaskId, action: core::PlayerAction) -> Result<(), ()> {
        let tpos = self.player_by_ptid(tpid.clone()).expect("valid tpid").tpos.clone();
        let pview = self.curr_game.get_player_game_view(tpos);

        // validate action
        if let Err(errmsg) = action.validate(&pview) {
            let player = self.player_by_ptid_mut(tpid).expect("valid tpid");
            let msg = srvcli::ServerMsg::InvalidAction(errmsg);
            return player.send_cli_or_disconnect(msg).await;
        }

        // apply action
        let res = self.curr_game.apply_action(tpos, action);
        if let Err(errmsg) = res {
            log::error!("Applying action failed (but validation succeeded): {}", errmsg);
            let player = self.player_by_ptid_mut(tpid).expect("valid tpid");
            let msg = srvcli::ServerMsg::InvalidAction(errmsg);
            return player.send_cli_or_disconnect(msg).await;
        }

        // action was applied successfully
        let newgame = res.unwrap();
        self.curr_game = newgame;

        match self.curr_game.state() {
            core::GameState::NextTurn(_) => (),
            core::GameState::GameDone(_) => (),
            core::GameState::RoundDone => self.curr_game.new_round(),
        }

        self.send_game_update_to_players().await
    }

    async fn handle_clireq(&mut self, ptid: PlayerTaskId, climsg: srvcli::ClientMsg) -> Result<(), ()> {
        use State::{InLobby, InGame};
        use srvcli::ClientMsg::{StartGame, SwapTpos, PlayerAction};

        match (self.state.clone(), climsg) {
            (InLobby, SwapTpos(tpos1, tpos2)) => {
                if !self.is_player_admin(ptid) {
                    log::error!("Non-admin player attempted to swap positions. Ignoring.");
                    return Ok(());
                }

                if !self.players_ready() {
                    log::error!("admin attempted to swap positions while players are not ready. Ignoring.");
                    return Ok(());
                }

                match self.swap_player_tpos(tpos1.clone(), tpos2.clone()) {
                    Ok(()) => (),
                    Err(x) => log::error!("Failed to switch player tpos tpos1:{:?} tpos2:{:?} err:{}", tpos1, tpos2, x),
                }
                self.send_lobby_update_to_players().await;
                Ok(())
            },

            (st, StartGame) => {
                if !self.is_player_admin(ptid) {
                    log::error!("Non-admin player attempted to start game. Ignoring.");
                    return Ok(());
                }

                if !self.players_ready() {
                    log::error!("admin attempted to start game but players are not ready. Ignoring.");
                    return Ok(());
                }

                match st {
                    InLobby => {
                        self.state = InGame;
                        self.send_game_update_to_players().await
                    },

                    InGame if self.curr_game.state().is_game_done() => {
                        self.curr_game.next_game();
                        self.send_game_update_to_players().await
                    },

                    InGame => {
                        log::error!("Trying to start game but game not done. Ignoring.");
                        Ok(())
                    },
                }

            },

            (InGame, PlayerAction(action)) => {
                self.apply_action(ptid, action).await
            },

            (st, msg) => {
                log::error!("Received message: {:?} from client while state is {:?}. Ignoring.", msg, st);
                Ok(())
            }
        }
    }

    async fn task(mut self, rep_tx: oneshot::Sender<srvcli::CreateRep>) {
        self.task_init(rep_tx).await;

        while let Some(cmd) = self.self_rx.recv().await {
            match cmd {
                GameReq::RegisterPlayer(pl_tx, name) => {
                    self.register_player(pl_tx, name).await;
                },

                GameReq::ClientReq(ptid, climsg) => {
                    if let Err(()) = self.handle_clireq(ptid, climsg).await {
                        // There was a sent error and the latest LobbyInfo structure sent to the
                        // players is not up-to-date with respect to disconnects.
                        self.send_lobby_update_to_players().await;
                    }
                }

                GameReq::PlayerTaskTerminated(ptid) => {
                    let p = self.player_by_ptid_mut(ptid).expect("valid ptid");
                    p.task = None;
                    self.send_lobby_update_to_players().await;
                }

                GameReq::ReconnectPlayer(pl_tx, name) => {
                    unimplemented!()
                }
            };

            // check if we have to terminate the game
            // NB: we also need a timeout here for created games that nobody ever joined
            if self.nplayers > 0 && self.all_disconnected() {
                break
            }
        }

        // We are done!
        // send message GameFinished(
        // TODO: close receiving channel
        // TODO: unregister from directory
        // return...
        unimplemented!()
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

impl Player {
    async fn send_task_or_disconnect(&mut self, msg: PlayerTaskMsg) -> Result<(), ()> {
        if let Some((id, tx)) = self.task.as_mut() {
            if let Err(x) = tx.send(msg).await {
                log::warn!("Error sending msg to player task {:?}: {:?}", id, x);
                self.task = None;
                Err(())
            } else {
                Ok(())
            }
        } else {
            log::warn!("Error sending msg to disconnected task");
            Err(())
        }
    }

    async fn send_cli_or_disconnect(&mut self, srvmsg: srvcli::ServerMsg) -> Result<(), ()> {
        let msg = PlayerTaskMsg::ForwardToClient(srvmsg);
        self.send_task_or_disconnect(msg).await
    }

    fn is_connected(&self) -> bool {
        self.task.is_some()
    }

    fn is_disconnected(&self) -> bool {
        self.task.is_none()
    }
}
