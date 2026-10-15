# Facial rendering source scan: AlessandraSalanitri/Animation-Mouth-Synch

Repository: https://github.com/AlessandraSalanitri/Animation-Mouth-Synch

Pinned source revision: d4b74ab8b8844d0e3ec3a32fabdead4bced9291a

Discovery topic: mouth animation

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"mouth": ["mouth-sync.js"]}

## Source file: mouth-sync.js

{
  "path": "mouth-sync.js",
  "blob_sha": "308414d020785ef30b819c441722ac62efdebc77",
  "sha256": "e59c332786e97ceb0fcd079a27045d69fb36d1bef1dc689ea102d0a217cfb118",
  "lines": 59,
  "bytes": 1308,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 3,
    "lip": 1
  },
  "url": "https://github.com/AlessandraSalanitri/Animation-Mouth-Synch/blob/d4b74ab8b8844d0e3ec3a32fabdead4bced9291a/mouth-sync.js",
  "function_signature_candidates": []
}

```text

const visemeGroups = {
  CHSHJ: ["ch", "sh", "j"],
  AEI: ["a", "e", "i"],
  BMP: ["b", "m", "p"],
  CDGKNSTXYZ: ["c", "d", "g", "k", "n", "s", "t", "x", "y", "z"],
  E: ["e"],
  FV: ["f", "v"],
  L: ["l"],
  O: ["o"],
  QW: ["q", "w"],
  R: ["r"],
  U: ["u"],
  TH: ["th"]
};

const mouth = document.getElementById("mouth");

function getViseme(char) {
  const c = char.toLowerCase();

  // Handle digraphs like "ch", "sh", "th"
  if (char.length === 2) {
    for (const [viseme, group] of Object.entries(visemeGroups)) {
      if (group.includes(c)) {
        return `assets/${viseme}.png`;
      }
    }
  }

  // Handle single characters
  for (const [viseme, group] of Object.entries(visemeGroups)) {
    if (group.includes(c)) {
      return `assets/${viseme}.png`;
    }
  }

  return "assets/neutral.png";
}

function startLipsync() {
  const text = document.getElementById("inputText").value;
  if (!text) return;

  let index = 0;
  const letters = text.split("");

  const interval = setInterval(() => {
    if (index >= letters.length) {
      mouth.src = "assets/neutral.png";
      clearInterval(interval);
      return;
    }

    const visemeFrame = getViseme(letters[index]);
    mouth.src = visemeFrame;

    index++;
  }, 100);
}


```
