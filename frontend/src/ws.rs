//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use web_sys;
use wasm_bindgen::{JsCast, closure::Closure};
use seed::prelude::*;

use seed::prelude::Orders;

use crate::{Msg};

#[derive(Debug,Clone)]
pub enum WsEvent {
    WsConnected(wasm_bindgen::JsValue),
    WsClose(wasm_bindgen::JsValue),
    WsError(wasm_bindgen::JsValue),
    WsMessage(web_sys::MessageEvent),
}

#[derive(Debug, Clone, Copy)]
pub enum WsState {
    Init,
    Ready,
    Closed,
    Error,
}

#[derive(Debug)]
pub struct Wsocket {
    pub ws: web_sys::WebSocket,
    pub ws_state: WsState,
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

