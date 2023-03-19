use nanoid::nanoid;
use std::{sync::Arc, time::Duration};

use actix_web::rt::time::interval;
use actix_web_lab::sse::{self, ChannelStream, Sse};
use futures_util::future;
use parking_lot::Mutex;

#[derive(Debug, Clone)]
struct SseClient {
    client: sse::Sender,
    name: String,
}

#[derive(Debug, Clone, Default)]
struct BroadcasterInner {
    clients: Vec<SseClient>,
}

pub struct Broadcaster {
    inner: Mutex<BroadcasterInner>,
}

impl Broadcaster {
    pub fn create() -> Arc<Self> {
        let this = Arc::new(Broadcaster {
            inner: Mutex::new(BroadcasterInner::default()),
        });
        Broadcaster::spawn_ping(Arc::clone(&this));
        this
    }

    fn spawn_ping(this: Arc<Self>) {
        actix_web::rt::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            loop {
                interval.tick().await;
                this.remove_stale_clients().await;
            }
        });
    }

    pub fn stop_client(&self, name: String) {
        self.inner.lock().clients.retain(|c| c.name != name);
    }

    async fn remove_stale_clients(&self) {
        let clients = self.inner.lock().clients.clone();
        println!("running clients {:?}", clients.len());

        let mut ok_clients = Vec::new();

        for client in clients {
            let ping = format!("ping from: {}", &client.name);
            if client
                .client
                .send(sse::Event::Comment(ping.into()))
                .await
                .is_ok()
            {
                ok_clients.push(client.clone());
            }
        }
        println!("current active clients {:?}", ok_clients.len());
        self.inner.lock().clients = ok_clients;
    }

    pub async fn new_client(&self, client_name: Option<&String>) -> Sse<ChannelStream> {
        let alphabet: [char; 16] = [
            '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
        ];
        println!("starting creation");
        let (tx, rx) = sse::channel(10);

        tx.send(sse::Data::new("connected")).await.unwrap();
        let name = if let Some(name) = client_name {
            println!("received name: {}", name);
            name.to_owned()
        } else {
            nanoid!(3, &alphabet)
        };
        let client = SseClient { client: tx, name };
        println!("creating new clients success {:?}", &client.name);
        self.inner.lock().clients.push(client);
        rx
    }

    pub async fn broadcast(&self, msg: &str) {
        let clients = self.inner.lock().clients.clone();
        let send_futures = clients.iter().map(|client| {
            client
                .client
                .send(sse::Data::new(format!("{}: {}", &client.name, msg)))
        });

        // try to send to all clients, ignoring failures
        // disconnected clients will get swept up by `remove_stale_clients`
        let _ = future::join_all(send_futures).await;
    }
}
