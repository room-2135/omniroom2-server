let cameraListDiv = document.getElementById('camera-list');
let statusDiv = document.getElementById('status');

let cameraTemplate = document.getElementById('camera');

let STATE = {
  cameras: {},
  connected: false,
}

let connection;
let rtc_configuration = {iceServers: [
                                      {urls: "stun:stun.l.google.com:19302"}]};


function Message(command, identifier, sdp_type, sdp, ice_candidate_index, ice_candidate) {
  this.command = command;
  this.identifier = identifier;
  this.sdp_type = sdp_type;
  this.sdp = sdp;
  this.ice_candidate_index = ice_candidate_index;
  this.ice_candidate = ice_candidate;
}

function sendMessage(message) {
  fetch("/message", {
    method: "POST",
    body: JSON.stringify(message),
  }).then((response) => {
    if (response.ok) console.log("Message sent");
  });
}

function addCamera(identifier) {
  if (STATE.cameras[identifier]) {
    return false;
  }
  console.log("Adding a new camera");

  var node = cameraTemplate.content.cloneNode(true);
  var room = node.querySelector(".camera");
  room.identifier = identifier;
  cameraListDiv.appendChild(node);

  STATE.cameras[identifier] = [];
  sendMessage(new Message(
    "call",
    "client1",
    "",
    "",
    0,
    ""
  ));
  return true;
}

function subscribe(uri) {
  var retryTime = 1;

  function connect(uri) {
    const events = new EventSource(uri);

    events.addEventListener("open", () => {
      setConnectedStatus(true);
      console.log(`connected to event stream at ${uri}`);
      retryTime = 1;
      sendMessage(new Message(
        "list_cameras",
        "client1",
        "",
        "",
        0,
        ""
      ));

    });

    events.addEventListener("message", (ev) => {
      const message = JSON.parse(ev.data);
      if (message.identifier == "client1") {
        return;
      }
      console.log("incoming message: ", message);
      if (message.command == "camera_ping") {
        addCamera(message.identifier);
      } else if (message.command == "sdp_offer") {
        console.log("New incoming SDP message")
        connection.setRemoteDescription({ type: message.sdp_type, sdp: message.sdp}).then(() => {
          connection.createAnswer().then((desc) => {
            connection.setLocalDescription(desc).then(function() {
              sendMessage(new Message(
                "sdp_answer",
                "client1",
                desc.type,
                desc.sdp,
                0,
                ""
              ));
              console.log("SDP Answer sent");
            });
          });
        });
      } else if (message.command == "ice_candidate") {
        console.log("New incoming ICE Candidate")
        connection.addIceCandidate(new RTCIceCandidate({
          "candidate": message.ice_candidate.candidate,
          "sdpMLineIndex": (message.ice_candidate.index & 0xFFFF),
        })).catch((err) => {console.log(err);});
      } else {
        console.log("Unknown command: ", message);
      }
    });

    events.addEventListener("error", () => {
      setConnectedStatus(false);
      events.close();

      let timeout = retryTime;
      retryTime = Math.min(64, retryTime * 2);
      console.log(`connection lost. attempting to reconnect in ${timeout}s`);
      setTimeout(() => connect(uri), (() => timeout * 1000)());
    });
  }

  connect(uri);
}

function setConnectedStatus(status) {
  STATE.connected = status;
  statusDiv.className = (status) ? "status status-connected" : "status status-reconnecting";
}

function getVideoElement() {
    return document.querySelector(".camera .camera__video");
}

function init() {
  connection = new RTCPeerConnection(rtc_configuration);

  connection.ontrack = (event) => {
    console.log("New incoming stream");
    let videoElement = getVideoElement();
    console.log(videoElement);
    console.log(event.streams);
    videoElement.srcObject = event.streams[0];
  };
  connection.onicecandidate = (event) => {
    console.log("ICE Candidate created");
    if (event.candidate == null) {
      console.log("ICE Candidate was null, done");
      return;
    }
    console.log(event.candidate);
    sendMessage(new Message(
      "ice_candidate",
      "client1",
      "",
      "",
      event.candidate.sdpMLineIndex,
      event.candidate.candidate
    ));
    console.log("ICE Candidate sent");
  };

  subscribe("/events");
}

init();
