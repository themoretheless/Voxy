# Facial rendering source scan: lab-x/Facial-MoCap

Repository: https://github.com/lab-x/Facial-MoCap

Pinned source revision: d03a286591075fed611977bcc65466503e176af6

Discovery topic: facial mocap

Description: Uses OpenCV 2.4.9

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["Facial-MoCap/Facial-MoCap/FaceModel.cpp", "Facial-MoCap/Facial-MoCap/FaceModel.h", "media/IMM/imm_face_db.pdf"]}

## Source file: Facial-MoCap/Facial-MoCap/FaceModel.h

{
  "path": "Facial-MoCap/Facial-MoCap/FaceModel.h",
  "blob_sha": "cb637437a90792ad5a430349d0681c038e60b2ea",
  "sha256": "fdf8e184ae3fd9d3e48182371883db65c1e7ea6688d8e00b292bc3915d9a6c4e",
  "lines": 83,
  "bytes": 2728,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 4
  },
  "url": "https://github.com/lab-x/Facial-MoCap/blob/d03a286591075fed611977bcc65466503e176af6/Facial-MoCap/Facial-MoCap/FaceModel.h",
  "function_signature_candidates": [
    {
      "line": 65,
      "text": "void cvPrintMat(Mat* Matrix);"
    },
    {
      "line": 67,
      "text": "void Anthropometric3DModel(int LineWidth);"
    },
    {
      "line": 69,
      "text": "void DrawAxes(float Size, float Radius);"
    },
    {
      "line": 74,
      "text": "void Init(void);"
    }
  ]
}

```text

#pragma once
/***************************************************************************
*   Copyright (C) 2007 by pedromartins   *
*   pedromartins@isr.uc.pt   *
*                                                                         *
*   This program is free software; you can redistribute it and/or modify  *
*   it under the terms of the GNU General Public License as published by  *
*   the Free Software Foundation; either version 2 of the License, or     *
*   (at your option) any later version.                                   *
*                                                                         *
*   This program is distributed in the hope that it will be useful,       *
*   but WITHOUT ANY WARRANTY; without even the implied warranty of        *
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
*   GNU General Public License for more details.                          *
*                                                                         *
*   You should have received a copy of the GNU General Public License     *
*   along with this program; if not, write to the                         *
*   Free Software Foundation, Inc.,                                       *
*   59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.             *
***************************************************************************/



//Compile with OpenGL and OpenCV libs
// g++ glAnthropometric3DModel.cpp -I /usr/local/include/opencv/ -L /usr/local/lib -lcv -lhighgui -lcvaux -lX11 -lXi -lXmu -lglut -lGL -lGLU -lm -o glAnthropometric3DModel


#ifdef _MSC_VER
#define _CRT_SECURE_NO_WARNINGS
#endif

#include <GL/glew.h>
#include <GL/gl.h>
#include <GL/glut.h>
#include <stdlib.h>
#include <stdio.h>
#include <iostream>


#include <opencv\cv.h>
#include <opencv\cvaux.h>
#include <opencv2\highgui\highgui.hpp>


#define cvM32f(Matrix,i,j) CV_MAT_ELEM(*(Matrix),float,(i),(j))


//Angle Conversion Macros
#define PI (3.141592653589793)

#define rad2deg(Theta) (Theta)*180/PI
#define deg2rad(Theta) (Theta)*PI/180

using cv::Mat;
using cv::Point3f;
using std::vector;

class FaceModel
{
public:
	FaceModel();
	~FaceModel();

	//Display Matrix
	void cvPrintMat(Mat* Matrix);

	void Anthropometric3DModel(int LineWidth);

	void DrawAxes(float Size, float Radius);

	const vector<Point3f>* getFeatures();

	//Set Texture Mapping Parameters
	void Init(void);

	GLfloat WorldTx = 0, WorldTy = 0, WorldTz = 0, WorldRoll = 0, WorldPitch = 0, WorldYaw = 0;
	GLfloat FaceTx = 0, FaceTy = 0, FaceTz = 0, FaceRoll = 0, FacePitch = 0, FaceYaw = 0;
private:
	static const vector<Point3f> Model3D;
	static const int MeanDelaunayTriangles[95][3];
	GLUquadricObj*Quadric = gluNewQuadric();
};



```

## Source file: Facial-MoCap/Facial-MoCap/AAM.h

{
  "path": "Facial-MoCap/Facial-MoCap/AAM.h",
  "blob_sha": "0882cd5642a0b2dc2946f14e61e5f380c3c545ba",
  "sha256": "2f501f55ac19a4cba8ee9e00dc1767412f6673f6c6bd3abed2e6d48d2fd5d86c",
  "lines": 79,
  "bytes": 2475,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/lab-x/Facial-MoCap/blob/d03a286591075fed611977bcc65466503e176af6/Facial-MoCap/Facial-MoCap/AAM.h",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "void buildAAM(string filePath);"
    },
    {
      "line": 51,
      "text": "void genMeanShapeModel(TImage * img, int cols, int rows);"
    },
    {
      "line": 53,
      "text": "void genMeanAppearanceModel();"
    },
    {
      "line": 56,
      "text": "void loadPCAPoints(const vector<Point2f>* points, Mat& pcaSet, unsigned index);"
    },
    {
      "line": 59,
      "text": "void loadPCAPixels(TImage* img, Mat& pcaSet, unsigned index);"
    },
    {
      "line": 62,
      "text": "int findPoint(float x, float y, vector<Point2f>* tImg);"
    }
  ]
}

```text

/*
By: Paul Jacob Logas
Licensed to the Apache Software Foundation (ASF) under one
or more contributor license agreements.  See the NOTICE file
distributed with this work for additional information
regarding copyright ownership.  The ASF licenses this file
to you under the Apache License, Version 2.0 (the
"License"); you may not use this file except in compliance
with the License.  You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing,
software distributed under the License is distributed on an
"AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
KIND, either express or implied.  See the License for the
specific language governing permissions and limitations
under the License.
*/

#pragma once
#include <opencv2/imgproc/imgproc.hpp>
#include <opencv2/highgui/highgui.hpp>
#include "TImage.h"
#include "Delaunay.h"
#include <string>

using cv::Mat;
using cv::Point2f;
using cv::Vec3i;
using cv::PCA;
using std::vector;
using std::string;

class AAM
{
public:
	AAM();
	~AAM();

	//Creates the statistical model on which feature recognition and tracking are built
	// Input path is the directory to all annotation files as well as a TrainingSet.txt file
	// describing the files one wishes to use in the model.
	void buildAAM(string filePath);

	//Finds the warp matrix of the input
	Mat warpToMean(TImage* img);

private:
	//Generates the mean model from the mean values of the PCA
	void genMeanShapeModel(TImage * img, int cols, int rows);

	void genMeanAppearanceModel();

	//Loads the point data into the PCA
	void loadPCAPoints(const vector<Point2f>* points, Mat& pcaSet, unsigned index);

	//Loads the warped pixel data into the PCA set matrix
	void loadPCAPixels(TImage* img, Mat& pcaSet, unsigned index);

	//Finds the point index number of the provided floats
	int findPoint(float x, float y, vector<Point2f>* tImg);

	//This vector holds the order of the triangle vertices and their point indeces for simple warping of triangles.
	vector<Vec3i> meanModel;
	vector<Point2f>* meanPoints;

	Mat meanAppearance;

	/*
	These variables will be used as an offset for the matrix to avoid excess space usage.
	The idea is to use the iteration during generateMeanModel to find the relevent maxes and mins of these values
	then with this information the meanPoints are normalized to this new, smaller frame.
	*/
	int minX, minY, maxX, maxY;

	PCA shapeModel;
	PCA appearModel;
};

```

## Source file: Facial-MoCap/Facial-MoCap/AAM.cpp

{
  "path": "Facial-MoCap/Facial-MoCap/AAM.cpp",
  "blob_sha": "06b82579f6e1a54c8f31e5309d563b73e4d964d4",
  "sha256": "8bcfbae46d2bf9c55606999e624f0990c6fd7401f55746068a147c020e221dd3",
  "lines": 197,
  "bytes": 6218,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2
  },
  "url": "https://github.com/lab-x/Facial-MoCap/blob/d03a286591075fed611977bcc65466503e176af6/Facial-MoCap/Facial-MoCap/AAM.cpp",
  "function_signature_candidates": []
}

```text

#include "AAM.h"

AAM::AAM()
{
	meanModel = vector<Vec3i>();
	minX = INT_MAX;
	minY = INT_MAX;
	maxY = 0;
	maxX = 0;
}

AAM::~AAM()
{
	delete meanPoints;
}

void AAM::buildAAM(string filePath)
{
	std::cout << "Loading Training Set";
	vector<TImage*>* imgs = TImage::loadAllTraining(filePath);
	std::cout << "...Done" << std::endl;
	
	int cols = imgs->at(0)->getCols();
	int rows = imgs->at(0)->getRows();
	
	{//Loads the annotation points into a PCA set and finds the mean shape model
		std::cout << "Add to PCA Set\n";
		//Creates a matrix that has rows = #training images and cols = #features tracked
		Mat pcaSet = Mat::eye(imgs->size(), imgs->at(0)->getPoints()->size() * 2, CV_64F);
		//Iterates through all training images warping and adding the feature points to the PCA set
		for (unsigned i = 0; i < imgs->size(); i++)
			loadPCAPoints(imgs->at(i)->getPoints(), pcaSet, i);
		std::cout << "...Done" << std::endl;
		std::cout << "Create PCA Object";
		shapeModel = PCA(pcaSet,				//pass the point data
			Mat(),					//no pre-computed mean vector, let PCA engine compute
			CV_PCA_DATA_AS_ROW,		//Data is stored as a row as the vectors span across columns
			pcaSet.cols				//How many principal components to retain
			);
		std::cout << "...Done" << std::endl;
		std::cout << "Generating Mean Shape Model";
		genMeanShapeModel(imgs->at(0), cols, rows);
		std::cout << "...Done" << std::endl;
	}

	{//Loads the relevant image points into the PCA set and finds the mean appearance
		std::cout << "Warp and add to PCA Set";
		//Creates a matrix that has rows = #training images and cols = #relevant pixels
		int pcaCols = (maxX - minX) * (maxY - minY);
		Mat pcaSet = Mat::eye(imgs->size(), pcaCols, CV_64F);
		for (unsigned i = 0; i < imgs->size(); i++)
			loadPCAPixels(imgs->at(i), pcaSet, i);
		std::cout << "...Done" << std::endl;
		std::cout << "Create PCA Object";
		appearModel = PCA(pcaSet,
			Mat(),
			CV_PCA_DATA_AS_ROW,
			10
			);
		std::cout << "...Done" << std::endl;
		std::cout << "Generating Mean Appearance Model";
		genMeanAppearanceModel();
		std::cout << "...Done" << std::endl;
	}
	TImage::deleteAllTraining(imgs);
}

int AAM::findPoint(float x, float y, vector<Point2f>* points)
{
	if (x > maxX)
		maxX = x+15;
	else if (x < minX)
		minX = x-30;
	if (y > maxY)
		maxY = y + 15;
	else if (y < minY)
		minY = y - 15;


	for (unsigned i = 0; i < points->size(); i++)
		if (points->at(i).x == x && points->at(i).y == y)
			return i;


	return INT_MAX;
}

Mat AAM::warpToMean(TImage * img)
{
	Mat *matImg = img->getImg();
	int rows = img->getRows();
	int cols = img->getCols();
	vector<Point2f>* point = img->getPoints();
	Mat warp_mat(2, 3, CV_32FC1);
	Mat warp_dst, warp_mask;
	Mat warp_final = Mat();
	for (unsigned i = 0; i < meanModel.size(); i++)
	{
		warp_dst = Mat::zeros(rows, cols, matImg->type());
		warp_mask = Mat::zeros(rows, cols, matImg->type());

		Point2f src[3] = { point->at(meanModel[i](0)), point->at(meanModel[i](1)), point->at(meanModel[i](2)) };
		Point2f dest[3] = { meanPoints->at(meanModel[i](0)), meanPoints->at(meanModel[i](1)), meanPoints->at(meanModel[i](2)) };

		//Affine Transform
		warp_mat = cv::getAffineTransform(src, dest);
		// Apply Transform to src
		cv::warpAffine(*matImg, warp_dst, warp_mat, warp_dst.size());
		cv::Point precious[3] = { dest[0], dest[1], dest[2] };
		cv::fillConvexPoly(warp_mask, precious, 3, CV_RGB(255, 255, 255), CV_AA, 0);
		warp_dst.copyTo(warp_final, warp_mask);
#if defined(_DEBUG)
		cv::imshow("Final", warp_final);
	}
	cv::waitKey(10);
#else
	}
#endif
	return warp_final;
}

void AAM::loadPCAPoints(const vector<Point2f>* points, Mat & pcaSet, unsigned index)
{
	int j = 0;
	//Iterates through the points passed in and adds them to the pcaSet in order
	for (unsigned i = 0; i < points->size(); i++)
	{
		pcaSet.at<double>(index, j++) = points->at(i).x;
		pcaSet.at<double>(index, j++) = points->at(i).y;
	}
}

void AAM::loadPCAPixels(TImage * img, Mat & pcaSet, unsigned index)
{
	Mat warped = warpToMean(img);
	int k = 0;
	cv::cvtColor(warped, meanAppearance, cv::COLOR_BGR2GRAY);
	for (unsigned i = minY; i < maxY; i++)
		for (unsigned j = minX; j < maxX; j++)
			if(meanAppearance.at<uchar>(j,i) != 0)
				pcaSet.at<double>(index, k++) = meanAppearance.at<uchar>(j, i);
}

void AAM::genMeanShapeModel(TImage* img, int cols, int rows)
{
	//Convenient use of a global variable
	meanPoints = img->getPoints();
	Subdiv2D subdiv = Delaunay::findSubdiv(img);
	vector<Vec6f> triangles;
	subdiv.getTriangleList(triangles);
	//Initializes the triangle path that all images will follow
	for (unsigned i = 0; i < triangles.size(); i++)
	{
		if (cols > triangles[i][0] && rows > triangles[i][1] &&
			cols > triangles[i][2] && rows > triangles[i][3] &&
			cols > triangles[i][4] && rows > triangles[i][5] &&
			0 <= triangles[i][0] && 0 <= triangles[i][1] &&
			0 <= triangles[i][2] && 0 <= triangles[i][3] &&
			0 <= triangles[i][4] && 0 <= triangles[i][5])
		{
		Vec3i vec = Vec3i();
		vec[0] = findPoint(triangles[i][0], triangles[i][1], meanPoints);
		vec[1] = findPoint(triangles[i][2], triangles[i][3], meanPoints);
		vec[2] = findPoint(triangles[i][4], triangles[i][5], meanPoints);
		meanModel.push_back(vec);
#if defined(_DEBUG)
		std::cout << std::endl << vec;
#endif
		}
	}
	//This portion of the function initializes the meanPoints property with the mean as
	// calculated by the PCA
	meanPoints = new vector<Point2f>(meanPoints->size());
	int j = 0;
	for (size_t i = 0; i < meanPoints->size(); i++)
	{
		meanPoints->at(i).x = shapeModel.mean.at<double>(0, j++);
		meanPoints->at(i).y = shapeModel.mean.at<double>(0, j++);
	}
}

void AAM::genMeanAppearanceModel()
{
	int k = 0;
	for (unsigned i = minY; i < maxY; i++)
		for (unsigned j = minX; j < maxX; j++)
		{
			if (meanAppearance.at<uchar>(j, i) != 0)
				meanAppearance.at<uchar>(j, i) = appearModel.mean.at<double>(0, k++);
		}
#ifdef _DEBUG
	cv::imshow("Mean Appearance", meanAppearance);
	//cv::imshow("pc1", appearModel.eigenvectors.row(0).reshape(1, meanAppearance.rows));
	//cv::imshow("pc2", appearModel.eigenvectors.row(1).reshape(1, meanAppearance.rows));
	//cv::imshow("pc3", appearModel.eigenvectors.row(2).reshape(1, meanAppearance.rows));
#endif
}


```

## Source file: Facial-MoCap/Facial-MoCap/TImage.h

{
  "path": "Facial-MoCap/Facial-MoCap/TImage.h",
  "blob_sha": "469d67ca04b26ea6e2385d79cbb1790f9c90ca1b",
  "sha256": "2bb03461c0acc3c06ba249c4d9eaa981ee9285b555b2829c8ded60fd0d17e568",
  "lines": 73,
  "bytes": 1834,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/lab-x/Facial-MoCap/blob/d03a286591075fed611977bcc65466503e176af6/Facial-MoCap/Facial-MoCap/TImage.h",
  "function_signature_candidates": [
    {
      "line": 52,
      "text": "static void deleteAllTraining(vector<TImage*>* imgs);"
    },
    {
      "line": 58,
      "text": "int getRows();"
    },
    {
      "line": 59,
      "text": "int getCols();"
    },
    {
      "line": 63,
      "text": "bool loadImg(string* filePath);"
    },
    {
      "line": 66,
      "text": "bool loadFeaturePoints(string* filePath);"
    }
  ]
}

```text

/*
By: Paul Jacob Logas
Licensed to the Apache Software Foundation (ASF) under one
or more contributor license agreements.  See the NOTICE file
distributed with this work for additional information
regarding copyright ownership.  The ASF licenses this file
to you under the Apache License, Version 2.0 (the
"License"); you may not use this file except in compliance
with the License.  You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing,
software distributed under the License is distributed on an
"AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
KIND, either express or implied.  See the License for the
specific language governing permissions and limitations
under the License.
*/

#pragma once
#include <opencv2/imgproc/imgproc.hpp>
#include <opencv2/highgui/highgui.hpp>
#include <iostream>
#include <fstream>
#include <sstream>
#include <string>

using cv::Mat;
using cv::Point2f;
using std::vector;
using std::string;

class TImage
{
public:
	TImage();
	TImage(string filePath);
	~TImage();

	/*
	Loads all training images found in the provided directory, following the format
	laid out in TrainingSet.txt file.
	Input format is: ../path/to/
	*/
	static vector<TImage*>* loadAllTraining(string filePath);

	/*
	Be sure to call this function whenever you expect the generated vector
	to go out of scope as it is a large memory object.
	*/
	static void deleteAllTraining(vector<TImage*>* imgs);

	Mat* const& getImg();

	vector<Point2f>* const& getPoints();

	int getRows();
	int getCols();

private:
	//Image loading function
	bool loadImg(string* filePath);

	//Annotation loading
	bool loadFeaturePoints(string* filePath);

	//Matrix that holds image data
	Mat img;

	//Vector of triangles in the image
	vector<Point2f> points;
};

```
