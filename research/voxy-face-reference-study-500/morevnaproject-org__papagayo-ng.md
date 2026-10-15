# Facial rendering source scan: morevnaproject-org/papagayo-ng

Repository: https://github.com/morevnaproject-org/papagayo-ng

Pinned source revision: 17fa36a619817362f79726f85ef520489c01fe6a

Discovery topic: mouth animation

Description: Papagayo is a lip-syncing program designed to help you line up phonemes (mouth shapes) with the actual recorded sound of actors speaking. Papagayo makes it easy to lip sync animated characters by making the process very simple - just type in the words being spoken (or copy/paste them from the animation's script), then drag the words on top of the sound's waveform until they line up with the proper sounds. 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["rsrc/icons/eye-2-line.png"], "mouth": ["MouthViewQT.py", "rsrc/mouths/1 - Mouth 1/AI.png", "rsrc/mouths/1 - Mouth 1/E.png", "rsrc/mouths/1 - Mouth 1/FV.png", "rsrc/mouths/1 - Mouth 1/L.png", "rsrc/mouths/1 - Mouth 1/MBP.png", "rsrc/mouths/1 - Mouth 1/O.png", "rsrc/mouths/1 - Mouth 1/U.png", "rsrc/mouths/1 - Mouth 1/WQ.png", "rsrc/mouths/1 - Mouth 1/etc.png", "rsrc/mouths/1 - Mouth 1/rest.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/AA.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/AE.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/AH.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/AO.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/AW.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/AY.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/B.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/CH.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/D.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/DH.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/EH.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/ER.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/EY.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/F.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/G.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/H.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/HH.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/IH.png", "rsrc/mouths/10 - Hunanbean @ 66% (CMU39)/IY.png"], "rig": ["rsrc/icons/arrow-left-right-line.png"], "lip": ["LipsyncDoc.py", "LipsyncFrameQT.py", "rsrc/icons/clipboard-line.png", "translations/LipsyncFrame.pot"]}

## Source file: LipsyncDoc.py

{
  "path": "LipsyncDoc.py",
  "blob_sha": "409bb903539d507affcbc332a08f2f6823fc5835",
  "sha256": "401387a2e7e30d222abd9f8f1f3347558134653e238d7cf8f3174363a2b7d8aa",
  "lines": 1123,
  "bytes": 53516,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 6,
    "mouth": 7,
    "rig": 18,
    "lip": 32
  },
  "url": "https://github.com/morevnaproject-org/papagayo-ng/blob/17fa36a619817362f79726f85ef520489c01fe6a/LipsyncDoc.py",
  "function_signature_candidates": [
    {
      "line": 77,
      "text": "def __init__(self):"
    },
    {
      "line": 87,
      "text": "def load_dictionary(self, path):"
    },
    {
      "line": 125,
      "text": "def load_language(self, language_config, force=False):"
    },
    {
      "line": 136,
      "text": "def language_details(self, dirname, names):"
    },
    {
      "line": 160,
      "text": "def init_languages(self):"
    },
    {
      "line": 173,
      "text": "def __init__(self, parent=None, children=None, object_type=\"voice\", text=\"\", start_frame=0, end_frame=0, name=\"\","
    },
    {
      "line": 196,
      "text": "def get_min_size(self):"
    },
    {
      "line": 207,
      "text": "def get_left_sibling(self):"
    },
    {
      "line": 210,
      "text": "def get_right_sibling(self):"
    },
    {
      "line": 213,
      "text": "def get_parent(self):"
    },
    {
      "line": 219,
      "text": "def get_frame_size(self):"
    },
    {
      "line": 225,
      "text": "def has_shrink_room(self):"
    },
    {
      "line": 234,
      "text": "def has_left_sibling(self):"
    },
    {
      "line": 241,
      "text": "def has_right_sibling(self):"
    },
    {
      "line": 248,
      "text": "def get_left_max(self):"
    },
    {
      "line": 259,
      "text": "def get_right_max(self):"
    },
    {
      "line": 274,
      "text": "def get_phoneme_at_frame(self, frame):"
    },
    {
      "line": 286,
      "text": "def reposition_to_left(self):"
    },
    {
      "line": 304,
      "text": "def reposition_descendants(self, did_resize=False, x_diff=0):"
    },
    {
      "line": 314,
      "text": "def reposition_descendants2(self, did_resize=False, x_diff=0):"
    },
    {
      "line": 369,
      "text": "def open(self, in_file):"
    },
    {
      "line": 396,
      "text": "def save(self, out_file):"
    },
    {
      "line": 412,
      "text": "def run_breakdown(self, frame_duration, parent_window, language, languagemanager, phonemeset):"
    },
    {
      "line": 537,
      "text": "def export(self, path):"
    },
    {
      "line": 562,
      "text": "def export_images(self, path, currentmouth):"
    },
    {
      "line": 591,
      "text": "def export_alelo(self, path, language, languagemanager):"
    },
    {
      "line": 634,
      "text": "def export_json(self, path):"
    },
    {
      "line": 670,
      "text": "def __str__(self):"
    },
    {
      "line": 685,
      "text": "def __repr__(self):"
    },
    {
      "line": 690,
      "text": "def __init__(self, langman: LanguageManager, parent):"
    },
    {
      "line": 708,
      "text": "def dirty(self):"
    },
    {
      "line": 712,
      "text": "def dirty(self, value):"
    },
    {
      "line": 715,
      "text": "def __del__(self):"
    },
    {
      "line": 720,
      "text": "def open_json(self, path):"
    },
    {
      "line": 761,
      "text": "def open(self, path):"
    },
    {
      "line": 789,
      "text": "def open_audio(self, path):"
    },
    {
      "line": 811,
      "text": "def open_from_dict(self, json_data):"
    },
    {
      "line": 851,
      "text": "def copy_to_dict(self, saved_sound_path=\"\"):"
    },
    {
      "line": 893,
      "text": "def save2(self, path):"
    },
    {
      "line": 906,
      "text": "def save(self, path):"
    },
    {
      "line": 925,
      "text": "def convert_to_phonemeset_old(self):"
    },
    {
      "line": 949,
      "text": "def convert_to_phonemeset(self):"
    },
    {
      "line": 990,
      "text": "def auto_recognize_phoneme(self, manual_invoke=False):"
    },
    {
      "line": 1080,
      "text": "def __str__(self):"
    },
    {
      "line": 1084,
      "text": "def __repr__(self):"
    },
    {
      "line": 1091,
      "text": "def __init__(self):"
    },
    {
      "line": 1107,
      "text": "def load(self, name=''):"
    }
  ]
}

```text

# Papagayo-NG, a lip-sync tool for use with several different animation suites
# Original Copyright (C) 2005 Mike Clifton
# Contact information at http://www.lostmarble.com
#
# This program is free software; you can redistribute it and/or
# modify it under the terms of the GNU General Public License
# as published by the Free Software Foundation; either version 2
# of the License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.

import math
import shutil
import sys

import anytree
from anytree import NodeMixin, Node, RenderTree
import importlib
import json
import fnmatch

from anytree.exporter import DotExporter

import utilities

try:
    import auto_recognition
except ModuleNotFoundError:
    auto_recognition = None
import os
import logging

from Rhubarb import Rhubarb, RhubarbTimeoutException

try:
    import configparser
except ImportError:
    import ConfigParser as configparser

import PySide2.QtCore as QtCore

import utilities
from PronunciationDialogQT import PronunciationDialog, show_pronunciation_dialog

logger = logging.getLogger('LipsyncDoc')

ini_path = os.path.join(utilities.get_app_data_path(), "settings.ini")
config = QtCore.QSettings(ini_path, QtCore.QSettings.IniFormat)

if config.value("audio_output", "old") == "old":
    import SoundPlayer as SoundPlayer
else:
    if sys.platform == "win32":
        import SoundPlayerNew as SoundPlayer
    elif sys.platform == "darwin":
        import SoundPlayerOSX as SoundPlayer
    else:
        import SoundPlayer as SoundPlayer

strip_symbols = '.,!?;-/()"'
strip_symbols += '\N{INVERTED QUESTION MARK}'
strip_symbols += "'"


###############################################################

class LanguageManager:
    __shared_state = {}

    def __init__(self):
        self.__dict__ = self.__shared_state
        self.language_table = {}
        self.phoneme_dictionary = {}
        self.raw_dictionary = {}
        self.current_language = ""

        self.export_conversion = {}
        self.init_languages()

    def load_dictionary(self, path):
        try:
            in_file = open(path, 'r')
        except FileNotFoundError:
            print("Unable to open phoneme dictionary!:{}".format(path))
            return
        new_cmu_version = False
        if in_file.readline().startswith(";;; # CMUdict"):
            new_cmu_version = True
        # process dictionary entries
        for line in in_file.readlines():
            if not new_cmu_version:
                if line[0] == '#':
                    continue  # skip comments in the dictionary
            else:
                if line.startswith(";;;"):
                    continue
            # strip out leading/trailing whitespace
            line.strip()
            line = line.rstrip('\r\n')

            # split into components
            entry = line.split()
            if len(entry) == 0:
                continue
            # check if this is a duplicate word (alternate transcriptions end with a number in parentheses) - if so, throw it out
            if entry[0].endswith(')'):
                continue
            # add this entry to the in-memory dictionary
            for i in range(len(entry)):
                if i == 0:
                    self.raw_dictionary[entry[0]] = []
                else:
                    rawentry = entry[i]
                    self.raw_dictionary[entry[0]].append(rawentry)
        in_file.close()
        in_file = None

    def load_language(self, language_config, force=False):
        if self.current_language == language_config["label"] and not force:
            return
        self.current_language = language_config["label"]

        if "dictionaries" in language_config:
            for dictionary in language_config["dictionaries"]:
                self.load_dictionary(os.path.join(utilities.get_main_dir(),
                                                  language_config["location"],
                                                  language_config["dictionaries"][dictionary]))

    def language_details(self, dirname, names):
        if "language.ini" in names:
            config = configparser.ConfigParser()
            config.read(os.path.join(dirname, "language.ini"))
            label = config.get("configuration", "label")
            ltype = config.get("configuration", "type")
            details = {"label": label, "type": ltype, "location": dirname}
            if ltype == "breakdown":
                details["breakdown_class"] = config.get("configuration", "breakdown_class")
                self.language_table[label] = details
            elif ltype == "dictionary":
                try:
                    details["case"] = config.get("configuration", "case")
                except:
                    details["case"] = "upper"
                details["dictionaries"] = {}

                if config.has_section('dictionaries'):
                    for key, value in config.items('dictionaries'):
                        details["dictionaries"][key] = value
                self.language_table[label] = details
            else:
                print("unknown type ignored language not added to table")

    def init_languages(self):
        if len(self.language_table) > 0:
            return
        for path, dirs, files in os.walk(os.path.join(utilities.get_main_dir(), "rsrc", "languages")):
            if "language.ini" in files:
                self.language_details(path, files)


class LipSyncObject(NodeMixin):
    '''
    This should be a general class for all LipSync Objects
    '''

    def __init__(self, parent=None, children=None, object_type="voice", text="", start_frame=0, end_frame=0, name="",
                 tags=None, num_children=0, sound_duration=0, fps=24):
        self.parent = parent
        ini_path = os.path.join(utilities.get_app_data_path(), "settings.ini")
        self.config = QtCore.QSettings(ini_path, QtCore.QSettings.IniFormat)
        self.config.setFallbacksEnabled(False)  # File only, not registry or or.
        if children:
            self.children = children
        self.object_type = object_type
        self.name = name
        self.text = text
        self.start_frame = start_frame
        self.end_frame = end_frame
        self.tags = tags if tags else []
        self.num_children = num_children
        self.move_button = None
        if self.parent:
            self.sound_duration = self.root.sound_duration
        else:
            self.sound_duration = sound_duration
        self.fps = fps
        self.last_returned_frame = "rest"

    def get_min_size(self):
        # An object should be at least be able to contain all it's phonemes since only 1 phoneme per frame is allowed.
        if self.object_type == "phoneme":
            num_of_phonemes = 1
        else:
            num_of_phonemes = 0
            for descendant in self.descendants:
                if descendant.object_type == "phoneme":
                    num_of_phonemes += 1
        return num_of_phonemes

    def get_left_sibling(self):
        return anytree.util.leftsibling(self)

    def get_right_sibling(self):
        return anytree.util.rightsibling(self)

    def get_parent(self):
        if self.object_type != "phrase":
            return self.parent
        else:
            return None

    def get_frame_size(self):
        if self.object_type == "phoneme":
            return 1
        else:
            return self.end_frame - self.start_frame

    def has_shrink_room(self):
        if self.object_type == "phoneme":
            return False
        else:
            if self.get_min_size() >= self.get_frame_size():
                return False
            else:
                return True

    def has_left_sibling(self):
        try:
            left_sibling = bool(self.get_left_sibling())
        except AttributeError:
            left_sibling = False
        return left_sibling

    def has_right_sibling(self):
        try:
            right_sibling = bool(self.get_right_sibling())
        except AttributeError:
            right_sibling = False
        return right_sibling

    def get_left_max(self):
        if self.has_left_sibling():
            temp = self.get_left_sibling()
            if not temp.object_type == "phoneme":
                left_most_pos = temp.end_frame
            else:
                left_most_pos = temp.end_frame + 1
        else:
            left_most_pos = self.parent.start_frame
        return left_most_pos

    def get_right_max(self):
        if self.has_right_sibling():
            right_most_pos = self.get_right_sibling().start_frame
        else:
            if self.parent.object_type == "voice":
                right_most_pos = self.root.sound_duration
            elif self.parent.object_type == "project":
                right_most_pos = self.root.sound_duration
            else:
                try:
                    right_most_pos = self.parent.end_frame
                except AttributeError:
                    right_most_pos = self.root.sound_duration
        return right_most_pos

    def get_phoneme_at_frame(self, frame):
        for descendant in self.descendants:
            if descendant.object_type == "phoneme":
                if descendant.start_frame == frame:
                    self.last_returned_frame = descendant.text
                    return descendant.text

        if not self.config.value("RepeatLastPhoneme", True):
            return "rest"
        else:
            return self.last_returned_frame

    def reposition_to_left(self):
        if self.has_left_sibling():
            if self.object_type == "phoneme":
                self.start_frame = self.get_left_sibling().start_frame + 1
            else:
                self.start_frame = self.get_left_sibling().end_frame
                self.end_frame = self.start_frame + self.get_min_size()
                for child in self.children:
                    child.reposition_to_left()
        else:
            if self.object_type == "phoneme":
                self.start_frame = self.parent.start_frame
            else:
                self.start_frame = self.parent.name.node.start_frame
                self.end_frame = self.start_frame + self.get_min_size()
                for child in self.children:
                    child.reposition_to_left()

    def reposition_descendants(self, did_resize=False, x_diff=0):
        if did_resize:
            for child in self.children:
                child.reposition_to_left()
        else:
            for child in self.descendants:
                child.start_frame += x_diff
                child.end_frame += x_diff
                child.move_button.after_reposition()

    def reposition_descendants2(self, did_resize=False, x_diff=0):
        if did_resize:
            if self.object_type == "word":
                for position, child in enumerate(self.children):
                    child.start_frame = round(self.start_frame +
                                              ((self.get_frame_size() / self.get_min_size()) * position))
                    child.move_button.after_reposition()
                #self.wfv_parent.doc.dirty = True
            elif self.object_type == "phrase":
                extra_space = self.get_frame_size() - self.get_min_size()
                for child in self.children:
                    if child.has_left_sibling():
                        child.start_frame = child.get_left_sibling().end_frame
                        child.end_frame = child.start_frame + child.get_min_size()
                    else:
                        child.start_frame = self.start_frame
                        child.end_frame = child.start_frame + child.get_min_size()
                last_position = -1
                moved_child = False
                while extra_space > 0:
                    if last_position == len(self.children) - 1:
                        last_position = -1
                    if not moved_child:
                        last_position = -1
                    moved_child = False
                    for position, child in enumerate(self.children):
                        if child.has_left_sibling():
                            if child.start_frame < child.get_left_sibling().end_frame:
                                child.start_frame += 1
                                child.end_frame += 1
                            else:
                                if extra_space and not moved_child and (position > last_position):
                                    child.end_frame += 1
                                    extra_space -= 1
                                    moved_child = True
                                    last_position = position
                        else:
                            if extra_space and not moved_child and (position > last_position):
                                child.end_frame += 1
                                extra_space -= 1
                                moved_child = True
                                last_position = position
                    if not moved_child and extra_space == 0:
                        break
                for child in self.children:
                    child.move_button.after_reposition()
                    child.reposition_descendants2(True, 0)
                #self.wfv_parent.doc.dirty = True
        else:
            for child in self.descendants:
                child.start_frame += x_diff
                child.end_frame += x_diff
                child.move_button.after_reposition()
            #self.wfv_parent.doc.dirty = True

    def open(self, in_file):
        self.name = in_file.readline().strip()
        temp_text = in_file.readline().strip()
        self.text = temp_text.replace('|', '\n')
        num_phrases = int(in_file.readline())
        for p in range(num_phrases):
            self.num_children += 1
            phrase = LipSyncObject(object_type="phrase", parent=self)
            phrase.text = in_file.readline().strip()
            phrase.start_frame = int(in_file.readline())
            phrase.end_frame = int(in_file.readline())
            num_words = int(in_file.readline())
            for w in range(num_words):
                self.num_children += 1
                word = LipSyncObject(object_type="word", parent=phrase)
                word_line = in_file.readline().split()
                word.text = word_line[0]
                word.start_frame = int(word_line[1])
                word.end_frame = int(word_line[2])
                num_phonemes = int(word_line[3])
                for p2 in range(num_phonemes):
                    self.num_children += 1
                    phoneme = LipSyncObject(object_type="phoneme", parent=word)
                    phoneme_line = in_file.readline().split()
                    phoneme.start_frame = phoneme.end_frame = int(phoneme_line[0])
                    phoneme.text = phoneme_line[1]

    def save(self, out_file):
        out_file.write("\t{}\n".format(self.name))
        temp_text = self.text.replace('\n', '|')
        out_file.write("\t{}\n".format(temp_text))
        out_file.write("\t{:d}\n".format(len(self.children)))
        for phrase in self.children:
            out_file.write("\t\t{}\n".format(phrase.text))
            out_file.write("\t\t{:d}\n".format(phrase.start_frame))
            out_file.write("\t\t{:d}\n".format(phrase.end_frame))
            out_file.write("\t\t{:d}\n".format(len(phrase.children)))
            for word in phrase.children:
                out_file.write(
                    "\t\t\t{} {:d} {:d} {:d}\n".format(word.text, word.start_frame, word.end_frame, len(word.children)))
                for phoneme in word.children:
                    out_file.write("\t\t\t\t{:d} {}\n".format(phoneme.start_frame, phoneme.text))

    def run_breakdown(self, frame_duration, parent_window, language, languagemanager, phonemeset):
        if self.object_type == "voice":
            # First we delete all children
            self.children = []
            # make sure there is a space after all punctuation marks
            repeat_loop = True
            while repeat_loop:
                repeat_loop = False
                for i in range(len(self.text) - 1):
                    if (self.text[i] in ".,!?;-/()") and (not self.text[i + 1].isspace()):
                        self.text = "{} {}".format(self.text[:i + 1], self.text[i + 1:])
                        repeat_loop = True
                        break
            # break text into phrases
            # self.phrases = []
            for line in self.text.splitlines():
                if len(line) == 0:
                    continue
                phrase = LipSyncObject(object_type="phrase", parent=self)
                phrase.text = line
                # self.children.append(phrase)
            # now break down the phrases
            for phrase in self.children:
                return_value = phrase.run_breakdown(frame_duration, parent_window, language, languagemanager, phonemeset)
                if return_value == -1:
                    return -1
            # for first-guess frame alignment, count how many phonemes we have
            phoneme_count = 0
            for phrase in self.children:
                for word in phrase.children:
                    if len(word.children) == 0:  # deal with unknown words
                        phoneme_count += 4
                    for phoneme in word.children:
                        phoneme_count += 1
            # now divide up the total time by phonemes
            if frame_duration > 0 and phoneme_count > 0:
                frames_per_phoneme = int(float(frame_duration) / float(phoneme_count))
                if frames_per_phoneme < 1:
                    frames_per_phoneme = 1
            else:
                frames_per_phoneme = 1
            # finally, assign frames based on phoneme durations
            cur_frame = 0
            for phrase in self.children:
                for word in phrase.children:
                    for phoneme in word.children:
                        phoneme.start_frame = phoneme.end_frame = cur_frame
                        cur_frame += frames_per_phoneme
                    if len(word.children) == 0:  # deal with unknown words
                        word.start_frame = cur_frame
                        word.end_frame = cur_frame + 3
                        cur_frame += 4
                    else:
                        word.start_frame = word.children[0].start_frame
                        word.end_frame = word.children[-1].end_frame + frames_per_phoneme - 1
                phrase.start_frame = phrase.children[0].start_frame
                phrase.end_frame = phrase.children[-1].end_frame
        elif self.object_type == "phrase":
            # self.words = []
            for w in self.text.split():
                if len(w) == 0:
                    continue
                word = LipSyncObject(object_type="word", parent=self)
                word.text = w
                # self.words.append(word)
            for word in self.children:
                result = word.run_breakdown(frame_duration, parent_window, language, languagemanager, phonemeset)
                if result == -1:
                    return -1
        elif self.object_type == "word":
            # self.phonemes = []
            try:
                text = self.text.strip(strip_symbols)
                details = languagemanager.language_table[language]
                if details["type"] == "breakdown":
                    breakdown = importlib.import_module(details["breakdown_class"])
                    pronunciation_raw = breakdown.breakdown_word(text)
                elif details["type"] == "dictionary":
                    if languagemanager.current_language != language:
                        languagemanager.load_language(details)
                        languagemanager.current_language = language
                    if details["case"] == "upper":
                        pronunciation_raw = languagemanager.raw_dictionary[text.upper()]
                    elif details["case"] == "lower":
                        pronunciation_raw = languagemanager.raw_dictionary[text.lower()]
                    else:
                        pronunciation_raw = languagemanager.raw_dictionary[text]
                else:
                    # TODO why is phonemeDictionary not defined?
                    pronunciation_raw = phonemeDictionary[text.upper()]

                pronunciation = []
                for i in range(len(pronunciation_raw)):
                    try:
                        pronunciation_phoneme = pronunciation_raw[i].rstrip("0123456789")
                        pronunciation.append(phonemeset.conversion[pronunciation_phoneme])
                    except KeyError:
                        print(("Unknown phoneme:", pronunciation_raw[i], "in word:", text))

                for p in pronunciation:
                    if len(p) == 0:
                        continue
                    phoneme = LipSyncObject(object_type="phoneme", parent=self)
                    phoneme.text = p
                    # self.phonemes.append(phoneme)
            except KeyError:
                # this word was not found in the phoneme dictionary
                # TODO: This now depends on QT, make it neutral!
                return_value = show_pronunciation_dialog(parent_window, phonemeset.set, self.text)
                if return_value == -1:
                    return -1
                elif not return_value:
                    pass
                else:
                    conversion_map_to_cmu = {v: k for k, v in parent_window.doc.parent.phonemeset.conversion.items()}
                    phonemes_as_list = []
                    for p in return_value:
                        phoneme = LipSyncObject(object_type="phoneme", parent=self)
                        phoneme.text = p
                        phoneme_as_cmu = conversion_map_to_cmu.get(p, "rest")
                        phonemes_as_list.append(phoneme_as_cmu)
                    languagemanager.raw_dictionary[self.text.upper()] = phonemes_as_list

    ###

    def export(self, path):

        out_file = open(path, 'w')
        out_file.write("MohoSwitch1\n")
        phoneme = ""
        if len(self.children) > 0:
            start_frame = self.children[0].start_frame
            end_frame = self.children[-1].end_frame
            if start_frame != 0:
                phoneme = "rest"
                out_file.write("{:d} {}\n".format(1, phoneme))
        else:
            start_frame = 0
            end_frame = 1
        last_phoneme = self.leaves[0]
        for phoneme in self.leaves:
            if last_phoneme.text != phoneme.text:
                if phoneme.text == "rest":
                    # export an extra "rest" phoneme at the end of a pause between words or phrases
                    out_file.write("{:d} {}\n".format(phoneme.start_frame, phoneme.text))
            last_phoneme = phoneme
            out_file.write("{:d} {}\n".format(phoneme.start_frame + 1, phoneme.text))
        out_file.write("{:d} {}\n".format(end_frame + 2, "rest"))
        out_file.close()

    def export_images(self, path, currentmouth):
        if not self.config.value("MouthDir"):
            print("Use normal procedure.\n")
            phonemedict = {}
            for files in os.listdir(os.path.join(utilities.get_main_dir(), "rsrc", "mouths", currentmouth)):
                phonemedict[os.path.splitext(files)[0]] = os.path.splitext(files)[1]
            for phoneme in self.leaves:
                try:
                    shutil.copy(os.path.join(utilities.get_main_dir(), "rsrc", "mouths", currentmouth) + "/" +
                                phoneme.text + phonemedict[phoneme.text],
                                path + str(phoneme.start_frame).rjust(6, '0') +
                                phoneme.text + phonemedict[phoneme.text])
                except KeyError:
                    print("Phoneme \'{0}\' does not exist in chosen directory.".format(phoneme.text))

        else:
            print("Use this dir: {}\n".format(self.config.value("MouthDir")))
            phonemedict = {}
            for files in os.listdir(self.config.value("MouthDir")):
                phonemedict[os.path.splitext(files)[0]] = os.path.splitext(files)[1]
            for phoneme in self.leaves:
                try:
                    shutil.copy(
                        "{}/{}{}".format(self.config.value("MouthDir"), phoneme.text, phonemedict[phoneme.text]),
                        "{}{}{}{}".format(path, str(phoneme.start_frame).rjust(6, "0"), phoneme.text,
                                          phonemedict[phoneme.text]))
                except KeyError:
                    print("Phoneme \'{0}\' does not exist in chosen directory.".format(phoneme.text))

    def export_alelo(self, path, language, languagemanager):
        out_file = open(path, 'w')
        for phrase in self.children:
            for word in phrase.children:
                text = word.text.strip(strip_symbols)
                details = languagemanager.language_table[language]
                if languagemanager.current_language != language:
                    languagemanager.LoadLanguage(details)
                    languagemanager.current_language = language
                if details["case"] == "upper":
                    pronunciation = languagemanager.raw_dictionary[text.upper()]
                elif details["case"] == "lower":
                    pronunciation = languagemanager.raw_dictionary[text.lower()]
                else:
                    pronunciation = languagemanager.raw_dictionary[text]
                first = True
                position = -1
                #               print word.text
                for phoneme in word.children:
                    #                   print phoneme.text
                    if first:
                        first = False
                    else:
                        try:
                            out_file.write("{:d} {:d} {}\n".format(last_phoneme.start_frame, phoneme.start_frame - 1,
                                                                   languagemanager.export_conversion[
                                                                       last_phoneme_text]))
                        except KeyError:
                            pass
                    if phoneme.text.lower() == "sil":
                        position += 1
                        out_file.write("{:d} {:d} sil\n".format(phoneme.start_frame, phoneme.start_frame))
                        continue
                    position += 1
                    last_phoneme_text = pronunciation[position]
                    last_phoneme = phoneme
                try:
                    out_file.write("{:d} {:d} {}\n".format(last_phoneme.start_frame, word.end_frame,
                                                           languagemanager.export_conversion[last_phoneme_text]))
                except KeyError:
                    pass
        out_file.close()

    def export_json(self, path):
        if len(self.children) > 0:
            start_frame = self.children[0].start_frame
            end_frame = self.children[-1].end_frame
        else:  # No phrases means no data, so do nothing
            return
        json_data = {"name": self.name, "start_frame": start_frame, "end_frame": end_frame,
                     "text": self.text, "num_children": self.num_children, "fps": self.fps}
        list_of_phrases = []
        list_of_used_phonemes = []
        for phr_id, phrase in enumerate(self.children):
            dict_phrase = {"id": phr_id, "text": phrase.text, "start_frame": phrase.start_frame,
                           "end_frame": phrase.end_frame, "tags": phrase.tags}
            list_of_words = []
            for wor_id, word in enumerate(phrase.children):
                dict_word = {"id": wor_id, "text": word.text, "start_frame": word.start_frame,
                             "end_frame": word.end_frame, "tags": word.tags + phrase.tags}
                list_of_phonemes = []
                for pho_id, phoneme in enumerate(word.children):
                    dict_phoneme = {"id": pho_id, "text": phoneme.text, "frame": phoneme.start_frame,
                                    "tags": phoneme.tags + word.tags + phrase.tags}
                    list_of_phonemes.append(dict_phoneme)
                    if phoneme.text not in list_of_used_phonemes:
                        list_of_used_phonemes.append(phoneme.text)
                dict_word["phonemes"] = list_of_phonemes
                list_of_words.append(dict_word)
            dict_phrase["words"] = list_of_words
            list_of_phrases.append(dict_phrase)
        json_data["phrases"] = list_of_phrases
        json_data["used_phonemes"] = list_of_used_phonemes
        file_path = open(path, "w")
        json.dump(json_data, file_path, indent=True)
        file_path.close()

    ###

    def __str__(self):
        if self.is_root:
            out_string = "LipSync{}:{}|{}|start_frame:{}|end_frame:{}|Children:{}".format(
                self.object_type.capitalize(),
                self.name, self.text,
                self.start_frame,
                self.end_frame, self.children)
        else:
            out_string = "LipSync{}:{}|{}|start_frame:{}|end_frame:{}|Parent:{}|Children:{}".format(
                self.object_type.capitalize(),
                self.name, self.text,
                self.start_frame,
                self.end_frame, self.parent.name or self.parent.text, self.children)
        return out_string

    def __repr__(self):
        return self.__str__()


class LipsyncDoc:
    def __init__(self, langman: LanguageManager, parent):
        self._dirty = False
        ini_path = os.path.join(utilities.get_app_data_path(), "settings.ini")
        self.settings = QtCore.QSettings(ini_path, QtCore.QSettings.IniFormat)
        self.settings.setFallbacksEnabled(False)  # File only, not registry or or.
        self.name = "Untitled"
        self.path = None
        self.fps = 24
        self.soundDuration = 72
        self.soundPath = ""
        self.sound = None
        self.voices = []
        self.current_voice = None
        self.language_manager = langman
        self.parent = parent
        self.project_node = LipSyncObject(name=self.name, object_type="project")

    @property
    def dirty(self):
        return self._dirty

    @dirty.setter
    def dirty(self, value):
        self._dirty = value

    def __del__(self):
        # Properly close down the sound object
        if self.sound is not None:
            del self.sound

    def open_json(self, path):
        self._dirty = False
        self.path = os.path.normpath(path)
        self.name = os.path.basename(path)
        self.project_node.name = self.name
        self.sound = None
        self.voices = []
        self.current_voice = None
        file_data = open(self.path, "r")
        json_data = json.load(file_data)
        self.soundPath = json_data.get("sound_path", "")
        if not os.path.isabs(self.soundPath):
            self.soundPath = os.path.normpath("{}/{}".format(os.path.dirname(self.path), self.soundPath))
        self.fps = json_data["fps"]
        self.soundDuration = json_data["sound_duration"]
        self.project_node.sound_duration = self.soundDuration
        self.parent.phonemeset.selected_set = json_data.get("phoneme_set", "preston_blair")
        num_voices = json_data["num_voices"]
        for voice in json_data["voices"]:
            temp_voice = LipSyncObject(name=voice["name"], text=voice["text"], num_children=voice["num_children"],
                                       fps=self.fps, parent=self.project_node, object_type="voice",
                                       sound_duration=self.soundDuration)
            for phrase in voice["phrases"]:
                temp_phrase = LipSyncObject(text=phrase["text"], start_frame=phrase["start_frame"],
                                            end_frame=phrase["end_frame"], tags=phrase["tags"], fps=self.fps,
                                            object_type="phrase", parent=temp_voice, sound_duration=self.soundDuration)
                for word in phrase["words"]:
                    temp_word = LipSyncObject(text=word["text"], start_frame=word["start_frame"], fps=self.fps,
                                              end_frame=word["end_frame"], tags=word["tags"], object_type="word",
                                              parent=temp_phrase, sound_duration=self.soundDuration)
                    for phoneme in word["phonemes"]:
                        temp_phoneme = LipSyncObject(text=phoneme["text"], start_frame=phoneme["frame"],
                                                     end_frame=phoneme["frame"], tags=phoneme["tags"], fps=self.fps,
                                                     object_type="phoneme", parent=temp_word,
                                                     sound_duration=self.soundDuration)
            self.voices.append(temp_voice)
        file_data.close()
        self.open_audio(self.soundPath)
        if len(self.voices) > 0:
            self.current_voice = self.voices[0]

    def open(self, path):
        self._dirty = False
        self.path = os.path.normpath(path)
        self.name = os.path.basename(path)
        self.project_node.name = self.name
        self.sound = None
        self.voices = []
        self.current_voice = None
        in_file = open(self.path, 'r')
        in_file.readline()  # discard the header
        self.soundPath = in_file.readline().strip()
        if not os.path.isabs(self.soundPath):
            self.soundPath = os.path.normpath("{}/{}".format(os.path.dirname(self.path), self.soundPath))
        self.fps = int(in_file.readline())
        print(("self.path: {}".format(self.path)))
        self.soundDuration = int(in_file.readline())
        print(("self.soundDuration: {:d}".format(self.soundDuration)))
        self.project_node.sound_duration = self.soundDuration
        num_voices = int(in_file.readline())
        for i in range(num_voices):
            voice = LipSyncObject(object_type="voice", parent=self.project_node)
            voice.open(in_file)
            self.voices.append(voice)
        in_file.close()
        self.open_audio(self.soundPath)
        if len(self.voices) > 0:
            self.current_voice = self.voices[0]

    def open_audio(self, path):
        if not os.path.exists(path):
            return
        if self.sound is not None:
            del self.sound
            self.sound = None
        # self.soundPath = str(path.encode("utf-8"))
        self.soundPath = path  # .encode('latin-1', 'replace')
        self.sound = SoundPlayer.SoundPlayer(self.soundPath, self.parent)
        if self.sound.IsValid():
            print("valid sound")
            self.soundDuration = int(self.sound.Duration() * self.fps)
            print(("self.sound.Duration(): {:d}".format(int(self.sound.Duration()))))
            print(("frameRate: {:d}".format(int(self.fps))))
            print(("soundDuration1: {:d}".format(self.soundDuration)))
            if self.soundDuration < self.sound.Duration() * self.fps:
                self.soundDuration += 1
                print(("soundDuration2: {:d}".format(self.soundDuration)))
            self.project_node.sound_duration = self.soundDuration
        else:
            self.sound = None

    def open_from_dict(self, json_data):
        self._dirty = False
        # self.path = os.path.normpath(path)
        # self.name = os.path.basename(path)
        self.project_node.name = "Test_Copy"
        self.project_node.children = []
        self.sound = None
        self.voices = []
        self.current_voice = None
        self.soundPath = json_data.get("sound_path", "")
        if not os.path.isabs(self.soundPath):
            self.soundPath = os.path.normpath("{}/{}".format(os.path.dirname(self.path), self.soundPath))
        self.fps = json_data["fps"]
        self.soundDuration = json_data["sound_duration"]
        self.project_node.sound_duration = self.soundDuration
        self.parent.phonemeset.selected_set = json_data.get("phoneme_set", "preston_blair")
        num_voices = json_data["num_voices"]
        for voice in json_data["voices"]:
            temp_voice = LipSyncObject(name=voice["name"], text=voice["text"], num_children=voice["num_children"],
                                       fps=self.fps, parent=self.project_node, object_type="voice",
                                       sound_duration=self.soundDuration)
            for phrase in voice["phrases"]:
                temp_phrase = LipSyncObject(text=phrase["text"], start_frame=phrase["start_frame"],
                                            end_frame=phrase["end_frame"], tags=phrase["tags"], fps=self.fps,
                                            object_type="phrase", parent=temp_voice, sound_duration=self.soundDuration)
                for word in phrase["words"]:
                    temp_word = LipSyncObject(text=word["text"], start_frame=word["start_frame"], fps=self.fps,
                                              end_frame=word["end_frame"], tags=word["tags"], object_type="word",
                                              parent=temp_phrase, sound_duration=self.soundDuration)
                    for phoneme in word["phonemes"]:
                        temp_phoneme = LipSyncObject(text=phoneme["text"], start_frame=phoneme["frame"],
                                                     end_frame=phoneme["frame"], tags=phoneme["tags"], fps=self.fps,
                                                     object_type="phoneme", parent=temp_word,
                                                     sound_duration=self.soundDuration)
            self.voices.append(temp_voice)
        # file_data.close()
        self.open_audio(self.soundPath)
        if len(self.voices) > 0:
            self.current_voice = self.voices[0]

    def copy_to_dict(self, saved_sound_path=""):
        if not saved_sound_path:
            saved_sound_path = self.soundPath
        out_dict = {"version": 2, "sound_path": saved_sound_path, "fps": self.fps,
                    "sound_duration": self.soundDuration,
                    "num_voices": len(self.project_node.children),
                    "phoneme_set": self.parent.phonemeset.selected_set}
        list_of_voices = []
        for voi_id, voice in enumerate(self.project_node.children):
            start_frame = 0
            end_frame = 1
            if len(voice.children) > 0:
                start_frame = voice.children[0].start_frame
                end_frame = voice.children[-1].end_frame
            json_data = {"name": voice.name, "start_frame": start_frame, "end_frame": end_frame,
                         "text": voice.text, "num_children": len(voice.descendants)}
            list_of_phrases = []
            list_of_used_phonemes = []
            for phr_id, phrase in enumerate(voice.children):
                dict_phrase = {"id": phr_id, "text": phrase.text, "start_frame": phrase.start_frame,
                               "end_frame": phrase.end_frame, "tags": phrase.tags}
                list_of_words = []
                for wor_id, word in enumerate(phrase.children):
                    dict_word = {"id": wor_id, "text": word.text, "start_frame": word.start_frame,
                                 "end_frame": word.end_frame, "tags": word.tags}
                    list_of_phonemes = []
                    for pho_id, phoneme in enumerate(word.children):
                        dict_phoneme = {"id": pho_id, "text": phoneme.text,
                                        "frame": phoneme.start_frame, "tags": phoneme.tags}
                        list_of_phonemes.append(dict_phoneme)
                        if phoneme.text not in list_of_used_phonemes:
                            list_of_used_phonemes.append(phoneme.text)
                    dict_word["phonemes"] = list_of_phonemes
                    list_of_words.append(dict_word)
                dict_phrase["words"] = list_of_words
                list_of_phrases.append(dict_phrase)
            json_data["phrases"] = list_of_phrases
            json_data["used_phonemes"] = list_of_used_phonemes
            list_of_voices.append(json_data)
        out_dict["voices"] = list_of_voices
        return out_dict

    def save2(self, path):
        self.path = os.path.normpath(path)
        self.name = os.path.basename(path)
        self.project_node.name = self.name
        if os.path.dirname(self.path) == os.path.dirname(self.soundPath):
            saved_sound_path = os.path.basename(self.soundPath)
        else:
            saved_sound_path = self.soundPath
        out_json = self.copy_to_dict(saved_sound_path)
        file_path = open(self.path, "w")
        json.dump(out_json, file_path, indent=True)
        file_path.close()

    def save(self, path):
        self.path = os.path.normpath(path)
        self.name = os.path.basename(path)
        self.project_node.name = self.name
        if os.path.dirname(self.path) == os.path.dirname(self.soundPath):
            saved_sound_path = os.path.basename(self.soundPath)
        else:
            saved_sound_path = self.soundPath
        out_file = open(self.path, "w")
        out_file.write("lipsync version 1\n")
        out_file.write("{}\n".format(saved_sound_path))
        out_file.write("{:d}\n".format(self.fps))
        out_file.write("{:d}\n".format(self.soundDuration))
        out_file.write("{:d}\n".format(len(self.project_node.children)))
        for voice in self.project_node.children:
            voice.save(out_file)
        out_file.close()
        self._dirty = False

    def convert_to_phonemeset_old(self):
        # The base set is the CMU39 set, we will convert everything to that and from it to the desired one for now
        new_set = self.parent.main_window.phoneme_set.currentText()
        old_set = self.parent.phonemeset.selected_set
        if old_set != new_set:
            if old_set != "CMU_39":
                conversion_map_to_cmu = {v: k for k, v in self.parent.phonemeset.conversion.items()}
                for voice in self.project_node.children:
                    for phrase in voice.children:
                        for word in phrase.children:
                            for phoneme in word.children:
                                phoneme.text = conversion_map_to_cmu.get(phoneme.text, "rest")
            new_map = PhonemeSet()
            new_map.load(new_set)
            conversion_map_from_cmu = new_map.conversion
            for voice in self.project_node.children:
                for phrase in voice.children:
                    for word in phrase.children:
                        for phoneme in word.children:
                            phoneme.text = conversion_map_from_cmu.get(phoneme.text, "rest")
            self.dirty = True
            self.parent.phonemeset.selected_set = new_set
            self.parent.main_window.waveform_view.set_document(self, force=True, clear_scene=True)

    def convert_to_phonemeset(self):
        # The base set is the CMU39 set, we will convert everything to that and from it to the desired one for now
        new_set = self.parent.main_window.phoneme_set.currentText()
        old_set = self.parent.phonemeset.selected_set
        conversion_dict = {}
        if old_set != new_set:
            new_map = PhonemeSet()
            new_map.load(new_set)
            for conversion_name in new_map.alternate_conversions:
                if conversion_name.startswith(old_set.lower()):
                    print(conversion_name)
                    conversion_dict = new_map.alternate_conversions[conversion_name]
            if conversion_dict:
                print(conversion_dict)
                for voice in self.project_node.children:
                    for phrase in voice.children:
                        for word in phrase.children:
                            for phoneme in word.children:
                                phoneme.text = conversion_dict.get(phoneme.text, "rest")
                self.dirty = True
                self.parent.phonemeset.selected_set = new_set
                self.parent.main_window.waveform_view.set_document(self, force=True, clear_scene=True)
            # if old_set != "CMU_39":
            #     conversion_map_to_cmu = {v: k for k, v in self.parent.phonemeset.conversion.items()}
            #     for voice in self.project_node.children:
            #         for phrase in voice.children:
            #             for word in phrase.children:
            #                 for phoneme in word.children:
            #                     phoneme.text = conversion_map_to_cmu.get(phoneme.text, "rest")
            # new_map = PhonemeSet()
            # new_map.load(new_set)
            # conversion_map_from_cmu = new_map.conversion
            # for voice in self.project_node.children:
            #     for phrase in voice.children:
            #         for word in phrase.children:
            #             for phoneme in word.children:
            #                 phoneme.text = conversion_map_from_cmu.get(phoneme.text, "rest")
            # self.dirty = True
            # self.parent.phonemeset.selected_set = new_set
            # self.parent.main_window.waveform_view.set_document(self, force=True, clear_scene=True)

    def auto_recognize_phoneme(self, manual_invoke=False):
        if self.settings.value("/VoiceRecognition/run_voice_recognition", "true").lower() == "true" or manual_invoke:
            if auto_recognition:
                if self.settings.value("/VoiceRecognition/recognizer", "Allosaurus") == "Allosaurus":
                    allo_recognizer = auto_recognition.AutoRecognize(self.soundPath)
                    out_or_none = allo_recognizer.recognize_allosaurus()
                    if out_or_none is None:
                        logger.warn('recognize_allosaurus returned None; no results')
                        # set default/empty values on none
                        results, peaks, allo_output = (None, None, None)
                    else:
                        results, peaks, allo_output = out_or_none

                    if results:
                        phonemes_as_text = ""
                        end_frame = math.floor(self.fps * (results[-1]["start"] + results[-1]["duration"] * 2))
                        phrase = LipSyncObject(object_type="phrase", parent=self.current_voice)
                        phrase.text = 'Auto detection Allosaurus'
                        phrase.start_frame = 0
                        phrase.end_frame = end_frame
                        for i in range(len(peaks) - 1):
                            peak_left = peaks[i]
                            peak_right = peaks[i + 1]

                            word_chunk = results[peak_left:peak_right]
                            word = LipSyncObject(object_type="word", parent=phrase)

                            word.text = "|".join(
                                letter["phoneme"] if letter["phoneme"] is not None else "rest" for letter in word_chunk)
                            word.start_frame = math.floor(self.fps * results[peak_left]["start"])
                            # word.end_frame = math.floor(self.fps * results[peak_right]["start"])
                            previous_frame_pos = math.floor(self.fps * results[peak_left]["start"]) - 1
                            for phoneme in word_chunk:
                                current_frame_pos = math.floor(self.fps * phoneme['start'])
                                if current_frame_pos == previous_frame_pos:
                                    current_frame_pos += 1
                                pg_phoneme = LipSyncObject(object_type="phoneme", parent=word)
                                pg_phoneme.start_frame = pg_phoneme.end_frame = current_frame_pos
                                previous_frame_pos = current_frame_pos
                                pg_phoneme.text = phoneme['phoneme'] if phoneme['phoneme'] is not None else 'rest'
                                # word.phonemes.append(pg_phoneme)
                                phonemes_as_text += pg_phoneme.text
                            phonemes_as_text += " "
                            word.end_frame = previous_frame_pos + 1
                            # phrase.words.append(word)
                        phonemes_as_text += "\n{}".format(str(allo_output))
                        try:
                            self.parent.main_window.text_edit.setText(phonemes_as_text)
                        except AttributeError:
                            pass
                        phrase.end_frame = phrase.children[-1].end_frame
                        # self.current_voice.phrases.append(phrase)
                        self.parent.phonemeset.selected_set = self.parent.phonemeset.load("CMU_39")
                        try:
                            current_index = self.parent.main_window.phoneme_set.findText(self.parent.phonemeset.selected_set)
                            self.parent.main_window.phoneme_set.setCurrentIndex(current_index)
                        except AttributeError:
                            pass
                elif self.settings.value("/VoiceRecognition/recognizer", "Allosaurus") == "Rhubarb":
                    try:
                        phonemes = Rhubarb(self.soundPath).run()
                        if not phonemes:
                            return
                        end_frame = math.floor(self.fps * phonemes[-1]['end'])
                        phrase = LipSyncObject(object_type="phrase", parent=self.current_voice)
                        phrase.text = 'Auto detection rhubarb'
                        phrase.start_frame = 0
                        phrase.end_frame = end_frame

                        word = LipSyncObject(object_type="word", parent=phrase)
                        word.text = 'rhubarb'
                        word.start_frame = 0
                        word.end_frame = end_frame

                        for phoneme in phonemes:
                            pg_phoneme = LipSyncObject(object_type="phoneme", parent=word)
                            pg_phoneme.start_frame = pg_phoneme.end_frame = math.floor(self.fps * phoneme['start'])
                            pg_phoneme.text = phoneme['value'] if phoneme['value'] != 'X' else 'rest'
                            # word.phonemes.append(pg_phoneme)

                        # phrase.words.append(word)
                        # self.current_voice.phrases.append(phrase)
                        self.parent.phonemeset.selected_set = self.parent.phonemeset.load("rhubarb")
                        current_index = self.parent.main_window.phoneme_set.findText(
                            self.parent.phonemeset.selected_set)
                        self.parent.main_window.phoneme_set.setCurrentIndex(current_index)

                    except RhubarbTimeoutException:
                        pass

    def __str__(self):
        out_string = "LipSyncDoc:{}|Objects:{}|Sound:{}|".format(self.name, self.project_node, self.soundPath)
        return out_string

    def __repr__(self):
        return self.__str__()


class PhonemeSet:
    __shared_state = {}

    def __init__(self):
        self.__dict__ = self.__shared_state
        self.set = []
        self.conversion = {}
        self.alternatives = []
        self.alternate_conversions = {}
        for file in os.listdir(os.path.join(utilities.get_main_dir(), "phonemes")):
            if fnmatch.fnmatch(file, '*.json'):
                self.alternatives.append(file.split(".")[0])

        # Try to load Preston Blair as default as before, but fall back just in case
        self.selected_set = self.load("preston_blair")
        if not self.selected_set:
            self.load(self.alternatives[0])
            self.selected_set = self.alternatives[0]

    def load(self, name=''):
        if name in self.alternatives:
            with open(os.path.join(utilities.get_main_dir(), "./phonemes/{}.json".format(name)), "r") as loaded_file:
                json_data = json.load(loaded_file)
                self.set = json_data["phoneme_set"]
                if name.lower() != "cmu_39":
                    self.conversion = json_data.get("cmu_39_phoneme_conversion")
                else:
                    for phoneme in self.set:
                        self.conversion[phoneme] = phoneme
                for key in json_data:
                    if key != "phoneme_set":
                        self.alternate_conversions[key] = json_data[key]
                return name
        else:
            print(("Can't find phonemeset! ({})".format(name)))
            return False


```

## Source file: MouthViewQT.py

{
  "path": "MouthViewQT.py",
  "blob_sha": "7d334623beb121f7abc60344e63bd88c3839a443",
  "sha256": "d7f73f78ae718b8479e79ba67706a54f7ab8282f9573fb9341d08fc02a751275",
  "lines": 128,
  "bytes": 5480,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 33,
    "rig": 1,
    "lip": 1
  },
  "url": "https://github.com/morevnaproject-org/papagayo-ng/blob/17fa36a619817362f79726f85ef520489c01fe6a/MouthViewQT.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def __init__(self, parent=None):"
    },
    {
      "line": 46,
      "text": "def draw_me(self, dc=None):"
    },
    {
      "line": 75,
      "text": "def set_frame(self, frame):"
    },
    {
      "line": 80,
      "text": "def set_phoneme_picture(self, phoneme):"
    },
    {
      "line": 98,
      "text": "def set_document(self, doc):"
    },
    {
      "line": 102,
      "text": "def process_mouth_dir(self, dir_name, names, supported_imagetypes):"
    },
    {
      "line": 112,
      "text": "def load_mouths(self):"
    },
    {
      "line": 117,
      "text": "def add_mouth(self, dir_name, names):"
    }
  ]
}

```text

# -*- coding: ISO-8859-1 -*-
# generated by wxGlade 0.3.5.1 on Fri Apr 15 17:02:14 2005

# Papagayo-NG, a lip-sync tool for use with several different animation suites
# Original Copyright (C) 2005 Mike Clifton
# Contact information at http://www.lostmarble.com
#
# This program is free software; you can redistribute it and/or
# modify it under the terms of the GNU General Public License
# as published by the Free Software Foundation; either version 2
# of the License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.

# import os
import os

from PySide2 import QtGui, QtCore, QtWidgets

import utilities


class MouthView(QtWidgets.QGraphicsView):
    def __init__(self, parent=None):
        super(MouthView, self).__init__(parent)
        self.setScene(QtWidgets.QGraphicsScene(self))
        self.setVerticalScrollBarPolicy(QtCore.Qt.ScrollBarAlwaysOff)
        self.setHorizontalScrollBarPolicy(QtCore.Qt.ScrollBarAlwaysOff)
        self.translator = utilities.ApplicationTranslator()
        # Other initialization
        self.doc = None
        self.cur_frame = 0
        self.old_frame = 0
        self.current_phoneme = "rest"
        self.current_mouth = None
        self.mouths = {}
        self.load_mouths()

    def draw_me(self, dc=None):
        if (self.doc is not None) and (self.doc.sound is not None) and (self.doc.sound.is_playing()):
            if self.doc.current_voice is not None:
                phoneme = self.doc.current_voice.get_phoneme_at_frame(self.cur_frame)
            else:
                phoneme = "rest"
            if phoneme == self.current_phoneme:
                return
            else:
                self.current_phoneme = phoneme
        else:
            self.current_phoneme = "rest"
        if not self.current_mouth:
            self.current_mouth = list(self.mouths)[0]
        if self.current_mouth in self.mouths.keys():
            if self.current_phoneme in self.mouths[self.current_mouth].keys():
                bitmap = self.mouths[self.current_mouth][self.current_phoneme]
            else:
                bitmap = self.mouths[self.current_mouth]["rest"]
        else:
            bitmap = self.mouths[list(self.mouths)[0]]["rest"]
        self.scene().clear()
        bitmap = bitmap.scaled(self.sceneRect().width() or 200, self.sceneRect().height() or 200,
                               QtCore.Qt.KeepAspectRatio)
        self.scene().addPixmap(bitmap)
        if self.current_phoneme not in self.mouths[self.current_mouth].keys():
            self.scene().addText(self.translator.translate("MouthView", "Missing Mouth: {0}").format(self.current_phoneme), QtGui.QFont("Swiss", 14))
        self.fitInView(self.x(), self.y(), self.width(), self.height())

    def set_frame(self, frame):
        self.old_frame = self.cur_frame
        self.cur_frame = frame
        self.draw_me()

    def set_phoneme_picture(self, phoneme):
        if not self.current_mouth:
            self.current_mouth = list(self.mouths)[0]
        if self.current_mouth in self.mouths.keys():
            if phoneme in self.mouths[self.current_mouth].keys():
                bitmap = self.mouths[self.current_mouth][phoneme]
            else:
                bitmap = self.mouths[self.current_mouth]["rest"]
        else:
            bitmap = self.mouths[list(self.mouths)[0]]["rest"]
        self.scene().clear()
        bitmap = bitmap.scaled(self.sceneRect().width() or 200, self.sceneRect().height() or 200,
                               QtCore.Qt.KeepAspectRatio)
        self.scene().addPixmap(bitmap)
        if phoneme not in self.mouths[self.current_mouth].keys():
            self.scene().addText(self.translator.translate("MouthView", "Missing Mouth: {0}").format(phoneme), QtGui.QFont("Swiss", 14))
        self.fitInView(self.x(), self.y(), self.width(), self.height())

    def set_document(self, doc):
        self.doc = doc
        self.draw_me()

    def process_mouth_dir(self, dir_name, names, supported_imagetypes):
        has_images = False
        for files in names:
            files = files.lower()
            if files.split(".")[-1] in supported_imagetypes:
                has_images = True
        if not has_images:
            return
        self.add_mouth(os.path.normpath(dir_name), names)

    def load_mouths(self):
        supported_imagetypes = QtGui.QImageReader.supportedImageFormats()
        for directory, dir_names, file_names in os.walk(os.path.join(utilities.get_main_dir(), "rsrc", "mouths")):
            self.process_mouth_dir(directory, file_names, supported_imagetypes)

    def add_mouth(self, dir_name, names):
        bitmaps = {}
        for files in names:
            if ".svn" in files:
                continue
            path = os.path.normpath(os.path.join(dir_name, files))
            bitmaps[files.split('.')[0]] = QtGui.QPixmap(path)
        self.mouths[os.path.basename(dir_name)] = bitmaps
        if self.current_mouth is None:
            self.current_mouth = os.path.basename(dir_name)

# end of class MouthView


```

## Source file: LipsyncFrameQT.py

{
  "path": "LipsyncFrameQT.py",
  "blob_sha": "5bdb020c1eca554231970b3d76b166eae362baae",
  "sha256": "8406d3e1a98f49187b50e0f64e2b6c9a17c471351106e5847de00f4e059ca6b3",
  "lines": 1143,
  "bytes": 60405,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 33,
    "rig": 29,
    "lip": 59
  },
  "url": "https://github.com/morevnaproject-org/papagayo-ng/blob/17fa36a619817362f79726f85ef520489c01fe6a/LipsyncFrameQT.py",
  "function_signature_candidates": [
    {
      "line": 66,
      "text": "def eventFilter(self, obj, event):"
    },
    {
      "line": 93,
      "text": "def sort_mouth_list_order(elem):"
    },
    {
      "line": 100,
      "text": "def open_file_no_gui(path, parent):"
    },
    {
      "line": 135,
      "text": "def __init__(self):"
    },
    {
      "line": 320,
      "text": "def change_stylesheet(self):"
    },
    {
      "line": 328,
      "text": "def download_general_finished(self):"
    },
    {
      "line": 346,
      "text": "def download_ffmpeg_finished(self):"
    },
    {
      "line": 357,
      "text": "def start_download(self, work_job):"
    },
    {
      "line": 370,
      "text": "def status_bar_progress(self, n):"
    },
    {
      "line": 374,
      "text": "def download_allosaurus_model(self, progress_callback):"
    },
    {
      "line": 405,
      "text": "def download_ffmpeg(self, progress_callback):"
    },
    {
      "line": 456,
      "text": "def download_rhubarb(self, progress_callback):"
    },
    {
      "line": 506,
      "text": "def load_ui_widget(self, ui_filename, parent=None):"
    },
    {
      "line": 516,
      "text": "def set_current_phoneme_set(self, event=None):"
    },
    {
      "line": 521,
      "text": "def select_voice_recognizer(self, event=None):"
    },
    {
      "line": 530,
      "text": "def change_voice_for_selection(self):"
    },
    {
      "line": 594,
      "text": "def add_tag(self):"
    },
    {
      "line": 603,
      "text": "def remove_tag(self):"
    },
    {
      "line": 610,
      "text": "def spread_out(self):"
    },
    {
      "line": 621,
      "text": "def apply_changed_fps(self):"
    },
    {
      "line": 641,
      "text": "def on_voice_recognize(self):"
    },
    {
      "line": 652,
      "text": "def close_doc_ok(self):"
    },
    {
      "line": 677,
      "text": "def on_open(self):"
    },
    {
      "line": 688,
      "text": "def open(self, path):"
    },
    {
      "line": 778,
      "text": "def on_save(self):"
    },
    {
      "line": 789,
      "text": "def on_save_as(self):"
    },
    {
      "line": 804,
      "text": "def on_close(self):"
    },
    {
      "line": 829,
      "text": "def on_quit(self, event=None):"
    },
    {
      "line": 833,
      "text": "def on_help(self, event=None):"
    },
    {
      "line": 839,
      "text": "def on_about(self, event=None):"
    },
    {
      "line": 843,
      "text": "def show_settings(self, event=None):"
    },
    {
      "line": 848,
      "text": "def settings_closed(self):"
    },
    {
      "line": 870,
      "text": "def on_play(self, event=None):"
    },
    {
      "line": 883,
      "text": "def on_stop(self, event=None):"
    },
    {
      "line": 898,
      "text": "def on_play_tick(self, event=None):"
    },
    {
      "line": 919,
      "text": "def change_volume(self, e):"
    },
    {
      "line": 924,
      "text": "def on_mouth_choice(self, event=None):"
    },
    {
      "line": 928,
      "text": "def on_export_choice(self, event=None):"
    },
    {
      "line": 934,
      "text": "def on_voice_name(self, event=None):"
    },
    {
      "line": 945,
      "text": "def on_voice_text(self, event=None):"
    },
    {
      "line": 952,
      "text": "def on_voice_breakdown(self, event=None):"
    },
    {
      "line": 969,
      "text": "def on_voice_export(self, event=None):"
    },
    {
      "line": 1025,
      "text": "def on_sel_voice_tab(self, e):"
    },
    {
      "line": 1045,
      "text": "def on_new_voice(self, event=None):"
    },
    {
      "line": 1076,
      "text": "def on_del_object(self):"
    },
    {
      "line": 1088,
      "text": "def on_del_voice(self, event=None):"
    },
    {
      "line": 1110,
      "text": "def on_voice_image_choose(self, event=None):"
    },
    {
      "line": 1135,
      "text": "def on_reload_dictionary(self, event=None):"
    },
    {
      "line": 1140,
      "text": "def quit_application(self):"
    }
  ]
}

```text

# -*- coding: ISO-8859-1 -*-
# generated by wxGlade 0.3.5.1 on Wed Apr 13 16:04:35 2005

# Papagayo-NG, a lip-sync tool for use with several different animation suites
# Original Copyright (C) 2005 Mike Clifton
# Contact information at http://www.lostmarble.com
#
# This program is free software; you can redistribute it and/or
# modify it under the terms of the GNU General Public License
# as published by the Free Software Foundation; either version 2
# of the License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.

# import os
# from utilities import Worker, WorkerSignals
import importlib
import json
import shutil
import sys
import os
import tarfile
import time
from functools import partial
from pprint import pprint

from PySide2.QtCore import QFile, QObject
from PySide2 import QtCore, QtGui, QtWidgets
from PySide2.QtUiTools import QUiLoader as uic

import urllib.request
import io
from zipfile import ZipFile
import utilities
import platform
import random
import re

from WaveformViewRewrite import WaveformView
from MouthViewQT import MouthView
# end wxGlade

from AboutBoxQT import AboutBox
from SettingsQT import SettingsWindow
import LipsyncDoc

app_title = "Papagayo-NG"
lipsync_extension_list = ("pgo", "pg2")
audio_extension_list = ("wav", "mp3", "aiff", "aif", "au", "snd", "mov", "m4a")
export_file_types = ("txt", "json", "dat")
exporter_list = ("MOHO", "ALELO", "Images", "JSON")
lipsync_extension = "".join(" *.{}".format(ext) for ext in lipsync_extension_list)[1:]
audio_extensions = "".join(" *.{}".format(ext) for ext in audio_extension_list)[1:]
open_wildcard = "{} and sound files ({} {})".format(app_title, audio_extensions, lipsync_extension)
save_wildcard = "{} files ({})".format(app_title, lipsync_extension)


class DropFilter(QtCore.QObject):
    def eventFilter(self, obj, event):
        if event.type() == QtCore.QEvent.DragEnter:
            if event.mimeData().hasUrls():
                event.accept()
            else:
                event.ignore()
            return True
        elif event.type() == QtCore.QEvent.Drop:
            if event.mimeData().hasUrls():
                # event.accept()
                for url in event.mimeData().urls():
                    if sys.platform == "darwin":
                        from Foundation import NSURL
                        fname = str(NSURL.URLWithString_(str(url.toString())).filePathURL().path())
                        obj.topLevelWidget().lip_sync_frame.open(fname)
                    else:
                        fname = str(url.toLocalFile())
                        obj.topLevelWidget().lip_sync_frame.open(fname)
                return True
            else:
                event.ignore()
                return False

        else:
            return False


def sort_mouth_list_order(elem):
    try:
        return int(elem.split("-")[0])
    except ValueError:
        return hash(elem)


def open_file_no_gui(path, parent):
    importlib.reload(LipsyncDoc)  # This makes the CLI version work on the first try but is very hacky
    langman = LipsyncDoc.LanguageManager()
    langman.init_languages()
    ini_path = os.path.join(utilities.get_app_data_path(), "settings.ini")
    config = QtCore.QSettings(ini_path, QtCore.QSettings.IniFormat)
    doc = LipsyncDoc.LipsyncDoc(langman, parent)
    if path.endswith(lipsync_extension_list):
        if path.endswith(lipsync_extension_list[0]):
            # open a lipsync project
            doc.open(path)
        elif path.endswith(lipsync_extension_list[1]):
            # open a json based lipsync project
            doc.open_json(path)
        if doc.sound is None:
            print("Could not load Sound file.")
            print(doc.soundPath)
            return None
    else:
        # open an audio file
        doc.fps = int(config.value("LastFPS", 24))
        doc.open_audio(path)
        if doc.sound is None:
            doc = None
        else:
            if len(doc.project_node.children) < 1:
                doc.current_voice = LipsyncDoc.LipSyncObject(object_type="voice", name="Voice 1",
                                                             parent=doc.project_node)
            elif not doc.current_voice:
                doc.current_voice = doc.project_node.children[0]
            doc.auto_recognize_phoneme()
    return doc


class LipsyncFrame:
    def __init__(self):
        QtCore.QCoreApplication.setAttribute(QtCore.Qt.AA_ShareOpenGLContexts)

        self.app = QtWidgets.QApplication(sys.argv)
        self.translator = utilities.ApplicationTranslator()
        self.loader = None
        self.ui_file = None
        self.ui = None
        self.doc = None
        self.about_dlg = None
        self.settings_dlg = None
        self.ui_path = os.path.join(utilities.get_main_dir(), "rsrc", "papagayo-ng2.ui")
        self.main_window = self.load_ui_widget(self.ui_path)
        self.main_window.setWindowTitle("%s" % app_title)
        self.main_window.lip_sync_frame = self
        ini_path = os.path.join(utilities.get_app_data_path(), "settings.ini")
        self.config = QtCore.QSettings(ini_path, QtCore.QSettings.IniFormat)
        self.config.setFallbacksEnabled(False)  # File only, not registry or or.

        tree_style = r'''QTreeView::branch:has-siblings:!adjoins-item {
                             border-image: url(./rsrc/vline.png) 0;}               
                         QTreeView::branch:has-siblings:adjoins-item {
                             border-image: url(./rsrc/branch-more.png) 0;}
                         QTreeView::branch:!has-children:!has-siblings:adjoins-item {
                             border-image: url(./rsrc/branch-end.png) 0;}
                         QTreeView::branch:has-children:!has-siblings:closed,
                         QTreeView::branch:closed:has-children:has-siblings {
                                 border-image: none;
                                 image: url(./rsrc/branch-closed.png); }
                         QTreeView::branch:open:has-children:!has-siblings,
                         QTreeView::branch:open:has-children:has-siblings  {
                                 border-image: none;
                                 image: url(./rsrc/branch-open.png); }'''
        self.main_window.parent_tags.setStyleSheet(tree_style)

        # TODO: need a good description for this stuff
        mouth_list = list(self.main_window.mouth_view.mouths.keys())
        mouth_list.sort(key=sort_mouth_list_order)
        for mouth in mouth_list:
            self.main_window.mouth_choice.addItem(mouth)
        self.main_window.mouth_choice.setCurrentIndex(0)
        self.main_window.mouth_choice.current_mouth = self.main_window.mouth_choice.currentText()
        self.langman = LipsyncDoc.LanguageManager()
        self.langman.init_languages()
        language_list = list(self.langman.language_table.keys())
        language_list.sort()

        c = 0
        select = 0
        for language in language_list:
            self.main_window.language_choice.addItem(language)
            if language == "English":
                select = c
            c += 1
        self.main_window.language_choice.setCurrentIndex(select)

        # setup phonemeset initialisation here
        self.phonemeset = LipsyncDoc.PhonemeSet()
        for name in self.phonemeset.alternatives:
            self.main_window.phoneme_set.addItem(name)
        current_index = self.main_window.phoneme_set.findText(self.phonemeset.selected_set)
        self.main_window.phoneme_set.setCurrentIndex(current_index)

        c = 0
        select = 0
        for exporter in exporter_list:
            self.main_window.export_combo.addItem(exporter)
            if exporter == "MOHO":
                select = c
            c += 1
        self.main_window.export_combo.setCurrentIndex(select)
        self.app.aboutToQuit.connect(self.on_close)
        self.ignore_text_changes = False
        # This adds our statuses to the statusbar
        self.mainframe_statusbar_fields = [app_title, self.translator.translate("LipsyncFrame", "Stopped")]
        self.play_status = QtWidgets.QLabel()
        self.play_status.setText(self.mainframe_statusbar_fields[1])
        # An empty Label to add a separator
        self.sep_status = QtWidgets.QLabel()
        self.sep_status.setText(u"")
        self.main_window.statusbar.addPermanentWidget(self.sep_status)
        self.main_window.statusbar.addPermanentWidget(self.play_status)
        self.main_window.statusbar.showMessage(self.mainframe_statusbar_fields[0])
        self.status_progress = QtWidgets.QProgressBar()
        self.main_window.statusbar.addPermanentWidget(self.status_progress)
        self.status_progress.hide()
        # This is for the buttons to add and remove voices from the TabBar
        self.tab_widgets = QtWidgets.QWidget()
        self.tab_layout = QtWidgets.QHBoxLayout()
        self.tab_add_button = QtWidgets.QToolButton()
        self.tab_add_button.setText("+")
        self.tab_add_button.setIcon(QtGui.QIcon(QtGui.QPixmap(":/rsrc/icons/add-line.png")))
        self.tab_add_button.setToolTip(self.translator.translate("LipsyncFrame", "Add a new Voice."))
        self.tab_remove_button = QtWidgets.QToolButton()
        self.tab_remove_button.setText("-")
        self.tab_remove_button.setIcon(QtGui.QIcon(QtGui.QPixmap(":/rsrc/icons/delete-bin-line.png")))
        self.tab_remove_button.setToolTip(self.translator.translate("LipsyncFrame", "Delete Current Voice."))
        self.tab_layout.addWidget(self.tab_add_button)
        self.tab_layout.addWidget(self.tab_remove_button)
        self.tab_layout.setContentsMargins(0, 0, 0, 0)
        self.tab_layout.setSpacing(1)
        self.tab_widgets.setContentsMargins(0, 0, 0, 0)
        self.tab_widgets.setLayout(self.tab_layout)
        self.main_window.current_voice.setCornerWidget(self.tab_widgets)
        # Connect Events
        self.main_window.action_play.triggered.connect(self.on_play)
        self.main_window.action_stop.triggered.connect(self.on_stop)
        self.main_window.action_exit.triggered.connect(self.quit_application)
        self.main_window.action_open.triggered.connect(self.on_open)
        self.main_window.action_save.triggered.connect(self.on_save)
        self.main_window.action_save_as.triggered.connect(self.on_save_as)
        self.main_window.action_zoom_in.triggered.connect(self.main_window.waveform_view.on_zoom_in)
        self.main_window.action_zoom_out.triggered.connect(self.main_window.waveform_view.on_zoom_out)
        self.main_window.action_reset_zoom.triggered.connect(self.main_window.waveform_view.on_zoom_reset)
        self.main_window.action_settings.triggered.connect(self.show_settings)
        self.main_window.action_select_as_current_set.triggered.connect(self.set_current_phoneme_set)
        self.main_window.reload_dict_button.clicked.connect(self.on_reload_dictionary)
        self.main_window.waveform_view.horizontalScrollBar().sliderMoved.connect(
            self.main_window.waveform_view.on_slider_change)
        self.main_window.action_help_topics.triggered.connect(self.on_help)
        self.main_window.action_about_papagayo_ng.triggered.connect(self.on_about)
        self.main_window.export_combo.currentIndexChanged.connect(self.on_export_choice)
        self.main_window.voice_name_input.textChanged.connect(self.on_voice_name)
        self.main_window.export_button.clicked.connect(self.on_voice_export)
        self.main_window.breakdown_button.clicked.connect(self.on_voice_breakdown)
        self.main_window.voice_recognition_button.clicked.connect(self.on_voice_recognize)
        self.main_window.choose_imageset_button.clicked.connect(self.on_voice_image_choose)
        self.main_window.mouth_choice.currentIndexChanged.connect(self.on_mouth_choice)
        self.main_window.volume_slider.valueChanged.connect(self.change_volume)
        self.main_window.text_edit.textChanged.connect(self.on_voice_text)
        self.main_window.apply_fps.clicked.connect(self.apply_changed_fps)
        self.main_window.spread_out.clicked.connect(self.spread_out)
        self.main_window.add_tag.clicked.connect(self.add_tag)
        self.main_window.tag_entry.returnPressed.connect(self.add_tag)
        self.main_window.remove_tag.clicked.connect(self.remove_tag)
        self.main_window.apply_voice_change.clicked.connect(self.change_voice_for_selection)
        self.tab_add_button.clicked.connect(self.on_new_voice)
        self.tab_remove_button.clicked.connect(self.on_del_voice)
        self.main_window.current_voice.tabBar().currentChanged.connect(self.on_sel_voice_tab)
        self.recognize_menu = QtWidgets.QMenu()
        self.allo_select = self.recognize_menu.addAction("Allosaurus")
        self.rhubarb_select = self.recognize_menu.addAction("Rhubarb")
        self.allo_select.setCheckable(True)
        self.rhubarb_select.setCheckable(True)
        if self.config.value("/VoiceRecognition/recognizer", "Allosaurus") == "Allosaurus":
            self.allo_select.setChecked(True)
        elif self.config.value("/VoiceRecognition/recognizer", "Allosaurus") == "Rhubarb":
            self.rhubarb_select.setChecked(True)
        self.main_window.connect(self.allo_select, QtCore.SIGNAL("triggered()"),
                                 partial(self.select_voice_recognizer, "Allosaurus"))
        self.main_window.connect(self.rhubarb_select, QtCore.SIGNAL("triggered()"),
                                 partial(self.select_voice_recognizer, "Rhubarb"))
        self.main_window.voice_recognition_button.setMenu(self.recognize_menu)

        self.dropfilter = DropFilter()
        self.main_window.topLevelWidget().installEventFilter(self.dropfilter)

        self.main_window.action_allo_download.triggered.connect(lambda: self.start_download(self.download_allosaurus_model))
        self.main_window.action_rhubarb_download.triggered.connect(lambda: self.start_download(self.download_rhubarb))
        self.main_window.action_ffmpeg_download.triggered.connect(lambda: self.start_download(self.download_ffmpeg))
        if not utilities.ffmpeg_binaries_exists():
            self.ffmpeg_action = QtWidgets.QAction("Download FFmpeg")
            self.ffmpeg_action.triggered.connect(lambda: self.start_download(self.download_ffmpeg))
            self.main_window.menubar.addAction(self.ffmpeg_action)
        if not utilities.allosaurus_model_exists():
            self.model_action = QtWidgets.QAction("Download AI Model")
            self.model_action.triggered.connect(lambda: self.start_download(self.download_allosaurus_model))
            self.main_window.menubar.addAction(self.model_action)
        if not utilities.rhubarb_binaries_exists():
            self.rhubarb_action = QtWidgets.QAction("Download Rhubarb")
            self.rhubarb_action.triggered.connect(lambda: self.start_download(self.download_rhubarb))
            self.main_window.menubar.addAction(self.rhubarb_action)
        self.change_stylesheet()
        self.cur_frame = 0
        self.timer = None
        self.wv_height = 1
        self.old_height = self.wv_height
        self.zoom_factor = 1
        self.scroll_position = 0
        self.did_resize = False
        self.wv_pen = QtGui.QPen(QtCore.Qt.darkBlue)
        self.wv_brush = QtGui.QBrush(QtCore.Qt.blue)
        self.start_time = time.time()
        self.threadpool = QtCore.QThreadPool.globalInstance()

    def change_stylesheet(self):
        style_file_path = self.config.value("qss_file_path", "")
        if style_file_path:
            with open(style_file_path, "r") as style_file:
                self.app.setStyleSheet(style_file.read())
        else:
            self.app.setStyleSheet("")

    def download_general_finished(self):
        if utilities.allosaurus_model_exists():
            try:
                self.main_window.menubar.removeAction(self.model_action)
            except AttributeError:
                pass  # was already deleted
        if utilities.ffmpeg_binaries_exists():
            try:
                self.main_window.menubar.removeAction(self.ffmpeg_action)
            except AttributeError:
                pass  # was already deleted
        if utilities.rhubarb_binaries_exists():
            try:
                self.main_window.menubar.removeAction(self.rhubarb_action)
            except AttributeError:
                pass  # was already deleted
        self.status_progress.hide()

    def download_ffmpeg_finished(self):
        self.download_general_finished()
        dlg = QtWidgets.QMessageBox()
        dlg.setText("Download of FFMPEG is finished. \nPlease close and restart Papagayo-NG")
        dlg.setWindowTitle(app_title)
        dlg.setWindowIcon(self.main_window.windowIcon())
        dlg.setStandardButtons(QtWidgets.QMessageBox.Ok)
        dlg.setDefaultButton(QtWidgets.QMessageBox.Ok)
        dlg.setIcon(QtWidgets.QMessageBox.Information)
        dlg.exec_()

    def start_download(self, work_job):
        worker = utilities.Worker(work_job)
        if work_job == self.download_ffmpeg:
            worker.signals.finished.connect(self.download_ffmpeg_finished)
        elif work_job == self.download_allosaurus_model:
            worker.signals.finished.connect(self.download_general_finished)
        elif work_job == self.download_rhubarb:
            worker.signals.finished.connect(self.download_general_finished)
        worker.signals.progress.connect(self.status_bar_progress)
        self.status_progress.show()
        self.status_progress.setMaximum(100)
        self.threadpool.start(worker)

    def status_bar_progress(self, n):
        self.status_progress.setValue(n)
        QtCore.QCoreApplication.processEvents()

    def download_allosaurus_model(self, progress_callback):
        model_name = self.config.value("/VoiceRecognition/allosaurus_model", "latest")
        url = 'https://github.com/xinjli/allosaurus/releases/download/v1.0/' + model_name + '.tar.gz'
        model_dir = os.path.join(utilities.get_app_data_path(), "allosaurus_model")
        with urllib.request.urlopen(url) as req:
            length = req.getheader('content-length')
            block_size = 1000000
            if length:
                length = int(length)
                block_size = max(4096, length // 100)
            buffer_all = io.BytesIO()
            size = 0
            while True:
                buffer_now = req.read(block_size)
                if not buffer_now:
                    break
                buffer_all.write(buffer_now)
                size += len(buffer_now)
                if length:
                    percent = int((size / length) * 100)
                    progress_callback.emit(percent)
            if buffer_all:
                buffer_all.seek(0)
                files = tarfile.open(fileobj=buffer_all)
                files.extractall(str(model_dir))
                if os.path.exists(os.path.join(model_dir, "latest")):
                    shutil.rmtree(os.path.join(model_dir, "latest"))
                old_model_dirname = os.listdir(model_dir)[0]
                os.rename(os.path.join(model_dir, old_model_dirname), os.path.join(model_dir, "latest"))
        return

    def download_ffmpeg(self, progress_callback):
        ffmpeg_binary = "ffmpeg.exe"
        ffprobe_binary = "ffprobe.exe"
        ffmpeg_build_url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
        ffmpeg_json = json.loads(
            urllib.request.urlopen("https://api.github.com/repos/BtbN/FFmpeg-Builds/releases").read())
        for download in ffmpeg_json[0]["assets"]:
            if download["name"].endswith("win64-lgpl.zip"):
                ffmpeg_build_url = download["browser_download_url"]
        if platform.system() == "Darwin":
            ffmpeg_binary = "ffmpeg"
            ffprobe_binary = "ffprobe"
            ffmpeg_build_url = "https://evermeet.cx/ffmpeg/getrelease/zip"
        ffmpeg_path = os.path.join(utilities.get_app_data_path(), ffmpeg_binary)
        ffprobe_path = os.path.join(utilities.get_app_data_path(), ffprobe_binary)
        try:
            with urllib.request.urlopen(ffmpeg_build_url) as req:
                length = req.getheader('content-length')
                block_size = 1000000
                if length:
                    length = int(length)
                    block_size = max(4096, length // 100)
                buffer_all = io.BytesIO()
                size = 0
                while True:
                    buffer_now = req.read(block_size)
                    if not buffer_now:
                        break
                    buffer_all.write(buffer_now)
                    size += len(buffer_now)
                    if length:
                        percent = int((size / length) * 100)
                        progress_callback.emit(percent)
                if buffer_all:
                    ffmpeg_zip = ZipFile(buffer_all)
                    for zfile in ffmpeg_zip.filelist:
                        if ffmpeg_binary in zfile.filename:
                            ffmpeg_file_content = ffmpeg_zip.read(zfile.filename)
                            ffmpeg_file = open(ffmpeg_path, "wb")
                            ffmpeg_file.write(ffmpeg_file_content)
                            ffmpeg_file.close()
                        elif ffprobe_binary in zfile.filename:
                            ffprobe_file_content = ffmpeg_zip.read(zfile.filename)
                            ffprobe_file = open(ffprobe_path, "wb")
                            ffprobe_file.write(ffprobe_file_content)
                            ffprobe_file.close()
        except TimeoutError:
            # Download Failed
            pass
        return

    def download_rhubarb(self, progress_callback):
        binary = "/rhubarb/rhubarb.exe"
        release_url = ""
        if platform.system() == "Darwin":
            binary = "/rhubarb/rhubarb"
        github_url = "https://api.github.com/repos/DanielSWolf/rhubarb-lip-sync/releases"
        download_json = json.loads(urllib.request.urlopen(github_url).read())
        for download in download_json[0]["assets"]:
            if platform.system() == "Darwin":
                if download["name"].endswith("-osx.zip"):
                    release_url = download["browser_download_url"]
            else:
                if download["name"].endswith("-win32.zip"):
                    release_url = download["browser_download_url"]
        rhubarb_path = os.path.join(utilities.get_app_data_path(), binary)

        try:
            with urllib.request.urlopen(release_url) as req:
                length = req.getheader('content-length')
                block_size = 1000000
                if length:
                    length = int(length)
                    block_size = max(4096, length // 100)
                buffer_all = io.BytesIO()
                size = 0
                while True:
                    buffer_now = req.read(block_size)
                    if not buffer_now:
                        break
                    buffer_all.write(buffer_now)
                    size += len(buffer_now)
                    if length:
                        percent = int((size / length) * 100)
                        progress_callback.emit(percent)
                if buffer_all:
                    rhubarb_zip = ZipFile(buffer_all)
                    print(os.path.join(utilities.get_app_data_path(), "rhubarb"))
                    print(rhubarb_zip.filelist)
                    dirs = list(set([os.path.dirname(x) for x in rhubarb_zip.namelist()]))
                    main_dir = os.path.dirname([os.path.split(x)[0] for x in dirs][0])
                    if os.path.exists(os.path.join(utilities.get_app_data_path(), "rhubarb")):
                        shutil.rmtree(os.path.join(utilities.get_app_data_path(), "rhubarb"))
                    rhubarb_zip.extractall(os.path.join(utilities.get_app_data_path()))
                    os.rename(os.path.join(utilities.get_app_data_path(), main_dir),
                              os.path.join(utilities.get_app_data_path(), "rhubarb"))
        except TimeoutError:
            # Download Failed
            pass
        return

    def load_ui_widget(self, ui_filename, parent=None):
        self.loader = uic()
        file = QFile(ui_filename)
        file.open(QFile.ReadOnly)
        self.loader.registerCustomWidget(MouthView)
        self.loader.registerCustomWidget(WaveformView)
        self.ui = self.loader.load(file, parent)
        file.close()
        return self.ui

    def set_current_phoneme_set(self, event=None):
        if self.doc:
            phonemeset_name = self.main_window.phoneme_set.currentText()
            self.phonemeset.selected_set = self.phonemeset.load(phonemeset_name)

    def select_voice_recognizer(self, event=None):
        self.config.setValue("/VoiceRecognition/recognizer", event)
        if event == "Allosaurus":
            self.allo_select.setChecked(True)
            self.rhubarb_select.setChecked(False)
        elif event == "Rhubarb":
            self.allo_select.setChecked(False)
            self.rhubarb_select.setChecked(True)

    def change_voice_for_selection(self):
        # TODO: We don't handle overlapping objects yet!
        print("Currently Selected Object: " + self.main_window.waveform_view.currently_selected_object.title)
        print(
            "Corresponding LipsyncObject: " + str(vars(self.main_window.waveform_view.currently_selected_object.node)))
        print("Current Voice: " + self.doc.current_voice.name)
        print("New Voice: " + self.main_window.voice_for_selection.currentText())
        moving_object = self.main_window.waveform_view.currently_selected_object.node
        new_voice_parent = None
        for voice in self.doc.project_node.children:
            if voice.name == self.main_window.voice_for_selection.currentText():
                new_voice_parent = voice
        # Find existing parent object for selected one
        old_parent = moving_object.parent
        parent_instance = old_parent.object_type
        new_parent_object = None

        if parent_instance == "voice":
            new_parent_object = new_voice_parent
            moving_object.parent = new_parent_object
        else:
            for possible_parent in new_voice_parent.children:
                if parent_instance == "phrase":
                    if possible_parent.start_frame <= moving_object.start_frame and possible_parent.end_frame >= moving_object.end_frame:
                        new_parent_object = possible_parent
                elif parent_instance == "word":
                    for possible_word_parent in possible_parent.children:
                        if possible_word_parent.start_frame <= moving_object.start_frame <= possible_word_parent.end_frame:
                            new_parent_object = possible_word_parent
            if not new_parent_object:
                if parent_instance == "phrase":
                    new_temp_parent = LipsyncDoc.LipSyncObject(object_type="phrase", text=moving_object.text,
                                                               start_frame=moving_object.start_frame,
                                                               end_frame=moving_object.end_frame,
                                                               parent=new_voice_parent,
                                                               children=[moving_object])
                    moving_object.parent = new_temp_parent

                if parent_instance == "word":
                    new_temp_parent = LipsyncDoc.LipSyncObject(object_type="phrase", text=moving_object.text,
                                                               start_frame=moving_object.start_frame,
                                                               end_frame=moving_object.end_frame,
                                                               parent=new_voice_parent)
                    new_word_parent = LipsyncDoc.LipSyncObject(object_type="word", text=moving_object.text,
                                                               start_frame=moving_object.start_frame,
                                                               end_frame=moving_object.end_frame,
                                                               parent=new_temp_parent,
                                                               children=[moving_object])
                    moving_object.parent = new_word_parent

            else:
                moving_object.parent = new_parent_object

        if old_parent.object_type == "word":
            phrase_parent = old_parent.parent
            if not old_parent.children:
                old_parent.parent = None
            if not phrase_parent.children:
                phrase_parent.parent = None
        if old_parent.object_type == "phrase":
            if not old_parent.children:
                old_parent.parent = None
        self.main_window.waveform_view.set_document(self.doc, force=True, clear_scene=True)

    def add_tag(self):
        if self.main_window.tag_entry.text():
            self.main_window.list_of_tags.addItem(self.main_window.tag_entry.text())
            self.main_window.tag_entry.clear()
            temp_list = []
            for i in range(self.main_window.list_of_tags.count()):
                temp_list.append(self.main_window.list_of_tags.item(i).text())
            self.main_window.waveform_view.currently_selected_object.set_tags(temp_list)

    def remove_tag(self):
        self.main_window.list_of_tags.takeItem(self.main_window.list_of_tags.currentRow())
        temp_list = []
        for i in range(self.main_window.list_of_tags.count()):
            temp_list.append(self.main_window.list_of_tags.item(i).text())
        self.main_window.waveform_view.currently_selected_object.set_tags(temp_list)

    def spread_out(self):
        wfv = self.main_window.waveform_view
        top_nodes = self.doc.current_voice.children
        num_frames = wfv.waveform_polygon.polygon().boundingRect().width() / wfv.frame_width
        frames_per_top_level = num_frames / len(top_nodes)
        for num, top_node in enumerate(top_nodes):
            top_node.start_frame = round((num * frames_per_top_level) + int(bool(num)))
            top_node.end_frame = round((num * frames_per_top_level) + frames_per_top_level)
            top_node.move_button.after_reposition()
            top_node.move_button.reposition_descendants2(True)

    def apply_changed_fps(self):
        new_fps_value = self.main_window.fps_input.value()
        print('FPS changed to: {0}'.format(str(new_fps_value)))
        old_fps_value = self.doc.fps
        resize_multiplier = new_fps_value / old_fps_value
        if resize_multiplier != 1:
            self.doc.fps = new_fps_value
            for l_object in self.doc.project_node.descendants:
                l_object.fps = new_fps_value
            wfv = self.main_window.waveform_view
            wfv.samples_per_sec = self.doc.fps * wfv.samples_per_frame
            wfv.start_recalc(True)
            wfv.scene().setSceneRect(wfv.scene().sceneRect().x(), wfv.scene().sceneRect().y(),
                                     wfv.sceneRect().width() * resize_multiplier, wfv.scene().sceneRect().height())
            wfv.setSceneRect(wfv.scene().sceneRect())
            wfv.scroll_position *= resize_multiplier
            wfv.scroll_position = 0
            wfv.horizontalScrollBar().setValue(wfv.scroll_position)
            self.doc.project_node.sound_duration = int(self.doc.sound.Duration() * self.doc.fps)

    def on_voice_recognize(self):
        if self.doc and self.doc.sound:
            if len(self.doc.project_node.children) < 1:
                self.doc.current_voice = LipsyncDoc.LipSyncObject(object_type="voice", name="Voice 1",
                                                                  parent=self.doc.project_node)
            else:
                self.doc.current_voice.children = []
            self.doc.auto_recognize_phoneme(manual_invoke=True)
            self.main_window.waveform_view.set_document(self.doc, force=True, clear_scene=True)
            self.main_window.mouth_view.set_document(self.doc)

    def close_doc_ok(self):
        if self.doc is not None:
            if not self.doc.dirty:
                return True
            dlg = QtWidgets.QMessageBox()
            dlg.setText("Save changes to this project?")
            dlg.setStandardButtons(QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No)
            dlg.setDefaultButton(QtWidgets.QMessageBox.Yes)
            dlg.setIcon(QtWidgets.QMessageBox.Question)
            result = dlg.exec_()
            if result == QtWidgets.QMessageBox.Yes:
                self.on_save()
                if not self.doc.dirty:
                    self.config.setValue("LastFPS", str(self.doc.fps))
                    return True
                else:
                    return False
            elif result == QtWidgets.QMessageBox.No:
                self.config.setValue("LastFPS", str(self.doc.fps))
                return True
            elif result == QtWidgets.QMessageBox.Cancel:
                return False
        else:
            return True

    def on_open(self):
        if not self.close_doc_ok():
            return
        file_path, _ = QtWidgets.QFileDialog.getOpenFileName(self.main_window,
                                                             "Open Audio or {} File".format(app_title),
                                                             self.config.value("WorkingDir", utilities.get_main_dir()),
                                                             open_wildcard)
        if file_path:
            self.config.setValue("WorkingDir", os.path.dirname(file_path))
            self.open(file_path)

    def open(self, path):
        while self.main_window.current_voice.tabBar().count() > 1:
            self.main_window.current_voice.tabBar().removeTab(self.main_window.current_voice.tabBar().count() - 1)
        self.doc = LipsyncDoc.LipsyncDoc(self.langman, self)
        if path.endswith(lipsync_extension_list):
            if path.endswith(lipsync_extension_list[0]):
                # open a lipsync project
                self.doc.open(path)
            elif path.endswith(lipsync_extension_list[1]):
                # open a json based lipsync project
                self.doc.open_json(path)
            while self.doc.sound is None:
                # if no sound file found, then ask user to specify one
                dlg = QtWidgets.QMessageBox(self.main_window)
                dlg.setText('Please load correct audio file')
                dlg.setWindowTitle(app_title)
                dlg.setIcon(QtWidgets.QMessageBox.Warning)
                dlg.exec_()  # This should open it as a modal blocking window
                file_path = QtWidgets.QFileDialog.getOpenFileName(self.main_window,
                                                                  "Open Audio",
                                                                  self.config.value("WorkingDir",
                                                                                    utilities.get_main_dir()),
                                                                  audio_extensions)[0]
                if file_path:
                    self.doc.open_audio(file_path)
        else:
            # open an audio file
            self.doc.fps = int(self.config.value("LastFPS", 24))
            self.doc.open_audio(path)
            if self.doc.sound is None:
                self.doc = None
            else:
                if len(self.doc.project_node.children) < 1:
                    self.doc.current_voice = LipsyncDoc.LipSyncObject(object_type="voice", name="Voice 1",
                                                                      parent=self.doc.project_node)
                elif not self.doc.current_voice:
                    self.doc.current_voice = self.doc.project_node.children[0]
                self.doc.auto_recognize_phoneme()
                # check for a .trans file with the same name as the doc
                try:
                    txt_file = open("{}.trans".format(path[0].rsplit('.', 1)[0]), 'r')  # TODO: Check if path is correct
                    for line in txt_file:
                        self.main_window.current_voice.tabBar().addTab(QtGui.QStandardItem(line))
                except:
                    pass
        if self.doc is not None:
            self.main_window.setWindowTitle("{} [{}] - {}".format(self.doc.name, path, app_title))
            self.main_window.waveform_view.first_update = True
            self.main_window.waveform_view.set_document(self.doc, force=True, clear_scene=True)
            self.main_window.mouth_view.set_document(self.doc)
            # Reenable all disabled widgets TODO: Can likely be reduced
            self.main_window.vertical_layout_right.setEnabled(True)
            self.main_window.vertical_layout_left.setEnabled(True)
            self.main_window.volume_slider.setEnabled(True)
            self.main_window.volume_slider.setValue(int(self.config.value("volume", 50)))
            self.main_window.action_save.setEnabled(True)
            self.main_window.action_save_as.setEnabled(True)
            self.main_window.action_cut.setEnabled(True)
            self.main_window.action_cut.triggered.connect(self.on_del_object)
            self.main_window.menu_edit.setEnabled(True)
            self.main_window.choose_imageset_button.setEnabled(False)
            self.main_window.action_convert_phonemes.triggered.connect(self.doc.convert_to_phonemeset)
            if self.doc.sound is not None:
                self.main_window.action_play.setEnabled(True)
                # self.main_window.action_stop.setEnabled(True)
                self.main_window.action_zoom_in.setEnabled(True)
                self.main_window.action_zoom_out.setEnabled(True)
                self.main_window.action_reset_zoom.setEnabled(True)
            self.main_window.tag_list_group.setEnabled(False)
            list_of_voices = [voice.name for voice in self.doc.project_node.children]
            self.main_window.voice_for_selection.clear()
            self.main_window.voice_for_selection.addItems(list_of_voices)
            current_index = self.main_window.phoneme_set.findText(self.phonemeset.selected_set)
            self.main_window.phoneme_set.setCurrentIndex(current_index)

            first_entry = True
            for voice in self.doc.project_node.children:
                if not first_entry:
                    self.main_window.current_voice.tabBar().addTab(voice.name)
                else:
                    self.main_window.current_voice.tabBar().setTabText(0, voice.name)
                    first_entry = False
            self.main_window.fps_input.setValue(self.doc.fps)
            self.main_window.voice_name_input.setText(self.doc.current_voice.name)
            self.main_window.text_edit.setText(self.doc.current_voice.text)

            # reload dictionary
            self.on_reload_dictionary()
            self.doc.dirty = False

    def on_save(self):
        if self.doc is None:
            return
        if self.doc.path is None:
            self.on_save_as()
            return
        if self.doc.path.endswith(lipsync_extension_list[0]):
            self.doc.save(self.doc.path)
        elif self.doc.path.endswith(lipsync_extension_list[1]):
            self.doc.save2(self.doc.path)

    def on_save_as(self):
        if self.doc is None:
            return
        file_path, _ = QtWidgets.QFileDialog.getSaveFileName(self.main_window,
                                                             "Save {} File".format(app_title),
                                                             self.config.value("WorkingDir", utilities.get_main_dir()),
                                                             save_wildcard)
        if file_path:
            self.config.setValue("WorkingDir", os.path.dirname(file_path))
            if file_path.endswith(lipsync_extension_list[0]):
                self.doc.save(file_path)
            elif file_path.endswith(lipsync_extension_list[1]):
                self.doc.save2(file_path)
            self.main_window.setWindowTitle("{} [{}] - {}".format(self.doc.name, file_path, app_title))

    def on_close(self):
        if self.doc is not None:
            self.close_doc_ok()
            self.config.setValue("LastFPS", str(self.doc.fps))
            del self.doc
        self.doc = None
        self.main_window.waveform_view.first_update = True
        self.main_window.waveform_view.set_document(self.doc)
        # clear voice controls
        self.main_window.voice_name_input.clear()
        self.main_window.text_edit.clear()
        self.main_window.fps_input.clear()
        # disabling widgets
        self.main_window.vertical_layout_right.setEnabled(False)
        self.main_window.vertical_layout_left.setEnabled(False)
        self.main_window.volume_slider.setEnabled(False)
        self.main_window.action_save.setEnabled(False)
        self.main_window.action_save_as.setEnabled(False)
        self.main_window.menu_edit.setEnabled(False)
        self.main_window.action_play.setEnabled(False)
        self.main_window.action_stop.setEnabled(False)
        self.main_window.action_zoom_in.setEnabled(False)
        self.main_window.action_zoom_out.setEnabled(False)
        self.main_window.action_reset_zoom.setEnabled(False)

    def on_quit(self, event=None):
        self.on_close()
        self.close(True)

    def on_help(self, event=None):
        github_path = "https://github.com/morevnaproject/papagayo-ng/issues"
        test_path = "file://{}".format(r"D:\Program Files (x86)\Papagayo\help\index.html")
        real_path = "file://{}".format(os.path.join(utilities.get_main_dir(), "help", "index.html"))
        QtGui.QDesktopServices.openUrl(github_path)

    def on_about(self, event=None):
        self.about_dlg = AboutBox()
        self.about_dlg.main_window.show()

    def show_settings(self, event=None):
        self.settings_dlg = SettingsWindow()
        self.settings_dlg.main_window.show()
        self.settings_dlg.main_window.finished.connect(self.settings_closed)

    def settings_closed(self):
        if not utilities.ffmpeg_binaries_exists():
            self.ffmpeg_action = QtWidgets.QAction(self.translator.translate("LipsyncFrame", "Download FFmpeg"))
            self.ffmpeg_action.triggered.connect(lambda: self.start_download(self.download_ffmpeg))
            self.main_window.menubar.addAction(self.ffmpeg_action)
        if not utilities.allosaurus_model_exists():
            self.model_action = QtWidgets.QAction(self.apptranslator.translate("LipsyncFrame", "Download AI Model"))
            self.model_action.triggered.connect(lambda: self.start_download(self.download_allosaurus_model))
            self.main_window.menubar.addAction(self.model_action)
        if not utilities.rhubarb_binaries_exists():
            self.rhubarb_action = QtWidgets.QAction(self.apptranslator.translate("LipsyncFrame", "Download Rhubarb"))
            self.rhubarb_action.triggered.connect(lambda: self.start_download(self.download_rhubarb))
            self.main_window.menubar.addAction(self.rhubarb_action)
        self.main_window.waveform_view.set_document(self.doc, True, True)
        if self.config.value("/VoiceRecognition/recognizer", "Allosaurus") == "Allosaurus":
            self.allo_select.setChecked(True)
            self.rhubarb_select.setChecked(False)
        elif self.config.value("/VoiceRecognition/recognizer", "Allosaurus") == "Rhubarb":
            self.rhubarb_select.setChecked(True)
            self.allo_select.setChecked(False)
        self.change_stylesheet()

    def on_play(self, event=None):
        if (self.doc is not None) and (self.doc.sound is not None):
            self.cur_frame = -1
            self.main_window.action_play.setEnabled(False)
            self.main_window.action_stop.setEnabled(True)
            self.doc.sound.set_cur_time(0)
            self.doc.sound.play(False)
            self.timer = QtCore.QTimer()
            self.main_window.waveform_view.temp_play_marker.setVisible(True)
            self.timer.timeout.connect(self.on_play_tick)
            # self.connect(self.timer, None, self.on_play_tick)
            self.timer.start(250.0 / self.doc.fps)

    def on_stop(self, event=None):
        if (self.doc is not None) and (self.doc.sound is not None):
            self.doc.sound.stop()
            self.doc.sound.set_cur_time(0)
            self.main_window.waveform_view.temp_play_marker.setVisible(False)
            self.main_window.mouth_view.set_frame(0)
            self.main_window.waveform_view.set_frame(0)
            self.main_window.action_stop.setEnabled(False)
            self.main_window.action_play.setEnabled(True)
            self.main_window.statusbar.showMessage(self.translator.translate("LipsyncFrame", "Stopped"))
            self.main_window.waveform_view.horizontalScrollBar().setValue(
                self.main_window.waveform_view.scroll_position)
            self.main_window.waveform_view.update()
            QtCore.QCoreApplication.processEvents()

    def on_play_tick(self, event=None):
        if (self.doc is not None) and (self.doc.sound is not None):
            if self.doc.sound.is_playing():
                cur_frame = int(self.doc.sound.current_time() * self.doc.fps)
                if self.cur_frame != cur_frame:
                    self.cur_frame = cur_frame
                    self.main_window.mouth_view.set_frame(self.cur_frame)
                    self.main_window.waveform_view.set_frame(self.cur_frame)
                    try:
                        fps = 1.0 / (time.time() - self.start_time)
                    except ZeroDivisionError:
                        fps = 60
                    self.main_window.statusbar.showMessage(self.translator.translate("LipsyncFrame", "Frame: {:d} FPS: {:d}".format((cur_frame + 1), int(fps))))
                    self.main_window.waveform_view.scroll_position = self.main_window.waveform_view.horizontalScrollBar().value()
                    self.start_time = time.time()
            else:
                self.main_window.waveform_view.temp_play_marker.setVisible(False)
                self.on_stop()
                self.timer.stop()
                del self.timer

    def change_volume(self, e):
        if self.doc and self.doc.sound:
            self.doc.sound.set_volume(int(self.main_window.volume_slider.value()))
            self.config.setValue("volume", int(self.main_window.volume_slider.value()))

    def on_mouth_choice(self, event=None):
        self.main_window.mouth_view.current_mouth = self.main_window.mouth_choice.currentText()
        self.main_window.mouth_view.draw_me()

    def on_export_choice(self, event=None):
        if self.main_window.export_combo.currentText() == "Images":
            self.main_window.choose_imageset_button.setEnabled(True)
        else:
            self.main_window.choose_imageset_button.setEnabled(False)

    def on_voice_name(self, event=None):

        if (self.doc is not None) and (self.doc.current_voice is not None):
            self.doc.dirty = True
            self.doc.current_voice.name = self.main_window.voice_name_input.text()
            self.main_window.voice_name_input.setText(self.doc.current_voice.name)
            self.main_window.current_voice.tabBar().setTabText(self.main_window.current_voice.tabBar().currentIndex(),
                                                               self.doc.current_voice.name)
            self.main_window.waveform_view.first_update = True
            self.main_window.waveform_view.set_document(self.doc)

    def on_voice_text(self, event=None):
        if self.ignore_text_changes:
            return
        if (self.doc is not None) and (self.doc.current_voice is not None):
            self.doc.dirty = True
            self.doc.current_voice.text = self.main_window.text_edit.toPlainText()

    def on_voice_breakdown(self, event=None):
        if (self.doc is not None) and (self.doc.current_voice is not None):
            language = self.main_window.language_choice.currentText()
            phonemeset_name = self.main_window.phoneme_set.currentText()
            old_doc = self.doc.copy_to_dict()
            self.doc.dirty = True
            self.doc.current_voice.children = []
            return_value = self.doc.current_voice.run_breakdown(self.doc.soundDuration, self, language, self.langman, self.phonemeset)
            if return_value == -1:
                self.doc.open_from_dict(old_doc)
            self.phonemeset.selected_set = self.phonemeset.load(phonemeset_name)
            self.main_window.waveform_view.first_update = True
            self.ignore_text_changes = True
            self.main_window.text_edit.setText(self.doc.current_voice.text)
            self.ignore_text_changes = False
            self.main_window.waveform_view.set_document(self.doc, True, True)

    def on_voice_export(self, event=None):
        language = self.main_window.language_choice.currentText()
        if (self.doc is not None) and (self.doc.current_voice is not None):
            exporter = self.main_window.export_combo.currentText()
            message = ""
            default_file = ""
            wildcard = ""
            if exporter == "MOHO":
                message = self.translator.translate("LipsyncFrame", "Export Lipsync Data (MOHO)")
                default_file = "{}".format(self.doc.soundPath.rsplit('.', 1)[0]) + ".dat"
                wildcard = self.translator.translate("LipsyncFrame", "Moho switch files (*.dat)")
            elif exporter == "ALELO":
                fps = int(self.config.value("FPS", 24))
                if fps != 100:
                    dlg = QtWidgets.QMessageBox()
                    dlg.setText(self.translator.translate("LipsyncFrame", "FPS is NOT 100 continue? (You will have issues downstream.)"))
                    dlg.setStandardButtons(QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No)
                    dlg.setDefaultButton(QtWidgets.QMessageBox.Yes)
                    dlg.setIcon(QtWidgets.QMessageBox.Question)
                    result = dlg.exec_()
                    if result == QtWidgets.QMessageBox.Yes:
                        message = self.translator.translate("LipsyncFrame", "Export Lipsync Data (ALELO)")
                        default_file = "{}.txt".format(self.doc.soundPath.rsplit('.', 1)[0])
                        wildcard = self.translator.translate("LipsyncFrame", "Alelo timing files (*.txt)|*.txt")
                    elif result == QtWidgets.QMessageBox.No:
                        return
                    elif result == QtWidgets.QMessageBox.Cancel:
                        return
                else:
                    message = self.translator.translate("LipsyncFrame", "Export Lipsync Data (ALELO)")
                    default_file = "{}.txt".format(self.doc.soundPath.rsplit('.', 1)[0])
                    wildcard = self.translator.translate("LipsyncFrame", "Alelo timing files (*.txt)|*.txt")
            elif exporter == "Images":
                message = self.translator.translate("LipsyncFrame", "Export Image Strip")
                default_file = "{}".format(self.doc.soundPath.rsplit('.', 1)[0])
                wildcard = ""
            elif exporter == "JSON":
                message = self.translator.translate("LipsyncFrame", "Export JSON Object")
                default_file = "{}.json".format(self.doc.soundPath.rsplit('.', 1)[0])
                wildcard = self.translator.translate("LipsyncFrame", "JSON object files (*.json)|*.json")
            file_path, _ = QtWidgets.QFileDialog.getSaveFileName(self.main_window,
                                                                 message,
                                                                 default_file,
                                                                 wildcard,
                                                                 options=QtWidgets.QFileDialog.DontUseNativeDialog)
            if file_path:
                self.config.setValue("WorkingDir", os.path.dirname(file_path))
                if exporter == "MOHO":
                    self.doc.current_voice.export(file_path if ("." in file_path) else file_path + ".dat")
                elif exporter == "ALELO":
                    self.doc.current_voice.export_alelo(file_path, language, self.langman)
                elif exporter == "Images":
                    self.doc.current_voice.export_images(file_path, self.main_window.mouth_choice.currentText())
                elif exporter == "JSON":
                    self.doc.current_voice.export_json(file_path)

    def on_sel_voice_tab(self, e):
        if not self.doc:
            return
        prev_dirty = self.doc.dirty
        self.ignore_text_changes = True
        for voice in self.doc.project_node.children:
            if voice.name == self.main_window.current_voice.tabBar().tabText(
                    self.main_window.current_voice.tabBar().currentIndex()):
                self.doc.current_voice = voice

        self.main_window.list_of_tags.clear()
        self.main_window.tag_list_group.setEnabled(False)
        self.main_window.voice_name_input.setText(self.doc.current_voice.name)
        self.main_window.text_edit.setText(self.doc.current_voice.text)
        self.ignore_text_changes = False
        self.main_window.voice_for_selection.setCurrentIndex(self.main_window.current_voice.tabBar().currentIndex())
        self.main_window.waveform_view.set_document(self.doc, True)
        self.main_window.mouth_view.draw_me()
        self.doc.dirty = prev_dirty

    def on_new_voice(self, event=None):
        if not self.doc:
            return
        self.doc.dirty = True
        voice_exist_count = 1
        new_voice_name = "Voice {:d}".format(len(self.doc.project_node.children) + voice_exist_count)
        voice_name_exists = True
        while voice_name_exists:
            voice_name_exists = False
            new_voice_name = "Voice {:d}".format(len(self.doc.project_node.children) + voice_exist_count)
            for voice in self.doc.project_node.children:
                if voice.name == new_voice_name:
                    voice_name_exists = True
                    voice_exist_count += 1
                    break

        self.doc.current_voice = LipsyncDoc.LipSyncObject(object_type="voice", name=new_voice_name,
                                                          parent=self.doc.project_node)
        self.main_window.current_voice.tabBar().addTab(self.doc.current_voice.name)
        self.main_window.current_voice.tabBar().setCurrentIndex(self.main_window.current_voice.tabBar().count() - 1)
        self.ignore_text_changes = True
        self.main_window.voice_name_input.setText(self.doc.current_voice.name)
        self.main_window.text_edit.setText(self.doc.current_voice.text)
        self.ignore_text_changes = False
        list_of_voices = [voice.name for voice in self.doc.project_node.children]
        self.main_window.voice_for_selection.clear()
        self.main_window.voice_for_selection.addItems(list_of_voices)
        self.main_window.voice_for_selection.setCurrentIndex(self.main_window.current_voice.tabBar().count() - 1)
        self.main_window.waveform_view.set_document(self.doc, True)
        self.main_window.mouth_view.draw_me()

    def on_del_object(self):
        self.main_window.waveform_view.currently_selected_object.node.parent = None
        all_delete_items = []
        all_delete_items.extend(self.main_window.waveform_view.currently_selected_object.node.descendants)
        all_delete_items.append(self.main_window.waveform_view.currently_selected_object.node)
        for item in self.main_window.waveform_view.scene().items():
            if isinstance(item, QtWidgets.QGraphicsProxyWidget):
                for to_delete in all_delete_items:
                    if item.widget() == to_delete.move_button:
                        self.main_window.waveform_view.scene().removeItem(item)
        del self.main_window.waveform_view.currently_selected_object.node

    def on_del_voice(self, event=None):
        if (not self.doc) or (len(self.doc.project_node.children) == 1):
            return
        self.doc.dirty = True
        new_index = self.doc.project_node.children.index(self.doc.current_voice)
        if new_index > 0:
            new_index -= 1
        else:
            new_index = 0
        self.doc.current_voice.parent = None
        del self.doc.current_voice
        self.doc.current_voice = self.doc.project_node.children[new_index]
        self.main_window.voice_name_input.setText(self.doc.current_voice.name)
        self.main_window.text_edit.setText(self.doc.current_voice.text)
        self.main_window.current_voice.tabBar().removeTab(self.main_window.current_voice.tabBar().currentIndex())
        list_of_voices = [voice.name for voice in self.doc.project_node.children]
        self.main_window.voice_for_selection.clear()
        self.main_window.voice_for_selection.addItems(list_of_voices)
        self.main_window.voice_for_selection.setCurrentIndex(new_index)
        self.main_window.waveform_view.set_document(self.doc, True, True)
        self.main_window.mouth_view.draw_me()

    def on_voice_image_choose(self, event=None):
        language = self.main_window.language_choice.currentText()
        if (self.doc is not None) and (self.doc.current_voice is not None):
            voiceimage_path = QtWidgets.QFileDialog.getExistingDirectory(self.main_window,
                                                                         self.translator.translate("LipsyncFrame", "Choose Path for Images"),
                                                                         self.config.value("MouthDir",
                                                                                           os.path.join(os.path.dirname(
                                                                                               os.path.abspath(
                                                                                                   __file__)),
                                                                                               "rsrc",
                                                                                               r"mouths/")))
            if voiceimage_path:
                self.config.setValue("MouthDir", voiceimage_path)
                supported_imagetypes = QtGui.QImageReader.supportedImageFormats()
                for directory, dir_names, file_names in os.walk(voiceimage_path):
                    self.main_window.mouth_view.process_mouth_dir(directory, file_names, supported_imagetypes)
                mouth_list = list(self.main_window.mouth_view.mouths.keys())
                mouth_list.sort(key=sort_mouth_list_order)
                self.main_window.mouth_choice.clear()
                for mouth in mouth_list:
                    self.main_window.mouth_choice.addItem(mouth)
                self.main_window.mouth_choice.setCurrentIndex(
                    self.main_window.mouth_choice.findText(os.path.basename(voiceimage_path)))
                self.main_window.mouth_view.current_mouth = self.main_window.mouth_choice.currentText()

    def on_reload_dictionary(self, event=None):
        print("reload the dictionary")
        lang_config = self.doc.language_manager.language_table[self.main_window.language_choice.currentText()]
        self.doc.language_manager.load_language(lang_config, force=True)

    def quit_application(self):
        sys.exit(self.app.exec_())

# end of class LipsyncFrame


```

## Source file: de_de.ts

{
  "path": "de_de.ts",
  "blob_sha": "6f34dd706db9417b4dfca2baae6c1f27982e8de8",
  "sha256": "fe5fbe79555a7b12aac0a60fc6c0ecbb07c7d9c96652903f72f2c3195a2c0e00",
  "lines": 647,
  "bytes": 25668,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 5,
    "rig": 3,
    "lip": 24
  },
  "url": "https://github.com/morevnaproject-org/papagayo-ng/blob/17fa36a619817362f79726f85ef520489c01fe6a/de_de.ts",
  "function_signature_candidates": []
}

```text

<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE TS>
<TS version="2.1" language="de_DE">
<context>
    <name>AboutBox</name>
    <message>
        <location filename="rsrc/about_box.ui" line="38"/>
        <source>About Papagayo-NG</source>
        <translation>Über Papagayo-NG</translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="89"/>
        <source>Papagayo-NG</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="102"/>
        <source>Version: 1.6.1</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="193"/>
        <source>A lip-sync tool.</source>
        <translation>Ein Lippensynchronisations Werkzeug.</translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="196"/>
        <source>&lt;!DOCTYPE HTML PUBLIC &quot;-//W3C//DTD HTML 4.0//EN&quot; &quot;http://www.w3.org/TR/REC-html40/strict.dtd&quot;&gt;
&lt;html&gt;&lt;head&gt;&lt;meta name=&quot;qrichtext&quot; content=&quot;1&quot; /&gt;&lt;title&gt;A lip-sync tool.&lt;/title&gt;&lt;style type=&quot;text/css&quot;&gt;
p, li { white-space: pre-wrap; }
&lt;/style&gt;&lt;/head&gt;&lt;body style=&quot; font-family:&apos;MS Shell Dlg 2&apos;; font-size:8.25pt; font-weight:400; font-style:normal;&quot;&gt;
&lt;p align=&quot;center&quot; style=&quot; margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;&quot;&gt;A lip-sync tool.&lt;/p&gt;
&lt;p align=&quot;center&quot; style=&quot; margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;&quot;&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt;This software uses the Carnegie Mellon &lt;/span&gt;&lt;a href=&quot;http://www.speech.cs.cmu.edu/cgi-bin/cmudict&quot;&gt;&lt;span style=&quot; font-size:8pt; text-decoration: underline; color:#0000ff;&quot;&gt;Pronouncing Dictionary&lt;/span&gt;&lt;/a&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt;. &lt;br /&gt;Spanish pronunciation code provided by &lt;/span&gt;&lt;a href=&quot;https://users.monash.edu.au/~myless/catnap/index.html&quot;&gt;&lt;span style=&quot; font-size:8pt; text-decoration: underline; color:#0000ff;&quot;&gt;Myles Strous&lt;/span&gt;&lt;/a&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt; &lt;br /&gt;3D mouth shapes provided by &lt;/span&gt;&lt;a href=&quot;http://www.garycmartin.com&quot;&gt;&lt;span style=&quot; font-size:8pt; text-decoration: underline; color:#0000ff;&quot;&gt;Gary C. Martin&lt;/span&gt;&lt;/a&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt;. &lt;/span&gt;&lt;/p&gt;
&lt;p align=&quot;center&quot; style=&quot; margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;&quot;&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt;CMU 39 Phonemes and Mouth Shapes provided by &lt;/span&gt;&lt;a href=&quot;https://github.com/Hunanbean-Collective/&quot;&gt;&lt;span style=&quot; font-size:8pt; text-decoration: underline; color:#0000ff;&quot;&gt;Hunanbean&lt;/span&gt;&lt;/a&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt;.&lt;br /&gt;&lt;/span&gt;&lt;a href=&quot;gpl.html&quot;&gt;&lt;span style=&quot; font-size:8pt; text-decoration: underline; color:#0000ff;&quot;&gt;License&lt;/span&gt;&lt;/a&gt;&lt;span style=&quot; font-size:8pt;&quot;&gt; &lt;/span&gt;&lt;/p&gt;&lt;/body&gt;&lt;/html&gt;</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="235"/>
        <source>OK</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="112"/>
        <source>© 2005-2006 Mike Clifton</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/about_box.ui" line="122"/>
        <source>© 2013-2021 Morevna Project</source>
        <translation type="unfinished"></translation>
    </message>
</context>
<context>
    <name>LipsyncFrame</name>
    <message>
        <location filename="LipsyncFrameQt.py" line="892"/>
        <source>Stopped</source>
        <translation>Gestoppt</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="850"/>
        <source>Download FFmpeg</source>
        <translation></translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="854"/>
        <source>Download AI Model</source>
        <translation></translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="858"/>
        <source>Download Rhubarb</source>
        <translation></translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="978"/>
        <source>Export Lipsync Data (MOHO)</source>
        <translation>Exportiere Lipsync Daten (MOHO)</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="980"/>
        <source>Moho switch files (*.dat)</source>
        <translation>Moho Switchdateien (*.dat)</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="984"/>
        <source>FPS is NOT 100 continue? (You will have issues downstream.)</source>
        <translation>FPS ist NICHT 100, fortfahren? (Das wird im weiteren Verlauf Probleme erzeugen.)</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="999"/>
        <source>Export Lipsync Data (ALELO)</source>
        <translation>Exportiere Lipsync Daten (ALELO)</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="1001"/>
        <source>Alelo timing files (*.txt)|*.txt</source>
        <translation>Alelo Timingdateien (*.txt)|*.txt</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="1003"/>
        <source>Export Image Strip</source>
        <translation>Exportiere Bilderserie</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="1007"/>
        <source>Export JSON Object</source>
        <translation>Exportiere JSON Objekt</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="1009"/>
        <source>JSON object files (*.json)|*.json</source>
        <translation>JSON Objektdateien (*.json)|*.json</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="1114"/>
        <source>Choose Path for Images</source>
        <translation>Wähle Pfad für Bilderserie</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="227"/>
        <source>Add a new Voice.</source>
        <translation>Füge neue Stimme hinzu.</translation>
    </message>
    <message>
        <location filename="LipsyncFrameQt.py" line="231"/>
        <source>Delete Current Voice.</source>
        <translation>Lösche aktuelle Stimme.</translation>
    </message>
</context>
<context>
    <name>MouthView</name>
    <message>
        <location filename="MouthViewQt.py" line="95"/>
        <source>Missing Mouth: {0}</source>
        <translation>Fehlender Mund: {0}</translation>
    </message>
</context>
<context>
    <name>PronunciationDialog</name>
    <message>
        <location filename="PronunciationDialogQt.py" line="40"/>
        <source>Unknown Word</source>
        <translation>Unbekanntes Wort</translation>
    </message>
    <message>
        <location filename="PronunciationDialogQt.py" line="41"/>
        <source>Break down the word:</source>
        <translation>Schlüssele das Wort auf:</translation>
    </message>
</context>
<context>
    <name>Rhubarb</name>
    <message>
        <location filename="Rhubarb.py" line="79"/>
        <source>Rhubarb did not respond for more than 30 seconds.</source>
        <translation>Rhubarb reagiert nicht mehr seit mehr als 30 Sekunden.</translation>
    </message>
</context>
<context>
    <name>SettingsDialog</name>
    <message>
        <location filename="rsrc/settings.ui" line="41"/>
        <source>Settings</source>
        <translation>Einstellungen</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="68"/>
        <source>General</source>
        <translation>Generell</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="109"/>
        <source>Graphical</source>
        <translation>Graphisch</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="141"/>
        <source>Voicerecognition</source>
        <translation>Spracherkennung</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="179"/>
        <source>Misc</source>
        <translation>Verschiedenes</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="243"/>
        <source>FPS</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="266"/>
        <source>Hold Phonemes during playback</source>
        <translation>Phoneme beim Abspielen halten</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="230"/>
        <source>Enable</source>
        <translation>Aktiv</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="302"/>
        <source>Frame Text Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="319"/>
        <source>Word Fill Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="336"/>
        <source>Word Line Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="353"/>
        <source>Phrase Line Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="370"/>
        <source>Playback Marker Fill Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="387"/>
        <source>Frame Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="406"/>
        <source>Waveform Fill Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="426"/>
        <source>Phoneme Line Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="444"/>
        <source>Playback Marker Line Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="460"/>
        <source>QSS StyleSheet</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="477"/>
        <source>Waveform Line Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="497"/>
        <source>Phoneme Fill Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="514"/>
        <source>Phrase Fill Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="537"/>
        <source>Background Color</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="548"/>
        <source>Emission Multiplier</source>
        <translation>Emissions Multiplikator</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="608"/>
        <source>eng</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="555"/>
        <source>LanguageID</source>
        <translation>SprachenID</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="657"/>
        <source>Auto Run Voice Recognition</source>
        <translation>Automatische Spracherkennung</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="664"/>
        <source>Voice Recognizer</source>
        <translation>Spracherkennung</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="701"/>
        <source>Remove FFMpeg</source>
        <translation>Lösche FFMpeg</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="708"/>
        <source>Remove AI Model</source>
        <translation>Lösche AI Model</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="724"/>
        <source>Open in FileBrowser</source>
        <translation>Öffne im Dateibrowser</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="731"/>
        <source>AppData Path:</source>
        <translation>AppData Pfad:</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="755"/>
        <source>Reset Colors</source>
        <translation>Farben Zurücksetzen</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="762"/>
        <source>Delete All Settings</source>
        <translation>Alle Einstellungen Löschen</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="771"/>
        <source>Remove Rhubarb</source>
        <translation>Lösche Rhubarb</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="273"/>
        <source>Language (needs restart to apply)</source>
        <translation>Sprache (benötigt Neustart zur Anwendung)</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="588"/>
        <source>Allosaurus Settings</source>
        <translation>Allosaurus Einstellungen</translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="601"/>
        <source>latest</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="615"/>
        <source>Model Name</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/settings.ui" line="677"/>
        <source>General Settings</source>
        <translation>Generelle Einstellungen</translation>
    </message>
</context>
<context>
    <name>SettingsWindow</name>
    <message>
        <location filename="SettingsQT.py" line="77"/>
        <source>Graphics</source>
        <translation>Grafiken</translation>
    </message>
    <message>
        <location filename="SettingsQT.py" line="80"/>
        <source>General</source>
        <translation>Generell</translation>
    </message>
    <message>
        <location filename="SettingsQT.py" line="83"/>
        <source>Misc</source>
        <translation>Verschiedenes</translation>
    </message>
    <message>
        <location filename="SettingsQT.py" line="86"/>
        <source>Voice</source>
        <translation>Stimme</translation>
    </message>
</context>
<context>
    <name>WaveformView</name>
    <message>
        <location filename="WaveformViewRewrite.py" line="435"/>
        <source>Selected Object Tags</source>
        <translation>Gewählte Objekt Tags</translation>
    </message>
    <message>
        <location filename="WaveformViewRewrite.py" line="501"/>
        <source>Phrase: </source>
        <translation>Satz: </translation>
    </message>
    <message>
        <location filename="WaveformViewRewrite.py" line="488"/>
        <source>Word: </source>
        <translation>Wort: </translation>
    </message>
    <message>
        <location filename="WaveformViewRewrite.py" line="682"/>
        <source>Preparing Waveform: {0}%</source>
        <translation>Bereite Waveform vor: {0}%</translation>
    </message>
    <message>
        <location filename="WaveformViewRewrite.py" line="824"/>
        <source>Preparing Buttons: {0}%</source>
        <translation>Bereite Buttons vor: {0}%</translation>
    </message>
</context>
<context>
    <name>main_window</name>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="17"/>
        <source>MainWindow</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="122"/>
        <source>Voice Name:</source>
        <translation>Stimmen Name:</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="161"/>
        <source>Spoken Text:</source>
        <translation>Gesprochener Text:</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="173"/>
        <source>Phonetic Breakdown:</source>
        <translation>Phonetische Aufschlüsselung:</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="201"/>
        <source>Breakdown</source>
        <translation>Aufschlüsselung</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="212"/>
        <source>Reload Dictionary</source>
        <translation>Wörterbuch neu laden</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="223"/>
        <source>Spread Out</source>
        <translation>Verteilen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="247"/>
        <source>Voice Recognition</source>
        <translation>Spracherkennung</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="304"/>
        <source>Export</source>
        <translation>Exportieren</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="315"/>
        <source>Choose Image Set...</source>
        <translation>Wähle Bilder Set...</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="395"/>
        <source>FPS:</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="405"/>
        <source>Apply</source>
        <translation>Anwenden</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="505"/>
        <source>Selected Object Tags</source>
        <translation>Gewählte Objekt Tags:</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="606"/>
        <source>Add</source>
        <translation>Hinzufügen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="619"/>
        <source>Remove</source>
        <translation>Entfernen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="635"/>
        <source>Change Voice</source>
        <translation>Wechsle Stimme</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="684"/>
        <source>&amp;File</source>
        <translation>&amp;Datei</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="696"/>
        <source>&amp;Edit</source>
        <translation>&amp;Bearbeiten</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="706"/>
        <source>&amp;Help</source>
        <translation>&amp;Hilfe</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="713"/>
        <source>&amp;Conversion</source>
        <translation>&amp;Umwandlung</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="760"/>
        <source>toolBar</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="784"/>
        <source>&amp;Help Topics</source>
        <translation>&amp;Hilfe Themen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="793"/>
        <source>&amp;About Papagayo-ng</source>
        <translation>&amp;Über Papagayo-NG</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="802"/>
        <source>&amp;Open</source>
        <translation>&amp;Öffnen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="814"/>
        <source>&amp;Save</source>
        <translation>&amp;Speichern</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="826"/>
        <source>&amp;Save As...</source>
        <translation>&amp;Speichern unter...</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="835"/>
        <source>&amp;Exit</source>
        <translation>&amp;Schliessen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="847"/>
        <source>Undo</source>
        <translation>Rückgängig</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="859"/>
        <source>&amp;Cut</source>
        <translation>&amp;Ausschneiden</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="871"/>
        <source>&amp;Copy</source>
        <translation>&amp;Kopieren</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="883"/>
        <source>&amp;Paste</source>
        <translation>&amp;Einfügen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="895"/>
        <source>Play</source>
        <translation>Abspielen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="907"/>
        <source>Stop</source>
        <translation></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="919"/>
        <source>ZoomIn</source>
        <translation>Rein Zoomen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="931"/>
        <source>ZoomOut</source>
        <translation>Raus Zoomen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="943"/>
        <source>ResetZoom</source>
        <translation>Zoom Wiederherstellen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="952"/>
        <source>&amp;Preferences</source>
        <translation>&amp;Einstellungen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="958"/>
        <source>Preferences</source>
        <translation>Einstellungen</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="963"/>
        <source>Convert Phonemes</source>
        <translation>Phoneme Umwandeln</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="966"/>
        <source>Convert Phonemes to currently selected Phoneme Set.</source>
        <translation>Wandle Phoneme in ausgewähltes Phonemset um.</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="971"/>
        <source>Select as Current Set</source>
        <translation>Wähle als aktuelles Set</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="974"/>
        <source>This sets the currently selected Phoneme Set for the Project.</source>
        <translation>Dies setzt das aktuell gewählte Phonemset für das aktuelle Project.</translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="720"/>
        <source>Download</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="724"/>
        <source>Allosaurus</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="730"/>
        <source>Rhubarb</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="736"/>
        <source>FFmpeg</source>
        <translation type="unfinished"></translation>
    </message>
    <message>
        <location filename="rsrc/papagayo-ng2.ui" line="1001"/>
        <source>Update/Download</source>
        <translation type="unfinished"></translation>
    </message>
</context>
</TS>


```
