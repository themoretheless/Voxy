# Facial rendering source scan: w0lfv/iFacialMocapTester

Repository: https://github.com/w0lfv/iFacialMocapTester

Pinned source revision: 38ea051d02e6d5ec2342f654e97744007201421d

Discovery topic: facial mocap

Description: iFacialMocap Testing Tool

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: Assets/Scripts/FacialTester.cs

{
  "path": "Assets/Scripts/FacialTester.cs",
  "blob_sha": "40a81804ae0c408deaa4b0c7c52c8e0ffe7b9ae8",
  "sha256": "8fa1d391b61f520950879c4ef58ff57fcf60f8ebab4d9abeb7c4fc262d3538b8",
  "lines": 112,
  "bytes": 6558,
  "selection_score": -2,
  "keyword_line_counts": {
    "eye": 17,
    "jaw": 5,
    "mouth": 24,
    "tongue": 2,
    "rig": 4
  },
  "url": "https://github.com/w0lfv/iFacialMocapTester/blob/38ea051d02e6d5ec2342f654e97744007201421d/Assets/Scripts/FacialTester.cs",
  "function_signature_candidates": [
    {
      "line": 77,
      "text": "private void Start()"
    },
    {
      "line": 91,
      "text": "private void Update()"
    },
    {
      "line": 97,
      "text": "private void OnDestroy()"
    },
    {
      "line": 102,
      "text": "private void Listener()"
    }
  ]
}

```text

using System.Collections;
using System.Collections.Generic;
using System.Net.Sockets;
using System.Net;
using UnityEngine;
using System.Text;
using System.Threading.Tasks;

public class FacialTester : MonoBehaviour
{
    [SerializeField] private string _ipAddress = "127.0.0.1";

    [SerializeField, Range(0, 100)] private int _mouthSmile_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookOut_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthUpperUp_L = 0;
    [SerializeField, Range(0, 100)] private int _eyeWide_R = 0;
    [SerializeField, Range(0, 100)] private int _mouthClose = 0;
    [SerializeField, Range(0, 100)] private int _mouthPucker = 0;
    [SerializeField, Range(0, 100)] private int _mouthRollLower = 0;
    [SerializeField, Range(0, 100)] private int _eyeBlink_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookDown_L = 0;
    [SerializeField, Range(0, 100)] private int _cheekSquint_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeBlink_L = 0;
    [SerializeField, Range(0, 100)] private int _tongueOut = 0;
    [SerializeField, Range(0, 100)] private int _jawRight = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookIn_R = 0;
    [SerializeField, Range(0, 100)] private int _cheekSquint_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthDimple_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthPress_L = 0;
    [SerializeField, Range(0, 100)] private int _eyeSquint_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthRight = 0;
    [SerializeField, Range(0, 100)] private int _mouthShrugLower = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookUp_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookOut_R = 0;
    [SerializeField, Range(0, 100)] private int _mouthPress_R = 0;
    [SerializeField, Range(0, 100)] private int _cheekPuff = 0;
    [SerializeField, Range(0, 100)] private int _jawForward = 0;
    [SerializeField, Range(0, 100)] private int _mouthLowerDown_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthFrown_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthShrugUpper = 0;
    [SerializeField, Range(0, 100)] private int _browOuterUp_L = 0;
    [SerializeField, Range(0, 100)] private int _browInnerUp = 0;
    [SerializeField, Range(0, 100)] private int _mouthDimple_R = 0;
    [SerializeField, Range(0, 100)] private int _browDown_R = 0;
    [SerializeField, Range(0, 100)] private int _mouthUpperUp_R = 0;
    [SerializeField, Range(0, 100)] private int _mouthRollUpper = 0;
    [SerializeField, Range(0, 100)] private int _mouthFunnel = 0;
    [SerializeField, Range(0, 100)] private int _mouthStretch_R = 0;
    [SerializeField, Range(0, 100)] private int _mouthFrown_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookDown_R = 0;
    [SerializeField, Range(0, 100)] private int _jawOpen = 0;
    [SerializeField, Range(0, 100)] private int _jawLeft = 0;
    [SerializeField, Range(0, 100)] private int _browDown_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthSmile_L = 0;
    [SerializeField, Range(0, 100)] private int _noseSneer_R = 0;
    [SerializeField, Range(0, 100)] private int _mouthLowerDown_R = 0;
    [SerializeField, Range(0, 100)] private int _noseSneer_L = 0;
    [SerializeField, Range(0, 100)] private int _eyeWide_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthStretch_L = 0;
    [SerializeField, Range(0, 100)] private int _browOuterUp_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookIn_L = 0;
    [SerializeField, Range(0, 100)] private int _eyeSquint_R = 0;
    [SerializeField, Range(0, 100)] private int _eyeLookUp_L = 0;
    [SerializeField, Range(0, 100)] private int _mouthLeft = 0;
    [SerializeField] private Vector3 _eyeLeftRot;
    [SerializeField] private Vector3 _eyeRightRot;

    private const ushort _port = 49983;
    private UdpClient _sock;
    private List<IPEndPoint> _targetClients;

    IPEndPoint _targetEndPoint;

    private bool _enabled;


    private void Start()
    {
        Application.targetFrameRate = 60;

        _sock = new UdpClient();
        _targetClients = new List<IPEndPoint>();

        _enabled = true;

        _targetEndPoint = new IPEndPoint(IPAddress.Parse(_ipAddress), _port);

        Task.Run(() => { Listener(); });
    }

    private void Update()
    {
        byte[] data = Encoding.UTF8.GetBytes($"mouthSmile_R-{_mouthSmile_R}|eyeLookOut_L-{_eyeLookOut_L}|mouthUpperUp_L-{_mouthUpperUp_L}|eyeWide_R-{_eyeWide_R}|mouthClose-{_mouthClose}|mouthPucker-{_mouthPucker}|mouthRollLower-{_mouthRollLower}|eyeBlink_R-{_eyeBlink_R}|eyeLookDown_L-{_eyeLookDown_L}|cheekSquint_R-{_cheekSquint_R}|eyeBlink_L-{_eyeBlink_L}|tongueOut-{_tongueOut}|jawRight-{_jawRight}|eyeLookIn_R-{_eyeLookIn_R}|cheekSquint_L-{_cheekSquint_L}|mouthDimple_L-{_mouthDimple_L}|mouthPress_L-{_mouthPress_L}|eyeSquint_L-{_eyeSquint_L}|mouthRight-{_mouthRight}|mouthShrugLower-{_mouthShrugLower}|eyeLookUp_R-{_eyeLookUp_R}|eyeLookOut_R-{_eyeLookOut_R}|mouthPress_R-{_mouthPress_R}|cheekPuff-{_cheekPuff}|jawForward-{_jawForward}|mouthLowerDown_L-{_mouthLowerDown_L}|mouthFrown_L-{_mouthFrown_L}|mouthShrugUpper-{_mouthShrugUpper}|browOuterUp_L-{_browOuterUp_L}|browInnerUp-{_browInnerUp}|mouthDimple_R-{_mouthDimple_R}|browDown_R-{_browDown_R}|mouthUpperUp_R-{_mouthUpperUp_R}|mouthRollUpper-{_mouthRollUpper}|mouthFunnel-{_mouthFunnel}|mouthStretch_R-{_mouthStretch_R}|mouthFrown_R-{_mouthFrown_R}|eyeLookDown_R-{_eyeLookDown_R}|jawOpen-{_jawOpen}|jawLeft-{_jawLeft}|browDown_L-{_browDown_L}|mouthSmile_L-{_mouthSmile_L}|noseSneer_R-{_noseSneer_R}|mouthLowerDown_R-{_mouthLowerDown_R}|noseSneer_L-{_noseSneer_L}|eyeWide_L-{_eyeWide_L}|mouthStretch_L-{_mouthStretch_L}|browOuterUp_R-{_browOuterUp_R}|eyeLookIn_L-{_eyeLookIn_L}|eyeSquint_R-{_eyeSquint_R}|eyeLookUp_L-{_eyeLookUp_L}|mouthLeft-{_mouthLeft}|=head#-21.488958,-6.038993,-6.6019735,-0.030653415,-0.10287084,-0.6584072|rightEye#{_eyeRightRot.x},{_eyeRightRot.y},{_eyeRightRot.z}|leftEye#{_eyeLeftRot.x},{_eyeLeftRot.y},{_eyeLeftRot.z}|");
        _sock.Send(data, data.Length, _targetEndPoint);
    }

    private void OnDestroy()
    {
        _enabled = false;
    }

    private void Listener()
    {
        IPEndPoint remoteEP = new IPEndPoint(IPAddress.Any, 0);
        while (_enabled)
        {
            string str = Encoding.UTF8.GetString(_sock.Receive(ref remoteEP));
            if(str == "iFacialMocap_sahuasouryya9218sauhuiayeta91555dy3719")
                _targetClients.Add(remoteEP);
        }
    }
}


```
