use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::battle_room::{PlayerInfo, ServerMsg};
use crate::models::users::User;
use crate::service::battle_service::BattleService;

struct Waiting {
    info: PlayerInfo,
    conn_id: Uuid,
    sender: mpsc::UnboundedSender<ServerMsg>,
}

pub struct MatchmakingService {
    queue: Mutex<Vec<Waiting>>,
    battle_service: Arc<BattleService>,
}

impl MatchmakingService {
    pub fn new(battle_service: Arc<BattleService>) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            battle_service,
        }
    }

    pub async fn handle(&self, socket: WebSocket, user: User) {
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
        let info = PlayerInfo {
            id: user.id,
            username: user.username,
            elo: user.elo,
        };

        if let Err(e) = self.enqueue(info, conn_id, tx.clone()).await {
            let _ = tx.send(ServerMsg::Error {
                message: e.to_string(),
            });
            drop(tx);
            let _ = writer.await;
            return;
        }

        while let Some(Ok(msg)) = stream.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }

        self.dequeue(conn_id).await;
        writer.abort();
    }

    async fn enqueue(
        &self,
        info: PlayerInfo,
        conn_id: Uuid,
        sender: mpsc::UnboundedSender<ServerMsg>,
    ) -> Result<(), AppError> {
        let mut queue = self.queue.lock().await;

        queue.retain(|w| !w.sender.is_closed() && w.info.id != info.id);

        let best = queue
            .iter()
            .enumerate()
            .min_by_key(|(_, w)| (w.info.elo - info.elo).abs())
            .map(|(i, _)| i);

        match best {
            None => {
                queue.push(Waiting {
                    info,
                    conn_id,
                    sender,
                });
                Ok(())
            }
            Some(i) => {
                let opponent = queue.remove(i);
                drop(queue);

                match self.battle_service.create_room(&info, &opponent.info).await {
                    Ok(battle_id) => {
                        let _ = opponent.sender.send(ServerMsg::MatchFound {
                            battle_id,
                            opponent: info.clone(),
                        });
                        let _ = sender.send(ServerMsg::MatchFound {
                            battle_id,
                            opponent: opponent.info.clone(),
                        });
                        Ok(())
                    }
                    Err(e) => {
                        let _ = opponent.sender.send(ServerMsg::Error {
                            message: "Не удалось создать бой".to_string(),
                        });
                        Err(e)
                    }
                }
            }
        }
    }

    async fn dequeue(&self, conn_id: Uuid) {
        self.queue.lock().await.retain(|w| w.conn_id != conn_id);
    }
}
