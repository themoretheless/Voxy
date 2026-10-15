# Facial rendering source scan: tajindervirk/FacePulse

Repository: https://github.com/tajindervirk/FacePulse

Pinned source revision: d3dfb9acdba670ffab74705474f557c9c219eed3

Discovery topic: FACS face

Description: Real-time, in-browser facial expression detection using MediaPipe. Features 13 distinct expressions powered by a FACS-based classifier, hand-gesture zoom, and a sleek spatial UI.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["src/components/FaceScanner.jsx", "src/hooks/useFaceExpression.js"]}

## Source file: src/hooks/useFaceExpression.js

{
  "path": "src/hooks/useFaceExpression.js",
  "blob_sha": "4efaa0ffad4a287a5f3a634f6809ff8f07c04c24",
  "sha256": "5730d4c445a9d06ac6883b9d485ea0de375ee600ad806e37824f10554897ec51",
  "lines": 196,
  "bytes": 7543,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 4,
    "blendshape": 4,
    "rig": 2,
    "face": 30,
    "lip": 2
  },
  "url": "https://github.com/tajindervirk/FacePulse/blob/d3dfb9acdba670ffab74705474f557c9c219eed3/src/hooks/useFaceExpression.js",
  "function_signature_candidates": []
}

```text

import { useEffect, useRef, useState } from 'react';
import { FaceLandmarker, HandLandmarker, FilesetResolver, DrawingUtils } from '@mediapipe/tasks-vision';
import { classifyExpression } from '../utils/expressionLogic';

export const useFaceExpression = (videoRef, canvasRef, overlayMode) => {
  const [faces, setFaces] = useState([]);
  const [pinchDistance, setPinchDistance] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);

  const landmarkerRef = useRef(null);
  const handLandmarkerRef = useRef(null);
  const animationRef = useRef(null);
  const lastVideoTime = useRef(-1);

  // Throttling state for UI and detection
  const lastUpdateTime = useRef(0);
  const THROTTLE_MS = 150; // 150ms throttle as requested

  useEffect(() => {
    let stream;
    let isActive = true;

    const init = async () => {
      try {
        setIsLoading(true);
        // Load WASM
        const vision = await FilesetResolver.forVisionTasks(
          "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm"
        );

        // Initialize Face Landmarker
        landmarkerRef.current = await FaceLandmarker.createFromOptions(vision, {
          baseOptions: {
            modelAssetPath: "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task",
            delegate: "GPU"
          },
          outputFaceBlendshapes: true,
          runningMode: "VIDEO",
          numFaces: 4
        });

        // Initialize Hand Landmarker
        handLandmarkerRef.current = await HandLandmarker.createFromOptions(vision, {
          baseOptions: {
            modelAssetPath: "https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/1/hand_landmarker.task",
            delegate: "GPU"
          },
          runningMode: "VIDEO",
          numHands: 1
        });

        // Start Camera
        stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: "user", width: 640, height: 480 }
        });
        
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          videoRef.current.onloadedmetadata = () => {
            if (isActive) {
              videoRef.current.play();
              detect();
            }
          };
        }
        
        setIsLoading(false);
      } catch (err) {
        console.error("Initialization error:", err);
        setError(err.message || "Failed to initialize camera or model.");
        setIsLoading(false);
      }
    };

    const drawOverlay = (faceResults, handResults) => {
      if (!canvasRef.current || !videoRef.current) return;
      const canvas = canvasRef.current;
      const ctx = canvas.getContext("2d");
      
      // Match canvas size to video
      canvas.width = videoRef.current.videoWidth;
      canvas.height = videoRef.current.videoHeight;
      
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      
      if (overlayMode === 'none') return;

      const drawingUtils = new DrawingUtils(ctx);
      
      if (faceResults.faceLandmarks) {
        for (const landmarks of faceResults.faceLandmarks) {
          if (overlayMode === 'mesh') {
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_TESSELATION, { color: "#C0C0C070", lineWidth: 1 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_RIGHT_EYE, { color: "#00E5FF", lineWidth: 1 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_LEFT_EYE, { color: "#00E5FF", lineWidth: 1 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_FACE_OVAL, { color: "#E0E0E0", lineWidth: 1 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_LIPS, { color: "#E0E0E0", lineWidth: 1 });
          } else if (overlayMode === 'contour') {
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_RIGHT_EYE, { color: "#00E5FF", lineWidth: 2 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_LEFT_EYE, { color: "#00E5FF", lineWidth: 2 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_FACE_OVAL, { color: "#E0E0E0", lineWidth: 2 });
            drawingUtils.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_LIPS, { color: "#E0E0E0", lineWidth: 2 });
          } else if (overlayMode === 'dots') {
            drawingUtils.drawLandmarks(landmarks, { color: "#00E5FF", radius: 1 });
          }
        }
      }

      if (handResults && handResults.landmarks) {
        for (const landmarks of handResults.landmarks) {
          drawingUtils.drawConnectors(landmarks, HandLandmarker.HAND_CONNECTIONS, { color: "#00E5FF", lineWidth: 2 });
          drawingUtils.drawLandmarks(landmarks, { color: "#E0E0E0", radius: 2 });
        }
      }
    };

    const detect = () => {
      if (!isActive || !landmarkerRef.current || !handLandmarkerRef.current || !videoRef.current) return;
      
      const video = videoRef.current;
      
      // Only process if video is playing and has valid dimensions
      if (video.readyState >= 2 && video.videoWidth > 0) {
        const now = performance.now();
        if (now - lastUpdateTime.current >= THROTTLE_MS) {
          lastUpdateTime.current = now;
          lastVideoTime.current = video.currentTime;
          
          try {
            const faceResults = landmarkerRef.current.detectForVideo(video, now);
            const handResults = handLandmarkerRef.current.detectForVideo(video, now);
            
            // Draw overlay
            drawOverlay(faceResults, handResults);

            // Face expressions
            if (faceResults.faceBlendshapes && faceResults.faceBlendshapes.length > 0) {
              const detectedFaces = faceResults.faceBlendshapes.map((face) => {
                const classification = classifyExpression(face.categories);
                return {
                  ...classification,
                  blendshapes: face.categories
                };
              });
              setFaces(detectedFaces);
            } else {
              setFaces([]);
            }

            // Hand pinch distance (Index tip is 8, Thumb tip is 4)
            if (handResults.landmarks && handResults.landmarks.length > 0) {
              const hand = handResults.landmarks[0];
              const thumb = hand[4];
              const index = hand[8];
              if (thumb && index) {
                const dx = thumb.x - index.x;
                const dy = thumb.y - index.y;
                setPinchDistance(Math.sqrt(dx * dx + dy * dy));
              } else {
                setPinchDistance(null);
              }
            } else {
              setPinchDistance(null);
            }

          } catch (e) {
            console.error("Detection error:", e);
          }
        }
      }
      
      animationRef.current = requestAnimationFrame(detect);
    };

    init();

    return () => {
      isActive = false;
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
      if (landmarkerRef.current) {
        landmarkerRef.current.close();
      }
      if (handLandmarkerRef.current) {
        handLandmarkerRef.current.close();
      }
      if (stream) {
        stream.getTracks().forEach(track => track.stop());
      }
    };
  }, [overlayMode]);

  return { faces, pinchDistance, isLoading, error };
};


```

## Source file: vite.config.js

{
  "path": "vite.config.js",
  "blob_sha": "7ef7356a927d46facb561de701f3b36c2e4cdb3d",
  "sha256": "03d8f2a23536cd7c789a60d85bc4e4816e68412c821559cfa474bb0683a3ac18",
  "lines": 8,
  "bytes": 184,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/tajindervirk/FacePulse/blob/d3dfb9acdba670ffab74705474f557c9c219eed3/vite.config.js",
  "function_signature_candidates": []
}

```text

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  base: '/FacePulse/',
  plugins: [react()],
})


```

## Source file: src/utils/expressionLogic.js

{
  "path": "src/utils/expressionLogic.js",
  "blob_sha": "b9f4460032c4f2feb77c5ddc0b00814ffe73be93",
  "sha256": "72f027c5b3c8048b9046b366eadcf1ce4568dff6e0735ce264998b81ad07b1dd",
  "lines": 216,
  "bytes": 9312,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 22,
    "blendshape": 44,
    "jaw": 6,
    "mouth": 33,
    "rig": 27,
    "lip": 4
  },
  "url": "https://github.com/tajindervirk/FacePulse/blob/d3dfb9acdba670ffab74705474f557c9c219eed3/src/utils/expressionLogic.js",
  "function_signature_candidates": []
}

```text

export const getScore = (blendshapes, name) => {
  if (!blendshapes || blendshapes.length === 0) return 0;
  const score = blendshapes.find((b) => b.categoryName === name)?.score || 0;
  return Math.max(0, score);
};

/**
 * Weighted scoring system for expression classification.
 * Instead of a fragile if/else chain, we compute a weighted confidence
 * score for EVERY emotion simultaneously, then pick the winner.
 * This prevents one emotion from "stealing" detection from another.
 */
export const classifyExpression = (blendshapes) => {
  if (!blendshapes || blendshapes.length === 0) {
    return { expression: 'Neutral', emoji: '😐', confidence: 0, colorAccent: 'hsl(var(--muted-foreground))' };
  }

  // Helper clamp function
  const clamp = (val) => Math.max(0, Math.min(1, val));

  // Extract all relevant blendshapes
  const s = {
    smileL:      getScore(blendshapes, 'mouthSmileLeft'),
    smileR:      getScore(blendshapes, 'mouthSmileRight'),
    frownL:      getScore(blendshapes, 'mouthFrownLeft'),
    frownR:      getScore(blendshapes, 'mouthFrownRight'),
    jawOpen:     getScore(blendshapes, 'jawOpen'),
    browInnerUp: getScore(blendshapes, 'browInnerUp'),
    browDownL:   getScore(blendshapes, 'browDownLeft'),
    browDownR:   getScore(blendshapes, 'browDownRight'),
    browOuterL:  getScore(blendshapes, 'browOuterUpLeft'),
    browOuterR:  getScore(blendshapes, 'browOuterUpRight'),
    sneerL:      getScore(blendshapes, 'noseSneerLeft'),
    sneerR:      getScore(blendshapes, 'noseSneerRight'),
    eyeWideL:    getScore(blendshapes, 'eyeWideLeft'),
    eyeWideR:    getScore(blendshapes, 'eyeWideRight'),
    blinkL:      getScore(blendshapes, 'eyeBlinkLeft'),
    blinkR:      getScore(blendshapes, 'eyeBlinkRight'),
    squintL:     getScore(blendshapes, 'eyeSquintLeft'),
    squintR:     getScore(blendshapes, 'eyeSquintRight'),
    cheekSquintL: getScore(blendshapes, 'cheekSquintLeft'),
    cheekSquintR: getScore(blendshapes, 'cheekSquintRight'),
    mouthPucker: getScore(blendshapes, 'mouthPucker'),
    mouthStretchL: getScore(blendshapes, 'mouthStretchLeft'),
    mouthStretchR: getScore(blendshapes, 'mouthStretchRight'),
    mouthPressL:   getScore(blendshapes, 'mouthPressLeft'),
    mouthPressR:   getScore(blendshapes, 'mouthPressRight'),
    upperLipRaiseL: getScore(blendshapes, 'mouthUpperUpLeft'),
    upperLipRaiseR: getScore(blendshapes, 'mouthUpperUpRight'),
    mouthLowerDownL: getScore(blendshapes, 'mouthLowerDownLeft'),
    mouthLowerDownR: getScore(blendshapes, 'mouthLowerDownRight'),
    cheekPuff:       getScore(blendshapes, 'cheekPuff'),
    mouthDimpleL:    getScore(blendshapes, 'mouthDimpleLeft'),
    mouthDimpleR:    getScore(blendshapes, 'mouthDimpleRight'),
    eyeLookDownL:    getScore(blendshapes, 'eyeLookDownLeft'),
    eyeLookDownR:    getScore(blendshapes, 'eyeLookDownRight'),
    mouthFunnel:     getScore(blendshapes, 'mouthFunnel'),
    mouthShrugLower: getScore(blendshapes, 'mouthShrugLower'),
    mouthShrugUpper: getScore(blendshapes, 'mouthShrugUpper'),
  };

  const avgSmile = (s.smileL + s.smileR) / 2;
  const avgCheek = (s.cheekSquintL + s.cheekSquintR) / 2;
  const smileDiff = Math.abs(s.smileL - s.smileR);
  const frownSuppression = clamp(1 - (s.frownL + s.frownR) / 2);

  // 1. Happy
  const Happy = clamp(avgSmile * 0.7 + avgCheek * 0.3) * clamp(1 - smileDiff) * frownSuppression;

  // 2. Sad
  const avgFrown = (s.frownL + s.frownR) / 2;
  const avgBrowDown = (s.browDownL + s.browDownR) / 2;
  const browSad = Math.min(s.browInnerUp, avgBrowDown);
  const smileSuppression = clamp(1 - avgSmile);
  const Sad = clamp(avgFrown * 0.55 + browSad * 0.3 + s.mouthShrugLower * 0.15) * smileSuppression;

  // 3. Surprised (removed eyeStretch/mouthStretch suppression to prevent surprise vs fear conflict failure)
  const avgBrowUp = (s.browInnerUp + (s.browOuterL + s.browOuterR) / 2) / 2;
  const avgEyeWide = (s.eyeWideL + s.eyeWideR) / 2;
  const avgBlink = (s.blinkL + s.blinkR) / 2;
  const avgSquint = (s.squintL + s.squintR) / 2;
  const Surprised = clamp((avgBrowUp + avgEyeWide + s.jawOpen) / 3) * clamp(1 - avgBlink) * clamp(1 - avgSquint);

  // 4. Angry
  const avgSquintAngry = (s.squintL + s.squintR) / 2;
  const avgPress = (s.mouthPressL + s.mouthPressR) / 2;
  const avgSneer = (s.sneerL + s.sneerR) / 2;
  const Angry = clamp(avgBrowDown * 0.4 + avgSquintAngry * 0.2 + avgPress * 0.2 + avgSneer * 0.2) * clamp(1 - s.browInnerUp) * clamp(1 - avgSmile);

  // 5. Disgusted
  const avgUpperLip = (s.upperLipRaiseL + s.upperLipRaiseR) / 2;
  const Disgusted = clamp(Math.max(avgSneer, avgUpperLip) * (0.6 * avgSneer + 0.4 * avgUpperLip)) * clamp(1 - avgSmile);

  // 6. Fearful
  const avgBrowOuter = (s.browOuterL + s.browOuterR) / 2;
  const avgStretch = (s.mouthStretchL + s.mouthStretchR) / 2;
  const Fearful = clamp(avgEyeWide * 0.4 + s.browInnerUp * 0.2 + avgBrowOuter * 0.2 + avgStretch * 0.2 + avgBrowDown * 0.2) * clamp(1 - avgBlink) * clamp(1 - avgSmile);

  // 7. Winking
  const maxBlink = Math.max(s.blinkL, s.blinkR);
  const minBlink = Math.min(s.blinkL, s.blinkR);
  const blinkDiff = Math.abs(s.blinkL - s.blinkR);
  const Winking = (blinkDiff > 0.3 && maxBlink > 0.5 && minBlink < 0.3) ? clamp(maxBlink * (1 - minBlink)) : 0;

  // 8. Puffed
  const avgCheekSquint = (s.cheekSquintL + s.cheekSquintR) / 2;
  const Puffed = clamp(s.cheekPuff * 0.8) * clamp(1 - avgCheekSquint) * clamp(1 - avgSmile);

  // 9. Confusion (removed average mouth frown suppression as it is a supporting signal for Confusion)
  const browOuterDiff = Math.abs(s.browOuterL - s.browOuterR);
  const browDownDiff = Math.abs(s.browDownL - s.browDownR);
  const browAsym = Math.max(browOuterDiff, browDownDiff);
  const Confusion = clamp(browAsym * 0.5 + s.browInnerUp * 0.3 + avgFrown * 0.2) * clamp(1 - avgSmile) * clamp(1 - s.jawOpen);

  // 10. Smirk
  const smileMin = Math.min(s.smileL, s.smileR);
  const smileMax = Math.max(s.smileL, s.smileR);
  const dimpleMin = Math.min(s.mouthDimpleL, s.mouthDimpleR);
  const dimpleMax = Math.max(s.mouthDimpleL, s.mouthDimpleR);
  const dimpleDiff = Math.abs(s.mouthDimpleL - s.mouthDimpleR);
  const asymSmile = smileDiff * clamp(1 - smileMin);
  const asymDimple = dimpleDiff * clamp(1 - dimpleMin);
  const Smirk = clamp(Math.max(asymSmile, asymDimple)) * clamp(Math.max(smileMax, dimpleMax));

  // 11. Yawning
  const eyeClosure = Math.max(avgBlink, avgSquint);
  const Yawning = clamp(s.jawOpen * eyeClosure) * clamp(1 - avgSmile);

  // 12. Boredom (excluded Sadness from suppression list, and added conditional blink asymmetry suppression)
  const eyeDroop = 4 * avgBlink * (1 - avgBlink);
  const avgLookDown = (s.eyeLookDownL + s.eyeLookDownR) / 2;
  const boredomRaw = clamp(eyeDroop * 0.46 + avgLookDown * 0.3 + avgFrown * 0.2);
  const activeExpressionsMax = Math.max(Happy, Surprised, Angry, Disgusted, Fearful, Puffed, Yawning);
  const Boredom = boredomRaw * clamp(1 - s.jawOpen) * clamp(1 - avgSmile) * clamp(1 - activeExpressionsMax) * (blinkDiff > 0.2 ? clamp(1 - 5 * blinkDiff) : 1);

  // Classification Selection
  const activeEmotions = {
    Happy,
    Sad,
    Surprised,
    Angry,
    Disgusted,
    Fearful,
    Winking,
    Puffed,
    Confusion,
    Smirk,
    Boredom,
    Yawning
  };

  let bestEmotion = 'Neutral';
  let bestScore = 0;

  for (const [emotion, score] of Object.entries(activeEmotions)) {
    if (score > bestScore) {
      bestScore = score;
      bestEmotion = emotion;
    }
  }

  // Minimum threshold guard (reverted to 0.12 to align with test suite expectations)
  const MIN_THRESHOLD = 0.12;
  if (bestScore < MIN_THRESHOLD) {
    bestEmotion = 'Neutral';
    bestScore = clamp(1 - Math.max(...Object.values(activeEmotions)));
  }

  const emojiMap = {
    Happy: '😄',
    Sad: '😢',
    Surprised: '😲',
    Angry: '😠',
    Disgusted: '🤢',
    Fearful: '😨',
    Winking: '😉',
    Puffed: '🐡',
    Confusion: '😕',
    Smirk: '😏',
    Boredom: '😑',
    Yawning: '🥱',
    Neutral: '😐'
  };

  const confPercent = Math.min(Math.max(bestScore * 100, 0), 100);

  return {
    expression: bestEmotion,
    emoji: emojiMap[bestEmotion] || '😐',
    confidence: confPercent,
    colorAccent: 'hsl(var(--foreground))'
  };
};

export const CATEGORIZED_BLENDSHAPES = {
  Brows: [
    'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft', 'browOuterUpRight'
  ],
  Eyes: [
    'eyeBlinkLeft', 'eyeBlinkRight', 'eyeLookDownLeft', 'eyeLookDownRight',
    'eyeLookInLeft', 'eyeLookInRight', 'eyeLookOutLeft', 'eyeLookOutRight',
    'eyeLookUpLeft', 'eyeLookUpRight', 'eyeSquintLeft', 'eyeSquintRight',
    'eyeWideLeft', 'eyeWideRight'
  ],
  Mouth: [
    'jawForward', 'jawLeft', 'jawOpen', 'jawRight', 'mouthClose', 'mouthDimpleLeft',
    'mouthDimpleRight', 'mouthFrownLeft', 'mouthFrownRight', 'mouthFunnel',
    'mouthLeft', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthPressLeft',
    'mouthPressRight', 'mouthPucker', 'mouthRight', 'mouthRollLower', 'mouthRollUpper',
    'mouthShrugLower', 'mouthShrugUpper', 'mouthSmileLeft', 'mouthSmileRight',
    'mouthStretchLeft', 'mouthStretchRight', 'mouthUpperUpLeft', 'mouthUpperUpRight'
  ],
  Cheeks: [
    'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft', 'noseSneerRight'
  ]
};


```
