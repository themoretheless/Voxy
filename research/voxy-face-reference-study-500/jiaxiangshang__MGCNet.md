# Facial rendering source scan: jiaxiangshang/MGCNet

Repository: https://github.com/jiaxiangshang/MGCNet

Pinned source revision: 0be2fe196361783218d4b42f198e3464b35b75d9

Discovery topic: face reconstruction 3D

Description: Self-Supervised Monocular 3D Face Reconstruction by Occlusion-Aware Multi-view Geometry Consistency[ECCV 2020]

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"skin": ["src_common/data/data_loader_semi_unsupervised_skin.py"], "face": ["src_common/common/face_io.py", "src_common/geometry/face_align/__init__.py", "src_common/geometry/face_align/align.py", "src_common/geometry/face_align/align_facenet.py", "src_common/geometry/face_align/gafr_std_align.py", "src_common/geometry/face_align/generate_align_self.py", "src_common/geometry/face_align/std_lm_pose.txt"]}

## Source file: src_common/common/face_io.py

{
  "path": "src_common/common/face_io.py",
  "blob_sha": "6726375e69f9257e3c37fa23b4350a0c98cfcbd0",
  "sha256": "b99d9b093fa4e5aedcab18000d9d76c520d7370a1b366b687f012e431239ae50",
  "lines": 187,
  "bytes": 5286,
  "selection_score": 15,
  "keyword_line_counts": {
    "tooth": 1
  },
  "url": "https://github.com/jiaxiangshang/MGCNet/blob/0be2fe196361783218d4b42f198e3464b35b75d9/src_common/common/face_io.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def write_self_lm(path_info_save, lm_all, inter=\",\"):"
    },
    {
      "line": 27,
      "text": "def parse_self_lm(path_info_save):"
    },
    {
      "line": 38,
      "text": "def format_file_list(data_root, split, fmt=None, sort=False):"
    },
    {
      "line": 44,
      "text": "def atoi(text):"
    },
    {
      "line": 47,
      "text": "def natural_keys(text):"
    },
    {
      "line": 70,
      "text": "def write_self_6DoF(path_info_save, dof, inter=\",\"):"
    },
    {
      "line": 83,
      "text": "def parse_self_6DoF(path_info_save, inter=\",\"):"
    },
    {
      "line": 95,
      "text": "def write_self_intrinsicMtx(path_info_save, intrinsic, inter=\",\"):"
    },
    {
      "line": 114,
      "text": "def parse_self_intrinsicMtx(path_info_save, inter=\",\"):"
    },
    {
      "line": 128,
      "text": "def write_self_camera(path_info_save, img_width, img_height, intrinsic, pose):"
    },
    {
      "line": 168,
      "text": "def parser_self_camera(path_info_save):"
    }
  ]
}

```text

from __future__ import division
import os
import numpy as np
from collections import defaultdict

def write_self_lm(path_info_save, lm_all, inter=","):
    if isinstance(path_info_save, str):
        f_info = open(path_info_save, 'w')
    else:
        f_info = path_info_save

    if isinstance(lm_all, list):
        lm_all = np.array(lm_all)

    f_info.write(str(lm_all.shape[0]))
    f_info.write("\n")
    for i in range(lm_all.shape[0]):
        lm = lm_all[i]
        for j in range(len(lm)):
            lm_xyz = lm[j]
            if j != len(lm)-1:
                f_info.write(("%f"+inter) % (lm_xyz))
            else:
                f_info.write("%f" % (lm_xyz))
        f_info.write('\n')

def parse_self_lm(path_info_save):
    with open(path_info_save) as f_info:
        lines = f_info.readlines()
        lines_lm = lines[1:]
        list_lm = []
        for lm2d in lines_lm:
            xyz = lm2d[:-1].split(',')
            xyz = [float(ele) for ele in xyz]
            list_lm.append(xyz)
    return list_lm

def format_file_list(data_root, split, fmt=None, sort=False):
    with open(data_root + '/%s.txt' % split, 'r') as f:
        frames = f.readlines()

    if sort:
        import re
        def atoi(text):
            return int(text) if text.isdigit() else text

        def natural_keys(text):
            '''
            alist.sort(key=natural_keys) sorts in human order
            http://nedbatchelder.com/blog/200712/human_sorting.html
            (See Toothy's implementation in the comments)
            '''
            return [atoi(c) for c in re.split(r'(\d+)', text)]

        frames = sorted(frames, key=natural_keys)


    subfolders = [x.split(' ')[0] for x in frames]
    frame_ids = [x.split(' ')[1][:-1] for x in frames]

    if fmt is None:
        image_file_list = [os.path.join(data_root, subfolders[i], frame_ids[i] + '.jpg') for i in range(len(frames))]
    else:
        image_file_list = [os.path.join(data_root, subfolders[i], frame_ids[i] + fmt) for i in range(len(frames))]
    cam_file_list = [os.path.join(data_root, subfolders[i], frame_ids[i] + '_info.txt') for i in range(len(frames))]

    return image_file_list, cam_file_list, subfolders, frame_ids

# MFS
def write_self_6DoF(path_info_save, dof, inter=","):
    if isinstance(path_info_save, str):
        f_info = open(path_info_save, 'w')
    else:
        f_info = path_info_save

    # intrinsic
    f_info.write('%f,%f,%f,%f,%f,%f\n' %
        (
            dof[0],dof[1],dof[2],dof[3],dof[4],dof[5]
        )
    )

def parse_self_6DoF(path_info_save, inter=","):
    if isinstance(path_info_save, str):
        f_info = open(path_info_save, 'r')
    else:
        f_info = path_info_save

    dof = f_info.readline()[:-1]
    dof = dof.split(inter)
    dof = [float(p) for p in dof]
    return dof


def write_self_intrinsicMtx(path_info_save, intrinsic, inter=","):
    """
    :param path_info_save:
    :param intrinsic: [focalx focaly u v]
    :param inter:
    :return:
    """
    if isinstance(path_info_save, str):
        f_info = open(path_info_save, 'w')
    else:
        f_info = path_info_save

    # intrinsic
    f_info.write('%f,0.,%f,0.,%f,%f,0.,0.,1.\n' %
        (
            intrinsic[0], intrinsic[1], intrinsic[2], intrinsic[3]
        )
    )

def parse_self_intrinsicMtx(path_info_save, inter=","):
    if isinstance(path_info_save, str):
        f_info = open(path_info_save, 'w')
    else:
        f_info = path_info_save

    intrin_mtx = f_info.readline()[:-1]
    intrin_mtx = intrin_mtx.split(inter)
    intrin_mtx = [float(p) for p in intrin_mtx]
    intrin_mtx = np.array(intrin_mtx)
    intrin_mtx = np.reshape(intrin_mtx, [3,3])
    return intrin_mtx

#
def write_self_camera(path_info_save, img_width, img_height, intrinsic, pose):
    """
    :param path_info_save: str
    :param intrinsic: shape=[3, 3]
    :param pose: shape=[6], rx, ry, rz, tx, ty, tz
    :return:
    """
    if isinstance(path_info_save, str):
        f_info = open(path_info_save, 'w')
    else:
        f_info = path_info_save

    if len(intrinsic.shape) == 2:
        intrinsic = np.reshape(intrinsic, [-1])

    f_info.write(str(img_width))
    f_info.write(" ")
    f_info.write(str(img_height))
    f_info.write("\n")

    f_info.write("intrinsic")
    f_info.write("\n")
    for i in range(intrinsic.shape[0]):
        row = intrinsic[i]
        if i != len(intrinsic) - 1:
            f_info.write("%f," % (row))
        else:
            f_info.write("%f" % (row))
    f_info.write('\n')

    f_info.write("external")
    f_info.write("\n")
    for i in range(pose.shape[0]):
        row = pose[i]
        if i != len(pose) - 1:
            f_info.write("%f," % (row))
        else:
            f_info.write("%f" % (row))
    f_info.write('\n')

def parser_self_camera(path_info_save):
    """
    :param path_info_save: str
    :param intrinsic: shape=[3, 3]
    :param pose: shape=[6], rx, ry, rz, tx, ty, tz
    :return:
    """
    f_info = open(path_info_save, 'r')

    frs = f_info.readline()
    img_width = int(frs[0])
    img_height = int(frs[1])

    f_info.readline()
    intrin_mtx = parse_self_intrinsicMtx(f_info)

    f_info.readline()
    pose = parse_self_6DoF(f_info)

    return img_width, img_height, intrin_mtx, pose


```

## Source file: src_common/geometry/face_align/align_facenet.py

{
  "path": "src_common/geometry/face_align/align_facenet.py",
  "blob_sha": "4ecd810ff8ec8ecc5cb9c565394e1505fa10cb27",
  "sha256": "eaa4cc1ca263c1766690ca62c408d37269de4554af12ada9caf79c29b2a1d482",
  "lines": 82,
  "bytes": 4557,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 13
  },
  "url": "https://github.com/jiaxiangshang/MGCNet/blob/0be2fe196361783218d4b42f198e3464b35b75d9/src_common/geometry/face_align/align_facenet.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def get_facenet_align_lm(img_height):"
    },
    {
      "line": 60,
      "text": "def facenet_align(list_image, list_lm, std_lm, img_height, img_width):"
    }
  ]
}

```text

#
import numpy as np
import tensorflow as tf

# self
from .align import lm2d_trans

# main
def get_facenet_align_lm(img_height):
    lm_facenet_align = [
        (0.0792396913815, 0.339223741112), (0.0829219487236, 0.456955367943),
        (0.0967927109165, 0.575648016728), (0.122141515615, 0.691921601066),
        (0.168687863544, 0.800341263616), (0.239789390707, 0.895732504778),
        (0.325662452515, 0.977068762493), (0.422318282013, 1.04329000149),
        (0.531777802068, 1.06080371126), (0.641296298053, 1.03981924107),
        (0.738105872266, 0.972268833998), (0.824444363295, 0.889624082279),
        (0.894792677532, 0.792494155836), (0.939395486253, 0.681546643421),
        (0.96111933829, 0.562238253072), (0.970579841181, 0.441758925744),
        (0.971193274221, 0.322118743967), (0.163846223133, 0.249151738053),
        (0.21780354657, 0.204255863861), (0.291299351124, 0.192367318323),
        (0.367460241458, 0.203582210627), (0.4392945113, 0.233135599851),
        (0.586445962425, 0.228141644834), (0.660152671635, 0.195923841854),
        (0.737466449096, 0.182360984545), (0.813236546239, 0.192828009114),
        (0.8707571886, 0.235293377042), (0.51534533827, 0.31863546193),
        (0.516221448289, 0.396200446263), (0.517118861835, 0.473797687758),
        (0.51816430343, 0.553157797772), (0.433701156035, 0.604054457668),
        (0.475501237769, 0.62076344024), (0.520712933176, 0.634268222208),
        (0.565874114041, 0.618796581487), (0.607054002672, 0.60157671656),
        (0.252418718401, 0.331052263829), (0.298663015648, 0.302646354002),
        (0.355749724218, 0.303020650651), (0.403718978315, 0.33867711083),
        (0.352507175597, 0.349987615384), (0.296791759886, 0.350478978225),
        (0.631326076346, 0.334136672344), (0.679073381078, 0.29645404267),
        (0.73597236153, 0.294721285802), (0.782865376271, 0.321305281656),
        (0.740312274764, 0.341849376713), (0.68499850091, 0.343734332172),
        (0.353167761422, 0.746189164237), (0.414587777921, 0.719053835073),
        (0.477677654595, 0.706835892494), (0.522732900812, 0.717092275768),
        (0.569832064287, 0.705414478982), (0.635195811927, 0.71565572516),
        (0.69951672331, 0.739419187253), (0.639447159575, 0.805236879972),
        (0.576410514055, 0.835436670169), (0.525398405766, 0.841706377792),
        (0.47641545769, 0.837505914975), (0.41379548902, 0.810045601727),
        (0.380084785646, 0.749979603086), (0.477955996282, 0.74513234612),
        (0.523389793327, 0.748924302636), (0.571057789237, 0.74332894691),
        (0.672409137852, 0.744177032192), (0.572539621444, 0.776609286626),
        (0.5240106503, 0.783370783245), (0.477561227414, 0.778476346951)
    ]

    TPL_MIN, TPL_MAX = np.min(lm_facenet_align, axis=0), np.max(lm_facenet_align, axis=0)
    lm_facenet_align = (lm_facenet_align - TPL_MIN) / (TPL_MAX - TPL_MIN)
    defined_lm_facenet_align = tf.constant(lm_facenet_align, shape=[1, 68, 2])
    # defined_lm_facenet_align = defined_lm_facenet_align[:, 17:, :]
    # defined_lm_facenet_align = defined_lm_facenet_align[:, 48:, :]
    # defined_lm_facenet_align = lm_dlib_to_celebA(defined_lm_facenet_align)

    facenet_scale = float(96) / 110.0
    defined_lm_facenet_align = img_height * defined_lm_facenet_align * facenet_scale + img_height * (1 - facenet_scale) / 2
    defined_lm_facenet_align = tf.cast(defined_lm_facenet_align, dtype=tf.float32)

    return defined_lm_facenet_align

def facenet_align(list_image, list_lm, std_lm, img_height, img_width):
    if isinstance(list_image, list) == False:
        list_image = [list_image]
    if isinstance(list_lm, list) == False:
        list_lm = [list_lm]

    list_image_warp = []
    for i in range(len(list_image)):
        images = list_image[i]
        images_pad = tf.pad(images, paddings=[[0, 0], [56, 56], [56, 56], [0, 0]], mode='REFLECT')
        lm2d_align = list_lm[i]  # bs, lm_num, xy
        lm2d_align = lm2d_align + 56
        # lm2d_align = lm2d_align[:, 48:, :]
        # lm2d_align = lm_dlib_to_celebA(lm2d_align)
        #
        with tf.device('/cpu:0'):
            trans_mat = lm2d_trans(std_lm, lm2d_align)
            # image_warp = transform(images, trans_mat)
            flat_transforms = tf.contrib.image.matrices_to_flat_transforms(trans_mat)
            image_warp = tf.contrib.image.transform(images_pad, flat_transforms, interpolation='BILINEAR',
                                   output_shape=[img_height, img_width])
            list_image_warp.append(image_warp)
    return list_image_warp

```

## Source file: src_common/data/data_loader_semi_unsupervised_skin.py

{
  "path": "src_common/data/data_loader_semi_unsupervised_skin.py",
  "blob_sha": "8fdf26aa91738727f92b3419bdd124c47000b13d",
  "sha256": "3e9491896c959cd34c7914e62a3dd7ab15e13ddaef5970726f65ce690e6ce3b8",
  "lines": 244,
  "bytes": 11870,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 13
  },
  "url": "https://github.com/jiaxiangshang/MGCNet/blob/0be2fe196361783218d4b42f198e3464b35b75d9/src_common/data/data_loader_semi_unsupervised_skin.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self,"
    },
    {
      "line": 33,
      "text": "def format_file_list(self, data_root, split):"
    },
    {
      "line": 69,
      "text": "def load_train_batch(self, is_test=False):"
    },
    {
      "line": 74,
      "text": "def _parse_train_img(img_path):"
    },
    {
      "line": 84,
      "text": "def _batch_preprocessing(stack_images, stack_images_skin, flag_sgl_mul, intrinsics, optional_data1, optional_data2):"
    },
    {
      "line": 136,
      "text": "def init_data_pipeline(self, sess, batch_sample, file_list):"
    },
    {
      "line": 138,
      "text": "def _load_cam_intrinsics(cam_filelist, read_pose, match_num):"
    },
    {
      "line": 219,
      "text": "def batch_unpack_image_sequence(self, image_seq, img_height, img_width, num_source):"
    }
  ]
}

```text

from __future__ import division
import os
import random
import tensorflow as tf
import numpy as np

from src_common.common.tf_io import unpack_image_sequence, data_augmentation_mul

class DataLoader(object):
    def __init__(self, 
                 dataset_dir=None,
                 batch_size=None,
                 img_height=None, 
                 img_width=None, 
                 num_source=None,
                 aug_crop_size=None,
                 read_pose=False,
                 match_num=0,
                 read_gpmm=False,
                 flag_data_aug=False,
                 flag_shuffle=True):
        self.dataset_dir = dataset_dir
        self.batch_size = batch_size
        self.img_height = img_height
        self.img_width = img_width
        self.num_source = num_source
        self.aug_crop_size = aug_crop_size
        self.read_pose = read_pose
        self.match_num = match_num
        self.flag_data_aug = flag_data_aug
        self.flag_shuffle = flag_shuffle

    def format_file_list(self, data_root, split):
        all_list = {}
        with open(data_root + '/%s.txt' % split, 'r') as f:
            frames = f.readlines()
        flag_slg_mul = [x.split(' ')[0] for x in frames]
        subfolders = [x.split(' ')[1] for x in frames]
        frame_ids = [x.split(' ')[2][:-1] for x in frames]

        image_file_list = [os.path.join(data_root, subfolders[i],
            frame_ids[i] + '.jpg') for i in range(len(frames))]
        cam_file_list = [os.path.join(data_root, subfolders[i],
            frame_ids[i] + '_cam.txt') for i in range(len(frames))]
        skin_file_list = [os.path.join(data_root, subfolders[i],
                                      frame_ids[i] + '_skin.jpg') for i in range(len(frames))]

        steps_per_epoch = int(len(image_file_list) // self.batch_size)
        print("*************************************************************** format_file_list ")
        img_cam = list(zip(image_file_list, cam_file_list, skin_file_list, flag_slg_mul))
        if self.flag_shuffle:
            random.shuffle(img_cam)
        else:
            print("Without shuffle")
        image_file_list, cam_file_list, skin_file_list, flag_slg_mul = zip(*img_cam)

        all_list['image_file_list'] = image_file_list
        all_list['cam_file_list'] = cam_file_list
        all_list['skin_file_list'] = skin_file_list
        all_list['flag_slg_mul'] = flag_slg_mul

        self.steps_per_epoch = int(len(all_list['image_file_list']) // self.batch_size)
        print("Finish format_file_list")
        if len(image_file_list) > 10:
            for i in range(10):
                print(image_file_list[i])
        return all_list

    def load_train_batch(self, is_test=False):
        """
        Load a batch of training instances using the new tensorflow
        Dataset api.
        """
        def _parse_train_img(img_path):
            with tf.device('/cpu:0'):
                img_buffer = tf.read_file(img_path)
                image_decoded = tf.image.decode_jpeg(img_buffer)
                # TODO: TW Image sequence structure
                tgt_image, src_image_stack = \
                     unpack_image_sequence(
                        image_decoded, self.img_height, self.img_width, self.num_source)
            return tgt_image, src_image_stack

        def _batch_preprocessing(stack_images, stack_images_skin, flag_sgl_mul, intrinsics, optional_data1, optional_data2):
            intrinsics = tf.cast(intrinsics, tf.float32)
            image_all = tf.concat([stack_images[0], stack_images[1], stack_images_skin[0], stack_images_skin[1]], axis=3)

            if not is_test and self.flag_data_aug:  # otherwise matches coords are wrong
                if self.match_num == 0:
                    image_all, intrinsics, matches = data_augmentation_mul(
                        image_all, intrinsics, self.img_height, self.img_width)
                else:
                    image_all, intrinsics, matches = data_augmentation_mul(
                        image_all, intrinsics, self.img_height, self.img_width, optional_data2)
            else:
                matches = optional_data2

            image = image_all[:, :, :, :3*(self.num_source+1)]
            image_skin = image_all[:, :, :, 3*(self.num_source+1):]
            return image, image_skin, flag_sgl_mul, intrinsics, optional_data1, matches

        input_image_names_ph = tf.placeholder(tf.string, shape=[None], name='input_image_names_ph')
        image_dataset = tf.data.Dataset.from_tensor_slices(input_image_names_ph).map(_parse_train_img)

        input_skin_names_ph = tf.placeholder(tf.string, shape=[None], name='input_skin_names_ph')
        skin_dataset = tf.data.Dataset.from_tensor_slices(input_skin_names_ph).map(_parse_train_img)

        flag_sgl_mul_ph = tf.placeholder(tf.int32, [None], name='flag_sgl_mul_ph')
        flag_sgl_mul_dataset = tf.data.Dataset.from_tensor_slices(flag_sgl_mul_ph)

        cam_intrinsics_ph = tf.placeholder(tf.float32, [None, 1+self.num_source, 3, 3], name='cam_intrinsics_ph')
        intrinsics_dataset = tf.data.Dataset.from_tensor_slices(cam_intrinsics_ph)

        datasets = (image_dataset, skin_dataset, flag_sgl_mul_dataset, intrinsics_dataset)
        if self.read_pose:
            poses_ph = tf.placeholder(tf.float32, [None, 1+self.num_source, 6], name='poses_ph')
            pose_dataset = tf.data.Dataset.from_tensor_slices(poses_ph)
            datasets = datasets + (pose_dataset,)
        else:
            datasets = datasets + (intrinsics_dataset,)
        if self.match_num > 0:
            matches_ph = tf.placeholder(tf.float32, [None, (1+self.num_source), self.match_num, 2], name='matches_ph')
            match_dataset = tf.data.Dataset.from_tensor_slices(matches_ph)
            datasets = datasets + (match_dataset,)
        else:
            datasets = datasets + (intrinsics_dataset,)

        all_dataset = tf.data.Dataset.zip(datasets)
        all_dataset = all_dataset.batch(self.batch_size)
        if self.flag_shuffle:
            all_dataset = all_dataset.shuffle(buffer_size=4000).repeat().prefetch(self.batch_size*4)
        all_dataset = all_dataset.map(_batch_preprocessing)
        iterator = all_dataset.make_initializable_iterator()
        return iterator

    def init_data_pipeline(self, sess, batch_sample, file_list):
        #
        def _load_cam_intrinsics(cam_filelist, read_pose, match_num):
            all_cam_intrinsics = []
            all_poses = []
            all_matches = []

            for i in range(len(cam_filelist)):
                filename = cam_filelist[i]
                if i % 50000 == 0:
                    print(i, ' in all: ', len(cam_filelist))
                f = open(filename)
                one_intrinsic = []
                for i in range(1 + self.num_source):
                    line = f.readline()
                    #
                    cam_intri_vec = [float(num) for num in line.strip().split(',')]
                    if len(cam_intri_vec) != 9:
                        print(filename, i, line)

                    cam_intrinsics = np.reshape(cam_intri_vec, [3, 3])
                    one_intrinsic.append(cam_intrinsics)
                one_intrinsic = np.stack(one_intrinsic, axis=0)
                all_cam_intrinsics.append(one_intrinsic)
                #
                if read_pose:
                    one_sample_pose = []
                    for i in range(0, 1 + self.num_source):
                        lines = f.readline()
                        pose = [float(num) for num in lines.strip().split(',')]
                        pose_vec = np.reshape(pose, [6])
                        one_sample_pose.append(pose_vec)
                    one_sample_pose = np.stack(one_sample_pose, axis=0)
                    all_poses.append(one_sample_pose)
                #
                if match_num > 0:
                    image_matches = []
                    for i in range(1 + self.num_source):
                        one_matches = []
                        line = f.readline()
                        for j in range(self.match_num):
                            line = f.readline()

                            match_coords = [float(num) for num in line.strip().split(',')]
                            match_vec = np.reshape(match_coords, [2])
                            one_matches.append(match_vec)
                        one_matches = np.stack(one_matches, axis=0) # 68
                        image_matches.append(one_matches)
                        # TODO: Very dangerous
                        # if i == self.num_source / 2:
                        #     image_matches = [one_matches] + image_matches
                        # else:

                    image_matches = np.stack(image_matches, axis=0) # (1 + self.num_source), 68

                    all_matches.append(image_matches)

                f.close()
            all_cam_intrinsics = np.stack(all_cam_intrinsics, axis=0)

            if read_pose:
                all_poses = np.stack(all_poses, axis=0)
            if match_num > 0:
                all_matches = np.stack(all_matches, axis=0)
            return all_cam_intrinsics, all_poses, all_matches

        # load cam_intrinsics using native python
        print('load camera intrinsics...')
        cam_intrinsics, all_poses, all_matches = _load_cam_intrinsics(file_list['cam_file_list'], self.read_pose, self.match_num)

        input_dict = {'data_loading/input_image_names_ph:0': file_list['image_file_list'][:self.batch_size * self.steps_per_epoch],
                      'data_loading/input_skin_names_ph:0': file_list['skin_file_list'][:self.batch_size * self.steps_per_epoch],
                      'data_loading/flag_sgl_mul_ph:0': file_list['flag_slg_mul'][:self.batch_size * self.steps_per_epoch],
                      'data_loading/cam_intrinsics_ph:0': cam_intrinsics[:self.batch_size*self.steps_per_epoch]}
        if self.read_pose:
            print('load pose data...')
            input_dict['data_loading/poses_ph:0'] = all_poses[:self.batch_size*self.steps_per_epoch]
        if self.match_num > 0:
            print('load matches data...')
            input_dict['data_loading/matches_ph:0'] = all_matches[:self.batch_size*self.steps_per_epoch]

        sess.run(batch_sample.initializer, feed_dict=input_dict)

    def batch_unpack_image_sequence(self, image_seq, img_height, img_width, num_source):
        # Assuming the center image is the target frame
        tgt_start_idx = int(img_width * (num_source//2))
        tgt_image = tf.slice(image_seq, 
                             [0, 0, tgt_start_idx, 0], 
                             [-1, -1, img_width, -1])
        # Source frames before the target frame
        src_image_1 = tf.slice(image_seq, 
                               [0, 0, 0, 0], 
                               [-1, -1, int(img_width * (num_source//2)), -1])
        # Source frames after the target frame
        src_image_2 = tf.slice(image_seq, 
                               [0, 0, int(tgt_start_idx + img_width), 0], 
                               [-1, -1, int(img_width * (num_source//2)), -1])
        src_image_seq = tf.concat([src_image_1, src_image_2], axis=2)
        # Stack source frames along the color channels (i.e. [B, H, W, N*3])
        src_image_stack = tf.concat([tf.slice(src_image_seq, 
                                    [0, 0, i*img_width, 0], 
                                    [-1, -1, img_width, -1]) 
                                    for i in range(num_source)], axis=3)
        return tgt_image, src_image_stack

if __name__ == "__main__":
    path_data = "/home/jshang/SHANG_Data_MOUNT/141/GAFR_semi_bfmAlign_3/11141_300WLP_CelebA_Mpie_tensor_MERGE"
    h_loader = DataLoader(batch_size=5)
    f = h_loader.format_file_list(path_data, 'train', 3, 2)

```

## Source file: src_common/geometry/face_align/align.py

{
  "path": "src_common/geometry/face_align/align.py",
  "blob_sha": "cfec102a46c636cc6e2e3790f1e1f8eae66f2a1a",
  "sha256": "dcbb86932666580cb1e201ff78520d0990ef3ccd4ab9037dc52e76b4c19f7ca1",
  "lines": 154,
  "bytes": 5072,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 17
  },
  "url": "https://github.com/jiaxiangshang/MGCNet/blob/0be2fe196361783218d4b42f198e3464b35b75d9/src_common/geometry/face_align/align.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def Quaternion2Mat(quat):"
    },
    {
      "line": 35,
      "text": "def CenterOfPoints(points):"
    },
    {
      "line": 39,
      "text": "def UncenterPoints(points):"
    },
    {
      "line": 47,
      "text": "def ScaleFromPoints(left_points, right_points):"
    },
    {
      "line": 63,
      "text": "def TransformFromPointsTF(left_points, right_points):"
    },
    {
      "line": 124,
      "text": "def lm2d_trans(lm_src, lm_tar):"
    }
  ]
}

```text

#
import tensorflow as tf
import numpy as np
import math


"""
From 
"""
def Quaternion2Mat(quat):
    """
    :param quat: 4
    :return: 3x3
    """
    quat = tf.squeeze(quat)
    w = quat[0]
    x = quat[1]
    y = quat[2]
    z = quat[3]

    val00 = 1 - 2 * y * y - 2 * z * z
    val01 = 2 * x * y - 2 * z * w
    val02 = 2 * x * z + 2 * y * w
    val10 = 2 * x * y + 2 * z * w

    val11 = 1 - 2 * x * x - 2 * z * z
    val12 = 2 * y * z - 2 * x * w
    val20 = 2 * x * z - 2 * y * w
    val21 = 2 * y * z + 2 * x * w
    val22 = 1 - 2 * x * x - 2 * y * y
    rotation = tf.stack([val00, val01, val02, val10, val11, val12, val20, val21, val22], axis=0)
    rotation = tf.reshape(rotation, shape=[3,3])
    return rotation

def CenterOfPoints(points):
    center = tf.reduce_mean(points, axis=1) # 3
    return center

def UncenterPoints(points):
    point_num = points.get_shape().as_list()[1]
    center = CenterOfPoints(points)    # 2
    center = tf.expand_dims(center, axis=-1)    # 2x1
    center_tile = tf.tile(center, [1, point_num])
    u_points = points - center_tile
    return u_points

def ScaleFromPoints(left_points, right_points):
    """
    Compute relative scale from left points to right points
    :param left_points: 3xN
    :param right_points: 3xN
    :return:
    """
    lefts = UncenterPoints(left_points)  # 3xN
    rights = UncenterPoints(right_points)

    ## Compute scale
    left_norm_square = tf.reduce_sum(tf.square(tf.norm(lefts, axis=0)))
    right_norm_square = tf.reduce_sum(tf.square(tf.norm(rights, axis=0)))
    scale = tf.sqrt(right_norm_square / left_norm_square)
    return scale

def TransformFromPointsTF(left_points, right_points):
    """
    Tensorflow implementatin of aligning left points to right points
    :param left_points: 3xN
    :param right_points: 3xN
    :return:
    """

    lefts = UncenterPoints(left_points)    # 3xN
    rights = UncenterPoints(right_points)
    # lefts = left_points
    # rights = right_points

    ## Compute scale
    left_norm_square = tf.reduce_sum(tf.square(tf.norm(lefts, axis=0)))
    right_norm_square = tf.reduce_sum(tf.square(tf.norm(rights, axis=0)))
    scale = tf.sqrt(right_norm_square / (left_norm_square+1e-6))

    ## Compute rotation
    #rights = tf.Print(rights, [rights], message='rights', summarize=2 * 68)
    M = tf.matmul(lefts, rights, transpose_b=True)  # 3x3
    #M = tf.Print(M, [M.shape, M], message="M", summarize=64)

    N00 = M[0, 0] + M[1, 1] + M[2, 2]
    N11 = M[0, 0] - M[1, 1] - M[2, 2]
    N22 = -M[0, 0] + M[1, 1] - M[2, 2]
    N33 = -M[0, 0] - M[1, 1] + M[2, 2]

    N01 = M[1, 2] - M[2, 1]
    N10 = M[1, 2] - M[2, 1]
    N02 = M[2, 0] - M[0, 2]
    N20 = M[2, 0] - M[0, 2]

    N03 = M[0, 1] - M[1, 0]
    N30 = M[0, 1] - M[1, 0]
    N12 = M[0, 1] + M[1, 0]
    N21 = M[0, 1] + M[1, 0]

    N13 = M[0, 2] + M[2, 0]
    N31 = M[0, 2] + M[2, 0]
    N23 = M[1, 2] + M[2, 1]
    N32 = M[1, 2] + M[2, 1]
    N = tf.stack([N00,N01,N02,N03,N10,N11,N12,N13,N20,N21,N22,N23,N30,N31,N32,N33], axis=0)
    N = tf.reshape(N, [4,4])

    #N = tf.Print(N, [N.shape, N], message="N", summarize=64)

    eigen_vals, eigen_vecs = tf.self_adjoint_eig(N)
    quaternion = tf.squeeze((tf.slice(eigen_vecs, [0, 3], [4, 1])))    # 4
    #quaternion = tf_render.Print(quaternion, [quaternion], message='quaternion', summarize=4)
    rotation = Quaternion2Mat(quaternion)   # 3x3

    ## Compute translation
    left_center = CenterOfPoints(left_points)
    right_center = CenterOfPoints(right_points)
    rot_left_center = tf.squeeze(tf.matmul(rotation, tf.expand_dims(left_center, axis=-1))) # 3
    translation = right_center - scale * rot_left_center

    return scale, rotation, translation

#
def lm2d_trans(lm_src, lm_tar):
    filler_mtx = tf.constant([0.0, 0.0, 1.0], shape=[1, 3])
    list_trans_mtx = []
    for b in range(lm_src.shape[0]):
        filler_z = tf.constant([0.0], shape=[1, 1])
        filler_z = tf.tile(filler_z, multiples=[lm_src.shape[1], 1])
        b_src = lm_src[b]
        b_src = tf.concat([b_src, filler_z], axis=1)
        b_src = tf.transpose(b_src)
        b_tar = lm_tar[b]
        b_tar = tf.concat([b_tar, filler_z], axis=1)
        b_tar = tf.transpose(b_tar)

        #b_src = tf.Print(b_src, [b_src], message='b_src', summarize=2 * 68)
        # b_tar = tf_render.Print(b_tar, [b_tar], message='b_tar', summarize=16)
        s, rot_mat, translation = TransformFromPointsTF(b_src, b_tar)

        # s = tf_render.Print(s, [s, s.shape], message='s', summarize=1)

        # rot_mat = tf_render.Print(rot_mat, [rot_mat], message='rot_mat', summarize=9)
        # translation = tf_render.Print(translation, [translation], message='translation', summarize=3)
        rot_mat = rot_mat[0:2, 0:2] * s
        translation = translation[0:2]
        translation = tf.expand_dims(translation, axis=-1)

        ext_mat = tf.concat([rot_mat, translation], axis=1)
        ext_mat = tf.concat([ext_mat, filler_mtx], axis=0)
        list_trans_mtx.append(ext_mat)

    trans_mtx = tf.stack(list_trans_mtx)
    return trans_mtx


```
