# Facial rendering source scan: HRLTY/DenseFace3D

Repository: https://github.com/HRLTY/DenseFace3D

Pinned source revision: a01e59bc6f9bc19fc0c7c4a4576cd2b536c93f38

Discovery topic: face reconstruction 3D

Description: a software for the reconstruction of a real-time 3D face model from a monocular camera.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["DenseFace3D.cpp"]}

## Source file: DenseFace3D.cpp

{
  "path": "DenseFace3D.cpp",
  "blob_sha": "14667969b18ebb78acc5adad6eaec91e5837e8ca",
  "sha256": "f63a6deb1d7cac203cff4409e4713e402a49f38ecc2ad7558f90e777fa4e5ace",
  "lines": 902,
  "bytes": 27789,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 6,
    "face": 37,
    "lip": 1
  },
  "url": "https://github.com/HRLTY/DenseFace3D/blob/a01e59bc6f9bc19fc0c7c4a4576cd2b536c93f38/DenseFace3D.cpp",
  "function_signature_candidates": [
    {
      "line": 56,
      "text": "void loadNext();"
    },
    {
      "line": 57,
      "text": "//void loadWithPoints(Mat& ip, Mat& img);"
    },
    {
      "line": 62,
      "text": "void drawFace();"
    },
    {
      "line": 63,
      "text": "void drawFaceWithTex();"
    },
    {
      "line": 64,
      "text": "void drawPrimitiveFace();"
    },
    {
      "line": 65,
      "text": "void loadTPSwithControlPoints();"
    },
    {
      "line": 128,
      "text": "void display(void)"
    },
    {
      "line": 181,
      "text": "void drawFace()"
    },
    {
      "line": 258,
      "text": "void drawFaceWithTex()"
    },
    {
      "line": 353,
      "text": "void drawPrimitiveFace()"
    },
    {
      "line": 399,
      "text": "void loadTPSwithControlPoints()"
    },
    {
      "line": 415,
      "text": "void updateLandmarkCoor(std::vector<point>& newPoints) //calculate the averate coordinate"
    },
    {
      "line": 455,
      "text": "void calNewSampleAndInsertForTri()"
    },
    {
      "line": 479,
      "text": "void loadNext() {"
    },
    {
      "line": 548,
      "text": "//void loadWithPoints(Mat& ip, Mat& img) {"
    },
    {
      "line": 587,
      "text": "void init_webcam(string file)"
    },
    {
      "line": 606,
      "text": "void move(int value)"
    },
    {
      "line": 623,
      "text": "void saveOpenGLBuffer() {"
    },
    {
      "line": 640,
      "text": "void key(unsigned char key, int x, int y)"
    },
    {
      "line": 697,
      "text": "void idle(void)"
    },
    {
      "line": 702,
      "text": "void myGLinit() {"
    },
    {
      "line": 740,
      "text": "void resize(int width, int height)"
    },
    {
      "line": 754,
      "text": "void readcsv()"
    },
    {
      "line": 802,
      "text": "int start_opengl() {"
    },
    {
      "line": 808,
      "text": "void init_opengl(int argc,char** argv) {"
    },
    {
      "line": 824,
      "text": "int main(int argc, char** argv)"
    },
    {
      "line": 862,
      "text": "void drawAxes() {"
    }
  ]
}

```text

/*
 Copyright (C) 2016 by Rui Huang
 huangrui@buaa.edu.cn
 
 This file is part of DenseFace3D.

DenseFace3D is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

DenseFace3D is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with DenseFace3D.  If not, see <http://www.gnu.org/licenses/>.
*/

#include "stdafx.h"
#include <vector>
#include <iostream>
#include <fstream>

#include "cv.h"
#include "highgui.h"

#include <dlib/opencv.h>
#include <opencv2/highgui/highgui.hpp>
#include <dlib/image_processing/frontal_face_detector.h>
#include <dlib/image_processing/render_face_detections.h>
#include <dlib/image_processing.h>
#include <dlib/gui_widgets.h>

#if defined(__APPLE__)
#  include <OpenGL/gl.h>
#  include <OpenGL/glu.h>
#elif defined(__linux__) || defined(__MINGW32__) || defined(WIN32)

#  include <GL/gl.h>
#  include <GL/glu.h>
#else
#  include <gl.h>
#endif

#include "glm.h"
#include "OGL_OCV_common.h"
#include "samplePoint.h"
#include "ThinPlateSpline.h"

using namespace std;
using namespace dlib;
using namespace cv;

void loadNext();
//void loadWithPoints(Mat& ip, Mat& img);


struct triangleIndex{int index[3]; };
std::vector<std::string> getNextLineAndSplitIntoTokens(std::istream& str);
void drawFace();
void drawFaceWithTex();
void drawPrimitiveFace();
void loadTPSwithControlPoints();

std::vector<triangleIndex> triangles;//fixed triangulation for primitive face.
std::vector<Point3d> landmark_points; //fixed landmark (now only used for depth values)
//full_object_detection realtime_landmark_detection; //the real time landmarks
std::vector<std::vector<point> > history_landmarks; //used for calculating the average

std::vector<long> x_coor, y_coor; //current active average landmark coordinates
long x_mean = 0, y_mean = 0;

ThinPlateSpline* tps; //interpolation and sampling
Work sampler;


static const int REVERSE = 50;//auto rotation boundary
double rotateY = 0, dir = 5; //rotation speed
bool AUTOROTATE = true;

double ZSCALE = 2, XSCALE, YSCALE; //scaling factors

double color1, color2, color3; //for randomrized coloring
bool CHANGECOLOR = true;

long XTRANS = 315, YTRANS = 150;
bool READY = false; //remain false until the first face is detected

//for triangulation
cv::Subdiv2D delaunay;
cv::Rect rect;
std::vector<Vec6f> triangleList;

//dlib detection and image
frontal_face_detector detector;
shape_predictor pose_model;
VideoCapture cap;
std::vector <Point2f> img_last;

//current texture
GLuint textureID;
OpenCVGLTexture imgTex,imgWithDrawing;


const GLfloat light_ambient[]  = { 0.0f, 0.0f, 0.0f, 1.0f };
const GLfloat light_diffuse[]  = { 1.0f, 1.0f, 1.0f, 1.0f };
const GLfloat light_specular[] = { 1.0f, 1.0f, 1.0f, 1.0f };
const GLfloat light_position[] = { 0.0f, 0.0f, 1.0f, 0.0f };

const GLfloat mat_ambient[]    = { 0.7f, 0.7f, 0.7f, 1.0f };
const GLfloat mat_diffuse[]    = { 0.8f, 0.8f, 0.8f, 1.0f };
const GLfloat mat_specular[]   = { 1.0f, 1.0f, 1.0f, 1.0f };
const GLfloat high_shininess[] = { 100.0f };

//double rot[9] = {0};


//Mat backPxls;
//std::vector<double> rv(3), tv(3);
//Mat rvec(rv),tvec(tv);
//Mat camMatrix;


//GLMmodel* head_obj;

void display(void)
{
    // draw the image in the back
    int vPort[4]; glGetIntegerv(GL_VIEWPORT, vPort);
    glEnable2D();
    //drawOpenCVImageInGL(imgTex);
    //glTranslated(vPort[2]/2.0, 0, 0);
    drawOpenCVImageInGL(imgTex);
    glDisable2D();
    
    glClear(GL_DEPTH_BUFFER_BIT); // we want to draw stuff over the image
    
    // draw only on left part
    //glViewport(0, 0, vPort[2]/2, vPort[3]);
    //printf("viewport : (0,0,%d,%d)\n",vPort[2], vPort[3]);
    
    glPushMatrix();
    //must be called before drawFaceTex since this one loads control points
    drawFace();
    
    // rotate it
    // double _d[16] = {	rot[0],rot[1],rot[2],0,
    // 					rot[3],rot[4],rot[5],0,
    // 					rot[6],rot[7],rot[8],0,
    // 					0,	   0,	  0		,1};
    // glMultMatrixd(_d);
    
    // draw the 3D head model
    // glColor4f(1, 1, 1,0.75);
    //glmDraw(head_obj, GLM_SMOOTH);
    
    //----------Axes
    //    glScaled(50, 50, 50);
    //    drawAxes();
    //----------End axes
    glPopMatrix();
    
    glPushMatrix();
    drawPrimitiveFace();
    glPopMatrix();
    
    glPushMatrix();
    drawFaceWithTex();
    glPopMatrix();
    
    // restore to looking at complete viewport
    glViewport(0, 0, vPort[2], vPort[3]);
    
    glutSwapBuffers();
}


//densified face
void drawFace()
{
    gluLookAt(300,130,100,
              300,130,0,
              0,-1,0);
    // put the object in the right position in space
    //Vec3d tvv(tv[0],tv[1],tv[2]);
    //Vec3d tvv(0,0,0);
    //glTranslated(tvv[0], tvv[1], tvv[2]);
    
    //face location
    glTranslated(-230 + XTRANS,	80 + YTRANS, 0);
    glRotated(rotateY, 0, 1, 0);
    //glColor3ub(0, 255, 0);
    
    glPolygonMode(GL_FRONT_AND_BACK, GL_LINE);
    glBegin(GL_TRIANGLES);
    delaunay.getTriangleList(triangleList);
    //int count = 0;
    if (READY) {
        //old way
        //        for (auto i = triangles.begin(); i != triangles.end();i++){
        //            count ++;
        //            for (size_t j = 0; j < 3; j++) {
        //                int index = i->index[j];
        //                Point3d p = landmark_points[index-1];
        //                point pp = realtime_landmark_detection.part(index-1);
        //                double x_pos = x_coor[index-1];
        //                double y_pos = y_coor[index-1];
        //                //glVertex3d(pp.x(), pp.y(), p.z * ZSCALE);
        //                glVertex3d(x_pos - x_mean, y_pos - y_mean, p.z * ZSCALE);
        //                //	printf("triangle %d point %lu: (%f,%f,%f)\n", count, j, p.x, p.y,p.z);
        //            }
        //        }
        //new  way
        
        loadTPSwithControlPoints();
        //autoscale to a fix scale
        XSCALE = 160 /(double) rect.width;
        YSCALE = 160 /(double) rect.height;
        
        double x_pos, y_pos, z_pos;
        
        std::vector<Point2d> pt(3);
        
        if(CHANGECOLOR){
            color1 = std::rand() / (double) RAND_MAX;
            color2 = std::rand() / (double) RAND_MAX;
            color3 = std::rand() / (double) RAND_MAX;
            CHANGECOLOR = false;
        }
        glLineWidth(2.0);
        glColor3f(color1, color2, color3);
        
        for (auto i = triangleList.begin(); i != triangleList.end();i++){
            //count ++;
            Vec6f t = *i;
            pt[0] = Point2d(t[0], t[1]);
            pt[1] = Point2d(t[2], t[3]);
            pt[2] = Point2d(t[4], t[5]);
            if ( rect.contains(pt[0]) && rect.contains(pt[1]) && rect.contains(pt[2])) {
                for (size_t j = 0; j < 3; j++) {
                    //int index = i->index[j];
                    x_pos = pt[j].x; y_pos = pt[j].y;
                    x_pos -= x_mean; y_pos -= y_mean;
                    //glVertex3d(pp.x(), pp.y(), p.z * ZSCALE);
                    z_pos = tps->calc_height(x_pos, y_pos);
                    x_pos *= XSCALE * .8; y_pos *= YSCALE * .8;
                    glVertex3d(x_pos, y_pos, z_pos);
                    //	printf("triangle %d point %lu: (%f,%f,%f)\n", count, j, p.x, p.y,p.z);
                }
            }
        }
    }
    glEnd();
}

void drawFaceWithTex()
{
    glDisable(GL_LIGHTING);
    glDisable(GL_BLEND);
    gluLookAt(300,130,100,
              300,130,0,
              0,-1,0);
    // put the object in the right position in space
    //Vec3d tvv(tv[0],tv[1],tv[2]);
    //Vec3d tvv(0,0,0);
    //glTranslated(tvv[0], tvv[1], tvv[2]);
    
    //face location
    glTranslated(-180 + XTRANS,	-130 + YTRANS, 0);
    //drawAxes();
    glRotated(rotateY, 0, 1, 0);
    //glColor3f(0, 1, 0);
    glPolygonMode(GL_FRONT_AND_BACK, GL_FILL);
    
    glEnable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, imgTex.tex_id);
    double aw2h = imgTex.aspect_w2h, ithr = imgTex.thr, itwr = imgTex.twr;
    
    //int count = 0;
    if (READY) {
        glColor3ub(255, 255, 255);
        glBegin(GL_TRIANGLES);
        //old way
        //        for (auto i = triangles.begin(); i != triangles.end();i++){
        //            count ++;
        //            for (size_t j = 0; j < 3; j++) {
        //                int index = i->index[j];
        //                Point3d p = landmark_points[index-1];
        //                double w = p.x / imgTex.image.cols * itwr;
        //                double h = (1- p.y / imgTex.image.rows) * ithr;
        //                //glTexCoord2d(w , h);
        //                //printf("part %d: srow %f(w%d), sclo %f.\n", index, p.x, p.y);
        //                point pp = realtime_landmark_detection.part(index-1);
        //                double x_pos = x_coor[index-1];
        //                double y_pos = y_coor[index-1];
        //                //double w_r = (double)pp.x() / imgTex.image.cols * itwr;
        //                double w_r = x_pos / imgTex.image.cols * itwr;
        //                //printf("part %d: row %d, clo %d.\n", index, pp.x(), pp.y());
        //                //printf("image row %d, col %d\n", imgTex.image.rows,imgTex.image.cols);
        //                //double h_r = (1- (double)pp.y() / imgTex.image.rows) * ithr;
        //                double h_r = (1- y_pos / imgTex.image.rows) * ithr;
        //                glTexCoord2d(w_r , h_r);
        //                //glVertex3d(p.x, p.y, p.z * ZSCALE);
        //                //glVertex3d(pp.x(), pp.y(), p.z * ZSCALE);
        //                glVertex3d(x_pos - x_mean, y_pos - y_mean, p.z * ZSCALE);
        //                //	printf("triangle %d point %lu: (%f,%f,%f)\n", count, j, p.x, p.y,p.z);
        //            }
        //        }
        
        //occasional printing scale params.
        //        static int PRINT = 0;
        //        if(PRINT == 0){
        //            std::cout<<"face width:"<<rect.width<<", height:"<<rect.height<<'\n';
        //            std::cout<<"XSCALE:"<<XSCALE<<", YSCALE:"<<YSCALE<<'\n';
        //            PRINT = 50;
        //        }else PRINT--;
        
        std::vector<Point2d> pt(3);
        double x_pos, y_pos, z_pos, w_r, h_r;
        
        for (auto i = triangleList.begin(); i != triangleList.end();i++){
            //count ++;
            Vec6f t = *i;
            pt[0] = Point2d(t[0], t[1]);
            pt[1] = Point2d(t[2], t[3]);
            pt[2] = Point2d(t[4], t[5]);
            if ( rect.contains(pt[0]) && rect.contains(pt[1]) && rect.contains(pt[2])) {
                for (size_t j = 0; j < 3; j++) {
                    //int index = i->index[j];
                    x_pos = pt[j].x; y_pos = pt[j].y;
                    w_r = x_pos / imgTex.image.cols * itwr;
                    h_r = (1- y_pos / imgTex.image.rows) * ithr;
                    
                    x_pos -= x_mean; y_pos -= y_mean;
                    //glVertex3d(pp.x(), pp.y(), p.z * ZSCALE);
                    z_pos  = tps->calc_height(x_pos, y_pos);
                    
                    glTexCoord2d(w_r , h_r);
                    x_pos *= XSCALE; y_pos *= YSCALE;
                    glVertex3d(x_pos, y_pos , z_pos);
                    //printf("tripoint: (%f,%f,%f)\n",x_pos , y_pos, z_pos);
                }
            }
        }
        glEnd();
        glDisable(GL_TEXTURE_2D);
    }
    glEnable(GL_LIGHTING);
    glEnable(GL_BLEND);
}
void drawPrimitiveFace()
{
    gluLookAt(300,130,100,
              300,130,0,
              0,-1,0);
    // put the object in the right position in space
    //Vec3d tvv(tv[0],tv[1],tv[2]);
    //Vec3d tvv(0,0,0);
    //glTranslated(tvv[0], tvv[1], tvv[2]);
    
    //face location
    glTranslated(-140 + XTRANS,	80 + YTRANS, 0);
    glRotated(rotateY, 0, 1, 0);
    //glColor3ub(0, 255, 0);
    glLineWidth(2.0);
    //glColor3ub(255, 255, 255);
    glPolygonMode(GL_FRONT_AND_BACK, GL_LINE);
    
    glBegin(GL_TRIANGLES);
    if (READY) {
        //old way
        glColor3f(1-color1, 1-color2, 1-color3);
        double x_pos, y_pos, z_pos;
        
        //use fixed triangulation
        for (auto i = triangles.begin(); i != triangles.end();i++){
            for (size_t j = 0; j < 3; j++) {
                int index = i->index[j];
                Point3d p = landmark_points[index-1];
                //point pp = realtime_landmark_detection.part(index-1);
                
                x_pos = x_coor[index-1]; y_pos = y_coor[index-1];
                x_pos -= x_mean; y_pos -= y_mean;
                //glVertex3d(pp.x(), pp.y(), p.z * ZSCALE);
                x_pos *= XSCALE * .8; y_pos *= YSCALE * .8;
                z_pos = p.z * ZSCALE;
                glVertex3d(x_pos, y_pos, z_pos);
                //	printf("triangle %d point %lu: (%f,%f,%f)\n", count, j, p.x, p.y,p.z);
            }
        }
    }
    glEnd();
}

//loading real time spline control points
// SID_EEFFECT: SUBTRACT MEAN FROM REAL TIME LOCATION.
void loadTPSwithControlPoints()
{
    std::vector<long> z_coor;
    for (auto i = landmark_points.begin(); i != landmark_points.end(); i++) {
        z_coor.push_back(i->z * ZSCALE);
    }
    std::vector<long> temx = x_coor, temy = y_coor;
    for (long& j : temx)
        j -= x_mean;
    for (long& j : temy)
        j -= y_mean;
    
    tps->insert(temx, z_coor, temy);
    tps->calc_tps();
}

void updateLandmarkCoor(std::vector<point>& newPoints) //calculate the averate coordinate
{
    
    if(history_landmarks.size() >= 3)//add new
        history_landmarks.erase(history_landmarks.begin());
    history_landmarks.push_back(newPoints);
    
    int num = (history_landmarks.end()-1)->size();
    x_coor.clear(); y_coor.clear();
    x_coor.insert(x_coor.begin(),num, 0);
    y_coor.insert(y_coor.begin(),num, 0);
    
    for(auto ith : history_landmarks){
        for (int i = 0; i < num; i++){
            x_coor[i] += ith[i].x();
            y_coor[i] += ith[i].y();
        }
    }
    
    int round = history_landmarks.size();
    for (int i = 0; i < num; i++){
        x_coor[i] /= round;
        y_coor[i] /= round;
    }
    
    for (long& j : x_coor)
        x_mean += j;
    for (long& j : y_coor)
        y_mean += j;
    
    x_mean /= x_coor.size();
    y_mean /= y_coor.size();
    
    //    for (long& j : x_coor)
    //        j -= x_mean;
    //    for (long& j : y_coor)
    //        j -= y_mean;
    //
}

void calNewSampleAndInsertForTri()
{
    std::vector<dpoint> tem;
    for(int i = 0; i < 27; i++){
        tem.push_back(dpoint(x_coor[i], y_coor[i]));
    }
    
    sampler.procsee(tem, 30, 30, 27);
    
    //printing for debugging
    //    int count = 0;
    //    std::fstream out("samples.txt", fstream::in | fstream::out | fstream::trunc );
    //    for (auto t : sampler.RPoints){
    //        out<<t.x()<<','<<t.y()<<'\n';
    //        //std::cout<<"point "<<++count<< ":("<< t.x() <<","<< t.y() <<")\n";
    //    }
    //    out.flush();
    //    out.close();
    //std::vector<dpoint> collector;
    
}

//Mat op;

void loadNext() {
    //static int counter = 1;
    std::vector<Point3f> imagePoints;
    Mat img,frame;
    if(!cap.read(frame)){
        printf("NO MORE VIDEO FRAME!\n");
        return;
    }
    //cv::resize(frame,img,cv::Size(0.4*frame.cols,0.4*frame.rows));
    
    //resize for speeding up the detection
    double limit = 500;
    if(frame.cols > limit)
        cv::resize(frame,img,cv::Size(limit, limit / frame.cols * frame.rows));
    //printf("load frame %d\n",counter);
    cv_image<bgr_pixel> cimg(img);
    
    // Detect faces
    std::vector<dlib::rectangle> faces = detector(cimg);
    std::vector<full_object_detection> shapes;
    for (unsigned long i = 0; i < faces.size(); ++i)
        shapes.push_back(pose_model(cimg, faces[i]));
    
    //for (size_t i = 0; i < shapes.size(); i++) {
    
    //only use the first detected face
    if (shapes.size() >= 1) {
        if(!READY){
            READY = true;
            printf("OK face here!\n");
            printf("Usage:\t q -- flat;\t w -- steep;\n");
            printf("\t s -- start/stop auto rotation;\n");
            printf("\t a -- rotate left;\t d -- rotate right;\n");
            printf("\t e -- speed up;\t r -- slow down\n");
            printf("\t SPACE -- capture and save;\n");
        }
        //const full_object_detection& d = shapes[0];
        
        //import real time detection into x_coor, y_coor
        full_object_detection& realtime_landmark_detection = shapes[0];
        int num_landmark = realtime_landmark_detection.num_parts();
        std::vector<point> newPoints;
        for (int i = 0; i < num_landmark; i++) {//read new landmarks
            point p = realtime_landmark_detection.part(i);
            newPoints.push_back(p);
        }
        updateLandmarkCoor(newPoints);
        calNewSampleAndInsertForTri();
    }
    
    
    //Mat ip(imagePoints);
    //cout<<ip;
    //sprintf(buf,"%sAngelina_Jolie/Angelina_Jolie_%04d.jpg",workingDir,counter);
    
    // Mat img = imread(buf);
    
    imgTex.set(img); //TODO: what if different size??
    
    // paint 2D feature points
    // for(unsigned int i=0;i<imagePoints.size();i++) circle(img,imagePoints[i],2,Scalar(255,0,255),CV_FILLED);
    //
    // loadWithPoints(ip,img);
    
    //imgWithDrawing.set(img);
    
    //counter = (counter+1);
}

//void loadWithPoints(Mat& ip, Mat& img) {
//    int max_d = MAX(img.rows,img.cols);
//    camMatrix = (Mat_<double>(3,3) << max_d, 0, img.cols/2.0,
//                 0,	max_d, img.rows/2.0,
//                 0,	0,	1.0);
//    cout << "using cam matrix " << endl << camMatrix << endl;
//
//    double _dc[] = {0,0,0,0};
//    solvePnP(op,ip,camMatrix,Mat(1,4,CV_64FC1,_dc),rvec,tvec,false,CV_EPNP);
//
//    Mat rotM(3,3,CV_64FC1,rot);
//    Rodrigues(rvec,rotM);
//    double* _r = rotM.ptr<double>();
//    printf("rotation mat: \n %.3f %.3f %.3f\n%.3f %.3f %.3f\n%.3f %.3f %.3f\n",
//           _r[0],_r[1],_r[2],_r[3],_r[4],_r[5],_r[6],_r[7],_r[8]);
//
//    printf("trans vec: \n %.3f %.3f %.3f\n",tv[0],tv[1],tv[2]);
//
//    double _pm[12] = {_r[0],_r[1],_r[2],tv[0],
//        _r[3],_r[4],_r[5],tv[1],
//        _r[6],_r[7],_r[8],tv[2]};
//
//    Matx34d P(_pm);
//    Mat KP = camMatrix * Mat(P);
//    //	cout << "KP " << endl << KP << endl;
//
//    //reproject object points - check validity of found projection matrix
//    for (int i=0; i<op.rows; i++) {
//        Mat_<double> X = (Mat_<double>(4,1) << op.at<float>(i,0),op.at<float>(i,1),op.at<float>(i,2),1.0);
//        //		cout << "object point " << X << endl;
//        Mat_<double> opt_p = KP * X;
//        Point2f opt_p_img(opt_p(0)/opt_p(2),opt_p(1)/opt_p(2));
//        //		cout << "object point reproj " << opt_p_img << endl;
//
//        circle(img, opt_p_img, 4, Scalar(0,0,255), 1);
//    }
//    rotM = rotM.t();// transpose to conform with majorness of opengl matrix
//}

void init_webcam(string file)
{
    bool FILE_EXIST = false;
    if(!file.empty()){
        std::fstream test(file);
        if(test.is_open())
            FILE_EXIST = true;
        test.close();
    }
    if (!FILE_EXIST)
        cap = VideoCapture(0);
    else
        cap = VideoCapture(file);
    detector = get_frontal_face_detector();
    deserialize("shape_predictor_68_face_landmarks.dat") >> pose_model;
}



void move(int value)
{
    if(AUTOROTATE){
        if(rotateY >= REVERSE || rotateY <= -REVERSE)
            dir *= -1;
        rotateY += dir;
    }
    //printf("Y:%d\n",rotateY);
    
    if(value % 20 == 0) //change color every 20 frame
        CHANGECOLOR = true;
    
    loadNext();
    glutPostRedisplay();
    glutTimerFunc(20, move, value+1);
}

void saveOpenGLBuffer() {
    static unsigned int opengl_buffer_num = 0;
    
    int vPort[4]; glGetIntegerv(GL_VIEWPORT, vPort);
    Mat_<Vec3b> opengl_image(vPort[3],vPort[2]);
    {
        Mat_<Vec4b> opengl_image_4b(vPort[3],vPort[2]);
        glReadPixels(0, 0, vPort[2], vPort[3], GL_RGBA, GL_UNSIGNED_BYTE, opengl_image_4b.data);
        flip(opengl_image_4b,opengl_image_4b,0);
        mixChannels(&opengl_image_4b, 1, &opengl_image, 1, &(Vec6i(0,0,1,1,2,2)[0]), 3);
    }
    stringstream ss; ss << "opengl_buffer_" << opengl_buffer_num++ << ".jpg";
    imwrite(ss.str(), opengl_image);
}

//int __w=250,__h=250;

void key(unsigned char key, int x, int y)
{
    switch (key)
    {
        case 27 :
            exit(0);
            break;
            
            //depth augmentation
        case 'w':
        case 'W':
            ZSCALE += 0.1;
            cout<<"z-scale changed to--> "<<ZSCALE<<endl;
            break;
        case 'Q':
        case 'q':
            ZSCALE -= 0.1;
            cout<<"z-scale changed to--> "<<ZSCALE<<endl;
            break;
            
        case ' ':
            cout<<"pic saved!\n";
            saveOpenGLBuffer();
            //loadNext();
            break;
            
            //manual rotation
        case 'a':
            AUTOROTATE = false;
            rotateY += 5;
            break;
        case 'd':
            AUTOROTATE = false;
            rotateY -= 5;
            break;
            
        case 's':
            AUTOROTATE = !AUTOROTATE;
            if(AUTOROTATE) rotateY = 0; //reset position
            break;
            
            //rotation spped
        case 'e':
            dir *= 1.1;
            std::cout<<"speed up:"<<dir<<'\n';
            break;
        case 'r':
            dir /= 1.1;
            std::cout<<"speed down:"<<dir<<'\n';
            break;
        default:
            break;
    }
    
    glutPostRedisplay();
}

void idle(void)
{
    glutPostRedisplay();
}

void myGLinit() {
    //    glutSetOption ( GLUT_ACTION_ON_WINDOW_CLOSE, GLUT_ACTION_CONTINUE_EXECUTION ) ;
    
    glEnable(GL_CULL_FACE);
    glCullFace(GL_BACK);
    
    
    glShadeModel(GL_SMOOTH);
    
    glEnable(GL_DEPTH_TEST);
    glDepthFunc(GL_LEQUAL);
    
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    
    glEnable(GL_LIGHT0);
    glEnable(GL_NORMALIZE);
    glEnable(GL_COLOR_MATERIAL);
    glColorMaterial ( GL_FRONT, GL_AMBIENT_AND_DIFFUSE );
    
    glLightfv(GL_LIGHT0, GL_AMBIENT,  light_ambient);
    glLightfv(GL_LIGHT0, GL_DIFFUSE,  light_diffuse);
    glLightfv(GL_LIGHT0, GL_SPECULAR, light_specular);
    glLightfv(GL_LIGHT0, GL_POSITION, light_position);
    
    glMaterialfv(GL_FRONT, GL_AMBIENT,   mat_ambient);
    glMaterialfv(GL_FRONT, GL_DIFFUSE,   mat_diffuse);
    glMaterialfv(GL_FRONT, GL_SPECULAR,  mat_specular);
    glMaterialfv(GL_FRONT, GL_SHININESS, high_shininess);
    
    glEnable(GL_LIGHTING);
    
    glMatrixMode(GL_MODELVIEW);
    glLoadIdentity();
    
}


void resize(int width, int height)
{
    //const float ar = (float) width / (float) height;
    
    glViewport(0, 0, width, height);
    
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    //glFrustum(-ar, ar, -1.0, 1.0, 2.0, 100.0);
    //gluPerspective(47,1.0,2,100);
    glOrtho(-300, 300, -200, 200, -800, 800);
    glMatrixMode(GL_MODELVIEW);
    glLoadIdentity();
}
void readcsv()
{
    std::ifstream landmark("landmark.txt");
    if(!landmark.is_open())
        std::cout<<"You need conn.txt in the same directory as the executable!\n";
    std::vector<std::string> tmp_point3d =  getNextLineAndSplitIntoTokens(landmark);
    int count = 0;
    while(tmp_point3d.size() == 3){
        landmark_points.push_back(Point3d(std::stod(tmp_point3d[0]), std::stod(tmp_point3d[1]), std::stod(tmp_point3d[2])));
        count ++;
        tmp_point3d =  getNextLineAndSplitIntoTokens(landmark);
    }
    landmark.close();
    std::cout<<"there are "<<count<<" landmarks read!\n";
    // landmark.close();
    // std::ifstream is("conn.txt");
    // is.seekg(0, is.end);
    // int length = is.tellg();
    // is.seekg(0, is.beg);
    //
    // std::string str;
    // str.resize(length, ' '); // reserve space
    // char* begin = &*str.begin();
    //
    // is.read(begin, length);
    // is.close();
    //
    // int tri_num = std::count(str.begin(), str.end(), '\n');
    // double tri_points[tri_num][3];
    // std::stringstream connectivities(str);
    count = 0;//cout<<count<<endl;
    std::ifstream connectivities("conn.txt");
    if(!connectivities.is_open())
        std::cout<<"You need conn.txt in the same directory as the executable!\n";
    std::vector<std::string> tmp_tri = getNextLineAndSplitIntoTokens(connectivities);
    while(tmp_tri.size() == 3){
        triangleIndex tmp;
        tmp.index[0] = std::stoi(tmp_tri[0]);
        tmp.index[1] = std::stoi(tmp_tri[1]);
        tmp.index[2] = std::stoi(tmp_tri[2]);
        triangles.push_back(tmp);
        count++;
        tmp_tri = getNextLineAndSplitIntoTokens(connectivities);
    }
    connectivities.close();
    std::cout<<"there are "<<count<<" triangles read!\n";
}

int start_opengl() {
    glutTimerFunc(20, move, 0);
    glutMainLoop();
    return 1;
}

void init_opengl(int argc,char** argv) {
    glutInitWindowSize(900,500);
    glutInitWindowPosition(40,40);
    glutInit(&argc, argv);
    glutInitDisplayMode(GLUT_RGB | GLUT_DOUBLE | GLUT_DEPTH); // | GLUT_MULTISAMPLE
    glutCreateWindow("DenseFace3D");
    
    myGLinit();
    
    glutReshapeFunc(resize);
    glutDisplayFunc(display);
    glutKeyboardFunc(key);
    //glutSpecialFunc(special);
    //glutIdleFunc(idle);
}

int main(int argc, char** argv)
{
    try {
        init_opengl(argc, argv); // get GL context, for loading textures
        
        string filename;
        if(argc >= 2)
            filename = string(argv[1]);
        
        //prepare dlib and cam
        init_webcam(filename);
        
        readcsv();
        // prepare OpenCV-OpenGL images
        imgTex = MakeOpenCVGLTexture(Mat());
        imgWithDrawing = MakeOpenCVGLTexture(Mat());
        
        tps = new ThinPlateSpline(68);
        
        loadNext();
        start_opengl();
        
        return 0;
    }
    catch(serialization_error& e)
    {
        cout << "You need dlib's default face landmarking model file to run this example." << endl;
        cout << "You can get it from the following URL: " << endl;
        cout << "   http://dlib.net/files/shape_predictor_68_face_landmarks.dat.bz2" << endl;
        cout << endl << e.what() << endl;
    }
    catch(exception& e)
    {
        cout << e.what() << endl;
    }
    
}

void drawAxes() {
    
    //Z = red
    glPushMatrix();
    glRotated(180,0,1,0);
    glColor4d(1,0,0,0.5);
    //	glutSolidCylinder(0.05,1,15,20);
    glBegin(GL_LINES);
    glVertex3d(0, 0, 0); glVertex3d(0, 0, 1);
    glEnd();
    glTranslated(0,0,1);
    glScaled(.1,.1,.1);
    glutSolidTetrahedron();
    glPopMatrix();
    
    //Y = green
    glPushMatrix();
    glRotated(-90,1,0,0);
    glColor4d(0,1,0,0.5);
    //	glutSolidCylinder(0.05,1,15,20);
    glBegin(GL_LINES);
    glVertex3d(0, 0, 0); glVertex3d(0, 0, 1);
    glEnd();
    glTranslated(0,0,1);
    glScaled(.1,.1,.1);
    glutSolidTetrahedron();
    glPopMatrix();
    
    //X = blue
    glPushMatrix();
    glRotated(-90,0,1,0);
    glColor4d(0,0,1,0.5);
    //	glutSolidCylinder(0.05,1,15,20);
    glBegin(GL_LINES);
    glVertex3d(0, 0, 0); glVertex3d(0, 0, 1);
    glEnd();
    glTranslated(0,0,1);
    glScaled(.1,.1,.1);
    glutSolidTetrahedron();
    glPopMatrix();
}


```

## Source file: glm.h

{
  "path": "glm.h",
  "blob_sha": "18594d89101331fae6ad18ac1b59f5535f546ee7",
  "sha256": "b6fad7693dddd097ab97a0085845103950c594b5d4aff4bd1dcc0da4bfe1a753",
  "lines": 316,
  "bytes": 11300,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2,
    "face": 13
  },
  "url": "https://github.com/HRLTY/DenseFace3D/blob/a01e59bc6f9bc19fc0c7c4a4576cd2b536c93f38/glm.h",
  "function_signature_candidates": []
}

```text

/*    
      glm.h
      Nate Robins, 1997, 2000
      nate@pobox.com, http://www.pobox.com/~nate
 
      Wavefront OBJ model file format reader/writer/manipulator.

      Includes routines for generating smooth normals with
      preservation of edges, welding redundant vertices & texture
      coordinate generation (spheremap and planar projections) + more.

      Improved version:
	  Tudor Carean - April 2008 - added texture support

 */


#ifdef _WIN32
#include <windows.h>
#endif
#include <GL/glut.h>


#ifndef M_PI
#define M_PI 3.14159265f
#endif

#define GLM_NONE     (0)            /* render with only vertices */
#define GLM_FLAT     (1 << 0)       /* render with facet normals */
#define GLM_SMOOTH   (1 << 1)       /* render with vertex normals */
#define GLM_TEXTURE  (1 << 2)       /* render with texture coords */
#define GLM_COLOR    (1 << 3)       /* render with colors */
#define GLM_MATERIAL (1 << 4)       /* render with materials */


/* GLMmaterial: Structure that defines a material in a model. 
 */
typedef struct _GLMmaterial
{
  char* name;                   /* name of material */
  GLfloat diffuse[4];           /* diffuse component */
  GLfloat ambient[4];           /* ambient component */
  GLfloat specular[4];          /* specular component */
  GLfloat emmissive[4];         /* emmissive component */
  GLfloat shininess;            /* specular exponent */
//this is for textures
  GLuint IDTextura;		// ID-ul texturii difuze
} GLMmaterial;

/* GLMtriangle: Structure that defines a triangle in a model.
 */
typedef struct _GLMtriangle {
  GLuint vindices[3];           /* array of triangle vertex indices */
  GLuint nindices[3];           /* array of triangle normal indices */
  GLuint tindices[3];           /* array of triangle texcoord indices*/
  GLuint findex;                /* index of triangle facet normal */
  //GLuint nrvecini;
  GLuint vecini[3];
  bool visible;
} GLMtriangle;

//adaugat pentru suport texturi
typedef struct _GLMtexture {
  char *name;
  GLuint id;                    /* ID-ul texturii */
  GLfloat width;		/* width and height for texture coordinates */
  GLfloat height;
} GLMtexture;

/* GLMgroup: Structure that defines a group in a model.
 */
typedef struct _GLMgroup {
  char*             name;           /* name of this group */
  GLuint            numtriangles;   /* number of triangles in this group */
  GLuint*           triangles;      /* array of triangle indices */
  GLuint            material;       /* index to material for group */
  struct _GLMgroup* next;           /* pointer to next group in model */
} GLMgroup;

/* GLMmodel: Structure that defines a model.
 */
typedef struct _GLMmodel {
  char*    pathname;            /* path to this model */
  char*    mtllibname;          /* name of the material library */

  GLuint   numvertices;         /* number of vertices in model */
  GLfloat* vertices;            /* array of vertices  */

  GLuint   numnormals;          /* number of normals in model */
  GLfloat* normals;             /* array of normals */

  GLuint   numtexcoords;        /* number of texcoords in model */
  GLfloat* texcoords;           /* array of texture coordinates */

  GLuint   numfacetnorms;       /* number of facetnorms in model */
  GLfloat* facetnorms;          /* array of facetnorms */

  GLuint       numtriangles;    /* number of triangles in model */
  GLMtriangle* triangles;       /* array of triangles */

  GLuint       nummaterials;    /* number of materials in model */
  GLMmaterial* materials;       /* array of materials */

  GLuint       numgroups;       /* number of groups in model */
  GLMgroup*    groups;          /* linked list of groups */

  // textures
  GLuint       numtextures;
  GLMtexture*  textures;

  GLfloat position[3];          /* position of the model */

} GLMmodel;

struct mycallback
{
	void (*loadcallback)(int,char *);
	int start; 
	int end;
	char *text;
};

GLvoid glmDraw(GLMmodel* model, GLuint mode,char *drawonly);

GLfloat glmDot(GLfloat* u, GLfloat* v);

/* glmUnitize: "unitize" a model by translating it to the origin and
 * scaling it to fit in a unit cube around the origin.  Returns the
 * scalefactor used.
 *
 * model - properly initialized GLMmodel structure 
 */
GLfloat
glmUnitize(GLMmodel* model);

/* glmDimensions: Calculates the dimensions (width, height, depth) of
 * a model.
 *
 * model      - initialized GLMmodel structure
 * dimensions - array of 3 GLfloats (GLfloat dimensions[3])
 */
GLvoid
glmDimensions(GLMmodel* model, GLfloat* dimensions);

/* glmScale: Scales a model by a given amount.
 * 
 * model - properly initialized GLMmodel structure
 * scale - scalefactor (0.5 = half as large, 2.0 = twice as large)
 */
GLvoid
glmScale(GLMmodel* model, GLfloat scale);

/* glmReverseWinding: Reverse the polygon winding for all polygons in
 * this model.  Default winding is counter-clockwise.  Also changes
 * the direction of the normals.
 * 
 * model - properly initialized GLMmodel structure 
 */
GLvoid
glmReverseWinding(GLMmodel* model);

/* glmFacetNormals: Generates facet normals for a model (by taking the
 * cross product of the two vectors derived from the sides of each
 * triangle).  Assumes a counter-clockwise winding.
 *
 * model - initialized GLMmodel structure
 */
GLvoid
glmFacetNormals(GLMmodel* model);

/* glmVertexNormals: Generates smooth vertex normals for a model.
 * First builds a list of all the triangles each vertex is in.  Then
 * loops through each vertex in the the list averaging all the facet
 * normals of the triangles each vertex is in.  Finally, sets the
 * normal index in the triangle for the vertex to the generated smooth
 * normal.  If the dot product of a facet normal and the facet normal
 * associated with the first triangle in the list of triangles the
 * current vertex is in is greater than the cosine of the angle
 * parameter to the function, that facet normal is not added into the
 * average normal calculation and the corresponding vertex is given
 * the facet normal.  This tends to preserve hard edges.  The angle to
 * use depends on the model, but 90 degrees is usually a good start.
 *
 * model - initialized GLMmodel structure
 * angle - maximum angle (in degrees) to smooth across
 */
GLvoid
glmVertexNormals(GLMmodel* model, GLfloat angle);

/* glmLinearTexture: Generates texture coordinates according to a
 * linear projection of the texture map.  It generates these by
 * linearly mapping the vertices onto a square.
 *
 * model - pointer to initialized GLMmodel structure
 */
GLvoid
glmLinearTexture(GLMmodel* model);

/* glmSpheremapTexture: Generates texture coordinates according to a
 * spherical projection of the texture map.  Sometimes referred to as
 * spheremap, or reflection map texture coordinates.  It generates
 * these by using the normal to calculate where that vertex would map
 * onto a sphere.  Since it is impossible to map something flat
 * perfectly onto something spherical, there is distortion at the
 * poles.  This particular implementation causes the poles along the X
 * axis to be distorted.
 *
 * model - pointer to initialized GLMmodel structure
 */
GLvoid
glmSpheremapTexture(GLMmodel* model);

/* glmDelete: Deletes a GLMmodel structure.
 *
 * model - initialized GLMmodel structure
 */
GLvoid
glmDelete(GLMmodel* model);

/* glmReadOBJ: Reads a model description from a Wavefront .OBJ file.
 * Returns a pointer to the created object which should be free'd with
 * glmDelete().
 *
 * filename - name of the file containing the Wavefront .OBJ format data.  
 */
//GLMmodel * glmReadOBJ(char* filename);
GLMmodel* glmReadOBJ(char* filename);
GLMmodel* glmReadOBJ(char* filename,mycallback *call);

/* glmWriteOBJ: Writes a model description in Wavefront .OBJ format to
 * a file.
 *
 * model    - initialized GLMmodel structure
 * filename - name of the file to write the Wavefront .OBJ format data to
 * mode     - a bitwise or of values describing what is written to the file
 *            GLM_NONE    -  write only vertices
 *            GLM_FLAT    -  write facet normals
 *            GLM_SMOOTH  -  write vertex normals
 *            GLM_TEXTURE -  write texture coords
 *            GLM_FLAT and GLM_SMOOTH should not both be specified.
 */
GLvoid
glmWriteOBJ(GLMmodel* model, char* filename, GLuint mode);

/* glmDraw: Renders the model to the current OpenGL context using the
 * mode specified.
 *
 * model    - initialized GLMmodel structure
 * mode     - a bitwise OR of values describing what is to be rendered.
 *            GLM_NONE    -  render with only vertices
 *            GLM_FLAT    -  render with facet normals
 *            GLM_SMOOTH  -  render with vertex normals
 *            GLM_TEXTURE -  render with texture coords
 *            GLM_FLAT and GLM_SMOOTH should not both be specified.
 */
GLvoid
glmDraw(GLMmodel* model, GLuint mode);

/* glmList: Generates and returns a display list for the model using
 * the mode specified.
 *
 * model    - initialized GLMmodel structure
 * mode     - a bitwise OR of values describing what is to be rendered.
 *            GLM_NONE    -  render with only vertices
 *            GLM_FLAT    -  render with facet normals
 *            GLM_SMOOTH  -  render with vertex normals
 *            GLM_TEXTURE -  render with texture coords
 *            GLM_FLAT and GLM_SMOOTH should not both be specified.  
 */
GLuint
glmList(GLMmodel* model, GLuint mode);

/* glmWeld: eliminate (weld) vectors that are within an epsilon of
 * each other.
 *
 * model      - initialized GLMmodel structure
 * epsilon    - maximum difference between vertices
 *              ( 0.00001 is a good start for a unitized model)
 *
 */
GLvoid
glmWeld(GLMmodel* model, GLfloat epsilon);

/* glmReadPPM: read a PPM raw (type P6) file.  The PPM file has a header
 * that should look something like:
 *
 *    P6
 *    # comment
 *    width height max_value
 *    rgbrgbrgb...
 *
 * where "P6" is the magic cookie which identifies the file type and
 * should be the only characters on the first line followed by a
 * carriage return.  Any line starting with a # mark will be treated
 * as a comment and discarded.   After the magic cookie, three integer
 * values are expected: width, height of the image and the maximum
 * value for a pixel (max_value must be < 256 for PPM raw files).  The
 * data section consists of width*height rgb triplets (one byte each)
 * in binary format (i.e., such as that written with fwrite() or
 * equivalent).
 *
 * The rgb data is returned as an array of unsigned chars (packed
 * rgb).  The malloc()'d memory should be free()'d by the caller.  If
 * an error occurs, an error message is sent to stderr and NULL is
 * returned.
 *
 * filename   - name of the .ppm file.
 * width      - will contain the width of the image on return.
 * height     - will contain the height of the image on return.
 *
 */
GLubyte* 
glmReadPPM(char* filename, int* width, int* height);

GLMgroup*
glmFindGroup(GLMmodel* model, char* name);


```

## Source file: glm.cpp

{
  "path": "glm.cpp",
  "blob_sha": "b3b4714a3de15e81f006b944cafe80a6dadbb1e5",
  "sha256": "66eb22d34634c78fe13957f83f60b348ca2567eaf084e7a75504f26d893c0f5d",
  "lines": 2050,
  "bytes": 72302,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2,
    "face": 54
  },
  "url": "https://github.com/HRLTY/DenseFace3D/blob/a01e59bc6f9bc19fc0c7c4a4576cd2b536c93f38/glm.cpp",
  "function_signature_candidates": [
    {
      "line": 248,
      "text": "int glmFindOrAddTexture(GLMmodel* model, char* name,mycallback *call)"
    }
  ]
}

```text

/*    
      glm.c
      Nate Robins, 1997, 2000
      nate@pobox.com, http://www.pobox.com/~nate
 
      Wavefront OBJ model file format reader/writer/manipulator.

      Includes routines for generating smooth normals with
      preservation of edges, welding redundant vertices & texture
      coordinate generation (spheremap and planar projections) + more.
	
      Improved version of GLM - 08.05.2008 Tudor Carean
	  Added support for textures and loading callbacks
*/

//#include <stdafx.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "glm.h"


//#define DebugVisibleSurfaces

#define total_textures 5

#define T(x) (model->triangles[(x)])
GLuint glmLoadTexture(char *filename, GLboolean alpha, GLboolean repeat, GLboolean filtering, GLboolean mipmaps, GLfloat *texcoordwidth, GLfloat *texcoordheight);

/* _GLMnode: general purpose node */
typedef struct _GLMnode {
    GLuint         index;
    GLboolean      averaged;
    struct _GLMnode* next;
} GLMnode;


/* glmMax: returns the maximum of two floats */
static GLfloat
glmMax(GLfloat a, GLfloat b) 
{
    if (b > a)
        return b;
    return a;
}

/* glmAbs: returns the absolute value of a float */
static GLfloat
glmAbs(GLfloat f)
{
    if (f < 0)
        return -f;
    return f;
}

/* glmDot: compute the dot product of two vectors
 *
 * u - array of 3 GLfloats (GLfloat u[3])
 * v - array of 3 GLfloats (GLfloat v[3])
 */
GLfloat glmDot(GLfloat* u, GLfloat* v)
{
    assert(u); assert(v);
    
    return u[0]*v[0] + u[1]*v[1] + u[2]*v[2];
}

/* glmCross: compute the cross product of two vectors
 *
 * u - array of 3 GLfloats (GLfloat u[3])
 * v - array of 3 GLfloats (GLfloat v[3])
 * n - array of 3 GLfloats (GLfloat n[3]) to return the cross product in
 */
static GLvoid
glmCross(GLfloat* u, GLfloat* v, GLfloat* n)
{
    assert(u); assert(v); assert(n);
    
    n[0] = u[1]*v[2] - u[2]*v[1];
    n[1] = u[2]*v[0] - u[0]*v[2];
    n[2] = u[0]*v[1] - u[1]*v[0];
}

/* glmNormalize: normalize a vector
 *
 * v - array of 3 GLfloats (GLfloat v[3]) to be normalized
 */
static GLvoid
glmNormalize(GLfloat* v)
{
    GLfloat l;
    
    assert(v);
    
    l = (GLfloat)sqrt(v[0]*v[0] + v[1]*v[1] + v[2]*v[2]);
    v[0] /= l;
    v[1] /= l;
    v[2] /= l;
}

/* glmEqual: compares two vectors and returns GL_TRUE if they are
 * equal (within a certain threshold) or GL_FALSE if not. An epsilon
 * that works fairly well is 0.000001.
 *
 * u - array of 3 GLfloats (GLfloat u[3])
 * v - array of 3 GLfloats (GLfloat v[3]) 
 */
static GLboolean
glmEqual(GLfloat* u, GLfloat* v, GLfloat epsilon)
{
    if (glmAbs(u[0] - v[0]) < epsilon &&
        glmAbs(u[1] - v[1]) < epsilon &&
        glmAbs(u[2] - v[2]) < epsilon) 
    {
        return GL_TRUE;
    }
    return GL_FALSE;
}

/* glmWeldVectors: eliminate (weld) vectors that are within an
 * epsilon of each other.
 *
 * vectors     - array of GLfloat[3]'s to be welded
 * numvectors - number of GLfloat[3]'s in vectors
 * epsilon     - maximum difference between vectors 
 *
 */
GLfloat*
glmWeldVectors(GLfloat* vectors, GLuint* numvectors, GLfloat epsilon)
{
    GLfloat* copies;
    GLuint   copied;
    GLuint   i, j;
    
    copies = (GLfloat*)malloc(sizeof(GLfloat) * 3 * (*numvectors + 1));
    memcpy(copies, vectors, (sizeof(GLfloat) * 3 * (*numvectors + 1)));
    
    copied = 1;
    for (i = 1; i <= *numvectors; i++) {
        for (j = 1; j <= copied; j++) {
            if (glmEqual(&vectors[3 * i], &copies[3 * j], epsilon)) {
                goto duplicate;
            }
        }
        
        /* must not be any duplicates -- add to the copies array */
        copies[3 * copied + 0] = vectors[3 * i + 0];
        copies[3 * copied + 1] = vectors[3 * i + 1];
        copies[3 * copied + 2] = vectors[3 * i + 2];
        j = copied;             /* pass this along for below */
        copied++;
        
duplicate:
/* set the first component of this vector to point at the correct
        index into the new copies array */
        vectors[3 * i + 0] = (GLfloat)j;
    }
    
    *numvectors = copied-1;
    return copies;
}

/* glmFindGroup: Find a group in the model */
GLMgroup*
glmFindGroup(GLMmodel* model, char* name)
{
    GLMgroup* group;
    
    assert(model);
    
    group = model->groups;
    while(group) {
        if (!strcmp(name, group->name))			
            break;
        group = group->next;
    }
    
    return group;
}

/* glmAddGroup: Add a group to the model */
GLMgroup*
glmAddGroup(GLMmodel* model, char* name)
{
    GLMgroup* group;
    
    group = glmFindGroup(model, name);
    if (!group) {
        group = (GLMgroup*)malloc(sizeof(GLMgroup));
        group->name = strdup(name);
        group->material = 0;
        group->numtriangles = 0;
        group->triangles = NULL;
        group->next = model->groups;
        model->groups = group;
        model->numgroups++;
    }
    
    return group;
}

/* glmFindGroup: Find a material in the model */
GLuint
glmFindMaterial(GLMmodel* model, char* name)
{
    GLuint i;
    
    /* XXX doing a linear search on a string key'd list is pretty lame,
    but it works and is fast enough for now. */
    for (i = 0; i < model->nummaterials; i++) {
        if (!strcmp(model->materials[i].name, name))
            goto found;
    }
    
    /* didn't find the name, so print a warning and return the default
    material (0). */
    printf("glmFindMaterial():  can't find material \"%s\".\n", name);
    i = 0;
    
found:
    return i;
}
/* glmDirName: return the directory given a path
 *
 * path - filesystem path
 *
 * NOTE: the return value should be free'd.
 */
static char*
glmDirName(char* path)
{
    char* dir;
    char* s;
    
    dir = strdup(path);
    
    s = strrchr(dir, '/');
    if (s)
        s[1] = '\0';
    else
        dir[0] = '\0';
    
    return dir;
}

int glmFindOrAddTexture(GLMmodel* model, char* name,mycallback *call)
{
    GLuint i;
    char *dir, *filename;
    float width, height;

    char *numefis = name;
    while (*numefis==' ') numefis++;

    for (i = 0; i < model->numtextures; i++) {
        if (!strcmp(model->textures[i].name, numefis))
            return i;
    }
	char afis[80];
	sprintf(afis,"Loading Textures (%s )...",name);
	int procent = ((float)((float)model->numtextures*30/total_textures)/100)*(call->end-call->start)+call->start;
	if (call) call->loadcallback(procent,afis); // textures represent 30% from the model (just saying :))
	if (strstr(name,":\\"))
	{
		filename = (char*)malloc(sizeof(char) * (strlen(name) + 1));
		strcpy(filename,name);		
	}
	else
	{
		dir = glmDirName(model->pathname);
		filename = (char*)malloc(sizeof(char) * (strlen(dir) + strlen(numefis) + 1));
	
		strcpy(filename, dir);
		//strcpy(filename, numefis);
		strcat(filename, numefis);
		free(dir);
	}
	int lung = strlen(filename);
	if (filename[lung-1]<32) filename[lung-1]=0;
	if (filename[lung-2]<32) filename[lung-2]=0;    

    model->numtextures++;
    model->textures = (GLMtexture*)realloc(model->textures, sizeof(GLMtexture)*model->numtextures);
    model->textures[model->numtextures-1].name = strdup(numefis);
//    model->textures[model->numtextures-1].id = glmLoadTexture(filename, GL_TRUE, GL_TRUE, GL_TRUE, GL_TRUE, &width, &height);	
    model->textures[model->numtextures-1].width = width;
    model->textures[model->numtextures-1].height = height;
    

    free(filename);

    return model->numtextures-1;
}




/* glmReadMTL: read a wavefront material library file
 *
 * model - properly initialized GLMmodel structure
 * name  - name of the material library
 */
static GLvoid
glmReadMTL(GLMmodel* model, char* name, mycallback *call)
{
    FILE* file;
    char* dir;
    char* filename;
	char* textura;
    char    buf[128];
    GLuint nummaterials, i;
	
    dir = glmDirName(model->pathname);
    filename = (char*)malloc(sizeof(char) * (strlen(dir) + strlen(name) + 1));
    strcpy(filename, dir);
    strcat(filename, name);
    free(dir);
    
    file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "glmReadMTL() failed: can't open material file \"%s\".\n",
            filename);
        exit(1);
    }
    free(filename);
    
    /* count the number of materials in the file */
    nummaterials = 1;
    while(fscanf(file, "%s", buf) != EOF) {
        switch(buf[0]) {
        case '#':               /* comment */
            /* eat up rest of line */
            fgets(buf, sizeof(buf), file);
            break;
        case 'n':               /* newmtl */
            fgets(buf, sizeof(buf), file);
            nummaterials++;
            sscanf(buf, "%s %s", buf, buf);
            break;
        default:
            /* eat up rest of line */
            fgets(buf, sizeof(buf), file);
            break;
        }
    }
    
    rewind(file);
    
    model->materials = (GLMmaterial*)malloc(sizeof(GLMmaterial) * nummaterials);
    model->nummaterials = nummaterials;
    
    /* set the default material */
    for (i = 0; i < nummaterials; i++) {
        model->materials[i].name = NULL;
        model->materials[i].shininess = 65.0;
        model->materials[i].diffuse[0] = 0.8;
        model->materials[i].diffuse[1] = 0.8;
        model->materials[i].diffuse[2] = 0.8;
        model->materials[i].diffuse[3] = 1.0;
        model->materials[i].ambient[0] = 0.2;
        model->materials[i].ambient[1] = 0.2;
        model->materials[i].ambient[2] = 0.2;
        model->materials[i].ambient[3] = 1.0;
        model->materials[i].specular[0] = 0.0;
        model->materials[i].specular[1] = 0.0;
        model->materials[i].specular[2] = 0.0;
        model->materials[i].specular[3] = 1.0;
		model->materials[i].IDTextura = -1;
    }
    model->materials[0].name = strdup("default");
    
    /* now, read in the data */
    nummaterials = 0;
    while(fscanf(file, "%s", buf) != EOF) {
        switch(buf[0]) {
        case '#':               /* comment */
            /* eat up rest of line */
            fgets(buf, sizeof(buf), file);
            break;
        case 'n':               /* newmtl */
            fgets(buf, sizeof(buf), file);
            sscanf(buf, "%s %s", buf, buf);
            nummaterials++;
            model->materials[nummaterials].name = strdup(buf);
            break;
        case 'N':
            if (buf[1]!='s') break; // 3DS pune 'i' aici pentru indici de refractie si se incurca
            fscanf(file, "%f", &model->materials[nummaterials].shininess);
            /* wavefront shininess is from [0, 1000], so scale for OpenGL */
            model->materials[nummaterials].shininess /= 1000.0;
            model->materials[nummaterials].shininess *= 128.0;
            break;
        case 'K':
            switch(buf[1]) {
            case 'd':
                fscanf(file, "%f %f %f",
                    &model->materials[nummaterials].diffuse[0],
                    &model->materials[nummaterials].diffuse[1],
                    &model->materials[nummaterials].diffuse[2]);
                break;
            case 's':
                fscanf(file, "%f %f %f",
                    &model->materials[nummaterials].specular[0],
                    &model->materials[nummaterials].specular[1],
                    &model->materials[nummaterials].specular[2]);
                break;
            case 'a':
                fscanf(file, "%f %f %f",
                    &model->materials[nummaterials].ambient[0],
                    &model->materials[nummaterials].ambient[1],
                    &model->materials[nummaterials].ambient[2]);
                break;
            default:
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                break;
            }
            break;
             case 'm':
            // harta de texturi
            filename = (char *)malloc(FILENAME_MAX);
            fgets(filename, FILENAME_MAX, file);
            textura = strdup(filename);
            free(filename);
            if(strncmp(buf, "map_Kd", 6) == 0) 
			{
				char afis[80];
				sprintf(afis,"Loading Textures (%s)...",textura);				
                model->materials[nummaterials].IDTextura = glmFindOrAddTexture(model, textura,call);
                free(textura);
            } else {
                //printf("map %s %s ignored",buf,t_filename);
                free(textura);
                //fgets(buf, sizeof(buf), file);
            }
            break;
            default:
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                break;
        }
    }
}

/* glmWriteMTL: write a wavefront material library file
 *
 * model   - properly initialized GLMmodel structure
 * modelpath  - pathname of the model being written
 * mtllibname - name of the material library to be written
 */
static GLvoid
glmWriteMTL(GLMmodel* model, char* modelpath, char* mtllibname)
{
    FILE* file;
    char* dir;
    char* filename;
    GLMmaterial* material;
    GLuint i;
    
    dir = glmDirName(modelpath);
    filename = (char*)malloc(sizeof(char) * (strlen(dir)+strlen(mtllibname)));
    strcpy(filename, dir);
    strcat(filename, mtllibname);
    free(dir);
    
    /* open the file */
    file = fopen(filename, "w");
    if (!file) {
        fprintf(stderr, "glmWriteMTL() failed: can't open file \"%s\".\n",
            filename);
        exit(1);
    }
    free(filename);
    
    /* spit out a header */
    fprintf(file, "#  \n");
    fprintf(file, "#  Wavefront MTL generated by GLM library\n");
    fprintf(file, "#  \n");
    fprintf(file, "#  GLM library\n");
    fprintf(file, "#  Nate Robins\n");
    fprintf(file, "#  ndr@pobox.com\n");
    fprintf(file, "#  http://www.pobox.com/~ndr\n");
    fprintf(file, "#  \n\n");
    
    for (i = 0; i < model->nummaterials; i++) {
        material = &model->materials[i];
        fprintf(file, "newmtl %s\n", material->name);
        fprintf(file, "Ka %f %f %f\n", 
            material->ambient[0], material->ambient[1], material->ambient[2]);
        fprintf(file, "Kd %f %f %f\n", 
            material->diffuse[0], material->diffuse[1], material->diffuse[2]);
        fprintf(file, "Ks %f %f %f\n", 
            material->specular[0],material->specular[1],material->specular[2]);
        fprintf(file, "Ns %f\n", material->shininess / 128.0 * 1000.0);
        fprintf(file, "\n");
    }
}


/* glmFirstPass: first pass at a Wavefront OBJ file that gets all the
 * statistics of the model (such as #vertices, #normals, etc)
 *
 * model - properly initialized GLMmodel structure
 * file  - (fopen'd) file descriptor 
 */
static GLvoid glmFirstPass(GLMmodel* model, FILE* file, mycallback *call) 
{
    GLuint  numvertices;        /* number of vertices in model */
    GLuint  numnormals;         /* number of normals in model */
    GLuint  numtexcoords;       /* number of texcoords in model */
    GLuint  numtriangles;       /* number of triangles in model */
    GLMgroup* group;            /* current group */
    unsigned    v, n, t;
    char        buf[128];
    
    /* make a default group */
    group = glmAddGroup(model, "default");
    
    numvertices = numnormals = numtexcoords = numtriangles = 0;
    while(fscanf(file, "%s", buf) != EOF) {
        switch(buf[0]) {
        case '#':               /* comment */
            /* eat up rest of line */
            fgets(buf, sizeof(buf), file);
            break;
        case 'v':               /* v, vn, vt */
            switch(buf[1]) {
            case '\0':          /* vertex */
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                numvertices++;
                break;
            case 'n':           /* normal */
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                numnormals++;
                break;
            case 't':           /* texcoord */
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                numtexcoords++;
                break;
            default:
                printf("glmFirstPass(): Unknown token \"%s\".\n", buf);
                exit(1);
                break;
            }
            break;
            case 'm': //mtllib
                fgets(buf, sizeof(buf), file);
                sscanf(buf, "%s %s", buf, buf);
                model->mtllibname = strdup(buf);
                glmReadMTL(model, buf, call);
                break;
            case 'u': //usemtl
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                break;
            case 'g':               /* group */
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
#if SINGLE_STRING_GROUP_NAMES
                sscanf(buf, "%s", buf);
#else
                buf[strlen(buf)-1] = '\0';  /* nuke '\n' */
#endif
                group = glmAddGroup(model, buf);
                break;
            case 'f':               /* face */
                v = n = t = 0;
                fscanf(file, "%s", buf);
                /* can be one of %d, %d//%d, %d/%d, %d/%d/%d %d//%d */
                if (strstr(buf, "//")) {
                    /* v//n */
                    sscanf(buf, "%d//%d", &v, &n);
                    fscanf(file, "%d//%d", &v, &n);
                    fscanf(file, "%d//%d", &v, &n);
                    numtriangles++;
                    group->numtriangles++;
                    while(fscanf(file, "%d//%d", &v, &n) > 0) {
                        numtriangles++;
                        group->numtriangles++;
                    }
                } else if (sscanf(buf, "%d/%d/%d", &v, &t, &n) == 3) {
                    /* v/t/n */
                    fscanf(file, "%d/%d/%d", &v, &t, &n);
                    fscanf(file, "%d/%d/%d", &v, &t, &n);
                    numtriangles++;
                    group->numtriangles++;
                    while(fscanf(file, "%d/%d/%d", &v, &t, &n) > 0) {
                        numtriangles++;
                        group->numtriangles++;
                    }
                } else if (sscanf(buf, "%d/%d", &v, &t) == 2) {
                    /* v/t */
                    fscanf(file, "%d/%d", &v, &t);
                    fscanf(file, "%d/%d", &v, &t);
                    numtriangles++;
                    group->numtriangles++;
                    while(fscanf(file, "%d/%d", &v, &t) > 0) {
                        numtriangles++;
                        group->numtriangles++;
                    }
                } else {
                    /* v */
                    fscanf(file, "%d", &v);
                    fscanf(file, "%d", &v);
                    numtriangles++;
                    group->numtriangles++;
                    while(fscanf(file, "%d", &v) > 0) {
                        numtriangles++;
                        group->numtriangles++;
                    }
                }
                break;
                
            default:
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                break;
        }
  }
  
  /* set the stats in the model structure */
  model->numvertices  = numvertices;
  model->numnormals   = numnormals;
  model->numtexcoords = numtexcoords;
  model->numtriangles = numtriangles;
  
  /* allocate memory for the triangles in each group */
  group = model->groups;
  while(group) {
      group->triangles = (GLuint*)malloc(sizeof(GLuint) * group->numtriangles);
      group->numtriangles = 0;
      group = group->next;
  }
}

/* glmSecondPass: second pass at a Wavefront OBJ file that gets all
 * the data.
 *
 * model - properly initialized GLMmodel structure
 * file  - (fopen'd) file descriptor 
 */
static GLvoid glmSecondPass(GLMmodel* model, FILE* file, mycallback *call) 
{
    GLuint  numvertices;        /* number of vertices in model */
    GLuint  numnormals;         /* number of normals in model */
    GLuint  numtexcoords;       /* number of texcoords in model */
    GLuint  numtriangles;       /* number of triangles in model */
    GLfloat*    vertices;           /* array of vertices  */
    GLfloat*    normals;            /* array of normals */
    GLfloat*    texcoords;          /* array of texture coordinates */
    GLMgroup* group;            /* current group pointer */
    GLuint  material;           /* current material */
    GLuint  v, n, t;
    char        buf[128];
	char afis[80];
    
    /* set the pointer shortcuts */
    vertices       = model->vertices;
    normals    = model->normals;
    texcoords    = model->texcoords;
    group      = model->groups;
    
    /* on the second pass through the file, read all the data into the
    allocated arrays */
    numvertices = numnormals = numtexcoords = 1;
    numtriangles = 0;
    material = 0;
    while(fscanf(file, "%s", buf) != EOF) {
        switch(buf[0]) {
        case '#':               /* comment */
            /* eat up rest of line */
            fgets(buf, sizeof(buf), file);
            break;
        case 'v':               /* v, vn, vt */
            switch(buf[1]) {
            case '\0':          /* vertex */
                fscanf(file, "%f %f %f", 
                    &vertices[3 * numvertices + 0], 
                    &vertices[3 * numvertices + 1], 
                    &vertices[3 * numvertices + 2]);
                numvertices++;
				if (numvertices%200==0)
					if (call)
					{
						sprintf(afis,"%s (%s )... ",call->text, group->name);
						int procent = ((float)((float)numvertices*70/model->numvertices+30)/100)*(call->end-call->start)+call->start;
						call->loadcallback(procent,afis); // Modelul e 70% din incarcare
					}
                break;
            case 'n':           /* normal */
                fscanf(file, "%f %f %f", 
                    &normals[3 * numnormals + 0],
                    &normals[3 * numnormals + 1], 
                    &normals[3 * numnormals + 2]);
                numnormals++;
                break;
            case 't':           /* texcoord */
                fscanf(file, "%f %f", 
                    &texcoords[2 * numtexcoords + 0],
                    &texcoords[2 * numtexcoords + 1]);
                numtexcoords++;
                break;
            }
            break;
            case 'u':
                fgets(buf, sizeof(buf), file);
                sscanf(buf, "%s %s", buf, buf);
                group->material = material = glmFindMaterial(model, buf);
                break;
            case 'g':               /* group */
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
#if SINGLE_STRING_GROUP_NAMES
                sscanf(buf, "%s", buf);
#else
                buf[strlen(buf)-1] = '\0';  /* nuke '\n' */
#endif
                group = glmFindGroup(model, buf);
                group->material = material;
                break;
            case 'f':               /* face */
                v = n = t = 0;
		T(numtriangles).findex = -1; // ???
		T(numtriangles).vecini[0]=-1;
		T(numtriangles).vecini[1]=-1;
		T(numtriangles).vecini[2]=-1;
                fscanf(file, "%s", buf);
                /* can be one of %d, %d//%d, %d/%d, %d/%d/%d %d//%d */
                if (strstr(buf, "//")) {
                    /* v//n */
                    sscanf(buf, "%d//%d", &v, &n);
                    T(numtriangles).vindices[0] = v;
					if (n== 181228)
					{
						printf("");
					}
                    T(numtriangles).nindices[0] = n;
                    fscanf(file, "%d//%d", &v, &n);
                    T(numtriangles).vindices[1] = v;
                    T(numtriangles).nindices[1] = n;
                    fscanf(file, "%d//%d", &v, &n);
                    T(numtriangles).vindices[2] = v;
                    T(numtriangles).nindices[2] = n;
                    group->triangles[group->numtriangles++] = numtriangles;
                    numtriangles++;
                    while(fscanf(file, "%d//%d", &v, &n) > 0) {						
                        T(numtriangles).vindices[0] = T(numtriangles-1).vindices[0];
                        T(numtriangles).nindices[0] = T(numtriangles-1).nindices[0];
                        T(numtriangles).vindices[1] = T(numtriangles-1).vindices[2];
                        T(numtriangles).nindices[1] = T(numtriangles-1).nindices[2];
                        T(numtriangles).vindices[2] = v;
                        T(numtriangles).nindices[2] = n;
                        group->triangles[group->numtriangles++] = numtriangles;
                        numtriangles++;
                    }
                } else if (sscanf(buf, "%d/%d/%d", &v, &t, &n) == 3) {
                    /* v/t/n */
					
                    T(numtriangles).vindices[0] = v;
                    T(numtriangles).tindices[0] = t;
                    T(numtriangles).nindices[0] = n;
                    fscanf(file, "%d/%d/%d", &v, &t, &n);
                    T(numtriangles).vindices[1] = v;
                    T(numtriangles).tindices[1] = t;
                    T(numtriangles).nindices[1] = n;
                    fscanf(file, "%d/%d/%d", &v, &t, &n);
                    T(numtriangles).vindices[2] = v;
                    T(numtriangles).tindices[2] = t;
                    T(numtriangles).nindices[2] = n;
                    group->triangles[group->numtriangles++] = numtriangles;
                    numtriangles++;
                    while(fscanf(file, "%d/%d/%d", &v, &t, &n) > 0) {
						if (n== 181228)					
                        T(numtriangles).vindices[0] = T(numtriangles-1).vindices[0];
                        T(numtriangles).tindices[0] = T(numtriangles-1).tindices[0];
                        T(numtriangles).nindices[0] = T(numtriangles-1).nindices[0];
                        T(numtriangles).vindices[1] = T(numtriangles-1).vindices[2];
                        T(numtriangles).tindices[1] = T(numtriangles-1).tindices[2];
                        T(numtriangles).nindices[1] = T(numtriangles-1).nindices[2];
                        T(numtriangles).vindices[2] = v;
                        T(numtriangles).tindices[2] = t;
                        T(numtriangles).nindices[2] = n;
                        group->triangles[group->numtriangles++] = numtriangles;
                        numtriangles++;
                    }
                } else if (sscanf(buf, "%d/%d", &v, &t) == 2) {
                    /* v/t */					
                    T(numtriangles).vindices[0] = v;
                    T(numtriangles).tindices[0] = t;
                    fscanf(file, "%d/%d", &v, &t);
                    T(numtriangles).vindices[1] = v;
                    T(numtriangles).tindices[1] = t;
                    fscanf(file, "%d/%d", &v, &t);
                    T(numtriangles).vindices[2] = v;
                    T(numtriangles).tindices[2] = t;
                    group->triangles[group->numtriangles++] = numtriangles;
                    numtriangles++;
                    while(fscanf(file, "%d/%d", &v, &t) > 0) {						
					
                        T(numtriangles).vindices[0] = T(numtriangles-1).vindices[0];
                        T(numtriangles).tindices[0] = T(numtriangles-1).tindices[0];
                        T(numtriangles).vindices[1] = T(numtriangles-1).vindices[2];
                        T(numtriangles).tindices[1] = T(numtriangles-1).tindices[2];
                        T(numtriangles).vindices[2] = v;
                        T(numtriangles).tindices[2] = t;
                        group->triangles[group->numtriangles++] = numtriangles;
                        numtriangles++;
                    }
                } else {
                    /* v */
					//if (n== 181228)				
                    sscanf(buf, "%d", &v);
                    T(numtriangles).vindices[0] = v;
                    fscanf(file, "%d", &v);
                    T(numtriangles).vindices[1] = v;
                    fscanf(file, "%d", &v);
                    T(numtriangles).vindices[2] = v;
                    group->triangles[group->numtriangles++] = numtriangles;
                    numtriangles++;
                    while(fscanf(file, "%d", &v) > 0) {
                        T(numtriangles).vindices[0] = T(numtriangles-1).vindices[0];
                        T(numtriangles).vindices[1] = T(numtriangles-1).vindices[2];
                        T(numtriangles).vindices[2] = v;
                        group->triangles[group->numtriangles++] = numtriangles;
                        numtriangles++;
                    }
                }
                break;
                
            default:
                /* eat up rest of line */
                fgets(buf, sizeof(buf), file);
                break;
    }
  }
  
#if 0
  /* announce the memory requirements */
  printf(" Memory: %d bytes\n",
      numvertices  * 3*sizeof(GLfloat) +
      numnormals   * 3*sizeof(GLfloat) * (numnormals ? 1 : 0) +
      numtexcoords * 3*sizeof(GLfloat) * (numtexcoords ? 1 : 0) +
      numtriangles * sizeof(GLMtriangle));
#endif
}


/* public functions */


/* glmUnitize: "unitize" a model by translating it to the origin and
 * scaling it to fit in a unit cube around the origin.   Returns the
 * scalefactor used.
 *
 * model - properly initialized GLMmodel structure 
 */
GLfloat
glmUnitize(GLMmodel* model)
{
    GLuint  i;
    GLfloat maxx, minx, maxy, miny, maxz, minz;
    GLfloat cx, cy, cz, w, h, d;
    GLfloat scale;
    
    assert(model);
    assert(model->vertices);
    
    /* get the max/mins */
    maxx = minx = model->vertices[3 + 0];
    maxy = miny = model->vertices[3 + 1];
    maxz = minz = model->vertices[3 + 2];
    for (i = 1; i <= model->numvertices; i++) {
        if (maxx < model->vertices[3 * i + 0])
            maxx = model->vertices[3 * i + 0];
        if (minx > model->vertices[3 * i + 0])
            minx = model->vertices[3 * i + 0];
        
        if (maxy < model->vertices[3 * i + 1])
            maxy = model->vertices[3 * i + 1];
        if (miny > model->vertices[3 * i + 1])
            miny = model->vertices[3 * i + 1];
        
        if (maxz < model->vertices[3 * i + 2])
            maxz = model->vertices[3 * i + 2];
        if (minz > model->vertices[3 * i + 2])
            minz = model->vertices[3 * i + 2];
    }
    
    /* calculate model width, height, and depth */
    w = glmAbs(maxx) + glmAbs(minx);
    h = glmAbs(maxy) + glmAbs(miny);
    d = glmAbs(maxz) + glmAbs(minz);
    
    /* calculate center of the model */
    cx = (maxx + minx) / 2.0;
    cy = (maxy + miny) / 2.0;
    cz = (maxz + minz) / 2.0;
    
    /* calculate unitizing scale factor */
    scale = 2.0 / glmMax(glmMax(w, h), d);
    
    /* translate around center then scale */
    for (i = 1; i <= model->numvertices; i++) {
        model->vertices[3 * i + 0] -= cx;
        model->vertices[3 * i + 1] -= cy;
        model->vertices[3 * i + 2] -= cz;
        model->vertices[3 * i + 0] *= scale;
        model->vertices[3 * i + 1] *= scale;
        model->vertices[3 * i + 2] *= scale;
    }
    
    return scale;
}

/* glmDimensions: Calculates the dimensions (width, height, depth) of
 * a model.
 *
 * model   - initialized GLMmodel structure
 * dimensions - array of 3 GLfloats (GLfloat dimensions[3])
 */
GLvoid
glmDimensions(GLMmodel* model, GLfloat* dimensions)
{
    GLuint i;
    GLfloat maxx, minx, maxy, miny, maxz, minz;
    
    assert(model);
    assert(model->vertices);
    assert(dimensions);
    
    /* get the max/mins */
    maxx = minx = model->vertices[3 + 0];
    maxy = miny = model->vertices[3 + 1];
    maxz = minz = model->vertices[3 + 2];
    for (i = 1; i <= model->numvertices; i++) {
        if (maxx < model->vertices[3 * i + 0])
            maxx = model->vertices[3 * i + 0];
        if (minx > model->vertices[3 * i + 0])
            minx = model->vertices[3 * i + 0];
        
        if (maxy < model->vertices[3 * i + 1])
            maxy = model->vertices[3 * i + 1];
        if (miny > model->vertices[3 * i + 1])
            miny = model->vertices[3 * i + 1];
        
        if (maxz < model->vertices[3 * i + 2])
            maxz = model->vertices[3 * i + 2];
        if (minz > model->vertices[3 * i + 2])
            minz = model->vertices[3 * i + 2];
    }
    
    /* calculate model width, height, and depth */
    dimensions[0] = glmAbs(maxx) + glmAbs(minx);
    dimensions[1] = glmAbs(maxy) + glmAbs(miny);
    dimensions[2] = glmAbs(maxz) + glmAbs(minz);
}

/* glmScale: Scales a model by a given amount.
 * 
 * model - properly initialized GLMmodel structure
 * scale - scalefactor (0.5 = half as large, 2.0 = twice as large)
 */
GLvoid
glmScale(GLMmodel* model, GLfloat scale)
{
    GLuint i;
    
    for (i = 1; i <= model->numvertices; i++) {
        model->vertices[3 * i + 0] *= scale;
        model->vertices[3 * i + 1] *= scale;
        model->vertices[3 * i + 2] *= scale;
    }
}

/* glmReverseWinding: Reverse the polygon winding for all polygons in
 * this model.   Default winding is counter-clockwise.  Also changes
 * the direction of the normals.
 * 
 * model - properly initialized GLMmodel structure 
 */
GLvoid
glmReverseWinding(GLMmodel* model)
{
    GLuint i, swap;
    
    assert(model);
    
    for (i = 0; i < model->numtriangles; i++) {
        swap = T(i).vindices[0];
        T(i).vindices[0] = T(i).vindices[2];
        T(i).vindices[2] = swap;
        
        if (model->numnormals) {
            swap = T(i).nindices[0];
            T(i).nindices[0] = T(i).nindices[2];
            T(i).nindices[2] = swap;
        }
        
        if (model->numtexcoords) {
            swap = T(i).tindices[0];
            T(i).tindices[0] = T(i).tindices[2];
            T(i).tindices[2] = swap;
        }
    }
    
    /* reverse facet normals */
    for (i = 1; i <= model->numfacetnorms; i++) {
        model->facetnorms[3 * i + 0] = -model->facetnorms[3 * i + 0];
        model->facetnorms[3 * i + 1] = -model->facetnorms[3 * i + 1];
        model->facetnorms[3 * i + 2] = -model->facetnorms[3 * i + 2];
    }
    
    /* reverse vertex normals */
    for (i = 1; i <= model->numnormals; i++) {
        model->normals[3 * i + 0] = -model->normals[3 * i + 0];
        model->normals[3 * i + 1] = -model->normals[3 * i + 1];
        model->normals[3 * i + 2] = -model->normals[3 * i + 2];
    }
}

/* glmFacetNormals: Generates facet normals for a model (by taking the
 * cross product of the two vectors derived from the sides of each
 * triangle).  Assumes a counter-clockwise winding.
 *
 * model - initialized GLMmodel structure
 */
GLvoid
glmFacetNormals(GLMmodel* model)
{
    GLuint  i;
    GLfloat u[3];
    GLfloat v[3];
    
    assert(model);
    assert(model->vertices);
    
    /* clobber any old facetnormals */
    if (model->facetnorms)
        free(model->facetnorms);
    
    /* allocate memory for the new facet normals */
    model->numfacetnorms = model->numtriangles;
    model->facetnorms = (GLfloat*)malloc(sizeof(GLfloat) *
                       3 * (model->numfacetnorms + 1));
    
    for (i = 0; i < model->numtriangles; i++) 
	{
        model->triangles[i].findex = i+1;		
        
        u[0] = model->vertices[3 * T(i).vindices[1] + 0] -
            model->vertices[3 * T(i).vindices[0] + 0];
        u[1] = model->vertices[3 * T(i).vindices[1] + 1] -
            model->vertices[3 * T(i).vindices[0] + 1];
        u[2] = model->vertices[3 * T(i).vindices[1] + 2] -
            model->vertices[3 * T(i).vindices[0] + 2];
        
        v[0] = model->vertices[3 * T(i).vindices[2] + 0] -
            model->vertices[3 * T(i).vindices[0] + 0];
        v[1] = model->vertices[3 * T(i).vindices[2] + 1] -
            model->vertices[3 * T(i).vindices[0] + 1];
        v[2] = model->vertices[3 * T(i).vindices[2] + 2] -
            model->vertices[3 * T(i).vindices[0] + 2];
        
        glmCross(u, v, &model->facetnorms[3 * (i+1)]);
        glmNormalize(&model->facetnorms[3 * (i+1)]);
    }
}

/* glmVertexNormals: Generates smooth vertex normals for a model.
 * First builds a list of all the triangles each vertex is in.   Then
 * loops through each vertex in the the list averaging all the facet
 * normals of the triangles each vertex is in.   Finally, sets the
 * normal index in the triangle for the vertex to the generated smooth
 * normal.   If the dot product of a facet normal and the facet normal
 * associated with the first triangle in the list of triangles the
 * current vertex is in is greater than the cosine of the angle
 * parameter to the function, that facet normal is not added into the
 * average normal calculation and the corresponding vertex is given
 * the facet normal.  This tends to preserve hard edges.  The angle to
 * use depends on the model, but 90 degrees is usually a good start.
 *
 * model - initialized GLMmodel structure
 * angle - maximum angle (in degrees) to smooth across
 */
GLvoid
glmVertexNormals(GLMmodel* model, GLfloat angle)
{
    GLMnode*    node;
    GLMnode*    tail;
    GLMnode** members;
    GLfloat*    normals;
    GLuint  numnormals;
    GLfloat average[3];
    GLfloat dot, cos_angle;
    GLuint  i, avg;
    
    assert(model);
    assert(model->facetnorms);
    
    /* calculate the cosine of the angle (in degrees) */
    cos_angle = cos(angle * M_PI / 180.0);
    
    /* nuke any previous normals */
    if (model->normals)
        free(model->normals);
    
    /* allocate space for new normals */
    model->numnormals = model->numtriangles * 3; /* 3 normals per triangle */
    model->normals = (GLfloat*)malloc(sizeof(GLfloat)* 3* (model->numnormals+1));
    
    /* allocate a structure that will hold a linked list of triangle
    indices for each vertex */
    members = (GLMnode**)malloc(sizeof(GLMnode*) * (model->numvertices + 1));
    for (i = 1; i <= model->numvertices; i++)
        members[i] = NULL;
    
    /* for every triangle, create a node for each vertex in it */
    for (i = 0; i < model->numtriangles; i++) {
        node = (GLMnode*)malloc(sizeof(GLMnode));
        node->index = i;
        node->next  = members[T(i).vindices[0]];
        members[T(i).vindices[0]] = node;
        
        node = (GLMnode*)malloc(sizeof(GLMnode));
        node->index = i;
        node->next  = members[T(i).vindices[1]];
        members[T(i).vindices[1]] = node;
        
        node = (GLMnode*)malloc(sizeof(GLMnode));
        node->index = i;
        node->next  = members[T(i).vindices[2]];
        members[T(i).vindices[2]] = node;
    }
    
    /* calculate the average normal for each vertex */
    numnormals = 1;
    for (i = 1; i <= model->numvertices; i++) {
    /* calculate an average normal for this vertex by averaging the
        facet normal of every triangle this vertex is in */
        node = members[i];
        if (!node)
            fprintf(stderr, "glmVertexNormals(): vertex w/o a triangle\n");
        average[0] = 0.0; average[1] = 0.0; average[2] = 0.0;
        avg = 0;
        while (node) {
        /* only average if the dot product of the angle between the two
        facet normals is greater than the cosine of the threshold
        angle -- or, said another way, the angle between the two
            facet normals is less than (or equal to) the threshold angle */
            dot = glmDot(&model->facetnorms[3 * T(node->index).findex],
                &model->facetnorms[3 * T(members[i]->index).findex]);
            if (dot > cos_angle) {
                node->averaged = GL_TRUE;
                average[0] += model->facetnorms[3 * T(node->index).findex + 0];
                average[1] += model->facetnorms[3 * T(node->index).findex + 1];
                average[2] += model->facetnorms[3 * T(node->index).findex + 2];
                avg = 1;            /* we averaged at least one normal! */
            } else {
                node->averaged = GL_FALSE;
            }
            node = node->next;
        }
        
        if (avg) {
            /* normalize the averaged normal */
            glmNormalize(average);
            
            /* add the normal to the vertex normals list */
            model->normals[3 * numnormals + 0] = average[0];
            model->normals[3 * numnormals + 1] = average[1];
            model->normals[3 * numnormals + 2] = average[2];
            avg = numnormals;
            numnormals++;
        }
        
        /* set the normal of this vertex in each triangle it is in */
        node = members[i];
        while (node) 
		{
			if (node->index==204543)
				{
					printf("");
				}
            if (node->averaged) {
				
                /* if this node was averaged, use the average normal */
                if (T(node->index).vindices[0] == i)
                    T(node->index).nindices[0] = avg;
                else if (T(node->index).vindices[1] == i)
                    T(node->index).nindices[1] = avg;
                else if (T(node->index).vindices[2] == i)
                    T(node->index).nindices[2] = avg;
            } else {
				
                /* if this node wasn't averaged, use the facet normal */
                model->normals[3 * numnormals + 0] = 
                    model->facetnorms[3 * T(node->index).findex + 0];
                model->normals[3 * numnormals + 1] = 
                    model->facetnorms[3 * T(node->index).findex + 1];
                model->normals[3 * numnormals + 2] = 
                    model->facetnorms[3 * T(node->index).findex + 2];
                if (T(node->index).vindices[0] == i)
                    T(node->index).nindices[0] = numnormals;
                else if (T(node->index).vindices[1] == i)
                    T(node->index).nindices[1] = numnormals;
                else if (T(node->index).vindices[2] == i)
                    T(node->index).nindices[2] = numnormals;
                numnormals++;
            }
            node = node->next;
        }
    }
    
    model->numnormals = numnormals - 1;
    
    /* free the member information */
    for (i = 1; i <= model->numvertices; i++) {
        node = members[i];
        while (node) {
            tail = node;
            node = node->next;
            free(tail);
        }
    }
    free(members);
    
    /* pack the normals array (we previously allocated the maximum
    number of normals that could possibly be created (numtriangles *
    3), so get rid of some of them (usually alot unless none of the
    facet normals were averaged)) */
    normals = model->normals;
    model->normals = (GLfloat*)malloc(sizeof(GLfloat)* 3* (model->numnormals+1));
    for (i = 1; i <= model->numnormals; i++) {
        model->normals[3 * i + 0] = normals[3 * i + 0];
        model->normals[3 * i + 1] = normals[3 * i + 1];
        model->normals[3 * i + 2] = normals[3 * i + 2];
    }
    free(normals);
}

GLvoid
glmLinearTexture(GLMmodel* model)
{
    GLMgroup *group;
    GLfloat dimensions[3];
    GLfloat x, y, scalefactor;
    GLuint i;
    
    assert(model);
    
    if (model->texcoords)
        free(model->texcoords);
    model->numtexcoords = model->numvertices;
    model->texcoords=(GLfloat*)malloc(sizeof(GLfloat)*2*(model->numtexcoords+1));
    
    glmDimensions(model, dimensions);
    scalefactor = 2.0 / 
        glmAbs(glmMax(glmMax(dimensions[0], dimensions[1]), dimensions[2]));
    
    /* do the calculations */
    for(i = 1; i <= model->numvertices; i++) {
        x = model->vertices[3 * i + 0] * scalefactor;
        y = model->vertices[3 * i + 2] * scalefactor;
        model->texcoords[2 * i + 0] = (x + 1.0) / 2.0;
        model->texcoords[2 * i + 1] = (y + 1.0) / 2.0;
    }
    
    /* go through and put texture coordinate indices in all the triangles */
    group = model->groups;
    while(group) {
        for(i = 0; i < group->numtriangles; i++) {
            T(group->triangles[i]).tindices[0] = T(group->triangles[i]).vindices[0];
            T(group->triangles[i]).tindices[1] = T(group->triangles[i]).vindices[1];
            T(group->triangles[i]).tindices[2] = T(group->triangles[i]).vindices[2];
        }    
        group = group->next;
    }
    
#if 0
    printf("glmLinearTexture(): generated %d linear texture coordinates\n",
        model->numtexcoords);
#endif
}

/* glmSpheremapTexture: Generates texture coordinates according to a
 * spherical projection of the texture map.  Sometimes referred to as
 * spheremap, or reflection map texture coordinates.  It generates
 * these by using the normal to calculate where that vertex would map
 * onto a sphere.  Since it is impossible to map something flat
 * perfectly onto something spherical, there is distortion at the
 * poles.  This particular implementation causes the poles along the X
 * axis to be distorted.
 *
 * model - pointer to initialized GLMmodel structure
 */
GLvoid
glmSpheremapTexture(GLMmodel* model)
{
    GLMgroup* group;
    GLfloat theta, phi, rho, x, y, z, r;
    GLuint i;
    
    assert(model);
    assert(model->normals);
    
    if (model->texcoords)
        free(model->texcoords);
    model->numtexcoords = model->numnormals;
    model->texcoords=(GLfloat*)malloc(sizeof(GLfloat)*2*(model->numtexcoords+1));
    
    for (i = 1; i <= model->numnormals; i++) {
        z = model->normals[3 * i + 0];  /* re-arrange for pole distortion */
        y = model->normals[3 * i + 1];
        x = model->normals[3 * i + 2];
        r = sqrt((x * x) + (y * y));
        rho = sqrt((r * r) + (z * z));
        
        if(r == 0.0) {
            theta = 0.0;
            phi = 0.0;
        } else {
            if(z == 0.0)
                phi = 3.14159265 / 2.0;
            else
                phi = acos(z / rho);
            
            if(y == 0.0)
                theta = 3.141592365 / 2.0;
            else
                theta = asin(y / r) + (3.14159265 / 2.0);
        }
        
        model->texcoords[2 * i + 0] = theta / 3.14159265;
        model->texcoords[2 * i + 1] = phi / 3.14159265;
    }
    
    /* go through and put texcoord indices in all the triangles */
    group = model->groups;
    while(group) {
        for (i = 0; i < group->numtriangles; i++) {
            T(group->triangles[i]).tindices[0] = T(group->triangles[i]).nindices[0];
            T(group->triangles[i]).tindices[1] = T(group->triangles[i]).nindices[1];
            T(group->triangles[i]).tindices[2] = T(group->triangles[i]).nindices[2];
        }
        group = group->next;
    }
}

/* glmDelete: Deletes a GLMmodel structure.
 *
 * model - initialized GLMmodel structure
 */
GLvoid
glmDelete(GLMmodel* model)
{
    GLMgroup* group;
    GLuint i;
    
    assert(model);
    
    if (model->pathname)     free(model->pathname);
    if (model->mtllibname) free(model->mtllibname);
    if (model->vertices)     free(model->vertices);
    if (model->normals)  free(model->normals);
    if (model->texcoords)  free(model->texcoords);
    if (model->facetnorms) free(model->facetnorms);
    if (model->triangles)  free(model->triangles);
    if (model->materials) {
        for (i = 0; i < model->nummaterials; i++)
            free(model->materials[i].name);
        free(model->materials);
    }
    if (model->textures) {
        for (i = 0; i < model->numtextures; i++) {
            free(model->textures[i].name);
            glDeleteTextures(1,&model->textures[i].id);
        }
        free(model->textures);
    }

    while(model->groups) {
        group = model->groups;
        model->groups = model->groups->next;
        free(group->name);
        free(group->triangles);
        free(group);
    }
    
    free(model);
}

/* glmReadOBJ: Reads a model description from a Wavefront .OBJ file.
 * Returns a pointer to the created object which should be free'd with
 * glmDelete().
 *
 * filename - name of the file containing the Wavefront .OBJ format data.  
 */

GLMmodel* glmReadOBJ(char* filename)
{
	return glmReadOBJ(filename,0);
}
GLMmodel* glmReadOBJ(char* filename,mycallback *call)
{
    GLMmodel* model;
    FILE*   file;
	//if (call) call->loadcallback(0,"Loading Models...");
    /* open the file */
    file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "glmReadOBJ() failed: can't open data file \"%s\".\n",
            filename);
        exit(1);
    }
    
    /* allocate a new model */
    model = (GLMmodel*)malloc(sizeof(GLMmodel));
    model->pathname    = strdup(filename);
    model->mtllibname    = NULL;
    model->numvertices   = 0;
    model->vertices    = NULL;
    model->numnormals    = 0;
    model->normals     = NULL;
    model->numtexcoords  = 0;
    model->texcoords       = NULL;
    model->numfacetnorms = 0;
    model->facetnorms    = NULL;
    model->numtriangles  = 0;
    model->triangles       = NULL;
    model->nummaterials  = 0;
    model->materials       = NULL;
    model->numtextures  = 0;
    model->textures       = NULL;
    model->numgroups       = 0;
    model->groups      = NULL;
    model->position[0]   = 0.0;
    model->position[1]   = 0.0;
    model->position[2]   = 0.0;
    
    /* make a first pass through the file to get a count of the number
    of vertices, normals, texcoords & triangles */
    glmFirstPass(model, file, call);
    
    /* allocate memory */
    model->vertices = (GLfloat*)malloc(sizeof(GLfloat) *
        3 * (model->numvertices + 1));
    model->triangles = (GLMtriangle*)malloc(sizeof(GLMtriangle) *
        model->numtriangles);
    if (model->numnormals) {
        model->normals = (GLfloat*)malloc(sizeof(GLfloat) *
            3 * (model->numnormals + 1));
    }
    if (model->numtexcoords) {
        model->texcoords = (GLfloat*)malloc(sizeof(GLfloat) *
            2 * (model->numtexcoords + 1));
    }
    
    /* rewind to beginning of file and read in the data this pass */
    rewind(file);
    
    glmSecondPass(model, file,call);
    
    /* close the file */
    fclose(file);
    
    return model;
}

/* glmWriteOBJ: Writes a model description in Wavefront .OBJ format to
 * a file.
 *
 * model - initialized GLMmodel structure
 * filename - name of the file to write the Wavefront .OBJ format data to
 * mode  - a bitwise or of values describing what is written to the file
 *             GLM_NONE     -  render with only vertices
 *             GLM_FLAT     -  render with facet normals
 *             GLM_SMOOTH   -  render with vertex normals
 *             GLM_TEXTURE  -  render with texture coords
 *             GLM_COLOR    -  render with colors (color material)
 *             GLM_MATERIAL -  render with materials
 *             GLM_COLOR and GLM_MATERIAL should not both be specified.  
 *             GLM_FLAT and GLM_SMOOTH should not both be specified.  
 */
GLvoid
glmWriteOBJ(GLMmodel* model, char* filename, GLuint mode)
{
    GLuint  i;
    FILE*   file;
    GLMgroup* group;  
    assert(model);
    
    /* do a bit of warning */
    if (mode & GLM_FLAT && !model->facetnorms) {
        printf("glmWriteOBJ() warning: flat normal output requested "
            "with no facet normals defined.\n");
        mode &= ~GLM_FLAT;
    }
    if (mode & GLM_SMOOTH && !model->normals) {
        printf("glmWriteOBJ() warning: smooth normal output requested "
            "with no normals defined.\n");
        mode &= ~GLM_SMOOTH;
    }
    if (mode & GLM_TEXTURE && !model->texcoords) {
        printf("glmWriteOBJ() warning: texture coordinate output requested "
            "with no texture coordinates defined.\n");
        mode &= ~GLM_TEXTURE;
    }
    if (mode & GLM_FLAT && mode & GLM_SMOOTH) {
        printf("glmWriteOBJ() warning: flat normal output requested "
            "and smooth normal output requested (using smooth).\n");
        mode &= ~GLM_FLAT;
    }
    if (mode & GLM_COLOR && !model->materials) {
        printf("glmWriteOBJ() warning: color output requested "
            "with no colors (materials) defined.\n");
        mode &= ~GLM_COLOR;
    }
    if (mode & GLM_MATERIAL && !model->materials) {
        printf("glmWriteOBJ() warning: material output requested "
            "with no materials defined.\n");
        mode &= ~GLM_MATERIAL;
    }
    if (mode & GLM_COLOR && mode & GLM_MATERIAL) {
        printf("glmWriteOBJ() warning: color and material output requested "
            "outputting only materials.\n");
        mode &= ~GLM_COLOR;
    }
    
    
    /* open the file */
    file = fopen(filename, "w");
    if (!file) {
        fprintf(stderr, "glmWriteOBJ() failed: can't open file \"%s\" to write.\n",
            filename);
        exit(1);
    }
    
    /* spit out a header */
    fprintf(file, "#  \n");
    fprintf(file, "#  Wavefront OBJ generated by GLM library\n");
    fprintf(file, "#  \n");
    fprintf(file, "#  GLM library\n");
    fprintf(file, "#  Nate Robins\n");
    fprintf(file, "#  ndr@pobox.com\n");
    fprintf(file, "#  http://www.pobox.com/~ndr\n");
    fprintf(file, "#  \n");
    
    if (mode & GLM_MATERIAL && model->mtllibname) {
        fprintf(file, "\nmtllib %s\n\n", model->mtllibname);
        glmWriteMTL(model, filename, model->mtllibname);
    }
    
    /* spit out the vertices */
    fprintf(file, "\n");
    fprintf(file, "# %d vertices\n", model->numvertices);
    for (i = 1; i <= model->numvertices; i++) {
        fprintf(file, "v %f %f %f\n", 
            model->vertices[3 * i + 0],
            model->vertices[3 * i + 1],
            model->vertices[3 * i + 2]);
    }
    
    /* spit out the smooth/flat normals */
    if (mode & GLM_SMOOTH) {
        fprintf(file, "\n");
        fprintf(file, "# %d normals\n", model->numnormals);
        for (i = 1; i <= model->numnormals; i++) {
            fprintf(file, "vn %f %f %f\n", 
                model->normals[3 * i + 0],
                model->normals[3 * i + 1],
                model->normals[3 * i + 2]);
        }
    } else if (mode & GLM_FLAT) {
        fprintf(file, "\n");
        fprintf(file, "# %d normals\n", model->numfacetnorms);
        for (i = 1; i <= model->numnormals; i++) {
            fprintf(file, "vn %f %f %f\n", 
                model->facetnorms[3 * i + 0],
                model->facetnorms[3 * i + 1],
                model->facetnorms[3 * i + 2]);
        }
    }
    
    /* spit out the texture coordinates */
    if (mode & GLM_TEXTURE) {
        fprintf(file, "\n");
        fprintf(file, "# %d texcoords\n", model->texcoords);
        for (i = 1; i <= model->numtexcoords; i++) {
            fprintf(file, "vt %f %f\n", 
                model->texcoords[2 * i + 0],
                model->texcoords[2 * i + 1]);
        }
    }
    
    fprintf(file, "\n");
    fprintf(file, "# %d groups\n", model->numgroups);
    fprintf(file, "# %d faces (triangles)\n", model->numtriangles);
    fprintf(file, "\n");
    
    group = model->groups;
    while(group) {
        fprintf(file, "g %s\n", group->name);
        if (mode & GLM_MATERIAL)
            fprintf(file, "usemtl %s\n", model->materials[group->material].name);
        for (i = 0; i < group->numtriangles; i++) {
            if (mode & GLM_SMOOTH && mode & GLM_TEXTURE) {
                fprintf(file, "f %d/%d/%d %d/%d/%d %d/%d/%d\n",
                    T(group->triangles[i]).vindices[0], 
                    T(group->triangles[i]).nindices[0], 
                    T(group->triangles[i]).tindices[0],
                    T(group->triangles[i]).vindices[1],
                    T(group->triangles[i]).nindices[1],
                    T(group->triangles[i]).tindices[1],
                    T(group->triangles[i]).vindices[2],
                    T(group->triangles[i]).nindices[2],
                    T(group->triangles[i]).tindices[2]);
            } else if (mode & GLM_FLAT && mode & GLM_TEXTURE) {
                fprintf(file, "f %d/%d %d/%d %d/%d\n",
                    T(group->triangles[i]).vindices[0],
                    T(group->triangles[i]).findex,
                    T(group->triangles[i]).vindices[1],
                    T(group->triangles[i]).findex,
                    T(group->triangles[i]).vindices[2],
                    T(group->triangles[i]).findex);
            } else if (mode & GLM_TEXTURE) {
                fprintf(file, "f %d/%d %d/%d %d/%d\n",
                    T(group->triangles[i]).vindices[0],
                    T(group->triangles[i]).tindices[0],
                    T(group->triangles[i]).vindices[1],
                    T(group->triangles[i]).tindices[1],
                    T(group->triangles[i]).vindices[2],
                    T(group->triangles[i]).tindices[2]);
            } else if (mode & GLM_SMOOTH) {
                fprintf(file, "f %d//%d %d//%d %d//%d\n",
                    T(group->triangles[i]).vindices[0],
                    T(group->triangles[i]).nindices[0],
                    T(group->triangles[i]).vindices[1],
                    T(group->triangles[i]).nindices[1],
                    T(group->triangles[i]).vindices[2], 
                    T(group->triangles[i]).nindices[2]);
            } else if (mode & GLM_FLAT) {
                fprintf(file, "f %d//%d %d//%d %d//%d\n",
                    T(group->triangles[i]).vindices[0], 
                    T(group->triangles[i]).findex,
                    T(group->triangles[i]).vindices[1],
                    T(group->triangles[i]).findex,
                    T(group->triangles[i]).vindices[2],
                    T(group->triangles[i]).findex);
            } else {
                fprintf(file, "f %d %d %d\n",
                    T(group->triangles[i]).vindices[0],
                    T(group->triangles[i]).vindices[1],
                    T(group->triangles[i]).vindices[2]);
            }
        }
        fprintf(file, "\n");
        group = group->next;
    }
    
    fclose(file);
}

/* glmDraw: Renders the model to the current OpenGL context using the
 * mode specified.
 *
 * model - initialized GLMmodel structure
 * mode  - a bitwise OR of values describing what is to be rendered.
 *             GLM_NONE     -  render with only vertices
 *             GLM_FLAT     -  render with facet normals
 *             GLM_SMOOTH   -  render with vertex normals
 *             GLM_TEXTURE  -  render with texture coords
 *             GLM_COLOR    -  render with colors (color material)
 *             GLM_MATERIAL -  render with materials
 *             GLM_COLOR and GLM_MATERIAL should not both be specified.  
 *             GLM_FLAT and GLM_SMOOTH should not both be specified.  
 */

GLvoid glmDraw(GLMmodel* model, GLuint mode)
{
	glmDraw(model,mode,0);
}
GLvoid glmDraw(GLMmodel* model, GLuint mode,char *drawonly)
{
    static GLuint i;
    static GLMgroup* group;
    static GLMtriangle* triangle;
    static GLMmaterial* material;
    GLuint IDTextura;
    
    assert(model);
    assert(model->vertices);
    
    /* do a bit of warning */
    if (mode & GLM_FLAT && !model->facetnorms) {
        printf("glmDraw() warning: flat render mode requested "
            "with no facet normals defined.\n");
        mode &= ~GLM_FLAT;
    }
    if (mode & GLM_SMOOTH && !model->normals) {
        printf("glmDraw() warning: smooth render mode requested "
            "with no normals defined.\n");
        mode &= ~GLM_SMOOTH;
    }
    if (mode & GLM_TEXTURE && !model->texcoords) {
        printf("glmDraw() warning: texture render mode requested "
            "with no texture coordinates defined.\n");
        mode &= ~GLM_TEXTURE;
    }
    if (mode & GLM_FLAT && mode & GLM_SMOOTH) {
        printf("glmDraw() warning: flat render mode requested "
            "and smooth render mode requested (using smooth).\n");
        mode &= ~GLM_FLAT;
    }
    if (mode & GLM_COLOR && !model->materials) {
        printf("glmDraw() warning: color render mode requested "
            "with no materials defined.\n");
        mode &= ~GLM_COLOR;
    }
    if (mode & GLM_MATERIAL && !model->materials) {
        printf("glmDraw() warning: material render mode requested "
            "with no materials defined.\n");
        mode &= ~GLM_MATERIAL;
    }
    if (mode & GLM_COLOR && mode & GLM_MATERIAL) {
        printf("glmDraw() warning: color and material render mode requested "
            "using only material mode.\n");
        mode &= ~GLM_COLOR;
    }
    if (mode & GLM_COLOR)
        glEnable(GL_COLOR_MATERIAL);
    else if (mode & GLM_MATERIAL)
        glDisable(GL_COLOR_MATERIAL);
    if (mode & GLM_TEXTURE) {
        glEnable(GL_TEXTURE_2D);
        glTexEnvf(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_MODULATE);
    }
    /* perhaps this loop should be unrolled into material, color, flat,
       smooth, etc. loops?  since most cpu's have good branch prediction
       schemes (and these branches will always go one way), probably
       wouldn't gain too much?  */
    
    IDTextura = -1;
    group = model->groups;
    while (group) 
	{
		if (drawonly)
			if (strcmp(group->name,drawonly))
			{
				group=group->next;
				continue;
			}		
		
		material = &model->materials[group->material];
		if (material)
			IDTextura = material->IDTextura;
		else IDTextura=-1;
        
		if (mode & GLM_MATERIAL) 
		{            
            glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT, material->ambient);
            glMaterialfv(GL_FRONT_AND_BACK, GL_DIFFUSE, material->diffuse);
            glMaterialfv(GL_FRONT_AND_BACK, GL_SPECULAR, material->specular);
            glMaterialf(GL_FRONT_AND_BACK, GL_SHININESS, material->shininess);
        }

		if (mode & GLM_TEXTURE) 
		{				
			if(IDTextura == -1)
				glBindTexture(GL_TEXTURE_2D, 0);
			else
				glBindTexture(GL_TEXTURE_2D, model->textures[IDTextura].id);		
		}
        
        if (mode & GLM_COLOR) {
            glColor3fv(material->diffuse);
        }
        
        glBegin(GL_TRIANGLES);
        for (i = 0; i < group->numtriangles; i++) {
            triangle = &T(group->triangles[i]);
#ifdef DebugVisibleSurfaces
			if (!triangle->visible) continue;
#endif
            if (mode & GLM_FLAT)
                glNormal3fv(&model->facetnorms[3 * triangle->findex]);
            
            if (mode & GLM_SMOOTH)
                glNormal3fv(&model->normals[3 * triangle->nindices[0]]);
            if (mode & GLM_TEXTURE)
                glTexCoord2fv(&model->texcoords[2 * triangle->tindices[0]]);
            glVertex3fv(&model->vertices[3 * triangle->vindices[0]]);
            
            if (mode & GLM_SMOOTH)
                glNormal3fv(&model->normals[3 * triangle->nindices[1]]);
            if (mode & GLM_TEXTURE)
	    {
                //if (IDTextura==-1) printf("Warning: GLM_TEXTURE este on dar nu este setata nici o textura in material!");
                glTexCoord2fv(&model->texcoords[2 * triangle->tindices[1]]);
            }
            glVertex3fv(&model->vertices[3 * triangle->vindices[1]]);
            
            if (mode & GLM_SMOOTH)
                glNormal3fv(&model->normals[3 * triangle->nindices[2]]);
            if (mode & GLM_TEXTURE)
                glTexCoord2fv(&model->texcoords[2 * triangle->tindices[2]]);
            glVertex3fv(&model->vertices[3 * triangle->vindices[2]]);
            
        }
        glEnd();
        
        group = group->next;
    }
}

/* glmList: Generates and returns a display list for the model using
 * the mode specified.
 *
 * model - initialized GLMmodel structure
 * mode  - a bitwise OR of values describing what is to be rendered.
 *             GLM_NONE     -  render with only vertices
 *             GLM_FLAT     -  render with facet normals
 *             GLM_SMOOTH   -  render with vertex normals
 *             GLM_TEXTURE  -  render with texture coords
 *             GLM_COLOR    -  render with colors (color material)
 *             GLM_MATERIAL -  render with materials
 *             GLM_COLOR and GLM_MATERIAL should not both be specified.  
 * GLM_FLAT and GLM_SMOOTH should not both be specified.  
 */
GLuint
glmList(GLMmodel* model, GLuint mode)
{
    GLuint list;
    
    list = glGenLists(1);
    glNewList(list, GL_COMPILE);
    glmDraw(model, mode);
    glEndList();
    
    return list;
}

/* glmWeld: eliminate (weld) vectors that are within an epsilon of
 * each other.
 *
 * model   - initialized GLMmodel structure
 * epsilon     - maximum difference between vertices
 *               ( 0.00001 is a good start for a unitized model)
 *
 */
GLvoid
glmWeld(GLMmodel* model, GLfloat epsilon)
{
    GLfloat* vectors;
    GLfloat* copies;
    GLuint   numvectors;
    GLuint   i;
    
    /* vertices */
    numvectors = model->numvertices;
    vectors  = model->vertices;
    copies = glmWeldVectors(vectors, &numvectors, epsilon);
    
#if 0
    printf("glmWeld(): %d redundant vertices.\n", 
        model->numvertices - numvectors - 1);
#endif
    
    for (i = 0; i < model->numtriangles; i++) {
        T(i).vindices[0] = (GLuint)vectors[3 * T(i).vindices[0] + 0];
        T(i).vindices[1] = (GLuint)vectors[3 * T(i).vindices[1] + 0];
        T(i).vindices[2] = (GLuint)vectors[3 * T(i).vindices[2] + 0];
    }
    
    /* free space for old vertices */
    free(vectors);
    
    /* allocate space for the new vertices */
    model->numvertices = numvectors;
    model->vertices = (GLfloat*)malloc(sizeof(GLfloat) * 
        3 * (model->numvertices + 1));
    
    /* copy the optimized vertices into the actual vertex list */
    for (i = 1; i <= model->numvertices; i++) {
        model->vertices[3 * i + 0] = copies[3 * i + 0];
        model->vertices[3 * i + 1] = copies[3 * i + 1];
        model->vertices[3 * i + 2] = copies[3 * i + 2];
    }
    
    free(copies);
}

/* glmReadPPM: read a PPM raw (type P6) file.  The PPM file has a header
 * that should look something like:
 *
 *    P6
 *    # comment
 *    width height max_value
 *    rgbrgbrgb...
 *
 * where "P6" is the magic cookie which identifies the file type and
 * should be the only characters on the first line followed by a
 * carriage return.  Any line starting with a # mark will be treated
 * as a comment and discarded.   After the magic cookie, three integer
 * values are expected: width, height of the image and the maximum
 * value for a pixel (max_value must be < 256 for PPM raw files).  The
 * data section consists of width*height rgb triplets (one byte each)
 * in binary format (i.e., such as that written with fwrite() or
 * equivalent).
 *
 * The rgb data is returned as an array of unsigned chars (packed
 * rgb).  The malloc()'d memory should be free()'d by the caller.  If
 * an error occurs, an error message is sent to stderr and NULL is
 * returned.
 *
 * filename   - name of the .ppm file.
 * width      - will contain the width of the image on return.
 * height     - will contain the height of the image on return.
 *
 */
GLubyte* 
glmReadPPM(char* filename, int* width, int* height)
{
    FILE* fp;
    int i, w, h, d;
    unsigned char* image;
    char head[70];          /* max line <= 70 in PPM (per spec). */
    
    fp = fopen(filename, "rb");
    if (!fp) {
        perror(filename);
        return NULL;
    }
    
    /* grab first two chars of the file and make sure that it has the
       correct magic cookie for a raw PPM file. */
    fgets(head, 70, fp);
    if (strncmp(head, "P6", 2)) {
        fprintf(stderr, "%s: Not a raw PPM file\n", filename);
        return NULL;
    }
    
    /* grab the three elements in the header (width, height, maxval). */
    i = 0;
    while(i < 3) {
        fgets(head, 70, fp);
        if (head[0] == '#')     /* skip comments. */
            continue;
        if (i == 0)
            i += sscanf(head, "%d %d %d", &w, &h, &d);
        else if (i == 1)
            i += sscanf(head, "%d %d", &h, &d);
        else if (i == 2)
            i += sscanf(head, "%d", &d);
    }
    
    /* grab all the image data in one fell swoop. */
    image = (unsigned char*)malloc(sizeof(unsigned char)*w*h*3);
    fread(image, sizeof(unsigned char), w*h*3, fp);
    fclose(fp);
    
    *width = w;
    *height = h;
    return image;
}

#if 0
/* normals */
if (model->numnormals) {
    numvectors = model->numnormals;
    vectors  = model->normals;
    copies = glmOptimizeVectors(vectors, &numvectors);
    
    printf("glmOptimize(): %d redundant normals.\n", 
        model->numnormals - numvectors);
    
    for (i = 0; i < model->numtriangles; i++) {
        T(i).nindices[0] = (GLuint)vectors[3 * T(i).nindices[0] + 0];
        T(i).nindices[1] = (GLuint)vectors[3 * T(i).nindices[1] + 0];
        T(i).nindices[2] = (GLuint)vectors[3 * T(i).nindices[2] + 0];
    }
    
    /* free space for old normals */
    free(vectors);
    
    /* allocate space for the new normals */
    model->numnormals = numvectors;
    model->normals = (GLfloat*)malloc(sizeof(GLfloat) * 
        3 * (model->numnormals + 1));
    
    /* copy the optimized vertices into the actual vertex list */
    for (i = 1; i <= model->numnormals; i++) {
        model->normals[3 * i + 0] = copies[3 * i + 0];
        model->normals[3 * i + 1] = copies[3 * i + 1];
        model->normals[3 * i + 2] = copies[3 * i + 2];
    }
    
    free(copies);
}

/* texcoords */
if (model->numtexcoords) {
    numvectors = model->numtexcoords;
    vectors  = model->texcoords;
    copies = glmOptimizeVectors(vectors, &numvectors);
    
    printf("glmOptimize(): %d redundant texcoords.\n", 
        model->numtexcoords - numvectors);
    
    for (i = 0; i < model->numtriangles; i++) {
        for (j = 0; j < 3; j++) {
            T(i).tindices[j] = (GLuint)vectors[3 * T(i).tindices[j] + 0];
        }
    }
    
    /* free space for old texcoords */
    free(vectors);
    
    /* allocate space for the new texcoords */
    model->numtexcoords = numvectors;
    model->texcoords = (GLfloat*)malloc(sizeof(GLfloat) * 
        2 * (model->numtexcoords + 1));
    
    /* copy the optimized vertices into the actual vertex list */
    for (i = 1; i <= model->numtexcoords; i++) {
        model->texcoords[2 * i + 0] = copies[2 * i + 0];
        model->texcoords[2 * i + 1] = copies[2 * i + 1];
    }
    
    free(copies);
}
#endif

#if 0
/* look for unused vertices */
/* look for unused normals */
/* look for unused texcoords */
for (i = 1; i <= model->numvertices; i++) {
    for (j = 0; j < model->numtriangles; i++) {
        if (T(j).vindices[0] == i || 
            T(j).vindices[1] == i || 
            T(j).vindices[1] == i)
            break;
    }
}
#endif


```

## Source file: stdafx.h

{
  "path": "stdafx.h",
  "blob_sha": "d057a651787ec7826e53de4e6ed5a498bf56d7be",
  "sha256": "b8f55f14e37131b1ad10ad7b731fd87ab1ad437c7088c39607fcc176dd622f05",
  "lines": 16,
  "bytes": 363,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/HRLTY/DenseFace3D/blob/a01e59bc6f9bc19fc0c7c4a4576cd2b536c93f38/stdafx.h",
  "function_signature_candidates": []
}

```text

// stdafx.h : include file for standard system include files,
// or project specific include files that are used frequently, but
// are changed infrequently
//

#pragma once

#include "targetver.h"
#ifdef _WIN32
#include <windows.h>
#endif
#include <stdio.h>
//#include <tchar.h>


// TODO: reference additional headers your program requires here


```
