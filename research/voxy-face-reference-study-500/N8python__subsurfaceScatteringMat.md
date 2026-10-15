# Facial rendering source scan: N8python/subsurfaceScatteringMat

Repository: https://github.com/N8python/subsurfaceScatteringMat

Pinned source revision: 47c7bbc6a0e29cadff85b951a2c52e1e315fb32f

Discovery topic: subsurface scattering

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["EffectShader.js"], "rig": ["skybox/Box_Right.bmp"]}

## Source file: EffectShader.js

{
  "path": "EffectShader.js",
  "blob_sha": "13b7b161b38a2ee3771d301bc850de30acd6547b",
  "sha256": "74e042a964cd4d7d9579401315604e5b66da51f9b645e90801ef2e729f3fb6c8",
  "lines": 26,
  "bytes": 568,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 4
  },
  "url": "https://github.com/N8python/subsurfaceScatteringMat/blob/47c7bbc6a0e29cadff85b951a2c52e1e315fb32f/EffectShader.js",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "void main() {"
    },
    {
      "line": 19,
      "text": "void main() {"
    }
  ]
}

```text

import * as THREE from 'https://cdn.skypack.dev/three@0.152.0';
const EffectShader = {

    uniforms: {

        'sceneDiffuse': { value: null }
    },

    vertexShader: /* glsl */ `
		varying vec2 vUv;
		void main() {
			vUv = uv;
			gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
		}`,

    fragmentShader: /* glsl */ `
		uniform sampler2D sceneDiffuse;
        varying vec2 vUv;
		void main() {
            vec4 diffuse = texture2D(sceneDiffuse, vUv);
            gl_FragColor = vec4(diffuse.rgb, 1.0);
		}`

};

export { EffectShader };

```

## Source file: main.js

{
  "path": "main.js",
  "blob_sha": "fc37bccd70fa8ea1106b57ff1d92d0b33c686823",
  "sha256": "fa22a508efd5ac77f0e74fd0b15b9692eb45273bfb1ac60ab772cd7c00478d35",
  "lines": 179,
  "bytes": 8596,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 3,
    "rig": 2
  },
  "url": "https://github.com/N8python/subsurfaceScatteringMat/blob/47c7bbc6a0e29cadff85b951a2c52e1e315fb32f/main.js",
  "function_signature_candidates": []
}

```text

import * as THREE from 'https://cdn.skypack.dev/three@0.152.0';
import { EffectComposer } from 'https://unpkg.com/three@0.152.0/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'https://unpkg.com/three@0.152.0/examples/jsm/postprocessing/RenderPass.js';
import { ShaderPass } from 'https://unpkg.com/three@0.152.0/examples/jsm/postprocessing/ShaderPass.js';
import { SMAAPass } from 'https://unpkg.com/three@0.152.0/examples/jsm/postprocessing/SMAAPass.js';
import { GammaCorrectionShader } from 'https://unpkg.com/three@0.152.0/examples/jsm/shaders/GammaCorrectionShader.js';
import { EffectShader } from "./EffectShader.js";
import { OrbitControls } from 'https://unpkg.com/three@0.152.0/examples/jsm/controls/OrbitControls.js';
import { OBJLoader } from "https://unpkg.com/three@0.152.0/examples/jsm/loaders/OBJLoader.js";
import { GLTFLoader } from "https://unpkg.com/three@0.152.0/examples/jsm/loaders/GLTFLoader.js";
import * as BufferGeometryUtils from "https://unpkg.com/three@0.152.0/examples/jsm/utils/BufferGeometryUtils.js";
import { FullScreenQuad } from "https://unpkg.com/three@0.152.0/examples/jsm/postprocessing/Pass.js";
import { GUI } from 'https://unpkg.com/three@0.152.0/examples/jsm/libs/lil-gui.module.min.js';
import { AssetManager } from './AssetManager.js';
import { MeshTranslucentMaterial } from './TranslucentMaterial.js';
import { Stats } from "./stats.js";
async function main() {
    // Setup basic renderer, controls, and profiler
    const clientWidth = window.innerWidth;
    const clientHeight = window.innerHeight;
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(75, clientWidth / clientHeight, 0.1, 1000);
    camera.position.set(50, 75, 50);
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(clientWidth, clientHeight);
    window.addEventListener('resize', () => {
        const clientWidth = window.innerWidth;
        const clientHeight = window.innerHeight;
        renderer.setSize(clientWidth, clientHeight);
        camera.aspect = clientWidth / clientHeight;
        camera.updateProjectionMatrix();
    });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.outputEncoding = THREE.sRGBEncoding;
    document.body.appendChild(renderer.domElement);
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.target.set(0, 25, 0);
    const stats = new Stats();
    stats.showPanel(0);
    document.body.appendChild(stats.dom);
    // Setup scene
    // Skybox
    const environment = new THREE.CubeTextureLoader().load([
        "skybox/Box_Right.bmp",
        "skybox/Box_Left.bmp",
        "skybox/Box_Top.bmp",
        "skybox/Box_Bottom.bmp",
        "skybox/Box_Front.bmp",
        "skybox/Box_Back.bmp"
    ]);
    environment.encoding = THREE.sRGBEncoding;
    scene.background = environment;
    const rSize = renderer.getDrawingBufferSize(new THREE.Vector2());
    rSize.x = Math.floor(rSize.x / 2);
    rSize.y = Math.floor(rSize.y / 2);
    // Lighting
    const ambientLight = new THREE.AmbientLight(new THREE.Color(1.0, 1.0, 1.0), 0.25);
    scene.add(ambientLight);
    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.35);
    directionalLight.position.set(152, 200, 50);
    // Shadows
    directionalLight.castShadow = true;
    directionalLight.shadow.mapSize.width = 1024;
    directionalLight.shadow.mapSize.height = 1024;
    directionalLight.shadow.camera.left = -75;
    directionalLight.shadow.camera.right = 75;
    directionalLight.shadow.camera.top = 75;
    directionalLight.shadow.camera.bottom = -75;
    directionalLight.shadow.camera.near = 0.1;
    directionalLight.shadow.camera.far = 500;
    directionalLight.shadow.bias = -0.001;
    directionalLight.shadow.blurSamples = 8;
    directionalLight.shadow.radius = 4;
    scene.add(directionalLight);
    // Add 6 point lights
    const colors = [
        new THREE.Color(1.0, 0.0, 0.0),
        new THREE.Color(0.0, 1.0, 0.0),
        new THREE.Color(0.0, 0.0, 1.0),
        new THREE.Color(1.0, 1.0, 0.0),
        new THREE.Color(1.0, 0.0, 1.0),
        new THREE.Color(0.0, 1.0, 1.0)
    ];
    const lights = [];
    for (let i = 0; i < 6; i++) {
        let color = colors[i % 6];
        const pointLight = new THREE.PointLight(color, 0.25);
        pointLight.position.set(40 * Math.random() - 20, 5 + 10 * Math.random(), 40 * Math.random() - 20);
        scene.add(pointLight);
        pointLight.add(new THREE.Mesh(new THREE.SphereGeometry(1, 32, 32), new THREE.MeshBasicMaterial({ color: color })));
        lights.push(pointLight);
    }
    // Objects
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(100, 100).applyMatrix4(new THREE.Matrix4().makeRotationX(-Math.PI / 2)), new THREE.MeshStandardMaterial({ side: THREE.DoubleSide, roughness: 0, envMap: environment, color: new THREE.Color(0.5, 0.5, 0.5) }));
    ground.castShadow = true;
    ground.receiveShadow = true;
    scene.add(ground);
    let buddha = await new OBJLoader().loadAsync("https://raw.githubusercontent.com/alecjacobson/common-3d-test-models/master/data/happy.obj");
    buddha.traverse((child) => {
        if (child.isMesh) {
            buddha = child;
        }
    });
    buddha.geometry.deleteAttribute('normal');
    buddha.geometry = BufferGeometryUtils.mergeVertices(buddha.geometry);
    buddha.geometry.computeVertexNormals();
    buddha.updateMatrixWorld();
    buddha.geometry.applyMatrix4(buddha.matrixWorld);
    buddha.geometry.center();
    buddha.geometry.scale(100, 100, 100);
    const translucentMesh = new THREE.Mesh(buddha.geometry, new MeshTranslucentMaterial({ side: THREE.DoubleSide, envMap: environment, transmission: 1.0, roughness: 0.5, ior: 1.5, attenuationColor: new THREE.Color(0.9, 0.6, 0.3), attenuationDistance: 0.33, dithering: true, thickness: 2.0 }));
    translucentMesh.position.y = new THREE.Box3().setFromObject(translucentMesh, true).getSize(new THREE.Vector3()).y / 2;
    translucentMesh.castShadow = true;
    //translucentMesh.receiveShadow = true;
    scene.add(translucentMesh);

    const clock = new THREE.Clock();
    const gui = new GUI();
    const effectController = {
        roughness: 0.5,
        internalRoughness: 0.5,
        scatteringAbsorption: 1.0,
        scattering: 1.0,
        roughnessBlurScale: 16.0,
        resolutionScale: 0.5,
        attenuationDistance: 0.33,
        attenuationColor: [0.9, 0.6, 0.3],
        moveLights: false
    }
    gui.add(effectController, "roughness", 0.0, 1.0, 0.01).onChange((value) => {
        translucentMesh.material.roughness = value;
    });
    gui.add(effectController, "internalRoughness", 0.0, 1.0, 0.01).onChange((value) => {
        translucentMesh.material.internalRoughness = value;
    });
    gui.add(effectController, "scatteringAbsorption", 0.0, 1.0, 0.01).onChange((value) => {
        translucentMesh.material.scatteringAbsorption = value;
    });
    gui.add(effectController, "scattering", 0.0, 1.0, 0.01).onChange((value) => {
        translucentMesh.material.scattering = value;
    });
    gui.add(effectController, "roughnessBlurScale", 0.0, 32.0, 0.01).onChange((value) => {
        translucentMesh.material.roughnessBlurScale = value;
    });
    gui.add(effectController, "resolutionScale", 0.25, 1.0, 0.01).onChange((value) => {
        translucentMesh.material.resolutionScale = value;
    });
    gui.add(effectController, "attenuationDistance", 0.0, 1.0, 0.01).onChange((value) => {
        translucentMesh.material.attenuationDistance = value;
    });
    gui.addColor(effectController, "attenuationColor").onChange((value) => {
        translucentMesh.material.attenuationColor = new THREE.Color(value[0], value[1], value[2]);
    });
    gui.add(effectController, "moveLights");




    function animate() {
        const delta = clock.getDelta();
        if (effectController.moveLights) {
            for (let i = 0; i < lights.length; i++) {
                const light = lights[i];
                const mag = Math.max(Math.hypot(light.position.x, light.position.z), 5);
                const angle = Math.atan2(light.position.z, light.position.x) + delta;
                light.position.x = mag * Math.cos(angle);
                light.position.z = mag * Math.sin(angle);
            }
        }
        renderer.render(scene, camera);
        controls.update();
        stats.update();
        requestAnimationFrame(animate);
    }
    requestAnimationFrame(animate);
}
main();

```

## Source file: stats.js

{
  "path": "stats.js",
  "blob_sha": "e01c11a452e2bc962eff11d292130b46ba8720a5",
  "sha256": "a6e3ec380423a2633afda9ccc262b00872061db221acaa1ddbbb509311988c22",
  "lines": 174,
  "bytes": 4020,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/N8python/subsurfaceScatteringMat/blob/47c7bbc6a0e29cadff85b951a2c52e1e315fb32f/stats.js",
  "function_signature_candidates": []
}

```text

var Stats = function() {

    var mode = 0;

    var container = document.createElement('div');
    container.style.cssText = 'position:fixed;top:0;left:0;cursor:pointer;opacity:0.9;z-index:10000';
    container.addEventListener('click', function(event) {

        event.preventDefault();
        showPanel(++mode % container.children.length);

    }, false);

    //

    function addPanel(panel) {

        container.appendChild(panel.dom);
        return panel;

    }

    function showPanel(id) {

        for (var i = 0; i < container.children.length; i++) {

            container.children[i].style.display = i === id ? 'block' : 'none';

        }

        mode = id;

    }

    //

    var beginTime = (performance || Date).now(),
        prevTime = beginTime,
        frames = 0;

    var fpsPanel = addPanel(new Stats.Panel('FPS', '#0ff', '#002'));
    var msPanel = addPanel(new Stats.Panel('MS', '#0f0', '#020'));

    if (self.performance && self.performance.memory) {

        var memPanel = addPanel(new Stats.Panel('MB', '#f08', '#201'));

    }

    showPanel(0);

    return {

        REVISION: 16,

        dom: container,

        addPanel: addPanel,
        showPanel: showPanel,

        begin: function() {

            beginTime = (performance || Date).now();

        },

        end: function() {

            frames++;

            var time = (performance || Date).now();

            msPanel.update(time - beginTime, 200);

            if (time >= prevTime + 1000) {

                fpsPanel.update((frames * 1000) / (time - prevTime), 100);

                prevTime = time;
                frames = 0;

                if (memPanel) {

                    var memory = performance.memory;
                    memPanel.update(memory.usedJSHeapSize / 1048576, memory.jsHeapSizeLimit / 1048576);

                }

            }

            return time;

        },

        update: function() {

            beginTime = this.end();

        },

        // Backwards Compatibility

        domElement: container,
        setMode: showPanel

    };

};

Stats.Panel = function(name, fg, bg) {

    var min = Infinity,
        max = 0,
        round = Math.round;
    var PR = round(window.devicePixelRatio || 1);

    var WIDTH = 80 * PR,
        HEIGHT = 48 * PR,
        TEXT_X = 3 * PR,
        TEXT_Y = 2 * PR,
        GRAPH_X = 3 * PR,
        GRAPH_Y = 15 * PR,
        GRAPH_WIDTH = 74 * PR,
        GRAPH_HEIGHT = 30 * PR;

    var canvas = document.createElement('canvas');
    canvas.width = WIDTH;
    canvas.height = HEIGHT;
    canvas.style.cssText = 'width:80px;height:48px';

    var context = canvas.getContext('2d');
    context.font = 'bold ' + (9 * PR) + 'px Helvetica,Arial,sans-serif';
    context.textBaseline = 'top';

    context.fillStyle = bg;
    context.fillRect(0, 0, WIDTH, HEIGHT);

    context.fillStyle = fg;
    context.fillText(name, TEXT_X, TEXT_Y);
    context.fillRect(GRAPH_X, GRAPH_Y, GRAPH_WIDTH, GRAPH_HEIGHT);

    context.fillStyle = bg;
    context.globalAlpha = 0.9;
    context.fillRect(GRAPH_X, GRAPH_Y, GRAPH_WIDTH, GRAPH_HEIGHT);

    return {

        dom: canvas,

        update: function(value, maxValue) {

            min = Math.min(min, value);
            max = Math.max(max, value);

            context.fillStyle = bg;
            context.globalAlpha = 1;
            context.fillRect(0, 0, WIDTH, GRAPH_Y);
            context.fillStyle = fg;
            context.fillText(round(value) + ' ' + name + ' (' + round(min) + '-' + round(max) + ')', TEXT_X, TEXT_Y);

            context.drawImage(canvas, GRAPH_X + PR, GRAPH_Y, GRAPH_WIDTH - PR, GRAPH_HEIGHT, GRAPH_X, GRAPH_Y, GRAPH_WIDTH - PR, GRAPH_HEIGHT);

            context.fillRect(GRAPH_X + GRAPH_WIDTH - PR, GRAPH_Y, PR, GRAPH_HEIGHT);

            context.fillStyle = bg;
            context.globalAlpha = 0.9;
            context.fillRect(GRAPH_X + GRAPH_WIDTH - PR, GRAPH_Y, PR, round((1 - (value / maxValue)) * GRAPH_HEIGHT));

        }

    };

};
export { Stats };

```

## Source file: AssetManager.js

{
  "path": "AssetManager.js",
  "blob_sha": "c8f13052525ea68e7f4dce9e50efea12725d407b",
  "sha256": "f1b0a4a480a0d40d693738ee9ce3ac585f77a3526851d859a143b2544d9ef07d",
  "lines": 48,
  "bytes": 1547,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/N8python/subsurfaceScatteringMat/blob/47c7bbc6a0e29cadff85b951a2c52e1e315fb32f/AssetManager.js",
  "function_signature_candidates": []
}

```text

import * as THREE from 'https://cdn.skypack.dev/three@0.152.0';
import {
    GLTFLoader
} from 'https://unpkg.com/three@0.152.0/examples/jsm/loaders/GLTFLoader.js';
const AssetManager = {};
AssetManager.gltfLoader = new GLTFLoader();
AssetManager.audioLoader = new THREE.AudioLoader();
AssetManager.loadGLTFAsync = (url) => {
    return new Promise((resolve, reject) => {
        AssetManager.gltfLoader.load(url, obj => {
            resolve(obj);
        })
    });
}

AssetManager.loadAudioAsync = (url) => {
    return new Promise((resolve, reject) => {
        AssetManager.audioLoader.load(url, (buffer) => {
            resolve(buffer);
        });
    })
}

AssetManager.loadTextureAsync = (url) => {
    return new Promise((resolve, reject) => {
        THREE.ImageUtils.loadTexture(url, null, (tex) => {
            resolve(tex);
        })
    })
}
AssetManager.loadAll = (promiseArr, element, message) => {
    let count = promiseArr.length;
    let results = [];
    element.innerHTML = `${message}&nbsp;(${promiseArr.length - count}&nbsp;/&nbsp;${promiseArr.length})...`
    return new Promise((resolve, reject) => {
        promiseArr.forEach((promise, i) => {
            promise.then(result => {
                results[i] = result;
                count--;
                element.innerHTML = `${message}&nbsp;(${promiseArr.length - count}&nbsp;/&nbsp;${promiseArr.length})...`
                if (count === 0) {
                    resolve(results);
                }
            })
        })
    });
}
export { AssetManager };

```
