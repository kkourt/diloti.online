#[macro_use]
extern crate log;
extern crate rand;
extern crate serde;
extern crate serde_json;

use rand::{distributions::Alphanumeric, Rng};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use log::Level;

use futures::future;
use tokio;
use tokio::sync::{mpsc, oneshot};
use warp;
use warp::Filter;

use backend::common::{CreateGameRep, CreateGameReq};

// Here's the idea.
// We want to build a server for playing a game
// The game is simple: each player takes actions in incrementing a counter
// each game has two players.
//
// There is the directory actor that controls a mapping from id -> games
// There is a game actor which manages the state for every game
// There is a client actor that manages the state for every client
//
// There is no real authentication. Everything happens with private URLs
// A client that creates a game gets a management URL for the game.
//  Within this URL, the managing client can sent invitations to other clients (via unique URLs)

// /create_game -> <game_id>
//
// /game/<game_id>/<management_id>
// /game/<game_id>/<player1_id>
// /game/<game_id>/<player2_id>

macro_rules! impl_char_id {
    ($t:ident, $l:expr) => {

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        struct $t([char; $l]);

        impl $t {
            pub fn len() -> usize { $l }

            pub fn new_random() -> Self {
                // NB: we could use the MaybeUninitialized stuff to avoid initalization, but I
                // think it's a bit too much
                let mut rarr: [char; $l] = ['x'; $l];
                let iter = rand::thread_rng() .sample_iter(&Alphanumeric) .take(rarr.len());
                for (i, c) in iter.enumerate() {
                    rarr[i] = c;
                }

                Self(rarr)
            }

            pub fn from_string(s: &String) -> Option<Self> {
                if s.len() != $l {
                    return None
                }

                // NB: we could use the MaybeUninitialized stuff to avoid initalization, but I
                // think it's a bit too much
                let mut arr: [char; $l] = ['y'; $l];
                for (i,c) in s.chars().enumerate() {
                    arr[i] = c;
                }

                Some(Self(arr))
            }

            pub fn to_string(&self) -> String {
                self.0.iter().cloned().collect::<String>()
            }
        }
    };
}

impl_char_id!(PlayerAuthId, 16);
impl_char_id!(GameId, 16);

struct Player {
    /// table location
    pub tloc: u8,

    /// auth id
    auth_id: PlayerAuthId,
}


/**
 * Game structures
 */

#[derive(Debug)]
pub struct GameConfig {
    nplayers: usize,
}

struct Game {
    players: Vec<Player>, // player players[0] is admin
    self_rx: GameTaskRx,
    dir_tx: DirTaskTx,
    gid: GameId,
    cfg: GameConfig,
}

/// Game task requests (includes oneshot channels for replies as needed)
#[derive(Debug)]
enum GameReq {
    RegisterPlayer(oneshot::Sender<Option<>>),
}

/// A channel to send requests (GameReq) to the game task
type GameTaskTx = tokio::sync::mpsc::Sender<GameReq>;
/// A channel to receive game requests
type GameTaskRx = tokio::sync::mpsc::Receiver<GameReq>;

/**
 * Directory structures
 */

struct Directory {
    /// ht: maps game ids to the game task's mpsc tx channel
    ht: HashMap<GameId, GameTaskTx>,
    self_rx: DirTaskRx,
    self_tx: DirTaskTx,
}

/// Directory requests (includes oneshot channels for replies as needed)
#[derive(Debug)]
enum DirReq {
    /// Create a new game, return the ID
    CreateGame(GameConfig, oneshot::Sender<CreateGameRep>),
    /// Request the game task for a given game
    GetGameHandle(GameId, oneshot::Sender<Option<GameTaskTx>>),
}

/// A channel to send requests to the directory task
type DirTaskTx = tokio::sync::mpsc::Sender<DirReq>;
/// A channel to receive directory requests
type DirTaskRx = tokio::sync::mpsc::Receiver<DirReq>;

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

    fn get_player_by_id(&self, auth_id: PlayerAuthId) -> Option<&Player> {
        for p in self.players.iter() {
            if p.auth_id == auth_id {
                return Some(p);
            }
        }

        None
    }

    /// add a new player, and return its reference
    /// Fails if we 've already reached the maximum number of players.
    fn new_player(&mut self) -> Option<&Player> {
        let len = self.players.len();

        // no more players allowed
        if len >= self.cfg.nplayers {
            assert_eq!(len, self.cfg.nplayers);
            return None;
        }

        let p_authid = loop {
            let x = PlayerAuthId::new_random();
            if self.get_player_by_id(x).is_none() {
                break x;
            }
        };

        let p = Player {
            tloc: len as u8,
            auth_id: p_authid,
        };
        self.players.push(p);

        Some(&self.players[len])
    }

    async fn task_init(&mut self, rep_tx: oneshot::Sender<CreateGameRep>) {
        // initialization: create the first player and send ther reply
        let game_id  = self.gid.to_string();
        let player = self.new_player().unwrap();
        let auth_id = player.auth_id.to_string();

        let reply = CreateGameRep {
            game_id: game_id,
            auth_id: auth_id,
        };

        if let Err(x) = rep_tx.send(reply) {
            eprintln!("Error sending CreateGameRep reply: {:?}", x);
            // TODO: self destruct or soemthing?
            unimplemented!()
        }
    }

    async fn task(mut self, rep_tx: oneshot::Sender<CreateGameRep>) {
        self.task_init(rep_tx).await;

        while let Some(cmd) = self.self_rx.recv().await {
            match cmd {
                _ => unimplemented!(),
            }
        }

        // TODO: remove self from directory
        unimplemented!()
    }
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
    pub fn new_game(&mut self, cfg: GameConfig, rep_tx: oneshot::Sender<CreateGameRep>) {
        loop {
            let gid = GameId::new_random();
            match self.ht.entry(gid) {
                Entry::Occupied(_) => continue, // retry
                Entry::Vacant(e) => {
                    let (game_tx, game_rx) = tokio::sync::mpsc::channel::<GameReq>(1024);
                    let game = Game::new(gid, cfg, game_rx, self.self_tx.clone());
                    // NB: we are detaching the game task by dropping its handle
                    let _game_task = tokio::spawn(game.task(rep_tx));
                    e.insert(game_tx);
                    return;
                }
            }
        }
    }

    pub fn get_game_handle(&self, gid: GameId, rep_tx: oneshot::Sender<Option<GameTaskTx>>) {
        let rep : Option<GameTaskTx> = self.ht.get(&gid).map(|v| v.clone());
        if let Err(x) = rep_tx.send(rep) {
            error!("Error sending game handle")
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

fn with_internal_error<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
    return warp::reply::with_status(reply, code);
}

fn with_unauthorized<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::UNAUTHORIZED;
    return warp::reply::with_status(reply, code);
}

fn with_ok<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::OK;
    return warp::reply::with_status(reply, code);
}

async fn create_game(mut dir_tx: mpsc::Sender<DirReq>)
-> Result<impl warp::Reply, std::convert::Infallible> {
    let cnf = GameConfig { nplayers: 2 };

    // contact directory task to create a new game
    let (tx, rx) = oneshot::channel::<CreateGameRep>();
    if let Err(x) = dir_tx.send(DirReq::CreateGame(cnf, tx)).await {
        error!("Error sending CreateGame request: {:?}", x);
        return Ok(with_internal_error(String::from("")))
    }

    // recceive reply from directory task
    if let Ok(ret) = rx.await {
        Ok(with_ok(serde_json::to_string(&ret).unwrap()))
    } else {
        error!("Error receiving result from directory");
        Ok(with_internal_error(String::from("")))
    }
}

async fn create_ws(
    game_id_s: String, auth_id_s: String,
    ws: warp::ws::Ws,
    mut dir_tx: mpsc::Sender<DirReq>)
-> Result<impl warp::Reply, std::convert::Infallible> {

    let invalid_reply = |s| {
        let code = warp::http::StatusCode::UNAUTHORIZED;
        warp::reply::with_status(s, code)
    };

    let auth_id = match PlayerAuthId::from_string(&auth_id_s) {
        None => return Ok(invalid_reply("invalid auth id")),
        Some(x) => x,
    };

    let game_id = match GameId::from_string(&game_id_s) {
        None => return Ok(invalid_reply("invalid game id")),
        Some(x) => x,
    };

    // contact directory server to get the tx endpoint for the game
    let (tx, rx) = oneshot::channel::<Option<GameTaskTx>>();
    if let Err(x) = dir_tx.send(DirReq::GetGameHandle(game_id, tx)).await {
        eprintln!("Error send CreateGame request: {:?}", x)
    }

    let game_tx: GameTaskTx = if let Ok(Some(x)) = rx.await {
        x
    } else {
        error!("Failed to response result from directory");
        return Ok(invalid_reply("invalid game id"));
    };

    let code_ok = warp::http::StatusCode::OK;
    Ok(warp::reply::with_status("", code_ok))
}


// game handler

#[tokio::main]
async fn main() {
    env_logger::init();
    let log = warp::log("dilotionline::backend");

    // Directory task
    // channel to directory
    let (dir_tx, dir_rx) = tokio::sync::mpsc::channel::<DirReq>(1024);
    let dir = Directory::new(dir_rx, dir_tx.clone());
    let dir_task = tokio::spawn(dir.task());

    // route: /hello
    let hello_r = warp::path("hello").map(|| "Hello! I love pizza!".to_string());
    // route: /creategame
    let create_r = {
        let dir_tx_ = dir_tx.clone();
        warp::path("creategame")
            .and(warp::put())
            .and_then(move || { create_game(dir_tx_.clone()) })
    };

    // route: /
    let index_r = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("frontend/index.html"));

    // route: /pkg
    let pkg_r = warp::path("pkg").and(warp::fs::dir("frontend/pkg/"));

    // GET /ws -> websocket for playing the game
    let ws = warp::path("ws")
        .and(warp::header("game_id"))
        .and(warp::header("auth_id"))
        .and(warp::ws()) // prepare the websocket handshake
        .and_then(
            move |game_id, auth_id, ws| {
                create_ws(game_id, auth_id, ws, dir_tx.clone())
            }
        );


    let routes = index_r.or(pkg_r).or(hello_r).or(create_r).with(log);
    let sockaddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    warp::serve(routes).run(sockaddr).await;
}
