// XXX: until code stabilizes...
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate rand;
extern crate rand_pcg;
extern crate web_sys;
extern crate wasm_bindgen;
extern crate serde_json;
extern crate url;

use std::convert::{From, TryFrom};
use seed::{*, prelude::*};
use wasm_bindgen::JsCast;

use core;
use core::srvcli;
use core::srvcli::{CreateReq, CreateReqDebug, CreateRep, LobbyInfo, ClientMsg, ServerMsg, PlayerId};

type XRng = rand_pcg::Pcg64;

const DEFAULT_NR_PLAYERS: u8 = 1;

pub fn hand_from_string(s: &str) -> Option<core::Deck> {
    let mut vec : Vec<core::Card> = vec![];
    for cs in s.split_whitespace() {
        let card = core::Card::try_from(cs).ok()?;
        vec.push(card)
    }

    Some(core::Deck { cards: vec })
}

/// Initial state

#[derive(Clone,Debug)]
enum InitMsg {
    StartGame,
    StartGameReply(ResponseDataResult<CreateRep>),
    SetPlayerCount(String),
    SetPlayerName(String),
    DebugHandCards(String),
    DebugTableCards(String),
}

trait ToElem {
    fn to_elem(&self) -> Node<Msg>;
}

impl ToElem for core::Card {
    fn to_elem(&self) -> Node<Msg> {
        let span = if self.suit.is_red() {
            span![ style!{"color" => "red"}, format!("{}", self) ]
        } else {
            span![ style!{"color" => "black"}, format!("{}", self) ]
        };

        span
    }
}


fn iter_to_elem<T: ToElem, I: Iterator<Item=T>>(start: &str, mut iter: I, end: &str) -> Node<Msg> {
    let mut span = span![span![start]];
        let i0 = iter.next();
        if let Some(x0) = i0 {
            span.add_child(x0.to_elem());
            for x in iter {
                span.add_child(span![" "]);
                span.add_child(x.to_elem());
            }
        }
        span.add_child(span![end]);
        span
}

impl ToElem for core::Deck {
    fn to_elem(&self) -> Node<Msg> {
        let mut span = span![span!["["]];
        for (i,card) in self.cards.iter().enumerate() {
            span.add_child(card.to_elem());
            if i + 1 < self.cards.len() {
                span.add_child(span![" "]);
            }
        }
        span.add_child(span!["]"]);

        span
    }
}

impl ToElem for core::TableEntry {
    fn to_elem(&self) -> Node<Msg> {
        match self {
            core::TableEntry::Card(c) => c.to_elem(),
            core::TableEntry::Decl(d) => {
                span![ style!{"color" => "blue"}, format!("\u{2605}{}", d.value()) ]
            },
        }
    }
}

impl ToElem for core::DeclActionBuilder {
    fn to_elem(&self) -> Node<Msg> {
        let mut span = span![];

        if self.current.len() > 0 {
            let curr = span![
                // NB: Can we avoid the cloned here?
                iter_to_elem("→", self.current.iter().cloned(), ""),
                " "
            ];
            span.add_child(curr);
        }

        if self.action.tentries.len() > 0 {
            for tev in self.action.tentries.iter() {
                // NB: Can we avoid the cloned here?
                let e = iter_to_elem(" (", tev.iter().cloned(), ") ");
                span.add_child(e);
            }
        }

        span
    }
}

impl ToElem for core::CaptureActionBuilder {
    fn to_elem(&self) -> Node<Msg> {
        let mut span = span![];

        if self.current.len() > 0 {
            let curr = span![
                // NB: Can we avoid the cloned here?
                iter_to_elem("→", self.current.iter().cloned(), ""),
                " "
            ];
            span.add_child(curr);
        }

        if self.action.tentries.len() > 0 {
            for tev in self.action.tentries.iter() {
                // NB: Can we avoid the cloned here?
                let e = iter_to_elem(" (", tev.iter().cloned(), ") ");
                span.add_child(e);
            }
        }

        span
    }
}

impl ToElem for core::ScoreSheet {
    fn to_elem(&self) -> Node<Msg> {
        let details = if self.score == 0 { span![":-("] } else {
            let mut nodes: Vec<Vec<Node<Msg>>> = vec![];
            if self.has_the_cards() {
                nodes.push(
                    vec![
                        span![format!("{} (cards: {})", core::scoring::NCARDS_SCORE, self.nr_cards)]
                    ]
                )
            }

            if self.nr_xeres > 0 {
                nodes.push(
                    vec![
                        span![format!("{} ({} {})",
                            self.nr_xeres*core::scoring::XERI_SCORE,
                            self.nr_xeres,
                            if self.nr_xeres == 1 {"ξερή"} else {"ξερές"},

                        )]
                    ]
                )
            }

            for c in self.score_cards.iter() {
                nodes.push(
                    vec![
                        span![format!("{} (", core::scoring::card_value(c))],
                        c.to_elem(),
                        span![")"],
                    ]
                )
            }

            let mut span = span![" ="];
            let sep = vec![span![" + "]];
            let mut joined = (&nodes[..]).join(&sep[..]);
            for n in joined.drain(..) {
                span.add_child(n);
            }
            span
        };

        span![
            b![format!("{}", self.score)],
            details,
        ]
    }
}

struct InitSt {
    /// Number of players
    nplayers: u8,
    /// Error when trying to start a game
    start_game_err: Option<String>,
    player_name: String,

    debug_hand: String,
    debug_table: String,
}

fn get_create_game_req_url() -> impl Into<std::borrow::Cow<'static, str>> {
    "/creategame"
}

impl InitSt {

    fn mk_create_req(&self) -> CreateReq {

        let debug = if self.nplayers == 1 {
            Some(CreateReqDebug {
                hand_s: self.debug_hand.clone(),
                table_s: self.debug_table.clone(),
            })
        } else { None };

        let mut ret = CreateReq {
            nplayers: self.nplayers,
            debug: debug,
        };

        // verify that debug strings are correct
        ret.verify_debug();

        if let (Some(dbg), Some(storage)) = (&ret.debug, &seed::storage::get_storage()) {
            seed::storage::store_data(&storage, "debug_hand_cards", &dbg.hand_s);
            seed::storage::store_data(&storage, "debug_table_cards", &dbg.table_s);
        }

        ret
    }

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
                        self.start_game_err = Some("Could not create new game".to_string());
                        log!(format!("Error creating game: {:?}", x));
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
                let req_body = self.mk_create_req();
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

            InitMsg::DebugHandCards(x) => {
                self.debug_hand = x.clone();
            },

            InitMsg::DebugTableCards(x) => {
                self.debug_table = x.clone();
            },
        };

        None
    }

    fn select_nplayers(&self) -> Node<Msg> {
        let get_option = |txt: &str, val: &str, selected| {
            if selected {
                option![txt, attrs!{At::Value => val, At::Selected => "true"} ]
            } else {
                option![txt, attrs!{At::Value => val,} ]
            }
        };

        let mut attrs = match self.nplayers {
            1 => attrs!{At::Value => "1"},
            2 => attrs!{At::Value => "2"},
            4 => attrs!{At::Value => "4"},
            x => panic!("Invalid player count: {}", x),
        };
        attrs.add(At::Id, "sel-nplayers");
        div![
            label!["Number of players: ", attrs! {At::For => "sel-nplayers" }],
            select![
                get_option("1 (debug)", "1", DEFAULT_NR_PLAYERS == 1),
                get_option("2", "2", DEFAULT_NR_PLAYERS == 2),
                get_option("4", "4", DEFAULT_NR_PLAYERS == 4),
                input_ev(Ev::Input, |x| Msg::Init(InitMsg::SetPlayerCount(x))),
                attrs,
            ],
        ]
    }

    fn debug_options(&self) -> Node<Msg> {

        let no_debug = self.debug_hand.len() == 0 && self.debug_table.len() == 0;
        let debug_hand = core::Deck::try_from(self.debug_hand.as_str()).map_or(span![], |x| x.to_elem());
        let debug_table = core::Deck::try_from(self.debug_table.as_str()).map_or(span![], |x| x.to_elem());

        let mut div = div![
            p!["Debug (use S,C,H,D for suit and 1-9,T,J,Q,K for rank; e.g.: HK C5)"],
            p![
                label!["Table cards: ", attrs!{At::For => "set-dbg-table" }],
                input![
                    input_ev(Ev::Input, |x| Msg::Init(InitMsg::DebugTableCards(x))),
                    attrs!{
                        At::Id => "set-dbg-table"
                        At::Value => self.debug_table,
                    }
                ]
            ],
            p![
                label!["Hand cards: ", attrs!{At::For => "set-dbg-hand" }],
                input![
                    input_ev(Ev::Input, |x| Msg::Init(InitMsg::DebugHandCards(x))),
                    attrs!{
                        At::Id => "set-dbg-hand"
                        At::Value => self.debug_hand,
                    },
                ],
            ],
        ];

        if !no_debug {
            div.add_child(p![span!["Table: "], debug_table, span![", Hand: "], debug_hand]);
        }

        div
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
        ];

        if self.nplayers == 1 {
            ret.add_child(self.debug_options());
        }

        ret.add_child(
            button![
                simple_ev(Ev::Click, Msg::Init(InitMsg::StartGame)),
                "Start!",
                style![St::MarginRight => px(10)],
            ],
        );

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

enum TurnProgress {
    Nothing(Node<Msg>),
    CardSelected(usize),
    DeclaringWith(usize, Option<core::DeclActionBuilder>),
    CapturingWith(usize, core::CaptureActionBuilder),
    ActionIssued(core::PlayerAction),
}

enum GamePhase {
    MyTurn(TurnProgress),
    OthersTurn(PlayerId),
    RoundDone,
    GameDone(Vec<core::ScoreSheet>),
}

impl GamePhase {
    fn get_hand_selected_card(&self) -> Option<usize> {
        use GamePhase::*;
        use TurnProgress::*;

        match self {
            OthersTurn(_) => None,
            MyTurn(Nothing(_)) => None,
            MyTurn(CardSelected(x)) => Some(*x),
            MyTurn(DeclaringWith(x,_)) => Some(*x),
            MyTurn(CapturingWith(x,_)) => Some(*x),
            MyTurn(ActionIssued(_)) => None,
            RoundDone | GameDone(_) => None,
        }
    }

    fn is_tentry_selected(&self, tentry: &core::TableEntry) -> bool {
        use GamePhase::*;
        use TurnProgress::*;

        match self {
            OthersTurn(_) => false,
            MyTurn(ActionIssued(_)) => false,
            MyTurn(Nothing(_)) => false,
            MyTurn(CardSelected(x)) => false,
            MyTurn(DeclaringWith(x, None)) => false,
            MyTurn(DeclaringWith(x, Some(db))) => db.has_tentry(tentry),
            MyTurn(CapturingWith(x, cb)) => cb.has_tentry(tentry),
            RoundDone | GameDone(_) => false,
        }
    }
}

impl From<(&LobbyInfo, &core::PlayerGameView)> for GamePhase {
    fn from(pieces: (&LobbyInfo, &core::PlayerGameView)) -> GamePhase {
        let (lobby_info, pview) = pieces;
        match pview.state {

            core::GameState::NextTurn(tpos) if tpos == lobby_info.my_tpos() => {
                let msg = p!["Your turn to play (select a card from your hand)"];
                GamePhase::MyTurn(TurnProgress::Nothing(msg))
            },

            core::GameState::NextTurn(tpos) => {
                let pid = lobby_info.player_id_from_tpos(tpos).unwrap();
                GamePhase::OthersTurn(pid)
            },

            core::GameState::RoundDone => {
                GamePhase::RoundDone
            },

            core::GameState::GameDone(ref sheets)  => {
                GamePhase::GameDone(sheets.clone())
            },
        }
    }
}

struct GameSt {
    view: core::PlayerGameView,
    phase: GamePhase,
    lobby_info: LobbyInfo,

    wsocket: std::rc::Rc<Wsocket>,
    tmp_error_msg: String,
}

pub fn get_string_from_storage(key: &str) -> String {
    if let Some(storage) = seed::storage::get_storage() {
        seed::storage::load_data(&storage, key)
    } else {
        None
    }.unwrap_or("".to_string())
}

impl Default for Model {
    fn default() -> Self {
        let player_name = get_string_from_storage("player_name");
        let debug_hand = get_string_from_storage("debug_hand_cards");
        let debug_table = get_string_from_storage("debug_table_cards");

        Self::Init(InitSt {
            nplayers: DEFAULT_NR_PLAYERS,
            start_game_err: None,
            player_name: player_name,
            debug_hand: debug_hand,
            debug_table: debug_table,
        })
    }
}

#[derive(Clone,Debug)]
enum InGameMsg {
    ClickHandCard(usize),
    ClickTableEntry(usize),
    LayDown(usize),
    DeclareWith(usize), // card index
    DeclareSetSum(u8),  // sum
    CaptureWith(usize),
    FinalizePhase,
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
            tmp_error_msg: "".into(),
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
        let user_msg = div![
            p![format!("Invalid play: {}", err)],
            p!["Still your turn! Play a card from your hand."],
        ];
        self.phase = GamePhase::MyTurn(TurnProgress::Nothing(user_msg));
    }

    fn issue_action(&mut self, act: core::PlayerAction) {
        let climsg = ClientMsg::PlayerAction(act.clone());
        let req = serde_json::to_string(&climsg).unwrap();
        if let Err(x) = self.wsocket.ws.send_with_str(&req) {
            error!("Failed to send data to server");
            unimplemented!();
        }

        self.phase = GamePhase::MyTurn(TurnProgress::ActionIssued(act));
    }

    fn issue_action_validate(&mut self, act: core::PlayerAction) {
        log(format!("Issuing action: {:?}", act));
        let valid_check = act.validate(&self.view);
        let invalid_msg = match valid_check {
            Ok(()) => {
                self.issue_action(act);
                return
            }
            Err(x) => x,
        };
        self.invalid_action(invalid_msg);
    }

    fn update_state(&mut self, msg: &InGameMsg) -> Option<Model> {
        use GamePhase::*;
        use TurnProgress::*;

        self.tmp_error_msg = "".into();
        match msg {
            InGameMsg::ClickHandCard(x) => {
                let new_phase = match self.phase {
                    OthersTurn(_) => None,
                    MyTurn(Nothing(_)) => Some(MyTurn(CardSelected(*x))),
                    MyTurn(ActionIssued(_)) => None,

                    // If a hand card is already selected, clicking it again, will reset phase
                    // progress
                    MyTurn(CardSelected(prev_x))    |
                    MyTurn(DeclaringWith(prev_x,_)) |
                    MyTurn(CapturingWith(prev_x,_)) => {
                        Some(MyTurn(CardSelected(*x)))
                    },

                    RoundDone | GameDone(_) => None,
                };

                if let Some(x) = new_phase {
                    self.phase = x;
                }
                return None;
            }

            InGameMsg::ClickTableEntry(eidx) => {
                let te = self.view.get_table_entry(*eidx);
                let is_selected = self.phase.is_tentry_selected(te);
                let new_phase = match &mut self.phase {
                    OthersTurn(_) => None,
                    MyTurn(Nothing(_)) => None,
                    MyTurn(ActionIssued(_)) => None,
                    MyTurn(CardSelected(_)) => None,
                    MyTurn(DeclaringWith(cidx, None)) => None,

                    MyTurn(DeclaringWith(cidx, Some(db))) => {
                        if is_selected {
                            // NB: we could do something smarter here
                            db.reset()
                        } else {
                            let res = db.add_table_entry(te);
                            if let Err(errstr) = res {
                                self.tmp_error_msg = errstr;
                            }
                        }
                        None
                    },

                    MyTurn(CapturingWith(prev_x,cb)) => {
                        if is_selected {
                            // NB: we could do something smarter here
                            cb.reset()
                        } else {
                            let res = cb.add_table_entry(te);
                            if let Err(errstr) = res {
                                self.tmp_error_msg = errstr;
                            }
                        }
                        None
                    },

                    RoundDone | GameDone(_) => None,
                };

                if let Some(x) = new_phase {
                    self.phase = x;
                }
                return None;
            },

            // User selected a card to lay down
            InGameMsg::LayDown(cidx) => {
                let cc = self.view.own_hand.cards[*cidx].clone();
                let action = core::PlayerAction::LayDown(cc);
                self.issue_action_validate(action);
                return None;
            },

            InGameMsg::CaptureWith(cidx) => {
                let card = self.view.own_hand.cards[*cidx].clone();
                let bld = core::CaptureActionBuilder::new(&card);
                self.phase = GamePhase::MyTurn(TurnProgress::CapturingWith(*cidx, bld));
                return None;
            },

            InGameMsg::DeclareWith(cidx) => {
                self.phase = GamePhase::MyTurn(TurnProgress::DeclaringWith(*cidx, None));
                return None;
            },

            InGameMsg::DeclareSetSum(sum) => {
                let cidx = match self.phase {
                    GamePhase::MyTurn(TurnProgress::DeclaringWith(x, None)) => x,
                    _ => unimplemented!(),
                };

                let card = &self.view.own_hand.cards[cidx];
                let valid_sum = self.view.iter_hand_cards()
                    .find(|c| *c != card && c.rank.0 == *sum)
                    .is_some();

                if !valid_sum {
                    self.tmp_error_msg = format!("Declaration sum {} is invalid because you do not hold a matching card", sum);
                    return None
                }

                match core::DeclActionBuilder::new(card, *sum) {
                    Err(x) => self.tmp_error_msg = x,
                    Ok(db) => self.phase = GamePhase::MyTurn(TurnProgress::DeclaringWith(cidx, Some(db))),
                }

                return None;
            },

            InGameMsg::FinalizePhase => {
                match &self.phase {
                    MyTurn(DeclaringWith(cidx, Some(ds))) => {
                        let action = ds.make_action();
                        self.issue_action(action);
                        return None;
                    },

                    MyTurn(CapturingWith(cidx, cb)) => {
                        let action = cb.make_action();
                        self.issue_action(action);
                        return None;
                    }

                    _ => unimplemented!(),
                };
            },
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

            _ => unimplemented!(),

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

    fn mk_table_card_div(&self, card: &core::Card, selected: bool) -> Node<Msg> {
        let mut vec = vec!["card"];
        if selected {
            vec.push("selected");
        }
        if card.suit.is_red() {
            vec.push("red")
        } else {
            vec.push("black")
        };

        let mut attr = Attrs::empty();
        attr.add_multiple(At::Class, &vec);
        div![ attr, p![format!("{}", card)]]
    }

    fn mk_table_decl_div(&self, decl: &core::Declaration, selected: bool) -> Node<Msg> {
        let mut vec = vec!["card", "blue"];
        if selected {
            vec.push("selected");
        }
        let mut attr = Attrs::empty();
        attr.add_multiple(At::Class, &vec);

        let mut div = div![
            attr,
            p![format!("\u{2605}{}", decl.value())],
        ];

        for cardv in decl.cards.iter() {
            // NB: Can we avoid the cloned here?
            div.add_child(p![
                attrs!{At::Class => "cards"},
                iter_to_elem("", cardv.iter().cloned(), ""),
            ]);
        }
        div.add_child(p![""]);
        let pinfo = self.lobby_info.player_from_tpos(decl.player).unwrap();
        div.add_child(p![
           attrs!{At::Class => "player"},
           &pinfo.name
        ]);

        div
    }

    fn mk_table_entry_div(&self, te: &core::TableEntry, selected: bool) -> Node<Msg> {
        match te {
            core::TableEntry::Decl(x) => self.mk_table_decl_div(x, selected),
            core::TableEntry::Card(x) => self.mk_table_card_div(x, selected),
        }
    }

    fn mk_hand_card_div(&self, card: &core::Card, selected: bool) -> Node<Msg> {
        let mut vec = vec!["card"];
        if selected {
            vec.push("selected");
        }

        if card.suit.is_red() {
            vec.push("red");
        } else {
            vec.push("black");
        }

        let mut attr = Attrs::empty();
        attr.add_multiple(At::Class, &vec);
        div![ attr, format!("{}", card)]
    }

    fn view(&self) -> Node<Msg> {

        // let p_id = self.game.turn.0;

        let table = {
            let mut entries: Vec<Node<Msg>> = vec![];
            for (eidx, entry) in self.view.enum_table_entries() {
                let selected: bool = self.phase.is_tentry_selected(entry);
                let mut e_div = self.mk_table_entry_div(entry, selected);
                e_div.add_listener(
                    simple_ev(Ev::Click, Msg::InGame(InGameMsg::ClickTableEntry(eidx)))
                );
                entries.push(e_div)
            }

            div![
                attrs!{At::Class => "container"},
                p!["Table"],
                div![ attrs!{At::Class => "table"},  entries ],
            ]
        };

        let hand = {
            let mut cards: Vec<Node<Msg>> = vec![];
            let selected_card_idx = self.phase.get_hand_selected_card();
            for (cidx, card) in self.view.enum_hand_cards() {
                let selected: bool = selected_card_idx.map_or(false, |sidx| sidx == cidx);
                let mut c_div = self.mk_hand_card_div(card, selected);
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

        // TODO: if a user has a declaration on the table, be helpful about their possible actions
        // :)
        let phase_elem = match &self.phase {
            GamePhase::OthersTurn(_) => p!["Waiting for other player's turn"],
            GamePhase::MyTurn(TurnProgress::Nothing(msg)) => msg.clone(),
            GamePhase::MyTurn(TurnProgress::CardSelected(cidx)) => self.view_selected_card(*cidx),
            GamePhase::MyTurn(TurnProgress::DeclaringWith(cidx, ts)) => self.view_declaration(*cidx, ts),
            GamePhase::MyTurn(TurnProgress::CapturingWith(cidx, cb)) => self.view_capture(*cidx, cb),
            GamePhase::MyTurn(TurnProgress::ActionIssued(a)) => p!["Issued action. Waiting for server."],
            GamePhase::GameDone(sheets) => self.view_score(sheets),
            GamePhase::RoundDone => p!["Round done! Wait for new cards."],
        };
        let mut phase = div![
            attrs!{At::Class => "container"},
            phase_elem,
        ];

        if self.tmp_error_msg.len() > 0 {
            phase.add_child(p![ class!["error-msg"], self.tmp_error_msg ]);
        }

        div![table, hand, phase]
    }

    fn view_score(&self, sheets: &Vec<core::ScoreSheet>) -> Node<Msg> {
        let mut div = div![ p![format!("Game done!")] ];
        assert!(sheets.len() == self.lobby_info.nteams());
        for (i,ss) in sheets.iter().enumerate() {
            let p = p![
                format!("Team T{} ({}) score: ", i, self.lobby_info.team_players(i).join(", ")),
                ss.to_elem()
            ];
            div.add_child(p);
        }

        div
    }

    fn view_selected_card(&self, cidx: usize) -> Node<Msg> {
        let card = &self.view.own_hand.cards[cidx];
        let span = card.to_elem();
        let mut div = div![];
        {
            let msg = InGameMsg::LayDown(cidx);
            div.add_child(
                button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Lay down ", span.clone()]
            );
        }

        if !card.rank.is_figure() {
            let msg = InGameMsg::DeclareWith(cidx);
            div.add_child(
                button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Declare with ", span.clone()]
            );
        };

        {
            let msg = InGameMsg::CaptureWith(cidx);
            div.add_child(
                button![ simple_ev(Ev::Click, Msg::InGame(msg)), "Capture with ", span.clone()]
            );
        }

        div
    }

    fn view_declaration(&self, cidx: usize, ts: &Option<core::DeclActionBuilder>) -> Node<Msg> {
        let card = &self.view.own_hand.cards[cidx];

        // NB: we can use that to filter user choices.
        let sum_set = self.view.iter_hand_cards()
            .filter(|c| *c != card && !c.rank.is_figure() && c.rank.0 > card.rank.0)
            .map(|c| c.rank.0)
            .collect::<std::collections::BTreeSet<u8>>();

        match ts {
            None => {
                let mut div = div![
                    p!["Declaring with: ", card.to_elem()],
                    span![" declaration value: "]
                ];
                let msg_fn = |x: String| Msg::InGame(InGameMsg::DeclareSetSum(x.parse::<u8>().unwrap()));
                let mut opts = vec![];
                for i in 1..11 {
                    opts.push(option![i.to_string(), attrs!{At::Value => i.to_string()}])
                }
                div.add_child(label!["Declaration value:", attrs!{ At::For => "sel-declval" }]);
                div.add_child(
                    select![
                        opts,
                        input_ev(Ev::Input, msg_fn),
                        attrs!{ At::Id => "sel-declval" },
                    ],
                );

                /*
                for i in 2..11 {
                //for i in sum_set.iter().clone() {
                    div.add_child(
                        button![ simple_ev(Ev::Click, msg_fn(i)), i.to_string() ]
                    );
                }
                */

                div
            },

            Some(db) => {
                let mut div = div![
                    p!["Declaring with ",
                       card.to_elem(),
                       format!(" for a total of "),
                       span![ style!{"color" => "blue"}, format!("{}", db.value) ],
                       format!(" (select cards from the table)"),
                    ],
                    p!["Selection: ", db.to_elem()],
                    //p![format!("Selection: ", ts.to_elem()),
                ];

                if db.is_ready() {
                    let msg = Msg::InGame(InGameMsg::FinalizePhase);
                    let done = button![simple_ev(Ev::Click, msg), "Play"];
                    div.add_child(done);
                }

                div
            },
        }
    }

    fn view_capture(&self, cidx: usize, cb: &core::CaptureActionBuilder) -> Node<Msg> {
        let card = &self.view.own_hand.cards[cidx];
        let mut div = div![
            p!["Capturing with ",
               card.to_elem(),
               format!(" (select cards from the table)"),
            ],
            p!["Selection: ", cb.to_elem()],
        ];

        if cb.is_ready() {
            let msg = Msg::InGame(InGameMsg::FinalizePhase);
            let done = button![simple_ev(Ev::Click, msg), "Play"];
            div.add_child(done);
        }

        div
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

    let player_name = get_string_from_storage("player_name");
    let debug_hand = get_string_from_storage("debug_hand_cards");
    let debug_table = get_string_from_storage("debug_table_cards");

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
