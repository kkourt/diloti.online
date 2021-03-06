//#[macro_use]
extern crate log;
extern crate rand;
extern crate rand_pcg;
extern crate serde;
extern crate serde_json;
extern crate percent_encoding;

mod directory_task;
mod game_task;
mod player_task;
mod directory;
mod player;
mod game;
mod chararr_id;

use percent_encoding::percent_decode_str;

// use futures::future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio;
use tokio::sync::{mpsc, oneshot};
use warp;
use warp::Filter;

use core::srvcli;

// Notes:
// There is the directory actor that controls a mapping from id -> games
// There is a game actor which manages the state for every game
// There is a players actor that manages the state for every client (player)
//
// URLs:
// create game: /create_game -> <game_id>
// connect to game: /ws/<game_id>/<player_name>

fn rep_with_internal_error<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
    return warp::reply::with_status(reply, code);
}

#[allow(dead_code)]
fn rep_with_unauthorized<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::UNAUTHORIZED;
    return warp::reply::with_status(reply, code);
}

fn rep_with_ok<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::OK;
    return warp::reply::with_status(reply, code);
}

#[allow(dead_code)]
fn rep_with_conflict<T: warp::Reply>(reply: T) -> warp::reply::WithStatus<T> {
    let code = warp::http::StatusCode::CONFLICT;
    return warp::reply::with_status(reply, code);
}

async fn create_game(req: srvcli::CreateReq, mut dir_tx: mpsc::Sender<directory_task::DirReq>)
-> Result<impl warp::Reply, std::convert::Infallible> {

    let cnf : game::GameConfig = req.into();

    // contact directory task to create a new game
    let (tx, rx) = oneshot::channel::<srvcli::CreateRep>();
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


// game handler

#[tokio::main]
async fn main() {
    env_logger::init();
    let log = warp::log("dilotionline::backend");

    // channel to directory task
    let dir_tx = directory::spawn_directory_task();

    // route: /
    let index_r = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("frontend/index.html")); // encoding does not seem to work here
    /*
    let index: std::borrow::Cow<str> = match std::fs::read_to_string("frontend/index.html") {
        Err(x) => {
            log::error!("Failed to open index file: {:?}", x);
            return
        },
        Ok(x) => x,
    }.into();

    let index_r = warp::get()
        .and(
            warp::path::end().map(move || {
                let index_ = index.clone();
                warp::http::Response::builder()
                    .header("content-type", "text/html; charset=utf-8")
                    .body(index_)
            })
        );
    */

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

    // /ingame is an internal thing used by the frontend. If we get a request for it (e.g., because
    // the user reloaded the page) just redirect them to /.
    let ingame_r = warp::path("ingame")
        .map(|| { warp::redirect(warp::http::Uri::from_static("/")) });

    // GET /ws/:game_id:/:player_name:/ -> websocket for joining the game
    let connect_r = warp::path("ws")
        .and(warp::path::param())
        .and(warp::path::param())
        .and(warp::ws()) // prepare the websocket handshake
        .and_then(
            move |game_id, player_name: String, ws| {
                let pname = percent_decode_str(&player_name).decode_utf8_lossy().to_string();
                player::player_setup(game_id, ws, pname, dir_tx.clone())
            }
        );

    let routes = index_r
        .or(hello_r)
        .or(ingame_r)
        .or(pkg_r)
        .or(create_r)
        .or(connect_r)
        .with(log);
    let sockaddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    warp::serve(routes).run(sockaddr).await;
}
