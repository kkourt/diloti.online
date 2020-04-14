//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

// Initial page (where you can create a game)

use std::convert::TryFrom;
use seed::{*, prelude::*};

use core::srvcli::{CreateRep, CreateReq, CreateReqDebug};
use crate::{
    DEFAULT_NR_PLAYERS, Model, Msg,
    lobby::{LobbySt},
    to_elem::{ToElem},
};

fn get_create_game_req_url() -> impl Into<std::borrow::Cow<'static, str>> {
    "/creategame"
}

#[derive(Clone,Debug)]
pub enum InitMsg {
    StartGame,
    StartGameReply(seed::ResponseDataResult<CreateRep>),
    SetPlayerCount(String),
    SetPlayerName(String),
    DebugHandCards(String),
    DebugTableCards(String),
}

pub struct InitSt {
    /// Number of players
    pub nplayers: u8,
    /// Error when trying to start a game
    pub start_game_err: Option<String>,
    pub player_name: String,

    pub debug_hand: String,
    pub debug_table: String,
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

    pub fn update_state(&mut self, msg: &InitMsg, orders: &mut impl Orders<Msg>) -> Option<Model> {
        // log!(format!("*************** {:?}", msg));

        match msg {
            InitMsg::StartGameReply(result) => {
                match result {
                    // change state to lobby
                    Ok(rep) => {
                        let ret = LobbySt::new(rep.game_id.clone(), self.player_name.clone(), orders,);
                        match ret {
                            Ok(st) => return Some(Model::InLobby(st)),
                            Err(x) => {
                                self.start_game_err = Some("Could not create new game".to_string());
                                log!(format!("Error creating game: {:?}", x));
                                return None;
                            }
                        }
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

    fn footer(&self) -> Node<Msg> {
        let email = "kk@diloti.online";
        let email_a = a![
            email,
            attrs!{At::Href => format!("mailto:{}", email)}
        ];
        let p = p!["For comments and suggestions email me at ", email_a, "."];
        let div = div![p, attrs!{At::Id => "footer"}];
        div
    }

    pub fn view(&self) -> Node<Msg> {
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

        ret.add_child(self.footer());
        ret
    }
}
