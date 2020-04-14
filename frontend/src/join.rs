//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use seed::{*, prelude::*};

use crate::{
    Model, Msg,
    lobby::{LobbySt, },
};

/// Join state

#[derive(Debug, Clone)]
pub enum JoinMsg {
    JoinGame,
    SetPlayerName(String),
}

pub struct JoinSt {
    pub game_id: String,
    pub player_name: String,
    pub join_game_err: Option<String>,
}

impl JoinSt {
    pub fn update_state(&mut self, msg: &JoinMsg, orders: &mut impl Orders<Msg>) -> Option<Model> {
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

    pub fn view(&self) -> Node<Msg> {
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
