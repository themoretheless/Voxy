# Facial rendering source scan: williamclot/character_creator

Repository: https://github.com/williamclot/character_creator

Pinned source revision: 7e9a76b8e6c1497577e9d369bc15a1ab39a4da2d

Discovery topic: character creator 3D

Description: Customizer tool in javascript to create your own unique 3D Character!

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: src/App.js

{
  "path": "src/App.js",
  "blob_sha": "d51383099a1b70084f8544726baca1cbebeee7b0",
  "sha256": "602cf13452e0595c0054adc977aac91defe584c4dbe83ce41cde1b4cd023b2ea",
  "lines": 166,
  "bytes": 4905,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/williamclot/character_creator/blob/7e9a76b8e6c1497577e9d369bc15a1ab39a4da2d/src/App.js",
  "function_signature_candidates": []
}

```text

import React, { Component } from "react";
import { BrowserView, MobileView } from "react-device-detect";
import Typed from "react-typed";

import { library } from "@fortawesome/fontawesome-svg-core";

// fontawesome imports
import { faTimesCircle } from "@fortawesome/free-solid-svg-icons";
import { faDollarSign } from "@fortawesome/free-solid-svg-icons";
import { faLink } from "@fortawesome/free-solid-svg-icons";
import { faSearch } from "@fortawesome/free-solid-svg-icons";
// Google Analytics
import ReactGA from "react-ga";
// import axios from "axios";

// Loading assets
import "./css/master.css";
import logo from "./graphic_assets/mmf_logo.png";

// Loading the data this way for now
import data from "./library/category.json";

// Loading the different components
import Name from "./components/Name";
import Footer from "./components/Footer";
import Category from "./components/Category";
import Buttons from "./components/Buttons";
import Popup from "./components/Popup";
import Logo from "./components/Logo";
import PageLoader from "./components/PageLoader";
import PartLoader from "./components/PartLoader";


library.add(faTimesCircle);
library.add(faDollarSign);
library.add(faLink);
library.add(faSearch);

class App extends Component {
  constructor(props) {
    super(props);
    this.state = {
      category: data,
      currentCategory: "head",
      characterName: "myCharacter",
      UIDisplayed: true,
      popup: false,
      loadedMeshes: {
        Torso: "turtle_torso",
        LegR: "default_leg_R",
        LegL: "default_leg_L",
        Head: "default_head",
        ArmR: "default_arm_R",
        ArmL: "default_arm_L",
        HandR: "open_hand_R",
        HandL: "open_hand_L",
        FootR: "default_foot_R",
        FootL: "default_foot_L",
        Stand: "circle"
      },
      editor: true,
      partloading: false,
      message:"Sorry this feature is still in development..."
    };
  }

  // Update the state of parent App from child Component
  updateCategory = currentCategory => {
    this.setState({ currentCategory });
  };
  updateCharacterName = characterName => {
    this.setState({ characterName });
  };
  updatePopup = popup => {
    this.setState({ popup });
  };
  updateMeshes = loadedMeshes => {
    this.setState({ loadedMeshes });
  };
  updateLoading = partloading => {
    this.setState({ partloading });
  };
  updatePopupMessage = message => {
    this.setState({ message });
  };

  componentDidMount() {
    // Google Analytics for the page
    ReactGA.initialize("UA-41837285-1");
    ReactGA.pageview("/mmf-hero");
    // axios({
    //   method: "get",
    //   url: "https://www.myminifactory.com/api/v2/search",
    //   headers: {
    //     "Content-Type": "application/json",
    //     Authorization: "Bearer b43ab57d-15f3-4c76-928a-ffa7a5662f1b"
    //   },
    //   params:{
    //     q: "CharacterCreator"
    //   }
    // }).then(response => {
    //   console.log(response)
    // });
  }

  render() {
    return (
      <div>
        <BrowserView>
          <PageLoader />
          <Logo />
          <Name
            characterName={this.state.characterName}
            updateCharacterName={this.updateCharacterName}
          />
          <Footer />
          <Buttons
            characterName={this.state.characterName}
            updatePopup={this.updatePopup}
            updatePopupMessage={this.updatePopupMessage}
            loadedMeshes={this.state.loadedMeshes}
          />
          <Popup
            popupDisplayed={this.state.popup}
            message={this.state.message}
            updatePopup={this.updatePopup}
          />
          <Category
            category={this.state.category}
            currentCategory={this.state.currentCategory}
            updateCategory={this.updateCategory}
            updatePose={this.updatePose}
            UIDisplayed={this.state.UIDisplayed}
            loadedMeshes={this.state.loadedMeshes}
            updateMeshes={this.updateMeshes}
            updatePopup={this.updatePopup}
            updatePopupMessage={this.updatePopupMessage}
            editor={this.state.editor}
            updateLoading={this.updateLoading}
          />
          <PartLoader 
            loading={this.state.partloading}
            updateLoading={this.updateLoading}
          />
        </BrowserView>
        <MobileView>
          <div className="abs top left smartphone">
            <img src={logo} alt="company logo" />
            <div className="fullScreenMessage">
              <Typed
                strings={[
                  "Sorry, this content is currently unavailable on mobile... ^2000",
                  "Come back soon for updates!"
                ]}
                typeSpeed={40}
                showCursor={false}
              />
            </div>
          </div>
        </MobileView>
      </div>
    );
  }
}

export default App;


```

## Source file: src/index.js

{
  "path": "src/index.js",
  "blob_sha": "cac5b7751359ed51c8c6064105dcf199f85b655c",
  "sha256": "fcdfc78c933b7c15f9bc62375d71d726e3af0f9a02db2da161be3f679026d0f1",
  "lines": 7,
  "bytes": 240,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/williamclot/character_creator/blob/7e9a76b8e6c1497577e9d369bc15a1ab39a4da2d/src/index.js",
  "function_signature_candidates": []
}

```text

import React from 'react';
import ReactDOM from 'react-dom';
import App from './App';
import registerServiceWorker from './registerServiceWorker';

ReactDOM.render(<App />, document.getElementById('customizerUI'));
registerServiceWorker();


```

## Source file: public/js/skeleton.js

{
  "path": "public/js/skeleton.js",
  "blob_sha": "4a0e7f1dfab9baeadd5e8e9caa51cadad5c66a2f",
  "sha256": "6b24e3eb9db13b7ba3ffa3b7b27ae2e9f45989d9c76dc063d8e7a7ef44cc468c",
  "lines": 674,
  "bytes": 18089,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/williamclot/character_creator/blob/7e9a76b8e6c1497577e9d369bc15a1ab39a4da2d/public/js/skeleton.js",
  "function_signature_candidates": []
}

```text

/**
 * CHARACTER CREATOR
 *
 * Based on https://github.com/mrdoob/three.js/
 * Tested on r95
 * @author williamclot / https://github.com/williamclot
 */

//Threejs important variables
var camera, scene, renderer;
var controls, loader;

var selected = "Head";
var color = { r: 0.555, g: 0.48, b: 0.49 };
var group = new THREE.Group(); //this group will contain all the meshes but not the floor, the lights etc...
var bBoxStand;
window.loaded = false;
window.partloaded = false;

//This keeps track of every mesh on the viewport
var loadedMeshes = {
  Torso: {
    name: "turtle_torso",
    rotation: { x: 0, y: 0, z: 0 }
  },
  LegR: {
    name: "robot_leg_R",
    rotation: { x: 0, y: 0, z: 0 }
  },
  LegL: {
    name: "default_leg_L",
    rotation: { x: 0, y: 0, z: 0 }
  },
  Head: {
    name: "default_head",
    rotation: { x: 0, y: 0, z: 0 }
  },
  ArmR: {
    name: "thin_arm_R",
    rotation: { x: 0, y: 0, z: 0 }
  },
  ArmL: {
    name: "thin_arm_L",
    rotation: { x: 0, y: 0, z: 0 }
  },
  HandR: {
    name: "open_hand_R",
    rotation: { x: 0, y: -1.57, z: 0 }
  },
  HandL: {
    name: "open_hand_L",
    rotation: { x: 0, y: 1.57, z: 0 }
  },
  FootR: {
    name: "boots_R",
    rotation: { x: 0, y: 0, z: 0 }
  },
  FootL: {
    name: "boots_L",
    rotation: { x: 0, y: 0, z: 0 }
  },
  Stand: {
    name: "default",
    rotation: { x: 0, y: 0, z: 0 }
  }
};

// List of information on the meshes (attach points, body groups...)
var meshStaticInfo = {
  Torso: {
    bodyPart: "torso",
    parentAttachment: undefined,
    childAttachment: undefined
  },
  Head: {
    bodyPart: "head",
    parentAttachment: "Torso_Neck",
    childAttachment: "Head_Neck"
  },
  ArmR: {
    bodyPart: "arm",
    parentAttachment: "Torso_UpperArm_R",
    childAttachment: "ArmR_UpperArm_R"
  },
  ArmL: {
    bodyPart: "arm",
    parentAttachment: "Torso_UpperArm_L",
    childAttachment: "ArmL_UpperArm_L"
  },
  HandR: {
    bodyPart: "hand",
    parentAttachment: "ArmR_Hand_R",
    childAttachment: "HandR_Hand_R"
  },
  HandL: {
    bodyPart: "hand",
    parentAttachment: "ArmL_Hand_L",
    childAttachment: "HandL_Hand_L"
  },
  LegR: {
    bodyPart: "leg",
    parentAttachment: "Torso_UpperLeg_R",
    childAttachment: "LegR_UpperLeg_R"
  },
  LegL: {
    bodyPart: "leg",
    parentAttachment: "Torso_UpperLeg_L",
    childAttachment: "LegL_UpperLeg_L"
  },
  FootR: {
    bodyPart: "foot",
    parentAttachment: "LegR_Foot_R",
    childAttachment: "FootR_Foot_R"
  },
  FootL: {
    bodyPart: "foot",
    parentAttachment: "LegL_Foot_L",
    childAttachment: "FootL_Foot_L"
  }
};
// List of parent/child relations
var childrenList = {
  ArmR: ["HandR"],
  ArmL: ["HandL"],
  Torso: ["ArmR", "ArmL", "Head", "LegR", "LegL"],
  LegR: ["FootR"],
  LegL: ["FootL"]
};

init();
animate();

// Init Function which will create all the
// Threejs environment and load the default meshes
function init() {
  "use strict";

  scene = new THREE.Scene();
  loader = new THREE.GLTFLoader();
  // fogColor = new THREE.Color(0xffffff);

  scene.background = new THREE.Color(0xeeeeee);
  scene.fog = new THREE.Fog(0xeeeeee, 1, 20);

  scene.add(group);

  buildCamera();
  buildRenderer();
  buildControls();
  buildLights();
  buildFloor();

  function buildCamera() {
    camera = new THREE.PerspectiveCamera(
      75,
      (6 / 5) * (window.innerWidth / window.innerHeight),
      0.001,
      1000
    );

    // Camera position in space (will be controled by the OrbitControls later on)
    camera.position.z = 2;
    camera.position.x = -1;
    camera.position.y = 2;
  }
  function buildRenderer() {
    // Create a renderer with Antialiasing
    renderer = new THREE.WebGLRenderer({ antialias: true });
    var container = document.getElementById("canvas");
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap; // default THREE.PCFShadowMap

    renderer.setSize((6 / 5) * window.innerWidth, window.innerHeight); // Configure renderer size
    // Append Renderer to DOM
    container.appendChild(renderer.domElement);

    var size = 50;
    var divisions = 60;

    var gridHelper = new THREE.GridHelper(size, divisions);
    scene.add(gridHelper);
  }
  function buildControls() {
    controls = new THREE.OrbitControls(camera, renderer.domElement);
    // controls.target.set(-1,0,0);
    controls.minDistance = 2; //Controling max and min for ease of use
    controls.maxDistance = 7;
    controls.minPolarAngle = 0;
    controls.maxPolarAngle = Math.PI / 2 - 0.1;
    controls.enablePan = false;
  }
  function buildLights() {
    //hemisphere light: like sun light but without any shadows
    var hemi = new THREE.HemisphereLight(0xffffff, 0xffffff);
    scene.add(hemi);

    //Create a PointLight and turn on shadows for the light
    var light = new THREE.PointLight(0xc1c1c1, 1, 100);
    light.position.set(3, 10, 10);
    light.castShadow = true;
    //Set up shadow properties for the light
    light.shadow.mapSize.width = 2048; // default
    light.shadow.mapSize.height = 2048; // default
    light.decay = 1;
    scene.add(light);

    // This light is here to show the details in the back (no shadows)
    var backlight = new THREE.PointLight(0xc4b0ac, 1, 100);
    backlight.position.set(0, 2, -20);
    backlight.penumbra = 2;
    scene.add(backlight);
  }
  function buildFloor() {
    //Create a plane that receives shadows (but does not cast them)
    var planeGeometry = new THREE.PlaneGeometry(2000, 2000);
    // planeGeometry.rotateX( - Math.PI / 2 );

    var planeMaterial = new THREE.ShadowMaterial();
    planeMaterial.opacity = 0.2;

    var plane = new THREE.Mesh(planeGeometry, planeMaterial);
    plane.name = "plane";
    plane.rotation.x = -Math.PI / 2;
    plane.position.y = 0;
    plane.receiveShadow = true;
    scene.add(plane);
  }
}

function clearPosition(item) {
  // This function is used to clear the position of an imported gltf file
  item.position.x = 0;
  item.position.y = 0;
  item.position.z = 0;
}
function rotateElement(item, clearRotation, rotation) {
  if (clearRotation === true) {
    item.rotation.x = 0;
    item.rotation.y = 0;
    item.rotation.z = 0;
  } else {
    item.rotation.x = rotation.x;
    item.rotation.y = rotation.y;
    item.rotation.z = rotation.z;
  }
}
function placeMesh(
  meshName,
  bodyPartClass,
  MeshType,
  parentAttachment,
  childAttachment,
  rotation,
  firstLoad,
  highLight,
  bones,
  poseData
) {
  // bodyPartClass : {arm, head, hand, torso, leg, foot}
  // MeshType : {ArmR, ArmL, Head, HandR, HandL, LegR, LegL, FootR, FootL, Torso}
  loader.load(
    "models/" + bodyPartClass + "/" + meshName + ".glb",
    gltf => {
      var root = gltf.scene.children[0];
      root.traverse(function(child) {
        if (child instanceof THREE.Mesh) {
          // Gives a fixed name to the mesh and same gray color
          child.name = "mesh-"+MeshType.toLowerCase();
          child.castShadow = true;
          child.material.color = { r: 0.5, g: 0.5, b: 0.5 };
        }
      });

      // group is one element with all the meshes and bones of the character
      group.add(root);
      scene.updateMatrixWorld(true);

      // Updates the loadedMeshes variable (used for replacing children)
      loadedMeshes[MeshType].name = meshName;
      loadedMeshes[MeshType].rotation = rotation;

      if (MeshType === "Head" && firstLoad) {
        changeColor("Head", color);
      }

      if (highLight) {
        changeColor(MeshType, color);
      }

      // Putting the new mesh in the pose configuration if any pose as been selected
      if (poseData) {
        root.traverse(function(child) {
          if (child instanceof THREE.Bone) {
            if (poseData[child.name]) {
              window.changeRotation(child.name, poseData[child.name].x, "x");
              window.changeRotation(child.name, poseData[child.name].y, "y");
              window.changeRotation(child.name, poseData[child.name].z, "z");
            }
          }
        });
      }

      if (
        typeof parentAttachment !== "undefined" &&
        typeof childAttachment !== "undefined"
      ) {
        let targetBone = scene.getObjectByName(parentAttachment);
        let object = scene.getObjectByName(childAttachment);
        clearPosition(object);
        rotateElement(object, true);
        rotateElement(object, false, rotation);
        targetBone.add(object);
      }

      //Going to look for all children of current mesh
      let children = childrenList[MeshType];
      if (children) {
        for (let i = 0; i < children.length; i++) {
          replaceMesh(children[i], firstLoad, bones, poseData);
        }
      }

      if (MeshType === "FootR") {
        if (scene.getObjectByName("FootL_Toes_L")) {
          scene.updateMatrixWorld();
          placeStand();
        }
      } else if (MeshType === "FootL") {
        if (scene.getObjectByName("FootR_Toes_R")) {
          scene.updateMatrixWorld();
          placeStand();
        }
      }
      window.partloaded = true;
    },
    null,
    function(error) {
      console.log(error);
    }
  );
}

function replaceMesh(MeshType, firstLoad, bones, poseData) {
  group.remove(group.getObjectByName(MeshType));
  placeMesh(
    loadedMeshes[MeshType].name,
    meshStaticInfo[MeshType].bodyPart,
    MeshType,
    meshStaticInfo[MeshType].parentAttachment,
    meshStaticInfo[MeshType].childAttachment,
    loadedMeshes[MeshType].rotation,
    firstLoad,
    false,
    bones,
    poseData
  );
}

function placeStand() {
  // var topStand;
  var minFinder = new THREE.FindMinGeometry();

  if (scene.getObjectByName("mesh-stand")) {
    var resultR = minFinder.parse(scene.getObjectByName("FootR"));
    var resultL = minFinder.parse(scene.getObjectByName("FootL"));
    var result = resultL > resultR ? resultR : resultL;
    console.log(result);

    // bBoxStand = new THREE.Box3().setFromObject(root);
    // topStand = bBoxStand.max.y;
    scene.getObjectByName("Torso_Hip").position.y -= result;
  } else {
    loader.load(
      "models/stand/circle.glb",
      gltf => {
        var root = gltf.scene.children[0];

        root.traverse(function(child) {
          if (child instanceof THREE.Mesh) {
            child.name = "mesh-stand"
            child.castShadow = true;
            child.receiveShadow = true;
          }
        });

        var resultR = minFinder.parse(scene.getObjectByName("FootR"));
        var resultL = minFinder.parse(scene.getObjectByName("FootL"));
        var result = resultL > resultR ? resultR : resultL;

        //Default color to all the meshes
        if (root.material) {
          root.material.color = { r: 0.5, g: 0.5, b: 0.5 };
        }

        group.add(root);
        scene.getObjectByName("Torso_Hip").position.y -= result;
        window.loaded = true;
      },
      null,
      function(error) {
        console.log(error);
      }
    );
  }
}

window.changeStand = function(stand) {
  var minFinder = new THREE.FindMinGeometry();
  if (scene.getObjectByName("mesh-stand")) {
    group.remove(scene.getObjectByName("mesh-stand"));
    loader.load(
      "models/stand/" + stand + ".glb",
      gltf => {
        var root = gltf.scene.children[0];

        root.traverse(function(child) {
          if (child instanceof THREE.Mesh) {
            child.name = "mesh-stand"
            child.castShadow = true;
            child.receiveShadow = true;
            child.material.color = { r: 0.555, g: 0.48, b: 0.49 };
          }
        });

        var resultR = minFinder.parse(scene.getObjectByName("FootR"));
        var resultL = minFinder.parse(scene.getObjectByName("FootL"));
        var result = resultL > resultR ? resultR : resultL;

        group.add(root);
        scene.getObjectByName("Torso_Hip").position.y -= result;
      },
      null,
      function(error) {
        console.log(error);
      }
    );
  }
};
window.loadDefaultMeshes = function(bones, poseData) {
  placeMesh(
    loadedMeshes["Torso"].name,
    meshStaticInfo["Torso"].bodyPart,
    "Torso",
    undefined,
    undefined,
    undefined,
    true,
    false,
    bones,
    poseData
  );
};
window.changeMesh = function(bodyPart, part, isLeft, bones, poseData) {
  window.partloaded = false;
  var meshType;
  var file;
  var rotation;

  switch (bodyPart) {
    case "torso":
      file = part.file;
      rotation = undefined;
      meshType = "Torso";
      break;
    case "head":
      file = part.file;
      rotation = part.rotation;
      meshType = "Head";
      break;
    case "hand":
      meshType = isLeft ? "HandL" : "HandR";
      file = isLeft ? part.file[0] : part.file[1];
      rotation = isLeft ? part.rotation[0] : part.rotation[1];
      break;
    case "arm":
      meshType = isLeft ? "ArmL" : "ArmR";
      file = isLeft ? part.file[0] : part.file[1];
      rotation = isLeft ? part.rotation[0] : part.rotation[1];
      break;
    case "foot":
      meshType = isLeft ? "FootL" : "FootR";
      file = isLeft ? part.file[0] : part.file[1];
      rotation = isLeft ? part.rotation[0] : part.rotation[1];
      break;
    case "leg":
      meshType = isLeft ? "LegL" : "LegR";
      file = isLeft ? part.file[0] : part.file[1];
      rotation = isLeft ? part.rotation[0] : part.rotation[1];
      break;
    default:
      meshType = undefined;
  }

  if (meshType) {
    let parentAttachment = meshStaticInfo[meshType].parentAttachment;
    let childAttachment = meshStaticInfo[meshType].childAttachment;
    let currentMesh = group.getObjectByName(meshType);
    let bonesToDelete =
      meshType === "Torso"
        ? scene.getObjectByName("Torso_Hip")
        : group.getObjectByName(parentAttachment);

    if (currentMesh) {
      group.remove(currentMesh);
      if (bonesToDelete.children) {
        for (let i = 0; i < bonesToDelete.children.length; i++) {
          if (bonesToDelete.children[i] instanceof THREE.Bone) {
            bonesToDelete.remove(bonesToDelete.children[i]);
          }
        }
      }
      placeMesh(
        file,
        bodyPart,
        meshType,
        parentAttachment,
        childAttachment,
        rotation,
        false,
        true,
        bones,
        poseData
      );
    }
  }
  return true;
};
window.selectedMesh = function(MeshType) {
  console.log(MeshType, selected);
  let normal = { r: 0.5, g: 0.5, b: 0.5 };

  changeColor(selected, normal);
  changeColor(MeshType, color);

  selected = MeshType;
};
function changeColor(item, choosenColor) {
  var mesh = item === "pose" ? group : scene.getObjectByName(item);
  mesh.traverse(function(child) {
    if (child instanceof THREE.Mesh) {
      if (child.material) {
        child.material.color.r = choosenColor.r;
        child.material.color.g = choosenColor.g;
        child.material.color.b = choosenColor.b;
      }
    }
  });
}

window.changeRotation = function(bone_name, value, axis) {
  var bone = scene.getObjectByName(bone_name);
  if (bone instanceof THREE.Bone) {
    switch (axis) {
      case "x":
        bone.rotation.x = value;
        break;
      case "y":
        bone.rotation.y = value;
        break;
      case "z":
        bone.rotation.z = value;
        break;
      default:
    }
  }
};
window.getRotation = function(bone_name) {
  var bone = scene.getObjectByName(bone_name);
  if (bone instanceof THREE.Bone) {
    return { x: bone.rotation.x, y: bone.rotation.y, z: bone.rotation.z };
  }
};
window.loadPose = function(poseData, bones) {
  var L,
    R = false;
  for (let i = 0; i < bones.length; i++) {
    let bone = bones[i].bone;
    window.changeRotation(bone, poseData[bone].x, "x");
    window.changeRotation(bone, poseData[bone].y, "y");
    window.changeRotation(bone, poseData[bone].z, "z");

    scene.updateMatrixWorld();

    if (bone === "LegL_Foot_L") {
      L = true;
      if (L && R) {
        placeStand();
      }
    }
    if (bone === "LegR_Foot_R") {
      R = true;
      if (L && R) {
        placeStand();
      }
    }
  }
};
window.export = function(name) {
  var exporter = new THREE.STLExporter();

  if (name) {
    saveString(exporter.parse(group), name + ".stl");
  } else {
    var stlList = []
    // I need to know in which order the files are exported...
    var Meshes = [
      "mesh-stand",
      "mesh-torso",
      "mesh-arml",
      "mesh-armr",
      "mesh-footl",
      "mesh-footr",
      "mesh-handl",
      "mesh-handr",
      "mesh-head",
      "mesh-legl",
      "mesh-legr",
      "mesh-neck"
    ];
    for (var i = 0; i < Meshes.length; i++) {
      group.traverse(function (child) {
        if (child.name === Meshes[i]) {
          stlList.push(exporter.parse(child))
        }
      });
    }
    return stlList

    scene.traverse(function(child){
      if (child instanceof THREE.Mesh){
        console.log(child.name)
      }
    })
  }
};

function onWindowResize() {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
}
function animate() {
  requestAnimationFrame(animate);
  controls.update();
  render();
}
function render() {
  camera.lookAt(new THREE.Vector3(0, 1, 0));
  renderer.render(scene, camera);
}
document.body.onresize = function() {
  renderer.setSize((6 / 5) * window.innerWidth, window.innerHeight); //size of viewport
  camera.aspect = ((6 / 5) * window.innerWidth) / window.innerHeight; //aspect ratio update
  camera.updateProjectionMatrix();
  renderer.domElement.style.position = -(1 / 5) * window.innerWidth;
};

var link = document.createElement("a");
link.style.display = "none";
document.body.appendChild(link); // Firefox workaround, see #6594

function save(blob, filename) {
  link.href = URL.createObjectURL(blob);
  link.download = filename || "data.json";
  link.click();
}
function saveArrayBuffer(buffer, filename) {
  save(new Blob([buffer], { type: "application/octet-stream" }), filename);
}
function saveString(text, filename) {
  save(new Blob([text], { type: "text/plain" }), filename);
}

window.addEventListener( 'click', onMouseClick, false );

```

## Source file: src/components/Logo.js

{
  "path": "src/components/Logo.js",
  "blob_sha": "472dd231fc0d357e24ee15a785255a690aef7bd8",
  "sha256": "e8b1599fa48483a3467de7b0ae8e68d938857bd354f4ec2626c24046e3798d67",
  "lines": 20,
  "bytes": 416,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/williamclot/character_creator/blob/7e9a76b8e6c1497577e9d369bc15a1ab39a4da2d/src/components/Logo.js",
  "function_signature_candidates": []
}

```text

import React, { Component } from "react";

import "../css/logo.css";
import logo from "../graphic_assets/logo_with_base.png";

class Logo extends Component {

  render() {
    return (
      <div className="abs logo">
        <a href="https://myminifactory.com">
          <span className="abs">Powered by: </span>
          <img src={logo} alt="logo" />
        </a>
      </div>
    );
  }
}

export default Logo;


```
