//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//
//

use warp::http::StatusCode;
use warp::filters::ws;
use tokio::time::Duration;

use core::srvcli;

use crate::game;
use crate::game_task::{
    PlayerTaskId,
    GameReq,
    GameTaskTx
};
use crate::player_task::{
    PlayerTaskMsg,
    PlayerTaskRx,
    PlayerTaskMsg::{RegistrationResult, ForwardToClient}
};

use futures::{SinkExt,StreamExt};

type WsTx = futures::stream::SplitSink<ws::WebSocket, ws::Message>;
type WsRx = futures::stream::SplitStream<ws::WebSocket>;
type SelfRx = u32;
type GaTx = u32;

struct PlayerTask {
    pub ws_tx: WsTx,
    pub ws_rx: WsRx,
    pub self_rx: PlayerTaskRx,
    pub game_tx: GameTaskTx,
    pub pid: PlayerTaskId,
}

impl PlayerTask {
    async fn do_handle_climsg(&mut self, climsg: ws::Message) -> Result<(), ()> {
        // Close(Some(CloseFrame { code: Away, reason: "" }))
        if climsg.is_close() {
            log::info!("Received close");
            Err(())
        } else if climsg.is_ping() {
            log::info!("Received ping");
            log::error!("TODO: implement pong");
            Err(())
        } else if climsg.is_text() {
            log::info!("Received message: {:?}", climsg);
            let req_s = climsg.to_str().expect("already checked");
            let cli_req: srvcli::ClientMsg = serde_json::from_str(&req_s).expect("ClientMsg serializable");
            let req = GameReq::ClientReq(self.pid.clone(), cli_req);
            if let Err(x) = self.game_tx.send(req).await {
                log::error!("Error forwarding client request: {:?}", x);
                Err(())
            } else {
                Ok(())
            }
        } else {
            log::error!("Received unexpected message");
            Err(())
        }
    }

    pub async fn handle_climsg(&mut self, cli_req: Option<Result<ws::Message, warp::Error>>) -> Result<(),()> {
        match cli_req {
            None => {
                log::error!("Empty message from client websocket. Bailing out");
                Err(())
            }

            Some(Err(x)) => {
                log::error!("Error in client websocket");
                Err(())
            },

            Some(Ok(climsg)) => {
                self.do_handle_climsg(climsg).await
            }
        }
    }

    pub async fn handle_game_req(&mut self, game_req: Option<PlayerTaskMsg>) -> Result<(), ()> {
        match game_req {
            Some(ForwardToClient(x)) => {
                let json = serde_json::to_string(&x).expect("serialization failed");
                log::info!("Sending message to client: {}", json);
                let msg = ws::Message::text(json);
                if let Err(x) = self.ws_tx.send(msg).await {
                    log::error!("Error forwarding message to client: {:?}", x);
                    Err(())
                } else {
                    Ok(())
                }
            },
            Some(RegistrationResult(x)) => {
                log::error!("Received unexpected registration result: {:?}", x);
                Err(())
            },
            None => {
                log::error!("Received None from game task");
                Err(())
            },
        }
    }
}

pub async fn player_setup(
    game_id: game::GameId,
    ws: warp::ws::Ws,
    mut game_tx: GameTaskTx,
    player_name: String,
) -> Result<impl warp::reply::Reply, StatusCode> {
    // create player task channel
    let (player_tx, mut player_rx) = tokio::sync::mpsc::channel::<PlayerTaskMsg>(1024);

    // register player and get player info from the game task
    let pid: PlayerTaskId = {
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
    // defined closure.
    let rep = ws.on_upgrade(move |websocket: warp::filters::ws::WebSocket| async move {
        let (ws_tx, ws_rx) : (WsTx, WsRx) = websocket.split();
        let mut task = PlayerTask {
            ws_tx: ws_tx,
            ws_rx: ws_rx,
            self_rx: player_rx,
            game_tx: game_tx,
            pid: pid.clone(),
        };

        // We either:
        // receive requests from the game task and send them to the client
        // receive requests from the client and send them to the game task
        loop {
            tokio::select! {
                cli_req = task.ws_rx.next() => {
                    match task.handle_climsg(cli_req).await {
                        Err(()) => break,
                        Ok(()) => (),
                    }
                },

                game_req = task.self_rx.next() => {
                    match task.handle_game_req(game_req).await {
                        Err(()) => break,
                        Ok(()) => (),
                    }
                },

                else => break,
            };
        }

        log::info!("game:{}/pid:{} player task returns", game_id.to_string(), task.pid.0);
        // Attemt to send a player disconnected to the game task
        {
            let dur = Duration::from_millis(10);
            let msg = GameReq::PlayerTaskTerminated(task.pid.clone());
            if let Err(x) = task.game_tx.send_timeout(msg, dur).await {
                log::info!("Error sending PlayerDisconnect to game task: {}", x)
            }
        }

    });

    Ok(rep)
}
