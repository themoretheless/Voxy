# Facial rendering source scan: tiborkubik/Robust-Teeth-Detection-in-3D-Dental-Scans

Repository: https://github.com/tiborkubik/Robust-Teeth-Detection-in-3D-Dental-Scans

Pinned source revision: da4fcdf64bd532d3d8d3007f019ed040ad1540ff

Discovery topic: teeth 3D

Description: [BIOIMAGING 2022] Robust Teeth Detection in 3D Dental Scans by Automated Multi-View Landmarking

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"jaw": ["src/JawModel.py", "src/trainer/JawDataset.py"], "teeth": ["src/trainer/MissingTeethSampler.py"]}

## Source file: src/JawModel.py

{
  "path": "src/JawModel.py",
  "blob_sha": "c84e5021a13a7f42e7213efab8a410caa3be5d23",
  "sha256": "74fe39983eae0c548d5594bc5b20dc8b3f1eb5abe8778f17b99bbdeb9baa9849",
  "lines": 270,
  "bytes": 9414,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 34,
    "rig": 2
  },
  "url": "https://github.com/tiborkubik/Robust-Teeth-Detection-in-3D-Dental-Scans/blob/da4fcdf64bd532d3d8d3007f019ed040ad1540ff/src/JawModel.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def expand_affine_transformation_r_c(t_matrix):"
    },
    {
      "line": 43,
      "text": "def get_new_model(model_path, mode='eval'):"
    },
    {
      "line": 65,
      "text": "def read_poly_data(file_name):"
    },
    {
      "line": 89,
      "text": "def __init__(self, jaw_path, jaw_poly, jaw_actor, mode='trainer'):"
    },
    {
      "line": 104,
      "text": "def get_mass_center(self):"
    },
    {
      "line": 118,
      "text": "def get_name_from_path(self):"
    },
    {
      "line": 127,
      "text": "def get_transformation_matrix(self):"
    },
    {
      "line": 139,
      "text": "def get_jaw_type(self):"
    },
    {
      "line": 161,
      "text": "def get_GTs(self):"
    },
    {
      "line": 188,
      "text": "def apply_jaw_transformations(self):"
    },
    {
      "line": 227,
      "text": "def get_transformation_matrix_from_xml(xml_path, model_number):"
    }
  ]
}

```text

"""
    :filename JawModel.py
    :author Tibor Kubik
    :email xkubik34@stud.fit.vutbr.cz

    File contains JawModel class creation.
    This class is used to represent the polygonal model of a jaw in scene.
"""

import os
import vtk
import math
import numpy as np
import xml.etree.ElementTree as ElementTree

import config


def expand_affine_transformation_r_c(t_matrix):
    """
    | a b c |       | a b c 0 |
    | d e f |   ->  | d e f 0 |
    | g h i |       | g h i 0 |
                    | 0 0 0 1 |

    :param t_matrix: 4x4 vtkMatrix

    :return: 4x4 matrix expanded by zeros in last row and column,
    except for the lower-right corner which is set to 1.
    """

    t_matrix.SetElement(0, 3, 0.0)
    t_matrix.SetElement(1, 3, 0.0)
    t_matrix.SetElement(2, 3, 0.0)
    t_matrix.SetElement(3, 3, 1.0)
    t_matrix.SetElement(3, 0, 0.0)
    t_matrix.SetElement(3, 1, 0.0)
    t_matrix.SetElement(3, 2, 0.0)

    return t_matrix


def get_new_model(model_path, mode='eval'):
    """
    Function creates an instance of JawModel. This object instance contains all necessary
    information for further processing in the pipeline, namely:
    Model path, model name, information for the rendering (poly data, actor) and in case of
    eval mode, the ground truth positions.

    :param model_path: The absolute or relative path to the STL object file
    :param mode: Which module called this function (eval/annotate)
    :return: instance of JawModel
    """
    reader = read_poly_data(model_path)
    mapper = vtk.vtkPolyDataMapper()
    mapper.SetInputConnection(reader.GetOutputPort())
    reader.Update()
    poly_data = reader.GetOutput()
    jawActor = vtk.vtkActor()
    jawActor.SetMapper(mapper)

    return JawModel(model_path, jaw_poly=poly_data, jaw_actor=jawActor, mode=mode)


def read_poly_data(file_name):
    """
    Function takes file name(path) as an input and creates a poly model using vtk STL reader.
    Both reader and data are returned as both of them are needed in further pipeline processing.

    :param file_name: Path to the STL file to be read.

    :return: STL Read and the mesh data.
    """

    poly_reader = vtk.vtkSTLReader()
    poly_reader.SetFileName(file_name)
    poly_reader.Update()

    return poly_reader


class JawModel:
    """
    Class represents currently processed model of jaw.
    Jaw is defined by path of source file, its poly and actor.
    It also contains the ground truth labels of landmarks associated with the dentition.
    """

    def __init__(self, jaw_path, jaw_poly, jaw_actor, mode='trainer'):
        """
        Class constructor.
        Instance of JawModel is usually created after connecting jaw poly on its actor.
        """
        self.path = jaw_path
        self.name = self.get_name_from_path()
        self.poly = jaw_poly
        self.actor = jaw_actor
        self.mass_center = self.get_mass_center()  # mass center is used in the transformation matrix creation
        self.transformation_matrix = self.get_transformation_matrix()
        # self.jaw_type = self.get_jaw_type()  # maxillary or mandibular
        self.mode = mode
        self.GTs = self.get_GTs()  # Ground truth landmark values

    def get_mass_center(self):
        """
        Method calculates mass center of given model.
JawModel
        :return: tuple (x, y, z) representing the mass center of a 3D model.
        """

        center_of_mass_filter = vtk.vtkCenterOfMass()
        center_of_mass_filter.SetInputData(self.poly)
        center_of_mass_filter.SetUseScalarsAsWeights(False)
        center_of_mass_filter.Update()

        return center_of_mass_filter.GetCenter()

    def get_name_from_path(self):
        """
        Method is used to parse name of file excluding file type from its path.

        :return: Extracted name from the file path, e.g. 000016_0.
        """

        return os.path.splitext(os.path.basename(self.path))[0]

    def get_transformation_matrix(self):
        """
        Method extracts transformation matrix from corresponding XML file, which should be provided with the polygon.

        :return: 4x4 VTK transformation matrix from the XML file.
        """

        path_to_xml = self.path[0:self.path.rfind('_')] + '.xml'
        model_number = self.path[self.path.rfind('_')+1:self.path.rfind('.')]

        return self.get_transformation_matrix_from_xml(path_to_xml, model_number)

    def get_jaw_type(self):
        """
        Method finds the type of the dentition.
        It searches in provided list of lower jaws the presence of the jaw.

        :return: jaw type: 'upper' or 'lower'.
        """

        try:
            file = open('src/lower-jaw-list.txt', 'r', newline='')
        except FileNotFoundError:
            file = open('lower-jaw-list.txt', 'r', newline='')
        finally:
            lower_jaws = file.readlines()

            for lower_jaw in lower_jaws:
                if lower_jaw.strip() == self.path[self.path.rfind('/')+1:]:

                    return 'lower'

        return 'upper'

    def get_GTs(self):
        """
        Method loads ground truth landmark values from corresponding csv file and saves them in an numpy array.

        :return: None in annotation mode, 32x3 Matrix of (x, y, z) GT values in evaluation mode.
        """

        if self.mode == 'eval':
            try:
                file = open('../test-data/annotations/' + self.name + '.csv')
            except FileNotFoundError:
                file = open('test-data/annotations/' + self.name + '.csv')
            finally:
                values = np.loadtxt(file, delimiter=',', usecols=(0, 1, 2, 3), skiprows=1)

                notations = values[:, -1]
                for notation in config.VALID_NOTATIONS:
                    if notation not in notations:
                        values = np.append(values, [math.inf, math.inf, math.inf, notation])  # missing lm -> INF

                values = np.reshape(values, (-1, 4))  # reshape from flatten array into desired
                values = np.array(sorted(values, key=lambda x: x[-1]))  # sort landmarks according to the notation

                return values

        return None

    def apply_jaw_transformations(self):
        """
        Apply transformation on loaded jaw model in the scene.
        All necessary information for the transformations are within the JawObject instance parameters.

        :param jaw: Instance of JawObject.

        :return: VTK 4x4 transformation instance needed for the jaw translation.
        """

        transform = vtk.vtkTransform()

        '''Translation to the object mass center'''
        transform.Translate(self.mass_center[0], self.mass_center[1], self.mass_center[2])

        if False:
            '''Applying jaw transformation matrix trom given XML file'''
            transform.Concatenate(self.transformation_matrix)

            '''Rotation of upper jaws to be in the same position as lower'''
            if self.jaw_type == 'upper':
                transform.RotateWXYZ(180, 0, 1, 0)

        '''Translate by the mass center.'''
        transform.Translate(-self.mass_center[0], -self.mass_center[1], -self.mass_center[2])

        transformFilter = vtk.vtkTransformPolyDataFilter()
        transformFilter.SetTransform(transform)
        transformFilter.SetInputData(self.poly)
        transformFilter.Update()

        mpr = vtk.vtkPolyDataMapper()
        mpr.SetInputConnection(transformFilter.GetOutputPort())

        self.actor.SetMapper(mpr)

        return transform

    @staticmethod
    def get_transformation_matrix_from_xml(xml_path, model_number):
        """
        Method extracts data from an XML file. These data form a transformation matrix that is later
        used for the polygon transformation.

        :param xml_path: Path to the XML file from which the data are to be extracted.
        :param model_number: Within the XML file, matrices for more model numbers are present.
                             This parameter specifies which one should be parsed.

        :return: VTK transformation matrix to be applied on the 3D model.
        """

        try:
            '''Find the XML tree root.'''
            root = ElementTree.parse(xml_path).getroot()

            '''Extract models from the XML.'''
            models_in_xml = root.findall('Model')

            '''Find the right Model within the tree.'''
            my_matrix = models_in_xml[int(model_number)].find('TransformationMatrix').text.split(' ')

            transformation_matrix = vtk.vtkMatrix4x4()

            '''Parse the values into matrix elements.'''
            for i in range(4):
                for j in range(4):
                    transformation_matrix.SetElement(i, j, float(my_matrix[i * 4 + j]))

            '''Expand the matrix with for easier matrix operations.'''
            transformation_matrix = expand_affine_transformation_r_c(transformation_matrix)
            transformation_matrix.Transpose()

        except (OSError, ElementTree.ParseError) as _:
            transformation_matrix = vtk.vtkMatrix4x4()

            for i in range(4):
                for j in range(4):
                    transformation_matrix.SetElement(i, j, float(0))

            transformation_matrix = expand_affine_transformation_r_c(transformation_matrix)
            transformation_matrix.Transpose()

        return transformation_matrix


```

## Source file: src/trainer/JawDataset.py

{
  "path": "src/trainer/JawDataset.py",
  "blob_sha": "d94e9c7534c875f3b12dd39dd29d66f47cf80547",
  "sha256": "e6436e16c3f8906454cd2a98e9235e582f332b3fded6510f02e19cc5a91d3fd2",
  "lines": 180,
  "bytes": 5667,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 4,
    "rig": 1
  },
  "url": "https://github.com/tiborkubik/Robust-Teeth-Detection-in-3D-Dental-Scans/blob/da4fcdf64bd532d3d8d3007f019ed040ad1540ff/src/trainer/JawDataset.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def __init__(self, dataset_path, transform=None, input_type='depth'):"
    },
    {
      "line": 51,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 136,
      "text": "def __len__(self):"
    },
    {
      "line": 143,
      "text": "def __init__(self, net_input_renders, input_format='depth'):"
    },
    {
      "line": 147,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 175,
      "text": "def __len__(self):"
    },
    {
      "line": 179,
      "text": "def rgb2gray(rgb):"
    }
  ]
}

```text

"""
    :filename JawDataset.py
    :author Tibor Kubik
    :email xkubik34@stud.fit.vutbr.cz

    JawDataset class file.

    Dataset wrappers used for the data representation during the training procedure and evaluation.
"""

import os
import math
import glob
import torch
import numpy as np

from PIL import Image
from skimage.transform import resize
from torch.utils.data import Dataset

import config


class JawDataset(Dataset):
    """Dataset class used for training and validation."""

    def __init__(self, dataset_path, transform=None, input_type='depth'):
        self.landmarks = []

        self.dataset_path = dataset_path
        self.transform = transform
        self.input_type = input_type

        assert self.input_type in ['depth', 'geom', 'depth+geom']

        EXT = "*.csv"

        all_csv_files = [file
                         for path, subdir, files in os.walk(self.dataset_path)
                         for file in glob.glob(os.path.join(path, EXT))]

        for csv_name in all_csv_files:
            lms = np.genfromtxt(csv_name, delimiter=',', usecols=(0, 1, 2), skip_header=1)
            lms = lms[~np.isnan(lms).any(axis=1)]

            self.landmarks.append([lms, csv_name])

        print(len(self.landmarks))
        self.landmarks = self.landmarks[:2000]  # for dataset split...

    def __getitem__(self, index):
        img_path = str(self.landmarks[index][-1]).replace('landmarks', 'images')

        if self.input_type == 'depth':
            img_path = img_path[:-3]
            img_path = img_path + 'png'

            img = Image.open(img_path)
        elif self.input_type == 'geom':
            view_num = img_path[-6:-4]

            img_path = img_path[:-6]
            img_path = img_path + 'GEOM_' + view_num + '.png'

            img = Image.open(img_path)
            img = img.convert('L')
        else:
            # depth
            img_path1 = img_path[:-3]
            img_path1 = img_path1 + 'png'

            img_depth = Image.open(img_path1)
            img_depth = np.array(img_depth)

            # geom
            view_num = img_path[-6:-4]

            img_path2 = img_path[:-6]
            img_path2 = img_path2 + 'GEOM_' + view_num + '.png'

            img_geom = Image.open(img_path2)
            img_geom = img_geom.convert('L')
            img_geom = np.array(img_geom)

            img = np.zeros((config.DIMENSIONS['original'], config.DIMENSIONS['original'], 2), dtype=np.float32)

            img[:, :, 0] = img_depth[:, :]
            img[:, :, 1] = img_geom[:, :]

        img = np.array(img)
        img = img / 255  # [0, 1]
        # img = img[::-1, :]  # revise height in (height, width, channel)
        img = img.astype(float)

        # plt.figure()
        # show_landmarks(img, pd.read_csv(self.landmarks[index][-1]))
        # plt.show()

        heatmaps_dir_path = str(self.landmarks[index][-1][:-4]) + '/'

        heatmaps = []

        '''Labels are used just for the over-sampling, otherwise no need for them.'''
        if 10.0 in self.landmarks[index][0] and 160.0 in self.landmarks[index][0]:
            label = '1-and-8-present'
        elif 10.0 in self.landmarks[index][0]:
            label = '1-present'
        elif 160.0 in self.landmarks[index][0]:
            label = '8-present'
        else:
            label = 'missing-both'

        for notation in config.VALID_NOTATIONS:
            heatmap = Image.open(heatmaps_dir_path + str(notation) + '.png')
            heatmap = np.array(heatmap)

            # heatmap = resize(heatmap, (32, 32), anti_aliasing=True)
            # plt.imshow(heatmap)
            # plt.show()

            # plt.imshow(heatmap)
            # plt.show()

            heatmap = heatmap / 255  # [0, 1]
            heatmaps.append(heatmap)

        sample = {'image': img,
                  'landmarks': heatmaps,
                  'label': label}

        if self.transform:
            sample = self.transform(sample)

        return sample

    def __len__(self):
        return len(self.landmarks)


class EvalJawDataset(Dataset):
    """Dataset for loading the outputs of rendering pipeline in evaluation of unseen polygons."""

    def __init__(self, net_input_renders, input_format='depth'):
        self.net_input_renders = net_input_renders
        self.input_format = input_format

    def __getitem__(self, index):
        if self.input_format == 'depth':
            image = resize(self.net_input_renders[index]['depth_map'], (128, 128), anti_aliasing=True)
        elif self.input_format == 'geom':
            image = resize(self.net_input_renders[index]['geometry'], (128, 128), anti_aliasing=True)
            image = self.rgb2gray(image)

        else:
            img_depth = resize(self.net_input_renders[index]['depth_map'], (128, 128), anti_aliasing=True)
            img_geom = resize(self.net_input_renders[index]['geometry'], (128, 128), anti_aliasing=True)
            img_geom = self.rgb2gray(img_geom)

            image = np.zeros((128, 128, 2), dtype=np.float32)

            image[:, :, 0] = img_depth[:, :, 0]  # img.depth.shape: (128, 128, 1)
            image[:, :, 1] = img_geom[:, :]  # img_geom.shape: (128, 128)

        image = image.astype(float)

        image = torch.from_numpy(image.copy())

        if self.input_format == 'geom':
            image = image.unsqueeze(2)
        image = image.permute(2, 0, 1)

        return {'image': image,
                'camera': self.net_input_renders[index]['camera']}

    def __len__(self):
        return len(self.net_input_renders)

    @staticmethod
    def rgb2gray(rgb):
        return np.dot(rgb[..., :3], [0.2989, 0.5870, 0.1140])


```

## Source file: src/trainer/MissingTeethSampler.py

{
  "path": "src/trainer/MissingTeethSampler.py",
  "blob_sha": "86555ec84d91fcc17bd9358a1205381dafffaf60",
  "sha256": "6e08888abae2a7dc0b1851861b4d8c80acf566d5125572efd3056f3da747c83b",
  "lines": 55,
  "bytes": 1834,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 12
  },
  "url": "https://github.com/tiborkubik/Robust-Teeth-Detection-in-3D-Dental-Scans/blob/da4fcdf64bd532d3d8d3007f019ed040ad1540ff/src/trainer/MissingTeethSampler.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def __init__(self, dataset):"
    },
    {
      "line": 48,
      "text": "def get_label(self, idx):"
    },
    {
      "line": 51,
      "text": "def __iter__(self):"
    },
    {
      "line": 54,
      "text": "def __len__(self):"
    }
  ]
}

```text

"""
    :filename MissingTeethSampler.py
    :author Tibor Kubik
    :email xkubik34@stud.fit.vutbr.cz

    MissingTeethSampler class file.

    This class is a wrapper for PyTorch Sampler
    (https://pytorch.org/cppdocs/api/classtorch_1_1data_1_1samplers_1_1_sampler.html)
    and is used to provide more minority images in each batch.
"""

import torch
from torch.utils.data.sampler import Sampler


class MissingTeethSampler(Sampler):
    """
    Class samples the samples from dataset according to the number of samples of given class.
    It is used to over-sample the images that contain third molars, as these images are a minority class.
    No under-sampling or other over-sampling is applied.
    """
    def __init__(self, dataset):
        super(Sampler, self).__init__()

        self.dataset = dataset
        self.indices = list(range(len(self.dataset)))

        teeth_distribution = {}

        print('Sampling imbalanced teeth 1 and 8...')
        for idx in self.indices:
            label = self.get_label(idx)
            if label in teeth_distribution:
                teeth_distribution[label] += 1
            else:
                teeth_distribution[label] = 1
            print(f'{idx}/{len(self.indices)}: {label}')

        teeth_distribution['1-present'] *= 16
        teeth_distribution['8-present'] *= 16
        teeth_distribution['1-and-8-present'] *= 16

        '''Calculate the weights for classes.'''
        weights = [1.0 / teeth_distribution[self.get_label(idx)] for idx in self.indices]
        self.weights = torch.DoubleTensor(weights)

    def get_label(self, idx):
        return self.dataset[idx]['label']

    def __iter__(self):
        return iter(torch.multinomial(self.weights, len(self.indices), replacement=True).tolist())

    def __len__(self):
        return len(self.indices)


```

## Source file: src/train.py

{
  "path": "src/train.py",
  "blob_sha": "82974786ddebb6f0224c2044dc1ed309cc3cbbb4",
  "sha256": "aaef7d0e3444de787c24c385c8e263c722a88eabad41ae24f435c7cafd0105d0",
  "lines": 99,
  "bytes": 4669,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 1,
    "gum": 15
  },
  "url": "https://github.com/tiborkubik/Robust-Teeth-Detection-in-3D-Dental-Scans/blob/da4fcdf64bd532d3d8d3007f019ed040ad1540ff/src/train.py",
  "function_signature_candidates": []
}

```text

"""
    :filename train.py
    :author Tibor Kubik
    :email xkubik34@stud.fit.vutbr.cz

    This script is used to train a neural network on the dataset of 2D depth maps of dentition.
    Goal is to train a neural network in a regression way, which can predict landmarks in a form of heatmaps.

    The training hyperparameters as well as the neural network architecture selection can be set in two ways:
        (i) By modifying the config.py file, which contains dictionaries with the hyperparameter values,
        (ii) or by provided script arguments.

        The only obligatory argument is the neural network architecture, which can be one of:
        (i) Batch Normalization U-Net: argument [-n|--network-name]=BatchNormUNet,
        (ii) Attention U-Net: argument [-n|--network-name]=AttUNet,
        (iii) or Nested U-Net: argument [-n|--network-name]=NestedUNet.
"""

import torch
import logging
import argparse

from torchsummary import summary

import config
from networks.UNet import UNet
from trainer.Trainer import Trainer
from networks.AttUNet import AttUNet
from networks.NestedUNet import NestedUNet


if __name__ == '__main__':
    logging.basicConfig(level=logging.INFO, format='%(levelname)s: %(message)s')

    '''Argument parsing for the config overwrite.'''
    parser = argparse.ArgumentParser(description='UNet for teeth landmark detection training',
                                     formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument('-e', '--epochs', metavar='E', type=int,
                        default=config.HYPERPARAMETERS['epochs'], help='Number of epochs', dest='epochs')
    parser.add_argument('-b', '--batch-size', metavar='B', type=int, nargs='?',
                        default=config.HYPERPARAMETERS['batch_size'], help='Batch size', dest='batch_size')
    parser.add_argument('-w', '--weight-decay', metavar='WD', type=float, nargs='?',
                        default=config.HYPERPARAMETERS['weight_decay'], help='Weight decay', dest='weight_decay')
    parser.add_argument('-l', '--learning-rate', metavar='LR', type=float, nargs='?',
                        default=config.HYPERPARAMETERS['learning_rate'], help='Learning rate', dest='lr')
    parser.add_argument('-n', '--network-name', metavar='NN', type=str,
                        default='BatchNormUNet', help='Network name', dest='network_name')
    parser.add_argument('-i', '--input-format', metavar='IF', type=str,
                        default='depth', help='Input type (depth, geom, depth+geom', dest='input_format')
    parser.add_argument('-f', '--folder-path', metavar='FP', type=str, help='Folder path to prepared'
                                                                            '2D renders and landmark heatmaps',
                        dest='folder_path')
    args = parser.parse_args()

    assert args.network_name in ['BatchNormUNet', 'AttUNet', 'NestedUNet']
    assert args.input_format in ['depth', 'geom', 'depth+geom']

    '''Better GPU performance.'''
    torch.backends.cudnn.benchmark = True

    in_channels = 1

    if args.input_format == 'depth+geom':
        in_channels = 2

    '''Creation of chosen network instance.'''
    if args.network_name == 'BatchNormUNet':
        network = UNet(in_channels=in_channels, out_channels=config.LANDMARKS_NUM, batch_norm=True, decoder_mode='upconv')
    elif args.network_name == 'AttUNet':
        network = AttUNet(in_channels=in_channels, out_channels=config.LANDMARKS_NUM)
    else:
        network = NestedUNet(in_channels=in_channels, out_channels=config.LANDMARKS_NUM)

    '''Device setup - strongly recommend the usage of NVIDIA GPU (cuda).'''
    device = torch.device('cuda:0' if torch.cuda.is_available() else 'cpu')
    network.to(device)

    '''
    Device and network information.
    Please uncomment last line if you want to check the network layers in the log window.
    '''
    logging.info(f'Device used: {device}')
    logging.info(f'GPU name: {torch.cuda.get_device_name(0)}')
    # logging.info(summary(network, input_size=(1, config.DIMENSIONS['input_net'], config.DIMENSIONS['input_net'])))

    try:
        trainer = Trainer(network=network,
                          device=device,
                          epochs=args.epochs,
                          batch_size=args.batch_size,
                          weight_decay=args.weight_decay,
                          lr=args.lr,
                          input_type=args.input_format,
                          dataset_path=args.folder_path)

        trainer.training()

    except KeyboardInterrupt:
        torch.save(network.state_dict(), 'interrupted_model.pt')


```
