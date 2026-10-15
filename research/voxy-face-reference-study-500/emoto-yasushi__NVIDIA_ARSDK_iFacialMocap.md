# Facial rendering source scan: emoto-yasushi/NVIDIA_ARSDK_iFacialMocap

Repository: https://github.com/emoto-yasushi/NVIDIA_ARSDK_iFacialMocap

Pinned source revision: ccbe82cb141cd427753f1ede7d0981135fb65cba

Discovery topic: facial mocap

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["OSS/LICENSE", "OSS/samples/external/Imgui/imgui-docking/LICENSE.txt", "OSS/samples/external/Imgui/imgui-docking/examples/libs/glfw/COPYING.txt", "OSS/samples/external/nlohmann/json/LICENSE.MIT"]

Topic paths: {"shader": ["OSS/build/samples/ExpressionApp/ExpressionApp.dir/Release/GLShaders.obj", "OSS/samples/ExpressionApp/BackEndOpenGL/GLShaders.cpp", "OSS/samples/ExpressionApp/BackEndOpenGL/GLShaders.h", "OSS/samples/external/Imgui/imgui-docking/backends/vulkan/glsl_shader.frag", "OSS/samples/external/Imgui/imgui-docking/backends/vulkan/glsl_shader.vert"], "rig": ["OSS/samples/external/glm/include/glm/detail/func_trigonometric.hpp", "OSS/samples/external/glm/include/glm/detail/func_trigonometric.inl", "OSS/samples/external/glm/include/glm/detail/func_trigonometric_simd.inl", "OSS/samples/external/glm/include/glm/ext/quaternion_trigonometric.hpp", "OSS/samples/external/glm/include/glm/ext/quaternion_trigonometric.inl", "OSS/samples/external/glm/include/glm/gtx/fast_trigonometry.hpp", "OSS/samples/external/glm/include/glm/gtx/fast_trigonometry.inl", "OSS/samples/external/glm/include/glm/simd/trigonometric.h", "OSS/samples/external/glm/include/glm/trigonometric.hpp"], "face": ["OSS/build/Release/FaceTrack.exe", "OSS/build/samples/ExpressionApp/ExpressionApp.dir/Release/FaceIO.obj", "OSS/build/samples/FaceTrack/CMakeFiles/generate.stamp", "OSS/build/samples/FaceTrack/CMakeFiles/generate.stamp.depend", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceEngine.obj", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.log", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.obj", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/CL.command.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/CL.read.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/CL.write.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/FaceTrack.lastbuildstate", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/FaceTrack.write.1u.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/custombuild.command.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/custombuild.read.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/custombuild.write.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/link.command.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/link.read.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FaceTrack.tlog/link.write.1.tlog", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/FeatureVertexName.obj", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/RenderingUtils.obj", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/_IsIncrementalBuild", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/nvARProxy.obj", "OSS/build/samples/FaceTrack/FaceTrack.dir/Release/nvCVImageProxy.obj", "OSS/build/samples/FaceTrack/FaceTrack.vcxproj", "OSS/build/samples/FaceTrack/FaceTrack.vcxproj.filters", "OSS/build/samples/FaceTrack/Release/FaceTrack.exp", "OSS/build/samples/FaceTrack/Release/FaceTrack.lib", "OSS/build/samples/FaceTrack/cmake_install.cmake", "OSS/samples/ExpressionApp/BackEndOpenGL/FaceIO.cpp", "OSS/samples/ExpressionApp/BackEndOpenGL/FaceIO.h"], "lip": ["OSS/samples/external/glm/include/glm/ext/matrix_clip_space.hpp", "OSS/samples/external/glm/include/glm/ext/matrix_clip_space.inl"]}

## Source file: OSS/samples/FaceTrack/FaceEngine.h

{
  "path": "OSS/samples/FaceTrack/FaceEngine.h",
  "blob_sha": "27932b995e5f5875f81306655bc2c893d15a7ab5",
  "sha256": "0c42382103315e0448e36b89e88d08df6798faf3189906a252638e29b78b754a",
  "lines": 214,
  "bytes": 7866,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 4,
    "face": 30
  },
  "url": "https://github.com/emoto-yasushi/NVIDIA_ARSDK_iFacialMocap/blob/ccbe82cb141cd427753f1ede7d0981135fb65cba/OSS/samples/FaceTrack/FaceEngine.h",
  "function_signature_candidates": [
    {
      "line": 49,
      "text": "void reset() {"
    },
    {
      "line": 60,
      "text": "void reset(float Q, float R) {"
    },
    {
      "line": 66,
      "text": "float update(float val) {"
    },
    {
      "line": 82,
      "text": "bool CheckResult(NvCV_Status nvErr, unsigned line);"
    },
    {
      "line": 123,
      "text": "void setInputImageWidth(int width) { input_image_width = width; }"
    },
    {
      "line": 124,
      "text": "void setInputImageHeight(int height) { input_image_height = height; }"
    },
    {
      "line": 125,
      "text": "int getInputImageWidth() { return input_image_width; }"
    },
    {
      "line": 126,
      "text": "int getInputImageHeight() { return input_image_height; }"
    },
    {
      "line": 127,
      "text": "int getInputImagePitch() { return input_image_pitch = input_image_width * 3 * sizeof(unsigned char); }"
    },
    {
      "line": 128,
      "text": "void setFaceModel(const char *faceModel) { face_model = faceModel; }"
    },
    {
      "line": 134,
      "text": "void destroyFeatures();"
    },
    {
      "line": 135,
      "text": "void destroyFaceDetectionFeature();"
    },
    {
      "line": 136,
      "text": "void destroyLandmarkDetectionFeature();"
    },
    {
      "line": 137,
      "text": "void destroyFaceFittingFeature();"
    },
    {
      "line": 142,
      "text": "void releaseFeatureIOParams();"
    },
    {
      "line": 143,
      "text": "void releaseFaceDetectionIOParams();"
    },
    {
      "line": 144,
      "text": "void releaseLandmarkDetectionIOParams();"
    },
    {
      "line": 145,
      "text": "void releaseFaceFittingIOParams();"
    },
    {
      "line": 154,
      "text": "float getAverageLandmarksConfidence();"
    },
    {
      "line": 155,
      "text": "void enlargeAndSquarifyImageBox(float enlarge, NvAR_Rect& box, int FLAG_variant);"
    },
    {
      "line": 156,
      "text": "static void jiggleBox(std::mt19937& ran, float minMag, float maxMag, const NvAR_Rect& cleanBox, NvAR_Rect& noisyBox);"
    },
    {
      "line": 165,
      "text": "void setFaceStabilization(bool);"
    },
    {
      "line": 166,
      "text": "int getNumShapeEigenvalues();"
    },
    {
      "line": 167,
      "text": "int getNumExpressionCoefficients();"
    },
    {
      "line": 169,
      "text": "int getNumLandmarks() { return numLandmarks; }"
    },
    {
      "line": 170,
      "text": "void DrawPose(const cv::Mat& src, const NvAR_Quaternion* pose) const;"
    },
    {
      "line": 212,
      "text": "void setAppMode(FaceEngine::mode _mAppMode);"
    }
  ]
}

```text

/*###############################################################################
#
# Copyright 2020-2021 NVIDIA Corporation
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of
# this software and associated documentation files (the "Software"), to deal in
# the Software without restriction, including without limitation the rights to
# use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
# the Software, and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
# FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
# COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
# IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
# CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#
###############################################################################*/
#ifndef __FACE_ENGINE__
#define __FACE_ENGINE__

#include <random>
#include "nvAR.h"
#include "nvCVOpenCV.h"
#include "opencv2/opencv.hpp"
#include "FeatureVertexName.h"
#define FITFACE_PRIVATE

class KalmanFilter1D {
 private:
  float Q_;          // Covariance of the process noise
  float xhat_;       // Current prediction
  float xhatminus_;  // Previous prediction
  float P_;          // Estimated accuracy of xhat_
  float Pminus_;     // Previous P_
  float K_;          // Kalman gain
  float R_;          // Covariance of the observation noise
  bool bFirstUse;

 public:
  KalmanFilter1D() { reset(); }

  KalmanFilter1D(float Q, float R) { reset(Q, R); }

  void reset() {
    R_ = 0.005f * 0.005f;
    Q_ = 1e-5f;
    xhat_ = 0.0f;
    xhatminus_ = 0.0f;
    P_ = 1;
    bFirstUse = true;
    Pminus_ = 0.0f;
    K_ = 0.0f;
  }

  void reset(float Q, float R) {
    reset();
    Q_ = Q;
    R_ = R;
  }

  float update(float val) {
    if (bFirstUse) {
      xhat_ = val;
      bFirstUse = false;
    }

    xhatminus_ = xhat_;
    Pminus_ = P_ + Q_;
    K_ = Pminus_ / (Pminus_ + R_);
    xhat_ = xhatminus_ + K_ * (val - xhatminus_);
    P_ = (1 - K_) * Pminus_;

    return xhat_;
  }
};

bool CheckResult(NvCV_Status nvErr, unsigned line);

#define BAIL_IF_ERR(err)                 \
do {                                     \
    if (0 != (err)) {                    \
      goto bail;                         \
    }                                    \
  } while (0)

#define BAIL_IF_NVERR(nvErr, err, code)  \
  do {                                   \
    if (!CheckResult(nvErr, __LINE__)) { \
      err = code;                        \
      goto bail;                         \
    }                                    \
  } while (0)

typedef struct LandmarksProperties {
  int numPoints;
  float confidence_threshold;
}LandmarksProperties;

/********************************************************************************
 * FaceEngine
 ********************************************************************************/

class FaceEngine {
 public:
  enum Err { errNone, errGeneral, errRun, errInitialization, errRead, errEffect, errParameter };
  int input_image_width, input_image_height, input_image_pitch;
  const LandmarksProperties LANDMARKS_INFO[2] = {
           { 68,  15.0f }, // number of landmark points, confidence threshold value
           { 126, 15.0f}, // 
  };

  // Keep Older Confidence Threshold for Linux Models.
  const LandmarksProperties LANDMARKS_INFO_UNIX[2] = {
           { 68,  10.0f }, // number of landmark points, confidence threshold value
           { 126, 5.0f}, // 
  };

  void setInputImageWidth(int width) { input_image_width = width; }
  void setInputImageHeight(int height) { input_image_height = height; }
  int getInputImageWidth() { return input_image_width; }
  int getInputImageHeight() { return input_image_height; }
  int getInputImagePitch() { return input_image_pitch = input_image_width * 3 * sizeof(unsigned char); }
  void setFaceModel(const char *faceModel) { face_model = faceModel; }

  Err createFeatures(const char* modelPath, unsigned int _batchSize = 1, unsigned int mode = 0);
  Err createFaceDetectionFeature(const char* modelPath, CUstream stream);
  Err createLandmarkDetectionFeature(const char* modelPath, unsigned int batchSize, CUstream stream, unsigned int mode = 0);
  Err createFaceFittingFeature(const char* modelPath, CUstream stream);
  void destroyFeatures();
  void destroyFaceDetectionFeature();
  void destroyLandmarkDetectionFeature();
  void destroyFaceFittingFeature();
  Err initFeatureIOParams();
  Err initFaceDetectionIOParams(NvCVImage* _inputImageBuffer);
  Err initLandmarkDetectionIOParams(NvCVImage* _inputImageBuffer);
  Err initFaceFittingIOParams(NvCVImage* _inputImageBuffer);
  void releaseFeatureIOParams();
  void releaseFaceDetectionIOParams();
  void releaseLandmarkDetectionIOParams();
  void releaseFaceFittingIOParams();

  unsigned findFaceBoxes();
  NvAR_Rect* getLargestBox();
  NvCV_Status findLandmarks();
  NvAR_BBoxes* getBoundingBoxes();
  NvAR_Point2f* getLandmarks();
  NvAR_Quaternion* getPose();
  float* getLandmarksConfidence();
  float getAverageLandmarksConfidence();
  void enlargeAndSquarifyImageBox(float enlarge, NvAR_Rect& box, int FLAG_variant);
  static void jiggleBox(std::mt19937& ran, float minMag, float maxMag, const NvAR_Rect& cleanBox, NvAR_Rect& noisyBox);
  unsigned findLargestFaceBox(NvAR_Rect& faceBox, int variant = 0);
  unsigned acquireFaceBox(cv::Mat& src, NvAR_Rect& faceBox, int variant = 0); 
  unsigned acquireFaceBoxAndLandmarks(cv::Mat& src, NvAR_Point2f* refMarks, NvAR_Rect& faceBox, int variant = 0);
  Err fitFaceModel(cv::Mat& frame);
  NvAR_FaceMesh* getFaceMesh();
  NvAR_RenderingParams* getRenderingParams();
  float* getShapeEigenvalues();
  float* getExpressionCoefficients();
  void setFaceStabilization(bool);
  int getNumShapeEigenvalues();
  int getNumExpressionCoefficients();
  Err setNumLandmarks(int);
  int getNumLandmarks() { return numLandmarks; }
  void DrawPose(const cv::Mat& src, const NvAR_Quaternion* pose) const;
  std::array<float, 2> GetAverageLandmarkPositionInGlSpace() const;

  NvCVImage inputImageBuffer{}, tmpImage{}, outputImageBuffer{};
  NvAR_FeatureHandle faceDetectHandle{}, landmarkDetectHandle{}, faceFitHandle{};
  std::vector<NvAR_Point2f> facial_landmarks;
  std::vector<float> facial_landmarks_confidence;
  std::vector<NvAR_Quaternion> facial_pose;
  NvAR_FaceMesh* face_mesh{};
  std::vector<NvAR_Vector3f> m_vertices;
  std::vector<NvAR_Vector3u16> m_triangles;
  NvAR_RenderingParams* rendering_params{};
  std::vector<float> shapeEigenvalues, expressionCoefficients;
  CUstream stream{};
  std::vector<NvAR_Rect> output_bbox_data;
  std::vector<float> output_bbox_conf_data;
  NvAR_BBoxes output_bboxes{};
  int batchSize;
  std::mt19937 ran;
  int numLandmarks;
  float confidenceThreshold;
  std::string face_model;
  
  bool bStabilizeFace;
  bool bUseOTAU;
  char *fdOTAModelPath, *ldOTAModelPath;

  FaceEngine() {
    batchSize = 1;
    bStabilizeFace = true;
    numLandmarks = LANDMARKS_INFO[0].numPoints;
    confidenceThreshold = LANDMARKS_INFO[0].confidence_threshold;
    appMode = faceMeshGeneration;
    input_image_width = 640;
    input_image_height = 480;
    input_image_pitch = 3 * input_image_width * sizeof(unsigned char);  // RGB
    bUseOTAU = false;
    fdOTAModelPath = NULL;
    ldOTAModelPath = NULL;
  }
  enum mode { faceDetection = 0, landmarkDetection, faceMeshGeneration
  } appMode;
  void setAppMode(FaceEngine::mode _mAppMode);
};
#endif


```

## Source file: OSS/samples/FaceTrack/FaceTrack.cpp

{
  "path": "OSS/samples/FaceTrack/FaceTrack.cpp",
  "blob_sha": "c78c87ecd0716d5191913dff03c391a2ba47a383",
  "sha256": "e9964cfa3b502c71f3f37e385a2ebec7a2510e3989bdcb1a3d891c9d99129ebd",
  "lines": 1121,
  "bytes": 41834,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "shader": 1,
    "rig": 6,
    "face": 159
  },
  "url": "https://github.com/emoto-yasushi/NVIDIA_ARSDK_iFacialMocap/blob/ccbe82cb141cd427753f1ede7d0981135fb65cba/OSS/samples/FaceTrack/FaceTrack.cpp",
  "function_signature_candidates": [
    {
      "line": 85,
      "text": "static void Usage() {"
    },
    {
      "line": 108,
      "text": "static bool GetFlagArgVal(const char *flag, const char *arg, const char **val) {"
    },
    {
      "line": 131,
      "text": "static bool GetFlagArgVal(const char *flag, const char *arg, std::string *val) {"
    },
    {
      "line": 138,
      "text": "static bool GetFlagArgVal(const char *flag, const char *arg, bool *val) {"
    },
    {
      "line": 148,
      "text": "bool GetFlagArgVal(const char *flag, const char *arg, long *val) {"
    },
    {
      "line": 157,
      "text": "static bool GetFlagArgVal(const char *flag, const char *arg, unsigned *val) {"
    },
    {
      "line": 170,
      "text": "static int StringToFourcc(const std::string &str) {"
    },
    {
      "line": 184,
      "text": "static int ParseMyArgs(int argc, char **argv) {"
    },
    {
      "line": 240,
      "text": "void start()  { t0 = std::chrono::high_resolution_clock::now();      }  /**< Start  the timer. */"
    },
    {
      "line": 241,
      "text": "void pause()  { dt = std::chrono::high_resolution_clock::now() - t0; }  /**< Pause  the timer. */"
    },
    {
      "line": 242,
      "text": "void resume() { t0 = std::chrono::high_resolution_clock::now() - dt; }  /**< Resume the timer. */"
    },
    {
      "line": 243,
      "text": "void stop()   { pause();                                             }  /**< Stop   the timer. */"
    },
    {
      "line": 309,
      "text": "void stop();"
    },
    {
      "line": 318,
      "text": "void showFaceFitErrorMessage();"
    },
    {
      "line": 319,
      "text": "void drawFPS(cv::Mat &img);"
    },
    {
      "line": 320,
      "text": "void DrawBBoxes(const cv::Mat &src, NvAR_Rect *output_bbox);"
    },
    {
      "line": 321,
      "text": "void DrawLandmarkPoints(const cv::Mat &src, NvAR_Point2f *facial_landmarks, int numLandmarks);"
    },
    {
      "line": 322,
      "text": "void DrawFaceMesh(const cv::Mat &src, NvAR_FaceMesh *face_mesh);"
    },
    {
      "line": 323,
      "text": "void drawKalmanStatus(cv::Mat &img);"
    },
    {
      "line": 324,
      "text": "void drawVideoCaptureStatus(cv::Mat &img);"
    },
    {
      "line": 326,
      "text": "void processKey(int key);"
    },
    {
      "line": 327,
      "text": "void writeVideoAndEstResults(const cv::Mat &frame, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks = NULL);"
    },
    {
      "line": 328,
      "text": "void writeFrameAndEstResults(const cv::Mat &frame, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks = NULL);"
    },
    {
      "line": 329,
      "text": "void writeEstResults(std::ofstream &outputFile, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks = NULL);"
    },
    {
      "line": 330,
      "text": "void getFPS();"
    },
    {
      "line": 919,
      "text": "int chooseGPU() {"
    },
    {
      "line": 1070,
      "text": "int main(int argc, char **argv) {"
    }
  ]
}

```text

/*###############################################################################
#
# Copyright 2020 NVIDIA Corporation
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of
# this software and associated documentation files (the "Software"), to deal in
# the Software without restriction, including without limitation the rights to
# use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
# the Software, and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
# FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
# COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
# IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
# CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#
###############################################################################*/
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <algorithm>
#include <chrono>
#include <cmath>
#include <ctime>
#include <iomanip>
#include <iostream>
#include <fstream>

#include "FaceEngine.h"
#include "RenderingUtils.h"
#include "nvAR.h"
#include "nvAR_defs.h"
#include "opencv2/opencv.hpp"

#if CV_MAJOR_VERSION >= 4
#define CV_CAP_PROP_FRAME_WIDTH cv::CAP_PROP_FRAME_WIDTH
#define CV_CAP_PROP_FRAME_HEIGHT cv::CAP_PROP_FRAME_HEIGHT
#define CV_CAP_PROP_FPS cv::CAP_PROP_FPS
#define CV_CAP_PROP_FRAME_COUNT cv::CAP_PROP_FRAME_COUNT
#endif

#ifndef M_PI
#define M_PI 3.1415926535897932385
#endif /* M_PI */
#ifndef M_2PI
#define M_2PI 6.2831853071795864769
#endif /* M_2PI */
#ifndef M_PI_2
#define M_PI_2 1.5707963267948966192
#endif /* M_PI_2 */
#define F_PI ((float)M_PI)
#define F_PI_2 ((float)M_PI_2)
#define F_2PI ((float)M_2PI)

#ifdef _MSC_VER
#define strcasecmp _stricmp
#endif /* _MSC_VER */

#define BAIL(err, code) \
  do {                  \
    err = code;         \
    goto bail;          \
  } while (0)

/********************************************************************************
 * Command-line arguments
 ********************************************************************************/

bool FLAG_debug = false, FLAG_verbose = false, FLAG_temporal = true, FLAG_captureOutputs = false,
FLAG_offlineMode = false, FLAG_isNumLandmarks126 = false; unsigned int FLAG_landmarkMode = 0;
std::string FLAG_outDir, FLAG_inFile, FLAG_outFile, FLAG_modelPath, FLAG_landmarks, FLAG_proxyWireframe,
    FLAG_captureCodec = "avc1", FLAG_camRes, FLAG_faceModel;
unsigned int FLAG_appMode = 2;

/********************************************************************************
 * Usage
 ********************************************************************************/

static void Usage() {
  printf(
      "FaceTrack [<args> ...]\n"
      "where <args> is\n"
      " --verbose[=(true|false)]          report interesting info\n"
      " --debug[=(true|false)]            report debugging info\n"
      " --temporal[=(true|false)]         temporally optimize face rect and landmarks\n"
      " --capture_outputs[=(true|false)]  enables video/image capture and writing face detection/landmark outputs\n"
      " --offline_mode[=(true|false)]     disables webcam, reads video from file and writes output video results\n"
      " --cam_res=[WWWx]HHH               specify resolution as height or width x height\n"
      " --codec=<fourcc>                  FOURCC code for the desired codec (default H264)\n"
      " --in=<file>                       specify the  input file\n"
      " --out=<file>                      specify the output file\n"
      " --model_path=<path>               specify the directory containing the TRT models\n"
      " --landmarks_126[=(true|false)]    set the number of facial landmark points to 126, otherwise default to 68\n"
      " --face_model=<file>               specify the  name of the face model\n"
      " --wireframe_mesh=<path>           specify the path to a proxy wireframe mesh\n"
      " --app_mode[=(0|1|2)]              App mode. 0: Face detection, 1: Landmark detection, 2: Face fitting "
      "(Default).\n"
      " --landmark_mode                   Select Landmark Detection Model. 0: Performance (Default),  1: Quality\n"            
      " --benchmarks[=<pattern>]          run benchmarks\n");
}

static bool GetFlagArgVal(const char *flag, const char *arg, const char **val) {
  if (*arg != '-') {
    return false;
  }
  while (*++arg == '-') {
    continue;
  }
  const char *s = strchr(arg, '=');
  if (s == NULL) {
    if (strcmp(flag, arg) != 0) {
      return false;
    }
    *val = NULL;
    return true;
  }
  unsigned n = (unsigned)(s - arg);
  if ((strlen(flag) != n) || (strncmp(flag, arg, n) != 0)) {
    return false;
  }
  *val = s + 1;
  return true;
}

static bool GetFlagArgVal(const char *flag, const char *arg, std::string *val) {
  const char *valStr;
  if (!GetFlagArgVal(flag, arg, &valStr)) return false;
  val->assign(valStr ? valStr : "");
  return true;
}

static bool GetFlagArgVal(const char *flag, const char *arg, bool *val) {
  const char *valStr;
  bool success = GetFlagArgVal(flag, arg, &valStr);
  if (success) {
    *val = (valStr == NULL || strcasecmp(valStr, "true") == 0 || strcasecmp(valStr, "on") == 0 ||
            strcasecmp(valStr, "yes") == 0 || strcasecmp(valStr, "1") == 0);
  }
  return success;
}

bool GetFlagArgVal(const char *flag, const char *arg, long *val) {
  const char *valStr;
  bool success = GetFlagArgVal(flag, arg, &valStr);
  if (success) {
    *val = strtol(valStr, NULL, 10);
  }
  return success;
}

static bool GetFlagArgVal(const char *flag, const char *arg, unsigned *val) {
  long longVal;
  bool success = GetFlagArgVal(flag, arg, &longVal);
  if (success) {
    *val = (unsigned)longVal;
  }
  return success;
}

/********************************************************************************
 * StringToFourcc
 ********************************************************************************/

static int StringToFourcc(const std::string &str) {
  union chint {
    int i;
    char c[4];
  };
  chint x = {0};
  for (int n = (str.size() < 4) ? (int)str.size() : 4; n--;) x.c[n] = str[n];
  return x.i;
}

/********************************************************************************
 * ParseMyArgs
 ********************************************************************************/

static int ParseMyArgs(int argc, char **argv) {
  // query NVAR_MODEL_DIR environment variable first before checking the command line arguments
  const char* modelPath = getenv("NVAR_MODEL_DIR");
  if (modelPath) {
    FLAG_modelPath = modelPath;
  }

  int errs = 0;
  for (--argc, ++argv; argc--; ++argv) {
    bool help;
    const char *arg = *argv;
    if (arg[0] != '-') {
      continue;
    } else if ((arg[1] == '-') &&
               (GetFlagArgVal("verbose", arg, &FLAG_verbose) || GetFlagArgVal("debug", arg, &FLAG_debug) ||
                GetFlagArgVal("in", arg, &FLAG_inFile) || GetFlagArgVal("in_file", arg, &FLAG_inFile) ||
                GetFlagArgVal("out", arg, &FLAG_outFile) || GetFlagArgVal("out_file", arg, &FLAG_outFile) ||
                GetFlagArgVal("offline_mode", arg, &FLAG_offlineMode) ||
                GetFlagArgVal("landmarks_126", arg, &FLAG_isNumLandmarks126) ||
                GetFlagArgVal("capture_outputs", arg, &FLAG_captureOutputs) ||
                GetFlagArgVal("cam_res", arg, &FLAG_camRes) || GetFlagArgVal("codec", arg, &FLAG_captureCodec) ||
                GetFlagArgVal("landmarks", arg, &FLAG_landmarks) || GetFlagArgVal("model_path", arg, &FLAG_modelPath) ||
                GetFlagArgVal("wireframe_mesh", arg, &FLAG_proxyWireframe) ||
                GetFlagArgVal("face_model", arg, &FLAG_faceModel) ||
                GetFlagArgVal("app_mode", arg, &FLAG_appMode) || GetFlagArgVal("temporal", arg, &FLAG_temporal) || 
                GetFlagArgVal("landmark_mode", arg, &FLAG_landmarkMode))) {
      continue;
    } else if (GetFlagArgVal("help", arg, &help)) {
      Usage();
    } else if (arg[1] != '-') {
      for (++arg; *arg; ++arg) {
        if (*arg == 'v') {
          FLAG_verbose = true;
        } else {
          // printf("Unknown flag: \"-%c\"\n", *arg);
        }
      }
      continue;
    } else {
      // printf("Unknown flag: \"%s\"\n", arg);
    }
  }
  return errs;
}

enum {
  myErrNone = 0,
  myErrShader = -1,
  myErrProgram = -2,
  myErrTexture = -3,
};

#if 1
class MyTimer {
 public:
  MyTimer()     { dt = dt.zero();                                      }  /**< Clear the duration to 0. */
  void start()  { t0 = std::chrono::high_resolution_clock::now();      }  /**< Start  the timer. */
  void pause()  { dt = std::chrono::high_resolution_clock::now() - t0; }  /**< Pause  the timer. */
  void resume() { t0 = std::chrono::high_resolution_clock::now() - dt; }  /**< Resume the timer. */
  void stop()   { pause();                                             }  /**< Stop   the timer. */
  double elapsedTimeFloat() const {
    return std::chrono::duration<double>(dt).count();
  } /**< Report the elapsed time as a float. */
 private:
  std::chrono::high_resolution_clock::time_point t0;
  std::chrono::high_resolution_clock::duration dt;
};
#endif

std::string getCalendarTime() {
  // Get the current time
  std::chrono::system_clock::time_point currentTimePoint = std::chrono::system_clock::now();
  // Convert to time_t from time_point
  std::time_t currentTime = std::chrono::system_clock::to_time_t(currentTimePoint);
  // Convert to tm to get structure holding a calendar date and time broken down into its components.
  std::tm brokenTime = *std::localtime(&currentTime);
  std::ostringstream calendarTime;
  // calendarTime << std::put_time(
  //     &brokenTime,
  //     "%Y-%m-%d-%H-%M-%S");  // (YYYY-MM-DD-HH-mm-ss)<Year>-<Month>-<Date>-<Hour>-<Mins>-<Seconds>
  char time_string[24];
  if (0 < strftime(time_string, sizeof(time_string), "%Y-%m-%d-%H-%M-%S] ", &brokenTime))
    calendarTime << time_string;  // (YYYY-MM-DD-HH-mm-ss)<Year>-<Month>-<Date>-<Hour>-<Mins>-<Seconds>
  // Get the time since epoch 0(Thu Jan  1 00:00:00 1970) and the remainder after division is
  // our milliseconds
  std::chrono::milliseconds currentMilliseconds =
      std::chrono::duration_cast<std::chrono::milliseconds>(currentTimePoint.time_since_epoch()) % 1000;
  // Append the milliseconds to the stream
  calendarTime << "-" << std::setfill('0') << std::setw(3) << currentMilliseconds.count();  // milliseconds
  return calendarTime.str();
}

class DoApp {
 public:
  enum Err {
    errNone           = FaceEngine::Err::errNone,
    errGeneral        = FaceEngine::Err::errGeneral,
    errRun            = FaceEngine::Err::errRun,
    errInitialization = FaceEngine::Err::errInitialization,
    errRead           = FaceEngine::Err::errRead,
    errEffect         = FaceEngine::Err::errEffect,
    errParameter      = FaceEngine::Err::errParameter,
    errUnimplemented,
    errMissing,
    errVideo,
    errImageSize,
    errNotFound,
    errFaceModelInit,
    errGLFWInit,
    errGLInit,
    errRendererInit,
    errGLResource,
    errGLGeneric,
    errFaceFit,
    errNoFace,
    errSDK,
    errCuda,
    errCancel,
    errCamera
  };
  Err doAppErr(FaceEngine::Err status) { return (Err)status; }
  FaceEngine face_ar_engine;
  DoApp();
  ~DoApp();

  void stop();
  Err initFaceEngine(const char *modelPath = nullptr, bool isLandmarks126 = false, int mode = 0);
  Err initCamera(const char *camRes = nullptr);
  Err initOfflineMode(const char *inputFilename = nullptr, const char *outputFilename = nullptr);
  Err acquireFrame();
  Err acquireFaceBox();
  Err acquireFaceBoxAndLandmarks();
  Err fitFaceModel();
  Err run();
  void showFaceFitErrorMessage();
  void drawFPS(cv::Mat &img);
  void DrawBBoxes(const cv::Mat &src, NvAR_Rect *output_bbox);
  void DrawLandmarkPoints(const cv::Mat &src, NvAR_Point2f *facial_landmarks, int numLandmarks);
  void DrawFaceMesh(const cv::Mat &src, NvAR_FaceMesh *face_mesh);
  void drawKalmanStatus(cv::Mat &img);
  void drawVideoCaptureStatus(cv::Mat &img);
  Err setProxyWireframe(const char *path);
  void processKey(int key);
  void writeVideoAndEstResults(const cv::Mat &frame, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks = NULL);
  void writeFrameAndEstResults(const cv::Mat &frame, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks = NULL);
  void writeEstResults(std::ofstream &outputFile, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks = NULL);
  void getFPS();
  static const char *errorStringFromCode(Err code);

  cv::VideoCapture cap{};
  cv::Mat frame, outputFrame;
  int inputWidth, inputHeight;
  cv::VideoWriter faceDetectOutputVideo{}, landMarkOutputVideo{}, faceFittingOutputVideo{};
  int frameIndex;
  static const char windowTitle[];
  double frameTime;
  const int batchSize = 1;
  // std::chrono::high_resolution_clock::time_point frameTimer;
  MyTimer frameTimer;
  cv::VideoWriter capturedVideo;
  std::ofstream faceEngineVideoOutputFile;
  FILE *exprFile, *shapeFile, *poseFile;

  FaceEngine::Err nvErr;
  float expr[6];
  bool drawVisualization, showFPS, captureVideo, captureFrame;
  std::vector<std::array<uint16_t, 3>> proxyWireframe;
  float scaleOffsetXY[4];
  std::vector<NvAR_Vector3u16> wfMesh_tvi_data;
};

DoApp *gApp = nullptr;
const char DoApp::windowTitle[] = "FaceTrack App";

void DoApp::processKey(int key) {
  switch (key) {
    case '3':
      face_ar_engine.destroyFeatures();
      face_ar_engine.setAppMode(FaceEngine::mode::faceMeshGeneration);
      nvErr =  face_ar_engine.createFeatures(FLAG_modelPath.c_str());
      if (nvErr == FaceEngine::Err::errNone) {
        face_ar_engine.initFeatureIOParams();
        break;
      } else if (nvErr == FaceEngine::Err::errInitialization) {
        showFaceFitErrorMessage();
        // If there is an error, fallback to mode '2' i.e. landmark detection
      }
    case '2':
      face_ar_engine.destroyFeatures();
      face_ar_engine.setAppMode(FaceEngine::mode::landmarkDetection);
      face_ar_engine.createFeatures(FLAG_modelPath.c_str());
      face_ar_engine.initFeatureIOParams();
      break;
    case '1':
      face_ar_engine.destroyFeatures();
      face_ar_engine.setAppMode(FaceEngine::mode::faceDetection);
      face_ar_engine.createFeatures(FLAG_modelPath.c_str());
      face_ar_engine.initFeatureIOParams();
      break;
    case 'C':
    case 'c':
      captureVideo = !captureVideo;
      break;
    case 'S':
    case 's':
      captureFrame = !captureFrame;
      break;
    case 'W':
    case 'w':
      drawVisualization = !drawVisualization;
      break;
    case 'F':
    case 'f':
      showFPS = !showFPS;
      break;
    default:
      break;
  }
}


DoApp::Err DoApp::initFaceEngine(const char *modelPath, bool isNumLandmarks126, int mode) {
  if (!cap.isOpened()) return errVideo;

  int numLandmarkPoints = isNumLandmarks126 ? 126 : 68;
  face_ar_engine.setNumLandmarks(numLandmarkPoints);

  nvErr = face_ar_engine.createFeatures(modelPath, batchSize, mode);
  if (nvErr != FaceEngine::Err::errNone) {
    if (nvErr == FaceEngine::Err::errInitialization && face_ar_engine.appMode == FaceEngine::mode::faceMeshGeneration) {
      showFaceFitErrorMessage();
      printf("WARNING: face fitting has failed, trying to initialize Landmark Detection\n");
      face_ar_engine.destroyFeatures();
      face_ar_engine.setAppMode(FaceEngine::mode::landmarkDetection);
      nvErr = face_ar_engine.createFeatures(modelPath, batchSize, mode);
    }
  }

#ifdef DEBUG
  detector->setOutputLocation(outputDir);
#endif  // DEBUG

#define VISUALIZE
#ifdef VISUALIZE
  if (!FLAG_offlineMode) cv::namedWindow(windowTitle, 1);
#endif  // VISUALIZE

  frameIndex = 0;

  return doAppErr(nvErr);
}

void DoApp::stop() {
  face_ar_engine.destroyFeatures();

  if (FLAG_offlineMode) {
    faceDetectOutputVideo.release();
    landMarkOutputVideo.release();
    faceFittingOutputVideo.release();
  }
  cap.release();
#ifdef VISUALIZE
  cv::destroyAllWindows();
#endif  // VISUALIZE
}

void DoApp::showFaceFitErrorMessage() {
  cv::Mat errBox = cv::Mat::zeros(120, 640, CV_8UC3);
  cv::putText(errBox, cv::String("Warning: Face Fitting needs face_model2.nvf in the path --model_path"), cv::Point(20, 20),
              cv::FONT_HERSHEY_SIMPLEX, 0.5, cv::Scalar(255, 255, 255), 1);
  cv::putText(errBox, cv::String("or in NVAR_MODEL_DIR environment variable."), cv::Point(20, 40), cv::FONT_HERSHEY_SIMPLEX,
              0.5, cv::Scalar(255, 255, 255), 1);
  cv::putText(errBox, cv::String("See samples\\FaceTrack\\Readme.txt"), cv::Point(20, 60), cv::FONT_HERSHEY_SIMPLEX,
              0.5, cv::Scalar(255, 255, 255), 1);
  cv::putText(errBox, cv::String("Press any key to continue with other features"), cv::Point(20, 80), cv::FONT_HERSHEY_SIMPLEX, 0.5,
              cv::Scalar(255, 255, 255), 1);
  cv::imshow(windowTitle, errBox);
  cv::waitKey(0);
}

void DoApp::DrawBBoxes(const cv::Mat &src, NvAR_Rect *output_bbox) {
  cv::Mat frm;
  if (FLAG_offlineMode)
    frm = src.clone();
  else
    frm = src;

  if (output_bbox)
    cv::rectangle(frm, cv::Point(lround(output_bbox->x), lround(output_bbox->y)),
                  cv::Point(lround(output_bbox->x + output_bbox->width), lround(output_bbox->y + output_bbox->height)),
                  cv::Scalar(255, 0, 0), 2);
  if (FLAG_offlineMode) faceDetectOutputVideo.write(frm);
}


void DoApp::writeVideoAndEstResults(const cv::Mat &frm, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks) {
  if (captureVideo) {
    if (!capturedVideo.isOpened()) {
      const std::string currentCalendarTime = getCalendarTime();
      const std::string capturedOutputFileName = currentCalendarTime + ".mp4";
      getFPS();
      if (frameTime) {
        float fps = (float)(1.0 / frameTime);
        capturedVideo.open(capturedOutputFileName, StringToFourcc(FLAG_captureCodec), fps,
                           cv::Size(frm.cols, frm.rows));
        if (!capturedVideo.isOpened()) {
          std::cout << "Error: Could not open video: \"" << capturedOutputFileName << "\"\n";
          return;
        }
        if (FLAG_verbose) {
          std::cout << "Capturing video started" << std::endl;
        }
      } else {  // If frameTime is 0.f, returns without writing the frame to the Video
        return;
      }
      const std::string outputsFileName = currentCalendarTime + ".txt";
      faceEngineVideoOutputFile.open(outputsFileName, std::ios_base::out);
      if (!faceEngineVideoOutputFile.is_open()) {
        std::cout << "Error: Could not open file: \"" << outputsFileName << "\"\n";
        return;
      }
      std::string landmarkDetectionMode = (landmarks == NULL) ? "Off" : "On";
      faceEngineVideoOutputFile << "// FaceDetectOn, LandmarkDetect" << landmarkDetectionMode << "\n ";
      faceEngineVideoOutputFile
          << "// kNumFaces, (bbox_x, bbox_y, bbox_w, bbox_h){ kNumFaces}, kNumLMs, [lm_x, lm_y]{kNumLMs}\n";
    }
    // Write each frame to the Video
    capturedVideo << frm;
    writeEstResults(faceEngineVideoOutputFile, output_bboxes, landmarks);
  } else {
    if (capturedVideo.isOpened()) {
      if (FLAG_verbose) {
        std::cout << "Capturing video ended" << std::endl;
      }
      capturedVideo.release();
      if (faceEngineVideoOutputFile.is_open()) faceEngineVideoOutputFile.close();
    }
  }
}

void DoApp::writeEstResults(std::ofstream &outputFile, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks) {
  /**
   * Output File Format :
   * FaceDetectOn, LandmarkDetectOn
   * kNumFaces, (bbox_x, bbox_y, bbox_w, bbox_h){ kNumFaces}, kNumLMs, [lm_x, lm_y]{kNumLMs}
   */

  int faceDetectOn = (face_ar_engine.appMode == FaceEngine::mode::faceDetection ||
                      face_ar_engine.appMode == FaceEngine::mode::landmarkDetection ||
                      face_ar_engine.appMode == FaceEngine::mode::faceMeshGeneration)
                         ? 1
                         : 0;
  int landmarkDetectOn = (face_ar_engine.appMode == FaceEngine::mode::landmarkDetection ||
                          face_ar_engine.appMode == FaceEngine::mode::faceMeshGeneration)
                             ? 1
                             : 0;
  outputFile << faceDetectOn << "," << landmarkDetectOn << "\n";

  if (faceDetectOn && output_bboxes.num_boxes) {
    // Append number of faces detected in the current frame
    outputFile << unsigned(output_bboxes.num_boxes) << ",";
    // write outputbboxes to outputFile
    for (size_t i = 0; i < output_bboxes.num_boxes; i++) {
      int x1 = (int)output_bboxes.boxes[i].x, y1 = (int)output_bboxes.boxes[i].y,
          width = (int)output_bboxes.boxes[i].width, height = (int)output_bboxes.boxes[i].height;
      outputFile << x1 << "," << y1 << "," << width << "," << height << ",";
    }
  } else {
    outputFile << "0,";
  }
  if (landmarkDetectOn && output_bboxes.num_boxes) {
    int numLandmarks = face_ar_engine.getNumLandmarks();
    // Append number of landmarks
    outputFile << numLandmarks << ",";
    // Append 2 * number of landmarks values
    NvAR_Point2f *pt, *endPt;
    for (endPt = (pt = (NvAR_Point2f *)landmarks) + numLandmarks; pt < endPt; ++pt)
      outputFile << pt->x << "," << pt->y << ",";
  } else {
    outputFile << "0,";
  }

  outputFile << "\n";
}

void DoApp::writeFrameAndEstResults(const cv::Mat &frm, NvAR_BBoxes output_bboxes, NvAR_Point2f *landmarks) {
  if (captureFrame) {
    const std::string currentCalendarTime = getCalendarTime();
    const std::string capturedFrame = currentCalendarTime + ".png";
    cv::imwrite(capturedFrame, frm);
    if (FLAG_verbose) {
      std::cout << "Captured the frame" << std::endl;
    }
    // Write Face Engine Outputs
    const std::string outputFilename = currentCalendarTime + ".txt";
    std::ofstream outputFile;
    outputFile.open(outputFilename, std::ios_base::out);
    if (!outputFile.is_open()) {
      std::cout << "Error: Could not open file: \"" << outputFilename << "\"\n";
      return;
    }
    std::string landmarkDetectionMode = (landmarks == NULL) ? "Off" : "On";
    outputFile << "// FaceDetectOn, LandmarkDetect" << landmarkDetectionMode << "\n";
    outputFile << "// kNumFaces, (bbox_x, bbox_y, bbox_w, bbox_h){ kNumFaces}, kNumLMs, [lm_x, lm_y]{kNumLMs}\n";
    writeEstResults(outputFile, output_bboxes, landmarks);
    if (outputFile.is_open()) outputFile.close();
    captureFrame = false;
  }
}

void DoApp::DrawLandmarkPoints(const cv::Mat &src, NvAR_Point2f *facial_landmarks, int numLandmarks) {
  cv::Mat frm;
  if (FLAG_offlineMode)
    frm = src.clone();
  else
    frm = src;
  NvAR_Point2f *pt, *endPt;
  for (endPt = (pt = (NvAR_Point2f *)facial_landmarks) + numLandmarks; pt < endPt; ++pt)
    cv::circle(frm, cv::Point(lround(pt->x), lround(pt->y)), 1, cv::Scalar(0, 0, 255), -1);
  NvAR_Quaternion *pose = face_ar_engine.getPose();
  if (pose)
    face_ar_engine.DrawPose(frm, pose);
  if (FLAG_offlineMode) landMarkOutputVideo.write(frm);
}

void DoApp::DrawFaceMesh(const cv::Mat &src, NvAR_FaceMesh *face_mesh) {
  cv::Mat render_frame;
  if (FLAG_offlineMode)
    render_frame = src.clone();
  else
    render_frame = src;

  const cv::Scalar wfColor(0, 160, 0, 255);
  //Low resolution mesh can be used for visualization only
  if (proxyWireframe.size() > 0) {
    NvAR_FaceMesh wfMesh{nullptr, 0, nullptr, 0};
    wfMesh.num_vertices = face_mesh->num_vertices;
    wfMesh.vertices = face_mesh->vertices;
    wfMesh.num_triangles = proxyWireframe.size();
    wfMesh_tvi_data.resize(wfMesh.num_triangles);
    wfMesh.tvi = wfMesh_tvi_data.data();

    for (int i = 0; i < proxyWireframe.size(); i++) {
      auto proxyWireframe_idx = proxyWireframe[i];
      wfMesh.tvi[i].vec[0] = proxyWireframe_idx[0];
      wfMesh.tvi[i].vec[1] = proxyWireframe_idx[1];
      wfMesh.tvi[i].vec[2] = proxyWireframe_idx[2];
    }
    draw_wireframe(render_frame, wfMesh, *face_ar_engine.getRenderingParams(), wfColor);
  } else {
    draw_wireframe(render_frame, *face_mesh, * face_ar_engine.getRenderingParams(), wfColor);
  }

  if (FLAG_offlineMode) faceFittingOutputVideo.write(render_frame);
}

DoApp::Err DoApp::acquireFrame() {
  Err err = errNone;

  // If the machine goes to sleep with the app running and then wakes up, the camera object is not destroyed but the
  // frames we try to read are empty. So we try to re-initialize the camera with the same resolution settings. If the
  // resolution has changed, you will need to destroy and create the features again with the new camera resolution (not
  // done here) as well as reallocate memory accordingly with FaceEngine::initFeatureIOParams()
  cap >> frame;  // get a new frame from camera into the class variable frame.
  if (frame.empty()) {
    // if in Offline mode, this means end of video,so we return
    if (FLAG_offlineMode) return errVideo;
    // try Init one more time if reading frames from camera
    err = initCamera(FLAG_camRes.c_str());
    if (err != errNone)
      return err;
    cap >> frame;
    if (frame.empty()) return errVideo;
  }

  return err;
}

DoApp::Err DoApp::acquireFaceBox() {
  Err err = errNone;
  NvAR_Rect output_bbox;

  // get landmarks in  original image resolution coordinate space
  unsigned n = face_ar_engine.acquireFaceBox(frame, output_bbox, 0);

  if (n && FLAG_verbose) {
    printf("FaceBox: [\n");
    printf("%7.1f%7.1f%7.1f%7.1f\n", output_bbox.x, output_bbox.y, output_bbox.x + output_bbox.width,
           output_bbox.y + output_bbox.height);
    printf("]\n");
  }
  if (FLAG_captureOutputs) {
    writeFrameAndEstResults(frame, face_ar_engine.output_bboxes);
    writeVideoAndEstResults(frame, face_ar_engine.output_bboxes);
  }

#ifdef VISUALIZE
  if (n > 0) {                              // At least one face was found
    if (drawVisualization) {
      DrawBBoxes(frame, &output_bbox);      // This will write a frame if in offlineMode 
    }
  }
  else {                                    // No faces found
    if (FLAG_offlineMode) {
      faceDetectOutputVideo.write(frame);   // This will write a frame if in offlineMode 
    }
    err = errNoFace;
  }
#else  // !VISUALIZE
  if (0 == n) err = errNoFace;
#endif  // VISUALIZE
  frameIndex++;

  return err;
}

DoApp::Err DoApp::acquireFaceBoxAndLandmarks() {
  Err err = errNone;
  int numLandmarks = face_ar_engine.getNumLandmarks();
  NvAR_Rect output_bbox;
  std::vector<NvAR_Point2f> facial_landmarks(numLandmarks);

  // get landmarks in  original image resolution coordinate space
  unsigned n = face_ar_engine.acquireFaceBoxAndLandmarks(frame, facial_landmarks.data(), output_bbox, 0);

  if (n && FLAG_verbose && face_ar_engine.appMode != FaceEngine::mode::faceDetection) {
    printf("Landmarks: [\n");
    for (const auto &pt : facial_landmarks) {
      printf("%7.1f%7.1f\n", pt.x, pt.y);
    }
    printf("]\n");
  }
  if (FLAG_captureOutputs) {
    writeFrameAndEstResults(frame, face_ar_engine.output_bboxes, facial_landmarks.data());
    writeVideoAndEstResults(frame, face_ar_engine.output_bboxes, facial_landmarks.data());
  }

#ifdef VISUALIZE
  if (n > 0) {                              // At least one face found
    if (drawVisualization) {
      DrawLandmarkPoints(frame, facial_landmarks.data(), numLandmarks); // Writes frame in offline mode
      if (FLAG_offlineMode) {
        DrawBBoxes(frame, &output_bbox);    // Writes frame in offline mode
      }
    }
  }
  else {                                    // No faces found
    if (FLAG_offlineMode) {
      faceDetectOutputVideo.write(frame);   // These two wrtite frames if a face was not detected
      landMarkOutputVideo.write(frame);
    }
  }
#else // !VISUALIZE
  if (0 == n) err = errNoFace;
#endif // VISUALIZE
  frameIndex++;

  return err;
}

DoApp::Err DoApp::initCamera(const char *camRes) {
  if (cap.open(0)) {
    if (camRes) {
      int n;
      n = sscanf(camRes, "%d%*[xX]%d", &inputWidth, &inputHeight);
      switch (n) {
        case 2:
          break;  // We have read both width and height
        case 1:
          inputHeight = inputWidth;
          inputWidth = (int)(inputHeight * (4. / 3.) + .5);
          break;
        default:
          inputHeight = 0;
          inputWidth = 0;
          break;
      }
      if (inputWidth) cap.set(CV_CAP_PROP_FRAME_WIDTH, inputWidth);
      if (inputHeight) cap.set(CV_CAP_PROP_FRAME_HEIGHT, inputHeight);

      inputWidth = (int)cap.get(CV_CAP_PROP_FRAME_WIDTH);
      inputHeight = (int)cap.get(CV_CAP_PROP_FRAME_HEIGHT);
      face_ar_engine.setInputImageWidth(inputWidth);
      face_ar_engine.setInputImageHeight(inputHeight);
    }
  } else
    return errCamera;
  return errNone;
}

DoApp::Err DoApp::initOfflineMode(const char *inputFilename, const char *outputFilename) {
  if (cap.open(inputFilename)) {
    inputWidth = (int)cap.get(CV_CAP_PROP_FRAME_WIDTH);
    inputHeight = (int)cap.get(CV_CAP_PROP_FRAME_HEIGHT);
    face_ar_engine.setInputImageWidth(inputWidth);
    face_ar_engine.setInputImageHeight(inputHeight);
  } else {
    printf("ERROR: Unable to open the input video file \"%s\" \n", inputFilename);
    return Err::errVideo;
  }

  std::string fdOutputVideoName, fldOutputVideoName, ffOutputVideoName,
              exprOutputFileName, shapeOutputFileName, poseOutputFileName, outputFilePrefix;
  if (outputFilename && strlen(outputFilename) != 0) {
    outputFilePrefix = outputFilename;
  } else {
    size_t lastindex = std::string(inputFilename).find_last_of(".");
    outputFilePrefix = std::string(inputFilename).substr(0, lastindex);
  }
  fdOutputVideoName   = outputFilePrefix + "_bbox.mp4";
  fldOutputVideoName  = outputFilePrefix + "_landmarks.mp4";
  ffOutputVideoName   = outputFilePrefix + "_faceModel.mp4";
  exprOutputFileName  = outputFilePrefix + "_expr.json";
  shapeOutputFileName = outputFilePrefix + "_shape.json";
  poseOutputFileName  = outputFilePrefix + "_pose.json";

  const int codec = StringToFourcc(FLAG_captureCodec);
  const double fps = cap.get(CV_CAP_PROP_FPS);
  const cv::Size frameSize(inputWidth, inputHeight);
  if (FLAG_verbose) {
    const unsigned long long frameCount = (unsigned long long)cap.get(CV_CAP_PROP_FRAME_COUNT);
    printf("codec='%.4s' fps=%.8g frameCount=%llu\n", (const char*)&codec, fps, frameCount);
  }
  if (!faceDetectOutputVideo.open(fdOutputVideoName, codec, fps, frameSize)) {
    printf("ERROR: Unable to open the output video file \"%s\" \n", fdOutputVideoName.c_str());
    return Err::errGeneral;
  }
  if (!landMarkOutputVideo.open(fldOutputVideoName, codec, fps, frameSize)) {
    printf("ERROR: Unable to open the output video file \"%s\" \n", fldOutputVideoName.c_str());
    return Err::errGeneral;
  }
  if (!faceFittingOutputVideo.open(ffOutputVideoName, codec, fps, frameSize)) {
    printf("ERROR: Unable to open the output video file \"%s\" \n", ffOutputVideoName.c_str());
    return Err::errGeneral;
  }
  if (FLAG_debug) {
    if (nullptr == (exprFile = fopen(exprOutputFileName.c_str(), "w"))) {
      printf("ERROR: Unable to open the output  file \"%s\" \n", exprOutputFileName.c_str());
      return Err::errGeneral;
    }
    fprintf(exprFile, "{\n  \"expression_series\":[");
    if (nullptr == (shapeFile = fopen(shapeOutputFileName.c_str(), "w"))) {
      printf("ERROR: Unable to open the output  file \"%s\" \n", shapeOutputFileName.c_str());
      return Err::errGeneral;
    }
    fprintf(shapeFile, "{\n  \"shape_series\":[");
    if (nullptr == (poseFile = fopen(poseOutputFileName.c_str(), "w"))) {
      printf("ERROR: Unable to open the output  file \"%s\" \n", poseOutputFileName.c_str());
      return Err::errGeneral;
    }
    fprintf(poseFile, "{\n  \"pose_series\":[");
  }
  return Err::errNone;
}

DoApp::Err DoApp::fitFaceModel() {
  DoApp::Err doErr = errNone;
  nvErr = face_ar_engine.fitFaceModel(frame);
  if (FLAG_captureOutputs) {
    writeFrameAndEstResults(frame, face_ar_engine.output_bboxes, face_ar_engine.getLandmarks());
    writeVideoAndEstResults(frame, face_ar_engine.output_bboxes, face_ar_engine.getLandmarks());
  }

  frameTimer.pause();
#ifdef VISUALIZE
  if (FaceEngine::Err::errNone == nvErr) {                // If a face and its landmarks were detected
    if (drawVisualization) {
      DrawFaceMesh(frame, face_ar_engine.getFaceMesh());  // This writes a frame in offline mode ...
      if (FLAG_offlineMode) {                             // ... as do the following two statements
        DrawLandmarkPoints(frame, face_ar_engine.getLandmarks(), face_ar_engine.getNumLandmarks());
        DrawBBoxes(frame, face_ar_engine.getLargestBox());
      }
    }
  } else {
    if (FLAG_offlineMode) {
      faceDetectOutputVideo.write(frame);   // These three will draw unannotated frames if a face was not detected
      landMarkOutputVideo.write(frame);
      faceFittingOutputVideo.write(frame);
    }
    doErr = errFaceFit;
  }
#else // !VISUALIZE
  if (FaceEngine::Err::errNone != nvErr)
    doErr = errFaceFit;
#endif // VISUALIZE
  if (FLAG_offlineMode && FLAG_debug) {
    unsigned n;
    const float *f;
    if (frameIndex)
      fprintf(exprFile, ",");
    fprintf(exprFile, "\n    [");
    for (n = face_ar_engine.getNumExpressionCoefficients(), f = face_ar_engine.getExpressionCoefficients(); n--; ++f)
      fprintf(exprFile, "%8.6f%c", *f, (n ? ',' : ']'));
    if (frameIndex)
      fprintf(shapeFile, ",");
    fprintf(shapeFile, "\n    [");
    for (n = face_ar_engine.getNumShapeEigenvalues(), f = face_ar_engine.getShapeEigenvalues(); n--; ++f)
      fprintf(shapeFile, "%+9.6f%c", *f, (n ? ',' : ']'));
    if (frameIndex)
      fprintf(poseFile, ",");
    f = (const float*)face_ar_engine.getPose();
    fprintf(poseFile, "\n    [%+9.6f,%+9.6f,%+9.6f,%+9.6f]", f[0], f[1], f[2], f[3]);
  }
  frameTimer.resume();
  frameIndex++;

  return doErr;
}

DoApp::DoApp() {
  // Make sure things are initialized properly
  gApp = this;
  drawVisualization = true;
  showFPS = false;
  captureVideo = false;
  captureFrame = false;
  frameTime = 0;
  frameIndex = 0;
  nvErr = FaceEngine::errNone;
  scaleOffsetXY[0] = scaleOffsetXY[2] = 1.f;
  scaleOffsetXY[1] = scaleOffsetXY[3] = 0.f;
  exprFile = nullptr;
  shapeFile = nullptr;
  poseFile = nullptr;
}

DoApp::~DoApp() {
  static const char termJsFile[] = { "\n  ]\n}\n" };
  if (exprFile)  { fprintf(exprFile,  termJsFile); fclose(exprFile);  }
  if (shapeFile) { fprintf(shapeFile, termJsFile); fclose(shapeFile); }
  if (poseFile)  { fprintf(poseFile,  termJsFile); fclose(poseFile);  }
}

char *g_nvARSDKPath = NULL;

int chooseGPU() {
  // If the system has multiple supported GPUs then the application
  // should use CUDA driver APIs or CUDA runtime APIs to enumerate
  // the GPUs and select one based on the application's requirements

  //Cuda device 0
  return 0;

}

void DoApp::getFPS() {
  const float timeConstant = 16.f;
  frameTimer.stop();
  float t = (float)frameTimer.elapsedTimeFloat();
  if (t < 100.f) {
    if (frameTime)
      frameTime += (t - frameTime) * (1.f / timeConstant);  // 1 pole IIR filter
    else
      frameTime = t;
  } else {            // Ludicrous time interval; reset
    frameTime = 0.f;  // WAKE UP
  }
  frameTimer.start();
}

void DoApp::drawFPS(cv::Mat &img) {
  getFPS();
  if (frameTime && showFPS) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%.1f", 1. / frameTime);
    cv::putText(img, buf, cv::Point(img.cols - 80, img.rows - 10), cv::FONT_HERSHEY_SIMPLEX, 1,
                cv::Scalar(255, 255, 255), 1);
  }
}

void DoApp::drawKalmanStatus(cv::Mat &img) {
  char buf[32];
  snprintf(buf, sizeof(buf), "Kalman %s", (face_ar_engine.bStabilizeFace ? "on" : "off"));
  cv::putText(img, buf, cv::Point(10, img.rows - 40), cv::FONT_HERSHEY_SIMPLEX, 1, cv::Scalar(255, 255, 255), 1);
}

void DoApp::drawVideoCaptureStatus(cv::Mat &img) {
  char buf[32];
  snprintf(buf, sizeof(buf), "Video Capturing %s", (captureVideo ? "on" : "off"));
  cv::putText(img, buf, cv::Point(10, img.rows - 70), cv::FONT_HERSHEY_SIMPLEX, 1, cv::Scalar(255, 255, 255), 1);
}

DoApp::Err DoApp::run() {
  DoApp::Err doErr = errNone;

  FaceEngine::Err err = face_ar_engine.initFeatureIOParams();
  if (err != FaceEngine::Err::errNone ) {
    return doAppErr(err);
  }

  while (1) {
    doErr = acquireFrame();
    if (frame.empty() && FLAG_offlineMode) {
      // We have reached the end of the video
      // so return without any error.
      return DoApp::errNone;
    } else if (doErr != DoApp::errNone) {
      return doErr;
    }
    if (face_ar_engine.appMode == FaceEngine::mode::faceDetection) {
      doErr = acquireFaceBox();
    } else if (face_ar_engine.appMode == FaceEngine::mode::landmarkDetection) {
      doErr = acquireFaceBoxAndLandmarks();
    } else if (face_ar_engine.appMode == FaceEngine::mode::faceMeshGeneration) {
      doErr = fitFaceModel();
    }
    if (DoApp::errCancel == doErr || DoApp::errVideo == doErr) return doErr;
    if (!frame.empty() && !FLAG_offlineMode) {
      if (drawVisualization) {
        drawFPS(frame);
        drawKalmanStatus(frame);
        if (FLAG_captureOutputs && captureVideo) drawVideoCaptureStatus(frame);
      }
      cv::imshow(windowTitle, frame);
    }
    if (!FLAG_offlineMode) {
      int n = cv::waitKey(1);
      if (n >= 0) {
        static const int ESC_KEY = 27;
        if (n == ESC_KEY) break;
        processKey(n);
      }
    }
  }
  return doErr;
}

DoApp::Err DoApp::setProxyWireframe(const char *path) {
  std::ifstream file(path);
  if (!file.is_open()) return errFaceModelInit;
  std::string lineBuf;
  while (getline(file, lineBuf)) {
    std::array<uint16_t, 3> tri;
    if (3 == sscanf(lineBuf.c_str(), "f %hd %hd %hd", &tri[0], &tri[1], &tri[2])) {
      --tri[0]; /* Convert from 1-based to 0-based indices */
      --tri[1];
      --tri[2];
      proxyWireframe.push_back(tri);
    }
  }
  file.close();
  return errNone;
}

const char *DoApp::errorStringFromCode(DoApp::Err code) {
  struct LUTEntry {
    Err code;
    const char *str;
  };
  static const LUTEntry lut[] = {
      {errNone, "no error"},
      {errGeneral, "an error has occured"},
      {errRun, "an error has occured while the feature is running"},
      {errInitialization, "Initializing Face Engine failed"},
      {errRead, "an error has occured while reading a file"},
      {errEffect, "an error has occured while creating a feature"},
      {errParameter, "an error has occured while setting a parameter for a feature"},
      {errUnimplemented, "the feature is unimplemented"},
      {errMissing, "missing input parameter"},
      {errVideo, "no video source has been found"},
      {errImageSize, "the image size cannot be accommodated"},
      {errNotFound, "the item cannot be found"},
      {errFaceModelInit, "face model initialization failed"},
      {errGLFWInit, "GLFW initialization failed"},
      {errGLInit, "OpenGL initialization failed"},
      {errRendererInit, "renderer initialization failed"},
      {errGLResource, "an OpenGL resource could not be found"},
      {errGLGeneric, "an otherwise unspecified OpenGL error has occurred"},
      {errFaceFit, "an error has occurred while face fitting"},
      {errNoFace, "no face has been found"},
      {errSDK, "an SDK error has occurred"},
      {errCuda, "a CUDA error has occurred"},
      {errCancel, "the user cancelled"},
      {errCamera, "unable to connect to the camera"},
  };
  for (const LUTEntry *p = lut; p < &lut[sizeof(lut) / sizeof(lut[0])]; ++p)
    if (p->code == code) return p->str;
  static char msg[18];
  snprintf(msg, sizeof(msg), "error #%d", code);
  return msg;
}

/********************************************************************************
 * main
 ********************************************************************************/

int main(int argc, char **argv) {
  // Parse the arguments
  if (0 != ParseMyArgs(argc, argv)) return -100;

  DoApp app;
  DoApp::Err doErr = DoApp::Err::errNone;

  app.face_ar_engine.setAppMode(FaceEngine::mode(FLAG_appMode));

  if (FLAG_verbose) printf("Enable temporal optimizations in detecting face and landmarks = %d\n", FLAG_temporal);
  app.face_ar_engine.setFaceStabilization(FLAG_temporal);

  doErr = DoApp::errFaceModelInit;
  if (FLAG_modelPath.empty()) {
    printf("WARNING: Model path not specified. Please set --model_path=/path/to/trt/and/face/models, "
      "SDK will attempt to load the models from NVAR_MODEL_DIR environment variable, "
      "please restart your application after the SDK Installation. \n");
  }
  if (!FLAG_faceModel.empty())
    app.face_ar_engine.setFaceModel(FLAG_faceModel.c_str());

  if (FLAG_offlineMode) {
    if (FLAG_inFile.empty()) {
      doErr = DoApp::errMissing;
      printf("ERROR: %s, please specify input file using --in_file or --in \n", app.errorStringFromCode(doErr));
      goto bail;
    }
    doErr = app.initOfflineMode(FLAG_inFile.c_str(), FLAG_outFile.c_str());
  } else {
    doErr = app.initCamera(FLAG_camRes.c_str());
  }
  BAIL_IF_ERR(doErr);

  if ((FLAG_landmarkMode < 0) | (FLAG_landmarkMode > 1)){
    doErr = DoApp::errParameter;
    printf("ERROR: %s, Please Select Either Mode 0 or 1! \n", app.errorStringFromCode(doErr));
  }

  doErr = app.initFaceEngine(FLAG_modelPath.c_str(), FLAG_isNumLandmarks126, FLAG_landmarkMode);
  BAIL_IF_ERR(doErr);

  if (!FLAG_proxyWireframe.empty()) app.setProxyWireframe(FLAG_proxyWireframe.c_str());

  doErr = app.run();
  BAIL_IF_ERR(doErr);

bail:
  if(doErr)
    printf("ERROR: %s\n", app.errorStringFromCode(doErr));
  app.stop();
  return (int)doErr;
}


```

## Source file: OSS/samples/FaceTrack/FaceEngine.cpp

{
  "path": "OSS/samples/FaceTrack/FaceEngine.cpp",
  "blob_sha": "876c024cbc33095837ed93ba525e97103df4a3f2",
  "sha256": "057b6182092127eaf600dd7820ffcd4021b5109388610c26eab1bfd18223893b",
  "lines": 717,
  "bytes": 26884,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 6,
    "face": 179,
    "lip": 13
  },
  "url": "https://github.com/emoto-yasushi/NVIDIA_ARSDK_iFacialMocap/blob/ccbe82cb141cd427753f1ede7d0981135fb65cba/OSS/samples/FaceTrack/FaceEngine.cpp",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "bool CheckResult(NvCV_Status nvErr, unsigned line) {"
    },
    {
      "line": 540,
      "text": "static bool IntersectRectWithImage(const cv::Rect& srcRect, const cv::Mat& src, cv::Rect& clipRect) {"
    }
  ]
}

```text

/*###############################################################################
#
# Copyright 2020-2021 NVIDIA Corporation
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of
# this software and associated documentation files (the "Software"), to deal in
# the Software without restriction, including without limitation the rights to
# use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
# the Software, and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
# FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
# COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
# IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
# CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#
###############################################################################*/
#include "FaceEngine.h"
#include "RenderingUtils.h"
#include <iostream>


bool CheckResult(NvCV_Status nvErr, unsigned line) {
  if (NVCV_SUCCESS == nvErr) return true;
  std::cout << "ERROR: " << NvCV_GetErrorStringFromCode(nvErr) << ", line " << line << std::endl;
  return false;
}

FaceEngine::Err FaceEngine::fitFaceModel(cv::Mat& frame) {
  FaceEngine::Err err = FaceEngine::Err::errNone;
  NvCV_Status nvErr;
  if (!frame.empty()) {
    NvCVImage fxSrcChunkyCPU;
    (void)NVWrapperForCVMat(&frame, &fxSrcChunkyCPU);
    NvCV_Status cvErr = NvCVImage_Transfer(&fxSrcChunkyCPU, &inputImageBuffer, 1.0f, stream, NULL);

    if (NVCV_SUCCESS != cvErr) {
      return errRun;
    }
  }

  nvErr = NvAR_Run(faceFitHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errRun);

  if (getAverageLandmarksConfidence() < confidenceThreshold) return FaceEngine::Err::errRun;

bail:
  return err;
}

NvAR_FaceMesh* FaceEngine::getFaceMesh() { return face_mesh; }

NvAR_RenderingParams* FaceEngine::getRenderingParams() { return rendering_params; }

float* FaceEngine::getShapeEigenvalues() { return shapeEigenvalues.data(); }
float* FaceEngine::getExpressionCoefficients() { return expressionCoefficients.data(); }
int FaceEngine::getNumShapeEigenvalues() {
  unsigned n = 0;
  (void)NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(ShapeEigenValueCount), &n);
  return n;
}
int FaceEngine::getNumExpressionCoefficients() {
  unsigned n = 0;
  (void)NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(ExpressionCount), &n);
  return n;
}


FaceEngine::Err FaceEngine::createFeatures(const char* modelPath, unsigned int _batchSize, unsigned int mode) {
  FaceEngine::Err err = FaceEngine::Err::errNone;

  NvCV_Status cuErr = NvAR_CudaStreamCreate(&stream);
  if (NVCV_SUCCESS != cuErr) {
    printf("Cannot create a cuda stream: %s\n", NvCV_GetErrorStringFromCode(cuErr));
    return errInitialization;
  }
  if (appMode == faceDetection) {
    err = createFaceDetectionFeature(modelPath, stream);
    if (err != Err::errNone) {
      printf("ERROR: An error has occurred while initializing Face Detection\n");
    }
  } else if (appMode == landmarkDetection) {
    err = createLandmarkDetectionFeature(modelPath, _batchSize, stream, mode);
    if (err != Err::errNone) {
      printf("ERROR: An error has occurred while initializing Landmark Detection\n");
    }
  } else if (appMode == faceMeshGeneration) {
    err = createFaceFittingFeature(modelPath, stream);
    if (err != Err::errNone) {
      printf("ERROR: An error has occurred while initializing Face Fitting\n");
    }
  }

  return err;
}

FaceEngine::Err FaceEngine::createFaceDetectionFeature(const char* modelPath, CUstream str) {
  FaceEngine::Err err = FaceEngine::Err::errNone;
  NvCV_Status nvErr;

  nvErr = NvAR_Create(NvAR_Feature_FaceDetection, &faceDetectHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errEffect);

  if (bUseOTAU && (!modelPath || !modelPath[0])) {
    nvErr = NvAR_SetString(faceDetectHandle, NvAR_Parameter_Config(ModelDir), this->fdOTAModelPath);
    BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  } else {
    nvErr = NvAR_SetString(faceDetectHandle, NvAR_Parameter_Config(ModelDir), modelPath);
    BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  }

  nvErr = NvAR_SetCudaStream(faceDetectHandle, NvAR_Parameter_Config(CUDAStream), str);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(faceDetectHandle, NvAR_Parameter_Config(Temporal), (bStabilizeFace ? -1 : 0));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_Load(faceDetectHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errInitialization);

bail:
  return err;
}

FaceEngine::Err FaceEngine::createLandmarkDetectionFeature(const char* modelPath, unsigned int _batchSize,
                                                           CUstream str, unsigned int mode) {
  FaceEngine::Err err = FaceEngine::Err::errNone;
  NvCV_Status nvErr;

  batchSize = _batchSize;
  nvErr = NvAR_Create(NvAR_Feature_LandmarkDetection, &landmarkDetectHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errEffect);

  if (bUseOTAU && (!modelPath || !modelPath[0])) {
      nvErr = NvAR_SetString(landmarkDetectHandle, NvAR_Parameter_Config(ModelDir), this->ldOTAModelPath);
      BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  }
  else {
    nvErr = NvAR_SetString(landmarkDetectHandle, NvAR_Parameter_Config(ModelDir), modelPath);
    BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  }

  nvErr = NvAR_SetCudaStream(landmarkDetectHandle, NvAR_Parameter_Config(CUDAStream), str);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(landmarkDetectHandle, NvAR_Parameter_Config(BatchSize), batchSize);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(landmarkDetectHandle, NvAR_Parameter_Config(Temporal), (bStabilizeFace ? -1 : 0));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(landmarkDetectHandle, NvAR_Parameter_Config(Landmarks_Size), numLandmarks);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(landmarkDetectHandle, NvAR_Parameter_Config(LandmarksConfidence_Size), numLandmarks);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(landmarkDetectHandle, NvAR_Parameter_Config(Mode), mode);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_Load(landmarkDetectHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errInitialization);

bail:
  return err;
}

FaceEngine::Err FaceEngine::createFaceFittingFeature(const char* modelPath, CUstream str) {
  FaceEngine::Err err = FaceEngine::Err::errNone;
  NvCV_Status nvErr;

  nvErr = NvAR_Create(NvAR_Feature_Face3DReconstruction, &faceFitHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errEffect);

  nvErr = NvAR_SetString(faceFitHandle, NvAR_Parameter_Config(ModelDir), modelPath);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetCudaStream(faceFitHandle, NvAR_Parameter_Config(CUDAStream), str);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetU32(faceFitHandle, NvAR_Parameter_Config(Landmarks_Size), numLandmarks);  // TODO: Check if nonzero??
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  if (!face_model.empty()) {
    nvErr = NvAR_SetString(faceFitHandle, NvAR_Parameter_Config(ModelName), face_model.c_str());
    BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  }

  nvErr = NvAR_Load(faceFitHandle);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errInitialization);

bail:
  return err;
}

FaceEngine::Err FaceEngine::initFeatureIOParams() {
  FaceEngine::Err err = FaceEngine::Err::errNone;

  NvCV_Status cvErr = NvCVImage_Alloc(&inputImageBuffer, input_image_width, input_image_height, NVCV_BGR, NVCV_U8,
                                      NVCV_CHUNKY, NVCV_GPU, 1);

  BAIL_IF_NVERR(cvErr, err, FaceEngine::Err::errInitialization);
  if (appMode == faceDetection) {
    err = initFaceDetectionIOParams(&inputImageBuffer);
    if (err != Err::errNone) {
      printf("ERROR: An error has occured while setting input, output parmeters for Face Detection\n");
    }
  } else if (appMode == landmarkDetection) {
    err = initLandmarkDetectionIOParams(&inputImageBuffer);
    if (err != Err::errNone) {
      printf("ERROR: An error has occured while setting input, output parmeters for Landmark Detection\n");
    }
  } else if (appMode == faceMeshGeneration) {
    err = initFaceFittingIOParams(&inputImageBuffer);
    if (err != Err::errNone) {
      printf("ERROR: An error has occured while setting input, output parmeters for Face Fitting\n");
    }
  }
  return err;

bail:
  return err;
}

FaceEngine::Err FaceEngine::initFaceDetectionIOParams(NvCVImage* inBuf) {
  NvCV_Status nvErr = NVCV_SUCCESS;
  FaceEngine::Err err = FaceEngine::Err::errNone;

  nvErr = NvAR_SetObject(faceDetectHandle, NvAR_Parameter_Input(Image), inBuf, sizeof(NvCVImage));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  output_bbox_data.assign(25, {0.f, 0.f, 0.f, 0.f});
  output_bbox_conf_data.assign(25, 0.f);
  output_bboxes.boxes = output_bbox_data.data();
  output_bboxes.max_boxes = (uint8_t)output_bbox_data.size();
  output_bboxes.num_boxes = 0;
  nvErr = NvAR_SetObject(faceDetectHandle, NvAR_Parameter_Output(BoundingBoxes), &output_bboxes, sizeof(NvAR_BBoxes));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetF32Array(faceDetectHandle, NvAR_Parameter_Output(BoundingBoxesConfidence),
                           output_bbox_conf_data.data(), output_bboxes.max_boxes);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

bail:
  return err;
}

FaceEngine::Err FaceEngine::initLandmarkDetectionIOParams(NvCVImage* inBuf) {
  NvCV_Status nvErr = NVCV_SUCCESS;
  FaceEngine::Err err = FaceEngine::Err::errNone;
  uint output_bbox_size;

  nvErr = NvAR_SetObject(landmarkDetectHandle, NvAR_Parameter_Input(Image), inBuf, sizeof(NvCVImage));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  unsigned int OUTPUT_SIZE_KPTS, OUTPUT_SIZE_KPTS_CONF;
  nvErr = NvAR_GetU32(landmarkDetectHandle, NvAR_Parameter_Config(Landmarks_Size), &OUTPUT_SIZE_KPTS);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_GetU32(landmarkDetectHandle, NvAR_Parameter_Config(LandmarksConfidence_Size), &OUTPUT_SIZE_KPTS_CONF);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  facial_landmarks.assign(batchSize * OUTPUT_SIZE_KPTS, {0.f, 0.f});
  facial_pose.assign(batchSize, {0.f, 0.f, 0.f, 0.f});
  facial_landmarks_confidence.assign(batchSize * OUTPUT_SIZE_KPTS_CONF, 0.f);

  nvErr = NvAR_SetObject(landmarkDetectHandle, NvAR_Parameter_Output(Landmarks), facial_landmarks.data(),
                         sizeof(NvAR_Point2f));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr =
      NvAR_SetObject(landmarkDetectHandle, NvAR_Parameter_Output(Pose), facial_pose.data(), sizeof(NvAR_Quaternion));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetF32Array(landmarkDetectHandle, NvAR_Parameter_Output(LandmarksConfidence),
                           facial_landmarks_confidence.data(), batchSize * OUTPUT_SIZE_KPTS);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  output_bbox_size = batchSize;
  if (!bStabilizeFace) output_bbox_size = 25;
  output_bbox_data.assign(output_bbox_size, {0.f, 0.f, 0.f, 0.f});
  output_bboxes.boxes = output_bbox_data.data();
  output_bboxes.max_boxes = (uint8_t)output_bbox_size;
  output_bboxes.num_boxes = (uint8_t)output_bbox_size;
  nvErr =
      NvAR_SetObject(landmarkDetectHandle, NvAR_Parameter_Output(BoundingBoxes), &output_bboxes, sizeof(NvAR_BBoxes));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

bail:
  return err;
}

FaceEngine::Err FaceEngine::initFaceFittingIOParams(NvCVImage* inBuf) {
  NvCV_Status nvErr = NVCV_SUCCESS;
  FaceEngine::Err err = FaceEngine::Err::errNone;

  face_mesh = new NvAR_FaceMesh();

  unsigned int num_vertices, num_triangles;
  nvErr = NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(VertexCount), &num_vertices);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(TriangleCount), &num_triangles);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  face_mesh->num_vertices = num_vertices;
  face_mesh->num_triangles = num_triangles;
  m_vertices.assign(num_vertices, {0.f, 0.f, 0.f});
  m_triangles.assign(num_triangles, { 0, 0, 0 });
  face_mesh->vertices = m_vertices.data();
  face_mesh->tvi = m_triangles.data();
  rendering_params = new NvAR_RenderingParams();

  nvErr = NvAR_SetObject(faceFitHandle, NvAR_Parameter_Input(Image), inBuf, sizeof(NvCVImage));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetS32(faceFitHandle, NvAR_Parameter_Input(Width), input_image_width);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetS32(faceFitHandle, NvAR_Parameter_Input(Height), input_image_height);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  unsigned int OUTPUT_SIZE_KPTS;
  nvErr = NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(Landmarks_Size), &OUTPUT_SIZE_KPTS);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  facial_landmarks.assign(batchSize * OUTPUT_SIZE_KPTS, {0.f, 0.f});
  nvErr =
      NvAR_SetObject(faceFitHandle, NvAR_Parameter_Output(Landmarks), facial_landmarks.data(), sizeof(NvAR_Point2f));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  facial_landmarks_confidence.assign(batchSize * OUTPUT_SIZE_KPTS, 0.f);
  nvErr = NvAR_SetF32Array(faceFitHandle, NvAR_Parameter_Output(LandmarksConfidence),
                           facial_landmarks_confidence.data(), batchSize * OUTPUT_SIZE_KPTS);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  facial_pose.assign(batchSize, {0.f, 0.f, 0.f, 0.f});
  nvErr = NvAR_SetObject(faceFitHandle, NvAR_Parameter_Output(Pose), facial_pose.data(), sizeof(NvAR_Quaternion));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  output_bbox_data.assign(batchSize, {0.f, 0.f, 0.f, 0.f});
  output_bboxes.boxes = output_bbox_data.data();
  output_bboxes.max_boxes = (uint8_t)batchSize;
  output_bboxes.num_boxes = (uint8_t)batchSize;
  nvErr = NvAR_SetObject(faceFitHandle, NvAR_Parameter_Output(BoundingBoxes), &output_bboxes, sizeof(NvAR_BBoxes));

  nvErr = NvAR_SetObject(faceFitHandle, NvAR_Parameter_Output(FaceMesh), face_mesh, sizeof(NvAR_FaceMesh));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_SetObject(faceFitHandle, NvAR_Parameter_Output(RenderingParams), rendering_params,
                         sizeof(NvAR_RenderingParams));
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  unsigned n;
  nvErr = NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(ShapeEigenValueCount), &n);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  shapeEigenvalues.resize(n);
  nvErr = NvAR_SetF32Array(faceFitHandle, NvAR_Parameter_Output(ShapeEigenValues), shapeEigenvalues.data(), n);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

  nvErr = NvAR_GetU32(faceFitHandle, NvAR_Parameter_Config(ExpressionCount), &n);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);
  expressionCoefficients.resize(n);
  nvErr = NvAR_SetF32Array(faceFitHandle, NvAR_Parameter_Output(ExpressionCoefficients), expressionCoefficients.data(), n);
  BAIL_IF_NVERR(nvErr, err, FaceEngine::Err::errParameter);

bail:
  return err;
}

void FaceEngine::destroyFeatures() {
  if (stream) {
    NvAR_CudaStreamDestroy(stream);
    stream = 0;
  }
  releaseFeatureIOParams();
  destroyFaceDetectionFeature();
  destroyLandmarkDetectionFeature();
  destroyFaceFittingFeature();
}

void FaceEngine::destroyFaceDetectionFeature() {
  if (faceDetectHandle) {
    (void)NvAR_Destroy(faceDetectHandle);
    faceDetectHandle = nullptr;
  }
}
void FaceEngine::destroyLandmarkDetectionFeature() {
  if (landmarkDetectHandle) {
    (void)NvAR_Destroy(landmarkDetectHandle);
    landmarkDetectHandle = nullptr;
  }
}
void FaceEngine::destroyFaceFittingFeature() {
  if (faceFitHandle) {
    (void)NvAR_Destroy(faceFitHandle);
    faceFitHandle = nullptr;
  }
}

void FaceEngine::releaseFeatureIOParams() {
  releaseFaceDetectionIOParams();
  releaseLandmarkDetectionIOParams();
  releaseFaceFittingIOParams();
}

void FaceEngine::releaseFaceDetectionIOParams() {
  NvCVImage_Dealloc(&inputImageBuffer);
  if (!output_bbox_data.empty()) output_bbox_data.clear();
  if (!output_bbox_conf_data.empty()) output_bbox_conf_data.clear();
}

void FaceEngine::releaseLandmarkDetectionIOParams() {
  NvCVImage_Dealloc(&inputImageBuffer);
  if (!output_bbox_data.empty()) output_bbox_data.clear();
  if (!facial_landmarks.empty()) facial_landmarks.clear();
  if (!facial_pose.empty()) facial_pose.clear();
  if (!facial_landmarks_confidence.empty()) facial_landmarks_confidence.clear();
}

void FaceEngine::releaseFaceFittingIOParams() {
  if (!output_bbox_data.empty()) output_bbox_data.clear();
  if (!facial_landmarks.empty()) facial_landmarks.clear();
  if (!facial_landmarks_confidence.empty()) facial_landmarks_confidence.clear();
  if (!facial_pose.empty()) facial_pose.clear();
  NvCVImage_Dealloc(&inputImageBuffer);
  if (rendering_params) {
    delete rendering_params;
    rendering_params = nullptr;
  }
  if (face_mesh) {
    delete face_mesh;
    face_mesh = nullptr;
  }
}

unsigned FaceEngine::findFaceBoxes() {
  NvCV_Status nvErr = NvAR_Run(faceDetectHandle);
  if (NVCV_SUCCESS != nvErr) return 0;
  return (unsigned)output_bboxes.num_boxes;
}

NvAR_Rect* FaceEngine::getLargestBox() {
  NvAR_Rect *box, *bigBox, *lastBox;
  float maxArea, area;
  for (lastBox = (box = &output_bboxes.boxes[0]) + output_bboxes.num_boxes, bigBox = nullptr, maxArea = 0;
       box != lastBox; ++box) {
    if (maxArea < (area = box->width * box->height)) {
      maxArea = area;
      bigBox = box;
    }
  }
  return bigBox;
}

NvAR_BBoxes* FaceEngine::getBoundingBoxes() { return &output_bboxes; }

void FaceEngine::enlargeAndSquarifyImageBox(float enlarge, NvAR_Rect& box, int FLAG_variant) {
  NvAR_Vector2f size = {box.width * .5f, box.height * .5f};
  NvAR_Point2f center = {box.x + size.x, box.y + size.y};
  float t;

  size.x *= (1.f + enlarge);
  size.y *= (1.f + enlarge);

  if (!(FLAG_variant & 1)) /* Default: enforce square bounding box */
  {
    if (size.x < size.y) /* Make square */
      size.x = size.y;
    else
      size.y = size.x;
  }

  if (center.x < size.x) /* Shift box into image left-right */
    center.x = size.x;
  else if (center.x > (t = input_image_width - size.x))
    center.x = t;

  if (center.y < size.y) /* Shift box into image up-down */
    center.y = size.y;
  else if (center.y > (t = input_image_height - size.y))
    center.y = t;

  // TODO: Above we assume that the box is smaller than the image.

  box.width = roundf(size.x * 2.f); /* Integral box */
  box.height = roundf(size.y * 2.f);
  box.x = roundf(center.x - box.width * .5f);
  box.y = roundf(center.y - box.height * .5f);
}

/**
 * Perturb a bounding box.
 * @param[in,out] ran the random number generator.
 * @param[in] minMag inset magnitude of noise.
 * @param[in] maxMag outset magnitude of noise.
 * @param[in] cleanBox the clean input box.
 * @param[out] noisyBox the perturbed output box.
 */
// TOD0: Fix the 8 bounding boxes instead if random jiggling
void FaceEngine::jiggleBox(std::mt19937& ran, float minMag, float maxMag, const NvAR_Rect& cleanBox,
                           NvAR_Rect& jitteredBox) {
  if (0.f == minMag && 0.f == maxMag) {
    jitteredBox = cleanBox;
    return;
  }

  std::uniform_real_distribution<float> uRand(minMag, maxMag);
#if 1  // jitter both sides
  jitteredBox.x = cleanBox.x - uRand(ran);
  jitteredBox.y = cleanBox.y - uRand(ran);
  jitteredBox.width = cleanBox.width + cleanBox.x + uRand(ran) - jitteredBox.x;
  jitteredBox.height = cleanBox.height + cleanBox.y + uRand(ran) - jitteredBox.y;
#elif 0  // jitter only the right side
  noisyBox.x = cleanBox.x;
  noisyBox.y = cleanBox.y;
  noisyBox.width = cleanBox.width + cleanBox.x + uRand(ran) - noisyBox.x;
  noisyBox.height = cleanBox.height + cleanBox.y + uRand(ran) - noisyBox.y;
#elif 1  // jitter only the location
  noisyBox.x = cleanBox.x - uRand(ran);
  noisyBox.y = cleanBox.y - uRand(ran);
  noisyBox.width = cleanBox.width;
  noisyBox.height = cleanBox.height;
#endif   // None of these makes a significant difference
}

#ifdef UNUSED
/** Intersect a rectangle with an image.
 * @param[in]   srcRect   the source rectangle.
 * @param[in]   src       the source image.
 * @param[out]  clipRect  the source rectangle clipped to the source image.
 * @return      true      if the rectangle was clipped,
 *              false     if the rectangle did not need to be clipped.
 */
static bool IntersectRectWithImage(const cv::Rect& srcRect, const cv::Mat& src, cv::Rect& clipRect) {
  bool result = 0;  // unclipped
  cv::Point2i rect[2] = {{srcRect.x, srcRect.y}, {srcRect.x + srcRect.width, srcRect.y + srcRect.height}};
  if (rect[0].x < 0) {
    result = 1;
    rect[0].x = 0;
  }  // clipped
  if (rect[0].y < 0) {
    result = 1;
    rect[0].y = 0;
  }  // clipped
  if (rect[1].x > src.cols) {
    result = 1;
    rect[1].x = src.cols;
  }  // clipped
  if (rect[1].y > src.rows) {
    result = 1;
    rect[1].y = src.rows;
  }  // clipped
  clipRect.x = rect[0].x;
  clipRect.y = rect[0].y;
  clipRect.width = rect[1].x - rect[0].x;
  clipRect.height = rect[1].y - rect[0].y;
  return result;
}
#endif // UNUSED

void FaceEngine::DrawPose(const cv::Mat& src, const NvAR_Quaternion* pose) const {
  const float vector_scale = 50.0f;
  const int thickness = 2;
  float rot_mat[3][3];
  set_rotation_from_quaternion(pose, rot_mat[0]);
  const auto avg_landmark_pos = GetAverageLandmarkPositionInGlSpace();

  cv::Point p0 = {static_cast<int>(avg_landmark_pos[0]), static_cast<int>(avg_landmark_pos[1])};
  cv::Point px = p0 + cv::Point(std::lround(rot_mat[0][0] * vector_scale), std::lround(rot_mat[0][1] * vector_scale));
  cv::Point py = p0 + cv::Point(std::lround(rot_mat[1][0] * vector_scale), std::lround(rot_mat[1][1] * vector_scale));
  cv::Point pz = p0 + cv::Point(std::lround(rot_mat[2][0] * vector_scale), std::lround(rot_mat[2][1] * vector_scale));

  // Convert from OpenGL to OpenCV space
  p0.y = src.rows - 1 - p0.y;
  px.y = src.rows - 1 - px.y;
  py.y = src.rows - 1 - py.y;
  pz.y = src.rows - 1 - pz.y;

  cv::line(src, p0, px, CV_RGB(255, 0, 0), thickness);
  cv::line(src, p0, py, CV_RGB(0, 255, 0), thickness);
  cv::line(src, p0, pz, CV_RGB(0, 0, 255), thickness);
}

NvCV_Status FaceEngine::findLandmarks() {
  NvCV_Status nvErr;

  nvErr = NvAR_Run(landmarkDetectHandle);
  if (NVCV_SUCCESS != nvErr) {
    return nvErr;
  }

  if (getAverageLandmarksConfidence() < confidenceThreshold) {
    return NVCV_ERR_GENERAL;
  } else {
    average_poses(getPose(), batchSize);
    NvAR_Point2f *pt, *endPt;
    int i = 0;
    for (endPt = (pt = getLandmarks()) + numLandmarks; pt != endPt; ++pt, i += 2) {
      for (int j = 1; j < batchSize; j++) {
        pt->x += pt[j * numLandmarks].x;
        pt->y += pt[j * numLandmarks].y;
      }
      // average batch of inferences to generate final result landmark points
      pt->x /= batchSize;
      pt->y /= batchSize;
    }
  }
  return NVCV_SUCCESS;
}

NvAR_Point2f* FaceEngine::getLandmarks() { return facial_landmarks.data(); }

float* FaceEngine::getLandmarksConfidence() { return facial_landmarks_confidence.data(); }

std::array<float, 2> FaceEngine::GetAverageLandmarkPositionInGlSpace() const {
  std::array<float, 2> res = { 0.0f, 0.0f };
  for (const auto& landmark : facial_landmarks) {
    res[0] += landmark.x;
    res[1] += landmark.y;
  }
  res[0] /= facial_landmarks.size();
  res[1] /= facial_landmarks.size();
  res[1] = input_image_height - res[1]; //Convert y coordinate from CV to GL space
  return res;
}

float FaceEngine::getAverageLandmarksConfidence() {
  float average_confidence = 0.0f;
  float* keypoints_landmarks_confidence = getLandmarksConfidence();
  for (int i = 0; i < batchSize; i++) {
    for (int j = 0; j < numLandmarks; j++) {
      average_confidence += keypoints_landmarks_confidence[i * numLandmarks + j];
    }
  }
  average_confidence /= batchSize * numLandmarks;
  return average_confidence;
}

NvAR_Quaternion* FaceEngine::getPose() { return facial_pose.data(); }

unsigned FaceEngine::findLargestFaceBox(NvAR_Rect& faceBox, int variant) {
  unsigned n;
  NvAR_Rect* pFaceBox;

  n = findFaceBoxes();
  if (n >= 1) {
    pFaceBox = getLargestBox();
    if (nullptr == pFaceBox) {
      faceBox.x = faceBox.y = faceBox.width = faceBox.height = 0.0f;
    } else {
      faceBox = *pFaceBox;
    }
    enlargeAndSquarifyImageBox(.2f, faceBox, variant);
  }
  return n;
}

unsigned FaceEngine::acquireFaceBox(cv::Mat& src, NvAR_Rect& faceBox, int variant) {
  unsigned n = 0;
  NvCVImage fxSrcChunkyCPU;
  (void)NVWrapperForCVMat(&src, &fxSrcChunkyCPU);
  NvCV_Status cvErr = NvCVImage_Transfer(&fxSrcChunkyCPU, &inputImageBuffer, 1.0f, stream, &tmpImage);

  if (NVCV_SUCCESS != cvErr) {
    return n;
  }

  n = findLargestFaceBox(faceBox, variant);
  return n;
}

unsigned FaceEngine::acquireFaceBoxAndLandmarks(cv::Mat& src, NvAR_Point2f* refMarks, NvAR_Rect& faceBox, int /*variant*/) {
  unsigned n = 0;
  NvCVImage fxSrcChunkyCPU;
  (void)NVWrapperForCVMat(&src, &fxSrcChunkyCPU);
  NvCV_Status cvErr = NvCVImage_Transfer(&fxSrcChunkyCPU, &inputImageBuffer, 1.0f, stream, &tmpImage);

  if (NVCV_SUCCESS != cvErr) {
    return n;
  }


  if (findLandmarks() != NVCV_SUCCESS) return 0;
  faceBox = output_bboxes.boxes[0];
  n = 1;
  memcpy(refMarks, getLandmarks(), sizeof(NvAR_Point2f) * numLandmarks);

  return n;
}

void FaceEngine::setFaceStabilization(bool _bStabilizeFace) { bStabilizeFace = _bStabilizeFace; }

FaceEngine::Err FaceEngine::setNumLandmarks(int n) {
  FaceEngine::Err err = errNone;
#ifdef _WIN32 // Keep Updated Thresholds for Windows Only
  for (auto const& info : LANDMARKS_INFO) 
#else
  for (auto const& info : LANDMARKS_INFO_UNIX)
#endif
  {
    if (n == info.numPoints) {
      numLandmarks = info.numPoints;
      confidenceThreshold = info.confidence_threshold;
      return err;
    }
  }    
  err = errGeneral;
  return err;
}

void FaceEngine::setAppMode(FaceEngine::mode _mode) { appMode = _mode; }


```

## Source file: OSS/samples/ExpressionApp/BackEndOpenGL/FaceIO.h

{
  "path": "OSS/samples/ExpressionApp/BackEndOpenGL/FaceIO.h",
  "blob_sha": "2c42dd086385ded2c5cca3204ceeffa5477df41f",
  "sha256": "cad5acb3a0e51d81ee45f6e54fc281590bc5e88ca2508dea58c95dd2134ef786",
  "lines": 231,
  "bytes": 14445,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 9,
    "jaw": 2,
    "rig": 19,
    "face": 49
  },
  "url": "https://github.com/emoto-yasushi/NVIDIA_ARSDK_iFacialMocap/blob/ccbe82cb141cd427753f1ede7d0981135fb65cba/OSS/samples/ExpressionApp/BackEndOpenGL/FaceIO.h",
  "function_signature_candidates": [
    {
      "line": 80,
      "text": "virtual void setTriangleListSize(uint32_t /*size*/) {} /* The triangle list */"
    },
    {
      "line": 84,
      "text": "virtual void setTextureCoordinatesSize(uint32_t /*size*/) {} /* The texture coordinates */"
    },
    {
      "line": 88,
      "text": "virtual void setNumBlendShapes(uint32_t /*numShapes*/) {} /* The blend shapes */"
    },
    {
      "line": 89,
      "text": "virtual void setBlendShapeName(uint32_t /*i*/, const char* /*name*/) {}"
    },
    {
      "line": 95,
      "text": "virtual void setIbugLandmarkMappingsSize(uint32_t /*n*/) {} /* The mappings from IBUG landmarks to vertex index */"
    },
    {
      "line": 98,
      "text": "virtual void appendIbugLandmarkMapping(uint16_t /*i*/) {}"
    },
    {
      "line": 99,
      "text": "virtual void appendIbugLandmarkMapping(uint16_t /*i*/, uint16_t /*j*/) {}"
    },
    {
      "line": 101,
      "text": "virtual void setIbugRightContourSize(uint32_t /*n*/) {} /* The IBUG contour on the right side of the face */"
    },
    {
      "line": 104,
      "text": "virtual void appendIbugRightContour(uint16_t /*i*/) {}"
    },
    {
      "line": 106,
      "text": "virtual void setIbugLeftContourSize(uint32_t /*n*/) {} /* The IBUG contour on the left side of the face */"
    },
    {
      "line": 109,
      "text": "virtual void appendIbugLeftContour(uint16_t /*i*/) {}"
    },
    {
      "line": 111,
      "text": "virtual void setModelRightContourSize(uint32_t /*n*/) {} /* The right contour of our model */"
    },
    {
      "line": 114,
      "text": "virtual void appendModelRightContour(uint16_t /*i*/) {}"
    },
    {
      "line": 116,
      "text": "virtual void setModelLeftContourSize(uint32_t /*n*/) {} /* The left contour of our model */"
    },
    {
      "line": 119,
      "text": "virtual void appendModelLeftContour(uint16_t /*i*/) {}"
    },
    {
      "line": 121,
      "text": "virtual void setAdjacentFacesSize(uint32_t /*n*/) {} /* The topology of adjacent faces to each edge */"
    },
    {
      "line": 124,
      "text": "virtual void appendAdjacentFace(uint16_t /*i*/) {}"
    },
    {
      "line": 125,
      "text": "virtual void appendAdjacentFaces(uint16_t /*i*/, uint16_t /*j*/) {}"
    },
    {
      "line": 127,
      "text": "virtual void setAdjacentVerticesSize(uint32_t /*n*/) {} /* The topology of adjacent vertices to each edge */"
    },
    {
      "line": 130,
      "text": "virtual void appendAdjacentVertex(uint16_t /*i*/) {}"
    },
    {
      "line": 131,
      "text": "virtual void appendAdjacentVertices(uint16_t /*i*/, uint16_t /*j*/) {}"
    },
    {
      "line": 133,
      "text": "virtual void setNvlmLandmarksSize(uint32_t /*n*/) {}      /* The tracked landmarks */"
    },
    {
      "line": 136,
      "text": "virtual void appendNvlmLandmark(uint16_t /*i*/) {}"
    },
    {
      "line": 138,
      "text": "virtual void setNvlmRightContourSize(uint32_t /*n*/) {}   /* The tracked right jawline contour */"
    },
    {
      "line": 141,
      "text": "virtual void appendNvlmRightContour(uint16_t /*i*/) {}"
    },
    {
      "line": 143,
      "text": "virtual void setNvlmLeftContourSize(uint32_t /*n*/) {};    /* The tracked left jawline contour */"
    },
    {
      "line": 146,
      "text": "virtual void appendNvlmLeftContour(uint16_t /*i*/) {}"
    },
    {
      "line": 148,
      "text": "virtual void setNumPartitions(uint32_t /*n*/) {}"
    },
    {
      "line": 149,
      "text": "virtual void setPartitionName(uint32_t /*i*/, const char* /*name*/) {}"
    },
    {
      "line": 150,
      "text": "virtual void setPartitionMaterialName(uint32_t /*i*/, const char* /*name*/) {}"
    },
    {
      "line": 151,
      "text": "virtual void setPartition(uint32_t /*i*/, uint32_t /*faceIndex*/, uint32_t /*numFaces*/,"
    }
  ]
}

```text

/*###############################################################################
#
# Copyright 2019-2021 NVIDIA Corporation
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of
# this software and associated documentation files (the "Software"), to deal in
# the Software without restriction, including without limitation the rights to
# use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
# the Software, and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
# FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
# COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
# IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
# CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#
###############################################################################*/

#ifndef __FACE_IO__
#define __FACE_IO__

#include <stdint.h>

enum FaceIOErr {
  kIOErrNone,
  kIOErrFileNotFound,
  kIOErrFileOpen,
  kIOErrEOF,
  kIOErrRead,
  kIOErrWrite,
  kIOErrSyntax,
  kIOErrFormat,
  kIOErrNotValue,
  kIOErrNullPointer,
  kIOErrParameter,
};

const char* FaceIOErrorStringFromCode(FaceIOErr err);

/********************************************************************************
 ********************************************************************************
 ********************************************************************************
 *****                               IO Adapter                             *****
 ********************************************************************************
 ********************************************************************************
 ********************************************************************************/

/********************************************************************************
 * FaceIOAdapter.
 * Subclass from this and supply the accessors.
 ********************************************************************************/

class FaceIOAdapter {
public:
  virtual uint32_t getShapeMeanSize() const { return 0; }  /* The size of the mean shape mean, in elements. */
  virtual uint32_t getShapeModesSize() const { return 0; } /* The total size of all shape modes (numModes*modeSize) */
  virtual uint32_t getShapeNumModes() const { return 0; }  /* The number of shape modes. */
  virtual uint32_t getShapeEigenvaluesSize() const { return 0; } /* The number of shape eigenvalues
                                                        (should equal the number of modes) */
  virtual float* getShapeMean(uint32_t /*size*/) { return nullptr; } /* Get a pointer to the shape mean.
                                                                 If a nonzero size if supplied, it is resized first. */
  virtual float* getShapeModes(uint32_t /*modeSize*/, uint32_t /*numModes*/) { return nullptr; } /* Get a pointer to the
                                                        shape modes, resizing first, if the parameters are nonzero. */
  virtual float* getShapeEigenvalues(uint32_t /*numModes*/) { return nullptr; } /* Get a pointer to the shape eigenvalues,
                                                             resizing first if numModes is nonzero. */

  virtual uint32_t getColorMeanSize() const { return 0; }  /* The color mean ... */
  virtual uint32_t getColorModesSize() const { return 0; } /* ... and modes */
  virtual uint32_t getColorNumModes() const { return 0; }
  virtual uint32_t getColorEigenvaluesSize() const { return 0; }
  virtual float* getColorMean(uint32_t /*size*/) { return nullptr; }
  virtual float* getColorModes(uint32_t /*modeSize*/, uint32_t /*numModes*/) { return nullptr; }
  virtual float* getColorEigenvalues(uint32_t /*numModes*/) { return nullptr; }

  virtual void setTriangleListSize(uint32_t /*size*/) {} /* The triangle list */
  virtual uint32_t getTriangleListSize() const = 0;
  virtual uint16_t* getTriangleList(uint32_t /*size*/) { return nullptr; }

  virtual void setTextureCoordinatesSize(uint32_t /*size*/) {} /* The texture coordinates */
  virtual uint32_t getTextureCoordinatesSize() const { return 0; }
  virtual float* getTextureCoordinates(uint32_t /*size*/) { return nullptr; }

  virtual void setNumBlendShapes(uint32_t /*numShapes*/) {} /* The blend shapes */
  virtual void setBlendShapeName(uint32_t /*i*/, const char* /*name*/) {}
  virtual uint32_t getNumBlendShapes() const { return 0; }
  virtual const char* getBlendShapeName(uint32_t /*i*/) const { return nullptr; }
  virtual uint32_t getBlendShapeSize(uint32_t /*i*/) const { return 0; }
  virtual float* getBlendShape(uint32_t /*i*/, uint32_t /*size*/) { return nullptr; }

  virtual void setIbugLandmarkMappingsSize(uint32_t /*n*/) {} /* The mappings from IBUG landmarks to vertex index */
  virtual uint32_t getIbugLandmarkMappingsSize() const { return 0; }
  virtual uint16_t* getIbugLandmarkMappings(uint32_t /*size*/) { return nullptr; }
  virtual void appendIbugLandmarkMapping(uint16_t /*i*/) {}
  virtual void appendIbugLandmarkMapping(uint16_t /*i*/, uint16_t /*j*/) {}

  virtual void setIbugRightContourSize(uint32_t /*n*/) {} /* The IBUG contour on the right side of the face */
  virtual uint32_t getIbugRightContourSize() const { return 0; }
  virtual uint16_t* getIbugRightContour(uint32_t /*size*/) { return nullptr; }
  virtual void appendIbugRightContour(uint16_t /*i*/) {}

  virtual void setIbugLeftContourSize(uint32_t /*n*/) {} /* The IBUG contour on the left side of the face */
  virtual uint32_t getIbugLeftContourSize() const { return 0; }
  virtual uint16_t* getIbugLeftContour(uint32_t /*size*/) { return nullptr; }
  virtual void appendIbugLeftContour(uint16_t /*i*/) {}

  virtual void setModelRightContourSize(uint32_t /*n*/) {} /* The right contour of our model */
  virtual uint32_t getModelRightContourSize() const { return 0; }
  virtual uint16_t* getModelRightContour(uint32_t /*size*/) { return nullptr; }
  virtual void appendModelRightContour(uint16_t /*i*/) {}

  virtual void setModelLeftContourSize(uint32_t /*n*/) {} /* The left contour of our model */
  virtual uint32_t getModelLeftContourSize() const { return 0; }
  virtual uint16_t* getModelLeftContour(uint32_t /*size*/) { return nullptr; }
  virtual void appendModelLeftContour(uint16_t /*i*/) {}

  virtual void setAdjacentFacesSize(uint32_t /*n*/) {} /* The topology of adjacent faces to each edge */
  virtual uint32_t getAdjacentFacesSize() const { return 0; }
  virtual uint16_t* getAdjacentFaces(uint32_t /*size*/) { return nullptr; }
  virtual void appendAdjacentFace(uint16_t /*i*/) {}
  virtual void appendAdjacentFaces(uint16_t /*i*/, uint16_t /*j*/) {}

  virtual void setAdjacentVerticesSize(uint32_t /*n*/) {} /* The topology of adjacent vertices to each edge */
  virtual uint32_t getAdjacentVerticesSize() const { return 0; }
  virtual uint16_t* getAdjacentVertices(uint32_t /*size*/) { return nullptr; }
  virtual void appendAdjacentVertex(uint16_t /*i*/) {}
  virtual void appendAdjacentVertices(uint16_t /*i*/, uint16_t /*j*/) {}

  virtual void setNvlmLandmarksSize(uint32_t /*n*/) {}      /* The tracked landmarks */
  virtual uint32_t getNvlmLandmarksSize() const { return 0; }
  virtual uint16_t* getNvlmLandmarks(uint32_t /*size*/) { return nullptr; }
  virtual void appendNvlmLandmark(uint16_t /*i*/) {}

  virtual void setNvlmRightContourSize(uint32_t /*n*/) {}   /* The tracked right jawline contour */
  virtual uint32_t getNvlmRightContourSize() const { return 0; }
  virtual uint16_t* getNvlmRightContour(uint32_t /*size*/) { return nullptr; }
  virtual void appendNvlmRightContour(uint16_t /*i*/) {}

  virtual void setNvlmLeftContourSize(uint32_t /*n*/) {};    /* The tracked left jawline contour */
  virtual uint32_t getNvlmLeftContourSize() const { return 0; }
  virtual uint16_t* getNvlmLeftContour(uint32_t /*size*/) { return nullptr; }
  virtual void appendNvlmLeftContour(uint16_t /*i*/) {}

  virtual void setNumPartitions(uint32_t /*n*/) {}
  virtual void setPartitionName(uint32_t /*i*/, const char* /*name*/) {}
  virtual void setPartitionMaterialName(uint32_t /*i*/, const char* /*name*/) {}
  virtual void setPartition(uint32_t /*i*/, uint32_t /*faceIndex*/, uint32_t /*numFaces*/,
                            uint32_t /*vertexIndex*/, uint32_t /*numVertices*/, int32_t /*smoothingGroup*/) {}
  virtual uint32_t getNumPartitions() const { return 0; }
  virtual const char* getPartitionName(uint32_t /*i*/) const { return nullptr; }
  virtual const char* getPartitionMaterialName(uint32_t /*i*/) const { return nullptr; }
  virtual int16_t getPartition(uint32_t /*i*/, uint32_t* faceIndex, uint32_t* numFaces, uint32_t* vertexIndex,
                               uint32_t* numVertices, int32_t* smoothingGroup) const
  { if (faceIndex) *faceIndex = 0u; if (numFaces) *numFaces = 0u; if (vertexIndex) *vertexIndex = 0u;
    if (numVertices) *numVertices = 0u; if (smoothingGroup) *smoothingGroup = -1; return /*partitionIndex*/-1;
  }


  /* Const accessors do not have the ability to resize. */
  const float* getShapeMean() const { return const_cast<FaceIOAdapter*>(this)->getShapeMean(0); }
  const float* getShapeModes() const { return const_cast<FaceIOAdapter*>(this)->getShapeModes(0, 0); }
  const float* getShapeEigenvalues() const { return const_cast<FaceIOAdapter*>(this)->getShapeEigenvalues(0); }
  const float* getColorMean() const { return const_cast<FaceIOAdapter*>(this)->getColorMean(0); }
  const float* getColorModes() const { return const_cast<FaceIOAdapter*>(this)->getColorModes(0, 0); }
  const float* getColorEigenvalues() const { return const_cast<FaceIOAdapter*>(this)->getColorEigenvalues(0); }
  const float* getTextureCoordinates() const { return const_cast<FaceIOAdapter*>(this)->getTextureCoordinates(0); }
  const uint16_t* getTriangleList() const { return const_cast<FaceIOAdapter*>(this)->getTriangleList(0); }
  const float* getBlendShape(uint32_t i) const { return const_cast<FaceIOAdapter*>(this)->getBlendShape(i, 0); }
  const uint16_t* getIbugLandmarkMappings() const { return const_cast<FaceIOAdapter*>(this)->getIbugLandmarkMappings(0); }
  const uint16_t* getIbugRightContour() const { return const_cast<FaceIOAdapter*>(this)->getIbugRightContour(0); }
  const uint16_t* getIbugLeftContour() const { return const_cast<FaceIOAdapter*>(this)->getIbugLeftContour(0); }
  const uint16_t* getModelRightContour() const { return const_cast<FaceIOAdapter*>(this)->getModelRightContour(0); }
  const uint16_t* getModelLeftContour() const { return const_cast<FaceIOAdapter*>(this)->getModelLeftContour(0); }
  const uint16_t* getAdjacentFaces() const { return const_cast<FaceIOAdapter*>(this)->getAdjacentFaces(0); }
  const uint16_t* getAdjacentVertices() const { return const_cast<FaceIOAdapter*>(this)->getAdjacentVertices(0); }
  const uint16_t* getNvlmLandmarks() const { return const_cast<FaceIOAdapter*>(this)->getNvlmLandmarks(0); }
  const uint16_t* getNvlmRightContour() const { return const_cast<FaceIOAdapter*>(this)->getNvlmRightContour(0); }
  const uint16_t* getNvlmLeftContour() const { return const_cast<FaceIOAdapter*>(this)->getNvlmLeftContour(0); }
};

/** Write the face model as an NVF model.
 * @param[in]   fac         the face I/O adapter for the target data structure.
 * @param[in]   fileName    the desired name of the output file.
 * @return      kIOErrNone       if the file was written completed successfully.
 * @return      kIOErrFileOpen   if the file could not be opened.
 * @return      kIOErrWrite      if an error occurred while writing the file.
 */
FaceIOErr WriteNVFFaceModel(FaceIOAdapter* fac, const char* fileName);

/** Read a face model from an NVF file.
 * @param[in]       fileName    the name of the file to be read.
 * @param[in,out]   fac         the face I/O adapter for the target data structure.
 * @return      kIOErrNone       if the file was read successfully.
 * @return      kIOErrFileNotFound   if the file was not found .
 * @return      kIOErrFileOpen       if the file could not be opened.
 * @return      kIOErrRead           if an error occurred while reading the file.
 * @return      kIOErrSyntax         if a syntax error has been encountered  while reading the file.
 */
FaceIOErr ReadNVFFaceModel(const char* fileName, FaceIOAdapter* fac);

/** Read a face model from five OES files.
 * @param[in]       shape                  the name of the shape        file to be read.
 * @param[in]       ibugNumandmarks        the number of Ibug landmarks.
 * @param[in]       blendShapes            the name of the blend shapes  file to be read.
 * @param[in]       contours               the name of the contours      file to be read.
 * @param[in]       topology               the name of the topology      file to be read.
 * @param[in,out]   fac                    the face I/O adapter for the target data structure.
 * @return      kIOErrNone       if the file was read successfully.
 * @return      kIOErrFileNotFound   if the file was not found .
 * @return      kIOErrFileOpen       if the file could not be opened.
 * @return      kIOErrRead           if an error occurred while reading the file.
 * @return      kIOErrSyntax         if a syntax error has been encountered  while reading the file.
 */
FaceIOErr ReadEOSFaceModel(const char* shape, const unsigned ibugNumLandmarks, const char* blendShapes,
                           const char* contours, const char* topology, FaceIOAdapter* fac);

/** Write the face model as a JSON model.
 * @param[in]   fac         the face I/O adapter for the target data structure.
 * @param[in]   fileName    the desired name of the output file.
 *                          If NULL is supplied, it is written to the standard output.
 * @return      kIOErrNone       if the file was written completed successfully.
 * @return      kIOErrFileOpen   if the file could not be opened.
 * @return      kIOErrWrite      if an error occurred while writing the file.
 */
FaceIOErr PrintJSONFaceModel(FaceIOAdapter* fac, const char* fileName);

#endif /* __FACE_IO__ */


```
