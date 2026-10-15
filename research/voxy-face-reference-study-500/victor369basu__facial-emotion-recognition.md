# Facial rendering source scan: victor369basu/facial-emotion-recognition

Repository: https://github.com/victor369basu/facial-emotion-recognition

Pinned source revision: 3cd21098ba2c2c9308bacd51d2d99387348ad95a

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: server/main.py

{
  "path": "server/main.py",
  "blob_sha": "5fac22c78abba477be9ac310e896c256a1dad19f",
  "sha256": "304b019c0edade94c8566e66006b0c85a09df1122df55c1ce7d1201cb70874a1",
  "lines": 36,
  "bytes": 1017,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/victor369basu/facial-emotion-recognition/blob/3cd21098ba2c2c9308bacd51d2d99387348ad95a/server/main.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "async def websocket_endpoint(websocket: WebSocket):"
    }
  ]
}

```text

import json
import io
import base64

from typing import Optional
from fastapi import FastAPI, WebSocket

import cv2
import numpy as np
from fer import FER

app = FastAPI()
detector = FER()

@app.websocket("/")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    #while True:
    try:
        payload = await websocket.receive_text()
        payload = json.loads(payload)
        imageByt64 = payload['data']['image'].split(',')[1]
        # decode and convert into image
        image = np.fromstring(base64.b64decode(imageByt64), np.uint8)
        image = cv2.imdecode(image, cv2.IMREAD_COLOR)
        
        # Detect Emotion via Tensorflow model
        prediction = detector.detect_emotions(image)
        response = {
            "predictions": prediction[0]['emotions'],
            "emotion": max(prediction[0]['emotions'], key=prediction[0]['emotions'].get)
        }
        await websocket.send_json(response)
        websocket.close()
    except:
        websocket.close()

```

## Source file: emotion-recognition/src/App.js

{
  "path": "emotion-recognition/src/App.js",
  "blob_sha": "08f216130cfb9d9f9d4afb0f4d54a88dd2e90712",
  "sha256": "dd169f4f902c002919071692aaa87e7e4247ad9cb96858c04cec491356713369",
  "lines": 173,
  "bytes": 5592,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 7,
    "face": 9
  },
  "url": "https://github.com/victor369basu/facial-emotion-recognition/blob/3cd21098ba2c2c9308bacd51d2d99387348ad95a/emotion-recognition/src/App.js",
  "function_signature_candidates": []
}

```text

import React, { useRef, useEffect } from "react";
import "./App.css";
import logo from './logo.svg';

import * as tf from "@tensorflow/tfjs";
import Webcam from "react-webcam";
import { drawMesh } from "./utilities";

function App() {
  const webcamRef = useRef(null);
  const canvasRef = useRef(null);
  const blazeface = require('@tensorflow-models/blazeface')

  //  Load blazeface
  const runFaceDetectorModel = async () => {

    const model = await blazeface.load()
    console.log("FaceDetection Model is Loaded..") 
    setInterval(() => {
      detect(model);
    }, 100);
 
  }

  const detect = async (net) => {
    if (
      typeof webcamRef.current !== "undefined" &&
      webcamRef.current !== null &&
      webcamRef.current.video.readyState === 4
    ) {
      // Get Video Properties
      const video = webcamRef.current.video;
      const videoWidth = webcamRef.current.video.videoWidth;
      const videoHeight = webcamRef.current.video.videoHeight;

      // Set video width
      webcamRef.current.video.width = videoWidth;
      webcamRef.current.video.height = videoHeight;

      // Set canvas width
      canvasRef.current.width = videoWidth;
      canvasRef.current.height = videoHeight;

      // Make Detections
      const face = await net.estimateFaces(video);
      //console.log(face);

      // Websocket
      var socket = new WebSocket('ws://localhost:8000')
      var imageSrc = webcamRef.current.getScreenshot()
      var apiCall = {
        event: "localhost:subscribe",
        data: { 
          image: imageSrc
        },
      };
      socket.onopen = () => socket.send(JSON.stringify(apiCall))
      socket.onmessage = function(event) {
        var pred_log = JSON.parse(event.data)
        document.getElementById("Angry").value = Math.round(pred_log['predictions']['angry']*100)
        document.getElementById("Neutral").value = Math.round(pred_log['predictions']['neutral']*100)
        document.getElementById("Happy").value = Math.round(pred_log['predictions']['happy']*100)
        document.getElementById("Fear").value = Math.round(pred_log['predictions']['fear']*100)
        document.getElementById("Surprise").value = Math.round(pred_log['predictions']['surprise']*100)
        document.getElementById("Sad").value = Math.round(pred_log['predictions']['sad']*100)
        document.getElementById("Disgust").value = Math.round(pred_log['predictions']['disgust']*100)

        document.getElementById("emotion_text").value = pred_log['emotion']

        // Get canvas context
        const ctx = canvasRef.current.getContext("2d");
        requestAnimationFrame(()=>{drawMesh(face, pred_log, ctx)});
      }
    }
  };

  useEffect(()=>{runFaceDetectorModel()}, []);
  return (
    <div className="App">
      <Webcam
          ref={webcamRef}
          style={{
            position: "absolute",
            marginLeft: "auto",
            marginRight: "auto",
            left: 0,
            right: 600,
            top:20,
            textAlign: "center",
            zindex: 9,
            width: 640,
            height: 480,
          }}
        />

        <canvas
          ref={canvasRef}
          style={{
            position: "absolute",
            marginLeft: "auto",
            marginRight: "auto",
            left: 0,
            right: 600,
            top:20,
            textAlign: "center",
            zindex: 9,
            width: 640,
            height: 480,
          }}
        />
      <header className="App-header">
        <img src={logo} 
        className="App-logo" 
        alt="logo"
        style={{
          position: "absolute",
          marginLeft: "auto",
          marginRight: "auto",
          bottom:10,
          left: 0,
          right: 0,
          width: 150,
          height: 150,
        }}
        />   
        <div className="Prediction" style={{
          position:"absolute",
          right:100,
          width:500,
          top: 60
        }}>
          <label forhtml="Angry" style={{color:'red'}}>Angry </label>
          <progress id="Angry" value="0" max = "100" >10%</progress>
          <br></br>
          <br></br>
          <label forhtml="Neutral" style={{color:'lightgreen'}}>Neutral </label>
          <progress id="Neutral" value="0" max = "100">10%</progress>
          <br></br>
          <br></br>
          <label forhtml="Happy" style={{color:'orange'}}>Happy </label>
          <progress id="Happy" value="0" max = "100" >10%</progress>
          <br></br>
          <br></br>
          <label forhtml="Fear" style={{color:'lightblue'}}>Fear </label>
          <progress id="Fear" value="0" max = "100" >10%</progress>
          <br></br>
          <br></br>
          <label forhtml="Surprise" style={{color:'yellow'}}>Surprised </label>
          <progress id="Surprise" value="0" max = "100" >10%</progress>
          <br></br>
          <br></br>
          <label forhtml="Sad" style={{color:'gray'}} >Sad </label>
          <progress id="Sad" value="0" max = "100" >10%</progress>
          <br></br>
          <br></br>
          <label forhtml="Disgust" style={{color:'pink'}} >Disgusted </label>
          <progress id="Disgust" value="0" max = "100" >10%</progress>
        </div>
        <input id="emotion_text" name="emotion_text" vale="Neutral"
               style={{
                 position:"absolute",
                 width:200,
                 height:50,
                 bottom:60,
                 left:300,
                 "font-size": "30px",
               }}></input>
      </header>
    </div>
  );
}

export default App;


```

## Source file: emotion-recognition/src/index.js

{
  "path": "emotion-recognition/src/index.js",
  "blob_sha": "ef2edf8ea3fc42258464231e29140c8723458c1e",
  "sha256": "14c67ad38f79b00139b690f6ca2ddc04525e38a0a3b34ce4ffdc820b64726aee",
  "lines": 17,
  "bytes": 500,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/victor369basu/facial-emotion-recognition/blob/3cd21098ba2c2c9308bacd51d2d99387348ad95a/emotion-recognition/src/index.js",
  "function_signature_candidates": []
}

```text

import React from 'react';
import ReactDOM from 'react-dom';
import './index.css';
import App from './App';
import reportWebVitals from './reportWebVitals';

ReactDOM.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
  document.getElementById('root')
);

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();


```

## Source file: emotion-recognition/src/utilities.js

{
  "path": "emotion-recognition/src/utilities.js",
  "blob_sha": "dde3853c9cee2d6b084179044c263c6be8ba8534",
  "sha256": "4f99a2ad02307f6938bb7b9003fbc9a6b7af055f754ebcc66bc018b5ed896359",
  "lines": 45,
  "bytes": 1348,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/victor369basu/facial-emotion-recognition/blob/3cd21098ba2c2c9308bacd51d2d99387348ad95a/emotion-recognition/src/utilities.js",
  "function_signature_candidates": []
}

```text

// Drawing Mesh
export const drawMesh = (predictions, emotions, ctx) => {
var emo = emotions['emotion']
if (predictions.length > 0) {
    for (let i = 0; i < predictions.length; i++) {
        const start = predictions[i].topLeft;
        const end = predictions[i].bottomRight;
        const size = [end[0] - start[0], end[1] - start[1]];
  
        // Render a rectangle over each detected face.
        ctx.beginPath();
        ctx.lineWidth = "3";
        if (emo==='angry'){
          ctx.fillStyle = 'red';
        }
        else if (emo==='neutral'){
          ctx.fillStyle = 'green';
        }
        else if (emo==='happy'){
          ctx.fillStyle = 'orange';
        }
        else if (emo==='fear'){
          ctx.fillStyle = 'blue';
        }
        else if (emo==='surprise'){
          ctx.fillStyle = 'yellow';
        }
        else if (emo==='sad'){
          ctx.fillStyle = 'gray';
        }
        else {
          ctx.fillStyle = 'pink';
        }
        ctx.globalAlpha = 0.2;
        ctx.rect(start[0], start[1], size[0], size[1]);
        ctx.fillRect(start[0], start[1], size[0], size[1]);
        ctx.stroke();
        ctx.font="20px Georgia";
        ctx.textAlign="center"; 
        ctx.textBaseline = "middle";
        ctx.fillStyle = "#000000";
        ctx.fillText(emo, start[0]+40, start[1]+15);
      }
  }
};

```
