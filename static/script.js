let cameraListDiv = document.getElementById('camera-list');
let statusDiv = document.getElementById('status');

let cameraTemplate = document.getElementById('camera');

let STATE = {
  cameras: {},
  connected: false,
}

let rtc_configuration = {iceServers: [
                                      {urls: "stun:stun.l.google.com:19302"}]};

function IncomingMessage(command, sender, sdp_type, sdp, ice_candidate_index, ice_candidate) {
  this.command = command;
  this.sender = sender;
  this.sdp_type = sdp_type;
  this.sdp = sdp;
  this.ice_candidate_index = ice_candidate_index;
  this.ice_candidate = ice_candidate;
}

function OutgoingMessage(command, recipient, sdp_type, sdp, ice_candidate_index, ice_candidate) {
  this.command = command;
  this.recipient = recipient;
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
  let videoElement = getVideoElement(identifier);
  if (videoElement == null) {
    console.log("Adding a new camera: " + identifier);

    let template = cameraTemplate.content.cloneNode(true);
    let new_camera = template.querySelector(".camera");
    cameraListDiv.appendChild(new_camera);
    new_camera.setAttribute("identifier", identifier);
    videoElement = new_camera.querySelector(".camera__video");
  }

  STATE.cameras[identifier] = {};
  STATE.cameras[identifier].connection = new RTCPeerConnection(rtc_configuration);
  STATE.cameras[identifier].connection.ontrack = (event) => {
    console.log("New incoming stream");
    console.log(videoElement);
    console.log(event.streams);
    videoElement.srcObject = event.streams[0];
  };

  STATE.cameras[identifier].connection.onicecandidate = (event) => {
    console.log("ICE Candidate created");
    if (event.candidate == null) {
      console.log("ICE Candidate was null, done");
      return;
    }
    console.log(event.candidate);
    sendMessage(new OutgoingMessage(
      "ice_candidate",
      identifier,
      "",
      "",
      event.candidate.sdpMLineIndex,
      event.candidate.candidate
    ));
    console.log("ICE Candidate sent");
  };
  sendMessage(new OutgoingMessage(
    "call",
    identifier,
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
      sendMessage(new OutgoingMessage(
        "list_cameras",
        "",
        "",
        "",
        0,
        ""
      ));

    });

    events.addEventListener("message", (ev) => {
      console.log(STATE);
      const message = JSON.parse(ev.data);
      console.log("incoming message: ", message);
      if (message.command == "camera_ping") {
        addCamera(message.sender);
      } else if (message.command == "sdp_offer") {
        console.log("New incoming SDP message")

        STATE.cameras[message.sender].connection.setRemoteDescription({ type: message.sdp_type, sdp: message.sdp}).then(() => {
          STATE.cameras[message.sender].connection.createAnswer().then((desc) => {
            STATE.cameras[message.sender].connection.setLocalDescription(desc).then(function() {
              sendMessage(new OutgoingMessage(
                "sdp_answer",
                message.sender,
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
        STATE.cameras[message.sender].connection.addIceCandidate(new RTCIceCandidate({
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

function getVideoElement(identifier) {
    return document.querySelector(".camera[identifier='" + identifier + "'] .camera__video");
}

function init() {
  subscribe("/events");
}

init();
