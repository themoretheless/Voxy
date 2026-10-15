# Facial rendering source scan: Shuisho10/VRC_iFacialMocap

Repository: https://github.com/Shuisho10/VRC_iFacialMocap

Pinned source revision: a919b9e76d31687fd0078e381aa802b0cc6cdf96

Discovery topic: facial mocap

Description: Tracking interface for iFacialMocap.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["VRC_iFacialMocap/src/iFacialMocapTrackingInterface.cs"]}

## Source file: VRC_iFacialMocap/src/iFacialMocapTrackingInterface.cs

{
  "path": "VRC_iFacialMocap/src/iFacialMocapTrackingInterface.cs",
  "blob_sha": "f54ca4e6f78356ede9c8d35cb1861f8d42f49309",
  "sha256": "5d456336961822c64961d23d60a58321b517fa7f32f554fb0f6ec0b26a17d399",
  "lines": 197,
  "bytes": 13732,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 30,
    "jaw": 5,
    "mouth": 42,
    "tongue": 2,
    "rig": 36,
    "face": 86,
    "lip": 14
  },
  "url": "https://github.com/Shuisho10/VRC_iFacialMocap/blob/a919b9e76d31687fd0078e381aa802b0cc6cdf96/VRC_iFacialMocap/src/iFacialMocapTrackingInterface.cs",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "public override void Update()"
    },
    {
      "line": 60,
      "text": "public override void Teardown()"
    },
    {
      "line": 66,
      "text": "void UpdateEyeData()"
    },
    {
      "line": 108,
      "text": "void UpdateMouthData()"
    }
  ]
}

```text

using iFacialMocapTrackingModule;
using VRCFaceTracking;
using VRCFaceTracking.Core.Library;
using VRCFaceTracking.Core.Params.Expressions;

public class iFacialMocapTrackingInterface : ExtTrackingModule
{
    iFacialMocapServer? server;
    // What your interface is able to send as tracking data.
    public override (bool SupportsEye, bool SupportsExpression) Supported => (true, true);

    // active tracking support
    private (bool, bool) _trackingSupported = (false, false);

    // This is the first function ran by VRCFaceTracking. Make sure to completely initialize 
    // your tracking interface or the data to be accepted by VRCFaceTracking here. This will let 
    // VRCFaceTracking know what data is available to be sent from your tracking interface at initialization.
    public override (bool eyeSuccess, bool expressionSuccess) Initialize(bool eyeAvailable, bool expressionAvailable)
    {
        ModuleInformation.Name = "iFacialMocap";

        // Example of an embedded image stream being referenced as a stream
        var stream = GetType().Assembly.GetManifestResourceStream("VRCFT_iFacialMocap.res.logo.png");


        // Setting the stream to be referenced by VRCFaceTracking.
        ModuleInformation.StaticImages =
            stream != null ? new List<Stream> { stream } : ModuleInformation.StaticImages;

        //... Initializing module. Modify state tuple as needed (or use bool contexts to determine what should be initialized).
        server = new iFacialMocapServer(ref Logger);
        server.Connect();
        // only initialize what is available
        _trackingSupported = (server.isTracking && eyeAvailable, server.isTracking && expressionAvailable);
        return _trackingSupported;
    }

    // Polls data from the tracking interface.
    // VRCFaceTracking will run this function in a separate thread;
    public override void Update()
    {
        // Get latest tracking data from interface and transform to VRCFaceTracking data.
        //server.ReadData(ref Logger);
        if (server != null && server.isTracking && Status == ModuleState.Active)
        {
            if (_trackingSupported.Item1)
            {
                UpdateEyeData();
            }
            if (_trackingSupported.Item2)
            {
                UpdateMouthData();
            }
        }
        // updates 250 times a second because there's no way someone is using a 240Hz camera and a model that outputs more than that.. 
        Thread.Sleep(4);
    }

    // Called when the module is unloaded or VRCFaceTracking itself tears down.
    public override void Teardown()
    {
        //... Deinitialize tracking interface; dispose any data created with the module.
        server?.Stop();
    }

    void UpdateEyeData()
    {
        //Could make a dict<UnifiedExpressions,string> or directly assigning Data.Shapes for better performance but can do math here so whatever for now.
        #region Eye Gaze
        // positive x is to the *right*
        //UnifiedTracking.Data.Eye.Left.Gaze.x = server.FaceData.BlendValue("eyeLookIn_L") - server.FaceData.BlendValue("eyeLookOut_L");
        //UnifiedTracking.Data.Eye.Left.Gaze.y = server.FaceData.BlendValue("eyeLookUp_L") - server.FaceData.BlendValue("eyeLookDown_L");
        //UnifiedTracking.Data.Eye.Right.Gaze.x = server.FaceData.BlendValue("eyeLookOut_R") - server.FaceData.BlendValue("eyeLookIn_R");
        //UnifiedTracking.Data.Eye.Right.Gaze.y = server.FaceData.BlendValue("eyeLookUp_R") - server.FaceData.BlendValue("eyeLookDown_R");

        // coordinate system is all wacky
        UnifiedTracking.Data.Eye.Left.Gaze.x = MathF.Tan(server.FaceData.leftEye[1] / 90.0f); // normalized range of -45 to 45 degrees, tan function
        UnifiedTracking.Data.Eye.Left.Gaze.y = -MathF.Tan(server.FaceData.leftEye[0] / 90.0f);
        UnifiedTracking.Data.Eye.Right.Gaze.x = MathF.Tan(server.FaceData.rightEye[1] / 90.0f);
        UnifiedTracking.Data.Eye.Right.Gaze.y = -MathF.Tan(server.FaceData.rightEye[0] / 90.0f);
        #endregion
        #region Eye Openness
        UnifiedTracking.Data.Eye.Left.Openness = 1.0f - (float)Math.Max(0, Math.Min(1, server.FaceData.BlendValue("eyeBlink_R") +
                server.FaceData.BlendValue("eyeBlink_R") * server.FaceData.BlendValue("eyeSquint_R")));

        UnifiedTracking.Data.Eye.Right.Openness = 1.0f - (float)Math.Max(0, Math.Min(1, server.FaceData.BlendValue("eyeBlink_L") +
                server.FaceData.BlendValue("eyeBlink_L") * server.FaceData.BlendValue("eyeSquint_L")));

        #endregion

        //// ===== iFacialMocap output by default (from ARKit) mirrors R/L because it's normal use-case is in a application acting as a mirror!!! ===== ////

        #region Eye Blends
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.EyeSquintLeft].Weight = server.FaceData.BlendValue("eyeSquint_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.EyeSquintRight].Weight = server.FaceData.BlendValue("eyeSquint_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.EyeWideLeft].Weight = server.FaceData.BlendValue("eyeWide_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.EyeWideRight].Weight = server.FaceData.BlendValue("eyeWide_L");
        #endregion
        #region Pupil
        //EyeDilation & EyeConstrict default in mid value idk
        UnifiedTracking.Data.Eye.Left.PupilDiameter_MM = 5f;
        UnifiedTracking.Data.Eye.Right.PupilDiameter_MM = 5f;
        UnifiedTracking.Data.Eye._minDilation = 0;
        UnifiedTracking.Data.Eye._maxDilation = 10;
        #endregion
    }

    void UpdateMouthData()
    {
        #region Eye Brow
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowInnerUpLeft].Weight = server.FaceData.BlendValue("browInnerUp_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowLowererLeft].Weight = server.FaceData.BlendValue("browDown_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowOuterUpLeft].Weight = server.FaceData.BlendValue("browOuterUp_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowPinchLeft].Weight = server.FaceData.BlendValue("browDown_R");

        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowInnerUpRight].Weight = server.FaceData.BlendValue("browInnerUp_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowLowererRight].Weight = server.FaceData.BlendValue("browDown_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowOuterUpRight].Weight = server.FaceData.BlendValue("browOuterUp_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.BrowPinchRight].Weight = server.FaceData.BlendValue("browDown_L");
        #endregion 
        #region Nose
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.NoseSneerLeft].Weight = server.FaceData.BlendValue("noseSneer_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.NoseSneerRight].Weight = server.FaceData.BlendValue("noseSneer_L");
        //Default NasalDitalation & NasalConstrict
        #endregion
        #region Cheek
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.CheekPuffLeft].Weight = server.FaceData.BlendValue("cheekPuff");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.CheekSquintLeft].Weight = server.FaceData.BlendValue("cheekSquint_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.CheekPuffRight].Weight = server.FaceData.BlendValue("cheekPuff");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.CheekSquintRight].Weight = server.FaceData.BlendValue("cheekSquint_L");
        //No CheekSuck'ing lol
        #endregion
        #region Mewing
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.JawLeft].Weight = server.FaceData.BlendValue("jawRight");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.JawRight].Weight = server.FaceData.BlendValue("jawLeft");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.JawOpen].Weight = server.FaceData.BlendValue("jawOpen");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthClosed].Weight = server.FaceData.BlendValue("mouthClose");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.JawForward].Weight = server.FaceData.BlendValue("jawForward");
        //Default JawBackward, JawClench & JawMandibleRaise
        #endregion
        #region Lip 
        //lips expressions = mouth expressions
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipPuckerUpperLeft].Weight = server.FaceData.BlendValue("mouthPucker");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipPuckerLowerLeft].Weight = server.FaceData.BlendValue("mouthPucker");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipPuckerUpperRight].Weight = server.FaceData.BlendValue("mouthPucker");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipPuckerLowerRight].Weight = server.FaceData.BlendValue("mouthPucker");

        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipFunnelUpperLeft].Weight = server.FaceData.BlendValue("mouthFunnel");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipFunnelLowerLeft].Weight = server.FaceData.BlendValue("mouthFunnel");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipFunnelUpperRight].Weight = server.FaceData.BlendValue("mouthFunnel");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipFunnelLowerRight].Weight = server.FaceData.BlendValue("mouthFunnel");

        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipSuckUpperLeft].Weight = Math.Min(
            1f - (float)Math.Pow(server.FaceData.BlendValue("mouthUpperUp_R"), 1 / 6f),
            server.FaceData.BlendValue("mouthRollUpper")
        );
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipSuckLowerLeft].Weight = server.FaceData.BlendValue("mouthRollLower");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipSuckUpperRight].Weight = Math.Min(
            1f - (float)Math.Pow(server.FaceData.BlendValue("mouthUpperUp_L"), 1 / 6f),
            server.FaceData.BlendValue("mouthRollUpper")
        );
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.LipSuckLowerRight].Weight = server.FaceData.BlendValue("mouthRollLower");
        #endregion
        #region Mouth
        //not sure if appropiate ussage of shrug
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthRaiserLower].Weight = server.FaceData.BlendValue("mouthShrugLower");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthRaiserUpper].Weight = server.FaceData.BlendValue("mouthShrugUpper");

        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthUpperLeft].Weight = server.FaceData.BlendValue("mouthRight");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthLowerLeft].Weight = server.FaceData.BlendValue("mouthRight");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthUpperUpLeft].Weight = server.FaceData.BlendValue("mouthUpperUp_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthLowerDownLeft].Weight = server.FaceData.BlendValue("mouthLowerDown_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthCornerPullLeft].Weight = server.FaceData.BlendValue("mouthSmile_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthLowerDownLeft].Weight = server.FaceData.BlendValue("mouthLowerDown_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthDimpleLeft].Weight = server.FaceData.BlendValue("mouthDimple_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthFrownLeft].Weight = server.FaceData.BlendValue("mouthFrown_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthPressLeft].Weight = server.FaceData.BlendValue("mouthPress_R");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthStretchLeft].Weight = server.FaceData.BlendValue("mouthStretch_R");

        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthUpperRight].Weight = server.FaceData.BlendValue("mouthLeft");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthLowerRight].Weight = server.FaceData.BlendValue("mouthLeft");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthUpperUpRight].Weight = server.FaceData.BlendValue("mouthUpperUp_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthLowerDownRight].Weight = server.FaceData.BlendValue("mouthLowerDown_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthCornerPullRight].Weight = server.FaceData.BlendValue("mouthSmile_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthLowerDownRight].Weight = server.FaceData.BlendValue("mouthLowerDown_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthDimpleRight].Weight = server.FaceData.BlendValue("mouthDimple_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthFrownRight].Weight = server.FaceData.BlendValue("mouthFrown_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthPressRight].Weight = server.FaceData.BlendValue("mouthPress_L");
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.MouthStretchRight].Weight = server.FaceData.BlendValue("mouthStretch_L");

        //Default MouthUpperDeepenLeft, MouthCornerSlantLeft & MouthTightenerLeft
        #endregion
        #region Tongue
        UnifiedTracking.Data.Shapes[(int)UnifiedExpressions.TongueOut].Weight = server.FaceData.BlendValue("tongueOut");
        #endregion
    }
}

```

## Source file: VRC_iFacialMocap/src/FacialData.cs

{
  "path": "VRC_iFacialMocap/src/FacialData.cs",
  "blob_sha": "6401b592c028373ddf91e23149525406f4d26af5",
  "sha256": "6431f9e61a5476616842e2478b1db1faa29e48e70fc269e4837275c1d3c009a7",
  "lines": 22,
  "bytes": 1201,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "jaw": 1,
    "mouth": 1,
    "tongue": 1,
    "rig": 2
  },
  "url": "https://github.com/Shuisho10/VRC_iFacialMocap/blob/a919b9e76d31687fd0078e381aa802b0cc6cdf96/VRC_iFacialMocap/src/FacialData.cs",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "public float BlendValue(string blend){"
    }
  ]
}

```text

using System.Collections.Generic;

namespace iFacialMocapTrackingModule
{
    //
    public class FacialMocapData{
        public Dictionary<string,int> blends = new();
        public float[] head = new float[]{0,0,0,0,0,0};
        public float[] rightEye = new float[]{0,0,0};
        public float[] leftEye = new float[]{0,0,0};
        
        public float BlendValue(string blend){
            if (blends.ContainsKey(blend))
                return blends[blend]/100f;
            else
                return 0f;
        }
    }

    //mouthSmile_R;eyeLookOut_L;mouthUpperUp_L;eyeWide_R;mouthClose;mouthPucker;mouthRollLower;eyeBlink_R;eyeLookDown_L;cheekSquint_R;eyeBlink_L;tongueOut;jawRight;eyeLookIn_R;cheekSquint_L;mouthDimple_L;mouthPress_L;eyeSquint_L;mouthRight;mouthShrugLower;eyeLookUp_R;eyeLookOut_R;mouthPress_R;cheekPuff;jawForward;mouthLowerDown_L;mouthFrown_L;mouthShrugUpper;browOuterUp_L;browInnerUp;mouthDimple_R;browDown_R;mouthUpperUp_R;mouthRollUpper;mouthFunnel;mouthStretch_R;mouthFrown_R;eyeLookDown_R;jawOpen;jawLeft;browDown_L;mouthSmile_L;noseSneer_R;mouthLowerDown_R;noseSneer_L;eyeWide_L;mouthStretch_L;browOuterUp_R;eyeLookIn_L;eyeSquint_R;eyeLookUp_L;mouthLeft;
}



```

## Source file: VRC_iFacialMocap/src/iFacialMocapServer.cs

{
  "path": "VRC_iFacialMocap/src/iFacialMocapServer.cs",
  "blob_sha": "f56911a363b60afef8e88a67f3fa8b20403874c6",
  "sha256": "a350c50d685d19f85f4a81089da8f3e6f7d2391dd761276a80d2508f925803a0",
  "lines": 304,
  "bytes": 12420,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 6,
    "rig": 3,
    "face": 3,
    "lip": 2
  },
  "url": "https://github.com/Shuisho10/VRC_iFacialMocap/blob/a919b9e76d31687fd0078e381aa802b0cc6cdf96/VRC_iFacialMocap/src/iFacialMocapServer.cs",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "public void Stop()"
    },
    {
      "line": 59,
      "text": "public bool Connect(string ipaddress = \"255.255.255.255\")"
    },
    {
      "line": 233,
      "text": "private void HandleChange(string blend)"
    }
  ]
}

```text

using System.Diagnostics;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Text;
using Microsoft.Extensions.Logging;


namespace iFacialMocapTrackingModule
{
    class iFacialMocapServer
    {
        static private int _port = 49983; //port
        private FacialMocapData _trackedData = new();
        private UdpClient? _udpListener, _udpClient;
        private IPEndPoint? _remoteIpEndPoint;
        public bool isTracking;
        private ILogger _logger;

        private Thread? _receiveThread;
        private CancellationTokenSource _cts;

        // call it a disconnect if we stop receiving after 5 seconds
        private int _receiveTimeoutMs = 5000;

        public FacialMocapData FaceData { get { return _trackedData; } }

        public iFacialMocapServer(ref ILogger logger) { 
            _logger = logger; 
            _cts = new CancellationTokenSource();
        }

        /// <summary>
        /// Stops and disposes the clients
        public void Stop()
        {
            _cts.Cancel();
            if (_udpClient != null) { _udpClient.Close(); _udpClient.Dispose(); }
            if (_udpListener != null) { _udpListener.Close(); _udpListener.Dispose(); }
            FaceData.blends.Clear();
        }
        /// </summary>
        public static IEnumerable<string> GetLocalIPAddresses()
        {
            var host = Dns.GetHostEntry(Dns.GetHostName());
            foreach (var ip in host.AddressList)
            {
                if (ip.AddressFamily == AddressFamily.InterNetwork)
                {
                    yield return ip.ToString();
                }
            }
        }

        /// <summary>
        /// Connects to the Facial Mockup server socket.
        /// </summary>
        /// <param name="ipaddress"></param>
        public bool Connect(string ipaddress = "255.255.255.255")
        {
            isTracking = false;

            _udpListener = new(_port);
            _udpClient = new();
            _remoteIpEndPoint = new IPEndPoint(IPAddress.Any, _port);

            var timeToWait = TimeSpan.FromSeconds(120);
            // compile list of IP addresses
            string likelyIpAddressList = "";
            string otherIpAddressList = "";
            foreach (string ip in GetLocalIPAddresses())
            {
                if (!string.IsNullOrEmpty(ip))
                {
                    // reference for expected internal IP address format: https://www.okta.com/identity-101/internal-ip/
                    string[] addressBytes = ip.Split('.');
                    string networkPart = $"{addressBytes[0]}.{addressBytes[1]}";

                    // 192.168.0.0 to 192.168.255.255, which offers about 65,000 unique IP addresses 
                    // 10.0.0.0 to 10.255.255.255, a range that provides up to 16 million unique IP addresses
                    if (networkPart.Equals("192.168") || addressBytes[0].Equals("10"))
                    {
                        likelyIpAddressList += $"\n\t{ip}";
                    }
                    else if (addressBytes[0].Equals("172"))
                    {
                        // convert addressBytes[1] to integer
                        int addressByte1 = 0;
                        try
                        {
                            addressByte1 = int.Parse(addressBytes[1]);
                        }
                        catch (FormatException)
                        {
                            _logger.LogError("Invalid IP address happened somehow..");
                            continue;
                        }
                        // 172.16.0.0 to 172.31.255.255, providing about 1 million unique IP addresses 
                        if (addressByte1 >= 16 && addressByte1 <= 31)
                        {
                            likelyIpAddressList += $"\n\t{ip}";
                        }
                    }
                    else
                    {
                        otherIpAddressList += $"\n\t{ip}";
                    }
                }
            }
            if (likelyIpAddressList.Length == 0 && otherIpAddressList.Length == 0)
            {
                _logger.LogError("No local network connection found!");
                return false;
            }
            _logger.LogInformation($"Seeking iFacialMocap connection for {timeToWait.TotalSeconds} seconds. " +
                $"Accepting data on: \n\nIP Address(es)\n========================{likelyIpAddressList}\n========================\n\nUse default iFacialMocap Port: {_port}\n");
            _logger.LogDebug($"Other IP Addresses found: \n{otherIpAddressList}\n");

            var asyncResult = _udpListener.BeginReceive(null, null);

            asyncResult.AsyncWaitHandle.WaitOne(timeToWait);
            if (asyncResult.IsCompleted)
            {
                try
                {
                    // EndReceive worked and we have received data and remote endpoint
                    byte[] receivedBytes = _udpListener.EndReceive(asyncResult, ref _remoteIpEndPoint);
                    _logger.LogInformation("Successful message receive");

                    //IPEndPoint dstAddr = new(IPAddress.Parse(ipaddress), _port);
                    string data = "iFacialMocap_sahuasouryya9218sauhuiayeta91555dy3719|sendDataVersion=v2";
                    byte[] bytes = Encoding.UTF8.GetBytes(data);
                    _udpClient.Send(bytes, bytes.Length, _remoteIpEndPoint);
                    // for synchronous receives, which ReceiveAsync won't use...
                    _udpListener.Client.ReceiveTimeout = 1000;
                    if (_remoteIpEndPoint == null)
                    {
                        _logger.LogWarning("Something went really wrong! Could not identify remote endpoint");
                        return false;
                    }
                    _logger.LogInformation($"Connecting to {_remoteIpEndPoint.Address}:{_remoteIpEndPoint.Port}");
                    isTracking = true;

                    // start async thread for receive
                    //_receiveThread = new Thread(async () => await ReadDataThread());
                    //_receiveThread.Start();
                    Task.Run(() => ReadDataThread());

                    return isTracking;
                }
                catch (Exception ex)
                {
                    // EndReceive failed and we ended up here
                    _logger.LogError($"Error Occurred Attempting Receiving Data: {ex.ToString()}");
                }
            }
            else
            {
                // The operation wasn't completed before the timeout and we're off the hook
                // nothing init so return false
                _logger.LogWarning("Did not receive iFacialMocap message within initialization period, re-initialize the module to try again...");
                return false;
            }

            return false;
        }

        private async Task ReadDataThread()
        {
            _logger.LogDebug("Starting iFacialMocap Receive thread");

            while (!_cts.IsCancellationRequested)
            {
                await ReadData();
            }
            _logger.LogDebug("iFacialMocap receive thread ended");
        }

        /// <summary>
        /// Reads and parses the data received by the UDP Client, 
        /// storing the facial data result in the Face Data attributes.
        /// </summary>

        public async Task ReadData()
        {
           if (_udpListener != null)
           {
                //IPEndPoint? RemoteIpEndPoint = null;
                try {
                    CancellationTokenSource receiveTimeOutCTS = new();
                    receiveTimeOutCTS.CancelAfter(_receiveTimeoutMs);
                    CancellationTokenSource linkedCTS = CancellationTokenSource.CreateLinkedTokenSource(receiveTimeOutCTS.Token, _cts.Token);
                    var receiveTask = _udpListener.ReceiveAsync(linkedCTS.Token);
                    await receiveTask;
                    if (!receiveTask.IsCompleted)
                    {
                        throw new TimeoutException();
                    }
                    string returnData = Encoding.ASCII.GetString(receiveTask.Result.Buffer);
                    if (isTracking == false)
                    {
                        isTracking = true;
                        _logger.LogInformation("iFacialMocap message received. Datastream re-established..");
                    }
                    string[] blendData = returnData.Split('|')[1..^1];
                    int i = 0;
                    while (i < blendData.Length) //While in the int attributes
                    {
                        HandleChange(blendData[i]);
                        i++;
                    }
                }
                catch(Exception e)
                {
                    if (isTracking)
                    {
                        _logger.LogError("Module failed to receive message from iFacialMocap after 5 seconds, waiting for reconnection...");
                    }
                    isTracking = false;
                }

           }
           else
           {
               _logger.LogError("UDPClient wasn't initialized.");
           }
        }

        /// <summary>
        /// Changes the facial data depending of the assignation received.
        /// </summary>
        /// <param name="blend"></param>
        private void HandleChange(string blend)
        {
            if (blend.Contains('#'))
            {
                string[] assignVal = blend.Split('#');
                try
                {
                    string[] unparsedValues = assignVal[1].Split(',');
                    float[] values = new float[unparsedValues.Length];

                    for (int j = 0; j < unparsedValues.Length; j++)
                    {
                        values[j] = float.Parse(unparsedValues[j], CultureInfo.InvariantCulture.NumberFormat);
                    }
                    if (assignVal[0] == "=head")
                    {
                        if (values.Length == 6)
                            _trackedData.head = values;
                        else
                            _logger.LogWarning("Insufficient data to assign head's position");
                    }
                    else if (assignVal[0] == "rightEye")
                    {
                        if (values.Length == 3)
                            _trackedData.rightEye = values;
                        else
                            _logger.LogWarning("Insufficient data to assign right eye's position");
                    }
                    else if (assignVal[0] == "leftEye")
                    {
                        if (values.Length == 3)
                            _trackedData.leftEye = values;
                        else
                            _logger.LogWarning("Insufficient data to assign left eye's position");
                    }
                    else
                    {
                        _logger.LogWarning($"Error on setting {assignVal}.");
                        return;
                    }
                }
                catch (Exception e)
                {
                    _logger.LogWarning($"Invalid assignation. [{e};;{blend}]");
                    return;
                }

            }
            else if (blend.Contains('-') || blend.Contains('&'))
            {
                char separator = blend.Contains('&') ? '&' : '-';
                string[] assignVal = blend.Split(separator);
                try
                {
                    _trackedData.blends[assignVal[0]] = int.Parse(assignVal[1]);

                }
                catch (Exception e)
                {
                    _logger.LogWarning($"Invalid assignation. [{e};;{blend}]");
                    return;
                }
            }

            else
            {
                _logger.LogWarning($"Data cropped.");
            }
        }

    }
}


```
