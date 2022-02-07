#[macro_use] extern crate rocket;

use rocket::{State, Shutdown};
use rocket::http::Cookie;
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize, json::Json, uuid::Uuid};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;

use rocket::request::{FromRequest, Outcome, Request};

// Incoming Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct GenericIncomingMessage {
    pub recipient: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct CameraPingIncomingMessage {
    pub recipient: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct SDPOfferIncomingMessage {
    pub recipient: String,
    pub description: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SDPAnswerIncomingMessage {
    pub recipient: String,
    pub description: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ICECandidateIncomingMessage {
    pub recipient: String,
    pub index: u32,
    pub candidate: String
}


#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct IncomingMessage {
    #[field(validate = len(..20))]
    pub command: String,
    pub recipient: String,
    pub description: Option<String>,
    pub index: Option<u32>,
    pub candidate: Option<String>,
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..20))]
    pub command: String,
    pub sender: String,
    pub recipient: String,
    pub description: Option<String>,
    pub index: Option<u32>,
    pub candidate: Option<String>,
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct OutgoingMessage {
    #[field(validate = len(..20))]
    pub command: String,
    pub sender: String,
    pub description: Option<String>,
    pub index: Option<u32>,
    pub candidate: Option<String>,
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown, user: User) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };
            if msg.recipient != msg.sender && ((msg.recipient == user.user_id || msg.command == "camera_ping") && msg.sender != user.user_id) {
                yield Event::json(&OutgoingMessage {
                    command: msg.command,
                    sender: msg.sender,
                    description: msg.description,
                    index: msg.index,
                    candidate: msg.candidate
                });
            }
        }
    }
}

#[post("/message/camera_ping", data = "<message>")]
fn camera_ping(message: Json<CameraPingIncomingMessage>, queue: &State<Sender<Message>>, user: User) {
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: "camera_ping".to_string(),
        sender: user.user_id,
        recipient: incoming_message.recipient,
        description: None,
        index: None,
        candidate: None
    });
}

#[post("/message/call_init", data = "<message>")]
fn call_init(message: Json<GenericIncomingMessage>, queue: &State<Sender<Message>>, user: User) {
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: "call_init".to_string(),
        sender: user.user_id,
        recipient: incoming_message.recipient,
        description: None,
        index: None,
        candidate: None
    });
}

#[post("/message/sdp_offer", data = "<message>")]
fn sdp_offer(message: Json<SDPOfferIncomingMessage>, queue: &State<Sender<Message>>, user: User) {
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: "sdp_offer".to_string(),
        sender: user.user_id,
        recipient: incoming_message.recipient,
        description: Some(incoming_message.description),
        index: None,
        candidate: None
    });
}

#[post("/message/sdp_answer", data = "<message>")]
fn sdp_answer(message: Json<SDPAnswerIncomingMessage>, queue: &State<Sender<Message>>, user: User) {
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: "sdp_answer".to_string(),
        sender: user.user_id,
        recipient: incoming_message.recipient,
        description: Some(incoming_message.description),
        index: None,
        candidate: None
    });
}

#[post("/message/ice_candidate", data = "<message>")]
fn ice_candidate(message: Json<ICECandidateIncomingMessage>, queue: &State<Sender<Message>>, user: User) {
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: "ice_candidate".to_string(),
        sender: user.user_id,
        recipient: incoming_message.recipient,
        description: None,
        index: Some(incoming_message.index),
        candidate: Some(incoming_message.candidate),
    });
}

#[post("/message", data = "<message>")]
fn post(message: Json<IncomingMessage>, queue: &State<Sender<Message>>, user: User) {
    let user_id = user.user_id;
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: incoming_message.command,
        sender: user_id,
        recipient: incoming_message.recipient,
        description: incoming_message.description,
        index: incoming_message.index,
        candidate: incoming_message.candidate
    });
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
        .mount("/", routes![camera_ping, call_init, sdp_offer, sdp_answer, ice_candidate, post, events])
        .mount("/", FileServer::from(relative!("static")).rank(1))
}

