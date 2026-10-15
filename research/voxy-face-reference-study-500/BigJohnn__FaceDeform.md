# Facial rendering source scan: BigJohnn/FaceDeform

Repository: https://github.com/BigJohnn/FaceDeform

Pinned source revision: b6afcbd09be61c6bf3eff9f435d3735a8fcd5358

Discovery topic: face deformation

Description: Please download MUCT dataset to test the face2face deformation, or use your own pictures.(need to pick points with annotation_with_opencv tool.)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"deform": ["FaceDeform.cpp"], "face": ["FaceDeform.cpp", "bearface.yml", "bearface_vertices.yml", "face_detector.cpp", "face_detector.hpp", "face_tracker.cpp", "face_tracker.hpp"]}

## Source file: face_tracker.cpp

{
  "path": "face_tracker.cpp",
  "blob_sha": "6fcf286ce9c31583028fc9e6ef8fec0f110f2bb8",
  "sha256": "6ac60a0be8809c42c8ed194c3790f99bac46f22fa3855d08fbbb4064b8ed3f7e",
  "lines": 243,
  "bytes": 8860,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 25
  },
  "url": "https://github.com/BigJohnn/FaceDeform/blob/b6afcbd09be61c6bf3eff9f435d3735a8fcd5358/face_tracker.cpp",
  "function_signature_candidates": []
}

```text

/*****************************************************************************
*   Non-Rigid Face Tracking
******************************************************************************
*   by Jason Saragih, 5th Dec 2012
*   http://jsaragih.org/
******************************************************************************
*   Ch6 of the book "Mastering OpenCV with Practical Computer Vision Projects"
*   Copyright Packt Publishing 2012.
*   http://www.packtpub.com/cool-projects-with-opencv/book
*****************************************************************************/
/*
  face_tracker: face tracking classes
  Jason Saragih (2012)
*/
#include "opencv_hotshots/ft/face_tracker.hpp"
#include "opencv_hotshots/ft/ft.hpp"
#include <iostream>
#include "stdio.h"      // For 'sprintf()'
#define fl at<float>
//==============================================================================
//==============================================================================
//==============================================================================
//=============================== fps_timer ====================================
//==============================================================================
//==============================================================================
//==============================================================================
void 
fps_timer::
increment()
{
  if(fnum >= 29) {
    t_end = cv::getTickCount();
    fps = 30.0/(float(t_end-t_start)/getTickFrequency()); 
    t_start = t_end; fnum = 0;
  }else fnum += 1;
}
//==============================================================================
void 
fps_timer::
reset(){
t_start = cv::getTickCount(); fps = 0; fnum = 0;
}
//==============================================================================
void 
fps_timer::
display_fps(Mat &im,
        Point p)
{
  char str[256]; Point pt; if(p.y < 0)pt = Point(10,im.rows-20); else pt = p;
  sprintf(str,"%d frames/sec",(int)cvRound(fps)); string text = str;
  putText(im,text,pt,FONT_HERSHEY_SIMPLEX,0.5,Scalar::all(255));
}
//==============================================================================
//==============================================================================
//==============================================================================
//========================== face_tracker_params ===============================
//==============================================================================
//==============================================================================
//==============================================================================
face_tracker_params::
face_tracker_params()
{
  ssize.resize(3); 
  ssize[0] = Size(21,21); ssize[1] = Size(11,11); ssize[2] = Size(5,5);
  robust = false; itol = 20; ftol = 1e-3;
  scaleFactor = 1.1; minNeighbours = 2; minSize = Size(30,30);
}
//==============================================================================
void 
face_tracker_params::
write(FileStorage &fs) const
{
  assert(fs.isOpened()); fs << "{";
  fs << "nlevels" << int(ssize.size());
  for(int i = 0; i < int(ssize.size()); i++){ char str[256]; const char* ss;
    sprintf(str,"w %d",i); ss = str; fs << ss << ssize[i].width;
    sprintf(str,"h %d",i); ss = str; fs << ss << ssize[i].height;
  }
  fs << "robust" << robust
     << "itol" << itol
     << "ftol" << ftol
     << "scaleFactor" << scaleFactor
     << "minNeighbours" << minNeighbours
     << "minWidth" << minSize.width
     << "minHeight" << minSize.height
     << "}";
}
//==============================================================================
void 
face_tracker_params::
read(const FileNode& node)
{
  assert(node.type() == FileNode::MAP);
  int n; node["nlevels"] >> n; ssize.resize(n);
  for(int i = 0; i < n; i++){ char str[256]; const char* ss;
    sprintf(str,"w %d",i); ss = str; node[ss] >> ssize[i].width;
    sprintf(str,"h %d",i); ss = str; node[ss] >> ssize[i].height;
  }
  node["robust"] >> robust;
  node["itol"] >> itol;
  node["ftol"] >> ftol;
  node["scaleFactor"] >> scaleFactor;
  node["minNeighbours"] >> minNeighbours;
  node["minWidth"] >> minSize.width;
  node["minHeight"] >> minSize.height;
}
//==============================================================================
void 
write(FileStorage& fs, 
      const string&, 
      const face_tracker_params& x)
{
  x.write(fs);
}
//==============================================================================
void 
read(const FileNode& node, 
     face_tracker_params& x,
     const face_tracker_params& d)
{
  if(node.empty())x = d; else x.read(node);
}
//==============================================================================
face_tracker_params 
load_face_tracker_params(const char* fname)
{
  face_tracker_params x; FileStorage f(fname,FileStorage::READ);
  f["face_tracker_params"] >> x; f.release(); return x;
}
//==============================================================================
void 
save_face_tracker_params(const char* fname,
             const face_tracker_params& x)
{
  FileStorage f(fname,FileStorage::WRITE);
  f << "face_tracker_params" << x; f.release();
}
//==============================================================================
//==============================================================================
//==============================================================================
//============================== face_tracker ==================================
//==============================================================================
//==============================================================================
//==============================================================================
int
face_tracker::
track(const Mat &im,const face_tracker_params &p)
{
  //convert image to greyscale
  Mat gray; if(im.channels()==1)gray = im; else cvtColor(im,gray,CV_RGB2GRAY);

  //initialise
  if(!tracking)
    points = detector.detect(gray,p.scaleFactor,p.minNeighbours,p.minSize);
  if((int)points.size() != smodel.npts())return 0;

  //fit
  for(int level = 0; level < int(p.ssize.size()); level++)
    points = this->fit(gray,points,p.ssize[level],p.robust,p.itol,p.ftol);

  //set tracking flag and increment timer
  tracking = true; timer.increment();  return 1;
}
//==============================================================================
void
face_tracker::
draw(Mat &im,
     const Scalar pts_color,
     const Scalar con_color)
{
  int n = points.size(); if(n == 0)return;
  for(int i = 0; i < smodel.C.rows; i++){
    int j = smodel.C.at<int>(i,0),k = smodel.C.at<int>(i,1);
    line(im,points[j],points[k],con_color,1);
  }
  for(int i = 0; i < n; i++)circle(im,points[i],1,pts_color,2,CV_AA);
}
//==============================================================================
vector<Point2f>
face_tracker::
fit(const Mat &image,
    const vector<Point2f> &init,
    const Size ssize,
    const bool robust,
    const int itol,
    const float ftol)
{
  int n = smodel.npts(); 
  assert((int(init.size())==n) && (pmodel.n_patches()==n));
  smodel.calc_params(init); vector<Point2f> pts = smodel.calc_shape();

  //find facial features in image around current estimates
  vector<Point2f> peaks = pmodel.calc_peaks(image,pts,ssize);

  //optimise
  if(!robust){
    smodel.calc_params(peaks); //compute shape model parameters        
    pts = smodel.calc_shape(); //update shape
  }else{
    Mat weight(n,1,CV_32F),weight_sort(n,1,CV_32F);
    vector<Point2f> pts_old = pts;
    for(int iter = 0; iter < itol; iter++){
      //compute robust weight
      for(int i = 0; i < n; i++)weight.fl(i) = norm(pts[i] - peaks[i]);
      cv::sort(weight,weight_sort,CV_SORT_EVERY_COLUMN|CV_SORT_ASCENDING);
      double var = 1.4826*weight_sort.fl(n/2); if(var < 0.1)var = 0.1;
      pow(weight,2,weight); weight *= -0.5/(var*var); cv::exp(weight,weight); 

      //compute shape model parameters    
      smodel.calc_params(peaks,weight);
      
      //update shape
      pts = smodel.calc_shape();
      
      //check for convergence
      float v = 0; for(int i = 0; i < n; i++)v += norm(pts[i]-pts_old[i]);
      if(v < ftol)break; else pts_old = pts;
    }
  }return pts;
}
//==============================================================================
void 
face_tracker::
write(FileStorage &fs) const
{
  assert(fs.isOpened()); 
  fs << "{"
     << "detector" << detector
     << "smodel"   << smodel
     << "pmodel"   << pmodel
     << "}";
}
//==============================================================================
void 
face_tracker::
read(const FileNode& node)
{
  assert(node.type() == FileNode::MAP);
  node["detector"] >> detector;
  node["smodel"]   >> smodel;
  node["pmodel"]   >> pmodel;
}
//==============================================================================


```

## Source file: face_tracker.hpp

{
  "path": "face_tracker.hpp",
  "blob_sha": "e99678edf9f73f89cd43e12228c2b31edc087c46",
  "sha256": "c9e1f9aed7f14b8a383ab29cbbbf4151f56400eadb52cee19518c4833e694de7",
  "lines": 98,
  "bytes": 4316,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 15
  },
  "url": "https://github.com/BigJohnn/FaceDeform/blob/b6afcbd09be61c6bf3eff9f435d3735a8fcd5358/face_tracker.hpp",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "void increment();                        //increment timer index"
    },
    {
      "line": 32,
      "text": "void reset();                            //reset timer"
    }
  ]
}

```text

/*****************************************************************************
*   Non-Rigid Face Tracking
******************************************************************************
*   by Jason Saragih, 5th Dec 2012
*   http://jsaragih.org/
******************************************************************************
*   Ch6 of the book "Mastering OpenCV with Practical Computer Vision Projects"
*   Copyright Packt Publishing 2012.
*   http://www.packtpub.com/cool-projects-with-opencv/book
*****************************************************************************/
/*
  face_tracker: face tracking classes
  Jason Saragih (2012)
*/
#ifndef _FT_FACE_TRACKER_HPP_
#define _FT_FACE_TRACKER_HPP_
#include "opencv_hotshots/ft/patch_model.hpp"
#include "opencv_hotshots/ft/shape_model.hpp"
#include "opencv_hotshots/ft/face_detector.hpp"
//==============================================================================
class fps_timer{                           //frames/second timer for tracking
public:
  int64 t_start;                           //start time
  int64 t_end;                             //end time
  float fps;                               //current frames/sec
  int fnum;                                //number of frames since @t_start

  fps_timer(){this->reset();}              //default constructor

  void increment();                        //increment timer index

  void reset();                            //reset timer

  void 
  display_fps(Mat &im,                     //image to display FPS on
          Point p = Point(-1,-1));     //bottom left corner of text
};
//==============================================================================
class face_tracker_params{                 //face tracking parameters
public:
  vector<Size> ssize;                      //search region size/level
  bool robust;                             //use robust fitting?
  int itol;                                //maximum number of iterations to try
  float ftol;                              //convergence tolerance
  float scaleFactor;                       //OpenCV Cascade detector parameters
  int minNeighbours;                       //...
  Size minSize;                            //...

  face_tracker_params();                   //sets default parameter settings

  void 
  write(FileStorage &fs) const;            //file storage object to write to

  void 
  read(const FileNode& node);              //file storage node to read from
};
//==============================================================================
class face_tracker{                        //face tracking class
public:
  bool tracking;                           //are we in tracking mode?
  fps_timer timer;                         //frames/second timer
  vector<Point2f> points;                  //current tracked points
  face_detector detector;                  //detector for initialisation
  shape_model smodel;                      //shape model
  patch_models pmodel;                     //feature detectors
  
  face_tracker(){tracking = false;}

  int                                      //0 = failure
  track(const Mat &im,                     //image containing face
    const face_tracker_params &p =     //fitting parameters 
    face_tracker_params());            //default tracking parameters

  void 
  reset(){                                 //reset tracker 
    tracking = false; timer.reset();
  }
  void
  draw(Mat &im,
       const Scalar pts_color = CV_RGB(255,0,0),
       const Scalar con_color = CV_RGB(0,255,0));
  void 
  write(FileStorage &fs) const;            //file storage object to write to

  void 
  read(const FileNode& node);              //file storage node to read from

protected:
  vector<Point2f>                          //points for fitted face in image
  fit(const Mat &image,                    //image containing face
      const vector<Point2f> &init,         //initial point estimates
      const Size ssize = Size(21,21),      //search region size
      const bool robust = false,           //use robust fitting?
      const int itol = 10,                 //maximum number of iterations to try
      const float ftol = 1e-3);            //convergence tolerance
};
//==============================================================================
#endif


```

## Source file: face_detector.cpp

{
  "path": "face_detector.cpp",
  "blob_sha": "7abdd5d753db82522a38fee438e40e47b5c3759e",
  "sha256": "728cc58d10cc12afe8c049e4be0574d168a8e966e90c1441c4ec225c03337291",
  "lines": 180,
  "bytes": 6854,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 33
  },
  "url": "https://github.com/BigJohnn/FaceDeform/blob/b6afcbd09be61c6bf3eff9f435d3735a8fcd5358/face_detector.cpp",
  "function_signature_candidates": []
}

```text

/*****************************************************************************
*   Non-Rigid Face Tracking
******************************************************************************
*   by Jason Saragih, 5th Dec 2012
*   http://jsaragih.org/
******************************************************************************
*   Ch6 of the book "Mastering OpenCV with Practical Computer Vision Projects"
*   Copyright Packt Publishing 2012.
*   http://www.packtpub.com/cool-projects-with-opencv/book
*****************************************************************************/
/*
  face_detector: Shape initializer for face tracking
  Jason Saragih (2012)
*/
#include "opencv_hotshots/ft/face_detector.hpp"
#include "opencv_hotshots/ft/ft.hpp"
#include <opencv2/imgproc/imgproc.hpp>
#include <opencv2/highgui/highgui.hpp>
#include <iostream>
#define fl at<float>
//==============================================================================
vector<Point2f>
face_detector::
detect(const Mat &im,
       const float scaleFactor,
       const int minNeighbours,
       const Size minSize)
{
  //convert image to greyscale
  Mat gray; if(im.channels()==1)gray = im; else cvtColor(im,gray,CV_RGB2GRAY);

  //detect faces
  vector<Rect> faces; Mat eqIm; equalizeHist(gray,eqIm);
  detector.detectMultiScale(eqIm,faces,scaleFactor,minNeighbours,0
                |CV_HAAR_FIND_BIGGEST_OBJECT
                |CV_HAAR_SCALE_IMAGE,minSize);
  if(faces.size() < 1){return vector<Point2f>();}
  
  //predict face placement
  Rect R = faces[0]; Vec3f scale = detector_offset*R.width;
  int n = reference.rows/2; vector<Point2f> p(n);
  for(int i = 0; i < n; i++){
    p[i].x = scale[2]*reference.fl(2*i  ) + R.x + 0.5 * R.width  + scale[0];
    p[i].y = scale[2]*reference.fl(2*i+1) + R.y + 0.5 * R.height + scale[1];
  }return p;
}
//==============================================================================
void
face_detector::
train(ft_data &data,
      const string fname,
      const Mat &ref,
      const bool mirror,
      const bool visi,
      const float frac,
      const float scaleFactor,
      const int minNeighbours,
      const Size minSize)
{
  detector.load(fname.c_str()); detector_fname = fname; reference = ref.clone();
  vector<float> xoffset(0),yoffset(0),zoffset(0);
  for(int i = 0; i < data.n_images(); i++){
    Mat im = data.get_image(i,0); if(im.empty())continue;
    vector<Point2f> p = data.get_points(i,false); int n = p.size();
    Mat pt = Mat(p).reshape(1,2*n);
    vector<Rect> faces; Mat eqIm; equalizeHist(im,eqIm);
    detector.detectMultiScale(eqIm,faces,scaleFactor,minNeighbours,0
                  |CV_HAAR_FIND_BIGGEST_OBJECT
                  |CV_HAAR_SCALE_IMAGE,minSize);
    if(faces.size() >= 1){
      if(visi){
    Mat I; cvtColor(im,I,CV_GRAY2RGB);
    for(int i = 0; i < n; i++)circle(I,p[i],1,CV_RGB(0,255,0),2,CV_AA);
    rectangle(I,faces[0].tl(),faces[0].br(),CV_RGB(255,0,0),3);
    imshow("face detector training",I); waitKey(10); 
      }
      //check if enough points are in detected rectangle
      if(this->enough_bounded_points(pt,faces[0],frac)){
    Point2f center = this->center_of_mass(pt); float w = faces[0].width;
    xoffset.push_back((center.x - (faces[0].x+0.5*faces[0].width ))/w);
    yoffset.push_back((center.y - (faces[0].y+0.5*faces[0].height))/w);
    zoffset.push_back(this->calc_scale(pt)/w);
      }
    }
    if(mirror){
      im = data.get_image(i,1); if(im.empty())continue;
      p = data.get_points(i,true);
      pt = Mat(p).reshape(1,2*n);
      equalizeHist(im,eqIm);
      detector.detectMultiScale(eqIm,faces,scaleFactor,minNeighbours,0
                  |CV_HAAR_FIND_BIGGEST_OBJECT
                |CV_HAAR_SCALE_IMAGE,minSize);
      if(faces.size() >= 1){
    if(visi){
      Mat I; cvtColor(im,I,CV_GRAY2RGB);
      for(int i = 0; i < n; i++)circle(I,p[i],1,CV_RGB(0,255,0),2,CV_AA);
      rectangle(I,faces[0].tl(),faces[0].br(),CV_RGB(255,0,0),3);
      imshow("face detector training",I); waitKey(10);
    }
    //check if enough points are in detected rectangle
    if(this->enough_bounded_points(pt,faces[0],frac)){
      Point2f center = this->center_of_mass(pt); float w = faces[0].width;
      xoffset.push_back((center.x - (faces[0].x+0.5*faces[0].width ))/w);
      yoffset.push_back((center.y - (faces[0].y+0.5*faces[0].height))/w);
      zoffset.push_back(this->calc_scale(pt)/w);
    }
      }
    }
  }
  //choose median value
  Mat X = Mat(xoffset),Xsort,Y = Mat(yoffset),Ysort,Z = Mat(zoffset),Zsort;
  cv::sort(X,Xsort,CV_SORT_EVERY_COLUMN|CV_SORT_ASCENDING); int nx = Xsort.rows;
  cv::sort(Y,Ysort,CV_SORT_EVERY_COLUMN|CV_SORT_ASCENDING); int ny = Ysort.rows;
  cv::sort(Z,Zsort,CV_SORT_EVERY_COLUMN|CV_SORT_ASCENDING); int nz = Zsort.rows;
  detector_offset = Vec3f(Xsort.fl(nx/2),Ysort.fl(ny/2),Zsort.fl(nz/2)); return;
}
//==============================================================================
bool
face_detector::
enough_bounded_points(const Mat &pts,
              const Rect R,
              const float frac)
{
  int n = pts.rows/2,m = 0;
  for(int i = 0; i < n; i++){
    if((pts.fl(2*i  ) >= R.x) && (pts.fl(2*i  ) <= R.x + R.width) &&
       (pts.fl(2*i+1) >= R.y) && (pts.fl(2*i+1) <= R.y + R.height))m++;
  }
  if(float(m)/n >= frac)return true; else return false;
}
//==============================================================================
Point2f
face_detector::
center_of_mass(const Mat &pts)
{
  float mx = 0,my = 0; int n = pts.rows/2;
  for(int i = 0; i < n; i++){
    mx += pts.fl(2*i); my += pts.fl(2*i+1);
  }return Point2f(mx/n,my/n);
}
//==============================================================================
float 
face_detector::
calc_scale(const Mat &pts)
{
  Point2f c = this->center_of_mass(pts); int n = pts.rows/2;
  Mat p(2*n,1,CV_32F);
  for(int i = 0; i < n; i++){
    p.fl(2*i  ) = pts.fl(2*i  ) - c.x;
    p.fl(2*i+1) = pts.fl(2*i+1) - c.y;
  }return reference.dot(p)/reference.dot(reference);
}
//==============================================================================
void 
face_detector::
write(FileStorage &fs) const
{
  assert(fs.isOpened()); 
  fs << "{"
     << "fname"     << detector_fname
     << "x offset"  << detector_offset[0]
     << "y offset"  << detector_offset[1]
     << "z offset"  << detector_offset[2]
     << "reference" << reference
     << "}";
}
//==============================================================================
void 
face_detector::
read(const FileNode& node)
{
  assert(node.type() == FileNode::MAP);
  node["fname"]     >> detector_fname;
  node["x offset"]  >> detector_offset[0];
  node["y offset"]  >> detector_offset[1];
  node["z offset"]  >> detector_offset[2];
  node["reference"] >> reference;
  detector.load(detector_fname.c_str());
}
//==============================================================================


```

## Source file: face_detector.hpp

{
  "path": "face_detector.hpp",
  "blob_sha": "c2b96473979aa57e5dff7a31da8be9680b94b27a",
  "sha256": "a2df185d3777161552f2369f9f09a91b3f1952f471210a0e395d2f33cf5460d1",
  "lines": 63,
  "bytes": 2954,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 8
  },
  "url": "https://github.com/BigJohnn/FaceDeform/blob/b6afcbd09be61c6bf3eff9f435d3735a8fcd5358/face_detector.hpp",
  "function_signature_candidates": []
}

```text

/*****************************************************************************
*   Non-Rigid Face Tracking
******************************************************************************
*   by Jason Saragih, 5th Dec 2012
*   http://jsaragih.org/
******************************************************************************
*   Ch6 of the book "Mastering OpenCV with Practical Computer Vision Projects"
*   Copyright Packt Publishing 2012.
*   http://www.packtpub.com/cool-projects-with-opencv/book
*****************************************************************************/
/*
  face_detector: Shape initializer for face tracking
  Jason Saragih (2012)
*/
#ifndef _FT_FACE_DETECTOR_HPP_
#define _FT_FACE_DETECTOR_HPP_
#include <opencv2/objdetect/objdetect.hpp>
#include "opencv_hotshots/ft/ft_data.hpp"
//==============================================================================
class face_detector{                       //face detector for initialisation
public:
  string detector_fname;                   //file containing cascade classifier
  Vec3f detector_offset;                   //offset from center of detection
  Mat reference;                           //reference shape
  CascadeClassifier detector;              //face detector

  vector<Point2f>                          //points for detected face in image
  detect(const Mat &im,                    //image containing face
     const float scaleFactor = 1.1,    //scale increment
     const int minNeighbours = 2,      //minimum neighbourhood size
     const Size minSize = Size(30,30));//minimum detection window size
  
  void
  train(ft_data &data,                     //training data
    const string fname,                //cascade detector
    const Mat &ref,                    //reference shape
    const bool mirror = false,         //mirror data?
    const bool visi = false,           //visualise training?
    const float frac = 0.8,            //fraction of points in detected rect
    const float scaleFactor = 1.1,     //scale increment
    const int minNeighbours = 2,       //minimum neighbourhood size
    const Size minSize = Size(30,30)); //minimum detection window size

  void 
  write(FileStorage &fs) const;            //file storage object to write to

  void 
  read(const FileNode& node);              //file storage node to read from

protected:
  bool                                     //are enough points bounded?
  enough_bounded_points(const Mat &pts,    //points to evaluate
            const Rect R,      //bounding rectangle
            const float frac); //fraction of points bounded

  Point2f                                  //center of mass
  center_of_mass(const Mat &pts);          //[x1;y1;...;xn;yn]

  float                                    //scaling from @reference to @pts
  calc_scale(const Mat &pts);              //[x1;y1;...;xn;yn]
};
//==============================================================================
#endif


```
