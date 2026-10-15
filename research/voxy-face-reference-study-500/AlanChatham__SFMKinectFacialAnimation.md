# Facial rendering source scan: AlanChatham/SFMKinectFacialAnimation

Repository: https://github.com/AlanChatham/SFMKinectFacialAnimation

Pinned source revision: 2e49253c786b27f3e826b066740f262709bdcb92

Discovery topic: facial animation

Description: Using the Kinect facial tracking to drive facial animation in Source Filmmaker

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["FaceTrackingBasics-WPF/App.config", "FaceTrackingBasics-WPF/App.xaml", "FaceTrackingBasics-WPF/App.xaml.cs", "FaceTrackingBasics-WPF/FaceTrackingBasics-WPF.csproj", "FaceTrackingBasics-WPF/FaceTrackingBasics-WPF.sln", "FaceTrackingBasics-WPF/FaceTrackingViewer.xaml", "FaceTrackingBasics-WPF/FaceTrackingViewer.xaml.cs", "FaceTrackingBasics-WPF/Images/Kinect.ico", "FaceTrackingBasics-WPF/Images/Logo.png", "FaceTrackingBasics-WPF/Images/Status.png", "FaceTrackingBasics-WPF/MainWindow.xaml", "FaceTrackingBasics-WPF/MainWindow.xaml.cs", "FaceTrackingBasics-WPF/Properties/AssemblyInfo.cs", "FaceTrackingBasics-WPF/Properties/Resources.Designer.cs", "FaceTrackingBasics-WPF/Properties/Resources.resx", "FaceTrackingBasics-WPF/Properties/Settings.Designer.cs", "FaceTrackingBasics-WPF/Properties/Settings.settings", "FaceTrackingVisualization/FaceTrackingVisualization.sln", "FaceTrackingVisualization/MultiFace/FTHelper2.cpp", "FaceTrackingVisualization/MultiFace/FTHelper2.h", "FaceTrackingVisualization/MultiFace/MultiFace.cpp", "FaceTrackingVisualization/MultiFace/MultiFace.h", "FaceTrackingVisualization/MultiFace/MultiFace.ico", "FaceTrackingVisualization/MultiFace/MultiFace.rc", "FaceTrackingVisualization/MultiFace/MultiFace.vcxproj", "FaceTrackingVisualization/MultiFace/MultiFace.vcxproj.filters", "FaceTrackingVisualization/MultiFace/ReadMe.txt", "FaceTrackingVisualization/MultiFace/Resource.h", "FaceTrackingVisualization/MultiFace/stdafx.cpp", "FaceTrackingVisualization/MultiFace/stdafx.h"]}

## Source file: FaceTrackingVisualization/MultiFace/MultiFace.h

{
  "path": "FaceTrackingVisualization/MultiFace/MultiFace.h",
  "blob_sha": "7a32793d38bc7094feb21c61fe55e6e7e31bfc88",
  "sha256": "db6304da79cd206f78ce986f2fc07bf61384ee993bd2872e2de1e799072c059d",
  "lines": 8,
  "bytes": 339,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3,
    "face": 1
  },
  "url": "https://github.com/AlanChatham/SFMKinectFacialAnimation/blob/2e49253c786b27f3e826b066740f262709bdcb92/FaceTrackingVisualization/MultiFace/MultiFace.h",
  "function_signature_candidates": []
}

```text

﻿//------------------------------------------------------------------------------
// <copyright file="Multiface.cpp" company="Microsoft">
//     Copyright (c) Microsoft Corporation.  All rights reserved.
// </copyright>
//------------------------------------------------------------------------------
#pragma once

#include "resource.h"


```

## Source file: FaceTrackingBasics-WPF/FaceTrackingViewer.xaml.cs

{
  "path": "FaceTrackingBasics-WPF/FaceTrackingViewer.xaml.cs",
  "blob_sha": "ec74c7283dda26f0b29e0e0ab128804d99c12222",
  "sha256": "8a7408de18a79fd30c129780f37ff8c45d552f4cbf99e550562af03c53255d9b",
  "lines": 363,
  "bytes": 13218,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3,
    "face": 66
  },
  "url": "https://github.com/AlanChatham/SFMKinectFacialAnimation/blob/2e49253c786b27f3e826b066740f262709bdcb92/FaceTrackingBasics-WPF/FaceTrackingViewer.xaml.cs",
  "function_signature_candidates": [
    {
      "line": 72,
      "text": "public void Dispose()"
    },
    {
      "line": 78,
      "text": "protected virtual void Dispose(bool disposing)"
    },
    {
      "line": 88,
      "text": "protected override void OnRender(DrawingContext drawingContext)"
    },
    {
      "line": 97,
      "text": "private void OnAllFramesReady(object sender, AllFramesReadyEventArgs allFramesReadyEventArgs)"
    },
    {
      "line": 196,
      "text": "private void OnSensorChanged(KinectSensor oldSensor, KinectSensor newSensor)"
    },
    {
      "line": 213,
      "text": "private void RemoveOldTrackers(int currentFrameNumber)"
    },
    {
      "line": 233,
      "text": "private void RemoveTracker(int trackingId)"
    },
    {
      "line": 239,
      "text": "private void ResetFaceTracking()"
    },
    {
      "line": 261,
      "text": "public void Dispose()"
    },
    {
      "line": 270,
      "text": "public void DrawFaceModel(DrawingContext drawingContext)"
    },
    {
      "line": 310,
      "text": "internal void OnFrameReady(KinectSensor kinectSensor, ColorImageFormat colorImageFormat, byte[] colorImage, DepthImageFormat depthImageFormat, short[] depthImage, Skeleton skeletonOfInterest)"
    }
  ]
}

```text

﻿// --------------------------------------------------------------------------------------------------------------------
// <copyright file="FaceTrackingViewer.xaml.cs" company="Microsoft">
//     Copyright (c) Microsoft Corporation.  All rights reserved.
// </copyright>
// --------------------------------------------------------------------------------------------------------------------

namespace FaceTrackingBasics
{
    using System;
    using System.Collections.Generic;
    using System.Diagnostics;
    using System.Windows;
    using System.Windows.Controls;
    using System.Windows.Media;
    using Microsoft.Kinect;
    using Microsoft.Kinect.Toolkit.FaceTracking;

    using Point = System.Windows.Point;

    /// <summary>
    /// Class that uses the Face Tracking SDK to display a face mask for
    /// tracked skeletons
    /// </summary>
    public partial class FaceTrackingViewer : UserControl, IDisposable
    {
        public static readonly DependencyProperty KinectProperty = DependencyProperty.Register(
            "Kinect", 
            typeof(KinectSensor), 
            typeof(FaceTrackingViewer), 
            new PropertyMetadata(
                null, (o, args) => ((FaceTrackingViewer)o).OnSensorChanged((KinectSensor)args.OldValue, (KinectSensor)args.NewValue)));

        private const uint MaxMissedFrames = 100;

        private readonly Dictionary<int, SkeletonFaceTracker> trackedSkeletons = new Dictionary<int, SkeletonFaceTracker>();

        private byte[] colorImage;

        private ColorImageFormat colorImageFormat = ColorImageFormat.Undefined;

        private short[] depthImage;

        private DepthImageFormat depthImageFormat = DepthImageFormat.Undefined;

        private bool disposed;

        private Skeleton[] skeletonData;

        public FaceTrackingViewer()
        {
            this.InitializeComponent();
        }

        ~FaceTrackingViewer()
        {
            this.Dispose(false);
        }

        public KinectSensor Kinect
        {
            get
            {
                return (KinectSensor)this.GetValue(KinectProperty);
            }

            set
            {
                this.SetValue(KinectProperty, value);
            }
        }

        public void Dispose()
        {
            this.Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                this.ResetFaceTracking();

                this.disposed = true;
            }
        }

        protected override void OnRender(DrawingContext drawingContext)
        {
            base.OnRender(drawingContext);
            foreach (SkeletonFaceTracker faceInformation in this.trackedSkeletons.Values)
            {
                faceInformation.DrawFaceModel(drawingContext);
            }
        }

        private void OnAllFramesReady(object sender, AllFramesReadyEventArgs allFramesReadyEventArgs)
        {
            ColorImageFrame colorImageFrame = null;
            DepthImageFrame depthImageFrame = null;
            SkeletonFrame skeletonFrame = null;

            try
            {
                colorImageFrame = allFramesReadyEventArgs.OpenColorImageFrame();
                depthImageFrame = allFramesReadyEventArgs.OpenDepthImageFrame();
                skeletonFrame = allFramesReadyEventArgs.OpenSkeletonFrame();

                if (colorImageFrame == null || depthImageFrame == null || skeletonFrame == null)
                {
                    return;
                }

                // Check for image format changes.  The FaceTracker doesn't
                // deal with that so we need to reset.
                if (this.depthImageFormat != depthImageFrame.Format)
                {
                    this.ResetFaceTracking();
                    this.depthImage = null;
                    this.depthImageFormat = depthImageFrame.Format;
                }

                if (this.colorImageFormat != colorImageFrame.Format)
                {
                    this.ResetFaceTracking();
                    this.colorImage = null;
                    this.colorImageFormat = colorImageFrame.Format;
                }

                // Create any buffers to store copies of the data we work with
                if (this.depthImage == null)
                {
                    this.depthImage = new short[depthImageFrame.PixelDataLength];
                }

                if (this.colorImage == null)
                {
                    this.colorImage = new byte[colorImageFrame.PixelDataLength];
                }
                
                // Get the skeleton information
                if (this.skeletonData == null || this.skeletonData.Length != skeletonFrame.SkeletonArrayLength)
                {
                    this.skeletonData = new Skeleton[skeletonFrame.SkeletonArrayLength];
                }

                colorImageFrame.CopyPixelDataTo(this.colorImage);
                depthImageFrame.CopyPixelDataTo(this.depthImage);
                skeletonFrame.CopySkeletonDataTo(this.skeletonData);

                // Update the list of trackers and the trackers with the current frame information
                foreach (Skeleton skeleton in this.skeletonData)
                {
                    if (skeleton.TrackingState == SkeletonTrackingState.Tracked
                        || skeleton.TrackingState == SkeletonTrackingState.PositionOnly)
                    {
                        // We want keep a record of any skeleton, tracked or untracked.
                        if (!this.trackedSkeletons.ContainsKey(skeleton.TrackingId))
                        {
                            this.trackedSkeletons.Add(skeleton.TrackingId, new SkeletonFaceTracker());
                        }

                        // Give each tracker the upated frame.
                        SkeletonFaceTracker skeletonFaceTracker;
                        if (this.trackedSkeletons.TryGetValue(skeleton.TrackingId, out skeletonFaceTracker))
                        {
                            skeletonFaceTracker.OnFrameReady(this.Kinect, colorImageFormat, colorImage, depthImageFormat, depthImage, skeleton);
                            skeletonFaceTracker.LastTrackedFrame = skeletonFrame.FrameNumber;
                        }
                    }
                }

                this.RemoveOldTrackers(skeletonFrame.FrameNumber);

                this.InvalidateVisual();
            }
            finally
            {
                if (colorImageFrame != null)
                {
                    colorImageFrame.Dispose();
                }

                if (depthImageFrame != null)
                {
                    depthImageFrame.Dispose();
                }

                if (skeletonFrame != null)
                {
                    skeletonFrame.Dispose();
                }
            }
        }

        private void OnSensorChanged(KinectSensor oldSensor, KinectSensor newSensor)
        {
            if (oldSensor != null)
            {
                oldSensor.AllFramesReady -= this.OnAllFramesReady;
                this.ResetFaceTracking();
            }

            if (newSensor != null)
            {
                newSensor.AllFramesReady += this.OnAllFramesReady;
            }
        }

        /// <summary>
        /// Clear out any trackers for skeletons we haven't heard from for a while
        /// </summary>
        private void RemoveOldTrackers(int currentFrameNumber)
        {
            var trackersToRemove = new List<int>();

            foreach (var tracker in this.trackedSkeletons)
            {
                uint missedFrames = (uint)currentFrameNumber - (uint)tracker.Value.LastTrackedFrame;
                if (missedFrames > MaxMissedFrames)
                {
                    // There have been too many frames since we last saw this skeleton
                    trackersToRemove.Add(tracker.Key);
                }
            }

            foreach (int trackingId in trackersToRemove)
            {
                this.RemoveTracker(trackingId);
            }
        }

        private void RemoveTracker(int trackingId)
        {
            this.trackedSkeletons[trackingId].Dispose();
            this.trackedSkeletons.Remove(trackingId);
        }

        private void ResetFaceTracking()
        {
            foreach (int trackingId in new List<int>(this.trackedSkeletons.Keys))
            {
                this.RemoveTracker(trackingId);
            }
        }

        private class SkeletonFaceTracker : IDisposable
        {
            private static FaceTriangle[] faceTriangles;

            private EnumIndexableCollection<FeaturePoint, PointF> facePoints;

            private FaceTracker faceTracker;

            private bool lastFaceTrackSucceeded;

            private SkeletonTrackingState skeletonTrackingState;

            public int LastTrackedFrame { get; set; }

            public void Dispose()
            {
                if (this.faceTracker != null)
                {
                    this.faceTracker.Dispose();
                    this.faceTracker = null;
                }
            }

            public void DrawFaceModel(DrawingContext drawingContext)
            {
                if (!this.lastFaceTrackSucceeded || this.skeletonTrackingState != SkeletonTrackingState.Tracked)
                {
                    return;
                }

                var faceModelPts = new List<Point>();
                var faceModel = new List<FaceModelTriangle>();

                for (int i = 0; i < this.facePoints.Count; i++)
                {
                    faceModelPts.Add(new Point(this.facePoints[i].X + 0.5f, this.facePoints[i].Y + 0.5f));
                }

                foreach (var t in faceTriangles)
                {
                    var triangle = new FaceModelTriangle();
                    triangle.P1 = faceModelPts[t.First];
                    triangle.P2 = faceModelPts[t.Second];
                    triangle.P3 = faceModelPts[t.Third];
                    faceModel.Add(triangle);
                }

                var faceModelGroup = new GeometryGroup();
                for (int i = 0; i < faceModel.Count; i++)
                {
                    var faceTriangle = new GeometryGroup();
                    faceTriangle.Children.Add(new LineGeometry(faceModel[i].P1, faceModel[i].P2));
                    faceTriangle.Children.Add(new LineGeometry(faceModel[i].P2, faceModel[i].P3));
                    faceTriangle.Children.Add(new LineGeometry(faceModel[i].P3, faceModel[i].P1));
                    faceModelGroup.Children.Add(faceTriangle);
                }

                drawingContext.DrawGeometry(Brushes.LightYellow, new Pen(Brushes.LightYellow, 1.0), faceModelGroup);
            }

            /// <summary>
            /// Updates the face tracking information for this skeleton
            /// </summary>
            internal void OnFrameReady(KinectSensor kinectSensor, ColorImageFormat colorImageFormat, byte[] colorImage, DepthImageFormat depthImageFormat, short[] depthImage, Skeleton skeletonOfInterest)
            {
                this.skeletonTrackingState = skeletonOfInterest.TrackingState;

                if (this.skeletonTrackingState != SkeletonTrackingState.Tracked)
                {
                    // nothing to do with an untracked skeleton.
                    return;
                }

                if (this.faceTracker == null)
                {
                    try
                    {
                        this.faceTracker = new FaceTracker(kinectSensor);
                    }
                    catch (InvalidOperationException)
                    {
                        // During some shutdown scenarios the FaceTracker
                        // is unable to be instantiated.  Catch that exception
                        // and don't track a face.
                        Debug.WriteLine("AllFramesReady - creating a new FaceTracker threw an InvalidOperationException");
                        this.faceTracker = null;
                    }
                }

                if (this.faceTracker != null)
                {
                    FaceTrackFrame frame = this.faceTracker.Track(
                        colorImageFormat, colorImage, depthImageFormat, depthImage, skeletonOfInterest);

                    this.lastFaceTrackSucceeded = frame.TrackSuccessful;
                    if (this.lastFaceTrackSucceeded)
                    {
                        if (faceTriangles == null)
                        {
                            // only need to get this once.  It doesn't change.
                            faceTriangles = frame.GetTriangles();
                        }

                        this.facePoints = frame.GetProjected3DShape();
                    }
                }
            }

            private struct FaceModelTriangle
            {
                public Point P1;
                public Point P2;
                public Point P3;
            }
        }
    }
}

```

## Source file: FaceTrackingVisualization/MultiFace/MultiFace.cpp

{
  "path": "FaceTrackingVisualization/MultiFace/MultiFace.cpp",
  "blob_sha": "db948660d1a4e34a4c023a9cf32f765e0d7eff71",
  "sha256": "ba0001213e230a7ee4684ab35320874803ab47a796584edf3837b55846e29c10",
  "lines": 616,
  "bytes": 21840,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 17,
    "face": 36
  },
  "url": "https://github.com/AlanChatham/SFMKinectFacialAnimation/blob/2e49253c786b27f3e826b066740f262709bdcb92/FaceTrackingVisualization/MultiFace/MultiFace.cpp",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "int Run(HINSTANCE hInst, PWSTR lpCmdLine, int nCmdShow);"
    },
    {
      "line": 24,
      "text": "void                        ParseCmdString(PWSTR lpCmdLine);"
    },
    {
      "line": 25,
      "text": "void                        UninitInstance();"
    },
    {
      "line": 33,
      "text": "static void                 FTHelperCallingBack(LPVOID lpParam, UINT userId);"
    },
    {
      "line": 34,
      "text": "static void                 FTHelperUserSelection(PVOID pVoid, KinectSensor * pKinectSensor, UINT nbUsers, FTHelperContext* pUserContexts);"
    }
  ]
}

```text

﻿//------------------------------------------------------------------------------
// <copyright file="MultiFace.cpp" company="Microsoft">
//     Copyright (c) Microsoft Corporation.  All rights reserved.
// </copyright>
//------------------------------------------------------------------------------

// MultiFace.cpp : Defines the entry point for the application.

#include "stdafx.h"
#include "MultiFace.h"
#include "EggAvatar.h"
#include <FaceTrackLib.h>
#include "FTHelper2.h"

class MultiFace
{
public:
    MultiFace();

    int Run(HINSTANCE hInst, PWSTR lpCmdLine, int nCmdShow);

protected:
    BOOL                        InitInstance(HINSTANCE hInst, PWSTR lpCmdLine, int nCmdShow);
    void                        ParseCmdString(PWSTR lpCmdLine);
    void                        UninitInstance();
    ATOM                        RegisterClass(PCWSTR szWindowClass);
    static LRESULT CALLBACK     WndProcStatic(HWND hwnd, UINT message, WPARAM wParam, LPARAM lParam);
    LRESULT CALLBACK            WndProc(HWND hwnd, UINT message, WPARAM wParam, LPARAM lParam);
    static INT_PTR CALLBACK     About(HWND hwnd, UINT message, WPARAM wParam, LPARAM lParam);
    BOOL                        PaintWindow(HDC hdc, HWND hWnd);
    BOOL                        ShowVideo(HDC hdc, int width, int height, int originX, int originY);
    BOOL                        ShowEggAvatar(HDC hdc, int width, int height, int originX, int originY, UINT avatarId);
    static void                 FTHelperCallingBack(LPVOID lpParam, UINT userId);
    static void                 FTHelperUserSelection(PVOID pVoid, KinectSensor * pKinectSensor, UINT nbUsers, FTHelperContext* pUserContexts);
    static int const            MaxLoadStringChars = 100;

    HINSTANCE                   m_hInst;
    HWND                        m_hWnd;
    HACCEL                      m_hAccelTable;
    UINT                        m_nbUsers;
    EggAvatar*                  m_eggavatar;
    FTHelper2                   m_FTHelper;
    IFTImage**                  m_pImageBuffer;
    IFTImage*                   m_pVideoBuffer;
    NUI_IMAGE_TYPE              m_depthType;
    NUI_IMAGE_TYPE              m_colorType;
    NUI_IMAGE_RESOLUTION        m_depthRes;
    NUI_IMAGE_RESOLUTION        m_colorRes;
    BOOL                        m_bNearMode;
	BOOL                        m_bSeatedSkeletonMode;
};

MultiFace::MultiFace(): m_hInst(NULL), 
    m_hWnd(NULL), 
    m_nbUsers(2),
    m_hAccelTable(NULL), 
    m_pVideoBuffer(NULL),
    m_pImageBuffer(NULL),
    m_eggavatar(NULL),
    m_depthType(NUI_IMAGE_TYPE_DEPTH_AND_PLAYER_INDEX),
    m_colorType(NUI_IMAGE_TYPE_COLOR),
    m_depthRes(NUI_IMAGE_RESOLUTION_320x240),
    m_colorRes(NUI_IMAGE_RESOLUTION_640x480),
    m_bNearMode(FALSE),
	m_bSeatedSkeletonMode(FALSE)
{
}
 

// Run the MultiFace application.
int MultiFace::Run(HINSTANCE hInst, PWSTR lpCmdLine, int nCmdShow)
{
    MSG msg = {static_cast<HWND>(0), static_cast<UINT>(0), static_cast<WPARAM>(-1)};
    if (InitInstance(hInst, lpCmdLine, nCmdShow))
    {
        // Main message loop:
        while (GetMessage(&msg, NULL, 0, 0))
        {
            if (!TranslateAccelerator(msg.hwnd, m_hAccelTable, &msg))
            {
                TranslateMessage(&msg);
                DispatchMessage(&msg);
            }
        }
    }
    UninitInstance();

    return (int)msg.wParam;
}

// In this function, we save the instance handle, then create and display the main program window.
BOOL MultiFace::InitInstance(HINSTANCE hInstance, PWSTR lpCmdLine, int nCmdShow)
{
    ParseCmdString(lpCmdLine);

    m_eggavatar = new EggAvatar[m_nbUsers];
    m_pImageBuffer = new IFTImage*[m_nbUsers];
    
    m_hInst = hInstance; // Store instance handle in our global variable

    WCHAR szTitle[MaxLoadStringChars];                  // The title bar text
    LoadString(m_hInst, IDS_APP_TITLE, szTitle, ARRAYSIZE(szTitle));

    static const PCWSTR RES_MAP[] = { L"80x60", L"320x240", L"640x480", L"1280x960" };
    static const PCWSTR IMG_MAP[] = { L"PLAYERID", L"RGB", L"YUV", L"YUV_RAW", L"DEPTH" };

    // Add mode params in title
    WCHAR szTitleComplete[MAX_PATH];
    swprintf_s(szTitleComplete, L"%s -- Depth:%s:%s Color:%s:%s NearMode:%s SeatedSkeleton:%s", szTitle,
        IMG_MAP[m_depthType], (m_depthRes < 0)? L"ERROR": RES_MAP[m_depthRes], IMG_MAP[m_colorType], (m_colorRes < 0)? L"ERROR": RES_MAP[m_colorRes], m_bNearMode? L"ON": L"OFF", m_bSeatedSkeletonMode? L"ON": L"OFF");

    WCHAR szWindowClass[MaxLoadStringChars];            // the main window class name
    LoadString(m_hInst, IDC_MULTIFACE, szWindowClass, ARRAYSIZE(szWindowClass));

    RegisterClass(szWindowClass);

    m_hAccelTable = LoadAccelerators(hInstance, MAKEINTRESOURCE(IDC_MULTIFACE));

    for (UINT i=0; i<m_nbUsers; i++)
    {
        m_pImageBuffer[i] = FTCreateImage();
    }
    m_pVideoBuffer = FTCreateImage();

    m_hWnd = CreateWindow(szWindowClass, szTitleComplete, WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, 0, CW_USEDEFAULT, 0, NULL, NULL, m_hInst, this);
    if (!m_hWnd)
    {
        return FALSE;
    }
   
   ShowWindow(m_hWnd, nCmdShow);
   UpdateWindow(m_hWnd);

   // Start the face tracking.
   return SUCCEEDED(m_FTHelper.Init(m_hWnd, m_nbUsers, FTHelperCallingBack, this, FTHelperUserSelection, this, m_depthType, m_depthRes, m_bNearMode, m_colorType, m_colorRes, m_bSeatedSkeletonMode));
}

void MultiFace::UninitInstance()
{
    // Clean up the memory allocated for Face Tracking and rendering.
    m_FTHelper.Stop();

    if (m_hAccelTable)
    {
        DestroyAcceleratorTable(m_hAccelTable);
        m_hAccelTable = NULL;
    }

    DestroyWindow(m_hWnd);
    m_hWnd = NULL;

    if (m_pImageBuffer)
    {
        for (UINT i=0; i<m_nbUsers; i++)
        {
            if (m_pImageBuffer[i])
            {
                m_pImageBuffer[i]->Release();
                m_pImageBuffer[i] = NULL;
            }
        }
        delete[] m_pImageBuffer;
    }

    if (m_pVideoBuffer)
    {
        m_pVideoBuffer->Release();
        m_pVideoBuffer = NULL;
    }

    if (m_eggavatar)
    {
        delete[] m_eggavatar;
    }
}


// Register the window class.
ATOM MultiFace::RegisterClass(PCWSTR szWindowClass)
{
    WNDCLASSEX wcex = {0};

    wcex.cbSize = sizeof(WNDCLASSEX);

    wcex.style          = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc    = &MultiFace::WndProcStatic;
    wcex.cbClsExtra     = 0;
    wcex.cbWndExtra     = 0;
    wcex.hInstance      = m_hInst;
    wcex.hIcon          = LoadIcon(m_hInst, MAKEINTRESOURCE(IDI_MULTIFACE));
    wcex.hCursor        = LoadCursor(NULL, IDC_ARROW);
    wcex.hbrBackground  = (HBRUSH)(COLOR_WINDOW+1);
    wcex.lpszMenuName   = MAKEINTRESOURCE(IDC_MULTIFACE);
    wcex.lpszClassName  = szWindowClass;

    return RegisterClassEx(&wcex);
}

LRESULT CALLBACK MultiFace::WndProcStatic(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam)
{
    static MultiFace* pThis = NULL; // cheating, but since there is just one window now, it will suffice.
    if (WM_CREATE == message)
    {
        pThis = reinterpret_cast<MultiFace*>(reinterpret_cast<CREATESTRUCT*>(lParam)->lpCreateParams);
    }
    return pThis ? pThis->WndProc(hWnd, message, wParam, lParam) : DefWindowProc(hWnd, message, wParam, lParam);
}

//  FUNCTION: WndProc(HWND, UINT, WPARAM, LPARAM)
//
//  PURPOSE:  Processes messages for the main window.
//
//  WM_COMMAND  - process the application menu
//  WM_KEYUP    - Exit in response to ESC key
//  WM_PAINT    - Paint the main window
//  WM_DESTROY  - post a quit message and return
LRESULT CALLBACK MultiFace::WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam)
{
    int wmId, wmEvent;
    PAINTSTRUCT ps;
    HDC hdc;

    switch (message)
    {
    case WM_COMMAND:
        wmId    = LOWORD(wParam);
        wmEvent = HIWORD(wParam);
        // Parse the menu selections:
        switch (wmId)
        {
        case IDM_ABOUT:
            DialogBox(m_hInst, MAKEINTRESOURCE(IDD_ABOUTBOX), hWnd, About);
            break;
        case IDM_EXIT:
            PostQuitMessage(0);
            break;
        default:
            return DefWindowProc(hWnd, message, wParam, lParam);
        }
        break;
    case WM_KEYUP:
        if (wParam == VK_ESCAPE)
        {
            PostQuitMessage(0);
        }
        break;
    case WM_PAINT:
        hdc = BeginPaint(hWnd, &ps);
        // Draw the avatar window and the video window
        PaintWindow(hdc, hWnd);
        EndPaint(hWnd, &ps);
        break;
    case WM_DESTROY:
        PostQuitMessage(0);
        break;
    default:
        return DefWindowProc(hWnd, message, wParam, lParam);
    }
    return 0;
}

// Message handler for about box.
INT_PTR CALLBACK MultiFace::About(HWND hDlg, UINT message, WPARAM wParam, LPARAM lParam)
{
    UNREFERENCED_PARAMETER(lParam);
    switch (message)
    {
    case WM_INITDIALOG:
        return (INT_PTR)TRUE;

    case WM_COMMAND:
        if (LOWORD(wParam) == IDOK || LOWORD(wParam) == IDCANCEL)
        {
            EndDialog(hDlg, LOWORD(wParam));
            return (INT_PTR)TRUE;
        }
        break;
    }
    return (INT_PTR)FALSE;
}

// Drawing the video window
BOOL MultiFace::ShowVideo(HDC hdc, int width, int height, int originX, int originY)
{
    BOOL ret = TRUE;

    // Now, copy a fraction of the camera image into the screen.
    IFTImage* colorImage = m_FTHelper.GetColorImage();
    if (colorImage)
    {
        int iWidth = colorImage->GetWidth();
        int iHeight = colorImage->GetHeight();
        if (iWidth > 0 && iHeight > 0)
        {
            int iTop = 0;
            int iBottom = iHeight;
            int iLeft = 0;
            int iRight = iWidth;

            // Keep a separate buffer.
            if (m_pVideoBuffer && SUCCEEDED(m_pVideoBuffer->Allocate(iWidth, iHeight, FTIMAGEFORMAT_UINT8_B8G8R8A8)))
            {
                // Copy do the video buffer while converting bytes
                colorImage->CopyTo(m_pVideoBuffer, NULL, 0, 0);

                // Compute the best approximate copy ratio.
                float w1 = (float)iHeight * (float)width;
                float w2 = (float)iWidth * (float)height;
                if (w2 > w1 && height > 0)
                {
                    // video image too wide
                    float wx = w1/height;
                    iLeft = (int)max(0, m_FTHelper.GetXCenterFace() - wx / 2);
                    iRight = iLeft + (int)wx;
                    if (iRight > iWidth)
                    {
                        iRight = iWidth;
                        iLeft = iRight - (int)wx;
                    }
                }
                else if (w1 > w2 && width > 0)
                {
                    // video image too narrow
                    float hy = w2/width;
                    iTop = (int)max(0, m_FTHelper.GetYCenterFace() - hy / 2);
                    iBottom = iTop + (int)hy;
                    if (iBottom > iHeight)
                    {
                        iBottom = iHeight;
                        iTop = iBottom - (int)hy;
                    }
                }

                int const bmpPixSize = m_pVideoBuffer->GetBytesPerPixel();
                SetStretchBltMode(hdc, HALFTONE);
                BITMAPINFO bmi = {sizeof(BITMAPINFO), static_cast<LONG>(iWidth), static_cast<LONG>(iHeight), static_cast<WORD>(1), static_cast<WORD>(bmpPixSize * CHAR_BIT), BI_RGB, m_pVideoBuffer->GetStride() * iHeight, 5000, 5000, 0, 0};
                if (0 == StretchDIBits(hdc, originX, originY, width, height,
                    iLeft, iBottom, iRight-iLeft, iTop-iBottom, m_pVideoBuffer->GetBuffer(), &bmi, DIB_RGB_COLORS, SRCCOPY))
                {
                    ret = FALSE;
                }
            }
        }
    }
    return ret;
}

// Drawing code
BOOL MultiFace::ShowEggAvatar(HDC hdc, int width, int height, int originX, int originY, UINT avatarId)
{
    static int errCount = 0;
    BOOL ret = FALSE;

    if (m_pImageBuffer[avatarId] && SUCCEEDED(m_pImageBuffer[avatarId]->Allocate(width, height, FTIMAGEFORMAT_UINT8_B8G8R8A8)))
    {
        memset(m_pImageBuffer[avatarId]->GetBuffer(), 0, m_pImageBuffer[avatarId]->GetStride() * height); // clear to black

        m_eggavatar[avatarId].SetScaleAndTranslationToWindow(height, width);
        m_eggavatar[avatarId].DrawImage(m_pImageBuffer[avatarId]);

        BITMAPINFO bmi = {sizeof(BITMAPINFO), width, height, 1, static_cast<WORD>(m_pImageBuffer[avatarId]->GetBytesPerPixel() * CHAR_BIT), BI_RGB, m_pImageBuffer[avatarId]->GetStride() * height, 5000, 5000, 0, 0};
        errCount += (0 == StretchDIBits(hdc, originX, originY, width, height, 0, 0, width, height, m_pImageBuffer[avatarId]->GetBuffer(), &bmi, DIB_RGB_COLORS, SRCCOPY));

        ret = TRUE;
    }

    return ret;
}

// Draw the egg head and the camera video with the mask superimposed.
BOOL MultiFace::PaintWindow(HDC hdc, HWND hWnd)
{
    static int errCount = 0;
    BOOL ret = FALSE;
    RECT rect;
    GetClientRect(hWnd, &rect);
    int width = rect.right - rect.left;
    int height = rect.bottom - rect.top;
    int halfWidth = width/2;

    // Show the video on the right of the window
    errCount += !ShowVideo(hdc, width - halfWidth, height, halfWidth, 0);

    // Draw the egg avatar on the left of the window
    int avatarHeight = height/m_nbUsers;
    for (UINT i=0; i<m_nbUsers; i++)
    {
        errCount += !ShowEggAvatar(hdc, halfWidth, avatarHeight, 0, i*avatarHeight, i);
    }
    return ret;
}

/*
 * The "FT Helper" class is generic. It will call back this function
 * after color image, skeleton and depth have been acquired by Kinect. 
 * The code needs to document the skeleton corresponding to each user context,
 * or set the "count until failure" paarmeter of the user context to zero
 * if the context shall not be tracked.
 */
void MultiFace::FTHelperUserSelection(PVOID pVoid, KinectSensor * pKinectSensor,
    UINT nbUsers, FTHelperContext* pUserContexts)
{
    MultiFace* pApp = reinterpret_cast<MultiFace*>(pVoid);
    if (!pApp)
        return;
    // Initialize an array of the available skeletons
    bool SkeletonIsAvailable[NUI_SKELETON_COUNT];
    for (UINT i=0; i<NUI_SKELETON_COUNT; i++)
    {
        SkeletonIsAvailable[i] = pKinectSensor->IsTracked(i);
    }
    // If the user's skeleton is still tracked, mark it unavailable
    // and make sure we will keep associating the user context to that skeleton
    // If the skeleton is not track anaymore, decrease a counter until we 
    // deassociate the user context from that skeleton id.
    for (UINT i=0; i<nbUsers; i++)
    {
        if (pUserContexts[i].m_CountUntilFailure > 0)
        {
            if (SkeletonIsAvailable[pUserContexts[i].m_SkeletonId])
            {
                SkeletonIsAvailable[pUserContexts[i].m_SkeletonId] = false;
                pUserContexts[i].m_CountUntilFailure++;
                if (pUserContexts[i].m_CountUntilFailure > 5)
                {
                    pUserContexts[i].m_CountUntilFailure = 5;
                }
            }
            else
            {
                pUserContexts[i].m_CountUntilFailure--;
            }
        }
    }

    // Try to find an available skeleton for users who do not have one
    for (UINT i=0; i<nbUsers; i++)
    {
        if (pUserContexts[i].m_CountUntilFailure == 0)
        {
            for (UINT j=0; j<NUI_SKELETON_COUNT; j++)
            {
                if (SkeletonIsAvailable[j])
                {
                    pUserContexts[i].m_SkeletonId = j;
                    pUserContexts[i].m_CountUntilFailure = 1;
                    SkeletonIsAvailable[j] = false;
                    break;
                }
            }
        }
    }
}

/*
 * The "FT Helper" class is generic. It will call back this function
 * after a face has been successfully tracked. The code in the call back passes the parameters
 * to the Egg Avatar, so it can be animated.
 */
void MultiFace::FTHelperCallingBack(PVOID pVoid, UINT userId)
{
    MultiFace* pApp = reinterpret_cast<MultiFace*>(pVoid);
    if (pApp)
    {
        IFTResult* pResult = pApp->m_FTHelper.GetResult(userId);
        if (pResult && SUCCEEDED(pResult->GetStatus()))
        {
            FLOAT* pAU = NULL;
            UINT numAU;
            pResult->GetAUCoefficients(&pAU, &numAU);
            pApp->m_eggavatar[userId].SetCandideAU(pAU, numAU);
            FLOAT scale;
            FLOAT rotationXYZ[3];
            FLOAT translationXYZ[3];
            pResult->Get3DPose(&scale, rotationXYZ, translationXYZ);
            pApp->m_eggavatar[userId].SetTranslations(translationXYZ[0], translationXYZ[1], translationXYZ[2]);
            pApp->m_eggavatar[userId].SetRotations(rotationXYZ[0], rotationXYZ[1], rotationXYZ[2]);
        }
    }
}

void MultiFace::ParseCmdString(PWSTR lpCmdLine)
{
    const WCHAR KEY_USERS[]                                 = L"-Users";
    const WCHAR KEY_DEPTH[]                                 = L"-Depth";
    const WCHAR KEY_COLOR[]                                 = L"-Color";
    const WCHAR KEY_NEAR_MODE[]                             = L"-NearMode";
    const WCHAR KEY_SEATED_SKELETON_MODE[]                  = L"-SeatedSkeleton";

    const WCHAR STR_NUI_IMAGE_TYPE_DEPTH[]                  = L"DEPTH";
    const WCHAR STR_NUI_IMAGE_TYPE_DEPTH_AND_PLAYER_INDEX[] = L"PLAYERID";
    const WCHAR STR_NUI_IMAGE_TYPE_COLOR[]                  = L"RGB";
    const WCHAR STR_NUI_IMAGE_TYPE_COLOR_YUV[]              = L"YUV";

    const WCHAR STR_NUI_IMAGE_RESOLUTION_80x60[]            = L"80x60";
    const WCHAR STR_NUI_IMAGE_RESOLUTION_320x240[]          = L"320x240";
    const WCHAR STR_NUI_IMAGE_RESOLUTION_640x480[]          = L"640x480";
    const WCHAR STR_NUI_IMAGE_RESOLUTION_1280x960[]         = L"1280x960";

    enum TOKEN_ENUM
    {
        TOKEN_ERROR,
        TOKEN_USERS,
        TOKEN_DEPTH,
        TOKEN_COLOR,
        TOKEN_NEARMODE,
		TOKEN_SEATEDSKELETON
    }; 

    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(lpCmdLine, &argc);

    for(int i = 0; i < argc; i++)
    {
        NUI_IMAGE_TYPE* pType = NULL;
        NUI_IMAGE_RESOLUTION* pRes = NULL;

        TOKEN_ENUM tokenType = TOKEN_ERROR; 
        PWCHAR context = NULL;
        PWCHAR token = wcstok_s(argv[i], L":", &context);
        if(0 == wcsncmp(token, KEY_DEPTH, ARRAYSIZE(KEY_DEPTH)))
        {
            tokenType = TOKEN_DEPTH;
            pType = &m_depthType;
            pRes = &m_depthRes;
        }
        else if(0 == wcsncmp(token, KEY_USERS, ARRAYSIZE(KEY_USERS)))
        {
            tokenType = TOKEN_USERS;
        }
        else if(0 == wcsncmp(token, KEY_COLOR, ARRAYSIZE(KEY_COLOR)))
        {
            tokenType = TOKEN_COLOR;
            pType = &m_colorType;
            pRes = &m_colorRes;
        }
        else if(0 == wcsncmp(token, KEY_NEAR_MODE, ARRAYSIZE(KEY_NEAR_MODE)))
        {
            tokenType = TOKEN_NEARMODE;
            m_bNearMode = TRUE;
        }
        else if(0 == wcsncmp(token, KEY_SEATED_SKELETON_MODE, ARRAYSIZE(KEY_NEAR_MODE)))
        {
            tokenType = TOKEN_SEATEDSKELETON;
            m_bSeatedSkeletonMode = TRUE;
        }

        if(tokenType == TOKEN_USERS)
        {
            while((token = wcstok_s(NULL, L":", &context)) != NULL)
            {
                UINT users = _wtoi(token);
                if(users > m_nbUsers)
                {
                    m_nbUsers = users;
                    break;
                }
            }
        }
        else if(tokenType == TOKEN_DEPTH || tokenType == TOKEN_COLOR)
        {
            _ASSERT(pType != NULL && pRes != NULL);

            while((token = wcstok_s(NULL, L":", &context)) != NULL)
            {
                if(0 == wcsncmp(token, STR_NUI_IMAGE_TYPE_DEPTH, ARRAYSIZE(STR_NUI_IMAGE_TYPE_DEPTH)))
                {
                    *pType = NUI_IMAGE_TYPE_DEPTH;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_TYPE_DEPTH_AND_PLAYER_INDEX, ARRAYSIZE(STR_NUI_IMAGE_TYPE_DEPTH_AND_PLAYER_INDEX)))
                {
                    *pType = NUI_IMAGE_TYPE_DEPTH_AND_PLAYER_INDEX;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_TYPE_COLOR, ARRAYSIZE(STR_NUI_IMAGE_TYPE_COLOR)))
                {
                    *pType = NUI_IMAGE_TYPE_COLOR;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_TYPE_COLOR_YUV, ARRAYSIZE(STR_NUI_IMAGE_TYPE_COLOR_YUV)))
                {
                    *pType = NUI_IMAGE_TYPE_COLOR_YUV;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_RESOLUTION_80x60, ARRAYSIZE(STR_NUI_IMAGE_RESOLUTION_80x60)))
                {
                    *pRes = NUI_IMAGE_RESOLUTION_80x60;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_RESOLUTION_320x240, ARRAYSIZE(STR_NUI_IMAGE_RESOLUTION_320x240)))
                {
                    *pRes = NUI_IMAGE_RESOLUTION_320x240;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_RESOLUTION_640x480, ARRAYSIZE(STR_NUI_IMAGE_RESOLUTION_640x480)))
                {
                    *pRes = NUI_IMAGE_RESOLUTION_640x480;
                }
                else if(0 == wcsncmp(token, STR_NUI_IMAGE_RESOLUTION_1280x960, ARRAYSIZE(STR_NUI_IMAGE_RESOLUTION_1280x960)))
                {
                    *pRes = NUI_IMAGE_RESOLUTION_1280x960;
                }
            }
        }
    }

    if(argv) LocalFree(argv);
}


// Program's main entry point
int APIENTRY wWinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, PWSTR lpCmdLine, int nCmdShow)
{
    UNREFERENCED_PARAMETER(hPrevInstance);
    MultiFace app;

    HeapSetInformation(NULL, HeapEnableTerminationOnCorruption, NULL, 0);

    return app.Run(hInstance, lpCmdLine, nCmdShow);
}


```

## Source file: FaceTrackingVisualization/SingleFace/SingleFace.h

{
  "path": "FaceTrackingVisualization/SingleFace/SingleFace.h",
  "blob_sha": "5e576035af26e1f1a5800837aa1b4222d5b64f72",
  "sha256": "e0f1a1cf42e554c6b30915127ea39c830827c2c6028319c344310787d5dca6b6",
  "lines": 8,
  "bytes": 338,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3,
    "face": 1
  },
  "url": "https://github.com/AlanChatham/SFMKinectFacialAnimation/blob/2e49253c786b27f3e826b066740f262709bdcb92/FaceTrackingVisualization/SingleFace/SingleFace.h",
  "function_signature_candidates": []
}

```text

﻿//------------------------------------------------------------------------------
// <copyright file="SingleFace.h" company="Microsoft">
//     Copyright (c) Microsoft Corporation.  All rights reserved.
// </copyright>
//------------------------------------------------------------------------------
#pragma once

#include "resource.h"


```
