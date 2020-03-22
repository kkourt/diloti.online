
extern crate rand;
extern crate rand_pcg;

use seed::{*, prelude::*};
use rand::SeedableRng;
use core::{deck::Deck,
           card::Card,
           game::{Game, PlayerGameView, TableEntry, GameVer, HandCardIdx, TableEntryIdx, PlayerAction}};

type XRng = rand_pcg::Pcg64;

/// Initial state

#[derive(Clone,Debug)]
enum InitMsg {
    StartGame,
    StartGameReply(ResponseDataResult<String>),
    SetPlayerCount(String),
}

struct InitSt {
    nplayers: u8,
    start_game_reply: Option<Result<String, String>>,
}

fn get_create_game_req_url() -> impl Into<std::borrow::Cow<'static, str>> {
    "/creategame"
}

impl InitSt {

    fn update_state(&mut self, msg: &InitMsg, orders: &mut impl Orders<Msg>) -> Option<Model> {
        // log!(format!("*************** {:?}", msg));

        match msg {
            InitMsg::StartGameReply(result) => {
                self.start_game_reply = Some(
                    result.as_ref()
                          .map(|x| String::from(x))
                          .map_err(|x| format!("Could not create new game: {:?}", x))
                );
            },

            InitMsg::StartGame => {
                let url = get_create_game_req_url();
                let req = Request::new(url.into())
                    .method(seed::browser::service::fetch::Method::Put)
                    .fetch_string_data(
                        |s| Msg::Init(InitMsg::StartGameReply(s))
                    );
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
        let initmsg = InitMsg::StartGame;

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

        if let Some(x) = &self.start_game_reply {
            match (x) {
                Ok(x) => {
                    ret.add_child(span![format!("{:?}", x)]);
                },
                Err(x) => {
                    ret.add_child(span!["Failed! :-("]);
                    ret.add_child(p![format!("Error;{:?}", x)]);
                }
            }
        }

        ret

    }
}


/// Game states

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
            start_game_reply: None,
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

enum Model {
    Init(InitSt),
    InGame(GameSt),
}


#[derive(Clone,Debug)]
enum Msg {
    Init(InitMsg),
    InGame(InGameMsg),
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    let upd_ret = match (&mut model, msg) {
        (&mut Model::Init(st), Msg::Init(ref msg)) => st.update_state(msg, orders),
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
    }

    /*
    button![
        simple_ev(Ev::Click, Msg::Increment),
        format!("Hello, World × {}", model.val)
    ]
    */
}

#[wasm_bindgen(start)]
pub fn render() {
    App::builder(update, view).build_and_start();
}
