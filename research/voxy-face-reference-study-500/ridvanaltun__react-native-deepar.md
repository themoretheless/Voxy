# Facial rendering source scan: ridvanaltun/react-native-deepar

Repository: https://github.com/ridvanaltun/react-native-deepar

Pinned source revision: 869a6b57581e46b31e6db59b3e2f8443806207e1

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["android/src/main/java/com/reactnativedeepar/ARSurfaceProvider.java", "docs/images/face-painting.jpeg", "docs/interfaces.md", "example/android/app/src/main/assets/custom/ball_face", "example/android/app/src/main/assets/custom/face_painting", "example/android/app/src/main/assets/custom/manly_face", "example/android/app/src/main/assets/custom/tape_face", "example/assets/effects/ball_face", "example/assets/effects/face_painting", "example/assets/effects/manly_face", "example/assets/effects/tape_face"], "lip": ["example/android/app/src/debug/java/com/example/reactnativedeepar/ReactNativeFlipper.java"]}

## Source file: src/Utils.ts

{
  "path": "src/Utils.ts",
  "blob_sha": "6798825808bc040e4f1f134dbb5e446b347a8b2f",
  "sha256": "2125bf306407ab35c8e98158f0d4d56d75fb6bd4b7d432f8fc7f5ad20dadb5af",
  "lines": 5,
  "bytes": 96,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/ridvanaltun/react-native-deepar/blob/869a6b57581e46b31e6db59b3e2f8443806207e1/src/Utils.ts",
  "function_signature_candidates": []
}

```text

export default {
  strToBool: (str: string | undefined) => {
    return str === 'true';
  },
};


```

## Source file: babel.config.js

{
  "path": "babel.config.js",
  "blob_sha": "f842b77fcfb8b74016365099d8c5fe54cf3269e2",
  "sha256": "c245ebbd64e49fc21a06292497f189b29de85e51e15752e3b936a393e845ef2c",
  "lines": 3,
  "bytes": 77,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/ridvanaltun/react-native-deepar/blob/869a6b57581e46b31e6db59b3e2f8443806207e1/babel.config.js",
  "function_signature_candidates": []
}

```text

module.exports = {
  presets: ['module:metro-react-native-babel-preset'],
};


```

## Source file: ios/RNTDeepAR.h

{
  "path": "ios/RNTDeepAR.h",
  "blob_sha": "1d536e4732fa82746713088aab6a85f84774c1fa",
  "sha256": "6e773e4305cc2bb09a7e650d1f75e976539a034454fa3e53f925274bc3e1de40",
  "lines": 52,
  "bytes": 2129,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2,
    "face": 3
  },
  "url": "https://github.com/ridvanaltun/react-native-deepar/blob/869a6b57581e46b31e6db59b3e2f8443806207e1/ios/RNTDeepAR.h",
  "function_signature_candidates": []
}

```text

//
//  RNTDeepAR.h
//  RNTDeepAR
//
//  Created by Work on 11.06.2022.
//  Copyright © 2022 Facebook. All rights reserved.
//

#ifndef RNTDeepAR_h
#define RNTDeepAR_h

#import <AVKit/AVKit.h>
#import <ARView.h>
#import <React/RCTComponent.h>
#import <React/RCTView.h>

@interface RNTDeepAR
    : RCTView <ARViewDelegate, AVCaptureVideoDataOutputSampleBufferDelegate>

@property(nonatomic, assign) BOOL flashOn;
@property(nonatomic, strong) CameraController *cameraController;
@property(nonatomic, strong) DeepAR *deepar;
@property(nonatomic, strong) ARView *arview;
@property(nonatomic, copy) RCTBubblingEventBlock onEventSent;
@property(nonatomic, copy) NSString *apiKey;

- (void)pause;
- (void)resume;
- (void)startRecording:(NSDictionary *)options;
- (void)resumeRecording;
- (void)pauseRecording;
- (void)finishRecording;
- (void)setAudioMute:(BOOL)enabled;
- (void)takeScreenshot;
- (void)switchEffect:(NSString *)effect andSlot:(NSString *)slot;
- (void)switchEffectWithPath:(NSString *)path andSlot:(NSString *)slot;
- (void)fireTrigger:(NSString *)trigger;
- (void)setFlashOn:(BOOL)flashOn;
- (void)setLiveMode:(BOOL)liveMode;
- (void)setFaceDetectionSensitivity:(int)sensitivity;
- (void)showStats:(BOOL)enabled;
- (void)setTouchMode:(BOOL)enabled;
- (void)changeParameterFloat:(NSString*)gameObject component:(NSString*)component parameter:(NSString*)parameter floatValue:(float)value;
- (void)changeParameterVec4:(NSString*)gameObject component:(NSString*)component parameter:(NSString*)parameter vectorValue:(Vector4)value;
- (void)changeParameterVec3:(NSString*)gameObject component:(NSString*)component parameter:(NSString*)parameter vector3Value:(Vector3)value;
- (void)changeParameterBool:(NSString*)gameObject component:(NSString*)component parameter:(NSString*)parameter boolValue:(bool)value;
- (void)changeParameterTexture:(NSString*)gameObject component:(NSString*)component parameter:(NSString*)parameter image:(UIImage*)image;
- (void)changeParameterString:(NSString*)gameObject component:(NSString*)component parameter:(NSString*)parameter stringValue:(NSString *)value;

@end

#endif /* RNTDeepAR_h */


```

## Source file: scripts/bootstrap.js

{
  "path": "scripts/bootstrap.js",
  "blob_sha": "6ba193d30aef1230c4b4a6672657e8469aa4f7a9",
  "sha256": "0ff0dde0a9950618cb9b104ad3a24cdc4faca90d5e7668a48d440605a28c3dc4",
  "lines": 24,
  "bytes": 682,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 2
  },
  "url": "https://github.com/ridvanaltun/react-native-deepar/blob/869a6b57581e46b31e6db59b3e2f8443806207e1/scripts/bootstrap.js",
  "function_signature_candidates": []
}

```text

const path = require('path');
const child_process = require('child_process');

const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
const options = {
  cwd: process.cwd(),
  env: process.env,
  stdio: 'inherit',
  encoding: 'utf-8',
};

let result;

if (process.cwd() !== root || args.length) {
  // We're not in the root of the project, or additional arguments were passed
  // In this case, forward the command to `yarn`
  result = child_process.spawnSync('yarn', args, options);
} else {
  // If `yarn` is run without arguments, perform bootstrap
  result = child_process.spawnSync('yarn', ['bootstrap'], options);
}

process.exitCode = result.status;


```
