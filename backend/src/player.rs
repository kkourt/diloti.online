//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//
//

use warp::filters::ws;
use tokio::time::Duration;

use core::srvcli;

use crate::{
    game,
    directory_task,
    game_task::{
        PlayerTaskId,
        GameReq,
        GameTaskTx
    },
    player_task::{
        PlayerTaskMsg,
        PlayerTaskRx,
        PlayerTaskTx,
        PlayerTaskMsg::{RegistrationResult, ForwardToClient}
    },
};

use futures::{SinkExt,StreamExt};

type WsTx = futures::stream::SplitSink<ws::WebSocket, ws::Message>;
type WsRx = futures::stream::SplitStream<ws::WebSocket>;

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
                log::error!("Empty message from client websocket. Failing.");
                Err(())
            }

            Some(Err(x)) => {
                log::error!("Error in client websocket: {:?}. Failing.", x);
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

/// Contact directory task to get the tx endpoint for the game task
async fn get_game_tx(
    dir_tx: &mut directory_task::DirTaskTx,
    gid_str: &str,
) -> Result<GameTaskTx, String> {

    let game_id = game::GameId::from_string(gid_str).ok_or("invalid game id")?;
    // create a oneshot channel for the reply
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<GameTaskTx>>();
    if let Err(x) = dir_tx.send(directory_task::DirReq::GetGameHandle(game_id, tx)).await {
        log::error!("Error sending CreateGame request: {:?}", x);
        return Err("Failed to register player to game".to_string());
    }

    match rx.await {
        Ok(Some(x)) => Ok(x),
        Ok(None) => {
            log::info!("Player requested to join invalid game id ({})", game_id.to_string());
            Err("Invalid game id".to_string())
        },
        Err(e) => {
            log::error!("Failed to get result from directory: {:?}", e);
            Err("Failed to register player to game".to_string())
        }
    }
}

/// Contact game task to register the player
async fn register_player(
    player_name: String,
    game_tx: &mut GameTaskTx,
    player_tx: PlayerTaskTx,
    player_rx: &mut PlayerTaskRx,
) -> Result<PlayerTaskId, String> {
    let req = GameReq::RegisterPlayer(player_tx, player_name);
    if let Err(x) = game_tx.send(req).await {
        log::error!("Error sending RegisterPlayer request: {:?}", x);
        return Err("Failed to register player to game".to_string())
    }

    match player_rx.recv().await {
        Some(RegistrationResult(Ok(x))) => Ok(x),
        Some(RegistrationResult(Err(e))) => Err(e),
        r => {
            log::error!("Error sending RegisterPlayer request: {:?}", r);
            Err("Failed to register player to game".to_string())
        }
    }
}

struct PlayerTaskArg {
    pub self_rx: PlayerTaskRx,
    pub game_tx: GameTaskTx,
    pub pid: PlayerTaskId,
}

/// Setup everything we ned to run the player task.
/// Returns a PlayerTask or a message to convey to the user in case of an error
async fn do_player_setup(
    game_id_s: &str,
    player_name: &str,
    mut dir_tx: &mut directory_task::DirTaskTx,
) -> Result<PlayerTaskArg, String> {
    let mut game_tx = get_game_tx(&mut dir_tx, game_id_s).await?;
    let (player_tx, mut player_rx) = tokio::sync::mpsc::channel::<PlayerTaskMsg>(1024);
    let player_id = register_player(
        player_name.to_string(),
        &mut game_tx,
        player_tx,
        &mut player_rx).await?;

    Ok(PlayerTaskArg {
        self_rx: player_rx,
        game_tx: game_tx,
        pid: player_id,
    })
}


pub async fn player_setup(
    game_id_s: String,
    ws: warp::ws::Ws,
    player_name: String,
    mut dir_tx: directory_task::DirTaskTx,
) -> Result<impl warp::Reply, std::convert::Infallible> {

    // create player task channel and perform the neccessary setup
    let ws_arg = do_player_setup(&game_id_s, &player_name, &mut dir_tx).await;

    // Here we define what will happen at a later point in time (when the protocol upgrade happens)
    // and we return rep which is a reply that will execute the upgrade and spawn a task with our
    // defined closure.
    let rep = ws.on_upgrade(move |mut websocket: warp::filters::ws::WebSocket| async move {
        let mut task = match ws_arg {
            Err(x) => {
                // Send the error message with a custom code and return
                log::info!("game:{}/pname:{} failed to setup player: {}.", &game_id_s, &player_name, &x);
                let msg = ws::Message::close_with(4444u16, x);
                websocket.send(msg).await.unwrap_or(());
                return;
            },
            Ok(arg) => {
                let (ws_tx, ws_rx) : (WsTx, WsRx) = websocket.split();
                PlayerTask {
                    ws_tx: ws_tx,
                    ws_rx: ws_rx,
                    self_rx: arg.self_rx,
                    game_tx: arg.game_tx,
                    pid: arg.pid
                }
            }
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

        log::info!("game:{}/pid:{} player task returns", game_id_s.to_string(), task.pid.0);
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
