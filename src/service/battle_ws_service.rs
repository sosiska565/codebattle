use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::models::battle_room::{ClienMsg, ServerMsg};
use crate::service::battle_service::BattleService;

pub struct BattleWsService {
    battle_service: Arc<BattleService>,
}

impl BattleWsService {
    pub fn new(battle_service: Arc<BattleService>) -> Self {
        Self { battle_service }
    }

    pub async fn handle_battle(&self, socket: WebSocket, battle_id: Uuid, user_id: Uuid) {
        let (mut sink, mut stream) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<ServerMsg>();

        let writer = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let Ok(text) = serde_json::to_string(&msg) else {
                    continue;
                };
                if sink.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        });

        let conn_id = Uuid::new_v4();

        if let Err(e) = self
            .battle_service
            .join(battle_id, user_id, conn_id, tx.clone())
            .await
        {
            let _ = tx.send(ServerMsg::Error {
                message: e.to_string(),
            });
            drop(tx);
            let _ = writer.await;
            return;
        }

        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Text(text) => match serde_json::from_str::<ClienMsg>(text.as_str()) {
                    Ok(ClienMsg::Submit { language, code }) => {
                        let service = self.battle_service.clone();
                        tokio::spawn(async move {
                            service.submit(battle_id, user_id, language, code).await;
                        });
                    }
                    Ok(ClienMsg::Surrender) => {
                        self.battle_service.surrender(battle_id, user_id).await;
                    }
                    Err(e) => tracing::warn!("некорректное сообщение клиента: {e}"),
                },
                _ => {}
            }
        }

        self.battle_service.leave(battle_id, user_id, conn_id).await;
        writer.abort();
    }

    // pub async fn echo(&self, socket: WebSocket) {
    //     let (mut sink, mut stream) = socket.split();
    //
    //     while let Some(next) = stream.next().await {
    //         let msg = match next {
    //             Ok(msg) => msg,
    //             Err(_) => {
    //                 break;
    //             }
    //         };
    //
    //         match msg {
    //             Message::Text(text) => {
    //                 let reply = format!("Answer: {text}");
    //                 if sink.send(Message::Text(reply.into())).await.is_err() {
    //                     break;
    //                 }
    //             }
    //             Message::Binary(_) => {}
    //             Message::Ping(_) => {}
    //             Message::Pong(_) => {}
    //             Message::Close(_) => break,
    //         }
    //     }
    // }
}
