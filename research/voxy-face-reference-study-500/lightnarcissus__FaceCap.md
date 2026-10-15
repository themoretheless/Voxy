# Facial rendering source scan: lightnarcissus/FaceCap

Repository: https://github.com/lightnarcissus/FaceCap

Pinned source revision: b69665315584c9cacdeb3d34a856543833ca1325

Discovery topic: facial animation

Description: markerless facial animation capture

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyerefl01_1008.jpeg", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyerefl01_1008.jpeg.meta", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyerefl01_1008.jpg", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyerefl01_1008.jpg.meta", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyes01_1007.jpeg", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyes01_1007.jpeg.meta", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyes01_1007.jpg", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_eyes01_1007.jpg.meta"], "mouth": ["FaceCapUnity/Assets/gen_male.fbm/RyEddie_mouth_1005.jpeg", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_mouth_1005.jpeg.meta", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_mouth_1005.jpg", "FaceCapUnity/Assets/gen_male.fbm/RyEddie_mouth_1005.jpg.meta"], "cornea": ["FaceCapUnity/Assets/Cornea.mat", "FaceCapUnity/Assets/Cornea.mat.meta"], "face": ["FaceCapUnity/Assets/AsyncIO.dll.meta", "FaceCapUnity/Assets/ClientObject.cs", "FaceCapUnity/Assets/ClientObject.cs.meta", "FaceCapUnity/Assets/Cornea.mat", "FaceCapUnity/Assets/Cornea.mat.meta", "FaceCapUnity/Assets/FuseModelTest.fbm.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Diffuse.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Diffuse.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Gloss.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Gloss.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Normal.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Normal.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Opacity.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Opacity.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Specular.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Body_Specular.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Diffuse.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Diffuse.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Gloss.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Gloss.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Normal.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Normal.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Opacity.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Opacity.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Specular.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Bottom_Specular.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Top_Diffuse.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Top_Diffuse.png.meta", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Top_Gloss.png", "FaceCapUnity/Assets/FuseModelTest.fbm/FuseModelTest_Top_Gloss.png.meta"]}

## Source file: Windows/face_landmark_detection.cpp

{
  "path": "Windows/face_landmark_detection.cpp",
  "blob_sha": "3dd0ba2accbe6811c2aa34947f4a3369dc4608d9",
  "sha256": "607706a1d1305cca76c6550fb7ad2d04ed7c892afd4c9b0e71521fdb5fc0d1f4",
  "lines": 313,
  "bytes": 11960,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 27,
    "lip": 2
  },
  "url": "https://github.com/lightnarcissus/FaceCap/blob/b69665315584c9cacdeb3d34a856543833ca1325/Windows/face_landmark_detection.cpp",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "void print_usage() {"
    },
    {
      "line": 56,
      "text": "int main(int argc, char** argv) {"
    }
  ]
}

```text


//CREDIT TO zhucebuliaolongchuan https://github.com/zhucebuliaolongchuan/avatar-facial-landmark-detection
/*
landmarks in images read from a webcam and points that drew by the program.
 */
#include <dlib/opencv.h>
#include <opencv2/highgui/highgui.hpp>
#include <opencv2/core/core.hpp>
#include <opencv2/opencv.hpp>
#include <opencv2/imgproc.hpp>
#include <dlib/image_processing/frontal_face_detector.h>
#include <dlib/image_processing/render_face_detections.h>
#include <dlib/image_processing.h>
#include <dlib/gui_widgets.h>
#include <stdio.h>
#include <opencv2/video/tracking.hpp>

using namespace cv;
using namespace dlib;

void print_usage() {
    std::cout << "Usage:" << std::endl;
    std::cout << "./face_landmark_detection [path/to/shape_predictor_68_face_landmarks.dat]" << std::endl;
}

// calculate the variance between the current points and last points
double cal_dist_diff(std::vector<cv::Point2f> curPoints, std::vector<cv::Point2f> lastPoints) {
    double variance = 0.0;
    double sum = 0.0;
    std::vector<double> diffs;
    if (curPoints.size() == lastPoints.size()) {
        for (int i = 0; i < curPoints.size(); i++) {
            double diff = std::sqrt(std::pow(curPoints[i].x - lastPoints[i].x, 2.0) + std::pow(curPoints[i].y - lastPoints[i].y, 2.0));
            sum += diff;
            diffs.push_back(diff);
        }
        double mean = sum / diffs.size();
        for (int i = 0; i < curPoints.size(); i++) {
            variance += std::pow(diffs[i] - mean, 2);
        }
        return variance / diffs.size();
    }
    return variance;
}

double calc_diff(cv::Point2f calibPoint, cv::Point2f trackedPoint)
{
	double variance = 0.0;
	double sum = 0.0;
	std::vector<double> diffs;
	double diff = std::sqrt(std::pow(trackedPoint.x - calibPoint.x, 2.0) + std::pow(trackedPoint.y - calibPoint.y, 2.0));
	return diff;
}

// Shows the results that combined with Optical Flow, Kalman Filter and Dlib based on the speed of face movement
int main(int argc, char** argv) {
    try {
		/*
        if (argc > 2) {
            print_usage();
            return 0;
        }
		*/
        std::cout << "Press q to exit." << std::endl;

        cv::VideoCapture cap(0);

        if (!cap.isOpened()) {
            std::cerr << "Unable to connect to camera" << std::endl;
            return 1;
        }

        // Load face detection and pose estimation models.
        frontal_face_detector detector = get_frontal_face_detector();
        shape_predictor pose_model;
		/*
        if (argc == 2) {
            deserialize(argv[1]) >> pose_model;
        } else {
            deserialize("../../data/shape_predictor_68_face_landmarks.dat") >> pose_model;
        }*/
		std::cout << "about to load face landmarks model"<< std::endl;
		deserialize("C:/Users/Ansh/Documents/shape_predictor_68_face_landmarks.dat") >> pose_model;
        // Initialize the points of last frame
        std::vector<cv::Point2f> last_object;
        for (int i = 0; i < 68; ++i) {
            last_object.push_back(cv::Point2f(0.0, 0.0));
        }

        double scaling = 0.5;
        int flag = -1;
        int count = 0;
        bool redetected = true;
        // std::vector<dlib::rectangle> faces;

        // Initialize measurement points
        std::vector<cv::Point2f> kalman_points;
        for (int i = 0; i < 68; i++) {
            kalman_points.push_back(cv::Point2f(0.0, 0.0));
        }

        // Initialize prediction points
        std::vector<cv::Point2f> predict_points;
        for (int i = 0; i < 68; i++) {
            predict_points.push_back(cv::Point2f(0.0, 0.0));
        }

        // Kalman Filter Setup (68 Points Test)
        const int stateNum = 272;
        const int measureNum = 136;

        KalmanFilter KF(stateNum, measureNum, 0);
        Mat state(stateNum, 1, CV_32FC1);
        Mat processNoise(stateNum, 1, CV_32F);
        Mat measurement = Mat::zeros(measureNum, 1, CV_32F);

        // Generate a matrix randomly
        randn(state, Scalar::all(0), Scalar::all(0.0));

        // Generate the Measurement Matrix
        KF.transitionMatrix = Mat::zeros(stateNum, stateNum, CV_32F);
        for (int i = 0; i < stateNum; i++) {
            for (int j = 0; j < stateNum; j++) {
                if (i == j || (j - measureNum) == i) {
                    KF.transitionMatrix.at<float>(i, j) = 1.0;
                } else {
                    KF.transitionMatrix.at<float>(i, j) = 0.0;
                }   
            }
        }

        //!< measurement matrix (H) Measurement Model  
        setIdentity(KF.measurementMatrix);
  
        //!< process noise covariance matrix (Q)  
        setIdentity(KF.processNoiseCov, Scalar::all(1e-5));
          
        //!< measurement noise covariance matrix (R)  
        setIdentity(KF.measurementNoiseCov, Scalar::all(1e-1));

        //!< priori error estimate covariance matrix (P'(k)): P'(k)=A*P(k-1)*At + Q)*/  A代表F: transitionMatrix  
        setIdentity(KF.errorCovPost, Scalar::all(1));
    
        randn(KF.statePost, Scalar::all(0), Scalar::all(0.0));

        // Initialize Optical Flow
        cv::Mat prevgray, gray;
        std::vector<cv::Point2f> prevTrackPts;
        std::vector<cv::Point2f> nextTrackPts;

		cv::Point2f calibPt;
		bool calibrated = false;
		int targetLandmark = 59;
        for (int i = 0; i < 68; i++) {
            prevTrackPts.push_back(cv::Point2f(0, 0));
        }

        // Grab and process frames until the main window is closed by the user.
        while(true) {
            // Grab a frame
            cv::Mat raw;
            cap >> raw;
            
            // Resize
            cv::Mat tmp;
            cv::resize(raw, tmp, cv::Size(), scaling, scaling);

            //Flip
            cv::Mat temp;
            cv::flip(tmp, temp, 1);

            // Turn OpenCV's Mat into something dlib can deal with.  Note that this just
            // wraps the Mat object, it doesn't copy anything.  So cimg is only valid as
            // long as temp is valid.  Also don't do anything to temp that would cause it
            // to reallocate the memory which stores the image as that will make cimg
            // contain dangling pointers.  This basically means you shouldn't modify temp
            // while using cimg.
            cv_image<bgr_pixel> cimg(temp);

            // Detect faces, load the vertexes as vector 
            // if (redetected == true || faces.size() == 0) {
            //     faces = detector(cimg);
            // }
            std::vector<dlib::rectangle> faces = detector(cimg);

            // Find the pose of each face.
            std::vector<full_object_detection> shapes;
            for (unsigned long i = 0; i < faces.size(); ++i) {
                shapes.push_back(pose_model(cimg, faces[i]));
            }

            // We cannot modify temp so we clone a new one
         //   cv::Mat face = temp.clone();
         //   cv::Mat face_2 = temp.clone();
         //   cv::Mat face_3 = temp.clone();
            cv::Mat frame = temp.clone();
            cv::Mat face_5 = temp.clone();

            // This function is to combine the optical flow + kalman filter + dlib to deteck and track the facial landmark
            if (flag == -1) {
                cvtColor(frame, prevgray, COLOR_BGR2GRAY);
                const full_object_detection& d = shapes[0];
                for (int i = 0; i < d.num_parts(); i++) {
                    prevTrackPts[i].x = d.part(i).x();
                    prevTrackPts[i].y = d.part(i).y();
                }
                flag = 1; 
            }

            // Update Kalman Filter Points
            if (shapes.size() == 1) {
                const full_object_detection& d = shapes[0];
                for (int i = 0; i < d.num_parts(); i++) {
                    kalman_points[i].x = d.part(i).x();
                    kalman_points[i].y = d.part(i).y();
                }
            }

            // Kalman Prediction
            Mat prediction = KF.predict();
            // std::vector<cv::Point2f> predict_points;
            for (int i = 0; i < 68; i++) {
                predict_points[i].x = prediction.at<float>(i * 2);
                predict_points[i].y = prediction.at<float>(i * 2 + 1);
            }

            // Optical Flow + Kalman Filter + Dlib based on the speed of face movement
            if (shapes.size() == 1) {
                cvtColor(frame, gray, COLOR_BGR2GRAY);
                if (prevgray.data) {
                    std::vector<uchar> status;
                    std::vector<float> err;
                    calcOpticalFlowPyrLK(prevgray, gray, prevTrackPts, nextTrackPts, status, err);
                 //   std::cout << "variance:" <<cal_dist_diff(prevTrackPts, nextTrackPts) << std::endl;

                    double diff = cal_dist_diff(prevTrackPts, nextTrackPts);
                    if (diff > 1.0) { // the threshold value here depends on the system, camera specs, etc
                         // if the face is moving so fast, use dlib to detect the face
                        const full_object_detection& d = shapes[0];
                     //   std::cout<< "DLIB" << std::endl;
                        for (int i = 0; i < d.num_parts(); i++) {
                            cv::circle(face_5, cv::Point2f(d.part(i).x(), d.part(i).y()), 2, cv::Scalar(0, 0, 255), -1);
                            nextTrackPts[i].x = d.part(i).x();
                            nextTrackPts[i].y = d.part(i).y();
                        }
                    } else if (diff <= 1.0 && diff > 0.005){
                        // In this case, use Optical Flow
                     //   std::cout<< "Optical Flow" << std::endl;
                        for (int i = 0; i < nextTrackPts.size(); i++) {
                            cv::circle(face_5, nextTrackPts[i], 2, cv::Scalar(255, 0, 0), -1);
                        }
                    } else {
                        // In this case, use Kalman Filter
                      //  std::cout<< "Kalman Filter" << std::endl;
                        for (int i = 0; i < predict_points.size(); i++) {
                            cv::circle(face_5, predict_points[i], 2, cv::Scalar(0, 255, 0), -1);
                            nextTrackPts[i].x = predict_points[i].x;
                            nextTrackPts[i].y = predict_points[i].y;
                        }
                        redetected = false;
                    }
                } else {
                    redetected = true;
                }
				std::cout << nextTrackPts.size() << std::endl;
                // previous points should be updated with the current points

				if (calibrated)
				{
					double diff=calc_diff(calibPt, nextTrackPts[targetLandmark]);
						std::cout << diff << std::endl;
					
				}
                std::swap(prevTrackPts, nextTrackPts);
                std::swap(prevgray, gray);
            } else {
                redetected = true;
            }

            // Update Measurement
            for (int i = 0; i < 136; i++) {
                if (i % 2 == 0) {
                    measurement.at<float>(i) = (float)kalman_points[i / 2].x;
                } else {
                    measurement.at<float>(i) = (float)kalman_points[(i - 1) / 2].y;
                }
            }

            // Update the Measurement Matrix
            measurement += KF.measurementMatrix * state;
            KF.correct(measurement);

            cv::imshow("KF+OF+DLIB", face_5);

            char key = cv::waitKey(1);
            if (key == 'q') {
                break;
            }
			else if(key == 'r')
			{
				std::cout << "about to calibrate" << std::endl;
				calibPt.x = nextTrackPts[targetLandmark].x;
				calibPt.y = nextTrackPts[targetLandmark].y;
				calibrated = true;
			}
        }
    } catch(serialization_error& e) {
        print_usage();
    } catch(std::exception& e) {
        std::cout << e.what() << std::endl;
    }
    return 0;
}


```

## Source file: FaceCapUnity/Assets/ClientObject.cs

{
  "path": "FaceCapUnity/Assets/ClientObject.cs",
  "blob_sha": "db75350b6653755042ff4b87c7ae5a8150b814d7",
  "sha256": "38bf4e4d24cc3da77aab14bf105d8afe5e48498d23ba5b215ac79adcbb61a551",
  "lines": 137,
  "bytes": 3672,
  "selection_score": 3,
  "keyword_line_counts": {
    "skin": 14,
    "blendshape": 10
  },
  "url": "https://github.com/lightnarcissus/FaceCap/blob/b69665315584c9cacdeb3d34a856543833ca1325/FaceCapUnity/Assets/ClientObject.cs",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "public delegate void MessageDelegate(string message);"
    },
    {
      "line": 19,
      "text": "private void ListenerWork()"
    },
    {
      "line": 39,
      "text": "public void Update()"
    },
    {
      "line": 61,
      "text": "public void Start()"
    },
    {
      "line": 67,
      "text": "public void Stop()"
    },
    {
      "line": 81,
      "text": "private void HandleMessage(string message)"
    },
    {
      "line": 116,
      "text": "private void Awake()"
    },
    {
      "line": 122,
      "text": "private void Start()"
    },
    {
      "line": 128,
      "text": "private void Update()"
    },
    {
      "line": 133,
      "text": "private void OnDestroy()"
    }
  ]
}

```text

﻿using System.Collections.Concurrent;
using System.Threading;
using NetMQ;
using UnityEngine;
using NetMQ.Sockets;

public class NetMqListener
{
    private readonly Thread _listenerWorker;

    private bool _listenerCancelled;

    public delegate void MessageDelegate(string message);

    private readonly MessageDelegate _messageDelegate;

    private readonly ConcurrentQueue<string> _messageQueue = new ConcurrentQueue<string>();

    private void ListenerWork()
    {
        AsyncIO.ForceDotNet.Force();
        using (var subSocket = new SubscriberSocket())
        {
            subSocket.Options.ReceiveHighWatermark = 1000;
            subSocket.Connect("tcp://localhost:12345");
            subSocket.Subscribe("");
            while (!_listenerCancelled)
            {
                string frameString;
                if (!subSocket.TryReceiveFrameString(out frameString)) continue;
                Debug.Log(frameString);
                _messageQueue.Enqueue(frameString);
            }
            subSocket.Close();
        }
        NetMQConfig.Cleanup();
    }

    public void Update()
    {
        while (!_messageQueue.IsEmpty)
        {
            string message;
            if (_messageQueue.TryDequeue(out message))
            {
                _messageDelegate(message);
            }
            else
            {
                break;
            }
        }
    }

    public NetMqListener(MessageDelegate messageDelegate)
    {
        _messageDelegate = messageDelegate;
        _listenerWorker = new Thread(ListenerWork);
    }

    public void Start()
    {
        _listenerCancelled = false;
        _listenerWorker.Start();
    }

    public void Stop()
    {
        _listenerCancelled = true;
        _listenerWorker.Join();
    }
}

public class ClientObject : MonoBehaviour
{
    private NetMqListener _netMqListener;

    SkinnedMeshRenderer skinnedMeshRenderer;
    Mesh skinnedMesh;

    private void HandleMessage(string message)
    {
        Debug.Log("handled message is " + message);
        var splittedStrings = message.Split(',');
        //UnityEngine.Debug.Log("the val is " + message.ToString());

        float val = float.Parse(splittedStrings[1]) * 20f;
        Debug.Log("val is " + val.ToString());
        if (splittedStrings[0] == "AU02")
        {
            skinnedMeshRenderer.SetBlendShapeWeight(8, val);
            skinnedMeshRenderer.SetBlendShapeWeight(9, val);
        }
        else if(splittedStrings[0]=="AU06")
        {
            skinnedMeshRenderer.SetBlendShapeWeight(41, val);
            skinnedMeshRenderer.SetBlendShapeWeight(42, val);
        }
        else if (splittedStrings[0] == "AU12")
        {
            skinnedMeshRenderer.SetBlendShapeWeight(41, val);
            skinnedMeshRenderer.SetBlendShapeWeight(42, val);
        }
        else if (splittedStrings[0] == "AU25")
        {
            skinnedMeshRenderer.SetBlendShapeWeight(35, val);
            //skinnedMeshRenderer.SetBlendShapeWeight(42, val);
        }
        else if (splittedStrings[0] == "AU45")
        {
            skinnedMeshRenderer.SetBlendShapeWeight(0, val);
            skinnedMeshRenderer.SetBlendShapeWeight(1, val);
        }
    }

    private void Awake()
    {

        skinnedMeshRenderer = GetComponent<SkinnedMeshRenderer>();
        skinnedMesh = GetComponent<SkinnedMeshRenderer>().sharedMesh;
    }
    private void Start()
    {
        _netMqListener = new NetMqListener(HandleMessage);
        _netMqListener.Start();
    }

    private void Update()
    {
        _netMqListener.Update();
    }

    private void OnDestroy()
    {
        _netMqListener.Stop();
    }
}


```

## Source file: FaceCapUnity/Assets/ServerObject.cs

{
  "path": "FaceCapUnity/Assets/ServerObject.cs",
  "blob_sha": "55de7cd5ab72013ae3683f64e6c5f6c17c39b2f6",
  "sha256": "c584e7e177cef77d21d2342cf47cf441b066ec2717a24d90a8afbcd13d14ad20",
  "lines": 93,
  "bytes": 2203,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/lightnarcissus/FaceCap/blob/b69665315584c9cacdeb3d34a856543833ca1325/FaceCapUnity/Assets/ServerObject.cs",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "private void ListenerWork()"
    },
    {
      "line": 51,
      "text": "public void Start()"
    },
    {
      "line": 57,
      "text": "public void Stop()"
    },
    {
      "line": 70,
      "text": "private void Start()"
    },
    {
      "line": 76,
      "text": "private void Update()"
    },
    {
      "line": 89,
      "text": "private void OnDestroy()"
    }
  ]
}

```text

﻿using System.Diagnostics;
using System.Threading;
using NetMQ;
using NetMQ.Sockets;
using UnityEngine;

public class NetMqPublisher
{
    private readonly Thread _listenerWorker;

    private bool _listenerCancelled;

    public delegate string MessageDelegate(string message);

    private readonly MessageDelegate _messageDelegate;

    private readonly Stopwatch _contactWatch;

    private const long ContactThreshold = 1000;

    public bool Connected;

    private void ListenerWork()
    {
        AsyncIO.ForceDotNet.Force();
        using (var server = new ResponseSocket())
        {
            server.Bind("tcp://*:12346");

            while (!_listenerCancelled)
            {
                Connected = _contactWatch.ElapsedMilliseconds < ContactThreshold;
                string message;
                if (!server.TryReceiveFrameString(out message)) continue;
                _contactWatch.Restart();
                var response = _messageDelegate(message);
                server.SendFrame(response);
            }
        }
        NetMQConfig.Cleanup();
    }

    public NetMqPublisher(MessageDelegate messageDelegate)
    {
        _messageDelegate = messageDelegate;
        _contactWatch = new Stopwatch();
        _contactWatch.Start();
        _listenerWorker = new Thread(ListenerWork);
    }

    public void Start()
    {
        _listenerCancelled = false;
        _listenerWorker.Start();
    }

    public void Stop()
    {
        _listenerCancelled = true;
        _listenerWorker.Join();
    }
}

public class ServerObject : MonoBehaviour
{
    public bool Connected;
    private NetMqPublisher _netMqPublisher;
    private string _response;

    private void Start()
    {
        _netMqPublisher = new NetMqPublisher(HandleMessage);
        _netMqPublisher.Start();
    }

    private void Update()
    {
        var position = transform.position;
//        _response = $"{position.x} {position.y} {position.z}";
        Connected = _netMqPublisher.Connected;
    }

    private string HandleMessage(string message)
    {
        // Not on main thread
        return _response;
    }

    private void OnDestroy()
    {
        _netMqPublisher.Stop();
    }
}


```

## Source file: FaceCapUnity/Assets/Plugins/Editor/JetBrains/Unity3DRider.cs

{
  "path": "FaceCapUnity/Assets/Plugins/Editor/JetBrains/Unity3DRider.cs",
  "blob_sha": "4b4cd10ecc3b7ac61d657ac0cdd38c7c50f5846b",
  "sha256": "9884622be5c63c5c6fc1be505472ba9f5db03da123fae4741ded859bb6539fd3",
  "lines": 1047,
  "bytes": 43424,
  "selection_score": 3,
  "keyword_line_counts": {
    "gum": 8,
    "shader": 1,
    "rig": 1,
    "face": 7
  },
  "url": "https://github.com/lightnarcissus/FaceCap/blob/b69665315584c9cacdeb3d34a856543833ca1325/FaceCapUnity/Assets/Plugins/Editor/JetBrains/Unity3DRider.cs",
  "function_signature_candidates": [
    {
      "line": 41,
      "text": "public static void OnGeneratedCSProjectFiles()"
    },
    {
      "line": 84,
      "text": "private static void UpgradeProjectFile(string projectFile)"
    },
    {
      "line": 101,
      "text": "private static void SetManuallyDefinedComilingSettings(string projectFile, XElement projectContentElement, XNamespace xmlns)"
    },
    {
      "line": 118,
      "text": "private static void ApplyManualCompilingSettings(string configFilePath, XElement projectContentElement, XNamespace xmlns)"
    },
    {
      "line": 158,
      "text": "private static void ApplyCustomDefines(string[] customDefines, XElement projectContentElement, XNamespace xmlns)"
    },
    {
      "line": 173,
      "text": "private static void ApplyAllowUnsafeBlocks(XElement projectContentElement, XNamespace xmlns)"
    },
    {
      "line": 179,
      "text": "private static bool IsPlayerProjectFile(string projectFile)"
    },
    {
      "line": 184,
      "text": "private static bool IsEditorProjectFile(string projectFile)"
    },
    {
      "line": 190,
      "text": "private static void FixTargetFrameworkVersion(XElement projectElement, XNamespace xmlns)"
    },
    {
      "line": 209,
      "text": "private static void SetLangVersion(XElement projectElement, XNamespace xmlns)"
    },
    {
      "line": 246,
      "text": "private static void SetXCodeDllReference(string name, XNamespace xmlns, XElement projectContentElement)"
    },
    {
      "line": 275,
      "text": "public static void Log(LoggingLevel level, string initialText)"
    },
    {
      "line": 400,
      "text": "private static void InitRiderPlugin()"
    },
    {
      "line": 413,
      "text": "private static void AddRiderToRecentlyUsedScriptApp(string userAppPath, string recentAppsKey)"
    },
    {
      "line": 429,
      "text": "private static void SetExternalScriptEditor(string path)"
    },
    {
      "line": 434,
      "text": "private static bool RiderPathExist(string path)"
    },
    {
      "line": 450,
      "text": "private static void InitializeEditorInstanceJson(string projectDirectory)"
    },
    {
      "line": 479,
      "text": "static bool OnOpenedAsset(int instanceID, int line)"
    },
    {
      "line": 521,
      "text": "private static bool DetectPortAndOpenFile(int line, string filePath, bool isWindows)"
    },
    {
      "line": 559,
      "text": "private static bool HttpOpenFile(int line, string filePath, bool isWindows, int port, WebClient client)"
    },
    {
      "line": 583,
      "text": "private static bool CallRider(string args)"
    },
    {
      "line": 615,
      "text": "private static void ActivateWindow()"
    },
    {
      "line": 670,
      "text": "static void MenuOpenProject()"
    },
    {
      "line": 680,
      "text": "static bool ValidateMenuOpenProject()"
    },
    {
      "line": 689,
      "text": "private static void SyncSolution()"
    },
    {
      "line": 704,
      "text": "static void RiderPreferencesItem()"
    },
    {
      "line": 792,
      "text": "public static int GetWindowProcessId(IntPtr hwnd)"
    },
    {
      "line": 838,
      "text": "internal static extern int SHGetFolderPath(IntPtr hwndOwner, int nFolder, IntPtr hToken, int dwFlags, StringBuilder lpszPath);"
    },
    {
      "line": 916,
      "text": "void GetPath([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cchMaxPath, out WIN32_FIND_DATAW pfd, SLGP_FLAGS fFlags);"
    },
    {
      "line": 920,
      "text": "void GetIDList(out IntPtr ppidl);"
    },
    {
      "line": 924,
      "text": "void SetIDList(IntPtr pidl);"
    },
    {
      "line": 928,
      "text": "void GetDescription([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cchMaxName);"
    },
    {
      "line": 932,
      "text": "void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);"
    },
    {
      "line": 936,
      "text": "void GetWorkingDirectory([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cchMaxPath);"
    },
    {
      "line": 940,
      "text": "void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);"
    },
    {
      "line": 944,
      "text": "void GetArguments([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cchMaxPath);"
    },
    {
      "line": 948,
      "text": "void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);"
    },
    {
      "line": 952,
      "text": "void GetHotkey(out short pwHotkey);"
    },
    {
      "line": 956,
      "text": "void SetHotkey(short wHotkey);"
    },
    {
      "line": 960,
      "text": "void GetShowCmd(out int piShowCmd);"
    },
    {
      "line": 964,
      "text": "void SetShowCmd(int iShowCmd);"
    },
    {
      "line": 968,
      "text": "void GetIconLocation([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cchIconPath, out int piIcon);"
    },
    {
      "line": 972,
      "text": "void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);"
    },
    {
      "line": 976,
      "text": "void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, int dwReserved);"
    },
    {
      "line": 980,
      "text": "void Resolve(IntPtr hwnd, SLR_FLAGS fFlags);"
    },
    {
      "line": 984,
      "text": "void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);"
    },
    {
      "line": 991,
      "text": "void GetClassID(out Guid pClassID);"
    },
    {
      "line": 1000,
      "text": "new void GetClassID(out Guid pClassID);"
    },
    {
      "line": 1003,
      "text": "int IsDirty();"
    },
    {
      "line": 1006,
      "text": "void Load([In, MarshalAs(UnmanagedType.LPWStr)] string pszFileName, uint dwMode);"
    }
  ]
}

```text

//------------------------------------------------------------------------------
// <auto-generated>
//     This code was generated by a tool.
//     Version: 2.0.0.0
//
//     Changes to this file may cause incorrect behavior and will be lost if
//     the code is regenerated.
// </auto-generated>
//------------------------------------------------------------------------------
using Application = UnityEngine.Application;
using Debug = UnityEngine.Debug;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using System.Text;
using System.Xml.Linq;
using System;
using UnityEditor;
using UnityEngine;

namespace Plugins.Editor.JetBrains
{
  public class RiderAssetPostprocessor : AssetPostprocessor
  {
    private const string UNITY_PLAYER_PROJECT_NAME = "Assembly-CSharp.csproj";
    private const string UNITY_EDITOR_PROJECT_NAME = "Assembly-CSharp-Editor.csproj";
    private const string UNITY_UNSAFE_KEYWORD = "-unsafe";
    private const string UNITY_DEFINE_KEYWORD = "-define:";
    private const string PLAYER_PROJECT_MANUAL_CONFIG_RELATIVE_FILE_PATH = "smcs.rsp";
    private static readonly string  PLAYER_PROJECT_MANUAL_CONFIG_ABSOLUTE_FILE_PATH
      = Path.Combine(UnityEngine.Application.dataPath, PLAYER_PROJECT_MANUAL_CONFIG_RELATIVE_FILE_PATH);
    private const string EDITOR_PROJECT_MANUAL_CONFIG_RELATIVE_FILE_PATH = "gmcs.rsp";
    private static readonly string  EDITOR_PROJECT_MANUAL_CONFIG_ABSOLUTE_FILE_PATH
      = Path.Combine(UnityEngine.Application.dataPath, EDITOR_PROJECT_MANUAL_CONFIG_RELATIVE_FILE_PATH);

    public static void OnGeneratedCSProjectFiles()
    {
      if (!RiderPlugin.Enabled)
        return;
      var currentDirectory = Directory.GetCurrentDirectory();
      var projectFiles = Directory.GetFiles(currentDirectory, "*.csproj");

      foreach (var file in projectFiles)
      {
        UpgradeProjectFile(file);
      }

      var slnFile = Directory.GetFiles(currentDirectory, "*.sln").First();
      RiderPlugin.Log(RiderPlugin.LoggingLevel.Verbose, string.Format("Post-processing {0}", slnFile));
      string content = File.ReadAllText(slnFile);
      var lines = content.Split(new[] { Environment.NewLine }, StringSplitOptions.RemoveEmptyEntries);
      var sb = new StringBuilder();
      foreach (var line in lines)
      {
        if (line.StartsWith("Project("))
        {
          MatchCollection mc = Regex.Matches(line, "\"([^\"]*)\"");
          //RiderPlugin.Log(RiderPlugin.LoggingLevel.Info, "mc[1]: "+mc[1].Value);
          //RiderPlugin.Log(RiderPlugin.LoggingLevel.Info, "mc[2]: "+mc[2].Value);
          sb.Append(line.Replace(mc[1].Value.TrimStart("\"".ToCharArray()).TrimEnd("\"".ToCharArray()), GetFileNameWithoutExtension(mc[2].Value).TrimStart("\"".ToCharArray()).TrimEnd("\"".ToCharArray())));
        }
        else
        {
          sb.Append(line);
        }
        sb.Append(Environment.NewLine);
      }
      File.WriteAllText(slnFile,sb.ToString());
    }

    private static string GetFileNameWithoutExtension(string path)
    {
      if (string.IsNullOrEmpty(path))
        return null;
      int length;
      return (length = path.LastIndexOf('.')) == -1 ? path : path.Substring(0, length);
    }

    private static void UpgradeProjectFile(string projectFile)
    {
      RiderPlugin.Log(RiderPlugin.LoggingLevel.Verbose, string.Format("Post-processing {0}", projectFile));
      var doc = XDocument.Load(projectFile);
      var projectContentElement = doc.Root;
      XNamespace xmlns = projectContentElement.Name.NamespaceName; // do not use var

      FixTargetFrameworkVersion(projectContentElement, xmlns);
      SetLangVersion(projectContentElement, xmlns);
      SetManuallyDefinedComilingSettings(projectFile, projectContentElement, xmlns);

      SetXCodeDllReference("UnityEditor.iOS.Extensions.Xcode.dll", xmlns, projectContentElement);
      SetXCodeDllReference("UnityEditor.iOS.Extensions.Common.dll", xmlns, projectContentElement);

      doc.Save(projectFile);
    }

    private static void SetManuallyDefinedComilingSettings(string projectFile, XElement projectContentElement, XNamespace xmlns)
    {
      string configPath;

      if (IsPlayerProjectFile(projectFile))
        configPath = PLAYER_PROJECT_MANUAL_CONFIG_ABSOLUTE_FILE_PATH;
      else if (IsEditorProjectFile(projectFile))
        configPath = EDITOR_PROJECT_MANUAL_CONFIG_ABSOLUTE_FILE_PATH;
      else
        configPath = null;

      if(!string.IsNullOrEmpty(configPath))
        ApplyManualCompilingSettings(configPath
          , projectContentElement
          , xmlns);
    }

    private static void ApplyManualCompilingSettings(string configFilePath, XElement projectContentElement, XNamespace xmlns)
    {
      if (File.Exists(configFilePath))
      {
        var configText = File.ReadAllText(configFilePath);
        if (configText.Contains(UNITY_UNSAFE_KEYWORD))
        {
          // Add AllowUnsafeBlocks to the .csproj. Unity doesn't generate it (although VSTU does).
          // Strictly necessary to compile unsafe code
          ApplyAllowUnsafeBlocks(projectContentElement, xmlns);
        }
        if (configText.Contains(UNITY_DEFINE_KEYWORD))
        {
          // defines could be
          // 1) -define:DEFINE1,DEFINE2
          // 2) -define:DEFINE1;DEFINE2
          // 3) -define:DEFINE1 -define:DEFINE2
          // 4) -define:DEFINE1,DEFINE2;DEFINE3
          // tested on "-define:DEF1;DEF2 -define:DEF3,DEF4;DEFFFF \n -define:DEF5"
          // result: DEF1, DEF2, DEF3, DEF4, DEFFFF, DEF5

          var definesList = new List<string>();
          var compileFlags = configText.Split(' ', '\n');
          foreach (var flag in compileFlags)
          {
            var f = flag.Trim();
            if (f.Contains(UNITY_DEFINE_KEYWORD))
            {
              var defineEndPos = f.IndexOf(UNITY_DEFINE_KEYWORD) + UNITY_DEFINE_KEYWORD.Length;
              var definesSubString = f.Substring(defineEndPos,f.Length - defineEndPos);
              definesSubString = definesSubString.Replace(";", ",");
              definesList.AddRange(definesSubString.Split(','));
            }
          }

          ApplyCustomDefines(definesList.ToArray(), projectContentElement, xmlns);
        }
      }
    }

    private static void ApplyCustomDefines(string[] customDefines, XElement projectContentElement, XNamespace xmlns)
    {
      var definesString = string.Join(";", customDefines);

      var DefineConstants = projectContentElement
        .Elements(xmlns+"PropertyGroup")
        .Elements(xmlns+"DefineConstants")
        .FirstOrDefault(definesConsts=> !string.IsNullOrEmpty(definesConsts.Value));

      if (DefineConstants != null)
      {
        DefineConstants.SetValue(DefineConstants.Value + ";" + definesString);
      }
    }

    private static void ApplyAllowUnsafeBlocks(XElement projectContentElement, XNamespace xmlns)
    {
      projectContentElement.AddFirst(
        new XElement(xmlns + "PropertyGroup", new XElement(xmlns + "AllowUnsafeBlocks", true)));
    }

    private static bool IsPlayerProjectFile(string projectFile)
    {
      return Path.GetFileName(projectFile) == UNITY_PLAYER_PROJECT_NAME;
    }

    private static bool IsEditorProjectFile(string projectFile)
    {
      return Path.GetFileName(projectFile) == UNITY_EDITOR_PROJECT_NAME;
    }

    // Helps resolve System.Linq under mono 4 - RIDER-573
    private static void FixTargetFrameworkVersion(XElement projectElement, XNamespace xmlns)
    {
      var targetFrameworkVersion = projectElement.Elements(xmlns + "PropertyGroup").
        Elements(xmlns + "TargetFrameworkVersion").FirstOrDefault(); // Processing csproj files, which are not Unity-generated #56
      if (targetFrameworkVersion != null)
      {
        var version = new Version(targetFrameworkVersion.Value.Substring(1));
        if (RiderPlugin.TargetFrameworkVersion45)
        {
          if (version < new Version(4, 5))
            targetFrameworkVersion.SetValue("v4.5");  
        }
        else
        {
          targetFrameworkVersion.SetValue("v3.5");
        }
      }
    }

    private static void SetLangVersion(XElement projectElement, XNamespace xmlns)
    {
      // Add LangVersion to the .csproj. Unity doesn't generate it (although VSTU does).
      // Not strictly necessary, as the Unity plugin for Rider will work it out, but setting
      // it makes Rider work if it's not installed.
      var langVersion = projectElement.Elements(xmlns + "PropertyGroup").
        Elements(xmlns + "LangVersion").FirstOrDefault(); // Processing csproj files, which are not Unity-generated #56
      if (langVersion != null)
      {
        langVersion.SetValue(GetLanguageLevel());
      }
      else
      {
        projectElement.AddFirst(new XElement(xmlns + "PropertyGroup",
          new XElement(xmlns + "LangVersion", GetLanguageLevel())));  
      }
    }

    private static string GetLanguageLevel()
    {
      // https://bitbucket.org/alexzzzz/unity-c-5.0-and-6.0-integration/src
      if (Directory.Exists(Path.Combine(Directory.GetCurrentDirectory(), "CSharp70Support")))
        return "7";
      if (Directory.Exists(Path.Combine(Directory.GetCurrentDirectory(), "CSharp60Support")))
        return "6";

      // Unity 5.5 supports C# 6, but only when targeting .NET 4.6. The enum doesn't exist pre Unity 5.5
      #if !UNITY_5_6_OR_NEWER
      if ((int)PlayerSettings.apiCompatibilityLevel >= 3)
      #else
      if ((int) PlayerSettings.GetApiCompatibilityLevel(EditorUserBuildSettings.selectedBuildTargetGroup) >= 3)
      #endif
        return "6";

      return "4";
    }

    private static void SetXCodeDllReference(string name, XNamespace xmlns, XElement projectContentElement)
    {
      string unityAppBaseFolder = Path.GetDirectoryName(EditorApplication.applicationPath);

      var xcodeDllPath = Path.Combine(unityAppBaseFolder, Path.Combine("Data/PlaybackEngines/iOSSupport", name));
      if (!File.Exists(xcodeDllPath))
        xcodeDllPath = Path.Combine(unityAppBaseFolder, Path.Combine("PlaybackEngines/iOSSupport", name));

      if (File.Exists(xcodeDllPath))
      {
        var itemGroup = new XElement(xmlns + "ItemGroup");
        var reference = new XElement(xmlns + "Reference");
        reference.Add(new XAttribute("Include", Path.GetFileNameWithoutExtension(xcodeDllPath)));
        reference.Add(new XElement(xmlns + "HintPath", xcodeDllPath));
        itemGroup.Add(reference);
        projectContentElement.Add(itemGroup);
      }
    }
  }
}

namespace Plugins.Editor.JetBrains
{
  [InitializeOnLoad]
  public static class RiderPlugin
  {   
    private static bool Initialized;
    private static string SlnFile;

    public static void Log(LoggingLevel level, string initialText)
    {
      if (level < SelectedLoggingLevel) return;

      var text = "[Rider] [" + level + "] " +  initialText;
      
      switch (level)
      {
        case LoggingLevel.Warning:
          Debug.LogWarning(text);
          break;
        default:
          Debug.Log(text);
          break;
      }
    }
    
    private static string GetDefaultApp()
    {
        var alreadySetPath = GetExternalScriptEditor();
        if (!string.IsNullOrEmpty(alreadySetPath) && RiderPathExist(alreadySetPath))
          return alreadySetPath;

        switch (SystemInfoRiderPlugin.operatingSystemFamily)
        {
          case OperatingSystemFamily.Windows:
            //"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\JetBrains\*Rider*.lnk"
            //%appdata%\Microsoft\Windows\Start Menu\Programs\JetBrains Toolbox\*Rider*.lnk
            string[] folders = {@"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\JetBrains", Path.Combine(
              Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
              @"Microsoft\Windows\Start Menu\Programs\JetBrains Toolbox")};

            var newPathLnks = folders.Select(b=>new DirectoryInfo(b)).Where(a => a.Exists).SelectMany(c=>c.GetFiles("*Rider*.lnk"));
            if (newPathLnks.Any())
            {
              var newPath = newPathLnks.Select(newPathLnk=> new FileInfo(ShortcutResolver.Resolve(newPathLnk.FullName))).OrderBy(a => FileVersionInfo.GetVersionInfo(a.FullName).ProductVersion).LastOrDefault();
              if (!string.IsNullOrEmpty(newPath.FullName))
              {
                /*RiderPlugin.Log(LoggingLevel.Verbose, "Update {0} to {1} product version: {2}", alreadySetPath, newPath, FileVersionInfo.GetVersionInfo(newPath.FullName).ProductVersion);
                SetExternalScriptEditor(newPath.FullName);*/
                return newPath.FullName;
              }
            }
            break;

          case OperatingSystemFamily.MacOSX:
            // "/Applications/*Rider*.app"
            //"~/Applications/JetBrains Toolbox/*Rider*.app"
            string[] foldersMac = {"/Applications",Path.Combine(Environment.GetEnvironmentVariable("HOME"), "Applications")};
            var newPathMac = foldersMac.Select(b=>new DirectoryInfo(b)).Where(a => a.Exists)
              .SelectMany(c=>c.GetDirectories("*Rider*.app")).OrderBy(a => FileVersionInfo.GetVersionInfo(a.FullName).ProductVersion).LastOrDefault();
            if (newPathMac != null)
            {
              if (!string.IsNullOrEmpty(newPathMac.FullName))
              {
                /*Log(LoggingLevel.Verbose, "Update {0} to {1}", alreadySetPath, newPathMac);
                SetExternalScriptEditor(newPathMac.FullName);*/
                return newPathMac.FullName;
              }
            }           
            break;
        }

        var riderPath = GetExternalScriptEditor();
        if (!RiderPathExist(riderPath))
        {
          Log(LoggingLevel.Warning, "Rider plugin for Unity is present, but Rider executable was not found. Please update 'External Script Editor'.");
          return null;
        }

        return riderPath;
    }

    public static bool TargetFrameworkVersion45
    {
      get { return EditorPrefs.GetBool("Rider_TargetFrameworkVersion45", true); }
      set { EditorPrefs.SetBool("Rider_TargetFrameworkVersion45", value); }
    }

    public enum LoggingLevel
    {
      Verbose = 0,
      Info = 1,
      Warning = 2
    }
    
    public static LoggingLevel SelectedLoggingLevel
    {
      get { return (LoggingLevel) EditorPrefs.GetInt("Rider_SelectedLoggingLevel", 1); }
      set { EditorPrefs.SetInt("Rider_SelectedLoggingLevel", (int) value); }
    }
    
    public static bool RiderInitializedOnce
    {
      get { return EditorPrefs.GetBool("RiderInitializedOnce", false); }
      set { EditorPrefs.SetBool("RiderInitializedOnce", value); }
    }

    internal static bool Enabled
    {
      get
      {
        var defaultApp = GetExternalScriptEditor();
        return !string.IsNullOrEmpty(defaultApp) && Path.GetFileName(defaultApp).ToLower().Contains("rider");
      }
    }

    static RiderPlugin()
    {
      var riderPath = GetDefaultApp();
      if (!RiderPathExist(riderPath)) 
        return;
      
      AddRiderToRecentlyUsedScriptApp(riderPath, "RecentlyUsedScriptApp");
      if (!RiderInitializedOnce)
      {
        SetExternalScriptEditor(riderPath);
        RiderInitializedOnce = true;
      }
      if (Enabled)
      {
        InitRiderPlugin();
      }
    }

    private static void InitRiderPlugin()
    {
      var projectDirectory = Directory.GetParent(Application.dataPath).FullName;

      var projectName = Path.GetFileName(projectDirectory);
      SlnFile = Path.Combine(projectDirectory, string.Format("{0}.sln", projectName));

      InitializeEditorInstanceJson(projectDirectory);

      Log(LoggingLevel.Info, "Rider plugin initialized. You may change the amount of Rider Debug output via Edit -> Preferences -> Rider -> Logging Level");
      Initialized = true;
    }
    
    private static void AddRiderToRecentlyUsedScriptApp(string userAppPath, string recentAppsKey)
    {
      for (int index = 0; index < 10; ++index)
      {
        string path = EditorPrefs.GetString(recentAppsKey + (object) index);
        if (File.Exists(path) && Path.GetFileName(path).ToLower().Contains("rider"))
          return;
      }
      EditorPrefs.SetString(recentAppsKey + 9, userAppPath);
    }
    
    private static string GetExternalScriptEditor()
    {
      return EditorPrefs.GetString("kScriptsDefaultApp");
    }

    private static void SetExternalScriptEditor(string path)
    {
      EditorPrefs.SetString("kScriptsDefaultApp", path);
    }

    private static bool RiderPathExist(string path)
    {
      if (string.IsNullOrEmpty(path))
        return false;
      // windows or mac
      var fileInfo = new FileInfo(path);
      if (!fileInfo.Name.ToLower().Contains("rider"))
        return false;
      var directoryInfo = new DirectoryInfo(path);
      return fileInfo.Exists || (SystemInfoRiderPlugin.operatingSystemFamily==OperatingSystemFamily.MacOSX && directoryInfo.Exists);
    }

    /// <summary>
    /// Creates and deletes Library/EditorInstance.json containing version and process ID
    /// </summary>
    /// <param name="projectDirectory">Path to the project root directory</param>
    private static void InitializeEditorInstanceJson(string projectDirectory)
    {
      // Only manage EditorInstance.json for 4.x and 5.x - it's a native feature for 2017.x
#if !UNITY_2017_1_OR_NEWER
      Log(LoggingLevel.Verbose, "Writing Library/EditorInstance.json");

      var library = Path.Combine(projectDirectory, "Library");
      var editorInstanceJsonPath = Path.Combine(library, "EditorInstance.json");

      File.WriteAllText(editorInstanceJsonPath, string.Format(@"{{
  ""process_id"": {0},
  ""version"": ""{1}""
}}", Process.GetCurrentProcess().Id, Application.unityVersion));

      AppDomain.CurrentDomain.DomainUnload += (sender, args) =>
      {
        Log(LoggingLevel.Verbose, "Deleting Library/EditorInstance.json");
        File.Delete(editorInstanceJsonPath);
      };
#endif
    }

    /// <summary>
    /// Asset Open Callback (from Unity)
    /// </summary>
    /// <remarks>
    /// Called when Unity is about to open an asset.
    /// </remarks>
    [UnityEditor.Callbacks.OnOpenAssetAttribute()]
    static bool OnOpenedAsset(int instanceID, int line)
    {
      if (Enabled)
      {
        if (!Initialized)
        {
          // make sure the plugin was initialized first.
          // this can happen in case "Rider" was set as the default scripting app only after this plugin was imported.
          InitRiderPlugin();
          RiderAssetPostprocessor.OnGeneratedCSProjectFiles();
        }

        string appPath = Path.GetDirectoryName(Application.dataPath);

        // determine asset that has been double clicked in the project view
        var selected = EditorUtility.InstanceIDToObject(instanceID);
        
        var assetFilePath = Path.Combine(appPath, AssetDatabase.GetAssetPath(selected));
        if (!(selected.GetType().ToString() == "UnityEditor.MonoScript" ||
            selected.GetType().ToString() == "UnityEngine.Shader" ||
            (selected.GetType().ToString() == "UnityEngine.TextAsset" && 
#if UNITY_5 || UNITY_5_5_OR_NEWER
            EditorSettings.projectGenerationUserExtensions.Contains(Path.GetExtension(assetFilePath).Substring(1))
#else
            EditorSettings.externalVersionControl.Contains(Path.GetExtension(assetFilePath).Substring(1))
#endif 
            ))) 
          return false;
        
        SyncSolution(); // added to handle opening file, which was just recently created.
        if (!DetectPortAndOpenFile(line, assetFilePath, SystemInfoRiderPlugin.operatingSystemFamily == OperatingSystemFamily.Windows))
        {
          var args = string.Format("{0}{1}{0} --line {2} {0}{3}{0}", "\"", SlnFile, line, assetFilePath);
          return CallRider(args);
        }
        return true;
      }

      return false;
    }


    private static bool DetectPortAndOpenFile(int line, string filePath, bool isWindows)
    {
      if (SystemInfoRiderPlugin.operatingSystemFamily == OperatingSystemFamily.Windows)
      {
        var process = GetRiderProcess();
        if (process == null) 
          return false;  
      }
      
      int[] ports = Enumerable.Range(63342, 20).ToArray();
      var res = ports.Any(port => 
      {
        var aboutUrl = string.Format("http://localhost:{0}/api/about/", port);
        var aboutUri = new Uri(aboutUrl);

        using (var client = new WebClient())
        {
          client.Headers.Add("origin", string.Format("http://localhost:{0}", port));
          client.Headers[HttpRequestHeader.ContentType] = "application/x-www-form-urlencoded";

          try
          {
            var responce = CallHttpApi(aboutUri, client);
            if (responce.ToLower().Contains("rider"))
            {
              return HttpOpenFile(line, filePath, isWindows, port, client);
            }
          }
          catch (Exception e)
          {
            Log(LoggingLevel.Verbose, string.Format("Exception in DetectPortAndOpenFile: {0}", e));
          }
        }
        return false;
      });
      return res;
    }

    private static bool HttpOpenFile(int line, string filePath, bool isWindows, int port, WebClient client)
    {
      var url = string.Format("http://localhost:{0}/api/file?file={1}{2}", port, filePath,
        line < 0
          ? "&p=0"
          : "&line=" + line); // &p is needed to workaround https://youtrack.jetbrains.com/issue/IDEA-172350
      if (isWindows)
        url = string.Format(@"http://localhost:{0}/api/file/{1}{2}", port, filePath, line < 0 ? "" : ":" + line);

      var uri = new Uri(url);
      Log(LoggingLevel.Verbose, string.Format("HttpRequestOpenFile({0})", uri.AbsoluteUri));

      CallHttpApi(uri, client);
      ActivateWindow();
      return true;
    }

    private static string CallHttpApi(Uri uri, WebClient client)
    {
      var responseString = client.DownloadString(uri);
      Log(LoggingLevel.Verbose, string.Format("HttpRequestOpenFile response: {0}", responseString));
      return responseString;
    }

    private static bool CallRider(string args)
    {
      var defaultApp = GetDefaultApp();
      if (!RiderPathExist(defaultApp))
      {
        return false;
      }

      var proc = new Process();
      if (SystemInfoRiderPlugin.operatingSystemFamily == OperatingSystemFamily.MacOSX)
      {
        proc.StartInfo.FileName = "open";
        proc.StartInfo.Arguments = string.Format("-n {0}{1}{0} --args {2}", "\"", "/" + defaultApp, args);
        Log(LoggingLevel.Verbose, string.Format("{0} {1}", proc.StartInfo.FileName, proc.StartInfo.Arguments));
      }
      else
      {
        proc.StartInfo.FileName = defaultApp;
        proc.StartInfo.Arguments = args;
        Log(LoggingLevel.Verbose, string.Format("{2}{0}{2}" + " {1}", proc.StartInfo.FileName, proc.StartInfo.Arguments, "\""));
      }

      proc.StartInfo.UseShellExecute = false;
      proc.StartInfo.WindowStyle = ProcessWindowStyle.Hidden;
      proc.StartInfo.CreateNoWindow = true;
      proc.StartInfo.RedirectStandardOutput = true;
      proc.Start();

      ActivateWindow();
      return true;
    }

    private static void ActivateWindow()
    {
      if (SystemInfoRiderPlugin.operatingSystemFamily == OperatingSystemFamily.Windows)
      {
        try
        {
          var process = GetRiderProcess();
          if (process != null)
          {
            // Collect top level windows
            var topLevelWindows = User32Dll.GetTopLevelWindowHandles();
            // Get process main window title
            var windowHandle = topLevelWindows.FirstOrDefault(hwnd => User32Dll.GetWindowProcessId(hwnd) == process.Id);
            Log(LoggingLevel.Info, string.Format("ActivateWindow: {0} {1}", process.Id, windowHandle));
            if (windowHandle != IntPtr.Zero)
            {
              //User32Dll.ShowWindow(windowHandle, 9); //SW_RESTORE = 9
              User32Dll.SetForegroundWindow(windowHandle);
            }
          }
        }
        catch (Exception e)
        {
          Log(LoggingLevel.Warning, "Exception on ActivateWindow: " + e);
        }
      }
    }

    private static Process GetRiderProcess()
    {
      var process = Process.GetProcesses().FirstOrDefault(p =>
      {
        string processName;
        try
        {
          processName = p.ProcessName; // some processes like kaspersky antivirus throw exception on attempt to get ProcessName
        }
        catch (Exception)
        {
          return false;
        }

        return !p.HasExited && processName.ToLower().Contains("rider");
      });
      return process;
    }

#if !UNITY_2017_1_OR_NEWER
    // The default "Open C# Project" menu item will use the external script editor to load the .sln
    // file, but unless Unity knows the external script editor can properly load solutions, it will
    // also launch MonoDevelop (or the OS registered app for .sln files). This menu item side steps
    // that issue, and opens the solution in Rider without opening MonoDevelop as well.
    // Unity 2017.1 and later recognise Rider as an app that can load solutions, so this menu isn't
    // needed in newer versions.
    [MenuItem("Assets/Open C# Project in Rider", false, 1000)]
    static void MenuOpenProject()
    {
      // Force the project files to be sync
      SyncSolution();

      // Load Project
      CallRider(string.Format("{0}{1}{0}", "\"", SlnFile));
    }

    [MenuItem("Assets/Open C# Project in Rider", true, 1000)]
    static bool ValidateMenuOpenProject()
    {
      return Enabled;
    }
#endif

    /// <summary>
    /// Force Unity To Write Project File
    /// </summary>
    private static void SyncSolution()
    {
      System.Type T = System.Type.GetType("UnityEditor.SyncVS,UnityEditor");
      System.Reflection.MethodInfo SyncSolution = T.GetMethod("SyncSolution",
        System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);
      SyncSolution.Invoke(null, null);
    }

    /// <summary>
    /// JetBrains Rider Integration Preferences Item
    /// </summary>
    /// <remarks>
    /// Contains all 3 toggles: Enable/Disable; Debug On/Off; Writing Launch File On/Off
    /// </remarks>
    [PreferenceItem("Rider")]
    static void RiderPreferencesItem()
    {
      EditorGUILayout.BeginVertical();

      var url = "https://github.com/JetBrains/resharper-unity";
      if (GUILayout.Button(url))
      {
        Application.OpenURL(url);
      }    
      
      EditorGUI.BeginChangeCheck();

      var help = @"For now target 4.5 is strongly recommended.
 - Without 4.5:
    - Rider will fail to resolve System.Linq on Mac/Linux
    - Rider will fail to resolve Firebase Analytics.
 - With 4.5 Rider will show ambiguous references in UniRx.
All those problems will go away after Unity upgrades to mono4.";
      TargetFrameworkVersion45 =
        EditorGUILayout.Toggle(
          new GUIContent("TargetFrameworkVersion 4.5",
            help), TargetFrameworkVersion45);
      EditorGUILayout.HelpBox(help, MessageType.None);

      EditorGUI.EndChangeCheck();

      EditorGUI.BeginChangeCheck();

      var loggingMsg =
        @"Sets the amount of Rider Debug output. If you are about to report an issue, please select Verbose logging level and attach Unity console output to the issue.";
      SelectedLoggingLevel = (LoggingLevel) EditorGUILayout.EnumPopup(new GUIContent("Logging Level", loggingMsg), SelectedLoggingLevel);
      EditorGUILayout.HelpBox(loggingMsg, MessageType.None);

      EditorGUI.EndChangeCheck();
      
/*      if (GUILayout.Button("reset RiderInitializedOnce = false"))
      {
        RiderInitializedOnce = false;
      }*/
      
      EditorGUILayout.EndVertical();
    }

    #region SystemInfoRiderPlugin
    static class SystemInfoRiderPlugin
    {
      public static OperatingSystemFamily operatingSystemFamily
      {
        get
        {
#if UNITY_5_5_OR_NEWER
return SystemInfo.operatingSystemFamily;
#else
          if (SystemInfo.operatingSystem.StartsWith("Mac", StringComparison.InvariantCultureIgnoreCase))
          {
            return OperatingSystemFamily.MacOSX;
          }
          if (SystemInfo.operatingSystem.StartsWith("Win", StringComparison.InvariantCultureIgnoreCase))
          {
            return OperatingSystemFamily.Windows;
          }
          if (SystemInfo.operatingSystem.StartsWith("Lin", StringComparison.InvariantCultureIgnoreCase))
          {
            return OperatingSystemFamily.Linux;
          }
          return OperatingSystemFamily.Other;
#endif
        }
      }
    }
#if !UNITY_5_5_OR_NEWER
    enum OperatingSystemFamily
    {
      Other,
      MacOSX,
      Windows,
      Linux,
    }
#endif
    #endregion
    
    static class User32Dll
    {

      /// <summary>
      /// Gets the ID of the process that owns the window.
      /// Note that creating a <see cref="Process"/> wrapper for that is very expensive because it causes an enumeration of all the system processes to happen.
      /// </summary>
      public static int GetWindowProcessId(IntPtr hwnd)
      {
        uint dwProcessId;
        GetWindowThreadProcessId(hwnd, out dwProcessId);
        return unchecked((int) dwProcessId);
      }

      /// <summary>
      /// Lists the handles of all the top-level windows currently available in the system.
      /// </summary>
      public static List<IntPtr> GetTopLevelWindowHandles()
      {
        var retval = new List<IntPtr>();
        EnumWindowsProc callback = (hwnd, param) =>
        {
          retval.Add(hwnd);
          return 1;
        };
        EnumWindows(Marshal.GetFunctionPointerForDelegate(callback), IntPtr.Zero);
        GC.KeepAlive(callback);
        return retval;
      }

      public delegate Int32 EnumWindowsProc(IntPtr hwnd, IntPtr lParam);

      [DllImport("user32.dll", CharSet = CharSet.Unicode, PreserveSig = true, SetLastError = true,
        ExactSpelling = true)]
      public static extern Int32 EnumWindows(IntPtr lpEnumFunc, IntPtr lParam);

      [DllImport("user32.dll", SetLastError = true)]
      static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

      [DllImport("user32.dll", CharSet = CharSet.Unicode, PreserveSig = true, SetLastError = true,
        ExactSpelling = true)]
      public static extern Int32 SetForegroundWindow(IntPtr hWnd);

      [DllImport("user32.dll", CharSet = CharSet.Unicode, PreserveSig = true, SetLastError = true,
        ExactSpelling = true)]
      public static extern UInt32 ShowWindow(IntPtr hWnd, Int32 nCmdShow);
    }
    
    static class ShortcutResolver
    {
      #region Signitures imported from http://pinvoke.net

      [DllImport("shfolder.dll", CharSet = CharSet.Auto)]
      internal static extern int SHGetFolderPath(IntPtr hwndOwner, int nFolder, IntPtr hToken, int dwFlags, StringBuilder lpszPath);

      [Flags()]
      enum SLGP_FLAGS
      {
        /// <summary>Retrieves the standard short (8.3 format) file name</summary>
        SLGP_SHORTPATH = 0x1,

        /// <summary>Retrieves the Universal Naming Convention (UNC) path name of the file</summary>
        SLGP_UNCPRIORITY = 0x2,

        /// <summary>Retrieves the raw path name. A raw path is something that might not exist and may include environment variables that need to be expanded</summary>
        SLGP_RAWPATH = 0x4
      }

      [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
      struct WIN32_FIND_DATAW
      {
        public uint dwFileAttributes;
        public long ftCreationTime;
        public long ftLastAccessTime;
        public long ftLastWriteTime;
        public uint nFileSizeHigh;
        public uint nFileSizeLow;
        public uint dwReserved0;
        public uint dwReserved1;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string cFileName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 14)] public string cAlternateFileName;
      }

      [Flags()]
      enum SLR_FLAGS
      {
        /// <summary>
        /// Do not display a dialog box if the link cannot be resolved. When SLR_NO_UI is set,
        /// the high-order word of fFlags can be set to a time-out value that specifies the
        /// maximum amount of time to be spent resolving the link. The function returns if the
        /// link cannot be resolved within the time-out duration. If the high-order word is set
        /// to zero, the time-out duration will be set to the default value of 3,000 milliseconds
        /// (3 seconds). To specify a value, set the high word of fFlags to the desired time-out
        /// duration, in milliseconds.
        /// </summary>
        SLR_NO_UI = 0x1,

        /// <summary>Obsolete and no longer used</summary>
        SLR_ANY_MATCH = 0x2,

        /// <summary>If the link object has changed, update its path and list of identifiers.
        /// If SLR_UPDATE is set, you do not need to call IPersistFile::IsDirty to determine
        /// whether or not the link object has changed.</summary>
        SLR_UPDATE = 0x4,

        /// <summary>Do not update the link information</summary>
        SLR_NOUPDATE = 0x8,

        /// <summary>Do not execute the search heuristics</summary>
        SLR_NOSEARCH = 0x10,

        /// <summary>Do not use distributed link tracking</summary>
        SLR_NOTRACK = 0x20,

        /// <summary>Disable distributed link tracking. By default, distributed link tracking tracks
        /// removable media across multiple devices based on the volume name. It also uses the
        /// Universal Naming Convention (UNC) path to track remote file systems whose drive letter
        /// has changed. Setting SLR_NOLINKINFO disables both types of tracking.</summary>
        SLR_NOLINKINFO = 0x40,

        /// <summary>Call the Microsoft Windows Installer</summary>
        SLR_INVOKE_MSI = 0x80
      }


      /// <summary>The IShellLink interface allows Shell links to be created, modified, and resolved</summary>
      [ComImport(), InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("000214F9-0000-0000-C000-000000000046")]
      interface IShellLinkW
      {
        /// <summary>Retrieves the path and file name of a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetPath([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cchMaxPath, out WIN32_FIND_DATAW pfd, SLGP_FLAGS fFlags);

        /// <summary>Retrieves the list of item identifiers for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetIDList(out IntPtr ppidl);

        /// <summary>Sets the pointer to an item identifier list (PIDL) for a Shell link object.</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetIDList(IntPtr pidl);

        /// <summary>Retrieves the description string for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetDescription([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cchMaxName);

        /// <summary>Sets the description for a Shell link object. The description can be any application-defined string</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);

        /// <summary>Retrieves the name of the working directory for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetWorkingDirectory([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cchMaxPath);

        /// <summary>Sets the name of the working directory for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);

        /// <summary>Retrieves the command-line arguments associated with a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetArguments([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cchMaxPath);

        /// <summary>Sets the command-line arguments for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);

        /// <summary>Retrieves the hot key for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetHotkey(out short pwHotkey);

        /// <summary>Sets a hot key for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetHotkey(short wHotkey);

        /// <summary>Retrieves the show command for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetShowCmd(out int piShowCmd);

        /// <summary>Sets the show command for a Shell link object. The show command sets the initial show state of the window.</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetShowCmd(int iShowCmd);

        /// <summary>Retrieves the location (path and index) of the icon for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetIconLocation([Out(), MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cchIconPath, out int piIcon);

        /// <summary>Sets the location (path and index) of the icon for a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);

        /// <summary>Sets the relative path to the Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, int dwReserved);

        /// <summary>Attempts to find the target of a Shell link, even if it has been moved or renamed</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void Resolve(IntPtr hwnd, SLR_FLAGS fFlags);

        /// <summary>Sets the path and file name of a Shell link object</summary>
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
      }

      [ComImport, Guid("0000010c-0000-0000-c000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
      public interface IPersist
      {
        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetClassID(out Guid pClassID);
      }


      [ComImport, Guid("0000010b-0000-0000-C000-000000000046"),
       InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
      public interface IPersistFile : IPersist
      {
        [MethodImpl(MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        new void GetClassID(out Guid pClassID);

        [MethodImpl(MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        int IsDirty();

        [MethodImpl(MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void Load([In, MarshalAs(UnmanagedType.LPWStr)] string pszFileName, uint dwMode);

        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void Save([In, MarshalAs(UnmanagedType.LPWStr)] string pszFileName, [In, MarshalAs(UnmanagedType.Bool)] bool fRemember);

        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void SaveCompleted([In, MarshalAs(UnmanagedType.LPWStr)] string pszFileName);

        [MethodImpl (MethodImplOptions.InternalCall | MethodImplOptions.PreserveSig, MethodCodeType = MethodCodeType.Runtime)]
        void GetCurFile([In, MarshalAs(UnmanagedType.LPWStr)] string ppszFileName);
      }

      const uint STGM_READ = 0;
      const int MAX_PATH = 260;

      // CLSID_ShellLink from ShlGuid.h 
      [
        ComImport(),
        Guid("00021401-0000-0000-C000-000000000046")
      ]
      public class ShellLink
      {
      }

      #endregion

      public static string Resolve(string filename)
      {
        ShellLink link = new ShellLink();
        ((IPersistFile) link).Load(filename, STGM_READ);
        // If I can get hold of the hwnd call resolve first. This handles moved and renamed files.  
        // ((IShellLinkW)link).Resolve(hwnd, 0) 
        StringBuilder sb = new StringBuilder(MAX_PATH);
        WIN32_FIND_DATAW data = new WIN32_FIND_DATAW();
        ((IShellLinkW) link).GetPath(sb, sb.Capacity, out data, 0);
        return sb.ToString();
      }
    }
  }
}

// Developed using JetBrains Rider =)


```
