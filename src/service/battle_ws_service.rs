use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};

pub struct BattleWsService {
    //pub repo: Arc<dyn BattleRepository + Send + Sync>,
}

impl BattleWsService {
    pub fn new() -> Self {
        // Self { repo }
        Self {}
    }

    pub async fn echo(&self, socket: WebSocket) {
        let (mut sink, mut stream) = socket.split();

        while let Some(next) = stream.next().await {
            let msg = match next {
                Ok(msg) => msg,
                Err(_) => {
                    break;
                }
            };

            match msg {
                Message::Text(text) => {
                    let reply = format!("Answer: {text}");
                    if sink.send(Message::Text(reply.into())).await.is_err() {
                        break;
                    }
                }
                Message::Binary(_) => {}
                Message::Ping(_) => {}
                Message::Pong(_) => {}
                Message::Close(_) => break,
            }
        }
    }
}
