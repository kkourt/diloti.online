//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use seed::{*, prelude::*};
use web_sys;

use core::{
    srvcli::{ClientMsg, ServerMsg, PlayerId, LobbyInfo},
};

use crate::{
    Msg, Model,
    to_elem::{iter_to_elem, tpos_char, ToElem, },
    ws::WsEvent,
};

/// Game state

pub struct GameSt {
    view: core::PlayerGameView,
    phase: GamePhase,
    lobby_info: LobbyInfo,
    wsocket: web_sys::WebSocket,
    tmp_error_msg: String,
}

#[derive(Clone,Debug)]
pub enum InGameMsg {
    ClickHandCard(usize),
    ClickTableEntry(usize),
    LayDown(usize),
    DeclareWith(usize), // card index
    DeclareSetSum(u8),  // sum
    CaptureWith(usize),
    FinalizePhase,
    ContinueGame,
}

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
    PlayersDisconnected(Vec<PlayerId>),
    RoundDone,
    GameDone(Vec<(core::ScoreSheet, usize)>),
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
            PlayersDisconnected(_) | RoundDone | GameDone(_) => None,
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
            PlayersDisconnected(_) | RoundDone | GameDone(_) => false,
        }
    }
}

impl From<(&LobbyInfo, &core::PlayerGameView)> for GamePhase {
    fn from(pieces: (&LobbyInfo, &core::PlayerGameView)) -> GamePhase {
        let (lobby_info, pview) = pieces;

        let disconnected_ps = lobby_info.disconnected_players();
        if disconnected_ps.len() > 0 {
            return GamePhase::PlayersDisconnected(disconnected_ps);
        }

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



impl GameSt {

    pub fn new(wsocket: web_sys::WebSocket, lobby_info: LobbyInfo, pview: core::PlayerGameView) -> GameSt {
        let phase: GamePhase = (&lobby_info, &pview).into();
        GameSt {
            view: pview,
            phase: phase,
            lobby_info: lobby_info,
            wsocket: wsocket,
            tmp_error_msg: "".into(),
        }
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
            p![format!("Invalid play: {}", err), style!{"color" => "red"}],
            p!["Still your turn! Play a card from your hand."],
        ];
        self.phase = GamePhase::MyTurn(TurnProgress::Nothing(user_msg));
    }

    fn issue_action(&mut self, act: core::PlayerAction) {
        let climsg = ClientMsg::PlayerAction(act.clone());
        let req = serde_json::to_string(&climsg).unwrap();
        if let Err(x) = self.wsocket.send_with_str(&req) {
            error!("Failed to send data to server");
            unimplemented!();
        }

        self.phase = GamePhase::MyTurn(TurnProgress::ActionIssued(act));
    }

    fn issue_action_validate(&mut self, act: core::PlayerAction) {
        //log(format!("Issuing action: {:?}", act));
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

    pub fn update_state(&mut self, msg: &InGameMsg) -> Option<Model> {
        use GamePhase::*;
        use TurnProgress::*;

        self.tmp_error_msg = "".into();
        match msg {
            InGameMsg::ClickHandCard(x) => {
                let new_phase = match self.phase {
                    OthersTurn(_) => None,
                    MyTurn(Nothing(_)) => Some(MyTurn(CardSelected(*x))),
                    MyTurn(ActionIssued(_)) => None,

                    // if a card is selected, clicking the card will return to initial state
                    MyTurn(CardSelected(prev_x)) => {
                        let msg = p!["Your turn to play (select a card from your hand)"];
                        Some(MyTurn(Nothing(msg)))
                    },

                    // if an action is selected, clicking on the selected card will reset the
                    // action
                    MyTurn(DeclaringWith(prev_x,_)) |
                    MyTurn(CapturingWith(prev_x,_)) => {
                        Some(MyTurn(CardSelected(*x)))
                    },

                    PlayersDisconnected(_) | RoundDone | GameDone(_) => None,
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
                    RoundDone => None,
                    PlayersDisconnected(_) => None,
                    GameDone(_) => None,

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

            InGameMsg::ContinueGame => {
                let req = serde_json::to_string(&ClientMsg::StartGame).unwrap();
                if let Err(x) = self.wsocket.send_with_str(&req) {
                    error!("Failed to send data to server");
                    unimplemented!();
                }
                return None;
            }
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
                self.view = pview;
                self.phase = (&self.lobby_info, &self.view).into();
                return None;
            },

            ServerMsg::LobbyUpdate(linfo) => {
                self.lobby_info = linfo;
                self.phase = (&self.lobby_info, &self.view).into();
                return None;
            }

        }
    }

    pub fn handle_ws_event(&mut self, ev: &WsEvent, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        // log!("ev: {:?}", ev);
        match ev {
            // TODO: proper error handling
            WsEvent::WsMessage(msg) => {
                let txt = msg.data().as_string().expect("No data in server message");
                //log(format!("Received message {:?}", txt));
                let srv_msg: ServerMsg = serde_json::from_str(&txt).unwrap();
                self.handle_server_message(srv_msg)
            },

            // TODO: proper error handling
            // (WsEvent::WsClose(_), _) => _,
            _ => unimplemented!(),
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

    fn view_table(&self) -> Node<Msg> {
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
            p![format!("Table (total: {} -- you might have to scroll down)", self.view.table.nentries())],
            div![ attrs!{At::Class => "table"},  entries ],
        ]
    }

    fn view_hand(&self) -> Node<Msg> {
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

        let hand_attrs =  if self.myturn() {
            let mut attrs_ = attrs!{};
            attrs_.add_multiple(At::Class, &["hand", "active"]);
            attrs_
        } else {
            attrs!{At::Class => "hand"}
         };

        div![
            attrs!{At::Class => "container"},
            //p!["Hand"],
            p![format!("Hand (total: {} -- you might have to scroll down)", self.view.own_hand.ncards())],
            div![hand_attrs, cards],
        ]
    }

    fn view_players_disconnected(&self, ps: &Vec<PlayerId>) -> Node<Msg> {
        let mut ul = ul![];
        for pid in ps.iter() {
            let player = self.lobby_info.get_player(*pid).expect("valid pid");
            ul.add_child(li![player.name]);
        }

        div![
            p!["Cannot continue. The following players are disconnected:"],
            ul,
        ]
    }

    fn view_phase(&self) -> Node<Msg> {
        // TODO: if a user has a declaration on the table, be helpful about their possible actions
        // :)
        let phase_elem = match &self.phase {
            GamePhase::OthersTurn(pid) => {
                let player = self.lobby_info.get_player(pid.clone()).unwrap();
                p![format!("Waiting for {} ({})", player.name, tpos_char(player.tpos))]
            },
            GamePhase::MyTurn(TurnProgress::Nothing(msg)) => msg.clone(),
            GamePhase::MyTurn(TurnProgress::CardSelected(cidx)) => self.view_selected_card(*cidx),
            GamePhase::MyTurn(TurnProgress::DeclaringWith(cidx, ts)) => self.view_declaration(*cidx, ts),
            GamePhase::MyTurn(TurnProgress::CapturingWith(cidx, cb)) => self.view_capture(*cidx, cb),
            GamePhase::MyTurn(TurnProgress::ActionIssued(a)) => p!["Issued action. Waiting for server."],
            GamePhase::GameDone(scores) => self.view_score(scores),
            GamePhase::RoundDone => p!["Round done! Wait for new cards."],
            GamePhase::PlayersDisconnected(ps) => self.view_players_disconnected(ps),
        };
        let mut phase = div![
            attrs!{At::Class => "container"},
            phase_elem,
        ];

        if self.tmp_error_msg.len() > 0 {
            phase.add_child(p![ class!["error-msg"], self.tmp_error_msg ]);
        }

        phase
    }

    fn view_players(&self) -> Node<Msg> {
        let mut players = div![p!["Players"]];
        let tpos_active = self.view.active_tpos();
        for (tpos, player) in self.lobby_info.iter_players_tpos() {
            let c = tpos_char(tpos);
            let attrs = if Some(tpos) == tpos_active {
                attrs!{At::Class => "active-player"}
            } else {
                attrs!{At::Class => "inactive-player"}
            };
            players.add_child(span!(attrs, format!("{} {} ", c, player.name)));
        }
        players
    }

    fn view_last_action(&self) -> Node<Msg> {
        let la = match self.view.last_action.as_ref() {
            None => return div![""],
            Some(x) => x,
        };

        let mut act_elem = match &la.action {
            core::PlayerAction::LayDown(c) => span!["laid down ", c.to_elem(),],
            core::PlayerAction::Capture(ca) if la.xeri => {
                let mut table_cards = ca.get_table_cards();
                span!["made a «ξερή» capturing ",
                      iter_to_elem("", table_cards.drain(..), ""),
                      " with ",
                      ca.handcard.to_elem()
                ]
            },
            core::PlayerAction::Capture(ca) => {
                let mut table_cards = ca.get_table_cards();
                span!["captured ",
                      iter_to_elem("", table_cards.drain(..), ""),
                      " with ",
                      ca.handcard.to_elem()
                ]
            },
            core::PlayerAction::Declare(da) => {
                match da.get_decl() {
                    None => span![
                        format!("created a declaration of value {} with ", da.value()),
                        da.handcard().to_elem(),
                    ],
                    Some(decl) if decl.value() < da.value() => span![
                        format!("raised a declaration from {} to {} with ", decl.value(), da.value()),
                        da.handcard().to_elem(),
                    ],
                    Some(decl) if decl.value() == da.value() => span![
                        format!("added to a declaration of value {} a ", da.value()),
                        da.handcard().to_elem(),
                    ],
                    _ => panic!("Invalid decl"),

                }
            }
        };

        if la.forced_cards.len() > 0 {
            act_elem.add_child(
                iter_to_elem(" (forced table cards:", la.forced_cards.iter().cloned(), ")")
            );
        }
        act_elem.add_child(span!["."]);

        let pname = self.lobby_info.player_from_tpos(la.player).unwrap().name.clone();
        div![
            span![format!("Last action from {}: ", pname)],
            act_elem
        ]
    }

    pub fn view(&self) -> Node<Msg> {
        match self.phase {
            GamePhase::MyTurn(_) |
            GamePhase::OthersTurn(_) |
            GamePhase::RoundDone |
            GamePhase::PlayersDisconnected(_) => {
                let players = self.view_players();
                let table = self.view_table();
                let hand = self.view_hand();
                let phase = self.view_phase();
                let last_action = self.view_last_action();
                let rem = p![format!("Remaining cards in the deck: {}", self.view.main_deck_sz)];
                div![players, table, hand, phase, last_action, rem]
            },

            GamePhase::GameDone(_) => {
                let phase = self.view_phase();
                let last_action = self.view_last_action();

                let cont = if self.lobby_info.am_i_admin() {
                    let msg = Msg::InGame(InGameMsg::ContinueGame);
                    let button = button![simple_ev(Ev::Click, msg), "Continue game"];
                    p![button]
                } else {
                    p![""]
                };

                div![h3!["Game done!"], last_action, phase, cont]

            }
        }
    }

    fn view_score(&self, sheets: &Vec<(core::ScoreSheet, usize)>) -> Node<Msg> {
        assert!(sheets.len() == self.lobby_info.nteams());
        let mut game_rows  = vec![tr![th!["player(s)"], th!["score"], th!["points"]]];
        let mut total_rows = vec![tr![th!["player(s)"], th!["score"]]];
        for (i, (ss, total_score)) in sheets.iter().enumerate() {
            let team_str = self.lobby_info
                .team_tpos(i)
                .iter()
                //.map( |tpos| format!("{} ({})", self.lobby_info.player_from_tpos(*tpos).unwrap().name, tpos_char(*tpos)))
                .map( |tpos| self.lobby_info.player_from_tpos(*tpos).unwrap().name.clone())
                .collect::<Vec<_>>()
                .join(", ");

            let players_td = td![team_str];
            let points_td = td![b![ss.score.to_string()], attrs!{At::Class => "score"}];
            let details_td = td![ss.to_elem()];
            let total_points_td = td![b![total_score.to_string()], attrs!{At::Class => "score"}];

            game_rows.push(tr![players_td.clone(), points_td, details_td]);
            total_rows.push(tr![players_td, total_points_td]);
        }

        let mut div = div![ h3!["Game score"] ];
        let table = table![game_rows, attrs!{At::Class => "scoring-table"}, ];
        div.add_child(table);

        div.add_child(div![ h3!["Total score"] ]);
        let table_total = table![total_rows, attrs!{At::Class => "scoring-table"}, ];
        div.add_child(table_total);

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
                let mut opts = vec![
                    option!["?", attrs!{At::Disabled => "true", At::Selected => "true"}],
                ];
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
                for i in sum_set.iter().clone() {
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
