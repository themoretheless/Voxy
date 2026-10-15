# Facial rendering source scan: krishnaveniammi/GenAI-Youth-Mental-Wellness

Repository: https://github.com/krishnaveniammi/GenAI-Youth-Mental-Wellness

Pinned source revision: fc7d904d338e30fe900c1194f2dab5818edfa379

Discovery topic: lip sync 3D

Description: Astra is an AI-powered 3D avatar companion designed to support youth mental wellness. It offers empathetic conversations, positive guidance, and a safe space for self-expression. With lifelike lip-sync and gestures, Astra creates human-like interaction, helping young people feel heard, valued, and supported.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: app.py

{
  "path": "app.py",
  "blob_sha": "2e09956ccb25ba109d07565df61d00a5f6b10af4",
  "sha256": "215248681b1d88736397da97cc7450177ae90f89230f3a8b28fa5040d08fd652",
  "lines": 133,
  "bytes": 4357,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/krishnaveniammi/GenAI-Youth-Mental-Wellness/blob/fc7d904d338e30fe900c1194f2dab5818edfa379/app.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def clean_for_tts(text: str):"
    },
    {
      "line": 30,
      "text": "def get_playlist_themes(user_mood: str):"
    },
    {
      "line": 45,
      "text": "def get_youtube_resource(query):"
    },
    {
      "line": 77,
      "text": "def add_playlist_links(reply: str):"
    },
    {
      "line": 97,
      "text": "def synthesize_speech(text, lang=\"en\", filename=None):"
    },
    {
      "line": 110,
      "text": "def index():"
    },
    {
      "line": 115,
      "text": "def chat():"
    }
  ]
}

```text

from flask import Flask, render_template, request, jsonify
import os, time, re
import google.generativeai as genai  # type: ignore
from gtts import gTTS  # type: ignore
from googleapiclient.discovery import build  # type: ignore

app = Flask(__name__, static_folder="static")

# ===== Configure Gemini + YouTube =====
GEMINI_KEY = os.environ.get("GEMINI_KEY")
YOUTUBE_KEY = os.environ.get("YOUTUBE_KEY")

if not GEMINI_KEY or not YOUTUBE_KEY:
    raise RuntimeError("❌ Missing API keys. Set GEMINI_KEY and YOUTUBE_KEY as environment variables.")

genai.configure(api_key=GEMINI_KEY)
model = genai.GenerativeModel("gemini-1.5-flash")
youtube = build("youtube", "v3", developerKey=YOUTUBE_KEY)


# ===== Utility Functions =====
def clean_for_tts(text: str):
    """Remove emojis, links & special chars before TTS."""
    text = re.sub(r"http\S+", "", text)               # remove URLs
    text = re.sub(r"[^A-Za-z0-9\s,.!?]", "", text)    # remove emojis & symbols
    text = re.sub(r"\s+", " ", text)                  # normalize spaces
    return text.strip()


def get_playlist_themes(user_mood: str):
    prompt = f"""
    The user feels: {user_mood}

    🎯 Your reply must always include:
    - 2 short healing or motivational tips
    - EXACTLY 2 Telugu playlist themes
    - EXACTLY 2 Hindi playlist themes
    - EXACTLY 2 English playlist themes
    - Format playlists as list items starting with "-"
    """
    response = model.generate_content(prompt)
    return response.text.strip()


def get_youtube_resource(query):
    """Search YouTube for a playlist (fallback: video)."""
    try:
        search_response = youtube.search().list(
            q=query,
            part="snippet",
            maxResults=1,
            type="playlist"
        ).execute()

        if search_response.get("items"):
            playlist_id = search_response["items"][0]["id"]["playlistId"]
            return f"https://www.youtube.com/playlist?list={playlist_id}"

        # fallback → video
        search_response = youtube.search().list(
            q=query,
            part="snippet",
            maxResults=1,
            type="video"
        ).execute()

        if search_response.get("items"):
            video_id = search_response["items"][0]["id"]["videoId"]
            return f"https://www.youtube.com/watch?v={video_id}"

    except Exception as e:
        return f"(link not available: {e})"

    return "No result found"


def add_playlist_links(reply: str):
    """Detect lines starting with '-' and append YouTube links."""
    lines = reply.splitlines()
    new_lines = []
    for line in lines:
        raw = line.strip()
        if raw.startswith("-"):  # playlist/theme line
            playlist_name = re.sub(r"^[-–•]+\s*", "", raw)  # remove "- " etc
            playlist_name = re.sub(r"[^\w\s]", "", playlist_name)  # strip symbols

            if playlist_name.strip():
                yt_link = get_youtube_resource(playlist_name.strip())
                new_lines.append(f"- {playlist_name.strip()} 🎵 {yt_link}")
            else:
                new_lines.append(line)
        else:
            new_lines.append(line)
    return "\n".join(new_lines)


def synthesize_speech(text, lang="en", filename=None):
    """Generate TTS audio and save in static folder."""
    clean_text = clean_for_tts(text)   # ✅ Clean before sending to gTTS
    if filename is None:
        filename = f"reply_{int(time.time())}.mp3"
    path = os.path.join("static", filename)
    tts = gTTS(text=clean_text, lang=lang)
    tts.save(path)
    return "/" + path


# ===== Routes =====
@app.route("/")
def index():
    return render_template("index.html")


@app.route("/chat", methods=["POST"])
def chat():
    data = request.get_json()
    user_input = data.get("message", "")

    reply = get_playlist_themes(user_input)

    # ✅ Add YouTube links
    reply_with_links = add_playlist_links(reply)

    # ✅ Generate audio only AFTER cleaning
    audio_url = synthesize_speech(reply_with_links, lang="en")

    return jsonify({"reply": reply_with_links, "audio": audio_url})


if __name__ == "__main__":
    port = int(os.environ.get("PORT", 5000))
    app.run(host="0.0.0.0", port=port, debug=True)



```

## Source file: config.py

{
  "path": "config.py",
  "blob_sha": "7c89520c6287dcf8207d17b72199a625958278d5",
  "sha256": "50f5c27d7d525e84a6c46b369e4209af6079367fe82c63e1aa14b17974f7c839",
  "lines": 2,
  "bytes": 111,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/krishnaveniammi/GenAI-Youth-Mental-Wellness/blob/fc7d904d338e30fe900c1194f2dab5818edfa379/config.py",
  "function_signature_candidates": []
}

```text

GEMINI_KEY = "AIzaSyAzReZjHEQlY6TqCQx9VwVAN1gyQgo_Qig"
YOUTUBE_KEY = "AIzaSyA16sE806yXpnSg08q03z539xsQYCzutNI"

```

## Source file: static/avatar.js

{
  "path": "static/avatar.js",
  "blob_sha": "f188c26133f3a35dccb8dba2602aa0511f136a24",
  "sha256": "18e227822e6b31a22d1e7f7f5d2a0b537a435471807ff28136a532987cce7229",
  "lines": 195,
  "bytes": 5378,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 7,
    "lip": 3
  },
  "url": "https://github.com/krishnaveniammi/GenAI-Youth-Mental-Wellness/blob/fc7d904d338e30fe900c1194f2dab5818edfa379/static/avatar.js",
  "function_signature_candidates": []
}

```text

// =================== Avatar.js ===================

// Quick nod expression
function triggerAvatarExpression() {
  if (!window.currentScene) return;
  const scene = window.currentScene;

  const head = scene.meshes.find(m => m.name.toLowerCase().includes("head"));
  if (head) {
    const anim = new BABYLON.Animation(
      "nod",
      "rotation.x",
      30,
      BABYLON.Animation.ANIMATIONTYPE_FLOAT,
      BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
    );

    const keys = [];
    keys.push({ frame: 0, value: 0 });
    keys.push({ frame: 15, value: 0.1 });
    keys.push({ frame: 30, value: 0 });
    anim.setKeys(keys);

    head.animations = [anim];
    scene.beginAnimation(head, 0, 30, true);
  }
}

// Gentle head tilt idle movement
function animateHead(scene) {
  const skeleton = scene.skeletons[0];
  if (!skeleton) return;

  const headBone = skeleton.bones.find(b => b.name.toLowerCase().includes("head"));
  if (!headBone) {
    console.warn("No head bone found.");
    return;
  }

  const anim = new BABYLON.Animation(
    "headIdle",
    "rotation.y",
    30,
    BABYLON.Animation.ANIMATIONTYPE_FLOAT,
    BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
  );

  const keys = [];
  keys.push({ frame: 0, value: 0 });
  keys.push({ frame: 15, value: 0.1 });
  keys.push({ frame: 30, value: -0.1 });
  keys.push({ frame: 45, value: 0 });
  anim.setKeys(keys);

  headBone.animations = [anim];
  scene.beginAnimation(headBone, 0, 45, true, 0.8);
}

// Hand wave/gesture
function animateHands(scene) {
  const skeleton = scene.skeletons[0];
  if (!skeleton) return;

  const leftHand = skeleton.bones.find(b => b.name.toLowerCase().includes("lefthand"));
  const rightHand = skeleton.bones.find(b => b.name.toLowerCase().includes("righthand"));

  [leftHand, rightHand].forEach(handBone => {
    if (!handBone) return;

    const anim = new BABYLON.Animation(
      "handWave",
      "rotation.z",
      30,
      BABYLON.Animation.ANIMATIONTYPE_FLOAT,
      BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
    );

    const keys = [];
    keys.push({ frame: 0, value: 0 });
    keys.push({ frame: 20, value: 0.2 });
    keys.push({ frame: 40, value: -0.2 });
    keys.push({ frame: 60, value: 0 });
    anim.setKeys(keys);

    handBone.animations = [anim];
    scene.beginAnimation(handBone, 0, 60, true, 1.0);
  });
}

// =============== Lip Sync (Approximate Phoneme-based) ===============
const phonemeVisemeMap = {
  A: "viseme_AA",
  E: "viseme_E",
  I: "viseme_I",
  O: "viseme_O",
  U: "viseme_U",
  M: "viseme_M",
  F: "viseme_F",
  S: "viseme_S"
};

function textToPhonemes(text) {
  text = text.toUpperCase();
  return text.split("").map(ch => {
    if ("AEIOU".includes(ch)) return ch;
    if ("MFV".includes(ch)) return "M";
    if ("SZ".includes(ch)) return "S";
    return "A";
  });
}

function triggerAvatarLipSync(scene, audioElement, text) {
  let morphTargetManager = null;
  scene.meshes.forEach(mesh => {
    if (mesh.morphTargetManager) morphTargetManager = mesh.morphTargetManager;
  });

  if (!morphTargetManager) return;

  const phonemes = textToPhonemes(text);
  const phonemeDuration = 120; // ms per phoneme fallback

  let index = 0;
  const interval = setInterval(() => {
    if (audioElement.paused || audioElement.ended || index >= phonemes.length) {
      for (let i = 0; i < morphTargetManager.numTargets; i++) {
        morphTargetManager.getTarget(i).influence = 0;
      }
      clearInterval(interval);
      return;
    }

    const phoneme = phonemes[index];
    const visemeName = phonemeVisemeMap[phoneme];

    for (let i = 0; i < morphTargetManager.numTargets; i++) {
      morphTargetManager.getTarget(i).influence = 0;
    }

    for (let i = 0; i < morphTargetManager.numTargets; i++) {
      const target = morphTargetManager.getTarget(i);
      if (target.name.includes(visemeName)) {
        target.influence = 1.0;
      }
    }

    index++;
  }, phonemeDuration);
}


    

// =============== Combined Speaking Action ===============
function triggerAvatarSpeaking(scene, audioElement, text) {
  triggerAvatarExpression();
  animateHead(scene);
  animateHands(scene);
  triggerAvatarLipSync(scene, audioElement, text);
}

// =============== Scene Setup ===============
window.addEventListener('DOMContentLoaded', function () {
  const canvas = document.getElementById("renderCanvas");
  const engine = new BABYLON.Engine(canvas, true);

  const createScene = async function () {
    const scene = new BABYLON.Scene(engine);

    // Light
    new BABYLON.HemisphericLight("light", new BABYLON.Vector3(1, 1, 0), scene);

    // Fixed zoom camera
    const camera = new BABYLON.UniversalCamera("Camera", new BABYLON.Vector3(0, 1.6, 1.2), scene);
    camera.setTarget(new BABYLON.Vector3(0, 1.5, 0));
    camera.attachControl(canvas, false);

    // Load Avatar
    await BABYLON.SceneLoader.AppendAsync(
      "https://models.readyplayer.me/68b66d0788d9bef7f4b929b3.glb",
      "",
      scene
    );

    window.currentScene = scene;
    return scene;
  };

  createScene().then(function (scene) {
    engine.runRenderLoop(() => scene.render());
    window.addEventListener("resize", () => engine.resize());
  });
});




```

## Source file: static/script.js

{
  "path": "static/script.js",
  "blob_sha": "3ae9151ef2277f2e7512659826f197b676022cc8",
  "sha256": "bbd1d105be0ce24d5ef89d16d5b7bf67f44da9d923b0c0435382e6fc55829d6b",
  "lines": 82,
  "bytes": 2383,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2,
    "lip": 1
  },
  "url": "https://github.com/krishnaveniammi/GenAI-Youth-Mental-Wellness/blob/fc7d904d338e30fe900c1194f2dab5818edfa379/static/script.js",
  "function_signature_candidates": []
}

```text

document.addEventListener("DOMContentLoaded", () => {
  const userInput = document.getElementById("userInput");
  const sendButton = document.getElementById("sendButton");
  const greetingBox = document.getElementById("greetingBox");
  const responseBox = document.getElementById("responseBox");

  let typingMsg = null; // for "..." effect

  function addMessage(sender, text) {
    const msg = document.createElement("div");
    msg.classList.add("message", sender);
    msg.textContent = text;
    responseBox.appendChild(msg);
    responseBox.scrollTop = 0; // ✅ always start at top
  }

  // ✅ Show typing animation
  function showTyping() {
    typingMsg = document.createElement("div");
    typingMsg.classList.add("message", "astra", "typing");
    typingMsg.innerHTML = "<span>.</span><span>.</span><span>.</span>";
    responseBox.appendChild(typingMsg);
    responseBox.scrollTop = 0;
  }

  // ✅ Remove typing animation
  function removeTyping() {
    if (typingMsg) {
      typingMsg.remove();
      typingMsg = null;
    }
  }

  async function handleSend() {
    const text = userInput.value.trim();
    if (!text) return;

    if (!responseBox.classList.contains("active")) {
      greetingBox.classList.add("hidden");
      responseBox.classList.remove("hidden");
      responseBox.classList.add("active");
    }

    addMessage("user", text);
    userInput.value = "";

    // ✅ show "..." while waiting
    showTyping();

    // Call backend
    const res = await fetch("/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ message: text }),
    });
    const data = await res.json();

    // Create audio element
    const audio = new Audio(data.audio);

    // Trigger avatar lipsync
    if (window.currentScene) {
      triggerAvatarSpeaking(window.currentScene, audio, data.reply);
    }

    // Show text only once audio starts
    audio.addEventListener("play", () => {
      removeTyping(); // ✅ remove dots
      addMessage("astra", data.reply);
    });

    audio.play(); // start audio
  }

  sendButton.addEventListener("click", handleSend);
  userInput.addEventListener("keypress", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  });
});


```
