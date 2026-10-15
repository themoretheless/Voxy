# Facial rendering source scan: lukerbs/pytoon

Repository: https://github.com/lukerbs/pytoon

Pinned source revision: f12dca508094cef7809647735bb4569f2a1594ce

Discovery topic: mouth animation

Description: PyToon is a Python based animation library for automatically animating a cartoon character's mouth movements and bodily expressions to sync with an audio recording of someone talking. PyToon uses machine learning based audio analysis techniques to automatically generate lip-synced character animations (see "Example Output Video" in README).

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"lip": ["pytoon/lipsync.py"]}

## Source file: pytoon/lipsync.py

{
  "path": "pytoon/lipsync.py",
  "blob_sha": "99575160edc8e63501c213148bc706e6c152a924",
  "sha256": "e7fc81ec6c32157408feddc30a672a6dff2c7ec3dcae284b4642b35666d12c98",
  "lines": 226,
  "bytes": 7842,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 3
  },
  "url": "https://github.com/lukerbs/pytoon/blob/f12dca508094cef7809647735bb4569f2a1594ce/pytoon/lipsync.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def viseme_sequencer(audio_file: str, transcript: str = None, fps: int = 48) -> list[WordViseme]:"
    },
    {
      "line": 101,
      "text": "def generate_viseme_frames(sequence: list, total_frames: int) -> list:"
    },
    {
      "line": 136,
      "text": "def upsample(sequence, length):"
    },
    {
      "line": 151,
      "text": "def phoneme_no_stress(phoneme: str) -> str:"
    },
    {
      "line": 166,
      "text": "def phoneme_to_viseme(phoneme: str) -> list[str]:"
    },
    {
      "line": 181,
      "text": "def get_silent_viseme(current_viseme, next_viseme, total_duration, target_frames):"
    },
    {
      "line": 211,
      "text": "def ending_silence(duration: float, fps: int, start_t: int):"
    }
  ]
}

```text

from forcealign import ForceAlign
from .util import read_json
from dataclasses import dataclass
from datetime import datetime
from typing import Union
import random
import re

# Viseme image for silence (i.e. closed mouth, not speaking)
SILENT_VISEME = "9.png"
SILENT_PHONEME = "PAUSE"
# ARPAbet phonemes to simplified phonemes mapping
PHONEMES = read_json("phonemes.json")
# Simplified phonemes to viseme-sequence mapping
VISEMES = read_json("visemes.json")


@dataclass
class WordViseme:
    word: Union[str, None]  # Word associated with viseme
    visemes: list[str]  # List of mouth shape images for viseme
    phonemes: list[str]  # The phoneme associated with the viseme
    time_start: datetime  # The time the word starts (seconds)
    time_end: datetime  # The time the word ends (seconds)
    duration: float  # total word duration from start to end (seconds)
    total_frames: int  # total number of frames in video for word
    breath: bool


def viseme_sequencer(audio_file: str, transcript: str = None, fps: int = 48) -> list[WordViseme]:
    """Converts and audio / txt file to force aligned viseme sequence

    Args:
        audio_file (str): Path to audio file of a person speaking english (.wav or .mp3)
        transcript (str): (optional) Trascript string of audio recording
            - If not transcript is provided, it will automatically detect with speech to text

    Returns:
        list[WordViseme]: A list of force aligned WordViseme objects
    """
    ENDING_SILENCE_SECONDS = 2.5
    # Provide path to audio_file and corresponding txt_file with audio transcript
    aligner = ForceAlign(audio_file=audio_file, transcript=transcript)

    # Runs forced alignment algorithm and returns alignment results
    words = aligner.inference()

    first_word = words[0]
    print(f"Time Start: {first_word.time_start}")
    last_word = words[-1]
    total_duration = last_word.time_end
    target_frames = int(total_duration * fps)
    print(f"Target Duration: {total_duration + ENDING_SILENCE_SECONDS}")
    print(f"Target Frames: {target_frames + int(ENDING_SILENCE_SECONDS *fps)}")

    viseme_sequence = []
    for word in words:
        phonemes = [phoneme_no_stress(phoneme) for phoneme in word.phonemes]
        images = [phoneme_to_viseme(phoneme=phoneme) for phoneme in phonemes]
        duration = word.time_end - word.time_start
        total_frames = int((duration / total_duration) * target_frames)

        remainder = ((duration / total_duration) * target_frames) % 1
        if random.choices([True, False], [remainder, (1 - remainder)])[0]:
            total_frames += 1

        # If viseme is more than one frame long
        visemes = generate_viseme_frames(sequence=images, total_frames=total_frames)
        total_frames = len(visemes)

        viseme_sequence.append(
            WordViseme(
                word=word.word,
                visemes=visemes,
                phonemes=phonemes,
                time_start=word.time_start,
                time_end=word.time_end,
                duration=duration,
                total_frames=total_frames,
                breath=word.breath,
            )
        )

    # Add silence viseme (closed mouth) between speaking visemes
    finished_sequence = []
    for i, _ in enumerate(viseme_sequence):
        finished_sequence.append(viseme_sequence[i])
        if i == len(viseme_sequence) - 1:
            break

        silent_viseme = get_silent_viseme(viseme_sequence[i], viseme_sequence[i + 1], total_duration, target_frames)
        if silent_viseme:
            finished_sequence.append(silent_viseme)

    finshed_sequence = upsample(finished_sequence, length=target_frames)
    silence = ending_silence(duration=ENDING_SILENCE_SECONDS, fps=fps, start_t=total_duration + 0.001)
    finished_sequence.append(silence)
    return finished_sequence


def generate_viseme_frames(sequence: list, total_frames: int) -> list:
    """Generates the complete viseme frame sequence for word viseme

    Args:
        sequence (list): List of visemes in word
        total_frames (int): Total frames allocated to full word

    Returns:
        list: Completed viseme video sequence of images for word
    """
    frames_per_subviseme = total_frames // len(sequence)
    remainder_end = total_frames % len(sequence)
    if frames_per_subviseme == 0:
        if random.choice([True, False]):
            frames_per_subviseme = 1
        else:
            return []

    viseme_frames = []
    for i, _ in enumerate(sequence):
        if len(sequence[i]) > frames_per_subviseme:
            viseme_frames.extend(sequence[i][:frames_per_subviseme])
        elif len(sequence[i]) < frames_per_subviseme:
            seq = upsample(sequence[i], frames_per_subviseme)
            viseme_frames.extend(seq)
        else:
            viseme_frames.extend(sequence[i])

    # Upsample the frames to target length
    if len(viseme_frames) < total_frames:
        viseme_frames = upsample(viseme_frames, total_frames)

    return viseme_frames


def upsample(sequence, length):
    repetitions = length // len(sequence)
    remainder = length % len(sequence)
    upsampled = [elem for elem in sequence for _ in range(repetitions)]

    final_upsampled = []
    for i, _ in enumerate(upsampled):
        if i > (len(upsampled) - remainder - 1):
            final_upsampled.extend([upsampled[i], upsampled[i]])
        else:
            final_upsampled.append(upsampled[i])

    return final_upsampled


def phoneme_no_stress(phoneme: str) -> str:
    """Removes stress symbol from an ARPAbet phoneme

    Args:
        phoneme (str): An ARPAbet phoneme

    Returns:
        str: ARPAbet phoneme without stress symbol
    """
    if phoneme[-1].isdigit():
        return phoneme[:-1]
    else:
        return phoneme


def phoneme_to_viseme(phoneme: str) -> list[str]:
    """Converts a phoneme to a viseme image sequence

    Args:
        phoneme (str): An ARPAbet phoneme

    Returns:
        str: A list of images files for the viseme
    """
    phoneme = phoneme_no_stress(phoneme=phoneme)
    simplified_phone = PHONEMES[phoneme]
    viseme = VISEMES[simplified_phone]
    return viseme


def get_silent_viseme(current_viseme, next_viseme, total_duration, target_frames):
    # The time the silent viseme should start after previous viseme (i.e. the next frame)
    delta = 0.00000000000000000001

    # Get start time, end time, and total duration of silence
    silence_start = current_viseme.time_end + delta
    silence_end = next_viseme.time_start - delta
    duration = silence_end - silence_start

    # Get number of frames for silence segment (24 frames per second of silence)
    total_frames = int((duration / total_duration) * target_frames)
    remainder = ((duration / total_duration) * target_frames) % 1
    if random.choices([True, False], [remainder, (1 - remainder)])[0]:
        total_frames += 1

    # Create frames for silence
    silent_visemes = [SILENT_VISEME for _ in range(total_frames)]
    phonemes = [SILENT_PHONEME for _ in range(total_frames)]
    return WordViseme(
        word=None,
        visemes=silent_visemes,
        phonemes=phonemes,
        time_start=silence_start,
        time_end=silence_end,
        duration=duration,
        total_frames=total_frames,
        breath=False,
    )


def ending_silence(duration: float, fps: int, start_t: int):
    total_frames = int(duration * fps)

    # Create frames for silence
    silent_visemes = [SILENT_VISEME for _ in range(total_frames)]
    phonemes = [SILENT_PHONEME for _ in range(total_frames)]
    return WordViseme(
        word=None,
        visemes=silent_visemes,
        phonemes=phonemes,
        time_start=start_t,
        time_end=start_t + duration,
        duration=duration,
        total_frames=total_frames,
        breath=False,
    )


```

## Source file: setup.py

{
  "path": "setup.py",
  "blob_sha": "5898acb96348b649885d9bb50d46759ef05911ef",
  "sha256": "34da9b9487a53c202cabfb847158ef2d5a8ac74af915c3bf7cee4a307f8e1ac0",
  "lines": 41,
  "bytes": 1137,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 2
  },
  "url": "https://github.com/lukerbs/pytoon/blob/f12dca508094cef7809647735bb4569f2a1594ce/setup.py",
  "function_signature_candidates": []
}

```text

from setuptools import setup, find_packages

# read the contents of your README file
from pathlib import Path

this_directory = Path(__file__).parent
long_description = (this_directory / "README.md").read_text()

setup(
    name="pytoon",
    version="1.5.0",
    packages=find_packages(),
    install_requires=[
        "forcealign>=1.1.9",
        "moviepy",
        "opencv-python",
        "scipy",
        "numpy",
        "pillow",
    ],
    author="Luke Kerbs",
    description="A Python library for lip-syncing cartoons to voice recordings.",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/lukerbs/apple",
    keywords=[
        "animation",
        "forced alignment",
        "lip-sync",
        "audio forced alignment",
        "phoneme",
        "generate subtitles",
    ],
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    package_data={"pytoon": ["assets/*", "assets/**/*", "assets/**/**/*"]},
    include_package_data=True,
)


```

## Source file: pytoon/util.py

{
  "path": "pytoon/util.py",
  "blob_sha": "b45b43c5e523770cebe41ebd22129b17b1ad4bce",
  "sha256": "3cabb43a49c0a4419662a3909dc7c7d420ca770582dfdbf41e9b0e9a01cee84e",
  "lines": 103,
  "bytes": 3457,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 4
  },
  "url": "https://github.com/lukerbs/pytoon/blob/f12dca508094cef7809647735bb4569f2a1594ce/pytoon/util.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def read_json(file: str) -> dict:"
    },
    {
      "line": 23,
      "text": "def write_json(data: dict, file: str) -> None:"
    },
    {
      "line": 35,
      "text": "def resample_audio("
    },
    {
      "line": 73,
      "text": "def add_outline(image_path, outline_color=(255, 255, 255), outline_width=5):"
    }
  ]
}

```text

import json
import os
from scipy.io import wavfile
from scipy.signal import resample
import numpy as np


def read_json(file: str) -> dict:
    """Reads a json file to dictionary

    Args:
        path (str): Path to the json file.

    Returns:
        dict: Python dictionary with json data.
    """
    path = f"{os.path.dirname(__file__)}/assets/{file}"
    with open(path, "r") as file:
        data = json.load(file)
    return data


def write_json(data: dict, file: str) -> None:
    """Writes a Python dictionary to a json file.

    Args:
        data (dict): Python dictionary with data.
        path (str): The path to the output json file.
    """
    path = f"{os.path.dirname(__file__)}/assets/{file}"
    with open(path, "w") as file:
        json.dump(data, file, indent=4)


def resample_audio(
    audio_file: str,
    output_file: str = None,
    target_sr: int = 48000,
    padding: bool = True,
):
    """Resamples audio to a target sample rate

    Args:
        audio_file (str): Path to audio file to be resampled
        output_file (str, optional): Path to save new file. Defaults to None.
        target_sr (int, optional): Target sample rate. Defaults to 48000.
        padding (bool, optional): Extends duration of audio to nearest integer number of seconds.

    Returns:
        tuple: Returns numpy array of resampled audio and the new audio sample rate
    """
    original_sample_rate, audio_data = wavfile.read(audio_file)
    resample_ratio = target_sr / original_sample_rate
    total_samples = int(len(audio_data) * resample_ratio)

    # Resamples the audio to have only 'total_samples' number of samples
    resampled_audio = resample(x=audio_data, num=total_samples)
    resampled_audio = resampled_audio.astype(np.int16)

    # Adds silence to end of audio so num samples is divisible by sample rate
    zero_pad = target_sr - (len(resampled_audio) % target_sr)
    if padding and int(zero_pad) not in [0, target_sr]:
        silence = np.zeros(zero_pad, dtype=np.int16)
        resampled_audio = np.concatenate((resampled_audio, silence))

    if output_file:
        wavfile.write(output_file, target_sr, resampled_audio)

    padding_secs = round((padding / target_sr), 2)
    return resampled_audio, target_sr


def add_outline(image_path, outline_color=(255, 255, 255), outline_width=5):
    """
    Adds a white outline around a .png image that has a transparent background
    """
    # Open the image
    img = Image.open(image_path)

    # Convert image to RGBA if not already
    img = img.convert("RGBA")

    # Create a new image with the same size as the original image
    outline_img = Image.new("RGBA", img.size, (255, 255, 255, 0))

    # Get pixel data
    pixels = img.load()
    outline_pixels = outline_img.load()

    # Add white outline
    for x in range(img.width):
        for y in range(img.height):
            r, g, b, a = pixels[x, y]
            if a > 0:  # Check if pixel is not transparent
                for dx in range(-outline_width, outline_width + 1):
                    for dy in range(-outline_width, outline_width + 1):
                        # Avoid out-of-bounds access
                        if 0 <= x + dx < img.width and 0 <= y + dy < img.height:
                            outline_pixels[x + dx, y + dy] = outline_color

    # Overlay the original image on top of the outline image
    outline_img.paste(img, (0, 0), mask=img)
    outline_img.save(image_path)


```

## Source file: pytoon/__init__.py

{
  "path": "pytoon/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/lukerbs/pytoon/blob/f12dca508094cef7809647735bb4569f2a1594ce/pytoon/__init__.py",
  "function_signature_candidates": []
}

```text



```
