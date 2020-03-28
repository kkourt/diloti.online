//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//
//

use warp::http::StatusCode;
use warp::filters::ws;

use core::srvcli;

use crate::game_task::{GameReq, GameTaskTx};
use crate::player_task::{PlayerTaskMsg, PlayerTaskMsg::{RegistrationResult, ForwardToClient}};


use futures::{SinkExt,StreamExt};

pub async fn player_setup(
    ws: warp::ws::Ws,
    mut game_tx: GameTaskTx,
    player_name: String,
) -> Result<impl warp::reply::Reply, StatusCode> {
    // create player task channel
    let (player_tx, mut player_rx) = tokio::sync::mpsc::channel::<PlayerTaskMsg>(1024);

    // register player and get player info from the game task
    let pid: srvcli::PlayerId = {
        let req = GameReq::RegisterPlayer(player_tx, player_name);
        if let Err(x) = game_tx.send(req).await {
            log::error!("Error sending RegisterPlayer request: {:?}", x);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        match player_rx.recv().await {
            Some(RegistrationResult(Ok(x))) => x,
            Some(RegistrationResult(Err(e))) => return Err(StatusCode::CONFLICT),
            r => {
                log::error!("Error sending RegisterPlayer request: {:?}", r);
                return Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    };

    // Here we define what will happen at a later point in time (when the protocol upgrade happens)
    // and we return rep which is a reply that will execute the upgrade and spawn a task with our
    // closure.
    let rep = ws.on_upgrade(move |websocket| async move {
        let self_pid = pid.clone();
        let (mut ws_tx, mut ws_rx) = websocket.split();
        // We either:
        // receive requests from the game tasj and send them to the client
        // receive requests from the client and send them to the game task
        loop {
            tokio::select! {
                // message from client
                cli_req = ws_rx.next() => {
                    match cli_req {
                        None => log::error!("Empty message from client websocket"),
                        Some(Err(x)) => {
                            log::error!("Error in client websocket");
                            unimplemented!()
                        },
                        Some(Ok(climsg)) => {
                            let req_s = climsg.to_str().unwrap();
                            log::info!("Received message: {:?}", req_s);
                            let cli_req: srvcli::ClientMsg = serde_json::from_str(&req_s).unwrap();
                            let req = GameReq::ClientReq(self_pid, cli_req);
                            if let Err(x) = game_tx.send(req).await {
                                log::error!("Error forwarding client request: {:?}", x);
                                unimplemented!();
                            }
                        }
                    }
                }

                // message from game task
                game_req = player_rx.next() => {
                    match game_req {
                        Some(ForwardToClient(x)) => {
                            let json = serde_json::to_string(&x).expect("serialization failed");
                            let msg = ws::Message::text(json);
                            log::info!("Sending message {:?}", msg);
                            ws_tx.send(msg).await.expect("error sending to ws")
                        },
                        Some(RegistrationResult(x)) => {
                            log::error!("Received unexpected registration result: {:?}", x)
                        },
                        None => {
                            log::error!("Received None from game task");
                            panic!();
                        },
                    }
                },

                else => break,
            };
        }
    });

    Ok(rep)
}
