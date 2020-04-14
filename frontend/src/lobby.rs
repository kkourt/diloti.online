//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use seed::{*, prelude::*};

use core::srvcli::{LobbyInfo, ServerMsg, ClientMsg, PlayerTpos};

use crate::{
    Model, Msg,
    ws::{WsEvent, WsState, Wsocket, register_ws_handler},
    game::{GameSt, },
    to_elem::{tpos_char},
};

#[derive(Debug,Clone)]
pub enum LobbyMsg {
    IssueStart,
    SwapTpos(PlayerTpos, PlayerTpos),
}

#[derive(Debug)]
pub struct LobbySt {
    /// server info
    pub lobby_info: Option<LobbyInfo>,
    /// game identifier
    pub game_id: String,
    pub player_name: String,
    /// websocket to server
    pub wsocket: std::rc::Rc<Wsocket>,
}

fn lobby_tpos_elem(lobby_info: &LobbyInfo, tpos: PlayerTpos) -> Node<Msg> {
    let am_admin = lobby_info.am_i_admin();
    let all_ready = lobby_info.all_ready();

    let msg_fn = |tpos: PlayerTpos| {
        let tpos1 = tpos.clone();
        move |x: String| {
            let tpos2 = PlayerTpos(x.parse::<u8>().unwrap());
            Msg::Lobby(LobbyMsg::SwapTpos(tpos1, tpos2))
        }
    };

    let tpos_str = tpos.0.to_string();
    let mut select_opts = vec![];
    for i in 0..lobby_info.nplayers {
        let tpos_i = PlayerTpos(i);
        let mut attrs = attrs!{At::Value => i.to_string()};
        if tpos_i == tpos {
            attrs.add(At::Selected, "true")
        }
        if !am_admin || !all_ready {
            attrs.add(At::Disabled, "true")
        }
        select_opts.push(option![tpos_char(tpos_i).to_string(), attrs, ]);
    }

    select![
        attrs!{At::Value => tpos.0.to_string()},
        select_opts,
        input_ev(Ev::Input, msg_fn(tpos))
    ]
}

impl LobbySt {

    fn view_players(&self) -> Node<Msg> {
        let lobby_info: &LobbyInfo = self.lobby_info.as_ref().unwrap();
        let am_admin = lobby_info.am_i_admin();
        let all_ready = lobby_info.all_ready();
        let nplayers = lobby_info.nplayers;

        let mut player_rows = vec![];

        if nplayers == 4 {
            player_rows.push(tr![th!["pos"], th!["name"], th![""], th!["team"]]);
        } else {
            player_rows.push(tr![th!["pos"], th!["name"], th![""]]);
        }

        for (tpos, player) in lobby_info.iter_players_tpos() {
            let mut vattrs = vec![];
            if lobby_info.is_self_from_tpos(tpos) {
                vattrs.push("you");
            }
            if player.admin {
                vattrs.push("admin");
            }

            if !player.connected {
                vattrs.push("disconnected \u{2718}");
            }

            let td_tpos = td![lobby_tpos_elem(lobby_info, tpos)];
            let td_name = td![player.name];
            let td_attrs = if vattrs.len() > 0 {
                td![format!("({})", vattrs.join(", "))]
            } else { td![""] };

            if nplayers == 4 {
                let td_team = td![
                    if tpos.0 % 2 == 0 {
                        "\u{25cf}" // black circle
                    } else {
                        "\u{25cb}" // white cirlce
                    }
                ];
                player_rows.push(tr![td_tpos, td_name, td_attrs, td_team]);
            } else {
                player_rows.push(tr![td_tpos, td_name, td_attrs]);
            }
        }

        let mut div = div![
            h3![format!("Players ({}/{})", lobby_info.players.len(), lobby_info.nplayers)],
            table![player_rows, attrs!{At::Class => "lobby-players"}, ]
        ];

        if am_admin {
            let attrs = if !all_ready {
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

        let disconnected = self.lobby_info.as_ref().map_or(vec![], |li| li.disconnected_players());
        if disconnected.len() > 0 {
            let lobby_info = self.lobby_info.as_ref().expect("valid lobby");
            let mut ul = ul![];
            for pid in disconnected.iter() {
                let player = lobby_info.get_player(*pid).expect("valid pid");
                ul.add_child(li![player.name]);
            }
            div.add_child(div![p!["The following players are disconnected:"], ul, ]);
        }

        div
    }

    pub fn view(&self) -> Node<Msg> {
        let hname = web_sys::window().expect("web_sys window").location().host().expect("location");
        let join_name = format!("{}/?join={}", hname, self.game_id);
        let join_href = format!("/?join={}", self.game_id);
        let body = if self.lobby_info.is_some() { self.view_players() } else { p!["--"] };

        let a = a![join_name, attrs!{
            At::Href => join_href,
            At::Target => "_blank",
            At::Rel => "noopener noreferrer",
        }];
        div![
            h2!["Lobby"],
            p!["link for your friends to join: ", a,],
            p![""],
            body,
        ]
    }

    pub fn update_state(&mut self, msg: &LobbyMsg, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        match msg {
            LobbyMsg::IssueStart => {
                let req = serde_json::to_string(&ClientMsg::StartGame).unwrap();
                if let Err(x) = self.wsocket.ws.send_with_str(&req) {
                    error!("Failed to send data to server");
                    unimplemented!();
                }
                None
            },
            LobbyMsg::SwapTpos(tpos1, tpos2) => {
                let req = serde_json::to_string(&ClientMsg::SwapTpos(*tpos1, *tpos2)).unwrap();
                if let Err(x) = self.wsocket.ws.send_with_str(&req) {
                    error!("Failed to send data to server");
                    unimplemented!();
                }
                None
            },
        }
    }

    pub fn handle_ws_event(&mut self, ev: &WsEvent, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        match (ev, self.wsocket.ws_state) {
            // websocket connected: change ws state to ready
            (WsEvent::WsConnected(jv), WsState::Init) => {
                {
                    //log(format!("Connected: {}", jv.as_string().unwrap_or("<None>".to_string())));
                    let ws = std::rc::Rc::get_mut(&mut self.wsocket).unwrap();
                    ws.ws_state = WsState::Ready;
                }
            },

            // websocket got a message
            (WsEvent::WsMessage(msg), WsState::Ready) => {
                let txt = msg.data().as_string().expect("No data in server message");
                //log(format!("Received message {:?}", txt));
                let srv_msg: ServerMsg = serde_json::from_str(&txt).unwrap();
                self.lobby_info = Some(match srv_msg {
                    ServerMsg::LobbyUpdate(x) => x,
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


    pub fn new(
        game_id: String,
        player_name: String,
        orders: &mut impl Orders<Msg>
    ) -> Result<LobbySt, String> {
        let loc = web_sys::window().expect("web_sys::window").location();
        let proto = loc.protocol().expect("location protocol");
        let ws_proto = if proto.starts_with("https") { "wss" } else { "ws" };
        let hname = loc.host().expect("location host");
        let ws_url = format!("{}://{}/ws/{}/{}", ws_proto, hname, game_id, player_name);
        // log(format!("**************** ws_url={}", ws_url));
        if let Some(storage) = seed::storage::get_storage() {
            seed::storage::store_data(&storage, "player_name", &player_name);
        }

        let wsocket = std::rc::Rc::<Wsocket>::new(Wsocket {
            ws: web_sys::WebSocket::new(&ws_url).expect(&format!("new websocket on {:?}", ws_url)),
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
