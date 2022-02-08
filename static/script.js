let cameraListDiv = document.getElementById('camera-list');
let statusDiv = document.getElementById('status');

let cameraTemplate = document.getElementById('camera');

let STATE = {
  cameras: {},
  connected: false,
}

let rtc_configuration = {iceServers: [
                                      {urls: "stun:stun.l.google.com:19302"}]};

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
    sendICECandidate(
      identifier,
      event.candidate.sdpMLineIndex,
      event.candidate.candidate
    );
    console.log("ICE Candidate sent");
  };
  sendCallInit(identifier);
  return true;
}

function setConnectedStatus(status) {
  STATE.connected = status;
  statusDiv.className = (status) ? "status status-connected" : "status status-reconnecting";
}

function getVideoElement(identifier) {
    return document.querySelector(".camera[identifier='" + identifier + "'] .camera__video");
}

function init() {
  subscribe("/events", () => {
    setConnectedStatus(true);
    console.log("connected to event stream");
    sendCameraDiscovery();
  }, (message) => {
    addCamera(message.sender);
  }, (message) => {
    console.log("New incoming SDP message")

    STATE.cameras[message.sender].connection.setRemoteDescription({ type: "offer", sdp: message.description}).then(() => {
      STATE.cameras[message.sender].connection.createAnswer().then((desc) => {
        STATE.cameras[message.sender].connection.setLocalDescription(desc).then(function() {
          sendSDPAnswer(
            message.sender,
            desc.sdp
          );
          console.log("SDP Answer sent");
        });
      });
    });
  }, (message) => {
    console.log("New incoming ICE Candidate")
    STATE.cameras[message.sender].connection.addIceCandidate(new RTCIceCandidate({
      "candidate": message.candidate,
      "sdpMLineIndex": (message.index & 0xFFFF),
    })).catch((err) => {console.log(err);});
  }, () => {
      setConnectedStatus(false);
  });
}

init();
