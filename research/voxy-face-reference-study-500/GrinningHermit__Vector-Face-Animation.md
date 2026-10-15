# Facial rendering source scan: GrinningHermit/Vector-Face-Animation

Repository: https://github.com/GrinningHermit/Vector-Face-Animation

Pinned source revision: 14664873fee44dc97b603d2d4a7f803855db8ba0

Discovery topic: facial animation

Description: Displaying facial animations on Anki's Vector face display

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["face_animation.py", "face_animation/.DS_Store", "face_animation/face1.png", "face_animation/face10.png", "face_animation/face11.png", "face_animation/face12.png", "face_animation/face13.png", "face_animation/face14.png", "face_animation/face15.png", "face_animation/face16.png", "face_animation/face17.png", "face_animation/face18.png", "face_animation/face19.png", "face_animation/face2.png", "face_animation/face20.png", "face_animation/face21.png", "face_animation/face22.png", "face_animation/face23.png", "face_animation/face24.png", "face_animation/face3.png", "face_animation/face4.png", "face_animation/face5.png", "face_animation/face6.png", "face_animation/face7.png", "face_animation/face8.png", "face_animation/face9.png"]}

## Source file: face_animation.py

{
  "path": "face_animation.py",
  "blob_sha": "a018764faede2f01b49040898a204ad76e919507",
  "sha256": "341f93a8f73cd4f566e370976c0ad1d8a1e7dd743a04a33290967c8edb80962a",
  "lines": 55,
  "bytes": 1429,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 10
  },
  "url": "https://github.com/GrinningHermit/Vector-Face-Animation/blob/14664873fee44dc97b603d2d4a7f803855db8ba0/face_animation.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def face_animation():"
    }
  ]
}

```text

#!/usr/bin/env python3

"""
    Display animation on Vector's face
"""


import os
import sys
import time

try:
    from PIL import Image
except ImportError:
    sys.exit("Cannot import from PIL: Do `pip3 install --user Pillow` to install")

import anki_vector
from anki_vector.util import degrees

args = anki_vector.util.parse_command_args()
    
def face_animation():
    image_settings = []
    face_images = []
    current_directory = os.path.dirname(os.path.realpath(__file__))
    image_path = os.path.join(current_directory, "face_animation")

    for i in range(1, 24):
        image_settings.append(image_path + '/face' + str(i) + '.png')

    for image_name in image_settings:
        image = Image.open(image_name)

        pixel_bytes = anki_vector.screen.convert_image_to_screen_data(image)

        face_images.append(pixel_bytes)

    num_loops = 20
    duration_s = 0.1

    print("Press CTRL-C to quit (or wait %s seconds to complete)" % int(num_loops*duration_s*len(face_images)))

    for _ in range(num_loops):
        for i in range(0, len(face_images)):
            robot.screen.set_screen_with_image_data(face_images[i], duration_s)
            time.sleep(duration_s)

with anki_vector.robot.Robot() as robot:
    robot.behavior.set_head_angle(degrees(45.0))
    robot.behavior.set_lift_height(0.0)
    robot.conn.release_control()
    time.sleep(1)
    robot.conn.request_control()
    face_animation()



```
