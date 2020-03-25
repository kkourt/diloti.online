// XXX: until code stabilizes...
#![allow(dead_code)]
#![allow(unused_variables)]

//#[macro_use]
extern crate log;
extern crate rand;
extern crate serde;
extern crate serde_json;

mod directory_task;
mod game_task;
mod player_task;
mod directory;
mod player;
mod game;
mod chararr_id;

// use futures::future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio;
use tokio::sync::{mpsc, oneshot};
use warp;
use warp::Filter;

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


fn rep_with_internal_error<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
    return warp::reply::with_status(reply, code);
}

fn rep_with_unauthorized<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::UNAUTHORIZED;
    return warp::reply::with_status(reply, code);
}

fn rep_with_ok<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::OK;
    return warp::reply::with_status(reply, code);
}

fn rep_with_conflict<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::CONFLICT;
    return warp::reply::with_status(reply, code);
}

async fn create_game(req: common::CreateReq, mut dir_tx: mpsc::Sender<directory_task::DirReq>)
-> Result<impl warp::Reply, std::convert::Infallible> {
    let cnf = game::GameConfig { nplayers: req.nplayers };

    // contact directory task to create a new game
    let (tx, rx) = oneshot::channel::<common::CreateRep>();
    if let Err(x) = dir_tx.send(directory_task::DirReq::CreateGame(cnf, tx)).await {
        log::error!("Error sending CreateGame request: {:?}", x);
        return Ok(rep_with_internal_error(String::from("")))
    }

    // recceive reply from directory task
    if let Ok(ret) = rx.await {
        Ok(rep_with_ok(serde_json::to_string(&ret).unwrap()))
    } else {
        log::error!("Error receiving result from directory");
        Ok(rep_with_internal_error(String::from("")))
    }
}

async fn start_player(
    game_id_s: String,
    player_name: String,
    ws: warp::ws::Ws,
    mut dir_tx: mpsc::Sender<directory_task::DirReq>,
) -> Result<Box<dyn warp::Reply>, std::convert::Infallible> {

    // shortcuts for some replies
    // NB: we a trait object to have a common return type. Not sure if there is a better way.
    let rep_with_code    = |s,c| Ok(Box::new(warp::reply::with_status(s,c)) as Box<dyn warp::Reply>);
    let rep_unauthorized = |s|   Ok(Box::new(rep_with_unauthorized(s)) as Box<dyn warp::Reply>);
    let rep_error        = |s|   Ok(Box::new(rep_with_internal_error(s)) as Box<dyn warp::Reply>);

    let game_id = match game::GameId::from_string(&game_id_s) {
        None => return rep_unauthorized("invalid game id"),
        Some(x) => x,
    };

    // contact directory server to get the tx endpoint for the game task
    let game_tx: game_task::GameTaskTx = {
        // create a oneshot channel for the reply
        let (tx, rx) = oneshot::channel::<Option<game_task::GameTaskTx>>();
        if let Err(x) = dir_tx.send(directory_task::DirReq::GetGameHandle(game_id, tx)).await {
            eprintln!("Error sedding CreateGame request: {:?}", x);
            return rep_error("Failed to register player to game");
        }

        match rx.await {
            Ok(Some(x)) => x,
            Ok(None) => return rep_unauthorized("invalid game id"),
            Err(e) => {
                log::error!("Failed to get result from directory: {:?}", e);
                return rep_error("Failed to register player to game");
            }
        }
    };

    match player::player_setup(ws, game_tx, player_name).await {
        Err(code) => rep_with_code("Error registering player into game", code),
        Ok(rep) => Ok(Box::new(rep)),
    }
}


// game handler

#[tokio::main]
async fn main() {
    env_logger::init();
    let log = warp::log("dilotionline::backend");

    // channel to directory task
    let dir_tx = directory::spawn_directory_task();

    //
    // route: /
    let index_r = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("frontend/index.html"));

    // route: /pkg
    let pkg_r = warp::path("pkg").and(warp::fs::dir("frontend/pkg/"));


    // route: /hello
    let hello_r = warp::path("hello").map(|| "Hello! I love pizza!".to_string());

    // route: /creategame
    let create_r = {
        let dir_tx_ = dir_tx.clone();
        warp::path("creategame")
            .and(warp::put())
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .and_then(move |req| { create_game(req, dir_tx_.clone()) })
    };

    // GET /ws/:game_id: -> websocket for joining the game
    let connect_r = warp::path("ws")
        .and(warp::path::param())
        .and(warp::path::param())
        .and(warp::ws()) // prepare the websocket handshake
        .and_then(move |game_id, player_name, ws| start_player(game_id, player_name, ws, dir_tx.clone()) );


    let routes = index_r.or(pkg_r).or(hello_r).or(create_r).or(connect_r).with(log);
    let sockaddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    warp::serve(routes).run(sockaddr).await;
}
