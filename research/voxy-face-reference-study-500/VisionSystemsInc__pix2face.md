# Facial rendering source scan: VisionSystemsInc/pix2face

Repository: https://github.com/VisionSystemsInc/pix2face

Pinned source revision: c68b635bac4f8e00fe8da8f87cd71c0d32b9bd7c

Discovery topic: human rendering

Description: 3D human face estimation and rendering from a single image

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["scripts/basel_to_surrey_rigid_transform.txt"], "face": ["docker/build_pix2face_sources.bsh", "docker/run_pix2face_interactively.bsh", "docker/run_pix2face_notebook.bsh", "docker/scripts/build_pix2face.bsh", "notebooks/pix2face_demo.ipynb", "pix2face_env.bsh", "pix2face_teaser.png", "python/pix2face_estimation/__init__.py", "python/pix2face_estimation/camera_decomposition.py", "python/pix2face_estimation/camera_estimation.py", "python/pix2face_estimation/coefficient_estimation.py", "python/pix2face_estimation/geometry_utils.py", "python/pix2face_estimation/mesh_renderer.py", "python/pix2face_estimation/subject_blend.py", "scripts/align_face_mesh.py", "scripts/pix2face_demo.py"]}

## Source file: scripts/align_face_mesh.py

{
  "path": "scripts/align_face_mesh.py",
  "blob_sha": "e238e319a84bf7207cb7b13526ec3338c7942549",
  "sha256": "633a095f671446b33bd04ab5ab942584a5c56d9c2643fec82de81065b8c06258",
  "lines": 89,
  "bytes": 3611,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 2,
    "blend_shape": 4,
    "deform": 3,
    "rig": 1,
    "face": 6
  },
  "url": "https://github.com/VisionSystemsInc/pix2face/blob/c68b635bac4f8e00fe8da8f87cd71c0d32b9bd7c/scripts/align_face_mesh.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def fmt_print(string, array):"
    }
  ]
}

```text

import eos
import numpy as np
import os
import sys

if __name__ == '__main__':
    script_dir = os.path.dirname(os.path.realpath(__file__))
    data_dir = os.path.join(script_dir, '..', 'face3d', 'data_3DMM')
    if not os.path.isdir(data_dir):
        print("Could not find directory %s. Did you download the data using download_data.bsh?" % data_dir)
        sys.exit(-1)
    model_path = os.path.join(data_dir, "sfm_shape_3448.bin")
    if not os.path.isfile(model_path):
        print("Could not find model at %s. Did you download the data using download_data.bsh?" % model_path)
        sys.exit(-1)
    model = eos.morphablemodel.load_model(model_path)
    mean_model = model.get_mean()
    shape_model = model.get_shape_model()
    texture_coords = np.array(model.get_texture_coordinates())
    mean_vertices = np.array(mean_model.vertices)
    triangles = np.array(shape_model.get_triangle_list())
    transform_path = os.path.join(script_dir, "basel_to_surrey_rigid_transform.txt")

    b_to_s = np.loadtxt(transform_path)
    s_to_b = np.linalg.inv(b_to_s)

    mean_vertices = np.hstack((mean_vertices, np.ones((mean_vertices.shape[0], 1))))
    mean_vertices = (s_to_b.dot(mean_vertices.T) ).T[:,0:3]

    def fmt_print(string, array):
        return '\n'.join([string.format(*x) for x in array])

    ply_str = '''ply
    format ascii 1.0
    comment surrey face mesh
    element vertex {}
    property float x
    property float y
    property float z
    property float s
    property float t
    element face {}
    property list uchar int vertex_indices
    end_header
    {}
    {}
    '''.format(
        len(mean_vertices),
        len(triangles),
        fmt_print("{} {} {} {} {}", np.hstack((mean_vertices, texture_coords))),
        fmt_print("3 {} {} {}", triangles))
    mean_face_head = ply_str

    # subject
    pca_coeff_stds_subject = np.sqrt(shape_model.get_eigenvalues()).reshape(-1,1)
    pca_coeff_ranges_subject = np.hstack((pca_coeff_stds_subject,pca_coeff_stds_subject)) * np.array((-4,4))
    pca_components_subject = shape_model.get_orthonormal_pca_basis().T
    # expression
    deformations = []
    names = []
    blend_shapes_path = os.path.join(data_dir, "expression_blendshapes_3448.bin")
    if not os.path.isfile(blend_shapes_path):
        print("Could not find the blended shapes at %s. Did you download the data using download_data.bsh?" % blend_shapes_path)
        sys.exit()
    blend = eos.morphablemodel.load_blendshapes(blend_shapes_path)
    for b in blend:
        deformations.append(b.deformation)
        names.append(b.name)

    pca_coeff_stds_expression = np.ones((6,1))
    pca_coeff_ranges_expression = np.ones((6,2)) * np.array((-4,4))
    pca_components_expression = np.array(deformations)
    expression_meanings = " ".join(names)
# save

    x = os.path.join(data_dir,"expression_meanings.txt")
    with open(x, 'w') as f:
        f.write(expression_meanings)

    x = os.path.join(data_dir,"mean_face_head.ply")
    with open(x, 'w') as f:
        f.write(mean_face_head)

    np.save(os.path.join(data_dir, "pca_coeff_ranges_expression.npy"), pca_coeff_ranges_expression)
    np.save(os.path.join(data_dir, "pca_coeff_ranges_subject.npy"), pca_coeff_ranges_subject)
    np.save(os.path.join(data_dir, "pca_coeff_stds_expression.npy"), pca_coeff_stds_expression)
    np.save(os.path.join(data_dir, "pca_coeff_stds_subject.npy"), pca_coeff_stds_subject)
    np.save(os.path.join(data_dir, "pca_components_expression.npy"), pca_components_expression)
    np.save(os.path.join(data_dir, "pca_components_subject.npy"), pca_components_subject)


```

## Source file: scripts/pix2face_demo.py

{
  "path": "scripts/pix2face_demo.py",
  "blob_sha": "5764306b3e5b0c3959664b0e8366ac6b9d57850a",
  "sha256": "4204867df382730c631d251e94ad190dcc57150ccea37467c4a6988d99608401",
  "lines": 95,
  "bytes": 4117,
  "selection_score": 13,
  "keyword_line_counts": {
    "rig": 1,
    "face": 14
  },
  "url": "https://github.com/VisionSystemsInc/pix2face/blob/c68b635bac4f8e00fe8da8f87cd71c0d32b9bd7c/scripts/pix2face_demo.py",
  "function_signature_candidates": []
}

```text

""" This Script Demonstrates the basic image -> PNCC + offsets --> coefficient estimation --> 3D Jitter pipeline. """

import numpy as np
import os
import pix2face_estimation.geometry_utils as geometry_utils
import face3d
import vxl
import pix2face
from PIL import Image


# Estimate PNCC and Offsets using pix2face network

cuda_device = 0

this_dir = os.path.dirname(__file__)
pix2face_data_dir = os.path.join(this_dir, '../pix2face_net/data/')

model = pix2face.test.load_pretrained_model(cuda_device=cuda_device)

img_fname = os.path.join(pix2face_data_dir, 'CASIA_0000107_004.jpg')
img = np.array(Image.open(img_fname))
print('Estimating PNCC + Offsets..')
outputs = pix2face.test.test(model, [img,], cuda_device=cuda_device)
pncc = outputs[0][0]
offsets = outputs[0][1]
print('..Done')


pvr_data_dir = os.path.join(this_dir, '../face3d/data_3DMM/')
debug_dir = ''
debug_mode = False

num_subject_coeffs = 199  # max 199
num_expression_coeffs = 29  # max 29

# load needed data files
head_mesh = face3d.head_mesh(pvr_data_dir)
subject_components = np.load(os.path.join(pvr_data_dir, 'pca_components_subject.npy'))
expression_components = np.load(os.path.join(pvr_data_dir, 'pca_components_expression.npy'))
subject_ranges = np.load(os.path.join(pvr_data_dir,'pca_coeff_ranges_subject.npy'))
expression_ranges = np.load(os.path.join(pvr_data_dir,'pca_coeff_ranges_expression.npy'))

# keep only the PCA components that we will be estimating
subject_components = vxl.vnl.matrix(subject_components[0:num_subject_coeffs,:])
expression_components = vxl.vnl.matrix(expression_components[0:num_expression_coeffs,:])
subject_ranges = vxl.vnl.matrix(subject_ranges[0:num_subject_coeffs,:])
expression_ranges = vxl.vnl.matrix(expression_ranges[0:num_expression_coeffs,:])

# create rendering object (encapsulates OpenGL context)
renderer = face3d.mesh_renderer()
# create coefficient estimator
coeff_estimator = face3d.media_coefficient_from_PNCC_and_offset_estimator(head_mesh, subject_components, expression_components, subject_ranges, expression_ranges, debug_mode, debug_dir)

# Estimate Coefficients from PNCC and Offsets
print('Estimating Coefficients..')
img_ids = ['img0',]
coeffs, result = coeff_estimator.estimate_coefficients_perspective(img_ids, [pncc,], [offsets,])
if not result.success:
    raise Exception('ERROR estimating coefficents for ' + img_fname)
print('..Done.')

# Print Yaw, Pitch, Roll of Head
R_cam = np.array(coeffs.camera(0).rotation.as_matrix())  # rotation matrix of estimated camera
R0 = np.diag((1,-1,-1))  # R0 is the rotation matrix of a frontal camera
R_head = np.dot(R0,R_cam)
yaw, pitch, roll = geometry_utils.matrix_to_Euler_angles(R_head, order='YXZ')
print('yaw, pitch, roll = %0.1f, %0.1f, %0.1f (degrees)' % (np.rad2deg(yaw), np.rad2deg(pitch), np.rad2deg(roll)))

# Render 3D-Jittered Images
print('Rendering Jittered Images..')
jitterer = face3d.media_jitterer_perspective([img,], coeffs, head_mesh, subject_components, expression_components, renderer, "")

# manually alter expression
new_expression_coeffs = np.zeros_like(coeffs.expression_coeffs(0))
new_expression_coeffs[1] = 2.0
new_expression_coeffs[2] = -2.0
new_expression_coeffs[14] = 1.5
render_expr = jitterer.render(coeffs.camera(0), coeffs.subject_coeffs(), new_expression_coeffs, subject_components, expression_components)


# manually alter pose
delta_R = vxl.vgl.rotation_3d(geometry_utils.Euler_angles_to_quaternion(np.pi/3, 0, 0, order='YXZ'))
cam = coeffs.camera(0)
new_R = cam.rotation * delta_R
new_cam = face3d.perspective_camera_parameters(cam.focal_len, cam.principal_point, new_R, cam.translation, cam.nx, cam.ny)
render_rot = jitterer.render(new_cam, coeffs.subject_coeffs(), coeffs.expression_coeffs(0), subject_components, expression_components)
print('..Done.')

# save out results
output_dir = this_dir
print('Saving results')
Image.fromarray(img).save(os.path.join(output_dir, "image_original.jpg"))
Image.fromarray(render_expr[:,:,0:3]).save(os.path.join(output_dir, "image_expression_jitter.jpg"))
Image.fromarray(render_rot[:,:,0:3]).save(os.path.join(output_dir, "image_pose_jitter.jpg"))


```

## Source file: python/pix2face_estimation/__init__.py

{
  "path": "python/pix2face_estimation/__init__.py",
  "blob_sha": "4cdb0f0988a182e71cb8cf239b6c45a670cfac9d",
  "sha256": "b832c2841a9f128104de4a27e5ceb98ee7b8b94744e54a33eb4c37f6bf8867c0",
  "lines": 3,
  "bytes": 97,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/VisionSystemsInc/pix2face/blob/c68b635bac4f8e00fe8da8f87cd71c0d32b9bd7c/python/pix2face_estimation/__init__.py",
  "function_signature_candidates": []
}

```text

from . import mesh_renderer
from . import camera_estimation
from . import coefficient_estimation


```

## Source file: python/pix2face_estimation/mesh_renderer.py

{
  "path": "python/pix2face_estimation/mesh_renderer.py",
  "blob_sha": "9a9115f3200f58b93d6aed4c2db311aad7f38e67",
  "sha256": "4a8f66352760e41d74316ea2d555d0b742eb2bd4170ef90fbe0be575cf974a11",
  "lines": 11,
  "bytes": 259,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/VisionSystemsInc/pix2face/blob/c68b635bac4f8e00fe8da8f87cd71c0d32b9bd7c/python/pix2face_estimation/mesh_renderer.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def get_mesh_renderer():"
    }
  ]
}

```text

import face3d

_mesh_renderer = None


def get_mesh_renderer():
    global _mesh_renderer
    # get the mesh renderer, which is created only once, lazily
    if _mesh_renderer is None:
        _mesh_renderer = face3d.mesh_renderer()
    return _mesh_renderer


```
