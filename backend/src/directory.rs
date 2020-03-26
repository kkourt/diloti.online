//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use tokio::sync::oneshot;

use core::srvcli;
use crate::{
    game::{GameId, GameConfig, spawn_game_task},
    game_task::{GameTaskTx},
    directory_task::{DirReq, DirTaskRx, DirTaskTx},
};

/**
 * Directory structures
 */

struct Directory {
    /// ht: maps game ids to the game task's mpsc tx channel
    ht: HashMap<GameId, GameTaskTx>,
    self_rx: DirTaskRx,
    self_tx: DirTaskTx,
}

impl Directory {
    pub fn new(rx: DirTaskRx, tx: DirTaskTx) -> Directory {
        Directory {
            ht: HashMap::new(),
            self_rx: rx,
            self_tx: tx,
        }
    }

    // create a new game:
    //  - add an entry to the directory
    //  - spawn a task for the game with a mpsc channel, and keep the tx end in the table
    pub fn new_game(&mut self, cfg: GameConfig, rep_tx: oneshot::Sender<srvcli::CreateRep>) {
        loop {
            let gid = GameId::new_random();
            match self.ht.entry(gid) {
                Entry::Occupied(_) => continue, // retry
                Entry::Vacant(e) => {
                    let game_tx = spawn_game_task(gid, cfg, self.self_tx.clone(), rep_tx);
                    e.insert(game_tx);
                    /*
                    let (game_tx, game_rx) = mpsc::channel::<GameReq>(1024);
                    let game = Game::new(gid, cfg, game_rx, self.self_tx.clone());
                    // NB: we are detaching the game task by dropping its handle
                    let _game_task = tokio::spawn(game.task(rep_tx));
                    e.insert(game_tx);
                    */
                    return;
                }
            }
        }
    }

    pub fn get_game_handle(&self, gid: GameId, rep_tx: oneshot::Sender<Option<GameTaskTx>>) {
        let rep : Option<GameTaskTx> = self.ht.get(&gid).map(|v| v.clone());
        if let Err(x) = rep_tx.send(rep) {
            log::error!("Error sending game handle")
        }
    }


    async fn task(mut self) {
        while let Some(cmd) = self.self_rx.recv().await {
            match cmd {
                DirReq::CreateGame(cfg, rep_tx) => {
                    self.new_game(cfg, rep_tx);
                }

                DirReq::GetGameHandle(gid, rep_tx) => {
                    self.get_game_handle(gid, rep_tx);
                }
            }
        }
    }
}


pub fn spawn_directory_task() -> DirTaskTx {
    let (dir_tx, dir_rx) = tokio::sync::mpsc::channel::<DirReq>(1024);
    let dir = Directory::new(dir_rx, dir_tx.clone());
    let dir_task = tokio::spawn(dir.task());
    dir_tx
}
