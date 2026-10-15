# Facial rendering source scan: Chryfi/LectorLips

Repository: https://github.com/Chryfi/LectorLips

Pinned source revision: b12332c1091ff5e090bc35a3464904e1f708e3fd

Discovery topic: mouth animation

Description: A cool script that converts Adobe Character Animator mouth animation into Blockbuster mod Sequencer Morph NBT data.

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"lip": ["LectorLipsLogo.png"]}

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "8faed860fd226a0ef360909fa93b46c8b884c926",
  "sha256": "d5f6e52cb87b7ff60813bcbaca0570945695486104457daf1368207e681c1f57",
  "lines": 43,
  "bytes": 1386,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 2,
    "rig": 1,
    "lip": 1
  },
  "url": "https://github.com/Chryfi/LectorLips/blob/b12332c1091ff5e090bc35a3464904e1f708e3fd/__init__.py",
  "function_signature_candidates": []
}

```text

import sys
import traceback
from commands import InvalidCommandArguments, InvalidCommand, CommandException, commands

__author__ = "Christian F. (known as Chryfi)"
__copyright__ = "Copyright 2022, LectorLips"
__credits__ = ["Christian F. (known as Chryfi)", "Florian Torgau"]
__maintainer__ = "Christian F. (known as Chryfi)"
__status__ = "Release"
__version__ = "1.1"

if __name__ == "__main__":
    if len(sys.argv) > 0:
        commandClass = None

        try:
            sys.argv.pop(0)

            if len(sys.argv) == 0 or (len(sys.argv) > 0 and commands.get(sys.argv[0]) is None):
                raise InvalidCommand("No existing command was provided. Type in -help for a list of commands")

            commandClass = commands.get(sys.argv[0])

            sys.argv.pop(0)

            command = commandClass(sys.argv)

            command.execute()
        except Exception as e:
            if "-debug" in sys.argv:
                print(traceback.format_exc())

            print(e)

            if isinstance(e, InvalidCommandArguments) and commandClass is not None:
                print("\n")
                print(commandClass.get_documentation())
            elif not isinstance(e, InvalidCommand) and not isinstance(e, CommandException) and not "-debug" in sys.argv:
                print("\nType in -debug for the stacktrace.")

            quit(1)

        quit(0)

```

## Source file: commands.py

{
  "path": "commands.py",
  "blob_sha": "20fed65488c75f4d27f7ae07bd35a18a9fecb550",
  "sha256": "cd8baa54c637fc9b8e449e437a73ba0f59ccf3bf6f576e5fc292708c5f290e71",
  "lines": 307,
  "bytes": 10621,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 1,
    "mouth": 5,
    "gum": 10,
    "rig": 1,
    "lip": 1
  },
  "url": "https://github.com/Chryfi/LectorLips/blob/b12332c1091ff5e090bc35a3464904e1f708e3fd/commands.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def execute(self) -> None:"
    },
    {
      "line": 35,
      "text": "def parse_args(self, args: list):"
    },
    {
      "line": 39,
      "text": "def get_documentation(self) -> str:"
    },
    {
      "line": 43,
      "text": "def clean_args(self, args: list) -> list:"
    },
    {
      "line": 55,
      "text": "def __init__(self, args):"
    },
    {
      "line": 58,
      "text": "def execute(self) -> None:"
    },
    {
      "line": 69,
      "text": "def parse_args(self, args: list) -> bool:"
    },
    {
      "line": 73,
      "text": "def get_documentation() -> str:"
    },
    {
      "line": 82,
      "text": "def __init__(self, args: list):"
    },
    {
      "line": 89,
      "text": "def execute(self) -> None:"
    },
    {
      "line": 125,
      "text": "def convert_to_sequencer_morph(self, keyframes: list, fps: int) -> str:"
    },
    {
      "line": 154,
      "text": "def load_viseme_mapping(self) -> dict:"
    },
    {
      "line": 164,
      "text": "def parse_keyframes(self, file) -> list:"
    },
    {
      "line": 179,
      "text": "def parse_args(self, args: list):"
    },
    {
      "line": 208,
      "text": "def get_documentation() -> str:"
    },
    {
      "line": 215,
      "text": "def check_file_path(self, file_path: str):"
    },
    {
      "line": 223,
      "text": "def check_texture_path(self, file_path: str) -> str:"
    },
    {
      "line": 237,
      "text": "def __init__(self, args: list):"
    },
    {
      "line": 242,
      "text": "def execute(self) -> None:"
    },
    {
      "line": 252,
      "text": "def parse_args(self, args: list):"
    },
    {
      "line": 274,
      "text": "def get_documentation() -> str:"
    },
    {
      "line": 284,
      "text": "def write(mapping: list):"
    },
    {
      "line": 288,
      "text": "def write(mapping: list, file_name: str):"
    },
    {
      "line": 296,
      "text": "def read():"
    },
    {
      "line": 300,
      "text": "def read(file_name: str) -> list:"
    }
  ]
}

```text

from abc import ABC, abstractmethod
from os.path import exists
import os
from datetime import datetime
import json

__author__ = "Christian F. (known as Chryfi)"
__copyright__ = "Copyright 2022, LectorLips"
__credits__ = ["Christian F. (known as Chryfi)", "Florian Torgau"]
__maintainer__ = "Christian F. (known as Chryfi)"
__status__ = "Production"
__version__ = "1.1"


class InvalidCommand(Exception):
    pass


class CommandException(Exception):
    pass


class InvalidCommandArguments(Exception):
    pass


class Command(ABC):
    standardParams = {"help" : "-help", "debug" : "-debug"}

    @abstractmethod
    def execute(self) -> None:
        pass

    @abstractmethod
    def parse_args(self, args: list):
        pass

    @abstractmethod
    def get_documentation(self) -> str:
        pass

    # Cleanup arguments from the standard parameters that are not specific to a Command
    def clean_args(self, args: list) -> list:
        args_cleaned = args.copy()

        for key, param in self.standardParams.items():
            if param in args_cleaned:
                args_cleaned.remove(param)

        return args_cleaned


class HelpCommand(Command):

    def __init__(self, args):
        pass

    def execute(self) -> None:
        print("List of possible commands:")

        for command in commands.keys():
            print(command)

        print("\nList of parameters that can be used with every command:")

        for key, param in self.standardParams.items():
            print(param)

    def parse_args(self, args: list) -> bool:
        pass

    @staticmethod
    def get_documentation() -> str:
        return ""


class CreateSequencerNBT(Command):
    minRequiredArgs = 2
    maxArgs = 4
    standardEndTickDuration = 100

    def __init__(self, args: list):
        self.endTickDuration = self.standardEndTickDuration
        self.filePath = None
        self.texturePath = None
        self.visemePath = VisemeMappingFile.fileName
        self.parse_args(args)

    def execute(self) -> None:
        fps_marker = "Units Per Second"
        time_remap_marker = "Time Remap"

        fps = None
        keyframes = None

        with open(self.filePath, 'r') as file:
            while True:
                line = file.readline()

                if not line:
                    break

                line = line.strip()

                if fps_marker in line:
                    fps = float(line.replace(fps_marker, "").strip())
                elif time_remap_marker in line:
                    file.readline()  # skip marker line
                    keyframes = self.parse_keyframes(file)

                    break

        if fps is None:
            raise CommandException("The \"" + fps_marker + "\" were not found in the keyframe file!")
        if keyframes is None:
            raise CommandException("The keyframes were not found or are empty. The keyframes are usually located at \"" + time_remap_marker + "\"")

        sequencerNBT = self.convert_to_sequencer_morph(keyframes, fps)

        with open(datetime.now().strftime("%Y-%m-%d_%H.%M.%S") + "_output.txt", 'x') as f:
            f.write(sequencerNBT)

        print("File was successfully created.")

    def convert_to_sequencer_morph(self, keyframes: list, fps: int) -> str:
        viseme_mapping = self.load_viseme_mapping()

        nbt_start = "{List:["
        nbt_end = "],KeepProgress:1b,Name:\"sequencer\",Offset:[0.0f,0.0f,0.0f]}"

        if keyframes is not None:
            for i in range(len(keyframes)):
                keyframe = keyframes[i]
                frame = float(keyframe[0])
                mouth = int(keyframe[1])

                if mouth >= len(viseme_mapping):
                    print("Skipped frame " + str(frame) + " with mouth " + str(mouth) + ". Mouth out of viseme mapping range!")
                    continue

                if i < len(keyframes) - 1:
                    next_keyframe = keyframes[i + 1]
                    next_frame = float(next_keyframe[0])
                    tick_duration = (next_frame - frame) / fps * 20
                else:
                    tick_duration = self.endTickDuration

                morph_nbt = "Morph: { Texture:\"" + self.texturePath + viseme_mapping.get(str(mouth)) + "\", Name:\"blockbuster.image\"}"

                nbt_start += "{Random:0.0f, SetDuration:1b, " + morph_nbt + ", Duration:" + str(tick_duration) + "f, EndPoint:0b}" + ("," if i < len(keyframes) - 1 else "")

        return nbt_start + nbt_end

    def load_viseme_mapping(self) -> dict:
        mapping = VisemeMappingFile.read(self.visemePath)

        if mapping is None:
            raise CommandException("The viseme mapping configuration file does not exist yet. You have to create the file using the command or do it manually.")
        elif len(mapping) == 0:
            raise CommandException("Something went wrong during reading the viseme mapping file! The mapping is empty.")

        return mapping

    def parse_keyframes(self, file) -> list:
        keyframes = []

        while True:
            line = file.readline().strip()

            if not line:
                break

            keyframe = line.split()

            keyframes.append(keyframe)

        return keyframes

    def parse_args(self, args: list):
        if self.standardParams.get("help") in args:
            print(self.get_documentation())

            quit(0)

        args_cleaned = self.clean_args(args)

        if len(args_cleaned) < self.minRequiredArgs:
            raise InvalidCommandArguments("Not all required arguments were provided")

        if len(args_cleaned) > self.maxArgs:
            raise InvalidCommandArguments("Unknown arguments: " + "".join(("\"" + str(args_cleaned[i]) + "\"" + (", " if i < len(args_cleaned) - 1 else "")) for i in range(len(args_cleaned))))

        self.check_file_path(args_cleaned[0])

        self.filePath = args_cleaned[0]
        self.texturePath = self.check_texture_path(args_cleaned[1])

        if len(args_cleaned) > 2:
            if not args_cleaned[2].isnumeric():
                raise InvalidCommandArguments("Third argument should be a number!")
            else:
                self.endTickDuration = args_cleaned[2]

        if len(args_cleaned) > 3:
            self.visemePath = args_cleaned[3]

    @staticmethod
    def get_documentation() -> str:
        return "Command arguments:\n<file path>  <image morph texture path>  optional: <end tick duration> <viseme_mapping_filename>\n\n" \
               "Documentation:\n<file path> the global or relative file path to the keyframes data\n" \
               "<image morph texture path> the Blockbuster file path to the image folder containing the mouth images for example \"b.a:skins/mouths/\"\n" \
               "Optional:\n<end tick duration> the duration of the last morph. Default is " + str(CreateSequencerNBT.standardEndTickDuration) + "\n" \
               "<viseme_mapping_filename> the name of a new viseme mapping file."

    def check_file_path(self, file_path: str):
        if not str(file_path).endswith(".txt"):
            raise CommandException("Keyframe file path does not end with a .txt! The keyframe data should be pasted into a .txt file.")

        if not exists(file_path):
            raise CommandException("Keyframe file does not exist at the specified path!")

    #return corrected file path
    def check_texture_path(self, file_path: str) -> str:
        if file_path.find(':') == -1:
            raise CommandException("Texture file path seems to be incorrect. It misses ':', for example 'blockbuster:textures/..'!")

        if not file_path.endswith("/"):
            file_path += "/"

        return file_path


class CreateVisemeMapping(Command):
    minArgs = 15
    maxArgs = 16

    def __init__(self, args: list):
        self.args = []
        self.visemeName = VisemeMappingFile.fileName
        self.parse_args(args)

    def execute(self) -> None:
        viseme_mapping = {}

        for i in range(15):
            viseme_mapping[str(i)] = self.args[i]

        VisemeMappingFile.write(viseme_mapping, self.visemeName)

        print("Successfully created json viseme mapping configuration file.")

    def parse_args(self, args: list):
        if self.standardParams.get("help") in args:
            print(self.get_documentation())

            quit(0)

        self.args = self.clean_args(args)

        if len(self.args) < self.minArgs:
            raise InvalidCommandArguments("Missing required " + str(self.minArgs) + " arguments.")

        if len(self.args) > self.maxArgs:
            raise InvalidCommandArguments("More than " + str(self.maxArgs) + " arguments were given.")

        if len(self.args) > 15:
            if not self.args[15].endswith(".json"):
                raise InvalidCommandArguments("The viseme mapping filename does not end with .json!")
            else:
                self.visemeName = self.args[15]


    @staticmethod
    def get_documentation() -> str:
        return "Command arguments:\n<list of 15 image file names>  optional: <viseme_mapping_filename>\n\n"\
               "<list of 15 image file names> type in 15 image file names in ascending order matching the viseme mapping.\n" \
               "Optional:\n<viseme_mapping_filename> if you want multiple viseme mapping files, you can specify your " \
               "own filename. The filename should end with .json"


class VisemeMappingFile:
    fileName = "viseme_mapping.json"
    @staticmethod
    def write(mapping: list):
        VisemeMappingFile.write(mapping, VisemeMappingFile.fileName)

    @staticmethod
    def write(mapping: list, file_name: str):
        if exists(file_name):
            os.remove(file_name)

        with open(file_name, 'x') as file:
            file.write(json.dumps(mapping, indent=4))

    @staticmethod
    def read():
        VisemeMappingFile.read(VisemeMappingFile.fileName)

    @staticmethod
    def read(file_name: str) -> list:
        if not exists(file_name):
            return None

        with open(file_name, 'r') as file:
            return json.loads(file.read())

commands = {"-help": globals()["HelpCommand"], "-create_sequencer": globals()["CreateSequencerNBT"], "-create_viseme_mapping": globals()["CreateVisemeMapping"]}


```
