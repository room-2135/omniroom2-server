function IncomingMessage(command, sender, description, ice_candidate_index, ice_candidate) {
  this.command = command;
  this.sender = sender;
  this.description = description;
  this.index = ice_candidate_index;
  this.candidate = ice_candidate;
}

function CameraDiscoveryOutgoingMessage() {
}

function CallInitOutgoingMessage(recipient) {
  this.recipient = recipient;
}

function SDPAnswerOutgoingMessage(recipient, description) {
  this.recipient = recipient;
  this.description = description;
}

function ICECandidateOutgoingMessage(recipient, index, candidate) {
  this.recipient = recipient;
  this.index = index;
  this.candidate = candidate;
}

function sendMessage(url_key, message) {
  fetch("/message/" + url_key, {
    method: "POST",
    body: JSON.stringify(message),
  }).then((response) => {
    if (response.ok) console.log("Message sent");
  });
}

function sendCameraDiscovery(recipient) {
  sendMessage("camera_discovery", new CameraDiscoveryOutgoingMessage());
}

function sendCallInit(recipient) {
  sendMessage("call_init", new CallInitOutgoingMessage(
    recipient
  ));
}

function sendSDPAnswer(recipient, description) {
  sendMessage("sdp_answer", new SDPAnswerOutgoingMessage(
    recipient,
    description
  ));
}

function sendICECandidate(recipient, index, candidate) {
  sendMessage("ice_candidate", new ICECandidateOutgoingMessage(
    recipient,
    index,
    candidate
  ));
}

function subscribe(uri, on_connected, on_camera_ping, on_sdp_offer, on_ice_candidate, on_error) {
  var retryTime = 1;

  function connect(uri) {
    const events = new EventSource(uri);

    events.addEventListener("open", () => {
      retryTime = 1;
      on_connected();
    });

    events.addEventListener("message", (ev) => {
      const message = JSON.parse(ev.data);
      if (message.command == "camera_ping") {
        on_camera_ping(message);
      } else if (message.command == "sdp_offer") {
        on_sdp_offer(message);
      } else if (message.command == "ice_candidate") {
        on_ice_candidate(message);
      } else {
        console.log("Unknown command: ", message);
      }
    });

    events.addEventListener("error", () => {
      events.close();
      on_error();

      let timeout = retryTime;
      retryTime = Math.min(64, retryTime * 2);
      console.log(`connection lost. attempting to reconnect in ${timeout}s`);
      setTimeout(() => connect(uri), (() => timeout * 1000)());
    });
  }

  connect(uri);
}
