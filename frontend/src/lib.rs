// XXX: until code stabilizes...
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate rand;
extern crate rand_pcg;
extern crate web_sys;
extern crate wasm_bindgen;
extern crate serde_json;
extern crate url;

use std::convert::From;
use seed::{*, prelude::*};
use wasm_bindgen::JsCast;

use core;

use core::srvcli;
use core::srvcli::{CreateReq, CreateRep, LobbyInfo, ClientMsg, ServerMsg, PlayerId};

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
    IssueStart,
}

#[derive(Debug)]
struct LobbySt {
    /// server info
    lobby_info: Option<LobbyInfo>,
    /// game identifier
    game_id: String,
    player_name: String,
    /// websocket to server
    wsocket: std::rc::Rc<Wsocket>,
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

    fn view_players(&self) -> Node<Msg> {
        if self.lobby_info.is_none() {
            return p![""];
        }

        let lobby_info: &LobbyInfo = self.lobby_info.as_ref().unwrap();

        let mut player_rows : Vec<Node<Msg>> = vec![];
        for i in 0..lobby_info.nplayers {
            let tpos_i = srvcli::PlayerTpos(i);
            let td_p = td![format!("P{}: ", i+1)];
            let (td_name, td_status, td_other) : (Node<Msg>, Node<Msg>, Node<Msg>) = {
                let pinfo = lobby_info.players.iter().enumerate().find( |(_, pi)| pi.tpos == tpos_i);
                match pinfo {
                    None => (td![""], td!["empty"], td![""]),
                    Some((xid, info)) => {
                        let td_other = {
                            let mut other = vec![];
                            if srvcli::PlayerId(xid) == lobby_info.self_id {
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
                        (td![info.name], td!["is ready"], td_other)
                    }
                }
            };
            player_rows.push(tr![td_p, td_name, td_status, td_other])

        }




        let hname = web_sys::window().unwrap().location().host().unwrap();
        let join_href = format!("{}/?join={}", hname, self.game_id);
        let mut div = div![
            p!["join link: ", a![attrs! {At::Href => join_href}, join_href]],
            p![""],
            h3!["Players"],
            table![player_rows],
        ];

        if lobby_info.players[lobby_info.self_id.0].admin {
            let attrs = if lobby_info.nplayers as usize != lobby_info.players.len() {
                attrs!{At::Disabled => "true"}
            } else {
                attrs!{}
            };
            let start_button = button![
                simple_ev(Ev::Click, Msg::Lobby(LobbyMsg::IssueStart)),
                "Start!",
                attrs,
            ];
            div.add_child(start_button);
        }

        div

    }

    fn view(&self) -> Node<Msg> {
        div![
            h2!["Lobby"],
            self.view_players(),
        ]
    }

    fn update_state(&mut self, msg: &LobbyMsg, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        match msg {
            LobbyMsg::IssueStart => {
                let req = serde_json::to_string(&ClientMsg::StartGame).unwrap();
                if let Err(x) = self.wsocket.ws.send_with_str(&req) {
                    error!("Failed to send data to server");
                    unimplemented!();
                }
                None
            },
        }
    }

    fn handle_ws_event(&mut self, ev: &WsEvent, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        match (ev, self.wsocket.ws_state) {
            (WsEvent::WsConnected(jv), WsState::Init) => {
                // change ws state to ready
                log(format!("Connected: {}", jv.as_string().unwrap_or("<None>".to_string())));
                {
                    let ws = std::rc::Rc::get_mut(&mut self.wsocket).unwrap();
                    ws.ws_state = WsState::Ready;
                }
            },
            (WsEvent::WsMessage(msg), WsState::Ready) => {
                let txt = msg.data().as_string().expect("No data in server message");
                log(format!("Received message {:?}", txt));
                let srv_msg: ServerMsg = serde_json::from_str(&txt).unwrap();
                self.lobby_info = Some(match srv_msg {
                    ServerMsg::InLobby(x) => x,
                    ServerMsg::GameUpdate(pview) => {
                        let lobby_info : &LobbyInfo = self.lobby_info
                            .as_ref()
                            .expect("At this point, we should have received lobby info from the server");
                        return Some(Model::InGame(GameSt::new(self, pview)));
                    },
                    _ => panic!("Unexpected message: {:?}", srv_msg),
                });
            },
            // TODO: have some kind of error model... (or reconnect?)
            // (WsEvent::WsClose(_), _) => _,
            _ => panic!("Invalid websocket state/message ({:?}/{:?})", ev, self.wsocket.ws_state)
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

        let wsocket = std::rc::Rc::<Wsocket>::new(Wsocket {
            ws: web_sys::WebSocket::new(&ws_url).unwrap(),
            ws_state: WsState::Init,
        });

        let ws = &wsocket.ws;
        register_ws_handler(
            web_sys::WebSocket::set_onopen,
            |jv| Msg::Ws(WsEvent::WsConnected(jv)),
            ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onclose,
            |jv| Msg::Ws(WsEvent::WsClose(jv)),
            ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onerror,
            |jv| Msg::Ws(WsEvent::WsError(jv)),
            ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onmessage,
            |me| Msg::Ws(WsEvent::WsMessage(me)),
            ws, orders);

        let ret = LobbySt {
            lobby_info: None,
            game_id: game_id,
            player_name: player_name,
            wsocket: wsocket,
        };

        Ok(ret)
    }
}

/// Game state

struct TableSelection {
    curent: Vec<usize>,
    existing: Vec<Vec<usize>>,
}

enum TurnProgress {
    Nothing(String),
    CardSelected(usize),
    DeclaringWith(usize, TableSelection),
    GatheringWith(usize, TableSelection),
    ActionIssued(core::PlayerAction),
}

enum GamePhase {
    MyTurn(TurnProgress),
    OthersTurn(PlayerId),
}

impl From<(&LobbyInfo, &core::PlayerGameView)> for GamePhase {
    fn from(pieces: (&LobbyInfo, &core::PlayerGameView)) -> GamePhase {
        let (lobby_info, pview) = pieces;
        let turn_tpos = pview.turn;
        let self_tpos = lobby_info.players[lobby_info.self_id.0].tpos;
        if self_tpos == turn_tpos {
            GamePhase::MyTurn(TurnProgress::Nothing("Your turn to play".into()))
        } else {
            let turn_pid = lobby_info.players
                .iter()
                .enumerate()
                .find( |(_, pi)| pi.tpos == turn_tpos)
                .map( |(i,_)| i)
                .unwrap();
            GamePhase::OthersTurn(srvcli::PlayerId(turn_pid))
        }
    }
}

struct GameSt {
    view: core::PlayerGameView,
    phase: GamePhase,
    lobby_info: LobbyInfo,

    wsocket: std::rc::Rc<Wsocket>,
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
    }
}

#[derive(Clone,Debug)]
enum InGameMsg {
    ClickHandCard(usize),
    LayDown(usize),
    TakeWith(usize),
    DeclareWith(usize),
}


impl GameSt {


    pub fn new(lobbyst: &LobbySt, pview: core::PlayerGameView) -> GameSt {
        let lobby_info = lobbyst.lobby_info.as_ref().unwrap();
        let phase: GamePhase = (lobby_info, &pview).into();
        GameSt {
            view: pview,
            phase: phase,
            lobby_info: lobby_info.clone(),
            wsocket: lobbyst.wsocket.clone(),
        }
    }

    pub fn new_view_from_server(&mut self, pview: core::PlayerGameView) {
        self.view = pview;
        self.phase = (&self.lobby_info, &self.view).into();
    }

    fn action_issued(&self) -> bool {
        match self.phase {
            GamePhase::MyTurn(TurnProgress::ActionIssued(_)) => true,
            _ => false,
        }
    }

    fn myturn(&self) -> bool {
        match self.phase {
            GamePhase::MyTurn(_) => true,
            _ => false,
        }
    }

    fn invalid_action(&mut self, err: String) {
        assert!(self.myturn());
        // reset phase
        let user_msg = format!("Invalid play: {}. Still your turn!", err);
        self.phase = GamePhase::MyTurn(TurnProgress::Nothing(user_msg));
    }

    fn try_issue_action(&mut self, act: core::PlayerAction) -> Result<(), ()> {
        log(format!("Issuing action: {:?}", act));
        let valid_check = self.view.validate_action(&act);
        let invalid_msg = match valid_check {
            Ok(()) => {
                let climsg = ClientMsg::PlayerAction(act.clone());
                let req = serde_json::to_string(&climsg).unwrap();
                if let Err(x) = self.wsocket.ws.send_with_str(&req) {
                    error!("Failed to send data to server");
                    unimplemented!();
                }
                self.phase = GamePhase::MyTurn(TurnProgress::ActionIssued(act));
                return Ok(())
            }
            Err(x) => x,
        };

        self.invalid_action(invalid_msg);
        Err(())
    }

    fn update_state(&mut self, msg: &InGameMsg) -> Option<Model> {
        match msg {
            InGameMsg::ClickHandCard(x) => {
                self.phase = GamePhase::MyTurn(TurnProgress::CardSelected(*x));
                return None;
            }

            // User selected a card to lay down
            InGameMsg::LayDown(cidx) => {
                let cc = self.view.own_hand.cards[*cidx].get_clone();
                let action = core::PlayerAction::LayDown(cc);
                match self.try_issue_action(action) {
                    Err(()) => (),
                    Ok(()) => (),
                };
                return None;
            }

            _ => unimplemented!(),
        }
    }

    fn handle_server_message(&mut self, msg: ServerMsg) -> Option<Model> {
        match msg {
            ServerMsg::InvalidAction(x) => {
                assert!(self.action_issued());
                self.invalid_action(x);
                return None
            },

            ServerMsg::GameUpdate(pview) => {
                self.new_view_from_server(pview);
                return None;
            },

            _ => unimplemented!()
        }
    }

    fn handle_ws_event(&mut self, ev: &WsEvent, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        match (ev, self.wsocket.ws_state) {
            (WsEvent::WsConnected(jv), _) => unimplemented!(),
            (WsEvent::WsMessage(msg), WsState::Ready) => {
                let txt = msg.data().as_string().expect("No data in server message");
                log(format!("Received message {:?}", txt));
                let srv_msg: ServerMsg = serde_json::from_str(&txt).unwrap();
                self.handle_server_message(srv_msg)
            },
            // TODO: have some kind of error model... (or reconnect?)
            // (WsEvent::WsClose(_), _) => _,
            _ => panic!("Invalid websocket state/message ({:?}/{:?})", ev, self.wsocket.ws_state)
        };

        None
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
            for entry in self.view.iter_table_entries() {
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
            for (cidx, card) in self.view.iter_hand_cards().enumerate() {
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

        let msg = match &self.phase {
            GamePhase::OthersTurn(_) => p!["Waiting for other player's turn"],
            GamePhase::MyTurn(TurnProgress::Nothing(msg)) => p![msg],
            GamePhase::MyTurn(TurnProgress::CardSelected(cidx)) => {
                    let card = &self.view.own_hand.cards[*cidx];

                    let span : Node<Msg> = if card.suit.is_red() {
                        span![ style!{"color" => "red"}, format!("{}", card) ]
                    } else {
                        span![ style!{"color" => "black"}, format!("{}", card) ]
                    };

                    let mut div = div![];
                    {
                        let msg = InGameMsg::LayDown(*cidx);
                        div.add_child(
                            button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Play ", span.clone()]
                        );
                    }

                    {
                        let msg = InGameMsg::TakeWith(*cidx);
                        div.add_child(
                            button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Take with ", span.clone()]
                        );
                    }

                    if !card.rank.is_figure() {
                        let msg = InGameMsg::DeclareWith(*cidx);
                        div.add_child(
                            button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Declare with ", span]
                        );
                    };

                    div
            }
            GamePhase::MyTurn(TurnProgress::DeclaringWith(cidx, tsel)) => unimplemented!(),
            GamePhase::MyTurn(TurnProgress::GatheringWith(cidx, tsel)) => unimplemented!(),
            GamePhase::MyTurn(TurnProgress::ActionIssued(a)) => p!["Issued action. Waiting for server."],
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

#[derive(Debug)]
struct Wsocket {
    ws: web_sys::WebSocket,
    ws_state: WsState,
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
        (&mut Model::InGame(st), Msg::Ws(ref msg))    => st.handle_ws_event(msg, orders),
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
