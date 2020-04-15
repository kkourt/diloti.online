//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use seed::{*, prelude::*};
use web_sys;
use wasm_bindgen::{JsCast, closure::Closure};

use core::srvcli::{LobbyInfo, ServerMsg, ClientMsg, PlayerTpos};

use crate::{
    Model, Msg,
    to_elem::{tpos_char},
    ws::WsEvent,
    game::{GameSt, },
};

#[derive(Debug,Clone)]
pub enum LobbyMsg {
    IssueStart,
    SwapTpos(PlayerTpos, PlayerTpos),
}

/// Internal lobby state
#[derive(Debug)]
enum State {
    /// Initialized websocket
    Initialized(web_sys::WebSocket),
    /// Initialized websocket, and got a response from the server
    Ready(web_sys::WebSocket, LobbyInfo),
    /// Something went wrong
    Error(String),
}

// NB: so that we use it with std::mem::take()
impl Default for State {
    fn default() -> State {
        State::Error("Internal error: invalid lobby state".to_string())
    }
}

#[derive(Debug)]
pub struct LobbySt {
    /// game identifier
    pub game_id: String,
    pub player_name: String,
    state: State,
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

fn lobby_info_view_players(lobby_info: &LobbyInfo) -> Node<Msg> {
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

        let disconnected = lobby_info.disconnected_players();
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
        } else if disconnected.len() == 0 {
            if all_ready {
                div.add_child(p!["Waiting for admin to start the game"]);
            } else {
                div.add_child(p!["Waiting for other players to join"]);
            }
        }

        if disconnected.len() > 0 {
            let mut ul = ul![];
            for pid in disconnected.iter() {
                let player = lobby_info.get_player(*pid).expect("valid pid");
                ul.add_child(li![player.name]);
            }

            //let hname = web_sys::window().expect("web_sys window").location().host().expect("location");
            div.add_child(div![
                p!["The following players have left:"],
                ul,
                p!["The game cannot start now :("],
                p![a!["Start again", attrs!{At::Href => "/"}]],
            ]);
        }

        div
}

// stolen from seed's examples
pub fn register_ws_handler<T, F>(
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

pub fn get_server_message(msg: &web_sys::MessageEvent) -> Result<ServerMsg, String> {
    let txt = msg.data().as_string().ok_or("No data in server message".to_string())?;
    serde_json::from_str(&txt).map_err(|x| x.to_string())
}

pub fn get_lobby_update(msg: &web_sys::MessageEvent) -> Result<LobbyInfo, String> {
    match get_server_message(msg)? {
        ServerMsg::LobbyUpdate(x) => Ok(x),
        _ => Err("Unexpected server message (not LobbyUpdate)".to_string()),
    }
}

impl LobbySt {

    pub fn view(&self) -> Node<Msg> {
        let body = match &self.state {
            State::Initialized(_) => {
                p!["Contacting server..."]
            },
            State::Ready(_, li) => {
                let mut b = div![];
                // add a join link for admins
                if li.am_i_admin() {
                    let hname = web_sys::window().expect("web_sys window").location().host().expect("location");
                    let join_name = format!("{}/?join={}", hname, self.game_id);
                    let join_href = format!("/?join={}", self.game_id);
                    let join_a = a![join_name, attrs!{
                        At::Href => join_href,
                        // NB: the attributes open the link on a new window.
                        // Maybe it would be better to have just text and/or a copy button.
                        At::Target => "_blank",
                        At::Rel => "noopener noreferrer",
                    }];
                    b.add_child(p!["link for your friends to join: ", join_a,]);
                }

                b.add_child(lobby_info_view_players(&li));
                b
            },
            State::Error(err) => {
                p![err]
            },
        };

        div![
            h2!["Lobby"],
            p![""],
            body,
        ]
    }

    fn get_wsocket_mut(&mut self) -> Option<&mut web_sys::WebSocket> {
        match &mut self.state {
            State::Initialized(wsocket) => Some(wsocket),
            State::Ready(wsocket, _) => Some(wsocket),
            State::Error(_) => None,
        }
    }

    pub fn update_state(&mut self, msg: &LobbyMsg, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        let ws = match self.get_wsocket_mut() {
            Some(x) => x,
            None => {
                error!("Spurious update_state");
                return None;
            },
        };

        let req = match msg {
            LobbyMsg::IssueStart => {
                serde_json::to_string(&ClientMsg::StartGame).unwrap()
            },
            LobbyMsg::SwapTpos(tpos1, tpos2) => {
                serde_json::to_string(&ClientMsg::SwapTpos(*tpos1, *tpos2)).unwrap()
            },
        };

        if let Err(x) = ws.send_with_str(&req) {
            error!("Failed to send data to server");
            self.state = State::Error("Failed to contact server".to_string());
        }

        None
    }

    pub fn handle_ws_event(&mut self, ev: &WsEvent, _orders: &mut impl Orders<Msg>) -> Option<Model> {
        // NB: once we fix the backend, we  can have a better explaination here.
        match ev {
            WsEvent::WsClose(_) | WsEvent::WsError(_) => {
                self.state = State::Error("Error: game (or server) no longer available.".to_string());
                return None;
            }
            _ => (),
        }

        let state = std::mem::take(&mut self.state);
        let new_state = match state {
            State::Error(x) => {
                State::Error(x)
            },

            State::Initialized(ws) => {
                match ev {
                    // websocket connected. Just wait for the server's first message
                    WsEvent::WsConnected(_) => State::Initialized(ws),
                    // The server's first message should be a LobbyUpdate. Once we get that, we
                    // switch to the ready state.
                    WsEvent::WsMessage(msg) => {
                        match get_lobby_update(msg) {
                            Err(x) => {
                                error!(format!("Error while expected LobbyUpdate: {}", x));
                                State::Error("Error contacting server".to_string())
                            },
                            Ok(li) => {
                                State::Ready(ws, li)
                            }
                        }
                    },
                    _ => State::Error("Something went wrong...".to_string())
                }
            },

            State::Ready(ws, lobby_info) => {
                match ev {
                    WsEvent::WsMessage(msg) => {
                        match get_server_message(msg) {
                            Err(x) => {
                                error!(format!("Error while expected LobbyUpdate: {}", x));
                                State::Error("Error contacting server".to_string())
                            },

                            Ok(ServerMsg::LobbyUpdate(new_lobby_info)) => {
                                State::Ready(ws, new_lobby_info)
                            }

                            Ok(ServerMsg::GameUpdate(pview)) => {
                                let game_st = GameSt::new(ws, lobby_info, pview);
                                let new_model = Model::InGame(game_st);
                                return Some(new_model)
                            }

                            Ok(x) => {
                                error!("Got unexpected server message: {:?}", x);
                                State::Error("Something went wrong...".to_string())
                            }
                        }
                    }
                    _ => State::Error("Something went wrong...".to_string())
                }
            },
        };

        self.state = new_state;
        None
    }


    pub fn new(
        game_id: String,
        player_name: String,
        orders: &mut impl Orders<Msg>
    ) -> Result<LobbySt, String> {

        // try to build the websocket URL
        let loc = web_sys::window().ok_or("Failed to get window")?.location();
        let proto = loc.protocol().map_err(|_| "Failed to get protocol".to_string())?;
        let ws_proto = if proto.starts_with("https") { "wss" } else { "ws" };
        let hname = loc.host().map_err(|_| "Failed to get host")?;
        let ws_url = format!("{}://{}/ws/{}/{}", ws_proto, hname, game_id, player_name);

        if let Some(storage) = seed::storage::get_storage() {
            seed::storage::store_data(&storage, "player_name", &player_name);
        }

        let ws = web_sys::WebSocket::new(&ws_url).map_err(|_| "Failed to create websocket")?;
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
            game_id: game_id,
            player_name: player_name,
            state: State::Initialized(ws),
        };

        Ok(ret)
    }
}

// Dropping this does not close the websocket by default, apparently, so we should do it.
impl Drop for LobbySt {
    fn drop(&mut self) {
        let ws = match self.get_wsocket_mut() {
            Some(ws) => ws,
            None => return,
        };

        ws.close().unwrap_or(())
    }
}
