# Facial rendering source scan: laurentiuvieriu/facecept3d_facs

Repository: https://github.com/laurentiuvieriu/facecept3d_facs

Pinned source revision: 99f5e9d9821064e2fb8836658d027fe1ab0d3c1a

Discovery topic: FACS face

Description: 

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"facs": ["facs/3dheadposeestimation.sln", "facs/CMakeLists.txt", "facs/Config/CommonProperties.props", "facs/Config/CommonPropertiesLibsDebug.props", "facs/Config/CommonPropertiesLibsRelease.props", "facs/HeadPoseEstimationFramework/Align/CloudMapper.cpp", "facs/HeadPoseEstimationFramework/Align/CloudMapper.h", "facs/HeadPoseEstimationFramework/Align/IHeadTemplateCreator.h", "facs/HeadPoseEstimationFramework/Align/IncrementalHeadTemplateCreator.cpp", "facs/HeadPoseEstimationFramework/Align/IncrementalHeadTemplateCreator.h", "facs/HeadPoseEstimationFramework/Align/TransformationEstimationHPE.cpp", "facs/HeadPoseEstimationFramework/Align/TransformationEstimationHPE.h", "facs/HeadPoseEstimationFramework/CMakeLists.txt", "facs/HeadPoseEstimationFramework/Common.h", "facs/HeadPoseEstimationFramework/Converter/IDataConverter.h", "facs/HeadPoseEstimationFramework/Converter/KinectDataConverter.cpp", "facs/HeadPoseEstimationFramework/Converter/KinectDataConverter.h", "facs/HeadPoseEstimationFramework/DataFilter/IDataFilter.h", "facs/HeadPoseEstimationFramework/DataObject/CloudXYZRGBA.h", "facs/HeadPoseEstimationFramework/DataObject/HeadPoseInfo.h", "facs/HeadPoseEstimationFramework/DataObject/IDataObject.h", "facs/HeadPoseEstimationFramework/DataObject/IDataStorage.h", "facs/HeadPoseEstimationFramework/DataObject/LandmarksObject.h", "facs/HeadPoseEstimationFramework/DataObject/MapDataStorage.cpp", "facs/HeadPoseEstimationFramework/DataObject/MapDataStorage.h", "facs/HeadPoseEstimationFramework/DataObject/RawFrames.h", "facs/HeadPoseEstimationFramework/Detector/DepthFrameDetector/DepthDetector.cpp", "facs/HeadPoseEstimationFramework/Detector/DepthFrameDetector/DepthDetector.h", "facs/HeadPoseEstimationFramework/Detector/DepthFrameDetector/LeafNode.cpp", "facs/HeadPoseEstimationFramework/Detector/DepthFrameDetector/LeafNode.h"], "rig": ["docs/template-creator/RotateLeftRight.jpg"], "face": ["FaceCept3D/3dheadposeestimation.sln", "FaceCept3D/CMakeLists.txt", "FaceCept3D/Config/CommonProperties.props", "FaceCept3D/Config/CommonPropertiesLibsDebug.props", "FaceCept3D/Config/CommonPropertiesLibsRelease.props", "FaceCept3D/HeadPoseEstimationFramework/Align/CloudMapper.cpp", "FaceCept3D/HeadPoseEstimationFramework/Align/CloudMapper.h", "FaceCept3D/HeadPoseEstimationFramework/Align/IHeadTemplateCreator.h", "FaceCept3D/HeadPoseEstimationFramework/Align/IncrementalHeadTemplateCreator.cpp", "FaceCept3D/HeadPoseEstimationFramework/Align/IncrementalHeadTemplateCreator.h", "FaceCept3D/HeadPoseEstimationFramework/Align/TransformationEstimationHPE.cpp", "FaceCept3D/HeadPoseEstimationFramework/Align/TransformationEstimationHPE.h", "FaceCept3D/HeadPoseEstimationFramework/CMakeLists.txt", "FaceCept3D/HeadPoseEstimationFramework/Common.h", "FaceCept3D/HeadPoseEstimationFramework/Converter/IDataConverter.h", "FaceCept3D/HeadPoseEstimationFramework/Converter/KinectDataConverter.cpp", "FaceCept3D/HeadPoseEstimationFramework/Converter/KinectDataConverter.h", "FaceCept3D/HeadPoseEstimationFramework/DataFilter/IDataFilter.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/CloudXYZRGBA.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/HeadPoseInfo.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/IDataObject.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/IDataStorage.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/LandmarksObject.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/MapDataStorage.cpp", "FaceCept3D/HeadPoseEstimationFramework/DataObject/MapDataStorage.h", "FaceCept3D/HeadPoseEstimationFramework/DataObject/RawFrames.h", "FaceCept3D/HeadPoseEstimationFramework/Detector/DepthFrameDetector/DepthDetector.cpp", "FaceCept3D/HeadPoseEstimationFramework/Detector/DepthFrameDetector/DepthDetector.h", "FaceCept3D/HeadPoseEstimationFramework/Detector/DepthFrameDetector/LeafNode.cpp", "FaceCept3D/HeadPoseEstimationFramework/Detector/DepthFrameDetector/LeafNode.h"]}

## Source file: facs/HeadPoseEstimationFramework/Interface/IInterface.h

{
  "path": "facs/HeadPoseEstimationFramework/Interface/IInterface.h",
  "blob_sha": "3a3c0ee6d142694dbaf7f298cda2f6bece69dc0f",
  "sha256": "b42ca7796d6dfbaa58d159dfc267a61436446b0a40b17d96df6e9857ce328378",
  "lines": 15,
  "bytes": 186,
  "selection_score": 18,
  "keyword_line_counts": {
    "face": 5
  },
  "url": "https://github.com/laurentiuvieriu/facecept3d_facs/blob/99f5e9d9821064e2fb8836658d027fe1ab0d3c1a/facs/HeadPoseEstimationFramework/Interface/IInterface.h",
  "function_signature_candidates": []
}

```text

#pragma once

#ifndef IINTERFACE_H
#define IINTERFACE_H

namespace hpe
{
    class IInterface
    {
        public:
            virtual ~IInterface() {}
    };
}

#endif // IINTERFACE_H


```

## Source file: FaceCept3D/HeadPoseEstimationFramework/Interface/IInterface.h

{
  "path": "FaceCept3D/HeadPoseEstimationFramework/Interface/IInterface.h",
  "blob_sha": "3a3c0ee6d142694dbaf7f298cda2f6bece69dc0f",
  "sha256": "b42ca7796d6dfbaa58d159dfc267a61436446b0a40b17d96df6e9857ce328378",
  "lines": 15,
  "bytes": 186,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 5
  },
  "url": "https://github.com/laurentiuvieriu/facecept3d_facs/blob/99f5e9d9821064e2fb8836658d027fe1ab0d3c1a/FaceCept3D/HeadPoseEstimationFramework/Interface/IInterface.h",
  "function_signature_candidates": []
}

```text

#pragma once

#ifndef IINTERFACE_H
#define IINTERFACE_H

namespace hpe
{
    class IInterface
    {
        public:
            virtual ~IInterface() {}
    };
}

#endif // IINTERFACE_H


```

## Source file: facs/HeadPoseEstimationFramework/FacialAU/facsLocalFunctions.h

{
  "path": "facs/HeadPoseEstimationFramework/FacialAU/facsLocalFunctions.h",
  "blob_sha": "5249151c415bf31b3916e275bd427703fdeeb71d",
  "sha256": "702df1665043edae4a8a83a860ecc2eebd91d28457d2c12bda70032b7cbabd26",
  "lines": 62,
  "bytes": 1931,
  "selection_score": 15,
  "keyword_line_counts": {
    "facs": 21,
    "rig": 1
  },
  "url": "https://github.com/laurentiuvieriu/facecept3d_facs/blob/99f5e9d9821064e2fb8836658d027fe1ab0d3c1a/facs/HeadPoseEstimationFramework/FacialAU/facsLocalFunctions.h",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "void writeMatToFile_facs(cv::Mat& m, const char* filename);"
    },
    {
      "line": 38,
      "text": "void RV_readParamList_facs(string foldername, paramList_facs *p);"
    },
    {
      "line": 48,
      "text": "int RV_imshow_facs(string windowName, Mat img);"
    },
    {
      "line": 49,
      "text": "void RV_readLPQParams_facs(string path, vector<Mat> &filters, Mat &V);"
    },
    {
      "line": 54,
      "text": "int RV_coutMat_facs(Mat in);"
    }
  ]
}

```text

#ifndef FACSLOCALFUNCTIONS_H
#define FACSLOCALFUNCTIONS_H

#include <string>
#include <algorithm>
#include <iostream>
#include <vector>
#include <stdint.h>
#include "opencv2/core/core.hpp"

namespace facs
{

# define M_PI           3.14159265358979323846  /* pi */

    using namespace std;
    using namespace cv;

    struct paramList_facs
    {
        int patchSize;
        int stride;
        int featSize;
        int noPatches;
        int noTrees;
        Mat patchCoords;
    };

    struct randomTree_facs
    {
        vector<double> cutVar, cutValue, rightChild, leafVal;
    };

    //vector<string> get_all_files_names_within_folder(string folder);

    Mat RV_readCSVMat_facs(string csvFrameName, int lin, int col);
    void writeMatToFile_facs(cv::Mat& m, const char* filename);
    void RV_readParamList_facs(string foldername, paramList_facs *p);
    vector<randomTree_facs> RV_readAllTrees_facs(string forestDir, paramList_facs params);
    vector<double> RV_computeHist_facs(vector<double> vec, int noBins);

    vector<double> RV_testForest_facs(Mat localData, vector<randomTree_facs> &localForest, paramList_facs params, int numberOfClasses);

    //Rect RV_getBoundingBox(const Mat &im3D, paramList_facs p);
    double RV_computeNorm_facs(vector<double> vec1, vector<double> vec2);
    double RV_computeMean_facs(vector<double> vec);

    int RV_imshow_facs(string windowName, Mat img);
    void RV_readLPQParams_facs(string path, vector<Mat> &filters, Mat &V);
    vector<Mat> RV_lpqImage_facs(Mat img, vector<Mat> filters, Mat V, vector<paramList_facs> params);
    vector<double> RV_lpqPatch_facs(Mat img, vector<Mat> filters, Mat V);
    Mat RV_conv2pair_facs(Mat in, Mat k0, Mat k1);
    vector<double> RV_computeHist_2_facs(vector<double> vec, int noBins, double *sum);
    int RV_coutMat_facs(Mat in);

    //Mat RV_lbp(const Mat &src);

    //vector<double> RV_extractLPQ_light(Mat img);

}

#endif


```

## Source file: save/stable/HeadPoseEstimationFramework/Interface/IInterface.h

{
  "path": "save/stable/HeadPoseEstimationFramework/Interface/IInterface.h",
  "blob_sha": "3a3c0ee6d142694dbaf7f298cda2f6bece69dc0f",
  "sha256": "b42ca7796d6dfbaa58d159dfc267a61436446b0a40b17d96df6e9857ce328378",
  "lines": 15,
  "bytes": 186,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 5
  },
  "url": "https://github.com/laurentiuvieriu/facecept3d_facs/blob/99f5e9d9821064e2fb8836658d027fe1ab0d3c1a/save/stable/HeadPoseEstimationFramework/Interface/IInterface.h",
  "function_signature_candidates": []
}

```text

#pragma once

#ifndef IINTERFACE_H
#define IINTERFACE_H

namespace hpe
{
    class IInterface
    {
        public:
            virtual ~IInterface() {}
    };
}

#endif // IINTERFACE_H


```
