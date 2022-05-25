class IncomingMessage {
  constructor(sender, payload) {
    this.sender = sender;
    this.payload = payload;
  }
}

class OutgoingMessage {
  constructor(recipient, payload) {
    this.recipient = recipient;
    this.payload[payload.constructor.name] = payload;
  }
}

class Welcome {
  constructor(test) {
    this.test = test;
  }
}

class CameraDiscovery {
  constructor() {
  }
}

class CallInit {
  constructor() {
  }
}

class SDP {
  constructor(description) {
    this.description = description;
  }
}

class ICE {
  constructor(index, candidate) {
    this.index = index;
    this.candidate = candidate;
  }
}

function sendMessage(message) {
  fetch("/message", {
    method: "POST",
    body: JSON.stringify(message),
  }).then((response) => {
    if (response.ok) console.log("Message sent");
  });
}

function sendCameraDiscovery(recipient) {
  sendMessage(new OutgoingMessage("", new CameraDiscovery()));
  console.log(new OutgoingMessage("", new CameraDiscovery()));
}

function sendCallInit(recipient) {
  sendMessage(new OutgoingMessage(recipient, new CameraDiscovery()));
}

function sendSDPAnswer(recipient, description) {
  sendMessage(new OutgoingMessage(recipient, new SDP(description)));
}

function sendICECandidate(recipient, index, candidate) {
  sendMessage(new OutgoingMessage(recipient, new ICE(index, candidate)));
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
      console.log(message);
      /*
      if (message.command == "camera_ping") {
        on_camera_ping(message);
      } else if (message.command == "sdp_offer") {
        on_sdp_offer(message);
      } else if (message.command == "ice_candidate") {
        on_ice_candidate(message);
      } else {
        console.log("Unknown command: ", message);
      }
    */
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
