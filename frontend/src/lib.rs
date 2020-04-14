// XXX: until code stabilizes...
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate rand;
extern crate rand_pcg;
extern crate web_sys;
extern crate wasm_bindgen;
extern crate serde_json;
extern crate url;

mod to_elem;
mod ws;
mod init;
mod lobby;
mod join;
mod game;

use seed::{*, prelude::*};

use init::{InitMsg, InitSt};
use lobby::{LobbyMsg, LobbySt};
use join::{JoinSt, JoinMsg};
use game::{GameSt, InGameMsg};
use ws::{WsEvent};

type XRng = rand_pcg::Pcg64;

const DEFAULT_NR_PLAYERS: u8 = 2;

/// Demultiplexers
// NB: there seem to be some facilities for better handling this demultiplexing:
// https://seed-rs.org/guide/complex-apps, but for now we just ad-hoc it.

pub enum Model {
    Init(InitSt),
    Join(JoinSt),
    InLobby(LobbySt),
    InGame(GameSt),
}

#[derive(Clone,Debug)]
pub enum Msg {
    Init(InitMsg),
    Join(JoinMsg),
    InGame(InGameMsg),
    Lobby(LobbyMsg),
    Ws(WsEvent),
}

pub fn get_string_from_storage(key: &str) -> String {
    if let Some(storage) = seed::storage::get_storage() {
        seed::storage::load_data(&storage, key)
    } else {
        None
    }.unwrap_or("".to_string())
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    //log(format!("update => {:?}", msg));
    let upd_ret = match (&mut model, msg) {
        (&mut Model::Init(st), Msg::Init(ref msg))     => st.update_state(msg, orders),
        (&mut Model::Join(st), Msg::Join(ref msg))     => st.update_state(msg, orders),
        (&mut Model::InLobby(st), Msg::Lobby(ref msg)) => st.update_state(msg, orders),
        (&mut Model::InLobby(st), Msg::Ws(ref msg))    => st.handle_ws_event(msg, orders),
        (&mut Model::InGame(st), Msg::InGame(ref msg)) => st.update_state(msg),
        (&mut Model::InGame(st), Msg::Ws(ref msg))     => st.handle_ws_event(msg, orders),
        _ => panic!("Invalid message for current state"),
    };

    if let Some(newmodel) = upd_ret {
        *model = newmodel;
    }
}

fn view(model: &Model) -> impl View<Msg> {
    match model {
        Model::Init(st) => st.view(),
        Model::Join(st) => st.view(),
        Model::InGame(st) => st.view(),
        Model::InLobby(st) => st.view(),
    }
}

fn after_mount(url: Url, orders: &mut impl Orders<Msg>) -> AfterMount<Model> {
    let href = web_sys::window().unwrap().location().href().expect("href not found");
    let url = url::Url::parse(&href).expect("invalid url");
    let join_game_id = url.query_pairs().find(|(k,_v)| k == "join").map(|(_k,v)| v);

    let player_name = get_string_from_storage("player_name");
    let debug_hand = get_string_from_storage("debug_hand_cards");
    let debug_table = get_string_from_storage("debug_table_cards");
    log(format!("Starting..."));

    if let Some(game_id) = join_game_id {
        let joinst = JoinSt {
            game_id: game_id.to_string(),
            player_name: player_name,
            join_game_err: None,
        };
        AfterMount::new(Model::Join(joinst))
    } else {
        let initst = InitSt {
            nplayers: DEFAULT_NR_PLAYERS,
            player_name: player_name,
            start_game_err: None,
            debug_hand: debug_hand,
            debug_table: debug_table,
        };
        AfterMount::new(Model::Init(initst))
    }
}

#[wasm_bindgen(start)]
pub fn render() {
    App::builder(update, view)
        .after_mount(after_mount)
        .build_and_start();
}
