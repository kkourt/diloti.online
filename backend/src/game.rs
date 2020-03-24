//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use tokio::sync::{oneshot, mpsc};

use crate::{
    game_task::{GameReq, GameTaskRx, GameTaskTx},
    player_task::{PlayerTaskMsg, PlayerTaskTx},
    directory_task::DirTaskTx,
};

/**
 * Backend-side game structures
 */

pub use crate::chararr_id::GameId;

#[derive(Debug, Clone, Copy)]
pub struct PlayerId(usize); // Player (internal) identifier

#[derive(Debug)]
pub struct GameConfig {
    pub nplayers: u8,
}

struct Player {
    player_info: common::PlayerInfo,
    tx: PlayerTaskTx,
}

#[derive(Debug)]
pub struct GameScore;

struct Game {
    players: Vec<Player>, // player players[0] is admin
    self_rx: GameTaskRx,
    dir_tx: DirTaskTx,
    gid: GameId,
    cfg: GameConfig,

    // curr_game: Option<core::Game>,
    //score: GameScore,
}

impl Game {
    pub fn new(gid: GameId, cfg: GameConfig, self_rx: GameTaskRx, dir_tx: DirTaskTx) -> Game {
        Game {
            gid: gid,
            players: vec![],
            self_rx: self_rx,
            dir_tx: dir_tx,
            cfg: cfg,
        }
    }

    fn get_player_by_id(&self, id: usize) -> Option<&Player> {
        self.players.get(id)
    }

    /// add a new player, and return its reference
    /// Fails if we 've already reached the maximum number of players.
    fn new_player(&mut self, ptx: &PlayerTaskTx) -> Result<PlayerId, String> {

        let len = self.players.len();

        // no more players allowed
        if len >= self.cfg.nplayers as usize {
            assert_eq!(len, self.cfg.nplayers as usize);
            return Err(String::from(format!("Game is already full ({} players)", len)));
        }

        let p = Player {
            player_info: common::PlayerInfo { tpos: len as u8, admin: len == 0, },
            tx: ptx.clone(),
        };
        self.players.push(p);

        Ok(PlayerId(len))
    }

    async fn task(mut self, rep_tx: oneshot::Sender<common::CreateRep>) {
        self.task_init(rep_tx).await;

        while let Some(cmd) = self.self_rx.recv().await {
            match cmd {
                GameReq::RegisterPlayer(mut pl_tx) => {
                    let ret = self.new_player(&pl_tx);
                    let rep = PlayerTaskMsg::RegistrationResult(ret);
                    if let Err(x) = pl_tx.send(rep).await {
                        eprintln!("Error sending RegisterPlayer reply: {:?}", x);
                        // TODO: remove player if they were registered (?)
                        unimplemented!()
                    }
                }
            }
        }

        // TODO: remove self from directory
        unimplemented!()
    }

    async fn task_init(&mut self, rep_tx: oneshot::Sender<common::CreateRep>) {
        // initialization: create the first player and send ther reply
        let game_id  = self.gid.to_string();
        let reply = common::CreateRep { game_id: game_id };

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
    rep_tx: oneshot::Sender<common::CreateRep>,
) -> GameTaskTx {
    let (game_tx, game_rx) = mpsc::channel::<GameReq>(1024);
    let game = Game::new(gid, cfg, game_rx, dir_tx);
    // NB: we are detaching the game task by dropping its handle
    let game_task = tokio::spawn(game.task(rep_tx));
    game_tx
}
