#[macro_use] extern crate rocket;
use rocket::{State, Shutdown};
use rocket::http::{Cookie, Status};
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize, json::Json, uuid::Uuid};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;

use rocket::request::{FromRequest, Outcome, Request};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
enum Payload {
    Welcome,
    NewCamera,
    CameraDiscovery,
    CameraPing,
    CallInit,
    SDP {
        description: String
    },
    ICE {
        index: u32,
        candidate: String
    }
}

// Incoming Messages
#[derive(Debug, Clone, Deserialize)]
#[serde(crate = "rocket::serde")]
struct IncomingMessage {
    pub recipient: Option<String>,
    pub payload: Payload
}

#[derive(Clone)]
struct Message {
    pub sender: String,
    pub recipient: Option<String>,
    pub payload: Payload
}

#[derive(Clone, Serialize)]
#[serde(crate = "rocket::serde")]
struct OutgoingMessage {
    pub sender: String,
    pub payload: Payload
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown, user: User) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        yield Event::json(&OutgoingMessage {
            sender: "server".to_string(),
            payload: Payload::Welcome
        });
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            let user_id = Some(user.user_id.clone());
            if msg.recipient == user_id || ((msg.payload == Payload::CameraDiscovery || msg.payload == Payload::NewCamera ) && msg.sender != user.user_id) {
                println!("Sending message from {:?} to {:?}", msg.sender, msg.recipient);
                yield Event::json(&OutgoingMessage {
                    sender: msg.sender,
                    payload: msg.payload
                });
            }
        }
    }
}

fn queue_message(message: IncomingMessage, queue: &State<Sender<Message>>, user: User) -> Status {
    match queue.send(Message {
        sender: user.user_id,
        recipient: message.recipient,
        payload: message.payload
    }) {
        Ok(_) => Status::Ok,
        Err(_) => Status::InternalServerError
    }
}

#[post("/message", data = "<message>")]
fn message(message: Json<IncomingMessage>, queue: &State<Sender<Message>>, user: User) -> Status {
    match message.payload {
        Payload::Welcome => {
            return Status::Ok;
        }
        Payload::NewCamera => {
            if message.recipient != None {
                return Status::BadRequest;
            }
        },
        Payload::CameraDiscovery => {
            if message.recipient != None {
                return Status::BadRequest;
            }
        },
        Payload::CameraPing => {
            if message.recipient == None || message.recipient == Some(user.user_id.clone()) {
                return Status::BadRequest;
            }
        },
        Payload::CallInit => {
            if message.recipient == None || message.recipient == Some(user.user_id.clone()) {
                return Status::BadRequest;
            }
        },
        Payload::SDP {..} => {
            if message.recipient == None || message.recipient == Some(user.user_id.clone()) {
                return Status::BadRequest;
            }
        },
        Payload::ICE {..} => {
            if message.recipient == None || message.recipient == Some(user.user_id.clone()) {
                return Status::BadRequest;
            }
        },
        //Useful in case of version mismatch between cameras, server and clients
        #[allow(unreachable_patterns)]
        _ => {
            eprintln!("Error: server does not support this payload type: {:?}", message.payload);
        }
    }
    queue_message(message.into_inner(), queue, user)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct User {
    user_id: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        let user_id: String = match request.cookies().get_private("user_id") {
            Some(c) => String::from(c.value()),
            None => {
                let new_user_id = Uuid::new_v4().to_hyphenated().to_string();
                let res = new_user_id.clone();
                request.cookies().add_private(Cookie::new("user_id", new_user_id));
                println!("======= generating new user_id: {} =======", res);
                res
            }
        };
        rocket::request::Outcome::Success(User {
            user_id
        })
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount("/", routes![events, message])
        .mount("/", FileServer::from(relative!("static")).rank(1))
}

