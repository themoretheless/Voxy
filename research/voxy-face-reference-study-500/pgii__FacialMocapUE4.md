# Facial rendering source scan: pgii/FacialMocapUE4

Repository: https://github.com/pgii/FacialMocapUE4

Pinned source revision: 6ca528e5108a029ba3158b4246516dc36907b65c

Discovery topic: facial mocap

Description: Facial Mocap testing with the help of dlib library in Unreal Engine

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["FacialMocapSender/packages/DlibDotNet.19.17.0.20190623/LICENSE.txt", "FacialMocapSender/packages/Newtonsoft.Json.12.0.2/LICENSE.md", "FacialMocapUE4/Plugins/UDPWrapper/LICENSE"]

Topic paths: {"eye": ["FacialMocapUE4/Content/FacialMocapCharacter/Materials/EyeMoisture.uasset", "FacialMocapUE4/Content/FacialMocapCharacter/Materials/EyeSocket.uasset", "FacialMocapUE4/Content/FacialMocapCharacter/Materials/Eyelashes.uasset", "FacialMocapUE4/Content/FacialMocapCharacter/Textures/Girl8Eyelashes_1001.uasset", "FacialMocapUE4/Content/FacialMocapCharacter/Textures/Girl8Eyes02D_1006.uasset", "FacialMocapUE4/Content/FacialMocapCharacter/Textures/Girl8EyesB_1006.uasset"], "blendshape": ["FacialMocapSender/FacialMocapSender/FacialBlendShapeController.cs"], "mouth": ["FacialMocapUE4/Content/FacialMocapCharacter/Materials/Mouth.uasset", "FacialMocapUE4/Content/FacialMocapCharacter/Textures/Girl8MouthD_1005.uasset"], "teeth": ["FacialMocapUE4/Content/FacialMocapCharacter/Materials/Teeth.uasset"], "iris": ["FacialMocapUE4/Content/FacialMocapCharacter/Materials/Irises.uasset"], "cornea": ["FacialMocapUE4/Content/FacialMocapCharacter/Materials/Cornea.uasset"], "face": ["FacialMocapSender/FacialMocapSender/bin/x64/Debug/shape_predictor_68_face_landmarks.dat", "FacialMocapSender/FacialMocapSender/bin/x64/Release/shape_predictor_68_face_landmarks.dat", "FacialMocapUE4/Content/FacialMocapCharacter/Materials/Face.uasset"], "lip": ["FacialMocapUE4/Content/FacialMocapCharacter/Materials/Lips.uasset"]}

## Source file: FacialMocapSender/FacialMocapSender/FacialBlendShapeController.cs

{
  "path": "FacialMocapSender/FacialMocapSender/FacialBlendShapeController.cs",
  "blob_sha": "33ad96a974bff563da6026324a362c4a2fc2f0a6",
  "sha256": "2ec130f087843cb2123f790b6e14d5980125523db797b81f30b519f05ee45725",
  "lines": 207,
  "bytes": 9223,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 20,
    "blendshape": 1,
    "jaw": 10,
    "mouth": 18,
    "rig": 28
  },
  "url": "https://github.com/pgii/FacialMocapUE4/blob/6ca528e5108a029ba3158b4246516dc36907b65c/FacialMocapSender/FacialMocapSender/FacialBlendShapeController.cs",
  "function_signature_candidates": [
    {
      "line": 36,
      "text": "public void InitNormalOffset(List<Point> points)"
    },
    {
      "line": 50,
      "text": "public void SetBottomJawOffset(List<Point> points)"
    },
    {
      "line": 72,
      "text": "public void SetSmileOffset(List<Point> points)"
    },
    {
      "line": 112,
      "text": "public void SetBrowUpOffset(List<Point> points)"
    },
    {
      "line": 153,
      "text": "public void SetEyeBlinkLeftOffset(List<Point> points)"
    },
    {
      "line": 158,
      "text": "public void SetEyeBlinkRightOffset(List<Point> points)"
    },
    {
      "line": 197,
      "text": "public static float ClampNegPos(float value)"
    }
  ]
}

```text

﻿using System.Collections.Generic;
using System.Drawing;
using Point = DlibDotNet.Point;

public class FacialBlendShapeController
{
    private int[] bottomJawPoint = new[] {8, 30};
    private int[] mouthCornerLeftPoint = new[] { 54, 30 };
    private int[] mouthCornerRightPoint = new[] { 48, 30 };
    private int[] browLeftPoint = new[] { 24, 30 };
    private int[] browRightPoint = new[] { 19, 30 };
    private int[] eyeBlinkLeftPoint = new[] { 43, 47 };
    private int[] eyeBlinkRightPoint = new[] { 38, 40 };

    private PointF bottomJawNormalOffset;
    private PointF bottomJawOffset;

    private PointF mouthCornerLeftNormalOffset;
    private PointF mouthCornerLeftOffset;

    private PointF mouthCornerRightNormalOffset;
    private PointF mouthCornerRightOffset;

    private PointF browLeftNormalOffset;
    private PointF browLeftOffset;

    private PointF browRightNormalOffset;
    private PointF browRightOffset;

    private PointF eyeBlinkLeftNormalOffset;
    private PointF eyeBlinkLeftOffset;

    private PointF eyeBlinkRightNormalOffset;
    private PointF eyeBlinkRightOffset;

    public void InitNormalOffset(List<Point> points)
    {
        bottomJawNormalOffset = new PointF(points[bottomJawPoint[0]].X - points[bottomJawPoint[1]].X, points[bottomJawPoint[0]].Y - points[bottomJawPoint[1]].Y);

        mouthCornerLeftNormalOffset = new PointF(points[mouthCornerLeftPoint[0]].X - points[mouthCornerLeftPoint[1]].X, points[mouthCornerLeftPoint[0]].Y - points[mouthCornerLeftPoint[1]].Y);
        mouthCornerRightNormalOffset = new PointF(points[mouthCornerRightPoint[1]].X - points[mouthCornerRightPoint[0]].X, points[mouthCornerRightPoint[0]].Y - points[mouthCornerRightPoint[1]].Y);

        browLeftNormalOffset = new PointF(points[browLeftPoint[0]].X - points[browLeftPoint[1]].X, points[browLeftPoint[0]].Y - points[browLeftPoint[1]].Y);
        browRightNormalOffset = new PointF(points[browRightPoint[1]].X - points[browRightPoint[0]].X, points[browRightPoint[0]].Y - points[browRightPoint[1]].Y);

        eyeBlinkLeftNormalOffset = new PointF(points[eyeBlinkLeftPoint[0]].X - points[eyeBlinkLeftPoint[1]].X, points[eyeBlinkLeftPoint[0]].Y - points[eyeBlinkLeftPoint[1]].Y);
        eyeBlinkRightNormalOffset = new PointF(points[eyeBlinkRightPoint[1]].X - points[eyeBlinkRightPoint[0]].X, points[eyeBlinkRightPoint[0]].Y - points[eyeBlinkRightPoint[1]].Y);
    }

    public void SetBottomJawOffset(List<Point> points)
    {
        bottomJawOffset = new PointF(points[bottomJawPoint[0]].X - points[bottomJawPoint[1]].X, points[bottomJawPoint[0]].Y - points[bottomJawPoint[1]].Y);
    }

    public PointF GetBottomJawPosition(List<Point> points)
    {
        PointF diff = new PointF(points[bottomJawPoint[0]].X - points[bottomJawPoint[1]].X, points[bottomJawPoint[0]].Y - points[bottomJawPoint[1]].Y);
        float resultX = (diff.X - bottomJawNormalOffset.X) / (bottomJawOffset.X - bottomJawNormalOffset.X);
        float resultY = (diff.Y - bottomJawNormalOffset.Y) / (bottomJawOffset.Y - bottomJawNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }

    public void SetSmileOffset(List<Point> points)
    {
        mouthCornerLeftOffset = new PointF(points[mouthCornerLeftPoint[0]].X - points[mouthCornerLeftPoint[1]].X, points[mouthCornerLeftPoint[0]].Y - points[mouthCornerLeftPoint[1]].Y);
        mouthCornerRightOffset = new PointF(points[mouthCornerRightPoint[1]].X - points[mouthCornerRightPoint[0]].X, points[mouthCornerRightPoint[0]].Y - points[mouthCornerRightPoint[1]].Y);
    }

    public PointF GetMouthCornerLeftPosition(List<Point> points)
    {
        PointF diff = new PointF(points[mouthCornerLeftPoint[0]].X - points[mouthCornerLeftPoint[1]].X, points[mouthCornerLeftPoint[0]].Y - points[mouthCornerLeftPoint[1]].Y);
        float resultX = (diff.X - mouthCornerLeftNormalOffset.X) / (mouthCornerLeftOffset.X - mouthCornerLeftNormalOffset.X);
        float resultY = (diff.Y - mouthCornerLeftNormalOffset.Y) / (mouthCornerLeftOffset.Y - mouthCornerLeftNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }

    public PointF GetMouthCornerRightPosition(List<Point> points)
    {
        PointF diff = new PointF(points[mouthCornerRightPoint[1]].X - points[mouthCornerRightPoint[0]].X, points[mouthCornerRightPoint[0]].Y - points[mouthCornerRightPoint[1]].Y);
        float resultX = (diff.X - mouthCornerRightNormalOffset.X) / (mouthCornerRightOffset.X - mouthCornerRightNormalOffset.X);
        float resultY = (diff.Y - mouthCornerRightNormalOffset.Y) / (mouthCornerRightOffset.Y - mouthCornerRightNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }

    public void SetBrowUpOffset(List<Point> points)
    {
        browLeftOffset = new PointF(points[browLeftPoint[0]].X - points[browLeftPoint[1]].X, points[browLeftPoint[0]].Y - points[browLeftPoint[1]].Y);
        browRightOffset = new PointF(points[browRightPoint[1]].X - points[browRightPoint[0]].X, points[browRightPoint[0]].Y - points[browRightPoint[1]].Y);
    }

    public PointF GetBrowLeftPosition(List<Point> points)
    {
        PointF diff = new PointF(points[browLeftPoint[0]].X - points[browLeftPoint[1]].X, points[browLeftPoint[0]].Y - points[browLeftPoint[1]].Y);
        float resultX = (diff.X - browLeftNormalOffset.X) / (browLeftOffset.X - browLeftNormalOffset.X);
        float resultY = (diff.Y - browLeftNormalOffset.Y) / (browLeftOffset.Y - browLeftNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }

    public PointF GetBrowRightPosition(List<Point> points)
    {
        PointF diff = new PointF(points[browRightPoint[1]].X - points[browRightPoint[0]].X, points[browRightPoint[0]].Y - points[browRightPoint[1]].Y);
        float resultX = (diff.X - browRightNormalOffset.X) / (browRightOffset.X - browRightNormalOffset.X);
        float resultY = (diff.Y - browRightNormalOffset.Y) / (browRightOffset.Y - browRightNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }


    public void SetEyeBlinkLeftOffset(List<Point> points)
    {
        eyeBlinkLeftOffset = new PointF(points[eyeBlinkLeftPoint[0]].X - points[eyeBlinkLeftPoint[1]].X, points[eyeBlinkLeftPoint[0]].Y - points[eyeBlinkLeftPoint[1]].Y);
    }

    public void SetEyeBlinkRightOffset(List<Point> points)
    {
        eyeBlinkRightOffset = new PointF(points[eyeBlinkRightPoint[1]].X - points[eyeBlinkRightPoint[0]].X, points[eyeBlinkRightPoint[0]].Y - points[eyeBlinkRightPoint[1]].Y);
    }

    public PointF GetEyeBlinkLeftPosition(List<Point> points)
    {
        PointF diff = new PointF(points[eyeBlinkLeftPoint[0]].X - points[eyeBlinkLeftPoint[1]].X, points[eyeBlinkLeftPoint[0]].Y - points[eyeBlinkLeftPoint[1]].Y);
        float resultX = (diff.X - eyeBlinkLeftNormalOffset.X) / (eyeBlinkLeftOffset.X - eyeBlinkLeftNormalOffset.X);
        float resultY = (diff.Y - eyeBlinkLeftNormalOffset.Y) / (eyeBlinkLeftOffset.Y - eyeBlinkLeftNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }

    public PointF GetEyeBlinkRightPosition(List<Point> points)
    {
        PointF diff = new PointF(points[eyeBlinkRightPoint[1]].X - points[eyeBlinkRightPoint[0]].X, points[eyeBlinkRightPoint[0]].Y - points[eyeBlinkRightPoint[1]].Y);
        float resultX = (diff.X - eyeBlinkRightNormalOffset.X) / (eyeBlinkRightOffset.X - eyeBlinkRightNormalOffset.X);
        float resultY = (diff.Y - eyeBlinkRightNormalOffset.Y) / (eyeBlinkRightOffset.Y - eyeBlinkRightNormalOffset.Y);

        if (float.IsInfinity(resultX))
            resultX = 0;
        if (float.IsInfinity(resultY))
            resultY = 0;

        resultX = ClampNegPos(resultX);
        resultY = ClampNegPos(resultY);

        return new PointF(resultX, resultY);
    }

    public static float ClampNegPos(float value)
    {
        if (value > 1F)
            return 1F;
        else if(value < -1F)
            return -1F;
        else
            return value;
    }
}



```

## Source file: FacialMocapSender/FacialMocapSender/Program.cs

{
  "path": "FacialMocapSender/FacialMocapSender/Program.cs",
  "blob_sha": "c1e1da5d308736970cc4a338268e27c7b452a232",
  "sha256": "59b8b50ff79d89555677c2d70299d1ce1a740d81ed23bba6389b7d32b65bc312",
  "lines": 16,
  "bytes": 339,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/pgii/FacialMocapUE4/blob/6ca528e5108a029ba3158b4246516dc36907b65c/FacialMocapSender/FacialMocapSender/Program.cs",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "static void Main()"
    }
  ]
}

```text

﻿using System;
using System.Windows.Forms;

namespace FacialMocapSender
{
    static class Program
    {
        [STAThread]
        static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            Application.Run(new MainForm());
        }
    }
}


```

## Source file: FacialMocapSender/FacialMocapSender/MainForm.cs

{
  "path": "FacialMocapSender/FacialMocapSender/MainForm.cs",
  "blob_sha": "31377ae84f6841e3c257be1e4306d86412786370",
  "sha256": "0ea37881b3d0dd1b2835223555c71a01d42884926fabfea823c528ba008c8150",
  "lines": 218,
  "bytes": 8645,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 15,
    "blendshape": 16,
    "jaw": 2,
    "mouth": 27,
    "rig": 20,
    "face": 6
  },
  "url": "https://github.com/pgii/FacialMocapUE4/blob/6ca528e5108a029ba3158b4246516dc36907b65c/FacialMocapSender/FacialMocapSender/MainForm.cs",
  "function_signature_candidates": [
    {
      "line": 57,
      "text": "public void UdpSendMessage(string text)"
    },
    {
      "line": 62,
      "text": "private void ProcessFrame(object sender, EventArgs e)"
    },
    {
      "line": 117,
      "text": "public async void GetFacialBlendShape()"
    },
    {
      "line": 188,
      "text": "private void btnMouthNormalCalib_Click(object sender, EventArgs e)"
    },
    {
      "line": 193,
      "text": "private void btnMouthOpenCalib_Click(object sender, EventArgs e)"
    },
    {
      "line": 198,
      "text": "private void btnMouthSmileCalib_Click(object sender, EventArgs e)"
    },
    {
      "line": 203,
      "text": "private void btnBrowUpCalib_Click(object sender, EventArgs e)"
    },
    {
      "line": 208,
      "text": "private void btnEyeBlinkLeft_Click(object sender, EventArgs e)"
    },
    {
      "line": 213,
      "text": "private void btnEyeBlinkRight_Click(object sender, EventArgs e)"
    }
  ]
}

```text

﻿using DlibDotNet;
using DlibDotNet.Extensions;
using Emgu.CV;
using Emgu.CV.CvEnum;
using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Forms;
using static UDPSocket;
using Dlib = DlibDotNet.Dlib;
using Point = DlibDotNet.Point;

namespace FacialMocapSender
{
    public partial class MainForm : Form
    {
        private ShapePredictor poseModel = ShapePredictor.Deserialize(Application.StartupPath + "\\shape_predictor_68_face_landmarks.dat");
        private FrontalFaceDetector detector = Dlib.GetFrontalFaceDetector();
        
        private VideoCapture _capture;

        FacialBlendShapeController facialBlendShapeController = new FacialBlendShapeController();

        List<Point> landmarkPoint = new List<Point>(68);

        UDPSocket udpSocket = new UDPSocket();
        public State state = new State();

        public MainForm()
        {
            InitializeComponent();

            udpSocket.Client("127.0.0.1", 7755);

            _capture = new VideoCapture(0);
            _capture.SetCaptureProperty(CapProp.FrameWidth, 800);
            _capture.SetCaptureProperty(CapProp.FrameHeight, 600);
            _capture.ImageGrabbed += ProcessFrame;

            if (_capture != null)
            {
                try
                {
                    _capture.Start();
                }
                catch (Exception ex)
                {
                    MessageBox.Show(ex.Message);
                }
            }
        }

        public void UdpSendMessage(string text)
        {
            udpSocket.Send(text);
        }

        private void ProcessFrame(object sender, EventArgs e)
        {
            Stopwatch SW = new Stopwatch();
            SW.Start();

            try
            {
                Mat temp = new Mat();
                _capture.Read(temp);

                var array = new byte[temp.Width * temp.Height * temp.ElementSize];
                temp.CopyTo(array);

                Array2D<RgbPixel> cimg = Dlib.LoadImageData<RgbPixel>(array, (uint) temp.Height, (uint) temp.Width, (uint) (temp.Width * temp.ElementSize));

                Rectangle[] faces = detector.Operator(cimg); 

                if (faces.Any())
                {
                    FullObjectDetection det = poseModel.Detect(cimg, faces[0]);
                    List<FullObjectDetection> shapes = new List<FullObjectDetection>();
                    shapes.Add(det);
                    FullObjectDetection shape = shapes[0];
                    ImageWindow.OverlayLine[] lines = Dlib.RenderFaceDetections(shapes);

                    if (chbShowLineOnly.Checked)
                        cimg = new Array2D<RgbPixel>(cimg.Rows, cimg.Columns);

                    foreach (var line in lines)
                        Dlib.DrawLine(cimg, line.Point1, line.Point2, new RgbPixel { Green = 255 });

                    pictureBoxImage.Image?.Dispose();
                    pictureBoxImage.Image = cimg.ToBitmap();

                    foreach (var line in lines)
                        line.Dispose();

                    for (uint i = 0; i < shape.Parts; i++)
                        landmarkPoint.Insert((int)i, new Point(shape.GetPart(i).X, shape.GetPart(i).Y));

                    GetFacialBlendShape();

                    foreach (var s in shapes)
                        s.Dispose();
                }
            }
            catch (Exception exception)
            {
                Console.WriteLine(exception.StackTrace);
            }

            SW.Stop();
            Debug.WriteLine(string.Format("FPS: {0}", 1000 / SW.ElapsedMilliseconds));
        }

        public async void GetFacialBlendShape()
        {
            await Task.Run(() =>
            {
                float mouthOpen = facialBlendShapeController.GetBottomJawPosition(landmarkPoint).Y;
                float mouthTight = 0;

                if (mouthOpen < 0)
                {
                    mouthTight = Math.Abs(mouthOpen);
                    mouthOpen = 0;
                }

                float mouthSmileLeft = facialBlendShapeController.GetMouthCornerLeftPosition(landmarkPoint).X;
                float mouthNarrowLeft = 0;

                if (mouthSmileLeft < 0)
                {
                    mouthNarrowLeft = Math.Abs(mouthSmileLeft);
                    mouthSmileLeft = 0;
                }

                float mouthSmileRight = facialBlendShapeController.GetMouthCornerRightPosition(landmarkPoint).X;
                float mouthNarrowRight = 0;

                if (mouthSmileRight < 0)
                {
                    mouthNarrowRight = Math.Abs(mouthSmileRight);
                    mouthSmileRight = 0;
                }

                float browUpLeft = facialBlendShapeController.GetBrowLeftPosition(landmarkPoint).Y;
                float browUpRight = facialBlendShapeController.GetBrowRightPosition(landmarkPoint).Y;

                float eyeBlinkLeft = facialBlendShapeController.GetEyeBlinkLeftPosition(landmarkPoint).Y;
                if (eyeBlinkLeft < 0.75)
                    eyeBlinkLeft = 0;
                else
                    eyeBlinkLeft = 1;

                float eyeBlinkRight = facialBlendShapeController.GetEyeBlinkRightPosition(landmarkPoint).Y;
                if (eyeBlinkRight < 0.75)
                    eyeBlinkRight = 0;
                else
                    eyeBlinkRight = 1;

                FacialDataModel facialDataModel = new FacialDataModel();
                facialDataModel.type = "facialData";
                facialDataModel.mouthOpen = mouthOpen.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.mouthTight = mouthTight.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.mouthSmileLeft = mouthSmileLeft.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.mouthSmileRight = mouthSmileRight.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.mouthNarrowLeft = mouthNarrowLeft.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.mouthNarrowRight = mouthNarrowRight.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.browUpLeft = browUpLeft.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.browUpRight = browUpRight.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.eyeBlinkLeft = eyeBlinkLeft.ToString("N2", CultureInfo.InvariantCulture);
                facialDataModel.eyeBlinkRight = eyeBlinkRight.ToString("N2", CultureInfo.InvariantCulture);

                string jsonString = JsonConvert.SerializeObject(facialDataModel);

                UdpSendMessage(jsonString);

                Debug.WriteLine(string.Format("mouthOpen: {0} mouthTight: {1}", mouthOpen.ToString("N1"), mouthTight.ToString("N1")));
                Debug.WriteLine(string.Format("mouthSmileLeft: {0} mouthSmileRight: {1}", mouthSmileLeft.ToString("N1"), mouthSmileRight.ToString("N1")));
                Debug.WriteLine(string.Format("mouthNarrowLeft: {0} mouthNarrowRight: {1}", mouthNarrowLeft.ToString("N1"), mouthNarrowRight.ToString("N1")));
                Debug.WriteLine(string.Format("browUpLeft: {0} browUpRight: {1}", browUpLeft.ToString("N1"), browUpRight.ToString("N1")));
                Debug.WriteLine(string.Format("eyeBlinkLeft: {0} eyeBlinkRight: {1}", eyeBlinkLeft.ToString("N1"), eyeBlinkRight.ToString("N1")));
            });
        }
        
        private void btnMouthNormalCalib_Click(object sender, EventArgs e)
        {
            facialBlendShapeController.InitNormalOffset(landmarkPoint);
        }

        private void btnMouthOpenCalib_Click(object sender, EventArgs e)
        {
            facialBlendShapeController.SetBottomJawOffset(landmarkPoint);
        }

        private void btnMouthSmileCalib_Click(object sender, EventArgs e)
        {
            facialBlendShapeController.SetSmileOffset(landmarkPoint);
        }

        private void btnBrowUpCalib_Click(object sender, EventArgs e)
        {
            facialBlendShapeController.SetBrowUpOffset(landmarkPoint);
        }

        private void btnEyeBlinkLeft_Click(object sender, EventArgs e)
        {
            facialBlendShapeController.SetEyeBlinkLeftOffset(landmarkPoint);
        }

        private void btnEyeBlinkRight_Click(object sender, EventArgs e)
        {
            facialBlendShapeController.SetEyeBlinkRightOffset(landmarkPoint);
        }
    }
}


```

## Source file: FacialMocapSender/FacialMocapSender/UDPSocket.cs

{
  "path": "FacialMocapSender/FacialMocapSender/UDPSocket.cs",
  "blob_sha": "e6a8133218b9536c010f3bbecfa705ce6fbee725",
  "sha256": "05a5d6a4caf45d7e722f60579d094b2e729e6df5a41407c709b9d08e7d18f77f",
  "lines": 33,
  "bytes": 846,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/pgii/FacialMocapUE4/blob/6ca528e5108a029ba3158b4246516dc36907b65c/FacialMocapSender/FacialMocapSender/UDPSocket.cs",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "public void Client(string address, int port)"
    },
    {
      "line": 23,
      "text": "public void Send(string text)"
    }
  ]
}

```text

﻿using System;
using System.Net;
using System.Net.Sockets;
using System.Text;

public class UDPSocket
{
    public Socket _socket = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, ProtocolType.Udp);
    public State state = new State();
    public EndPoint epFrom = new IPEndPoint(IPAddress.Any, 0);
    public AsyncCallback recv = null;

    public class State
    {
        public byte[] buffer = new byte[8192];
    }

    public void Client(string address, int port)
    {
        _socket.Connect(IPAddress.Parse(address), port);
    }

    public void Send(string text)
    {
        byte[] data = Encoding.ASCII.GetBytes(text);
        _socket.BeginSend(data, 0, data.Length, SocketFlags.None, (ar) =>
        {
            State so = (State)ar.AsyncState;
            int bytes = _socket.EndSend(ar);
        }, state);
    }

}

```
