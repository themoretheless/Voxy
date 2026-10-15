# Facial rendering source scan: aaryanp2904/SpeechToMouthAnimation

Repository: https://github.com/aaryanp2904/SpeechToMouthAnimation

Pinned source revision: 9d6f4df52f294b608b8a3700cfe9de1f165d29a7

Discovery topic: mouth animation

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "5be9396f0427abc42c319353be768fe595f9a2d2",
  "sha256": "07f395a7bc0c8acd454bc05b255ae96ef61f37a1349a356d17ec60ee14f3a3ac",
  "lines": 102,
  "bytes": 3242,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2
  },
  "url": "https://github.com/aaryanp2904/SpeechToMouthAnimation/blob/9d6f4df52f294b608b8a3700cfe9de1f165d29a7/main.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def get_transcription(audio_name):"
    },
    {
      "line": 13,
      "text": "def create_video(audio_name, video_name, img_dims, db_name):"
    },
    {
      "line": 20,
      "text": "def get_image(chars):"
    }
  ]
}

```text

import whisper_timestamped as w
import cv2
import numpy as np
import subprocess
from dbsetup import get_image_path_from_db
import imageio.v2 as imageio


def get_transcription(audio_name):
    return w.transcribe(w.load_model("tiny", device="cpu"), w.load_audio(audio_name), language="en")['segments'][0]['words']


def create_video(audio_name, video_name, img_dims, db_name):

    img_dict = {}

    frame_rate = 30

    # Helps us lazy load the images for the animation
    def get_image(chars):
        if chars not in img_dict.keys():
            ret = get_image_path_from_db(db_name, chars)
            if ret != -1:
                img_dict[chars] = np.asarray(imageio.imread(ret))
            else:
                raise KeyError

        return img_dict[chars]

    words = get_transcription(audio_name)

    # Calculate number of frames if animation were to last 2 seconds after last word spoken
    num_frames = int((words[-1]['end'] + 2) * frame_rate)

    out = cv2.VideoWriter("temporary.mp4", cv2.VideoWriter_fourcc(*'mp4v'), frame_rate, img_dims)

    cur_word = 0

    frame_index = 0

    while cur_word < len(words):

        # Remove all punctuation
        text = list(filter(lambda y: y.isalpha(), words[cur_word]['text']))

        start = float(words[cur_word]['start'])

        end = float(words[cur_word]['end'])

        print(f"Parsing through word '{text}' which starts at {start} and ends at {end}")

        for x in range(frame_index, int(start * frame_rate)):
            out.write(get_image("NEUTRAL"))

        frac_time = int(((end - start) * frame_rate) // len(text))

        letter_index = 0
        while letter_index < len(text):
            c = text[letter_index].lower()

            try:
                # In case the sound is 'ch', 'sh' or 'th', we try to get the image
                # including the next letter as well, the function takes care of the
                # cases where the combinations of the two letters aren't valid by
                # throwing an exception so that we just get the first letter
                im = get_image(c + text[letter_index + 1].lower())

                # If the exception wasn't thrown, we actually used the two letters
                # in which case, we must increment our letter index by 2
                num_letters_traversed = 2

            except (KeyError, IndexError):
                im = get_image(c)
                num_letters_traversed = 1

            for x in range(num_letters_traversed * frac_time):
                out.write(im)

            letter_index += num_letters_traversed

        frame_index = int(end * frame_rate) - 1

        cur_word += 1

    for x in range(frame_index, num_frames):
        out.write(get_image("NEUTRAL"))

    out.release()

    # Add the initial audio as an overlay to the animation
    cmd = f'ffmpeg -y -i {audio_name} -r 30 -i temporary.mp4 -filter:a aresample=async=1 -c:a flac -c:v copy {video_name}'
    subprocess.call(cmd, shell=True)

    # Delete video with just animation
    cmd = 'del "./temporary.mp4"'
    subprocess.call(cmd, shell=True)

aud_name = "./Fast.wav"
vid_name = "./Fast_Video.mp4"
img_dim = (512, 512)
db = "BaseAnimation.db"
create_video(aud_name, vid_name, img_dim, db)

```

## Source file: dbsetup.py

{
  "path": "dbsetup.py",
  "blob_sha": "b2f42425d68527c509a5defb56e8e79569224775",
  "sha256": "24617c1bd889fa087b93446a087e38edab8f1a0276e4deeac10fb0c4d216887c",
  "lines": 69,
  "bytes": 3127,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/aaryanp2904/SpeechToMouthAnimation/blob/9d6f4df52f294b608b8a3700cfe9de1f165d29a7/dbsetup.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def setup(file):"
    },
    {
      "line": 52,
      "text": "def get_image_path_from_db(file, chars):"
    }
  ]
}

```text

import sqlite3 as sql


def setup(file):
    conn = sql.connect(file)

    cur = conn.cursor()

    create_db = "CREATE TABLE animation (characters TEXT PRIMARY KEY, img_path TEXT)"
    cur.execute(create_db)

    entries = ["INSERT INTO animation VALUES (\"a\", \"./images/aei.jpg\");",
               "INSERT INTO animation VALUES (\"b\", \"./images/bmp.jpg\");",
               "INSERT INTO animation VALUES (\"c\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"d\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"e\", \"./images/aei.jpg\");",
               "INSERT INTO animation VALUES (\"f\", \"./images/fv.jpg\");",
               "INSERT INTO animation VALUES (\"g\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"h\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"i\", \"./images/aei.jpg\");",
               "INSERT INTO animation VALUES (\"j\", \"./images/chjsh.jpg\");",
               "INSERT INTO animation VALUES (\"k\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"l\", \"./images/l.jpg\");",
               "INSERT INTO animation VALUES (\"m\", \"./images/bmp.jpg\");",
               "INSERT INTO animation VALUES (\"n\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"o\", \"./images/o.jpg\");",
               "INSERT INTO animation VALUES (\"p\", \"./images/bmp.jpg\");",
               "INSERT INTO animation VALUES (\"q\", \"./images/wq.jpg\");",
               "INSERT INTO animation VALUES (\"r\", \"./images/r.jpg\");",
               "INSERT INTO animation VALUES (\"s\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"t\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"u\", \"./images/u.jpg\");",
               "INSERT INTO animation VALUES (\"v\", \"./images/fv.jpg\");",
               "INSERT INTO animation VALUES (\"w\", \"./images/wq.jpg\");",
               "INSERT INTO animation VALUES (\"x\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"y\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"z\", \"./images/cdothers.jpg\");",
               "INSERT INTO animation VALUES (\"ch\", \"./images/chjsh.jpg\");",
               "INSERT INTO animation VALUES (\"sh\", \"./images/chjsh.jpg\");",
               "INSERT INTO animation VALUES (\"th\", \"./images/th.jpg\");",
               "INSERT INTO animation VALUES (\"NEUTRAL\", \"./images/bmp.jpg\");"
               ]

    for entry in entries:
        cur.execute(entry)

    conn.commit()

    conn.close()


def get_image_path_from_db(file, chars):
    conn = sql.connect(file)

    cur = conn.cursor()

    select_statement = f'SELECT img_path FROM animation WHERE characters = "{chars}"'
    rows = cur.execute(select_statement).fetchall()

    if len(rows) == 0:
        return -1

    return rows[0][0]


if __name__ == "__main__":
    f = "BaseAnimation.db"
    setup(f)
    print(get_image_path_from_db(f, "ch"))


```
