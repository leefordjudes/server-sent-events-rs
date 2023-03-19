use std::{collections::HashMap, sync::Arc, thread, time};

use actix_web::{
    web::{self, Query},
    App, HttpResponse, HttpServer, Responder,
};
use actix_web_lab::extract::Path;

mod broadcast;
use self::broadcast::Broadcaster;

pub struct AppState {
    broadcaster: Arc<Broadcaster>,
}

// SSE
pub async fn start_client(
    state: web::Data<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> impl Responder {
    println!("in api");
    state.broadcaster.new_client(query.get("name")).await
}

pub async fn stop_client(state: web::Data<AppState>, Path(name): Path<String>) -> impl Responder {
    state.broadcaster.stop_client(name);
    HttpResponse::Ok().finish()
}

pub async fn broadcast_msg(state: web::Data<AppState>) -> impl Responder {
    // state.broadcaster.broadcast(&msg).await;
    for i in 1..=1000 {
        println!("{i}");
        thread::sleep(time::Duration::from_secs(1));
        let msg = format!("msg-{i}");
        state.broadcaster.broadcast(&msg).await;
    }
    HttpResponse::Ok().body("msg sent")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let broadcaster = Broadcaster::create();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                broadcaster: Arc::clone(&broadcaster),
            }))
            // This route is used to listen events/ sse events
            .route("/start", web::get().to(start_client))
            .route("/stop/{name}", web::get().to(stop_client))
            // This route will create notification
            .route("/broadcast", web::get().to(broadcast_msg))
    })
    .bind(format!("{}:{}", "127.0.0.1", "8000"))?
    .run()
    .await
}
