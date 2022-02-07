#[macro_use] extern crate rocket;

use rocket::{State, Shutdown};
use rocket::http::{Cookie, CookieJar};
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize, json::Json, uuid::Uuid};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct IncomingMessage {
    #[field(validate = len(..20))]
    pub command: String,
    pub recipient: String,
    pub sdp_type: Option<String>,
    pub sdp: Option<String>,
    pub ice_candidate_index: Option<u32>,
    pub ice_candidate: Option<String>,
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..20))]
    pub command: String,
    pub sender: String,
    pub recipient: String,
    pub sdp_type: Option<String>,
    pub sdp: Option<String>,
    pub ice_candidate_index: Option<u32>,
    pub ice_candidate: Option<String>,
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct OutgoingMessage {
    #[field(validate = len(..20))]
    pub command: String,
    pub sender: String,
    pub sdp_type: Option<String>,
    pub sdp: Option<String>,
    pub ice_candidate_index: Option<u32>,
    pub ice_candidate: Option<String>,
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown, cookies: &CookieJar<'_>) -> EventStream![] {
    let mut rx = queue.subscribe();
    let user_id: String = match cookies.get_private("user_id") {
        Some(c) => String::from(c.value()),
        None => {
            let new_user_id = Uuid::new_v4().to_hyphenated().to_string();
            let res = new_user_id.clone();
            cookies.add_private(Cookie::new("user_id", new_user_id));
            println!("======= generating new user_id: {} =======", res);
            res
        }
    };
    EventStream! {
        yield Event::json(&OutgoingMessage {
            command: "welcome".to_string(),
            sender: "".to_string(),
            sdp_type: None,
            sdp: None,
            ice_candidate_index: None,
            ice_candidate: None
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
            let user_id = user_id.clone();
            if msg.recipient != msg.sender && (msg.recipient == user_id || ((msg.command == "list_cameras" || msg.command == "camera_ping") && msg.sender != user_id)) {
                yield Event::json(&OutgoingMessage {
                    command: msg.command,
                    sender: msg.sender,
                    sdp_type: msg.sdp_type,
                    sdp: msg.sdp,
                    ice_candidate_index: msg.ice_candidate_index,
                    ice_candidate: msg.ice_candidate
                });
            }
        }
    }
}

#[post("/message", data = "<message>")]
fn post(message: Json<IncomingMessage>, queue: &State<Sender<Message>>, cookies: &CookieJar<'_>) {
    let user_id: String = match cookies.get_private("user_id") {
        Some(c) => String::from(c.value()),
        None => {
            let new_user_id = Uuid::new_v4().to_hyphenated().to_string();
            let res = new_user_id.clone();
            cookies.add_private(Cookie::new("user_id", new_user_id));
            println!("======= generating new user_id: {} =======", res);
            res
        }
    };
    let incoming_message = message.into_inner();
    let _res = queue.send(Message {
        command: incoming_message.command,
        sender: user_id,
        recipient: incoming_message.recipient,
        sdp_type: incoming_message.sdp_type,
        sdp: incoming_message.sdp,
        ice_candidate_index: incoming_message.ice_candidate_index,
        ice_candidate: incoming_message.ice_candidate
    });
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount("/", routes![post, events])
        .mount("/", FileServer::from(relative!("static")).rank(1))
}

