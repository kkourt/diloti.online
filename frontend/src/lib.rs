// XXX: until code stabilizes...
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate rand;
extern crate rand_pcg;
extern crate web_sys;
extern crate wasm_bindgen;
extern crate serde_json;
extern crate url;

use rand::SeedableRng;
use seed::{*, prelude::*};
use wasm_bindgen::JsCast;

use core;

use common::{CreateReq, CreateRep, /* PreReq */};

type XRng = rand_pcg::Pcg64;

const DEFAULT_NR_PLAYERS: u8 = 2;

/// Initial state

#[derive(Clone,Debug)]
enum InitMsg {
    StartGame,
    StartGameReply(ResponseDataResult<CreateRep>),
    SetPlayerCount(String),
    SetPlayerName(String),
}

struct InitSt {
    /// Number of players
    nplayers: u8,
    /// Error when trying to start a game
    start_game_err: Option<String>,
    player_name: String,
}

fn get_create_game_req_url() -> impl Into<std::borrow::Cow<'static, str>> {
    "/creategame"
}

impl InitSt {

    fn update_state(&mut self, msg: &InitMsg, orders: &mut impl Orders<Msg>) -> Option<Model> {
        // log!(format!("*************** {:?}", msg));

        match msg {
            InitMsg::StartGameReply(result) => {
                match result {
                    // change state to lobby
                    Ok(rep) => {
                        return Some(Model::InLobby(
                            // TODO: proper error checking
                            LobbySt::new(
                                rep.game_id.clone(),
                                self.player_name.clone(),
                                orders,
                            ).unwrap()
                        ));
                    }

                    Err(x) => {
                        self.start_game_err = Some(format!("Could not create new game: {:?}", x));
                    }
                }
            },

            InitMsg::StartGame => {
                if self.player_name.len() == 0 {
                    self.start_game_err = Some(format!("Please select a non-empty name"));
                    return None;
                }

                if !self.player_name.chars().all(char::is_alphanumeric) {
                    self.start_game_err = Some(format!("Please only use alphanumeric characters for the name"));
                    return None;
                }

                let url = get_create_game_req_url();
                let req_body = CreateReq { nplayers: self.nplayers };
                let req = Request::new(url.into())
                    .method(seed::browser::service::fetch::Method::Put)
                    .send_json(&req_body)
                    .fetch_json_data( |o| Msg::Init(InitMsg::StartGameReply(o)));
                orders.perform_cmd(req);
            },

            InitMsg::SetPlayerCount(x) => {
                if x == "1" {
                    self.nplayers = 1;
                } else if x == "2" {
                    self.nplayers = 2;
                } else if x == "4" {
                    self.nplayers = 4;
                } else {}
            },

            InitMsg::SetPlayerName(x) => {
                log!("SetPlayerName: {x}", x);
                self.player_name = x.to_string();
            },
        };

        None
    }

    fn select_nplayers(&self) -> Node<Msg> {
        let mut attrs = match self.nplayers {
            1 => attrs!{At::Value => "1"},
            2 => attrs!{At::Value => "2"},
            4 => attrs!{At::Value => "4"},
            x => panic!("Invalid player count: {}", x),
        };
        attrs.add(At::Id, "sel-nplayers");
        assert_eq!(DEFAULT_NR_PLAYERS, 2);
        div![
            label!["Number of players: ", attrs! {At::For => "sel-nplayers" }],
            select![
                attrs,
                option!["1 (debug)", attrs!{At::Value => "1"}],
                option!["2", attrs!{At::Value => "2", At::Selected => "selected"}],
                option!["4", attrs!{At::Value => "4"}],
                input_ev(Ev::Input, |x| Msg::Init(InitMsg::SetPlayerCount(x)))
            ],
        ]
    }

    fn set_name(&self) -> Node<Msg> {
        div![
            label!["Your name: ", attrs!{At::For => "set-name" }],
            input![
                input_ev(Ev::Input, |x| Msg::Init(InitMsg::SetPlayerName(x))),
                attrs! {
                    At::Id => "set-name",
                    At::Value => self.player_name,
                }
            ]
        ]
    }

    fn view(&self) -> Node<Msg> {
        let mut ret = div![
            h2!["Create new game"],
            self.set_name(),
            self.select_nplayers(),

            button![
                simple_ev(Ev::Click, Msg::Init(InitMsg::StartGame)),
                "Start!",
                style![St::MarginRight => px(10)],
            ],
        ];

        if let Some(x) = &self.start_game_err {
            ret.add_child(span!["Failed! :-("]);
            ret.add_child(p![format!("Error: {}", x)]);
        }

        ret
    }
}

/// Join state

#[derive(Debug, Clone)]
enum JoinMsg {
    JoinGame,
    SetPlayerName(String),
}

struct JoinSt {
    game_id: String,
    player_name: String,
    join_game_err: Option<String>,
}

impl JoinSt {
    fn update_state(&mut self, msg: &JoinMsg, orders: &mut impl Orders<Msg>) -> Option<Model> {
        match msg {
            JoinMsg::JoinGame => {
                if self.player_name.len() == 0 {
                    self.join_game_err = Some(format!("Please select a non-empty name"));
                    return None;
                }

                if !self.player_name.chars().all(char::is_alphanumeric) {
                    self.join_game_err = Some(format!("Please only use alphanumeric characters for the name"));
                    return None;
                }
                Some(Model::InLobby(LobbySt::new(
                    self.game_id.clone(),
                    self.player_name.clone(),
                    orders
                ).unwrap()))
            },
            JoinMsg::SetPlayerName(name) => {
                self.player_name = name.to_string();
                None
            },
        }
    }

    fn set_name(&self) -> Node<Msg> {
        div![
            label!["Your name: ", attrs!{At::For => "set-name" }],
            input![
                input_ev(Ev::Input, |x| Msg::Join(JoinMsg::SetPlayerName(x))),
                attrs! {
                    At::Id => "set-name",
                    At::Value => self.player_name,
                }
            ]
        ]
    }

    fn view(&self) -> Node<Msg> {
        let mut ret = div![
            h2!["Join game"],
            self.set_name(),
            button![
                simple_ev(Ev::Click, Msg::Join(JoinMsg::JoinGame)),
                "Join!",
                style![St::MarginRight => px(10)],
            ],
        ];

        if let Some(x) = &self.join_game_err {
            ret.add_child(span!["Failed! :-("]);
            ret.add_child(p![format!("Error: {}", x)]);
        }

        ret
    }
}

/// Loby state

#[derive(Debug,Clone)]
enum LobbyMsg {
}

#[derive(Debug)]
struct LobbySt {
    /// server info
    lobby_info: Option<common::LobbyInfo>,
    /// game identifier
    game_id: String,
    player_name: String,
    /// websocket to server
    ws: web_sys::WebSocket,
    ws_state: WsState,
}


// stolen from seed's examples
fn register_ws_handler<T, F>(
    ws_cb_setter: fn(&web_sys::WebSocket, Option<&js_sys::Function>),
    msg: F,
    ws: &web_sys::WebSocket,
    orders: &mut impl Orders<Msg>,
) where
    T: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: Fn(T) -> Msg + 'static,
{
    let (app, msg_mapper) = (orders.clone_app(), orders.msg_mapper());

    let closure = Closure::new(move |data| {
        app.update(msg_mapper(msg(data)));
    });

    ws_cb_setter(ws, Some(closure.as_ref().unchecked_ref()));
    closure.forget();
}


impl LobbySt {

    fn view_server_info(&self) -> Node<Msg> {
        if self.lobby_info.is_none() {
            return p![""];
        }

        let lobby_info: &common::LobbyInfo = self.lobby_info.as_ref().unwrap();

        let mut player_rows : Vec<Node<Msg>> = vec![];
        for i in 0..lobby_info.nplayers {
            // player id column
            let td_p = td![format!("P{}: ", i+1)];
            let (td_name, td_status, td_other) : (Node<Msg>, Node<Msg>, Node<Msg>) = {
                let pinfo = lobby_info.players.iter().enumerate().find( |(_, pi)| pi.tpos == i);
                match pinfo {
                    None => (td![""], td!["empty"], td![""]),
                    Some((xid, info)) => {
                        let td_other = {
                            let mut other = vec![];
                            if xid == lobby_info.self_id {
                                other.push("you!");
                            }
                            if info.admin {
                                other.push("game admin");
                            }

                            if other.len() == 0 {
                                td![""]
                            } else {
                                td![format!("({})", other.join(", "))]
                            }
                        };
                        (td![info.name], td!["ready"], td_other)
                    }
                }
            };
            player_rows.push(tr![td_p, td_name, td_status, td_other])
        }

        let hname = web_sys::window().unwrap().location().host().unwrap();
        let join_href = format!("{}/?join={}", hname, self.game_id);
        div![
            p!["join link: ", a![attrs! {At::Href => join_href}, join_href]],
            p![""],
            h3!["Players"],
            table![player_rows],
        ]
    }

    fn view(&self) -> Node<Msg> {
        let lobby_info = self.view_server_info();
        div![
            h2!["Lobby"],
            lobby_info,
        ]
    }

    fn update_state(&mut self, msg: &LobbyMsg, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        unimplemented!();
    }

    fn handle_ws_event(&mut self, ev: &WsEvent, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        match (ev, self.ws_state) {
            (WsEvent::WsConnected(jv), WsState::Init) => {
                // change ws state to ready
                log(format!("Connected: {}", jv.as_string().unwrap_or("<None>".to_string())));
                self.ws_state = WsState::Ready;
            },
            (WsEvent::WsMessage(msg), WsState::Ready) => {
                let txt = msg.data().as_string().expect("No data in server message");
                log(format!("Received message {:?}", txt));
                let srv_msg: common::ServerMsg = serde_json::from_str(&txt).unwrap();
                self.lobby_info = Some(match srv_msg {
                    common::ServerMsg::InLobby(x) => x,
                    _ => panic!("Unexpected message: {:?}", srv_msg),
                });
            },
            // TODO: have some kind of error model... (or reconnect?)
            // (WsEvent::WsClose(_), _) => _,
            _ => panic!("Invalid websocket state/message ({:?}/{:?})", ev, self.ws_state)
        };

        None
    }


    fn new(
        game_id: String,
        player_name: String,
        orders: &mut impl Orders<Msg>
    ) -> Result<LobbySt,String> {
        let hname = web_sys::window().unwrap().location().host().unwrap();
        let ws_url = format!("ws://{}/ws/{}/{}", hname, game_id, player_name);
        // log(format!("**************** ws_url={}", ws_url));
        if let Some(storage) = seed::storage::get_storage() {
            seed::storage::store_data(&storage, "player_name", &player_name);
        }

        let ws = web_sys::WebSocket::new(&ws_url).unwrap();

        register_ws_handler(
            web_sys::WebSocket::set_onopen,
            |jv| Msg::Ws(WsEvent::WsConnected(jv)),
            &ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onclose,
            |jv| Msg::Ws(WsEvent::WsClose(jv)),
            &ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onerror,
            |jv| Msg::Ws(WsEvent::WsError(jv)),
            &ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onmessage,
            |me| Msg::Ws(WsEvent::WsMessage(me)),
            &ws, orders);

        let ret = LobbySt {
            lobby_info: None,
            game_id: game_id,
            player_name: player_name,
            ws: ws,
            ws_state: WsState::Init,
        };

        Ok(ret)
    }
}

/// Game state

struct TableSelection {
    curent: Vec<usize>,
    existing: Vec<Vec<usize>>,
    ver: core::GameVer,
}

enum TurnProgress {
    Nothing,
    CardSelected(core::HandCardIdx),
    DeclaringWith(core::HandCardIdx, TableSelection),
    GatheringWith(core::HandCardIdx, TableSelection),
    ActionIssued(core::PlayerAction),
}

enum GamePhase {
    MyTurn(TurnProgress),
    OthersTurn,
}

struct GameSt {
    pub game: core::Game<XRng>,
    pub phase: GamePhase,
    pub view: core::PlayerGameView,
}


impl Default for GameSt {
    fn default() -> Self {
        let rng = XRng::from_rng(rand::rngs::OsRng).expect("unable to initalize RNG");
        let game = core::Game::new_2p(rng);
        let view = game.get_player_game_view();
        Self {
            game: game,
            view: view,
            phase: GamePhase::MyTurn(TurnProgress::Nothing),
        }
    }
}


impl Default for Model {
    fn default() -> Self {
        let player_name = if let Some(storage) = seed::storage::get_storage() {
            seed::storage::load_data(&storage, "player_name")
        } else {
            None
        }.unwrap_or("".to_string());

        Self::Init(InitSt {
            nplayers: DEFAULT_NR_PLAYERS,
            start_game_err: None,
            player_name: player_name,
        })
        // Self::InGame(GameSt::default())
    }
}

#[derive(Clone,Debug)]
enum InGameMsg {
    ClickHandCard(core::HandCardIdx),
    PutDown(core::HandCardIdx),
    TakeWith(core::HandCardIdx),
    DeclareWith(core::HandCardIdx),
}


impl GameSt {
    fn update_state(&mut self, msg: &InGameMsg) -> Option<Model> {
        match msg {
            InGameMsg::ClickHandCard(x) => {
                self.phase = GamePhase::MyTurn(TurnProgress::CardSelected(*x));
            }

            InGameMsg::PutDown(x) => {
                let action = core::PlayerAction::Play(*x);
                self.issue_action(&action);
                self.phase = GamePhase::MyTurn(TurnProgress::ActionIssued(action));
            }

            _ => unimplemented!(),
        }

        None
    }

    fn issue_action(&self, act: &core::PlayerAction) {
        // TODO
        log(format!("Issuing action: {:?}", act));
    }

    fn mk_card_div(&self, card: &core::Card) -> Node<Msg> {
        let attr = if card.suit.is_red() {
            let mut a = Attrs::empty();
            a.add_multiple(At::Class, &["card", "red"]);
            a
        } else {
            let mut a = Attrs::empty();
            a.add_multiple(At::Class, &["card", "black"]);
            a
        };

        div![ attr, format!("{}", card)]
    }

    fn mk_table_entry_div(&self, te: &core::TableEntry) -> Node<Msg> {
        match te {
            core::TableEntry::Decl(x) => unimplemented!(),
            core::TableEntry::Card(x) => self.mk_card_div(x),
        }
    }

    fn view(&self) -> Node<Msg> {

        // let p_id = self.game.turn.0;

        let table = {
            let mut entries: Vec<Node<Msg>> = vec![];
            for (_eidx, entry) in self.view.iter_table_entries() {
                let e = self.mk_table_entry_div(entry);
                entries.push(e)
            }
            div![
                attrs!{At::Class => "container"},
                p!["Table"],
                div![ attrs!{At::Class => "table"},  entries ]
            ]
        };

        let hand = {
            let mut cards: Vec<Node<Msg>> = vec![];
            for (cidx, card) in self.view.iter_hand_cards() {
                let mut c_div = self.mk_card_div(card);
                c_div.add_listener(
                    simple_ev(Ev::Click, Msg::InGame(InGameMsg::ClickHandCard(cidx)))
                );
                cards.push(c_div);
            }

            div![
                attrs!{At::Class => "container"},
                p!["Hand"],
                div![ attrs!{At::Class => "hand"}, cards],
            ]
        };

        let msg = match self.phase {
            GamePhase::OthersTurn => p!["Waiting for other player's turn"],
            GamePhase::MyTurn(TurnProgress::Nothing) => p!["Your turn! Select the card you want to play"],
            GamePhase::MyTurn(TurnProgress::CardSelected(c)) => {
                    let card = self.view.get_hand_card(&c);

                    let span : Node<Msg> = if card.suit.is_red() {
                        span![ style!{"color" => "red"}, format!("{}", card) ]
                    } else {
                        span![ style!{"color" => "black"}, format!("{}", card) ]
                    };

                    let mut div = div![];
                    {
                        let msg = InGameMsg::PutDown(c);
                        div.add_child(
                            button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Play ", span.clone()]
                        );
                    }

                    {
                        let msg = InGameMsg::TakeWith(c);
                        div.add_child(
                            button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Take with ", span.clone()]
                        );
                    }

                    if !card.rank.is_figure() {
                        let msg = InGameMsg::DeclareWith(c);
                        div.add_child(
                            button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Declare with ", span]
                        );
                    };

                    div
            }
            _ => panic!(""),
        };

        div![ table, hand, msg]
    }
}

/// Demultiplexers
// NB: there seem to be some facilities for better handling this demultiplexing:
// https://seed-rs.org/guide/complex-apps, but for now we just ad-hoc it.


#[derive(Debug,Clone)]
enum WsEvent {
    WsConnected(wasm_bindgen::JsValue),
    WsClose(wasm_bindgen::JsValue),
    WsError(wasm_bindgen::JsValue),
    WsMessage(web_sys::MessageEvent),
}

#[derive(Debug, Clone, Copy)]
enum WsState {
    Init,
    Ready,
    Closed,
    Error,
}

enum Model {
    Init(InitSt),
    Join(JoinSt),
    InLobby(LobbySt),
    InGame(GameSt),
}

#[derive(Clone,Debug)]
enum Msg {
    Init(InitMsg),
    Join(JoinMsg),
    InGame(InGameMsg),
    Lobby(LobbyMsg),
    Ws(WsEvent),
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    log(format!("update => {:?}", msg));
    let upd_ret = match (&mut model, msg) {
        (&mut Model::Init(st), Msg::Init(ref msg))     => st.update_state(msg, orders),
        (&mut Model::Join(st), Msg::Join(ref msg))     => st.update_state(msg, orders),
        (&mut Model::InLobby(st), Msg::Lobby(ref msg)) => st.update_state(msg, orders),
        (&mut Model::InLobby(st), Msg::Ws(ref msg))    => st.handle_ws_event(msg, orders),
        (&mut Model::InGame(st), Msg::InGame(ref msg)) => st.update_state(msg),
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

    let player_name = if let Some(storage) = seed::storage::get_storage() {
        seed::storage::load_data(&storage, "player_name")
    } else {
        None
    }.unwrap_or("".to_string());

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
