use std::{collections::HashMap, sync::Arc, time::Duration};

use actix_web::rt::time::interval;
use actix_web_lab::sse::{self, ChannelStream, Sse};

use futures_util::future;
use parking_lot::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;

use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Payload {
    Welcome,
    NewCamera,
    CameraDiscovery,
    CameraPing,
    CallInit,
    SDP { description: String },
    ICE { i: u32, candidate: String },
}

// Incoming Messages
#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage {
    pub recipient: Option<Uuid>,
    pub payload: Payload,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: Uuid,
    pub recipient: Option<Uuid>,
    pub payload: Payload,
}

#[derive(Clone, Serialize)]
struct OutgoingMessage {
    pub sender: Uuid,
    pub payload: Payload,
}

pub struct Broadcaster {
    inner: Mutex<BroadcasterInner>,
}

#[derive(Debug, Clone, Default)]
struct BroadcasterInner {
    clients: HashMap<Uuid, sse::Sender>,
}

impl Broadcaster {
    /// Constructs new broadcaster and spawns ping loop.
    pub fn create() -> Arc<Self> {
        let this = Arc::new(Broadcaster {
            inner: Mutex::new(BroadcasterInner::default()),
        });

        Broadcaster::spawn_ping(Arc::clone(&this));

        this
    }

    /// Pings clients every 10 seconds to see if they are alive and remove them from the broadcast
    /// list if not.
    fn spawn_ping(this: Arc<Self>) {
        actix_web::rt::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;
                this.remove_stale_clients().await;
            }
        });
    }

    /// Removes all non-responsive clients from broadcast list.
    async fn remove_stale_clients(&self) {
        let clients = self.inner.lock().clients.clone();

        let mut ok_clients = HashMap::new();

        for (id, client) in clients {
            if client
                .send(sse::Event::Comment("ping".into()))
                .await
                .is_ok()
            {
                ok_clients.insert(id, client.clone());
            }
        }

        self.inner.lock().clients = ok_clients;
    }

    /// Registers client with broadcaster, returning an SSE response body.
    pub async fn new_client(&self, uuid: Uuid) -> Sse<ChannelStream> {
        let (tx, rx) = sse::channel(10);

        tx.send(sse::Data::new(
            json!(OutgoingMessage {
                sender: Uuid::nil(),
                payload: Payload::Welcome
            })
            .to_string(),
        ))
        .await
        .unwrap();

        self.inner.lock().clients.insert(uuid, tx);

        rx
    }

    pub async fn send(&self, msg: Message) {
        let outgoing_message = OutgoingMessage {
            sender: msg.sender,
            payload: msg.payload,
        };

        match msg.recipient {
            Some(recipient) => self.send_to(recipient, outgoing_message).await,
            None => self.broadcast(outgoing_message).await,
        }
    }

    async fn send_to(&self, recipient: Uuid, msg: OutgoingMessage) {
        let clients = self.inner.lock().clients.clone();

        match clients.get(&recipient) {
            Some(client) => client
                .send(sse::Data::new(json!(msg).to_string()))
                .await
                .unwrap(),
            None => {}
        }
    }

    /// Broadcasts `msg` to all clients.
    async fn broadcast(&self, msg: OutgoingMessage) {
        let clients = self.inner.lock().clients.clone();

        let send_futures = clients.values().into_iter().map(|client| {
            client.send(sse::Data::new(
                json!(OutgoingMessage {
                    sender: msg.sender.clone(),
                    payload: msg.payload.clone(),
                })
                .to_string(),
            ))
        });

        // try to send to all clients, ignoring failures
        // disconnected clients will get swept up by `remove_stale_clients`
        let _ = future::join_all(send_futures).await;
    }
}
