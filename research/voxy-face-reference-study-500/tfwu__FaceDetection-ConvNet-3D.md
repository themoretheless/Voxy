# Facial rendering source scan: tfwu/FaceDetection-ConvNet-3D

Repository: https://github.com/tfwu/FaceDetection-ConvNet-3D

Pinned source revision: f9251c48eb40c5aec8fba7455115c355466555be

Discovery topic: face model 3D

Description: Source code for our ECCV16 paper, Face Detection with End-to-End Integration of a ConvNet and a 3D Model

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "R-package/LICENSE", "dmlc-core/LICENSE", "mshadow/LICENSE", "ps-lite/LICENSE", "scala-package/LICENSE"]

Topic paths: {"gum": ["R-package/man/arguments.Rd"], "rig": ["README-mxnet-original.md"], "face": ["doc/rnn_interface.md", "docs/system/rnn_interface.md", "mshadow/mshadow/extension/face3dproj_backward.h", "mshadow/mshadow/extension/face3dproj_forward.h", "mshadow/mshadow/extension/face_element_sum_forward.h", "src/operator/face_3d_proj-inl.h", "src/operator/face_3d_proj.cc", "src/operator/face_3d_proj.cu", "src/operator/face_element_sum-inl.h", "src/operator/face_element_sum.cc", "src/operator/face_element_sum.cu"], "lip": ["AFLW/MATLAB/ellipse.m", "R-package/man/mx.nd.clip.Rd", "src/operator/rec2ellipse-inl.h", "src/operator/rec2ellipse.cc", "src/operator/rec2ellipse.cu"]}

## Source file: src/operator/rec2ellipse.cc

{
  "path": "src/operator/rec2ellipse.cc",
  "blob_sha": "8c5d88b32bd551c2775c16556e7b85b58dd776d8",
  "sha256": "0bd9c5d040e6b367b3e5f7278bd86af31cf6c6c05384b4c2b505c1acf2ad71b6",
  "lines": 22,
  "bytes": 580,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "lip": 10
  },
  "url": "https://github.com/tfwu/FaceDetection-ConvNet-3D/blob/f9251c48eb40c5aec8fba7455115c355466555be/src/operator/rec2ellipse.cc",
  "function_signature_candidates": []
}

```text

#include "./rec2ellipse-inl.h"

namespace mxnet {
namespace op {
template<>
Operator *CreateOp<cpu>(Rec2EllipseParam param) {
	return new Rec2EllipseOp<cpu>(param);
}

Operator* Rec2EllipseProp::CreateOperator(Context ctx) const {
	DO_BIND_DISPATCH(CreateOp, param_);
}

DMLC_REGISTER_PARAMETER(Rec2EllipseParam);

MXNET_REGISTER_OP_PROPERTY(Rec2Ellipse, Rec2EllipseProp)
.describe("Rec to ellipse.")
.add_argument("data", "Symbol", "Input data to Rec2Ellipse.")
.add_argument("offset", "Symbol", "Input label to Rec2Ellipse.")
.add_arguments(Rec2EllipseParam::__FIELDS__());
}
}


```

## Source file: src/operator/rec2ellipse.cu

{
  "path": "src/operator/rec2ellipse.cu",
  "blob_sha": "b516bfcf5a5cebba35f8a878e4aca2f3bf3bea72",
  "sha256": "244f3202461bca1ce2a34cda6c1d0b8aad63de947c3cdb8823e8b11acf4a2cba",
  "lines": 11,
  "bytes": 173,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 3
  },
  "url": "https://github.com/tfwu/FaceDetection-ConvNet-3D/blob/f9251c48eb40c5aec8fba7455115c355466555be/src/operator/rec2ellipse.cu",
  "function_signature_candidates": []
}

```text

#include "./rec2ellipse-inl.h"

namespace mxnet {
namespace op {
template <>
Operator *CreateOp<gpu>(Rec2EllipseParam param) {
	return new Rec2EllipseOp<gpu>(param);
}

}
}


```

## Source file: src/operator/face_3d_proj.cc

{
  "path": "src/operator/face_3d_proj.cc",
  "blob_sha": "3af5170c9777215ee3c8a14db44b6c906c6d8099",
  "sha256": "41c25f97704a6ef21c316f7b6559a27eb557f77cddc296d48d3018b0b40623f7",
  "lines": 22,
  "bytes": 565,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "face": 10
  },
  "url": "https://github.com/tfwu/FaceDetection-ConvNet-3D/blob/f9251c48eb40c5aec8fba7455115c355466555be/src/operator/face_3d_proj.cc",
  "function_signature_candidates": []
}

```text

#include "./face_3d_proj-inl.h"

namespace mxnet {
namespace op {
template<>
Operator *CreateOp<cpu>(Face3DProjParam param) {
	return new Face3DProjOp<cpu>(param);
}

Operator* Face3DProjProp::CreateOperator(Context ctx) const {
	DO_BIND_DISPATCH(CreateOp, param_);
}

DMLC_REGISTER_PARAMETER(Face3DProjParam);

MXNET_REGISTER_OP_PROPERTY(Face3DProj, Face3DProjProp)
.describe("Face 3d projection layer.")
.add_argument("data", "Symbol", "Input data to Face3DProj.")
.add_argument("mface", "Symbol", "Mean face.")
.add_arguments(Face3DProjParam::__FIELDS__());
}
}


```

## Source file: src/operator/face_3d_proj.cu

{
  "path": "src/operator/face_3d_proj.cu",
  "blob_sha": "7e2c8c838810461d60b9d0cd0955d59bfd5721ec",
  "sha256": "e37b8509b35378b281c566cadaf20d1f52283b13da9fc33c13bffff695541e10",
  "lines": 11,
  "bytes": 171,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 3
  },
  "url": "https://github.com/tfwu/FaceDetection-ConvNet-3D/blob/f9251c48eb40c5aec8fba7455115c355466555be/src/operator/face_3d_proj.cu",
  "function_signature_candidates": []
}

```text

#include "./face_3d_proj-inl.h"

namespace mxnet {
namespace op {
template<>
Operator *CreateOp<gpu>(Face3DProjParam param) {
	return new Face3DProjOp<gpu>(param);
}

}
}


```
