//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use web_sys;

// TODO: move this to lib
#[derive(Debug,Clone)]
pub enum WsEvent {
    WsConnected(wasm_bindgen::JsValue),
    //WsClose(wasm_bindgen::JsValue),
    //WsClose(web_sys::CloseEvent),
    WsClose(web_sys::CloseEvent),
    WsError(wasm_bindgen::JsValue),
    //WsError(web_sys::ErrorEvent),
    WsMessage(web_sys::MessageEvent),
}
