# Facial rendering source scan: suchipi/arkit-face-blendshapes

Repository: https://github.com/suchipi/arkit-face-blendshapes

Pinned source revision: 7c3c37ef69b55c45996ae1a4090035998be2ab6f

Discovery topic: blendshape

Description: A website which shows examples of the various blendshapes that can be animated using ARKit.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["public/images/eyeBlinkLeft.webp", "public/images/eyeBlinkRight.webp", "public/images/eyeLookDownLeft.webp", "public/images/eyeLookDownRight.webp", "public/images/eyeLookInLeft.webp", "public/images/eyeLookInRight.webp", "public/images/eyeLookOutLeft.webp", "public/images/eyeLookOutRight.webp", "public/images/eyeLookUpLeft.webp", "public/images/eyeLookUpRight.webp", "public/images/eyeSquintLeft.webp", "public/images/eyeSquintRight.webp", "public/images/eyeWideLeft.webp", "public/images/eyeWideRight.webp", "src/models/eyeBlinkLeft.obj", "src/models/eyeBlinkRight.obj", "src/models/eyeLookDownLeft.obj", "src/models/eyeLookDownRight.obj", "src/models/eyeLookInLeft.obj", "src/models/eyeLookInRight.obj", "src/models/eyeLookOutLeft.obj", "src/models/eyeLookOutRight.obj", "src/models/eyeLookUpLeft.obj", "src/models/eyeLookUpRight.obj", "src/models/eyeSquintLeft.obj", "src/models/eyeSquintRight.obj", "src/models/eyeWideLeft.obj", "src/models/eyeWideRight.obj"], "jaw": ["public/images/jawForward.webp", "public/images/jawLeft.webp", "public/images/jawOpen.webp", "public/images/jawRight.webp", "src/models/jawForward.obj", "src/models/jawLeft.obj", "src/models/jawOpen.obj", "src/models/jawRight.obj"], "mouth": ["public/images/mouthClose.webp", "public/images/mouthDimpleLeft.webp", "public/images/mouthDimpleRight.webp", "public/images/mouthFrownLeft.webp", "public/images/mouthFrownRight.webp", "public/images/mouthFunnel.webp", "public/images/mouthLeft.webp", "public/images/mouthLowerDownLeft.webp", "public/images/mouthLowerDownRight.webp", "public/images/mouthPressLeft.webp", "public/images/mouthPressRight.webp", "public/images/mouthPucker.webp", "public/images/mouthRight.webp", "public/images/mouthRollLower.webp", "public/images/mouthRollUpper.webp", "public/images/mouthShrugLower.webp", "public/images/mouthShrugUpper.webp", "public/images/mouthSmileLeft.webp", "public/images/mouthSmileRight.webp", "public/images/mouthStretchLeft.webp", "public/images/mouthStretchRight.webp", "public/images/mouthUpperUpLeft.webp", "public/images/mouthUpperUpRight.webp", "src/models/mouthClose.obj", "src/models/mouthDimpleLeft.obj", "src/models/mouthDimpleRight.obj", "src/models/mouthFrownLeft.obj", "src/models/mouthFrownRight.obj", "src/models/mouthFunnel.obj", "src/models/mouthLeft.obj"], "rig": ["public/images/browDownRight.webp", "public/images/browOuterUpRight.webp", "public/images/cheekSquintRight.webp", "public/images/eyeBlinkRight.webp", "public/images/eyeLookDownRight.webp", "public/images/eyeLookInRight.webp", "public/images/eyeLookOutRight.webp", "public/images/eyeLookUpRight.webp", "public/images/eyeSquintRight.webp", "public/images/eyeWideRight.webp", "public/images/jawRight.webp", "public/images/mouthDimpleRight.webp", "public/images/mouthFrownRight.webp", "public/images/mouthLowerDownRight.webp", "public/images/mouthPressRight.webp", "public/images/mouthRight.webp", "public/images/mouthSmileRight.webp", "public/images/mouthStretchRight.webp", "public/images/mouthUpperUpRight.webp", "public/images/noseSneerRight.webp", "src/models/browDownRight.obj", "src/models/browOuterUpRight.obj", "src/models/cheekSquintRight.obj", "src/models/eyeBlinkRight.obj", "src/models/eyeLookDownRight.obj", "src/models/eyeLookInRight.obj", "src/models/eyeLookOutRight.obj", "src/models/eyeLookUpRight.obj", "src/models/eyeSquintRight.obj", "src/models/eyeWideRight.obj"]}

## Source file: src/App.js

{
  "path": "src/App.js",
  "blob_sha": "3210d1f00297883c05ee9309f96adc57b760269a",
  "sha256": "d390f6808504aee961025e5e244c66d75da09cdbbf3e4ab07a2f43f074d2b3a8",
  "lines": 43,
  "bytes": 1152,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/suchipi/arkit-face-blendshapes/blob/7c3c37ef69b55c45996ae1a4090035998be2ab6f/src/App.js",
  "function_signature_candidates": []
}

```text

import React from "react";
import names from "./names";
import imageSizes from "./imageSizes";
import Preamble from "./Preamble";
import ModelViewer from "./ModelViewer";

export default function App() {
  return (
    <div style={{ maxWidth: 1967, margin: "0 auto" }}>
      <Preamble />
      <table>
        <tr>
          <th>Name</th>
          <th>Picture</th>
          <th>3D Model (Click and drag)</th>
        </tr>
        {names.map((name) => (
          <tr key={name}>
            <td>{name}</td>
            <td>
              <img
                style={{ minWidth: "300px", maxWidth: "50vw" }}
                src={`${process.env.PUBLIC_URL}/images/${name}.webp`}
                width={imageSizes[name][0]}
                height={imageSizes[name][1]}
                loading="lazy"
                decoding="async"
                alt={name}
              />
            </td>
            <td>
              <ModelViewer
                width={500}
                height={500}
                src={require("./models/" + name + ".obj")}
              />
            </td>
          </tr>
        ))}
      </table>
    </div>
  );
}


```

## Source file: src/index.js

{
  "path": "src/index.js",
  "blob_sha": "846ed2ced3efa7a12a247e81b7232392a3ff4caa",
  "sha256": "39a57c8db01174d107846aeb770073505d2de022d97aacee25187e9dfa1305c5",
  "lines": 11,
  "bytes": 219,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/suchipi/arkit-face-blendshapes/blob/7c3c37ef69b55c45996ae1a4090035998be2ab6f/src/index.js",
  "function_signature_candidates": []
}

```text

import React from "react";
import ReactDOM from "react-dom";
import "./index.css";
import App from "./App";

ReactDOM.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
  document.getElementById("root")
);


```

## Source file: src/names.js

{
  "path": "src/names.js",
  "blob_sha": "b75a98c49215aa92f46eb881ef6fba81a232289e",
  "sha256": "c8ceadba152c6c529e616dfa8ba6884324179653e07b8b09994caa89b4a8b901",
  "lines": 56,
  "bytes": 1095,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 14,
    "jaw": 4,
    "mouth": 23,
    "tongue": 1,
    "rig": 20
  },
  "url": "https://github.com/suchipi/arkit-face-blendshapes/blob/7c3c37ef69b55c45996ae1a4090035998be2ab6f/src/names.js",
  "function_signature_candidates": []
}

```text

export default [
  "browInnerUp",
  "browDownLeft",
  "browDownRight",
  "browOuterUpLeft",
  "browOuterUpRight",
  "eyeLookUpLeft",
  "eyeLookUpRight",
  "eyeLookDownLeft",
  "eyeLookDownRight",
  "eyeLookInLeft",
  "eyeLookInRight",
  "eyeLookOutLeft",
  "eyeLookOutRight",
  "eyeBlinkLeft",
  "eyeBlinkRight",
  "eyeSquintRight",
  "eyeSquintLeft",
  "eyeWideLeft",
  "eyeWideRight",
  "cheekPuff",
  "cheekSquintLeft",
  "cheekSquintRight",
  "noseSneerLeft",
  "noseSneerRight",
  "jawOpen",
  "jawForward",
  "jawLeft",
  "jawRight",
  "mouthFunnel",
  "mouthPucker",
  "mouthLeft",
  "mouthRight",
  "mouthRollUpper",
  "mouthRollLower",
  "mouthShrugUpper",
  "mouthShrugLower",
  "mouthClose",
  "mouthSmileLeft",
  "mouthSmileRight",
  "mouthFrownLeft",
  "mouthFrownRight",
  "mouthDimpleLeft",
  "mouthDimpleRight",
  "mouthUpperUpLeft",
  "mouthUpperUpRight",
  "mouthLowerDownLeft",
  "mouthLowerDownRight",
  "mouthPressLeft",
  "mouthPressRight",
  "mouthStretchLeft",
  "mouthStretchRight",

  // I couldn't find an image or model for this, so leave it out
  // "tongueOut",
];


```

## Source file: get-images.js

{
  "path": "get-images.js",
  "blob_sha": "489f64dbdb7d27457dbccd7a7252eac789c703aa",
  "sha256": "9cd9576daa45287d86be71269bfa5e9b6231d146854da2ef1679c26fecbba815",
  "lines": 111,
  "bytes": 2967,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 1,
    "face": 1
  },
  "url": "https://github.com/suchipi/arkit-face-blendshapes/blob/7c3c37ef69b55c45996ae1a4090035998be2ab6f/get-images.js",
  "function_signature_candidates": []
}

```text

const fs = require("fs");
const path = require("path");
const axios = require("axios");
const chalk = require("chalk");
const makePypress = require("pypress");

function downloadFile(fileUrl, outputLocationPath) {
  const writer = fs.createWriteStream(outputLocationPath);

  return axios({
    method: "get",
    url: fileUrl,
    responseType: "stream",
  }).then((response) => {
    return new Promise((resolve, reject) => {
      response.data.pipe(writer);
      let error = null;
      writer.on("error", (err) => {
        error = err;
        writer.close();
        reject(err);
      });
      writer.on("close", () => {
        if (!error) {
          resolve(true);
        }
        //no need to call the reject here, as it will have been called in the
        //'error' stream;
      });
    });
  });
}

async function main() {
  const py = makePypress({
    log: (message) => {
      switch (message.type) {
        case "RUN": {
          if (message.command.name === "evaluate") {
            delete message.command.args;
          }

          console.log(chalk.blue("RUN:"), message.command);
          break;
        }
        case "ERROR": {
          console.log(
            chalk.red("ERROR:"),
            message.error && message.error.stack
              ? message.error.stack
              : message.error
          );
          break;
        }
      }
    },
  });

  py.launch({ headless: false });

  py.goto(
    "https://developer.apple.com/documentation/arkit/arfaceanchor/blendshapelocation"
  );

  const { els } = await py.get(".link.has-adjacent-elements");

  const stuffToGet = [];
  for (const el of els) {
    console.log(chalk.blue(`Scraping name for element...`));
    const { lastReturnValue: name } = await py.evaluate((el) => {
      const identifierChild = el.querySelector(".identifier");
      if (!identifierChild) return "unknown";

      return identifierChild.textContent;
    }, el);
    console.log(chalk.blue(`Identified name: ${name}`));

    console.log(chalk.blue(`Scraping link url for ${name}.`));
    const { lastReturnValue: url } = await py.evaluate((el) => {
      return "https://developer.apple.com" + el.getAttribute("href");
    }, el);

    stuffToGet.push({ name, url });
  }

  for (const { name, url } of stuffToGet) {
    console.log(chalk.blue(`Navigating to page for ${name}...`));

    await py.goto(url);
    const { el } = await py.get("figure img");
    if (!el) {
      console.log(chalk.yellow(`Skipping ${name}: "No image found"`));
      continue;
    }

    console.log(chalk.blue(`Scraping img src for ${name}...`));
    const { lastReturnValue: imgurl } = await py.evaluate((el) => {
      return el.getAttribute("src");
    }, el);

    await downloadFile(imgurl, path.join(__dirname, "images", name + ".png"));
    console.log(chalk.green(`Wrote images/${name}.png`));
  }

  await py.asPromise();
}

main().catch((err) => {
  process.exitCode = 1;
  console.error(err);
});


```
