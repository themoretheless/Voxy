# Facial rendering source scan: alexismorin/OpenFace-FACS-Unity-Facial-Animator

Repository: https://github.com/alexismorin/OpenFace-FACS-Unity-Facial-Animator

Pinned source revision: be922424e2ba2f0ca2a45f91f265fa095b54b4fa

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/LICENSE", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/LICENSE.meta"]

Topic paths: {"blendshape": ["modules/process_facstoblend/au2blendshapes_mb.py", "modules/process_facstoblend/blendshapes_MB.json"], "facs": ["modules/facsvatarzeromq.py", "modules/input_facsfromcsv/Dockerfile", "modules/input_facsfromcsv/__init__.py", "modules/input_facsfromcsv/main.py", "modules/input_facsfromcsv/openfacefiltercsv.py", "modules/output_facstofile/main.py", "modules/process_facsdnnfacs/AU_json.json", "modules/process_facsdnnfacs/main.py", "modules/process_facstoblend/Dockerfile", "modules/process_facstoblend/au2blendshapes_mb.py", "modules/process_facstoblend/au_json/AU01.json", "modules/process_facstoblend/au_json/AU02.json", "modules/process_facstoblend/au_json/AU04.json", "modules/process_facstoblend/au_json/AU05.json", "modules/process_facstoblend/au_json/AU06.json", "modules/process_facstoblend/au_json/AU09.json", "modules/process_facstoblend/au_json/AU10.json", "modules/process_facstoblend/au_json/AU12.json", "modules/process_facstoblend/au_json/AU14.json", "modules/process_facstoblend/au_json/AU15.json", "modules/process_facstoblend/au_json/AU17.json", "modules/process_facstoblend/au_json/AU20.json", "modules/process_facstoblend/au_json/AU23.json", "modules/process_facstoblend/au_json/AU25.json", "modules/process_facstoblend/au_json/AU26.json", "modules/process_facstoblend/au_json/AU45.json", "modules/process_facstoblend/au_json/AU61.json", "modules/process_facstoblend/au_json/AU62.json", "modules/process_facstoblend/au_json/AU63.json", "modules/process_facstoblend/au_json/AU64.json"], "face": ["Unity/Assets/OpenFace.meta", "Unity/Assets/OpenFace/Character.meta", "Unity/Assets/OpenFace/Character/Black.mat", "Unity/Assets/OpenFace/Character/Black.mat.meta", "Unity/Assets/OpenFace/Character/Brown.mat", "Unity/Assets/OpenFace/Character/Brown.mat.meta", "Unity/Assets/OpenFace/Character/White.mat", "Unity/Assets/OpenFace/Character/White.mat.meta", "Unity/Assets/OpenFace/Character/face.fbx", "Unity/Assets/OpenFace/Character/face.fbx.meta", "Unity/Assets/OpenFace/Demo.unity", "Unity/Assets/OpenFace/Demo.unity.meta", "Unity/Assets/OpenFace/Packages.meta", "Unity/Assets/OpenFace/Packages/AsyncIO.dll", "Unity/Assets/OpenFace/Packages/AsyncIO.dll.meta", "Unity/Assets/OpenFace/Packages/NetMQ.dll", "Unity/Assets/OpenFace/Packages/NetMQ.dll.meta", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher.meta", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/LICENSE", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/LICENSE.meta", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/README.md", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/README.md.meta", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/UnityMainThreadDispatcher.cs", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/UnityMainThreadDispatcher.cs.meta", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/UnityMainThreadDispatcher.prefab", "Unity/Assets/OpenFace/Packages/UnityMainThreadDispatcher/UnityMainThreadDispatcher.prefab.meta", "Unity/Assets/OpenFace/Scripts.meta", "Unity/Assets/OpenFace/Scripts/ActionUnitAnimator.cs", "Unity/Assets/OpenFace/Scripts/ActionUnitAnimator.cs.meta", "Unity/Assets/OpenFace/Scripts/OpenFaceAnimator.cs"]}

## Source file: modules/input_facsfromcsv/openfacefiltercsv.py

{
  "path": "modules/input_facsfromcsv/openfacefiltercsv.py",
  "blob_sha": "e93d0d214ddf0fd3443e74bee5a12740cd9247c3",
  "sha256": "89547c0c61964c3bc261e9cebd9356c3fe7020375417ad3f933a1438c1a368cc",
  "lines": 103,
  "bytes": 3447,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 1,
    "face": 3,
    "lip": 1
  },
  "url": "https://github.com/alexismorin/OpenFace-FACS-Unity-Facial-Animator/blob/be922424e2ba2f0ca2a45f91f265fa095b54b4fa/modules/input_facsfromcsv/openfacefiltercsv.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def __init__(self, col_keep=None):  # , dir_processed, col_keep, dir_timestamp"
    },
    {
      "line": 37,
      "text": "def clean_header_space(self):"
    },
    {
      "line": 41,
      "text": "def clean_unsuccessful(self):"
    },
    {
      "line": 46,
      "text": "def clean_columns(self):"
    },
    {
      "line": 56,
      "text": "def match_index_frame(self):"
    },
    {
      "line": 60,
      "text": "def reset_au_interval(self):"
    },
    {
      "line": 65,
      "text": "def csv_save(self, csv_clean):"
    },
    {
      "line": 77,
      "text": "def clean_controller(self, csv_raw, csv_folder_clean):"
    }
  ]
}

```text

"""Cleans an OpenFace .csv file

- Removes unnecessary columns
- Removes low confidence rows
- Normalizes AU interval to 0-1 from 0-5

Saves cleaned csv in path/to/folder_clean
"""

# Copyright (c) Stef van der Struijk.
# License: GNU Lesser General Public License


import os
from pathlib import Path
import pandas as pd


# clean csv output of OpenFace, remove irrelevant columns
class FilterCSV:
    def __init__(self, col_keep=None):  # , dir_processed, col_keep, dir_timestamp
        """Transform csv to Panda dataframe which can be accessed by other functions

        :param col_keep: columns to keep in .csv file [AU*_r, pose_R.*, gaze_angle*]; None for all

        If cleaned csv exist, skip cleaning and load directly as dataframe
        """

        if col_keep is None:
            col_keep = ['AU.*_r', 'pose_R.*', 'gaze_angle*']

        self.df_csv = None
        # Action Units and head rotation
        self.col_keep = col_keep  # , 'gaze_angle*'

    # removes space in header, e.g. ' confidence' --> 'confidence'
    def clean_header_space(self):
        self.df_csv.columns = self.df_csv.columns.str.strip()

    # remove rows where success == 0 and confidence < 0.8
    def clean_unsuccessful(self):
        self.df_csv.drop(self.df_csv[(self.df_csv.success == 0) | (self.df_csv.confidence < 0.8)].index,
                        inplace=True)

    # remove unnecessary columns
    def clean_columns(self):
        if len(self.col_keep) >= 1:
            # doesn't include AU28_c (lip suck), which has no AU28_r
            reg = "(frame)|(timestamp)|(confidence)|(success)|{}" \
                .format("|".join("({})".format(c) for c in self.col_keep))
        else:
            reg = "(frame)|(timestamp)|(confidence)"
        # print("regex col filter: {}".format(reg))
        self.df_csv = self.df_csv.filter(regex=reg)

    def match_index_frame(self):
        self.df_csv['frame'] = self.df_csv.index

    # get AU values and set interval from 0-5 to 0-1
    def reset_au_interval(self):
        c = self.df_csv.columns.str.contains("AU.*_r")
        self.df_csv.loc[:, c] /= 5

    # save cleaned dataframe as csv
    def csv_save(self, csv_clean):
        """Dataframe to csv

        :param csv_clean: Path object for saving cleaned csv
        """

        # create dir preprocessed if not existing, Python 3.2+
        os.makedirs(csv_clean.parents[0], exist_ok=True)

        self.df_csv.to_csv(csv_clean, index=False)

        # calls all cleaning + save functions
    def clean_controller(self, csv_raw, csv_folder_clean):
        """ Manages cleaning of raw openface csv file

        :param csv_raw: path to raw file
        :param csv_folder_clean: path to folder with cleaned files
        """

        print("Cleaning: {} and save to {}".format(csv_raw, csv_folder_clean))
        self.df_csv = pd.read_csv(csv_raw)

        # removes space in header, e.g. ' confidence' --> 'confidence'
        self.clean_header_space()

        # remove rows with bad AU tracking TODO do something with failed tracking
        #self.clean_unsuccessful()

        # dataframe with only c_keep columns
        self.clean_columns()

        # Set frame -1 to match index
        self.match_index_frame()

        # divide AU*_r by 5 (range from 0-1 instead of 0-5)
        self.reset_au_interval()

        # save cleaned dataframe; add csv file name to clean path
        self.csv_save(csv_folder_clean / csv_raw.name)


```

## Source file: modules/process_facstoblend/au2blendshapes_mb.py

{
  "path": "modules/process_facstoblend/au2blendshapes_mb.py",
  "blob_sha": "21cf2488e230d0b62303c9cc502c203441faee1d",
  "sha256": "cf2bf9f3440db7dea50a3470fde8ff49826a885494173514e42583b3c6de38df",
  "lines": 128,
  "bytes": 4922,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 28,
    "facs": 4,
    "face": 3
  },
  "url": "https://github.com/alexismorin/OpenFace-FACS-Unity-Facial-Animator/blob/be922424e2ba2f0ca2a45f91f265fa095b54b4fa/modules/process_facstoblend/au2blendshapes_mb.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, au_folder=\"AU_json\"):"
    },
    {
      "line": 23,
      "text": "def load_json(self, json_dir):"
    },
    {
      "line": 35,
      "text": "def load_au(self, filepath):"
    },
    {
      "line": 45,
      "text": "def load_blendshape_dict(self):"
    },
    {
      "line": 55,
      "text": "def json_blendshape_matcher(self, json_MB):"
    },
    {
      "line": 85,
      "text": "def calc_blendshapes(self, facs_dict):"
    },
    {
      "line": 123,
      "text": "def output_blendshapes(self, facs_dict):"
    }
  ]
}

```text

import os
from os.path import join
import sys
import json
from collections import defaultdict


# change AU values from OpenFace to Blend Shape (Shape Key) values understood by Unity / Blender / Unreal
# for characters created in Blender + Manual Bastioni addon
class AUtoBlendShapes:
    def __init__(self, au_folder="AU_json"):
        # call function for dictionary of values for changing AU to blendshapes
        self.au_dict = self.load_json(au_folder)
        print(self.au_dict)

        # frame tracker for index in dataframe
        self.frame_tracker = 0

        # AU to blendshapes
        self.blendshape_dict_new = json.load(open(join('blendshapes_MB.json'), 'r'))

    # open all AU .json file in directory
    def load_json(self, json_dir):
        au_dict = {}
        for au_filename in os.listdir(json_dir):
            au_filepath = join(json_dir, au_filename)
            e_item, extension = os.path.splitext(au_filename)
            if "json" in extension:
                #self.expressions_labels.add(e_item)
                # add "_min" or "_max" to blenshape + double value (because it uses 0.5 as cutoff)
                au_dict[e_item] = self.json_blendshape_matcher(self.load_au(au_filepath))
        return au_dict

    # extract "structural" data for how to change blend shapes per AU
    def load_au(self, filepath):
        try:
            json_file = json.load(open(filepath, 'r'))
        except:
            sys.exit(f"json file corrupted: {filepath}")

        #if "structural" in json_file:
        return json_file["structural"]

    # json dict of blendshape names
    def load_blendshape_dict(self):
        # load json as dict
        # if not self.blendshape_dict:
        #     self.blendshape_dict_new = json.load(open(join('blendshapes.json'), 'r'))

        # reset dict to default 0; make thread safe (untested) deep copy
        self.blendshape_dict = json.loads(json.dumps(self.blendshape_dict_new))

    # add "_min" or "_max" to blendshape + value conversion (because it uses 0.5 as cutoff for min/max)
    # based on Manuel Bastioni v1.6.0
    def json_blendshape_matcher(self, json_MB):
        # json_MB: JSON based on Manuel Bastioni used in Blender

        # create new dict
        dict_blendshape = {}

        # add values to new dict; based on manuelbastioni/animationengine.py
        for name, value in json_MB.items():
            if value < 0.5:
                name = name + "_min"
                value = (0.5 - value) * 2
            else:
                name = name + "_max"
                value = (value - 0.5) * 2

            # make dict less ugly
            dict_blendshape[name] = round(value, 5)

        return dict_blendshape

    # https://stackoverflow.com/a/35689816/3399066
    # sum 2 dicts
    # def npe_method(self, tests):
    #     ret = defaultdict(int)
    #     for d in tests:
    #         for k, v in d.items():
    #             ret[k] += v
    #     return dict(ret)

    # receive AU values and change to blendshapes
    def calc_blendshapes(self, facs_dict):
        # get a clean blendshape dict
        self.load_blendshape_dict()

        # loop over AU values from OpenFace data frame
        for au, au_v in facs_dict.items():
            # only loop over AU values
            if au.startswith('AU'):
                # check if we have AU convert file
                if au in self.au_dict:
                    # don't waste computing power when nothing changes
                    # TODO make if statement about difference previous AU
                    if au_v > 0.001:
                        # remove '_r'
                        #au = au[:-2]
                        #print(au, au_v)

                        # loop over all blendshapes related to that AU
                        #print(self.au_dict[au])
                        # TODO error: KeyError: 'AU'
                        for exp, exp_v in self.au_dict[au].items():
                            #print(exp, exp_v)
                            # multiply AU value with au_dict to get blendshape values and
                            # add blendshape values to total blendshape_dict
                            self.blendshape_dict[exp] += round(exp_v * au_v, 5)
                            #print(self.blendshape_dict[exp])

                else:
                    print("No json file found for {}".format(au))

        # TODO resolve _min _max (only 1 can have value, so subtract from each other)

        # TODO limit blendshape value to max 1
        #min(current_val + sk_value, 1.0)

        # TODO? return None for non-changed values

    # iterate over AU data extracted by OpenFace
    def output_blendshapes(self, facs_dict):
        print("Frame: {}".format(self.frame_tracker))
        self.calc_blendshapes(facs_dict)  # next(self.df_au.itertuples())
        self.frame_tracker += 1

        return self.blendshape_dict  # self.blend_val

```

## Source file: modules/facsvatarzeromq.py

{
  "path": "modules/facsvatarzeromq.py",
  "blob_sha": "c17207d44fcd4a7dba6152fa2a917e23b7bdc9b3",
  "sha256": "ecf93ec024671fb896edfc2d1936ed0ec75706a10ee5498fddc4d2c9fbfd4161",
  "lines": 370,
  "bytes": 15220,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 1,
    "facs": 8,
    "rig": 1
  },
  "url": "https://github.com/alexismorin/OpenFace-FACS-Unity-Facial-Animator/blob/be922424e2ba2f0ca2a45f91f265fa095b54b4fa/modules/facsvatarzeromq.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def __init__(self, module_id=\"module\", loglevel=\"INFO\","
    },
    {
      "line": 139,
      "text": "def zeromq_context(self, ip, port, socket_type, bind):"
    },
    {
      "line": 160,
      "text": "def start(self, async_func_list=None):"
    },
    {
      "line": 194,
      "text": "def __init__(self, socket, key='', csv_filename=\"test.csv\"):"
    },
    {
      "line": 223,
      "text": "async def pub(self, data, key=None):"
    },
    {
      "line": 281,
      "text": "async def sub(self, raw=False):"
    },
    {
      "line": 322,
      "text": "def sub_topic(self, key=None, unsub_all=False):"
    },
    {
      "line": 340,
      "text": "def write_to_csv(self, data):"
    },
    {
      "line": 355,
      "text": "def time_hns():"
    }
  ]
}

```text

"""FACSvatar base classes"""

# Copyright (c) Stef van der Struijk.
# License: GNU Lesser General Public License

import os
import sys
import inspect
from pathlib import Path, PurePath
import traceback
import logging
from abc import ABC, abstractmethod
import asyncio
import zmq.asyncio
from zmq.asyncio import Context
import time
import json
import csv


# setup ZeroMQ publisher / subscriber
class FACSvatarZeroMQ(abstractmethod(ABC)):
    """Base class for initializing FACSvatar ZeroMQ sockets"""

    def __init__(self, module_id="module", loglevel="INFO",
                 pub_ip='127.0.0.1', pub_port=None, pub_key='', pub_bind=True,
                 sub_ip='127.0.0.1', sub_port=None, sub_key='', sub_bind=False,
                 deal_ip='127.0.0.1', deal_port=None, deal_key='', deal_topic='', deal_bind=False,
                 deal2_ip='127.0.0.1', deal2_port=None, deal2_key='', deal2_topic='', deal2_bind=False,
                 deal3_ip='127.0.0.1', deal3_port=None, deal3_key='', deal3_topic='', deal3_bind=False,
                 rout_ip='127.0.0.1', rout_port=None, rout_bind=True,
                 **misc):
        """Sets-up a socket bound/connected to an url

        module_id: string that identifies the module calling this class
        loglevel: specifies how detailed terminal and logfile output should be
        xxx_ip: ip of publisher/subscriber/dealer/router
        xxx_port: port of publisher/subscriber/dealer/router
        xxx_key: key for filtering out messages (leave empty to receive all) (pub/sub only)
        xxx_bind: True for bind (only 1 socket can bind to 1 address) or false for connect (many can connect)
        """

        # get filepath of the file calling this script
        frame = inspect.stack()[1]
        module = inspect.getmodule(frame[0])
        file_path = Path(module.sys.argv[0]).absolute()
        # print(f"path: {file_path.parts[-2]}\n\n")
        # use module name in logfile
        logfile = Path(file_path.parent, "logging", "logging_%s.log" % module_id)
        # make logging dir if not exist
        logfile.parent.mkdir(exist_ok=True)

        # set logging level; TODO logger per module / pattern instead of root
        numeric_level = getattr(logging, loglevel.upper(), None)
        if not isinstance(numeric_level, int):
            raise ValueError('Invalid log level: %s' % loglevel)
        logging.basicConfig(level=numeric_level)
        logger = logging.getLogger()
        fh = logging.FileHandler(filename=logfile, mode='w')
        fh.setFormatter(logging.Formatter("%(asctime)s:%(levelname)s:%(message)s"))
        logger.addHandler(fh)

        # get ZeroMQ version
        logging.info("Current libzmq version is %s" % zmq.zmq_version())
        logging.info("Current  pyzmq version is %s" % zmq.pyzmq_version())

        self.pub_socket = None
        self.sub_socket = None

        # set-up publish socket only if a port is given
        if pub_port:
            logging.info("Publisher port is specified")
            self.pub_socket = FACSvatarSocket(self.zeromq_context(pub_ip, pub_port, zmq.PUB, pub_bind),
                                              pub_key, "pub.csv")
            logging.info("Publisher socket set-up complete")
        else:
            logging.info("pub_port not specified, not setting-up publisher")

        # set-up subscriber socket only if a port is given
        if sub_port:
            logging.info("Subscriber port is specified")
            # self.sub_socket = self.zeromq_context(sub_ip, sub_port, zmq.SUB, sub_bind)
            self.sub_key = sub_key  # TODO use sub_topic() instead
            # self.sub_socket.setsockopt(zmq.SUBSCRIBE, self.sub_key.encode('ascii'))

            self.sub_socket = FACSvatarSocket(self.zeromq_context(sub_ip, sub_port, zmq.SUB, sub_bind),
                                              sub_key, "sub.csv")
            self.sub_socket.sub_topic()
            logging.info("Subscriber socket set-up complete")
        else:
            logging.info("sub_port not specified, not setting-up subscriber")

        # set-up dealer socket only if a port is given
        if deal_port:
            logging.info("Dealer port is specified")
            self.deal_socket = self.zeromq_context(deal_ip, deal_port, zmq.DEALER, deal_bind)
            self.deal_socket.setsockopt(zmq.IDENTITY, deal_key.encode('ascii'))
            # add variable with key f
            self.deal_topic = deal_topic
            logging.info("Dealer socket set-up complete")
        else:
            logging.info("deal_port not specified, not setting-up dealer")

        # set-up dealer socket only if a port is given; TODO better solution for multiple same sockets
        if deal2_port:
            logging.info("Dealer port 2 is specified")
            self.deal2_socket = self.zeromq_context(deal2_ip, deal2_port, zmq.DEALER, deal2_bind)
            self.deal2_socket.setsockopt(zmq.IDENTITY, deal2_key.encode('ascii'))
            # add variable with key f
            self.deal2_topic = deal2_topic
            logging.info("Dealer 2 socket set-up complete")
        else:
            logging.info("deal2_port not specified, not setting-up dealer")

        # set-up dealer socket only if a port is given; TODO better solution for multiple same sockets
        if deal3_port:
            logging.info("Dealer port 3 is specified")
            self.deal3_socket = self.zeromq_context(deal3_ip, deal3_port, zmq.DEALER, deal3_bind)
            self.deal3_socket.setsockopt(zmq.IDENTITY, deal3_key.encode('ascii'))
            # add variable with key f
            self.deal3_topic = deal3_topic
            logging.info("Dealer 3 socket set-up complete")
        else:
            logging.info("deal3_port not specified, not setting-up dealer")

        # set-up router socket only if a port is given
        if rout_port:
            logging.info("Router port is specified")
            self.rout_socket = self.zeromq_context(rout_ip, rout_port, zmq.ROUTER, rout_bind)
            logging.info("Router socket set-up complete")
        else:
            logging.info("rout_port not specified, not setting-up router")

        logging.info("ZeroMQ sockets successfully set-up\n")

        # extra named arguments
        self.misc = misc

    def zeromq_context(self, ip, port, socket_type, bind):
        """Returns a bound / connected ZeroMQ socket through tcp with given ip and port

        ip+port: tcp://{ip}:{port}
        socket_type: ZeroMQ socket type; e.g. zmq.PUB / zmq.SUB
        bind: True for bind (only 1 socket can bind to 1 address) or false for connect (many can connect)
        """

        url = "tcp://{}:{}".format(ip, port)
        logging.info("Creating ZeroMQ context on: {}".format(url))
        ctx = Context.instance()
        socket = ctx.socket(socket_type)
        if bind:
            socket.bind(url)
            logging.info("Bind to {} successful".format(url))
        else:
            socket.connect(url)
            logging.info("Connect to {} successful".format(url))

        return socket

    def start(self, async_func_list=None):
        """Starts asynchronously any given async function"""

        # activate publishers / subscribers
        if async_func_list:
            # capture ZeroMQ errors; ZeroMQ using asyncio doesn't print out errors
            # TODO working properly?
            try:
                asyncio.get_event_loop().run_until_complete(asyncio.wait(
                    [func() for func in async_func_list]
                ))
            except Exception as e:
                logging.critical("Error with async function")
                # print(e)
                logging.error(traceback.format_exc())

            finally:
                # TODO disconnect pub/sub
                pass

        else:
            logging.info("No functions given, nothing to start")


# TODO option to enable / disable debugging
class FACSvatarSocket:
    """Simplify sending messages according to FACSvatar format

    Automatically attempts to encode / decode, and data from/to JSON if not done so yet
    `key` should be string or already encoded as 'ascii'
    `data` should be a dict, JSON formatted string or already encoded as 'utf8'
    Access ZeroMQ socket functionality through `FACSvatarSocket.socket.xxx` (e.g. `.setsockopt()`
    """

    def __init__(self, socket, key='', csv_filename="test.csv"):
        self.socket = socket
        self.key = key.encode('ascii')
        # time previous message send or useful for start-up time before first message
        self.pub_timestamp_old = time_hns()
        self.sub_time_received = 0
        self.frame_count = -1

        csv_dir = "logging"
        csv_filename = "timestamps_" + csv_filename
        csv_location = os.path.join(csv_dir, csv_filename)

        os.makedirs(csv_dir, exist_ok=True)

        # increase file name number if file exist
        logging.info("Write timestamps to: {}".format(csv_location))
        # while os.path.exists(csv_location):
        #     csv_location = csv_location[:-5] + str(int(csv_location[-5]) + 1) + csv_location[-4:]

        # delete existing csv file
        if os.path.exists(csv_location):
            os.remove(csv_location)

        with open(csv_location, 'a') as file:
            writer = csv.writer(file, delimiter=',')
            writer.writerow(["msg", "time_prev", "time_now"])

        self.csv_location = csv_location

    async def pub(self, data, key=None):
        """ Publish data async

        :param data: dict, JSON formatted string or bytes encoded as 'utf-8'
        :param key: string or bytes encoded as 'ascii'; send specified key instead of default
        :return:
        """

        if not key:
            key = self.key
        else:
            # check if not yet encoded
            if not isinstance(key, bytes):
                key = key.encode('ascii')

        # not b''
        if data:
            # if data not yet bytes
            if not isinstance(data, bytes):
                # if data != JSON:
                if not isinstance(data, str):
                    # change to json string
                    data = json.dumps(data)

                # change from string to bytes
                data = data.encode('utf-8')

            # timestamp = self.time_now()
            timestamp = time_hns()
            timestamp_enc = str(timestamp).encode('ascii')

            # print(f"Key type: {type(key)}\nTimestamp type: {type(timestamp)}\nData type: {type(data)}")
            await self.socket.send_multipart([key, timestamp_enc, data])

            # check if in DEBUG mode for logging performance; TODO seperate?
            if logging.getLogger().isEnabledFor(logging.DEBUG):
                logging.debug("PUB: Time prev msg:\t\t\t{}".format(self.pub_timestamp_old))
                logging.debug("PUB: Time publishing:\t\t\t{}".format(timestamp))
                logging.debug("PUB: Difference prev msg nanosec:\t{}".format(timestamp - self.pub_timestamp_old))
                logging.debug("PUB: Difference prev msg milliseconds:\t{}".format((timestamp - self.pub_timestamp_old) / 1000000))

                # NOT WORKING due to pub and sub not using same instance of this class
                # assume module: receive msg sub --> process --> pub when sub_time_received != 0
                # print(self.sub_time_received)
                # if self.sub_time_received:
                #     print("Module performance diff sub-pub:\t{}".format(timestamp - self.sub_time_received))

                self.write_to_csv([self.pub_timestamp_old, timestamp])

                self.pub_timestamp_old = timestamp

        # send message with no timestamp or data
        else:
            logging.info("PUB: Data finished")
            await self.socket.send_multipart([key, b'', b''])

        print()

    async def sub(self, raw=False):
        """ Wait for subscribed data async

        :param raw: if False, decode (and json.loads()) key, timestamp and data; if True, return as-is (bytes)
        :return: key, timestamp, data
        """

        key, timestamp, data = await self.socket.recv_multipart()

        # not received finish message b''
        if timestamp:
            timestamp_decoded = int(timestamp.decode('ascii'))

            # check if in DEBUG mode for logging performance; TODO seperate?
            if logging.getLogger().isEnabledFor(logging.DEBUG):
                # self.sub_time_received = self.time_now()
                self.sub_time_received = time_hns()
                time_difference = self.sub_time_received - timestamp_decoded
                logging.debug("SUB: Time data published:\t\t{}\nSUB: Time subscribed data received:\t{}\n"
                      "SUB: Difference nanoseconds:\t\t{}\nSUB: Difference milliseconds:\t\t{}"
                      .format(timestamp_decoded, self.sub_time_received, time_difference, time_difference / 1000000))

                self.write_to_csv([timestamp_decoded, self.sub_time_received])

            # byte data
            if raw:
                return key, timestamp, data
            # decode data
            else:
                data = data.decode('utf-8')
                # # check not empty byte string b''
                # if data:
                data = json.loads(data)

                return key.decode('ascii'), timestamp_decoded, data

        else:
            # key, '', ''
            return key.decode('ascii'), timestamp.decode('ascii'), data.decode('utf-8')

    # TODO check if single key or list
    def sub_topic(self, key=None, unsub_all=False):
        """ Subscribe to a (new) key

        :param key: set new self.key and subscribe; if None, only subscribe to self.key
        :param unsub_all: if True, unsubscribe all keys
        """

        # TODO multiple sub keys
        if unsub_all:
            self.socket.setsockopt(zmq.UNSUBSCRIBE, self.key)

        if isinstance(key, str):
            self.key = key.encode('ascii')
        elif isinstance(key, bytes):
            self.key = key

        self.socket.setsockopt(zmq.SUBSCRIBE, self.key)

    def write_to_csv(self, data):
        logging.debug("Storing time data to csv")

        with open(self.csv_location, 'a') as file:
            writer = csv.writer(file, delimiter=',')
            # add frame no to beginning of data list
            data.insert(0, self.frame_count)
            # write data to csv
            writer.writerow(data)
            # writer.writerow(self.frame_count, data)

        self.frame_count += 1


# @staticmethod
def time_hns():
    """Return time in 100 nanoseconds (more precise with >= python 3.7)"""

    # Python 3.7 or newer use nanoseconds
    if (sys.version_info.major == 3 and sys.version_info.minor >= 7) or sys.version_info.major >= 4:
        # 100 nanoseconds / 0.1 microseconds
        time_now = int(time.time_ns() / 100)
    else:
        # timestamp = int(time.time() * 1000)
        # timestamp = timestamp.to_bytes((timestamp.bit_length() + 7) // 8, byteorder='big')

        # match 100 nanoseconds / 0.1 microseconds
        time_now = int(time.time() * 10000000)  # time.time()
        # time_now = time.time()

    return time_now


```

## Source file: Unity/Assets/OpenFace/Scripts/OpenFaceAnimator.cs

{
  "path": "Unity/Assets/OpenFace/Scripts/OpenFaceAnimator.cs",
  "blob_sha": "1e35711d5107ad61803a92ff6b0b8be40db773ce",
  "sha256": "00dd4ae7632e7ad8aa49a4d15c48313793afc89be897a08a7a28c2577fdc4313",
  "lines": 181,
  "bytes": 6407,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1,
    "blend_shape": 4,
    "facs": 7,
    "face": 7
  },
  "url": "https://github.com/alexismorin/OpenFace-FACS-Unity-Facial-Animator/blob/be922424e2ba2f0ca2a45f91f265fa095b54b4fa/Unity/Assets/OpenFace/Scripts/OpenFaceAnimator.cs",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "public delegate void MessageDelegate (List<string> msg_list);"
    },
    {
      "line": 30,
      "text": "private void ListenerWork () {"
    },
    {
      "line": 100,
      "text": "public void Update () {"
    },
    {
      "line": 117,
      "text": "public void Start () {"
    },
    {
      "line": 123,
      "text": "public void Stop () {"
    },
    {
      "line": 147,
      "text": "private void HandleMessage (List<string> msg_list) {"
    },
    {
      "line": 166,
      "text": "private void Start () {"
    },
    {
      "line": 174,
      "text": "private void Update () {"
    },
    {
      "line": 178,
      "text": "private void OnDestroy () {"
    }
  ]
}

```text

﻿//https://github.com/valkjsaaa/Unity-ZeroMQ-Example
using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using NetMQ;
using NetMQ.Sockets;
using Newtonsoft.Json.Linq;
using UnityEngine;

// connect with ZeroMQ
public class OpenFaceAnimatorListener {
    public string sub_to_ip;
    public string sub_to_port;
    private readonly Thread _listenerWorker;
    private bool _listenerCancelled;
    public delegate void MessageDelegate (List<string> msg_list);
    private readonly MessageDelegate _messageDelegate;
    private readonly ConcurrentQueue<List<string>> _messageQueue = new ConcurrentQueue<List<string>> ();
    //private string csv_folder = "Assets/Logging/";
    private string csv_path = "Assets/Logging/unity_timestamps_sub.csv";
    private StreamWriter csv_writer;
    private long msg_count;
    public OpenFaceAnimatorListener (string sub_to_ip, string sub_to_port) {
        this.sub_to_ip = sub_to_ip;
        this.sub_to_port = sub_to_port;
    }

    private void ListenerWork () {
        Debug.Log ("Setting up subscriber sock");
        AsyncIO.ForceDotNet.Force ();
        using (var subSocket = new SubscriberSocket ()) {
            // set limit on how many messages in memory
            subSocket.Options.ReceiveHighWatermark = 1000;
            // socket connection
            // subSocket.Connect("tcp://localhost:5572");
            subSocket.Connect ("tcp://" + sub_to_ip + ":" + sub_to_port);
            // subscribe to topics; "" == all topics
            subSocket.Subscribe ("");
            Debug.Log ("sub socket initiliased");

            string topic;
            //string frame;
            string timestamp;
            //string blend_shapes;
            //string head_pose;
            string facsvatar_json;
            while (!_listenerCancelled) {
                //string frameString;
                // wait for full message
                //if (!subSocket.TryReceiveFrameString(out frameString)) continue;
                //Debug.Log(frameString);
                //_messageQueue.Enqueue(frameString);

                List<string> msg_list = new List<string> ();
                if (!subSocket.TryReceiveFrameString (out topic)) continue;
                //if (!subSocket.TryReceiveFrameString(out frame)) continue;
                if (!subSocket.TryReceiveFrameString (out timestamp)) continue;
                //if (!subSocket.TryReceiveFrameString(out blend_shapes)) continue;
                //if (!subSocket.TryReceiveFrameString(out head_pose)) continue;
                if (!subSocket.TryReceiveFrameString (out facsvatar_json)) continue;

                //Debug.Log("Received messages:");
                //Debug.Log(frame);
                //Debug.Log(timestamp);
                //Debug.Log(facsvatar_json);

                // check if we're not done; timestamp is empty
                if (timestamp != "") {
                    msg_list.Add (topic);
                    msg_list.Add (timestamp);
                    msg_list.Add (facsvatar_json);
                    long timeNowMs = UnixTimeNowMillisec ();
                    msg_list.Add (timeNowMs.ToString ()); // time msg received; for unity performance

                    msg_count++;

                    _messageQueue.Enqueue (msg_list);
                }
                // done
                else {
                    Debug.Log ("Received all messages");
                }
            }
            subSocket.Close ();
        }
        NetMQConfig.Cleanup ();
    }

    public static long UnixTimeNowMillisec () {
        DateTime unixStart = new DateTime (1970, 1, 1, 0, 0, 0, 0, System.DateTimeKind.Utc);
        long unixTimeStampInTicks = (DateTime.UtcNow - unixStart).Ticks;
        long timeNowMs = unixTimeStampInTicks / (TimeSpan.TicksPerMillisecond / 10000); // 100ns
        //Debug.Log(timeNowMs);
        return timeNowMs;
    }

    // check queue for messages
    public void Update () {
        while (!_messageQueue.IsEmpty) {
            List<string> msg_list;
            if (_messageQueue.TryDequeue (out msg_list)) {
                _messageDelegate (msg_list);
            } else {
                break;
            }
        }
    }

    // threaded message listener
    public OpenFaceAnimatorListener (MessageDelegate messageDelegate) {
        _messageDelegate = messageDelegate;
        _listenerWorker = new Thread (ListenerWork);
    }

    public void Start () {

        _listenerCancelled = false;
        _listenerWorker.Start ();
    }

    public void Stop () {
        _listenerCancelled = true;
        _listenerWorker.Join ();

    }
}

// act on messages received
public class OpenFaceAnimator : MonoBehaviour {
    private OpenFaceAnimatorListener oFListener;
    public string sub_to_ip = "127.0.0.1";
    public string sub_to_port = "5572";

    // logging
    private long msg_count;
    private string csv_folder = "Assets/Logging/";
    private string csv_path = "Assets/Logging/unity_timestamps_sub.csv";
    private StreamWriter csv_writer;
    private string csv_path_total = "Assets/Logging/unity_timestamps_total.csv";
    private StreamWriter csv_writer_total;

    public ActionUnitAnimator actionUnitAnimator;

    // Receive data from FACSvatar/OpenFace.
    private void HandleMessage (List<string> msg_list) {

        JObject facsvatar = JObject.Parse (msg_list[2]);
        // Action Unit Data
        JObject blend_shapes = facsvatar["au_r"].ToObject<JObject> ();

        UnityMainThreadDispatcher.Instance ().Enqueue (actionUnitAnimator.RequestBlendshapes (blend_shapes));

        msg_count++;
    }

    public static long UnixTimeNowMillisec () {
        DateTime unixStart = new DateTime (1970, 1, 1, 0, 0, 0, 0, System.DateTimeKind.Utc);
        long unixTimeStampInTicks = (DateTime.UtcNow - unixStart).Ticks;
        long timeNowMs = unixTimeStampInTicks / (TimeSpan.TicksPerMillisecond / 10000); // 100ns
        //Debug.Log(timeNowMs);
        return timeNowMs;
    }

    private void Start () {

        oFListener = new OpenFaceAnimatorListener (HandleMessage);
        oFListener.sub_to_ip = sub_to_ip;
        oFListener.sub_to_port = sub_to_port;
        oFListener.Start ();
    }

    private void Update () {
        oFListener.Update ();
    }

    private void OnDestroy () {
        oFListener.Stop ();
    }
}

```
