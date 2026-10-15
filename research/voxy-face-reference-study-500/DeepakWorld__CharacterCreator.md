# Facial rendering source scan: DeepakWorld/CharacterCreator

Repository: https://github.com/DeepakWorld/CharacterCreator

Pinned source revision: 8fb9741dd36310f58124e542a5e571a0eebc0827

Discovery topic: character creator 3D

Description: 3D Avatar Creator for Everyone

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["public/3d/icons-gradient/eyes.svg", "public/3d/icons/eye.png", "public/assets/icons/eyes.svg", "public/ui/traits/Eye 2.png"], "skin": ["public/3d/icons-gradient/skin.svg", "public/3d/icons/skin-color.png", "public/assets/icons/skin.svg", "public/ui/skinSelector/Vector.png", "public/ui/traits/skin-color.png"], "shader": ["src/library/particle/shader.js"], "rig": [".prettierignore", "public/assets/icons/arrowRight.svg", "public/assets/icons/expandRight.svg", "public/assets/icons/slotRightHand.svg"], "lip": ["src/library/lipsync.js"]}

## Source file: src/library/lipsync.js

{
  "path": "src/library/lipsync.js",
  "blob_sha": "45dfaf6c404937a743ea9c0bca6f690e77ead76d",
  "sha256": "ec085265017818133cfb6e92064d84db84d6b6415af8109955f19445ef116fe2",
  "lines": 193,
  "bytes": 6766,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 15
  },
  "url": "https://github.com/DeepakWorld/CharacterCreator/blob/8fb9741dd36310f58124e542a5e571a0eebc0827/src/library/lipsync.js",
  "function_signature_candidates": []
}

```text

import { VRMExpressionPresetName } from "@pixiv/three-vrm";

const BoundingFrequencyMasc = [0, 400, 560, 2400, 4800]
const BoundingFrequencyFem = [0, 500, 700, 3000, 6000]
const IndicesFrequencyFemale= []
const IndicesFrequencyMale = []
const FFT_SIZE = 1024
const samplingFrequency = 44100

for (let m = 0; m < BoundingFrequencyMasc.length; m++) {
    IndicesFrequencyMale[m] = Math.round(((2 * FFT_SIZE) / samplingFrequency) * BoundingFrequencyMasc[m])
  }

  for (let m = 0; m < BoundingFrequencyFem.length; m++) {
    IndicesFrequencyFemale[m] = Math.round(((2 * FFT_SIZE) / samplingFrequency) * BoundingFrequencyFem[m])
  }

export class LipSync {
  constructor(vrm) {
    this.vrm = vrm

    const update = (deltaTime, elapsedTime) => {
      requestAnimationFrame(update)
      this.update(deltaTime, elapsedTime)
    }

    update()

  }

  start(stream) {
    this.audioContext = new AudioContext()
    this.mediaStreamSource = this.audioContext.createMediaStreamSource(stream)
    this.meter = LipSync.createAudioMeter(this.audioContext)
    this.mediaStreamSource.connect(this.meter)
  }

  startFromAudioFile(file) {
    if(!this.audioContext) this.audioContext = new AudioContext()


    if(!this.userSpeechAnalyzer)
        this.userSpeechAnalyzer = this.audioContext.createAnalyser()
    this.userSpeechAnalyzer.smoothingTimeConstant = 0.5
    this.userSpeechAnalyzer.fftSize = FFT_SIZE

    if (this.mediaStreamSource)
      this.mediaStreamSource.stop()

    this.audioContext.decodeAudioData(file).then((buffer) => {
        this.mediaStreamSource = this.audioContext.createBufferSource()
        this.mediaStreamSource.buffer = buffer
        this.meter = LipSync.createAudioMeter(this.audioContext)
        this.mediaStreamSource.connect(this.meter)
        this.mediaStreamSource.connect(this.audioContext.destination)
        this.mediaStreamSource.start()

        // connect the output of mediaStreamSource to the input of userSpeechAnalyzer
        this.mediaStreamSource.connect(this.userSpeechAnalyzer)
    })

    }




  destroy() {
    this.meter?.shutdown()
    this.meter = null
    this.mediaStreamSource?.disconnect()
    return this.audioContext?.close().catch(() => {}) || Promise.resolve()
  }

  update(deltaTime) {
    if (this.meter) {
        
      const { volume } = this.meter
      if (volume < 0.01) {
    
        // get the shape keys from the VRM avatar
        this.vrm.expressionManager.setValue(VRMExpressionPresetName.Oh, 0)
        this.vrm.expressionManager.setValue(VRMExpressionPresetName.Ah, 0)
        this.vrm.expressionManager.setValue(VRMExpressionPresetName.Ee, 0)
      } else {

        const {ah, oh, ee} = this.update2();

        this.vrm.expressionManager.setValue(VRMExpressionPresetName.Oh, oh)
        this.vrm.expressionManager.setValue(VRMExpressionPresetName.Ah, ah)
        this.vrm.expressionManager.setValue(VRMExpressionPresetName.Ee, ee)
        
        // update the shape keys
        this.vrm.expressionManager.update(deltaTime)
      }
    }
  }

  update2() {      
      function getSensitivityMap(spectrum) {
        const sensitivity_threshold = 0.5
        const stPSD = new Float32Array(spectrum.length)
        for (let i = 0; i < spectrum.length; i++) {
          stPSD[i] = sensitivity_threshold + (spectrum[i] + 20) / 140
        }
        return stPSD
      }

    const spectrum = new Float32Array(this.userSpeechAnalyzer.frequencyBinCount)
    // Populate frequency data for computing frequency intensities
    this.userSpeechAnalyzer.getFloatFrequencyData(spectrum) // getByteTimeDomainData gets volumes over the sample time
    // Populate time domain for calculating RMS
    // userSpeechAnalyzer.getFloatTimeDomainData(spectrum);
    // RMS (root mean square) is a better approximation of current input level than peak (just sampling this frame)
    // spectrumRMS = getRMS(spectrum);

    const sensitivityPerPole = getSensitivityMap(spectrum)

    // Lower and higher voices have different frequency domains, so we'll separate and max them
    const EnergyBinMasc = new Float32Array(BoundingFrequencyMasc.length)
    const EnergyBinFem = new Float32Array(BoundingFrequencyFem.length)

    // Masc energy bins (groups of frequency-depending energy)
    for (let m = 0; m < BoundingFrequencyMasc.length - 1; m++) {
      for (let j = IndicesFrequencyMale[m]; j <= IndicesFrequencyMale[m + 1]; j++)
        if (sensitivityPerPole[j] > 0) EnergyBinMasc[m] += sensitivityPerPole[j]
      EnergyBinMasc[m] /= IndicesFrequencyMale[m + 1] - IndicesFrequencyMale[m]
    }

    // Fem energy bin
    for (let m = 0; m < BoundingFrequencyFem.length - 1; m++) {
      for (let j = IndicesFrequencyMale[m]; j <= IndicesFrequencyMale[m + 1]; j++)
        if (sensitivityPerPole[j] > 0) EnergyBinFem[m] += sensitivityPerPole[j]
      EnergyBinMasc[m] /= IndicesFrequencyMale[m + 1] - IndicesFrequencyMale[m]
      EnergyBinFem[m] = EnergyBinFem[m] / (IndicesFrequencyFemale[m + 1] - IndicesFrequencyFemale[m])
    }
    const oh =
      Math.max(EnergyBinFem[1], EnergyBinMasc[1]) > 0.2
        ? 1 - 2 * Math.max(EnergyBinMasc[2], EnergyBinFem[2])
        : (1 - 2 * Math.max(EnergyBinMasc[2], EnergyBinFem[2])) * 5 * Math.max(EnergyBinMasc[1], EnergyBinFem[1])

    const ah = 3 * Math.max(EnergyBinMasc[3], EnergyBinFem[3])
    const ee = 0.8 * (Math.max(EnergyBinMasc[1], EnergyBinFem[1]) - Math.max(EnergyBinMasc[3], EnergyBinFem[3]))
    return { oh, ee, ah }

  }

  static createAudioMeter(audioContext) {
    const processor = audioContext.createScriptProcessor(512)
    processor.onaudioprocess = (event) => {
      const buf = event.inputBuffer.getChannelData(0)
      const bufLength = buf.length
      let sum = 0
      let x

      // eslint-disable-next-line no-plusplus
      for (let i = 0; i < bufLength; i++) {
        x = buf[i]
        if (Math.abs(x) >= processor.clipLevel) {
          processor.clipping = true
          processor.lastClip = window.performance.now()
        }
        sum += x * x
      }
      const rms = Math.sqrt(sum / bufLength)
      processor.volume = Math.max(rms, processor.volume * processor.averaging)
    }
    processor.clipping = false
    processor.lastClip = 0
    processor.volume = 0
    processor.clipLevel = 0.98
    processor.averaging = 0.95
    processor.clipLag = 750

    processor.connect(audioContext.destination)

    processor.checkClipping = () => {
      if (!processor.clipping) {
        return false
      }
      if (processor.lastClip + processor.clipLag < window.performance.now()) {
        processor.clipping = false
      }
      return processor.clipping
    }

    processor.shutdown = () => {
      processor.disconnect()
      processor.onaudioprocess = null
    }

    return processor
  }
}

```

## Source file: src/library/particle/shader.js

{
  "path": "src/library/particle/shader.js",
  "blob_sha": "5e4a077256486797bebe1f27f9cf24d8ee1355e3",
  "sha256": "00029c8839884e926e15cfc2c21d100fa09307671ad1c99e94d8bbf21d89118e",
  "lines": 245,
  "bytes": 6515,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 9,
    "rig": 2,
    "face": 12
  },
  "url": "https://github.com/DeepakWorld/CharacterCreator/blob/8fb9741dd36310f58124e542a5e571a0eebc0827/src/library/particle/shader.js",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "void main() {"
    },
    {
      "line": 29,
      "text": "void main() {"
    },
    {
      "line": 69,
      "text": "vec3 rotateVecQuat(vec3 position, vec4 q) {"
    },
    {
      "line": 74,
      "text": "void main() {"
    },
    {
      "line": 95,
      "text": "void main() {"
    },
    {
      "line": 126,
      "text": "void main() {"
    },
    {
      "line": 150,
      "text": "void main() {"
    },
    {
      "line": 173,
      "text": "vec3 rotateVecQuat(vec3 position, vec4 q) {"
    },
    {
      "line": 177,
      "text": "void main() {"
    },
    {
      "line": 193,
      "text": "void main() {"
    },
    {
      "line": 207,
      "text": "void main() {"
    },
    {
      "line": 224,
      "text": "void main() {"
    }
  ]
}

```text

const beamVertex = `\       
  uniform vec3 cameraDir;
  uniform vec3 eye;
  
  varying vec3 vWorldPosition;
  varying vec3 vSurfaceNormal;

  varying vec2 vUv;

  void main() {
    vUv = uv;
    vec3 pos = position;
    vSurfaceNormal = normalize(normal);
    vWorldPosition = (modelMatrix * vec4( pos, 1.0 )).xyz;
    gl_Position = projectionMatrix * modelViewMatrix * vec4( pos, 1.0 ); 
  }
`
const beamFragment = `\ 
  uniform float switchItemDuration;
  uniform float switchItemTime;
  uniform vec3 cameraDir;
  uniform vec3 eye;

  uniform sampler2D auraTexture;

  varying vec3 vSurfaceNormal;
  varying vec3 vWorldPosition;
  varying vec2 vUv;
  void main() {
    float aura = texture2D(
      auraTexture, 
      vec2(
        vUv.x * 2.0,
        vUv.y * 1.0 * (switchItemDuration - switchItemTime)
      )
    ).r;
    
    float dissolveThreshold = 0.1;
    float auraStep = switchItemTime <= dissolveThreshold ? 0. : clamp(switchItemTime, dissolveThreshold, 1.0);
    aura = smoothstep(0.0, auraStep, aura);
    

    vec3 eyeDirection = normalize(eye - vWorldPosition);
    
    float EdotN = max(0.0, dot(eyeDirection, vSurfaceNormal));
    float rimStrength = 0.7;
    float rim = mix(0.0, 1.0, pow(1. - EdotN, rimStrength));
    float glowIntensity = 2.0;

    vec3 rimColor = mix(vec3(0.00960, 0.833, 0.960), vec3(0., 0., 0.960), rim);
    gl_FragColor.rgb = rimColor * (1. - rim) * glowIntensity;
    gl_FragColor.a = clamp((1. - rim) * glowIntensity, 0.0, 1.0);
    gl_FragColor.a *= (switchItemDuration - switchItemTime);
    gl_FragColor *= aura;
  }
`

const pixelVertex = `\       
  uniform vec4 cameraBillboardQuaternion;

  attribute vec2 scales;
  attribute float opacity;
  attribute vec3 positions;

  varying vec2 vUv;
  varying float vOpacity;
  varying vec3 vWorldPosition;

  vec3 rotateVecQuat(vec3 position, vec4 q) {
      vec3 v = position.xyz;
      return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
  }
  
  void main() {  
    vUv = uv;
    vOpacity = opacity;
    
    vec3 pos = position;
    pos = rotateVecQuat(pos, cameraBillboardQuaternion);
    pos.xz *= scales.x;
    pos.y *= scales.y;
    pos += positions;
    vec4 modelPosition = modelMatrix * vec4(pos, 1.0);
    vWorldPosition = modelPosition.xyz;
    vec4 viewPosition = viewMatrix * modelPosition;
    vec4 projectionPosition = projectionMatrix * viewPosition;
    gl_Position = projectionPosition;
  }
`
const pixelFragment = `\ 
  varying vec2 vUv;
  varying float vOpacity;
  varying vec3 vWorldPosition;

  void main() {
    float angle = 0.;
    vec2 trig = vec2(cos(angle), sin(angle));
    
    vec2 pos = (vUv - 0.5) * mat2(trig.x, trig.y, -trig.y, trig.x);
    float size = 0.1;
    
    float dist = length(max(abs(pos) - size, 0.));
    float glow = 1. / (dist * 25. + .5);

    gl_FragColor = vec4(glow);
    gl_FragColor.rgb *= vec3(0.00960, 0.833, 0.960);
    float avatarGroundPosition = 0.;
    if (glow < 0.1 || vWorldPosition.y < avatarGroundPosition) {
      discard;
    }
    gl_FragColor.a *= vOpacity;
  }
`

const ringVertex = `\       
  attribute vec2 scales;
  attribute float opacity;
  attribute vec3 positions;

  varying vec2 vUv;
  varying float vOpacity;
  varying vec3 vWorldPosition;
  varying vec3 vSurfaceNormal;
  

  void main() {  
    vUv = uv;
    vOpacity = opacity;
    vSurfaceNormal = normalize(normal);
    
    vec3 pos = position;
    pos.xz *= scales.x;
    pos.y *= scales.y;
    pos += positions;
    vec4 modelPosition = modelMatrix * vec4(pos, 1.0);
    vWorldPosition = modelPosition.xyz;
    vec4 viewPosition = viewMatrix * modelPosition;
    vec4 projectionPosition = projectionMatrix * viewPosition;
    gl_Position = projectionPosition;
  }
`
const ringFragment = `\ 
  uniform float radius;

  varying vec2 vUv;
  varying float vOpacity;
  varying vec3 vWorldPosition;
  varying vec3 vSurfaceNormal;

  void main() {

    vec3 eyeDirection = normalize(vec3(0., 10., 0.) - vWorldPosition);
      
    float EdotN = max(0.0, dot(eyeDirection, vSurfaceNormal));
    float rimStrength = 10.0;
    float rim = distance(vWorldPosition.xz, vec2(0.)) / radius;
    rim = pow(rim, rimStrength);
    
  
    gl_FragColor.rgb = vec3(rim) * vec3(0.00960, 0.833, 0.960) * 2.0;
    gl_FragColor.a = rim;
    gl_FragColor.a = smoothstep(0.3, 0.8, gl_FragColor.a);
    gl_FragColor.a *= vOpacity;
  }
`

const teleportVertex = `\       
  uniform vec4 cameraBillboardQuaternion;

  varying vec2 vUv;
  varying vec3 vWorldPosition;
  
  vec3 rotateVecQuat(vec3 position, vec4 q) {
      vec3 v = position.xyz;
      return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
  }
  void main() {  
    vUv = uv;
    
    vec3 pos = position;
    pos = rotateVecQuat(pos, cameraBillboardQuaternion);
    vec4 modelPosition = modelMatrix * vec4(pos, 1.0);
    vWorldPosition = modelPosition.xyz;
    vec4 viewPosition = viewMatrix * modelPosition;
    vec4 projectionPosition = projectionMatrix * viewPosition;
    gl_Position = projectionPosition;
  }
`
const teleportFragment = `\ 
  varying vec2 vUv;
  varying vec3 vWorldPosition;

  void main() {
    float col = smoothstep(0.5, 0.2, length(vUv - 0.5));
    gl_FragColor.rgb = mix(vec3(0., 0., 0.960), vec3(0.00960, 0.833, 0.960), col);
    gl_FragColor.a = col;
    gl_FragColor.a *= clamp(vWorldPosition.y * 10., 0.0, 1.0);
  }
`

const spotLightVertex = `\       
  varying vec3 vWorldPosition;
  varying vec3 vSurfaceNormal;

  varying vec2 vUv;

  void main() {
    vUv = uv;
    vec3 pos = position;
    vSurfaceNormal = normalize(normal);
    vWorldPosition = (modelMatrix * vec4( pos, 1.0 )).xyz;
    gl_Position = projectionMatrix * modelViewMatrix * vec4( pos, 1.0 ); 
  }
`
const spotLightFragment = `\ 
  uniform vec3 cameraDir;
  uniform vec3 eye;

  uniform float opacity;

  varying vec3 vSurfaceNormal;
  varying vec3 vWorldPosition;
  varying vec2 vUv;
  void main() {
    vec3 eyeDirection = normalize(eye - vWorldPosition);
    
    float EdotN = max(0.0, dot(eyeDirection, vSurfaceNormal));
    float rimStrength = 0.7;
    float rim = mix(0.0, 1.0, pow(1. - EdotN, rimStrength));
    float glowIntensity = 2.0;

    vec3 rimColor = mix(vec3(1.0), vec3(0.400, 0.723, 0.910), rim);
    gl_FragColor = vec4(rimColor, pow(1. - rim, 2.0));
    gl_FragColor *= pow(1. - vUv.y, 3.0);
    gl_FragColor *= opacity;
  }
`

export {
  beamVertex, beamFragment,
  pixelVertex, pixelFragment,
  ringVertex, ringFragment,
  teleportVertex, teleportFragment,
  spotLightVertex, spotLightFragment,
};

```

## Source file: vite.config.js

{
  "path": "vite.config.js",
  "blob_sha": "861b04b35601de92787a1a0db6c9fa190975d220",
  "sha256": "7a82de43ce44daf753154d196e7e27db10999867a5e8b00b607589896d76b5b0",
  "lines": 7,
  "bytes": 167,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DeepakWorld/CharacterCreator/blob/8fb9741dd36310f58124e542a5e571a0eebc0827/vite.config.js",
  "function_signature_candidates": []
}

```text

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
})


```

## Source file: src/lib/chat.js

{
  "path": "src/lib/chat.js",
  "blob_sha": "ae5be71b75ae57c076424328a4736794d67d4287",
  "sha256": "702c75ea1a1102f3b781d6b496df0267d878ceacf805024bda92dc31a18bff5a",
  "lines": 25,
  "bytes": 526,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/DeepakWorld/CharacterCreator/blob/8fb9741dd36310f58124e542a5e571a0eebc0827/src/lib/chat.js",
  "function_signature_candidates": []
}

```text

// Constants

const messagesMaxCharacters = 20000

// Prune Messages Function

export async function pruneMessages(messages) {
  let currentSize = 0
  const newMessages = []

  for (let i = messages.length - 1; i >= 0; i--) {
    const message = messages[i]

    currentSize += message.length

    // Add up to N characters.
    if (currentSize < messagesMaxCharacters) newMessages.push(message)
    else break
  }

  // Reverse the array so that the newest messages are first.
  newMessages.reverse()

  return newMessages
}


```
