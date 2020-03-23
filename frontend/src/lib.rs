
extern crate rand;
extern crate rand_pcg;
extern crate web_sys;
extern crate wasm_bindgen;

use rand::SeedableRng;
use seed::{*, prelude::*};
use wasm_bindgen::JsCast;

use core::{deck::Deck,
           card::Card,
           game::{Game, PlayerGameView, TableEntry, GameVer, HandCardIdx, TableEntryIdx, PlayerAction}};
use common::CreateGameRep;

type XRng = rand_pcg::Pcg64;

/// Initial state

#[derive(Clone,Debug)]
enum InitMsg {
    StartGame,
    StartGameReply(ResponseDataResult<CreateGameRep>),
    SetPlayerCount(String),
}

struct InitSt {
    /// Number of players
    nplayers: u8,
    /// Error when trying to start a game
    start_game_err: Option<String>,
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
                            LobbySt::new(self.nplayers, rep.game_id.clone(), orders).unwrap()
                        ));
                    }

                    Err(x) => {
                        self.start_game_err = Some(format!("Could not create new game: {:?}", x));
                    }
                }
            },

            InitMsg::StartGame => {
                let url = get_create_game_req_url();
                let req = Request::new(url.into())
                    .method(seed::browser::service::fetch::Method::Put)
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
            }
        };

        None
    }

    fn view(&self) -> Node<Msg> {
        let msg = if self.nplayers != 1 {
            span![style!{"color" => "red"}, " (Sorry, just one player for now)"]
        } else {
           span![]
        };

        let mut ret = div![
            h2!["Welcome!"],

            span!["Number of players: "],
            select![
                attrs!{At::Value => "1"},
                option!["4", attrs!{At::Value => "4"}],
                option!["2", attrs!{At::Value => "2"}],
                option!["1 (debug)", attrs!{At::Value => "1", At::Selected => true.as_at_value()}],
                input_ev(Ev::Input, |x| Msg::Init(InitMsg::SetPlayerCount(x)))
            ],
            span![msg],

            p![""],
            button![
                simple_ev(Ev::Click, Msg::Init(InitMsg::StartGame)),
                "Start new game",
                style![St::MarginRight => px(10)],
            ],
        ];

        if let Some(x) = &self.start_game_err {
            ret.add_child(span!["Failed! :-("]);
            ret.add_child(p![format!("Error:{:?}", x)]);
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
    /// Number of players
    nplayers: u8,
    /// game identifier
    game_id: String,
    /// websocket to server
    ws: web_sys::WebSocket,
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
    fn view(&self) -> Node<Msg> {
        h2!["Welcome to the Lobby"]
    }

    fn update_state(&mut self, msg: &LobbyMsg, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        unimplemented!();
        None
    }

    fn new(nplayers: u8, game_id: String, orders: &mut impl Orders<Msg>) -> Result<LobbySt,String> {
        let hname = web_sys::window().unwrap().location().host().unwrap();
        let ws_url = format!("ws://{}/ws/{}", hname, game_id);
        log(format!("**************** ws_url={}", ws_url));
        let ws = web_sys::WebSocket::new(&ws_url).unwrap();

        let ret = LobbySt {
            nplayers: nplayers,
            game_id: game_id,
            ws: ws,
        };

        register_ws_handler(
            web_sys::WebSocket::set_onopen,
            |jv| Msg::Ws(WsEvent::WsConnected(jv)),
            &ret.ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onclose,
            |jv| Msg::Ws(WsEvent::WsClose(jv)),
            &ret.ws,
            orders);

        register_ws_handler(
            web_sys::WebSocket::set_onerror,
            |jv| Msg::Ws(WsEvent::WsError(jv)),
            &ret.ws, orders);

        register_ws_handler(
            web_sys::WebSocket::set_onmessage,
            |me| Msg::Ws(WsEvent::WsMessage(me)),
            &ret.ws, orders);

        Ok(ret)
    }
}

/// Game state

struct TableSelection {
    curent: Vec<usize>,
    existing: Vec<Vec<usize>>,
    ver: GameVer,
}

enum TurnProgress {
    Nothing,
    CardSelected(HandCardIdx),
    DeclaringWith(HandCardIdx, TableSelection),
    GatheringWith(HandCardIdx, TableSelection),
    ActionIssued(PlayerAction),
}

enum GamePhase {
    MyTurn(TurnProgress),
    OthersTurn,
}

struct GameSt {
    pub game: Game<XRng>,
    pub phase: GamePhase,
    pub view: PlayerGameView,
}


impl Default for GameSt {
    fn default() -> Self {
        let rng = XRng::from_rng(rand::rngs::OsRng).expect("unable to initalize RNG");
        let game = Game::new_2p(rng);
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
        Self::Init(InitSt {
            nplayers: 1,
            start_game_err: None,
        })
        // Self::InGame(GameSt::default())
    }
}

#[derive(Clone,Debug)]
enum InGameMsg {
    ClickHandCard(HandCardIdx),
    PutDown(HandCardIdx),
    TakeWith(HandCardIdx),
    DeclareWith(HandCardIdx),
}


impl GameSt {
    fn update_state(&mut self, msg: &InGameMsg) -> Option<Model> {
        match msg {
            InGameMsg::ClickHandCard(x) => {
                self.phase = GamePhase::MyTurn(TurnProgress::CardSelected(*x));
            }

            InGameMsg::PutDown(x) => {
                let action = PlayerAction::Play(*x);
                self.issue_action(&action);
                self.phase = GamePhase::MyTurn(TurnProgress::ActionIssued(action));
            }

            _ => unimplemented!(),
        }

        None
    }

    fn issue_action(&self, act: &PlayerAction) {
        // TODO
        log(format!("Issuing action: {:?}", act));
    }

    fn mk_card_div(&self, card: &Card) -> Node<Msg> {
        let attr = if card.suite.is_red() {
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

    fn mk_table_entry_div(&self, te: &TableEntry) -> Node<Msg> {
        match te {
            TableEntry::Decl(x) => unimplemented!(),
            TableEntry::Card(x) => self.mk_card_div(x),
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

                    let span : Node<Msg> = if card.suite.is_red() {
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

enum Model {
    Init(InitSt),
    InLobby(LobbySt),
    InGame(GameSt),
}

#[derive(Clone,Debug)]
enum Msg {
    Init(InitMsg),
    InGame(InGameMsg),
    Lobby(LobbyMsg),
    Ws(WsEvent),
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    let upd_ret = match (&mut model, msg) {
        (&mut Model::Init(st), Msg::Init(ref msg)) => st.update_state(msg, orders),
        (&mut Model::InLobby(st), Msg::Lobby(ref msg)) => st.update_state(msg, orders),
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
        Model::InGame(st) => st.view(),
        Model::InLobby(st) => st.view(),
    }
}

#[wasm_bindgen(start)]
pub fn render() {
    App::builder(update, view).build_and_start();
}
