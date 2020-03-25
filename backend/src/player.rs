//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//
//

use warp::http::StatusCode;
use warp::filters::ws;

use crate::game;
use crate::game_task::{GameReq, GameTaskTx};
use crate::player_task::{PlayerTaskMsg, PlayerTaskMsg::{RegistrationResult, ForwardToClient}};

use futures::{SinkExt,StreamExt};

pub async fn player_setup(
    ws: warp::ws::Ws,
    mut game_tx: GameTaskTx)
-> Result<impl warp::reply::Reply, StatusCode> {
    // create player task channel
    let (player_tx, mut player_rx) = tokio::sync::mpsc::channel::<PlayerTaskMsg>(1024);

    // register player and get player info from the game task
    let pid: game::PlayerId = {
        if let Err(x) = game_tx.send(GameReq::RegisterPlayer(player_tx)).await {
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
    let rep = ws.on_upgrade(|websocket| async move {
        let (mut ws_tx, mut ws_rx) = websocket.split();
        // We either:
        // receive requests from the game tasj and send them to the client
        // receive requests from the client and send them to the game task
        loop {
            tokio::select! {
                cli_req = ws_rx.next() => unimplemented!(),
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
                            log::error!("Received None from game task")
                        },
                    };
                },
                else => break,
            };
        }
    });

    Ok(rep)
}
