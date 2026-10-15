# Facial rendering source scan: zhaofuq/Instant-NSR

Repository: https://github.com/zhaofuq/Instant-NSR

Pinned source revision: c008cee25818c460f19966e7cec996770068e4a0

Discovery topic: human rendering

Description: Human Performance Modeling and Rendering via Neural Animated Mesh (Published in SIGGRAPH Asia 2022)

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["ffmlp/include/cutlass/LICENSE.txt", "license"]

Topic paths: {"lip": ["flip/__init__.py", "flip/main.py", "flip/utils.py"]}

## Source file: flip/main.py

{
  "path": "flip/main.py",
  "blob_sha": "d854382199ba9aa07aec13f123b56762e3ab7529",
  "sha256": "d8cdd30b1560268865fe2eaaf8f66a33946d7447800305d301da1d9638f906f4",
  "lines": 66,
  "bytes": 2849,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 5,
    "lip": 6
  },
  "url": "https://github.com/zhaofuq/Instant-NSR/blob/c008cee25818c460f19966e7cec996770068e4a0/flip/main.py",
  "function_signature_candidates": []
}

```text

#########################################################################
# Copyright (c) 2020, NVIDIA CORPORATION. All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
#  * Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
#  * Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#  * Neither the name of NVIDIA CORPORATION nor the names of its
#    contributors may be used to endorse or promote products derived
#    from this software without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY
# EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT OWNER OR
# CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
# EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
# PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
# PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
# OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
#########################################################################

# FLIP: A Difference Evaluator for Alternating Images
# High Performance Graphics, 2020.
# by Pontus Andersson, Jim Nilsson, Tomas Akenine-Moller, Magnus Oskarsson, Kalle Astrom, and Mark D. Fairchild
#
# Pointer to our paper: https://research.nvidia.com/publication/2020-07_FLIP
# code by Pontus Andersson, Jim Nilsson, and Tomas Akenine-Moller

import numpy as np

from flip import compute_flip
from utils import *

if __name__ == '__main__':
	# Set viewing conditions
	monitor_distance = 0.7
	monitor_width = 0.7
	monitor_resolution_x = 3840
	
	# Compute number of pixels per degree of visual angle
	pixels_per_degree = monitor_distance * (monitor_resolution_x / monitor_width) * (np.pi / 180)
	
	# Load sRGB images
	reference = load_image_array('../images/reference.png')
	test = load_image_array('../images/test.png')

	# Compute FLIP map
	deltaE = compute_flip(reference, test, pixels_per_degree)

	# Save error map
	index_map = np.floor(255.0 * deltaE.squeeze(0))

	use_color_map = True
	if use_color_map:
		result = CHWtoHWC(index2color(index_map, get_magma_map()))
	else:
		result = index_map / 255.0
	save_image("../images/flip.png", result)
	print("Done")


```

## Source file: flip/utils.py

{
  "path": "flip/utils.py",
  "blob_sha": "e5a8b95bc08d4fe302c1714c6c2d2e65badc1473",
  "sha256": "0535257e05173cdb6992c9624c778014165e52f6cd663c22f243c878455c8142",
  "lines": 324,
  "bytes": 15162,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 5,
    "lip": 3
  },
  "url": "https://github.com/zhaofuq/Instant-NSR/blob/c008cee25818c460f19966e7cec996770068e4a0/flip/utils.py",
  "function_signature_candidates": [
    {
      "line": 39,
      "text": "def HWCtoCHW(x):"
    },
    {
      "line": 43,
      "text": "def CHWtoHWC(x):"
    },
    {
      "line": 47,
      "text": "def save_image(img_file, img):"
    },
    {
      "line": 52,
      "text": "def load_image_array(img_file):"
    },
    {
      "line": 59,
      "text": "def index2color(index_map, color_map):"
    },
    {
      "line": 66,
      "text": "def get_magma_map():"
    }
  ]
}

```text

#########################################################################
# Copyright (c) 2020, NVIDIA CORPORATION. All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
#  * Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
#  * Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#  * Neither the name of NVIDIA CORPORATION nor the names of its
#    contributors may be used to endorse or promote products derived
#    from this software without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY
# EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT OWNER OR
# CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
# EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
# PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
# PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
# OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
#########################################################################

# FLIP: A Difference Evaluator for Alternating Images
# High Performance Graphics, 2020.
# by Pontus Andersson, Jim Nilsson, Tomas Akenine-Moller, Magnus Oskarsson, Kalle Astrom, and Mark D. Fairchild
#
# Pointer to our paper: https://research.nvidia.com/publication/2020-07_FLIP
# code by Pontus Andersson, Jim Nilsson, and Tomas Akenine-Moller

import numpy as np
from PIL import Image

def HWCtoCHW(x):
	return np.rollaxis(x, 2)


def CHWtoHWC(x):
	return np.swapaxes(np.swapaxes(x, 0, 1), 1, 2)


def save_image(img_file, img):
	img_array = (np.clip(img, 0.0, 1.0) * 255.0).astype(np.uint8)
	im = Image.fromarray(img_array)
	im.save(img_file)

def load_image_array(img_file):
    img = Image.open(img_file, 'r').convert('RGB')
    img = np.asarray(img).astype(np.float32)
    img = HWCtoCHW(img)
    img = img / 255.0
    return img

def index2color(index_map, color_map):
	dim = index_map.shape
	index_map = index_map.flatten().astype(int)
	column_stacked_colors = color_map[index_map, :]
	heat_map = np.reshape(column_stacked_colors.transpose(), (3, dim[0], dim[1]))
	return heat_map

def get_magma_map():
    # Source: https://bids.github.io/colormap/
    color_map = [[0.001462, 0.000466, 0.013866],
                [0.002258, 0.001295, 0.018331],
                [0.003279, 0.002305, 0.023708],
                [0.004512, 0.003490, 0.029965],
                [0.005950, 0.004843, 0.037130],
                [0.007588, 0.006356, 0.044973],
                [0.009426, 0.008022, 0.052844],
                [0.011465, 0.009828, 0.060750],
                [0.013708, 0.011771, 0.068667],
                [0.016156, 0.013840, 0.076603],
                [0.018815, 0.016026, 0.084584],
                [0.021692, 0.018320, 0.092610],
                [0.024792, 0.020715, 0.100676],
                [0.028123, 0.023201, 0.108787],
                [0.031696, 0.025765, 0.116965],
                [0.035520, 0.028397, 0.125209],
                [0.039608, 0.031090, 0.133515],
                [0.043830, 0.033830, 0.141886],
                [0.048062, 0.036607, 0.150327],
                [0.052320, 0.039407, 0.158841],
                [0.056615, 0.042160, 0.167446],
                [0.060949, 0.044794, 0.176129],
                [0.065330, 0.047318, 0.184892],
                [0.069764, 0.049726, 0.193735],
                [0.074257, 0.052017, 0.202660],
                [0.078815, 0.054184, 0.211667],
                [0.083446, 0.056225, 0.220755],
                [0.088155, 0.058133, 0.229922],
                [0.092949, 0.059904, 0.239164],
                [0.097833, 0.061531, 0.248477],
                [0.102815, 0.063010, 0.257854],
                [0.107899, 0.064335, 0.267289],
                [0.113094, 0.065492, 0.276784],
                [0.118405, 0.066479, 0.286321],
                [0.123833, 0.067295, 0.295879],
                [0.129380, 0.067935, 0.305443],
                [0.135053, 0.068391, 0.315000],
                [0.140858, 0.068654, 0.324538],
                [0.146785, 0.068738, 0.334011],
                [0.152839, 0.068637, 0.343404],
                [0.159018, 0.068354, 0.352688],
                [0.165308, 0.067911, 0.361816],
                [0.171713, 0.067305, 0.370771],
                [0.178212, 0.066576, 0.379497],
                [0.184801, 0.065732, 0.387973],
                [0.191460, 0.064818, 0.396152],
                [0.198177, 0.063862, 0.404009],
                [0.204935, 0.062907, 0.411514],
                [0.211718, 0.061992, 0.418647],
                [0.218512, 0.061158, 0.425392],
                [0.225302, 0.060445, 0.431742],
                [0.232077, 0.059889, 0.437695],
                [0.238826, 0.059517, 0.443256],
                [0.245543, 0.059352, 0.448436],
                [0.252220, 0.059415, 0.453248],
                [0.258857, 0.059706, 0.457710],
                [0.265447, 0.060237, 0.461840],
                [0.271994, 0.060994, 0.465660],
                [0.278493, 0.061978, 0.469190],
                [0.284951, 0.063168, 0.472451],
                [0.291366, 0.064553, 0.475462],
                [0.297740, 0.066117, 0.478243],
                [0.304081, 0.067835, 0.480812],
                [0.310382, 0.069702, 0.483186],
                [0.316654, 0.071690, 0.485380],
                [0.322899, 0.073782, 0.487408],
                [0.329114, 0.075972, 0.489287],
                [0.335308, 0.078236, 0.491024],
                [0.341482, 0.080564, 0.492631],
                [0.347636, 0.082946, 0.494121],
                [0.353773, 0.085373, 0.495501],
                [0.359898, 0.087831, 0.496778],
                [0.366012, 0.090314, 0.497960],
                [0.372116, 0.092816, 0.499053],
                [0.378211, 0.095332, 0.500067],
                [0.384299, 0.097855, 0.501002],
                [0.390384, 0.100379, 0.501864],
                [0.396467, 0.102902, 0.502658],
                [0.402548, 0.105420, 0.503386],
                [0.408629, 0.107930, 0.504052],
                [0.414709, 0.110431, 0.504662],
                [0.420791, 0.112920, 0.505215],
                [0.426877, 0.115395, 0.505714],
                [0.432967, 0.117855, 0.506160],
                [0.439062, 0.120298, 0.506555],
                [0.445163, 0.122724, 0.506901],
                [0.451271, 0.125132, 0.507198],
                [0.457386, 0.127522, 0.507448],
                [0.463508, 0.129893, 0.507652],
                [0.469640, 0.132245, 0.507809],
                [0.475780, 0.134577, 0.507921],
                [0.481929, 0.136891, 0.507989],
                [0.488088, 0.139186, 0.508011],
                [0.494258, 0.141462, 0.507988],
                [0.500438, 0.143719, 0.507920],
                [0.506629, 0.145958, 0.507806],
                [0.512831, 0.148179, 0.507648],
                [0.519045, 0.150383, 0.507443],
                [0.525270, 0.152569, 0.507192],
                [0.531507, 0.154739, 0.506895],
                [0.537755, 0.156894, 0.506551],
                [0.544015, 0.159033, 0.506159],
                [0.550287, 0.161158, 0.505719],
                [0.556571, 0.163269, 0.505230],
                [0.562866, 0.165368, 0.504692],
                [0.569172, 0.167454, 0.504105],
                [0.575490, 0.169530, 0.503466],
                [0.581819, 0.171596, 0.502777],
                [0.588158, 0.173652, 0.502035],
                [0.594508, 0.175701, 0.501241],
                [0.600868, 0.177743, 0.500394],
                [0.607238, 0.179779, 0.499492],
                [0.613617, 0.181811, 0.498536],
                [0.620005, 0.183840, 0.497524],
                [0.626401, 0.185867, 0.496456],
                [0.632805, 0.187893, 0.495332],
                [0.639216, 0.189921, 0.494150],
                [0.645633, 0.191952, 0.492910],
                [0.652056, 0.193986, 0.491611],
                [0.658483, 0.196027, 0.490253],
                [0.664915, 0.198075, 0.488836],
                [0.671349, 0.200133, 0.487358],
                [0.677786, 0.202203, 0.485819],
                [0.684224, 0.204286, 0.484219],
                [0.690661, 0.206384, 0.482558],
                [0.697098, 0.208501, 0.480835],
                [0.703532, 0.210638, 0.479049],
                [0.709962, 0.212797, 0.477201],
                [0.716387, 0.214982, 0.475290],
                [0.722805, 0.217194, 0.473316],
                [0.729216, 0.219437, 0.471279],
                [0.735616, 0.221713, 0.469180],
                [0.742004, 0.224025, 0.467018],
                [0.748378, 0.226377, 0.464794],
                [0.754737, 0.228772, 0.462509],
                [0.761077, 0.231214, 0.460162],
                [0.767398, 0.233705, 0.457755],
                [0.773695, 0.236249, 0.455289],
                [0.779968, 0.238851, 0.452765],
                [0.786212, 0.241514, 0.450184],
                [0.792427, 0.244242, 0.447543],
                [0.798608, 0.247040, 0.444848],
                [0.804752, 0.249911, 0.442102],
                [0.810855, 0.252861, 0.439305],
                [0.816914, 0.255895, 0.436461],
                [0.822926, 0.259016, 0.433573],
                [0.828886, 0.262229, 0.430644],
                [0.834791, 0.265540, 0.427671],
                [0.840636, 0.268953, 0.424666],
                [0.846416, 0.272473, 0.421631],
                [0.852126, 0.276106, 0.418573],
                [0.857763, 0.279857, 0.415496],
                [0.863320, 0.283729, 0.412403],
                [0.868793, 0.287728, 0.409303],
                [0.874176, 0.291859, 0.406205],
                [0.879464, 0.296125, 0.403118],
                [0.884651, 0.300530, 0.400047],
                [0.889731, 0.305079, 0.397002],
                [0.894700, 0.309773, 0.393995],
                [0.899552, 0.314616, 0.391037],
                [0.904281, 0.319610, 0.388137],
                [0.908884, 0.324755, 0.385308],
                [0.913354, 0.330052, 0.382563],
                [0.917689, 0.335500, 0.379915],
                [0.921884, 0.341098, 0.377376],
                [0.925937, 0.346844, 0.374959],
                [0.929845, 0.352734, 0.372677],
                [0.933606, 0.358764, 0.370541],
                [0.937221, 0.364929, 0.368567],
                [0.940687, 0.371224, 0.366762],
                [0.944006, 0.377643, 0.365136],
                [0.947180, 0.384178, 0.363701],
                [0.950210, 0.390820, 0.362468],
                [0.953099, 0.397563, 0.361438],
                [0.955849, 0.404400, 0.360619],
                [0.958464, 0.411324, 0.360014],
                [0.960949, 0.418323, 0.359630],
                [0.963310, 0.425390, 0.359469],
                [0.965549, 0.432519, 0.359529],
                [0.967671, 0.439703, 0.359810],
                [0.969680, 0.446936, 0.360311],
                [0.971582, 0.454210, 0.361030],
                [0.973381, 0.461520, 0.361965],
                [0.975082, 0.468861, 0.363111],
                [0.976690, 0.476226, 0.364466],
                [0.978210, 0.483612, 0.366025],
                [0.979645, 0.491014, 0.367783],
                [0.981000, 0.498428, 0.369734],
                [0.982279, 0.505851, 0.371874],
                [0.983485, 0.513280, 0.374198],
                [0.984622, 0.520713, 0.376698],
                [0.985693, 0.528148, 0.379371],
                [0.986700, 0.535582, 0.382210],
                [0.987646, 0.543015, 0.385210],
                [0.988533, 0.550446, 0.388365],
                [0.989363, 0.557873, 0.391671],
                [0.990138, 0.565296, 0.395122],
                [0.990871, 0.572706, 0.398714],
                [0.991558, 0.580107, 0.402441],
                [0.992196, 0.587502, 0.406299],
                [0.992785, 0.594891, 0.410283],
                [0.993326, 0.602275, 0.414390],
                [0.993834, 0.609644, 0.418613],
                [0.994309, 0.616999, 0.422950],
                [0.994738, 0.624350, 0.427397],
                [0.995122, 0.631696, 0.431951],
                [0.995480, 0.639027, 0.436607],
                [0.995810, 0.646344, 0.441361],
                [0.996096, 0.653659, 0.446213],
                [0.996341, 0.660969, 0.451160],
                [0.996580, 0.668256, 0.456192],
                [0.996775, 0.675541, 0.461314],
                [0.996925, 0.682828, 0.466526],
                [0.997077, 0.690088, 0.471811],
                [0.997186, 0.697349, 0.477182],
                [0.997254, 0.704611, 0.482635],
                [0.997325, 0.711848, 0.488154],
                [0.997351, 0.719089, 0.493755],
                [0.997351, 0.726324, 0.499428],
                [0.997341, 0.733545, 0.505167],
                [0.997285, 0.740772, 0.510983],
                [0.997228, 0.747981, 0.516859],
                [0.997138, 0.755190, 0.522806],
                [0.997019, 0.762398, 0.528821],
                [0.996898, 0.769591, 0.534892],
                [0.996727, 0.776795, 0.541039],
                [0.996571, 0.783977, 0.547233],
                [0.996369, 0.791167, 0.553499],
                [0.996162, 0.798348, 0.559820],
                [0.995932, 0.805527, 0.566202],
                [0.995680, 0.812706, 0.572645],
                [0.995424, 0.819875, 0.579140],
                [0.995131, 0.827052, 0.585701],
                [0.994851, 0.834213, 0.592307],
                [0.994524, 0.841387, 0.598983],
                [0.994222, 0.848540, 0.605696],
                [0.993866, 0.855711, 0.612482],
                [0.993545, 0.862859, 0.619299],
                [0.993170, 0.870024, 0.626189],
                [0.992831, 0.877168, 0.633109],
                [0.992440, 0.884330, 0.640099],
                [0.992089, 0.891470, 0.647116],
                [0.991688, 0.898627, 0.654202],
                [0.991332, 0.905763, 0.661309],
                [0.990930, 0.912915, 0.668481],
                [0.990570, 0.920049, 0.675675],
                [0.990175, 0.927196, 0.682926],
                [0.989815, 0.934329, 0.690198],
                [0.989434, 0.941470, 0.697519],
                [0.989077, 0.948604, 0.704863],
                [0.988717, 0.955742, 0.712242],
                [0.988367, 0.962878, 0.719649],
                [0.988033, 0.970012, 0.727077],
                [0.987691, 0.977154, 0.734536],
                [0.987387, 0.984288, 0.742002],
                [0.987053, 0.991438, 0.749504]]
    return np.asarray(color_map)

```

## Source file: flip/__init__.py

{
  "path": "flip/__init__.py",
  "blob_sha": "3b082fee03041ecbb1b9b59f76da9e51ab05041d",
  "sha256": "dd2c7a824dc925e372dae10e6b7960827e4ee4701924dcf825664b8c44332617",
  "lines": 333,
  "bytes": 14504,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 5,
    "lip": 4
  },
  "url": "https://github.com/zhaofuq/Instant-NSR/blob/c008cee25818c460f19966e7cec996770068e4a0/flip/__init__.py",
  "function_signature_candidates": [
    {
      "line": 39,
      "text": "def color_space_transform(input_color, fromSpace2toSpace):"
    },
    {
      "line": 154,
      "text": "def generate_spatial_filter(pixels_per_degree, channel):"
    },
    {
      "line": 197,
      "text": "def spatial_filter(img, s_a, s_rg, s_by, radius):"
    },
    {
      "line": 219,
      "text": "def hunt_adjustment(img):"
    },
    {
      "line": 233,
      "text": "def hyab(reference, test):"
    },
    {
      "line": 238,
      "text": "def redistribute_errors(power_deltaE_hyab, cmax):"
    },
    {
      "line": 252,
      "text": "def feature_detection(imgy, pixels_per_degree, feature_type):"
    },
    {
      "line": 286,
      "text": "def compute_flip(reference, test, pixels_per_degree):"
    }
  ]
}

```text

#########################################################################
# Copyright (c) 2020, NVIDIA CORPORATION. All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
#  * Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
#  * Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#  * Neither the name of NVIDIA CORPORATION nor the names of its
#    contributors may be used to endorse or promote products derived
#    from this software without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY
# EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT OWNER OR
# CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
# EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
# PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
# PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
# OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
#########################################################################

# FLIP: A Difference Evaluator for Alternating Images
# High Performance Graphics, 2020.
# by Pontus Andersson, Jim Nilsson, Tomas Akenine-Moller, Magnus Oskarsson, Kalle Astrom, and Mark D. Fairchild
#
# Pointer to our paper: https://research.nvidia.com/publication/2020-07_FLIP
# code by Pontus Andersson, Jim Nilsson, and Tomas Akenine-Moller

import numpy as np
from scipy import signal

def color_space_transform(input_color, fromSpace2toSpace):
    dim = input_color.shape

    if fromSpace2toSpace == "srgb2linrgb":
        limit = 0.04045
        transformed_color = np.where(input_color > limit, np.power((input_color + 0.055) / 1.055, 2.4), input_color / 12.92)

    elif fromSpace2toSpace == "linrgb2srgb":
        limit = 0.0031308
        transformed_color = np.where(input_color > limit, 1.055 * (input_color ** (1.0 / 2.4)) - 0.055, 12.92 * input_color)

    elif fromSpace2toSpace == "linrgb2xyz" or fromSpace2toSpace == "xyz2linrgb":
        # Source: https://www.image-engineering.de/library/technotes/958-how-to-convert-between-srgb-and-ciexyz
        # Assumes D65 standard illuminant
        a11 = 10135552 / 24577794
        a12 = 8788810  / 24577794
        a13 = 4435075  / 24577794
        a21 = 2613072  / 12288897
        a22 = 8788810  / 12288897
        a23 = 887015   / 12288897
        a31 = 1425312  / 73733382
        a32 = 8788810  / 73733382
        a33 = 70074185 / 73733382
        A = np.array([[a11, a12, a13],
                        [a21, a22, a23],
                        [a31, a32, a33]])

        input_color = np.transpose(input_color, (2, 0, 1)) # C(H*W)
        if fromSpace2toSpace == "xyz2linrgb":
            A = np.linalg.inv(A)
        transformed_color = np.matmul(A, input_color)
        transformed_color = np.transpose(transformed_color, (1, 2, 0))

    elif fromSpace2toSpace == "xyz2ycxcz":
        reference_illuminant = color_space_transform(np.ones(dim), 'linrgb2xyz')
        input_color = np.divide(input_color, reference_illuminant)
        y = 116 * input_color[1:2, :, :] - 16
        cx = 500 * (input_color[0:1, :, :] - input_color[1:2, :, :])
        cz = 200 * (input_color[1:2, :, :] - input_color[2:3, :, :])
        transformed_color = np.concatenate((y, cx, cz), 0)

    elif fromSpace2toSpace == "ycxcz2xyz":
        y = (input_color[0:1, :, :] + 16) / 116
        cx = input_color[1:2, :, :] / 500
        cz = input_color[2:3, :, :] / 200

        x = y + cx
        z = y - cz
        transformed_color = np.concatenate((x, y, z), 0)

        reference_illuminant = color_space_transform(np.ones(dim), 'linrgb2xyz')
        transformed_color = np.multiply(transformed_color, reference_illuminant)

    elif fromSpace2toSpace == "xyz2lab":
        reference_illuminant = color_space_transform(np.ones(dim), 'linrgb2xyz')
        input_color = np.divide(input_color, reference_illuminant)
        delta = 6 / 29
        limit = 0.00885

        input_color = np.where(input_color > limit, np.power(input_color, 1 / 3), (input_color / (3 * delta * delta)) + (4 / 29))

        l = 116 * input_color[1:2, :, :] - 16
        a = 500 * (input_color[0:1,:, :] - input_color[1:2, :, :])
        b = 200 * (input_color[1:2, :, :] - input_color[2:3, :, :])

        transformed_color = np.concatenate((l, a, b), 0)

    elif fromSpace2toSpace == "lab2xyz":
        y = (input_color[0:1, :, :] + 16) / 116
        a =  input_color[1:2, :, :] / 500
        b =  input_color[2:3, :, :] / 200

        x = y + a
        z = y - b

        xyz = np.concatenate((x, y, z), 0)
        delta = 6 / 29
        xyz = np.where(xyz > delta,  xyz ** 3, 3 * delta ** 2 * (xyz - 4 / 29))

        reference_illuminant = color_space_transform(np.ones(dim), 'linrgb2xyz')
        transformed_color = np.multiply(xyz, reference_illuminant)

    elif fromSpace2toSpace == "srgb2xyz":
        transformed_color = color_space_transform(input_color, 'srgb2linrgb')
        transformed_color = color_space_transform(transformed_color,'linrgb2xyz')
    elif fromSpace2toSpace == "srgb2ycxcz":
        transformed_color = color_space_transform(input_color, 'srgb2linrgb')
        transformed_color = color_space_transform(transformed_color, 'linrgb2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2ycxcz')
    elif fromSpace2toSpace == "linrgb2ycxcz":
        transformed_color = color_space_transform(input_color, 'linrgb2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2ycxcz')
    elif fromSpace2toSpace == "srgb2lab":
        transformed_color = color_space_transform(input_color, 'srgb2linrgb')
        transformed_color = color_space_transform(transformed_color, 'linrgb2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2lab')
    elif fromSpace2toSpace == "linrgb2lab":
        transformed_color = color_space_transform(input_color, 'linrgb2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2lab')
    elif fromSpace2toSpace == "ycxcz2linrgb":
        transformed_color = color_space_transform(input_color, 'ycxcz2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2linrgb')
    elif fromSpace2toSpace == "lab2srgb":
        transformed_color = color_space_transform(input_color, 'lab2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2linrgb')
        transformed_color = color_space_transform(transformed_color, 'linrgb2srgb')
    elif fromSpace2toSpace == "ycxcz2lab":
        transformed_color = color_space_transform(input_color, 'ycxcz2xyz')
        transformed_color = color_space_transform(transformed_color, 'xyz2lab')
    else:
        print('The color transform is not defined!')
        transformed_color = input_color

    return transformed_color

def generate_spatial_filter(pixels_per_degree, channel):
    a1_A = 1 
    b1_A = 0.0047
    a2_A = 0
    b2_A = 1e-5 # avoid division by 0
    a1_rg = 1
    b1_rg = 0.0053
    a2_rg = 0
    b2_rg = 1e-5 # avoid division by 0
    a1_by = 34.1
    b1_by = 0.04
    a2_by = 13.5
    b2_by = 0.025
    if channel == "A": #Achromatic CSF
        a1 = a1_A
        b1 = b1_A
        a2 = a2_A
        b2 = b2_A
    elif channel == "RG": #Red-Green CSF
        a1 = a1_rg
        b1 = b1_rg
        a2 = a2_rg
        b2 = b2_rg
    elif channel == "BY": # Blue-Yellow CSF
        a1 = a1_by
        b1 = b1_by
        a2 = a2_by
        b2 = b2_by

    # Determine evaluation domain
    max_scale_parameter = max([b1_A, b2_A, b1_rg, b2_rg, b1_by, b2_by])
    r = np.ceil(3 * np.sqrt(max_scale_parameter / (2 * np.pi**2)) * pixels_per_degree)
    r = int(r)
    deltaX = 1.0 / pixels_per_degree
    x, y = np.meshgrid(range(-r, r + 1), range(-r, r + 1))
    z = (x * deltaX)**2 + (y * deltaX)**2
    
    # Generate weights
    g = a1 * np.sqrt(np.pi / b1) * np.exp(-np.pi**2 * z / b1) + a2 * np.sqrt(np.pi / b2) * np.exp(-np.pi**2 * z / b2)
    g = g / np.sum(g)

    return g, r

def spatial_filter(img, s_a, s_rg, s_by, radius):
    # Filters image img using Contrast Sensitivity Functions.
    # Returns linear RGB

    dim = img.shape
    # Prepare convolution input
    img_pad_a = np.pad(img[0:1, :, :], ((0, 0), (radius, radius), (radius, radius)), mode='edge')
    img_pad_rg = np.pad(img[1:2, :, :], ((0, 0), (radius, radius), (radius, radius)), mode='edge')
    img_pad_by = np.pad(img[2:3, :, :], ((0, 0), (radius, radius), (radius, radius)), mode='edge')

    # Apply Gaussian filters
    img_tilde_opponent = np.zeros((dim[0], dim[1], dim[2]))
    img_tilde_opponent[0:1, :, :] = signal.convolve2d(img_pad_a.squeeze(0), s_a, mode='valid')
    img_tilde_opponent[1:2, :, :] = signal.convolve2d(img_pad_rg.squeeze(0), s_rg, mode='valid')
    img_tilde_opponent[2:3, :, :] = signal.convolve2d(img_pad_by.squeeze(0), s_by, mode='valid')

    # Transform to linear RGB for clamp
    img_tilde_linear_rgb = color_space_transform(img_tilde_opponent, 'ycxcz2linrgb')
    
    # Clamp to RGB box
    return np.clip(img_tilde_linear_rgb, 0.0, 1.0)

def hunt_adjustment(img):
    # Applies Hunt adjustment to L*a*b* image img
    
    # Extract luminance component
    L = img[0:1, :, :]
    
    # Apply Hunt adjustment
    img_h = np.zeros(img.shape)
    img_h[0:1, :, :] = L
    img_h[1:2, :, :] = np.multiply((0.01 * L), img[1:2, :, :])
    img_h[2:3, :, :] = np.multiply((0.01 * L), img[2:3, :, :])

    return img_h

def hyab(reference, test):
    # Computes HyAB distance between L*a*b* images reference and test
    delta = reference - test
    return abs(delta[0:1, :, :]) + np.linalg.norm(delta[1:3, :, :], axis=0)

def redistribute_errors(power_deltaE_hyab, cmax):
    # Set redistribution parameters
    pc = 0.4
    pt = 0.95
    
    # Re-map error to 0-1 range. Values between 0 and
    # pccmax are mapped to the range [0, pt],
    # while the rest are mapped to the range (pt, 1]
    deltaE_c = np.zeros(power_deltaE_hyab.shape)
    pccmax = pc * cmax
    deltaE_c = np.where(power_deltaE_hyab < pccmax, (pt / pccmax) * power_deltaE_hyab, pt + ((power_deltaE_hyab - pccmax) / (cmax - pccmax)) * (1.0 - pt))

    return deltaE_c

def feature_detection(imgy, pixels_per_degree, feature_type):
    # Finds features of type feature_type in image img based on current PPD
    
    # Set peak to trough value (2x standard deviations) of human edge
    # detection filter
    w = 0.082
    
    # Compute filter radius
    sd = 0.5 * w * pixels_per_degree
    radius = int(np.ceil(3 * sd))

    # Compute 2D Gaussian
    [x, y] = np.meshgrid(range(-radius, radius+1), range(-radius, radius+1))
    g = np.exp(-(x ** 2 + y ** 2) / (2 * sd * sd))
    
    if feature_type == 'edge': # Edge detector
        # Compute partial derivative in x-direction
        Gx = np.multiply(-x, g)
    else: # Point detector
        # Compute second partial derivative in x-direction
        Gx = np.multiply(x ** 2 / (sd * sd) - 1, g)
 
    # Normalize positive weights to sum to 1 and negative weights to sum to -1
    negative_weights_sum = -np.sum(Gx[Gx < 0])
    positive_weights_sum = np.sum(Gx[Gx > 0])
    Gx = np.where(Gx < 0, Gx / negative_weights_sum, Gx / positive_weights_sum)
    
    # Detect features
    imgy_pad = np.pad(imgy, ((0, 0), (radius, radius), (radius, radius)), mode='edge').squeeze(0)
    featuresX = signal.convolve2d(imgy_pad, Gx, mode='valid')
    featuresY = signal.convolve2d(imgy_pad, np.transpose(Gx), mode='valid')

    return np.stack((featuresX, featuresY))

def compute_flip(reference, test, pixels_per_degree):
    assert reference.shape == test.shape

    # Set color and feature exponents
    qc = 0.7
    qf = 0.5

    # Transform reference and test to opponent color space
    reference = color_space_transform(reference, 'srgb2ycxcz')
    test = color_space_transform(test, 'srgb2ycxcz')

    # --- Color pipeline ---
    # Spatial filtering
    s_a, radius_a = generate_spatial_filter(pixels_per_degree, 'A')
    s_rg, radius_rg = generate_spatial_filter(pixels_per_degree, 'RG')
    s_by, radius_by = generate_spatial_filter(pixels_per_degree, 'BY')
    radius = max(radius_a, radius_rg, radius_by)
    filtered_reference = spatial_filter(reference, s_a, s_rg, s_by, radius)
    filtered_test = spatial_filter(test, s_a, s_rg, s_by, radius)

    # Perceptually Uniform Color Space
    preprocessed_reference = hunt_adjustment(color_space_transform(filtered_reference, 'linrgb2lab'))
    preprocessed_test = hunt_adjustment(color_space_transform(filtered_test, 'linrgb2lab'))

    # Color metric
    deltaE_hyab = hyab(preprocessed_reference, preprocessed_test)
    hunt_adjusted_green = hunt_adjustment(color_space_transform(np.array([[[0.0]], [[1.0]], [[0.0]]]), 'linrgb2lab'))
    hunt_adjusted_blue = hunt_adjustment(color_space_transform(np.array([[[0.0]], [[0.0]], [[1.0]]]), 'linrgb2lab'))
    cmax = np.power(hyab(hunt_adjusted_green, hunt_adjusted_blue), qc)
    deltaE_c = redistribute_errors(np.power(deltaE_hyab, qc), cmax)

    # --- Feature pipeline ---
    # Extract and normalize achromatic component
    reference_y = (reference[0:1, :, :] + 16) / 116
    test_y = (test[0:1, :, :] + 16) / 116

    # Edge and point detection
    edges_reference = feature_detection(reference_y, pixels_per_degree, 'edge')
    points_reference = feature_detection(reference_y, pixels_per_degree, 'point')
    edges_test = feature_detection(test_y, pixels_per_degree, 'edge')
    points_test = feature_detection(test_y, pixels_per_degree, 'point')

    # Feature metric
    deltaE_f = np.maximum(abs(np.linalg.norm(edges_reference, axis=0) - np.linalg.norm(edges_test, axis=0)), abs(np.linalg.norm(points_test, axis=0) - np.linalg.norm(points_reference, axis=0)))
    deltaE_f = np.power(((1 / np.sqrt(2)) * deltaE_f), qf)

    # --- Final error ---
    return np.power(deltaE_c, 1 - deltaE_f)


```

## Source file: loss.py

{
  "path": "loss.py",
  "blob_sha": "b238f939b5610b1b5f19a8a99dc47e20c7279e10",
  "sha256": "ceb43b56e730716213e6cd55672604ffd97982a62b8f4a91086fbbffd5a0305a",
  "lines": 11,
  "bytes": 268,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/zhaofuq/Instant-NSR/blob/c008cee25818c460f19966e7cec996770068e4a0/loss.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def mape_loss(pred, target):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F

def mape_loss(pred, target):
    # pred, target: [B, 1], torch tenspr
    difference = (pred - target).abs()
    scale = 1 / (target.abs() + 1e-2)
    loss = difference * scale

    return loss.mean()

```
