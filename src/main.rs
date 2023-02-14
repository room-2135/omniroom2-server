use std::{io, sync::Arc};

use actix_files as fs;
use actix_session::Session;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::{get, middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder};

use uuid::Uuid;

mod broadcast;
use broadcast::Broadcaster;
use broadcast::{IncomingMessage, Message, Payload};

fn get_user_from_session(session: &Session) -> Uuid {
    let existing_user: Option<Uuid> = session.get("user_id").unwrap();
    dbg!(existing_user);
    match existing_user {
        Some(user_id) => user_id,
        None => {
            let user_id = Uuid::new_v4();
            println!(
                "======= generating new user_id: {} =======",
                user_id.hyphenated().to_string()
            );
            session.insert("user_id", user_id).unwrap();
            user_id
        }
    }
}

#[get("/events")]
async fn event_stream(broadcaster: web::Data<Broadcaster>, session: Session) -> impl Responder {
    let user = get_user_from_session(&session);
    broadcaster.new_client(user).await
}

#[post("/message")]
async fn message(broadcaster: web::Data<Broadcaster>, session: Session) -> impl Responder {
    let user = get_user_from_session(&session);

    if let Some(message) = &session.get::<IncomingMessage>("body").unwrap() {
        match message.payload {
            Payload::Welcome => {
                return HttpResponse::Ok().body("");
            }
            Payload::NewCamera => {
                if message.recipient != None {
                    return HttpResponse::BadRequest().body("");
                }
            }
            Payload::CameraDiscovery => {
                if message.recipient != None {
                    return HttpResponse::BadRequest().body("");
                }
            }
            Payload::CameraPing => {
                if message.recipient == None || message.recipient == Some(user) {
                    return HttpResponse::BadRequest().body("");
                }
            }
            Payload::CallInit => {
                if message.recipient == None || message.recipient == Some(user) {
                    return HttpResponse::BadRequest().body("");
                }
            }
            Payload::SDP { .. } => {
                if message.recipient == None || message.recipient == Some(user) {
                    return HttpResponse::BadRequest().body("");
                }
            }
            Payload::ICE { .. } => {
                if message.recipient == None || message.recipient == Some(user) {
                    return HttpResponse::BadRequest().body("");
                }
            }
            //Useful in case of version mismatch between cameras, server and clients
            #[allow(unreachable_patterns)]
            _ => {
                eprintln!(
                    "Error: server does not support this payload type: {:?}",
                    message.payload
                );
            }
        }

        broadcaster
            .send(Message {
                sender: user,
                recipient: message.recipient,
                payload: message.payload.clone(),
            })
            .await;
    }

    HttpResponse::Ok().body("msg sent")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let data = Broadcaster::create();

    let secret_key: Key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(Arc::clone(&data)))
            .service(event_stream)
            .service(message)
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .wrap(Logger::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
    })
    .bind(("0.0.0.0", 8000))?
    .workers(2)
    .run()
    .await
}
