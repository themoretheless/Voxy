# Facial rendering source scan: JerryWu96/3D_Face_Reconstrction

Repository: https://github.com/JerryWu96/3D_Face_Reconstrction

Pinned source revision: eedab62a6b0633bda545279c98fbabdd66b2d357

Discovery topic: face model 3D

Description: My personal project that reconstructs a 3D face model from a single image. Used C++, Qt, OpenCV, OpenGL with the help of Surrey Face Model.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["dependencies/expression_blendshapes_3448.bin"], "face": ["program screenshots/face_landmarks.jpeg", "source/FaceDetection.cpp"]}

## Source file: source/FaceDetection.cpp

{
  "path": "source/FaceDetection.cpp",
  "blob_sha": "9d193fc2573d8524b826c532507dad1621d8779b",
  "sha256": "2308d03e5e37ed21f0ba4d5eb521eb8e545c944ec9ef74972e5299561b3bb3aa",
  "lines": 130,
  "bytes": 3988,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 14
  },
  "url": "https://github.com/JerryWu96/3D_Face_Reconstrction/blob/eedab62a6b0633bda545279c98fbabdd66b2d357/source/FaceDetection.cpp",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "void FaceLandmarkDetect(string picDir)"
    }
  ]
}

```text

﻿/**
*  @FaceLandmarkDetect函数
*  利用OpenCV + dlib库实现了：
*  1 68个人脸特征点特征点提取
*  2 特征点格式化输出为.pts
*  pts文件包含了一个68*2的矩阵，对应68个特征点及对应的x/y坐标
*/


#include <dlib/image_processing/frontal_face_detector.h>
#include <dlib/image_processing/render_face_detections.h>
#include <dlib/image_processing.h>
#include <dlib/gui_widgets.h>
#include <dlib/image_io.h>
#include <dlib/opencv.h>  

#include "opencv2/core/core.hpp"
#include "opencv2/highgui/highgui.hpp"
#include "opencv2/imgproc/imgproc.hpp"

#include <iostream>
#include <string>
#include <exception>
#include "Functions.h"

using namespace dlib;
using namespace std;

void FaceLandmarkDetect(string picDir)
{
	string gtmp = "tmp.txt";
	string gpts = "out.pts";

	try
	{


		// 初始化dlib训练的人脸特征检测器frontal_face_detector
		frontal_face_detector detector = get_frontal_face_detector();

		// 利用dlib模型，我们还需要构建特征提取器shape_predictor.它在给定的图片和bounding box中预测人脸位置
		// 这里我采用官方训练好的模型shape_predictor_68_face_landmarks.dat
		// 如果有足够样本，dlib也有相关训练方法，可以训练自己的模型
		shape_predictor sp;
		deserialize("shape_predictor_68_face_landmarks.dat") >> sp;
		//image_window win, win_faces;
		// 创建一个二维矩阵，rgb编码形式存储图片
		array2d<rgb_pixel> img;

		try
		{
			load_image(img, picDir);
		}
		catch (const std::exception& e)
		{
			cout << "Error loading the original image: " << e.what() << endl;
			cout << e.what() << endl;
			cout << "Press any button to exit." << endl;
			cin.get();
			return;
		}

		// 若图片中人脸较小，可以允许使用该函数进行图片采样放大，便于检测
		//pyramid_up(img);

		// 在图片的人脸周边，利用特征检测器，确定bounding box
		std::vector<rectangle> dets = detector(img);

		// 利用特征提取器，获取68个特征点，存放在shapes容器中
		// full_object_detection 亦包含了bounding box rectangle
		std::vector<full_object_detection> shapes;
		std::ofstream tmpfile(gtmp);
		// 当然.利用dets可以针对同一张照片多张人脸进行同时检测，存入shapes.
		// 本系统只检测一张人脸
		full_object_detection shape = sp(img, dets[0]);
		cout << "number of points: " << shape.num_parts() << endl;
		tmpfile << "version: 1" << endl << "n_points:  68" << endl << '{' << endl;
		for (int i = 0; i < shape.num_parts(); i++)
		{
			tmpfile << shape.part(i) << endl;
		}
		tmpfile << '}';
		tmpfile.close();

		// 和ibug数据格式统一
		std::ifstream infile2(gtmp);
		string tmp{ istreambuf_iterator<char>(infile2), istreambuf_iterator<char>() };
		infile2.close();

		strReplace(tmp, "(", "");
		strReplace(tmp, ")", "");
		strReplace(tmp, ",", " ");

		//将特征点坐标保存至.pts文件
		std::ofstream outfile(gpts);
		outfile << tmp;
		outfile.close();
		shapes.push_back(shape);

		/* 独立的人脸特征点模块可允许下列函数进行显示，系统就不需要了
		// 在屏幕上显示描绘了特征点折线的人脸
		// 清空图像窗口
		win.clear_overlay();
		// 显示输入的图片
		win.set_image(img);
		// 在图片检测到的人脸上绘制特征点折线
		win.add_overlay(render_face_detections(shapes));

		// 同时，显示一个裁剪的、旋转至标准化姿势的人脸,进行对比
		dlib::array<array2d<rgb_pixel> > face_chips;
		extract_image_chips(img, get_face_chip_details(shapes), face_chips);
		win_faces.set_image(tile_images(face_chips));

		*/

		//cout << "Hit enter to continue..." << endl;
		//cin.get();

	}
	catch (exception& e)
	{

		cout << "\nexception thrown during face detection module!" << endl;
		cin.get();
		cout << e.what() << endl;
		std::terminate();
	}

}



```

## Source file: include/Model.h

{
  "path": "include/Model.h",
  "blob_sha": "a49b11e6c17dfa3b64b2360c70fac1085a46ab66",
  "sha256": "83ae82298df5ea431184f88a0694c062faa79173990137c1a91c3e4af3296744",
  "lines": 52,
  "bytes": 900,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/JerryWu96/3D_Face_Reconstrction/blob/eedab62a6b0633bda545279c98fbabdd66b2d357/include/Model.h",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "void render();"
    },
    {
      "line": 34,
      "text": "int faces() const { return Faces.size(); }"
    },
    {
      "line": 35,
      "text": "int points() const { return Vertices.size(); }"
    },
    {
      "line": 48,
      "text": "void LoadMTL(QString fn, QString MTLname);"
    },
    {
      "line": 49,
      "text": "void LoadTexture();"
    }
  ]
}

```text

#pragma once
#pragma once
#ifndef MODEL_H
#define MODEL_H

//QT_FORWARD_DECLARE_CLASS(QOpenGLFunctions_1_1)

#include <QVector>
#include <QTextStream>
#include <QVector3D>
#include <QVector2D>
#include <QtOpenGL>

struct Face
{
	//QVector<QVector3D> vn;

	QVector<QVector3D> v;
	QVector<QVector2D> t;
	Face() {
		v.resize(3);
		t.resize(3);
		//vn.resize(3);
	}
};

class Model
{
public:
	Model() {}
	explicit Model(QString filename);
	void render();

	int faces() const { return Faces.size(); }
	int points() const { return Vertices.size(); }

private:
	QString fileName;
	QString textureName;
	//QVector<QVector3D> VNormals; //vn

	QVector<QVector3D> Vertices; //v
	QVector<QVector2D> UVs;      //vt
	QVector<Face>      Faces;    //f

	GLuint texture;
	QImage textureImg;
	void LoadMTL(QString fn, QString MTLname);
	void LoadTexture();
};

#endif

```

## Source file: source/main.cpp

{
  "path": "source/main.cpp",
  "blob_sha": "6d0a882b4d5c1fd782dd3f07200cc2c29d5cac1b",
  "sha256": "ac7e6ad4bd6b6de8d9f02aa8fdc920a0bba816ff6465dd36242a5c20f388242c",
  "lines": 63,
  "bytes": 1395,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/JerryWu96/3D_Face_Reconstrction/blob/eedab62a6b0633bda545279c98fbabdd66b2d357/source/main.cpp",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "int main(int argc, char *argv[])"
    }
  ]
}

```text

﻿/**
*  @main函数
*  进行QT初始化、其他功能的入口
*/
#include <QApplication>
#include <QDesktopWidget>
#include <Qt3DCore/QEntity>
#include <Qt3DCore/QTransform>
#include <Qt3DCore/QAspectEngine>
#include <Qt3DRender/qrenderaspect.h>
#include <Qt3DRender/QCamera>
#include <Qt3DRender/QMaterial>
#include <Qt3DRender/QMesh>
#include <Qt3DExtras/Qt3DWindow>
#include <Qt3DExtras/QTorusMesh>
#include <Qt3DExtras/QOrbitCameraController>
#include <Qt3DExtras/QPhongMaterial>


#include "viewer.h"
#include "window.h"
#include "myglwidget.h"
#include "Model.h"
#include "Functions.h"



int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
	Viewer viewer;
	viewer.show();

	while (!viewer.showFlag())
	{
		delay(1);
		if (viewer.showFlag())
		{
			Reconstruct();
			Window window;
			window.setWindowTitle("obj Model Viewer");
			window.resize(window.sizeHint());
			int desktopArea = QApplication::desktop()->width() * QApplication::desktop()->height();
			int widgetArea = window.width() * window.height();
			if (((float)widgetArea / (float)desktopArea) < 0.75f)
			{
				window.show();
			}
			else
			{
				window.showMaximized();
			}
			viewer.setFlag(0);
			while (!window.showFlag())
			{
				//qDebug() << "showFlag:" << window.showFlag();
				delay(1);
			}
		}
	}
	
    return app.exec();
}


```

## Source file: include/viewer.h

{
  "path": "include/viewer.h",
  "blob_sha": "7116b675fa0713cad8f97358f7157f82a1ebc4e1",
  "sha256": "9a36357517da689a3d1e48c807f049439babcdfb5a6c70464b5282b27a5ee9a7",
  "lines": 46,
  "bytes": 734,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/JerryWu96/3D_Face_Reconstrction/blob/eedab62a6b0633bda545279c98fbabdd66b2d357/include/viewer.h",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "bool showFlag();"
    },
    {
      "line": 31,
      "text": "void setFlag(bool flag);"
    }
  ]
}

```text

// window.h

#ifndef VIEWER_H
#define VIEWER_H

#include <QtWidgets/QMainWindow>
#include "ui_viewer.h"
#include <QPushButton>
#include <QInputDialog>
#include <QFileDialog>
#include <QString>
#include <QFile>
#include <QTextStream>

#include <string>
#include <iostream>
#include <QImage>
#include <QLabel>

namespace Ui {
	class Viewer;
}

class Viewer : public QMainWindow
{
	Q_OBJECT

public:
	Viewer(QWidget *parent = Q_NULLPTR);
	bool showFlag();
	void setFlag(bool flag);
private slots:
	QString handleButton();
	//QString getImageName();

private:
	
	Ui::ViewerClass ui;
	bool flag;
	QPushButton *m_button;
	//QGraphicsScene *scene;
	QPixmap image;
	//QString m_image;
};

#endif


```
