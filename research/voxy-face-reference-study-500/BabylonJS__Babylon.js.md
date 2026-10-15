# Facial rendering source scan: BabylonJS/Babylon.js

Repository: https://github.com/BabylonJS/Babylon.js

Pinned source revision: 02b5a5e79b0140f79dc75f703e82cb79d8b5fa4c

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["NOTICE.md", "license.md", "packages/dev/smartFilterBlocks/license.md", "packages/dev/smartFilters/license.md", "packages/public/@babylonjs/accessibility/license.md", "packages/public/@babylonjs/addons/NOTICE.md", "packages/public/@babylonjs/addons/license.md", "packages/public/@babylonjs/core/NOTICE.md", "packages/public/@babylonjs/core/license.md", "packages/public/@babylonjs/gui-editor/license.md", "packages/public/@babylonjs/gui/license.md", "packages/public/@babylonjs/inspector-v2/license.md", "packages/public/@babylonjs/inspector/license.md", "packages/public/@babylonjs/ktx2decoder/NOTICE.md", "packages/public/@babylonjs/ktx2decoder/license.md", "packages/public/@babylonjs/ktx2decoder/wasm/license.md", "packages/public/@babylonjs/loaders/BVH/license.md", "packages/public/@babylonjs/loaders/license.md", "packages/public/@babylonjs/materials/license.md", "packages/public/@babylonjs/node-editor/license.md"]

Topic paths: {"eye": ["packages/dev/core/src/XR/features/WebXREyeTracking.pure.ts", "packages/dev/core/src/XR/features/WebXREyeTracking.ts", "packages/dev/core/test/unit/Materials/babylon.uniformBuffer.ownerKeyedSlots.test.ts", "packages/dev/inspector-v2/src/services/textureEditor/tools/eyedropper.tsx", "packages/dev/inspector/src/components/actionTabs/tabs/propertyGrids/materials/textures/assets/eyeClosed.svg", "packages/dev/inspector/src/components/actionTabs/tabs/propertyGrids/materials/textures/assets/eyeOpen.svg", "packages/dev/inspector/src/components/actionTabs/tabs/propertyGrids/materials/textures/defaultTools/eyedropper.ts"], "skin": ["packages/tools/nme-mcp-server/examples/HumanSkin.json", "packages/tools/tests/test/visualization/ReferenceImages/FrameGraph-motion-blur-skinned.png", "packages/tools/tests/test/visualization/ReferenceImages/fbx-m09-skinning.png", "packages/tools/tests/test/visualization/ReferenceImages/gltfAnimationSkin0.png", "packages/tools/tests/test/visualization/ReferenceImages/gltfAnimationSkin1.png", "packages/tools/tests/test/visualization/ReferenceImages/gltfAnimationSkinType.png", "packages/tools/tests/test/visualization/ReferenceImages/gltfSerializerSkinningAndAnimation.png"], "subsurface": ["packages/dev/core/src/Materials/Node/Blocks/PBR/subSurfaceBlock.pure.ts", "packages/dev/core/src/Materials/Node/Blocks/PBR/subSurfaceBlock.ts", "packages/dev/core/src/Materials/PBR/pbrSubSurfaceConfiguration.ts", "packages/dev/core/src/PostProcesses/subSurfaceScatteringPostProcess.pure.ts", "packages/dev/core/src/PostProcesses/subSurfaceScatteringPostProcess.ts", "packages/dev/core/src/Rendering/subSurfaceConfiguration.ts", "packages/dev/core/src/Rendering/subSurfaceSceneComponent.pure.ts", "packages/dev/core/src/Rendering/subSurfaceSceneComponent.ts", "packages/dev/core/src/Rendering/subSurfaceSceneComponent.types.ts", "packages/dev/core/src/Shaders/ShadersInclude/openpbrSubsurfaceLayerData.fx", "packages/dev/core/src/Shaders/ShadersInclude/pbrBlockSubSurface.fx", "packages/dev/core/src/Shaders/ShadersInclude/subSurfaceScatteringFunctions.fx", "packages/dev/core/src/Shaders/subSurfaceScattering.fragment.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/openpbrSubsurfaceLayerData.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/pbrBlockSubSurface.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/subSurfaceScatteringFunctions.fx", "packages/dev/core/src/ShadersWGSL/subSurfaceScattering.fragment.fx", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Color-Scaling---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Dense-Scattering---Analytic-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Dense-Scattering---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Dense-Scattering---Realtime-IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Radius-vs-Anisotropy---Analytic-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Radius-vs-Anisotropy---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Radius-vs-Radius-Scale---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Radius-vs-Roughness---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-SSS-Frame-Graph---Analytic-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-SSS-Frame-Graph---Shadows---Combination-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-SSS-Frame-Graph---Shadows---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Transmission-Blending---Analytic-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-Transmission-Blending---IBL.png"], "sss": ["packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-SSS-Frame-Graph---Analytic-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-SSS-Frame-Graph---Shadows---Combination-Lights.png", "packages/tools/tests/test/visualization/ReferenceImages/OpenPBR-Subsurface-SSS-Frame-Graph---Shadows---IBL.png", "packages/tools/tests/test/visualization/ReferenceImages/needDepthPrePassStandardAlphaTest.png", "packages/tools/tests/test/visualization/ReferenceImages/ssShadowsLeftHanded.png", "packages/tools/tests/test/visualization/ReferenceImages/ssShadowsRightHanded.png"], "blendshape": ["packages/dev/loaders/src/FBX/interpreter/blendShapes.ts", "packages/dev/loaders/test/unit/FBX/interpreter/blendShapes.test.ts"], "flame": ["packages/tools/playground/public/textures/FlameBlastSpriteSheet.png", "scripts/folderSizeFlameChart.mjs"], "shader": ["packages/dev/addons/src/atmosphere/Shaders/ShadersInclude/atmosphereFragmentDeclaration.fx", "packages/dev/addons/src/atmosphere/Shaders/ShadersInclude/atmosphereFunctions.fx", "packages/dev/addons/src/atmosphere/Shaders/ShadersInclude/atmosphereUboDeclaration.fx", "packages/dev/addons/src/atmosphere/Shaders/ShadersInclude/atmosphereVertexDeclaration.fx", "packages/dev/addons/src/atmosphere/Shaders/ShadersInclude/depthFunctions.fx", "packages/dev/addons/src/atmosphere/Shaders/aerialPerspective.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/compositeAerialPerspective.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/compositeGlobeAtmosphere.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/compositeSky.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/diffuseSkyIrradiance.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/fullscreenTriangle.vertex.fx", "packages/dev/addons/src/atmosphere/Shaders/multiScattering.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/skyView.fragment.fx", "packages/dev/addons/src/atmosphere/Shaders/transmittance.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/ShadersInclude/atmosphereFunctions.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/ShadersInclude/atmosphereUboDeclaration.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/ShadersInclude/depthFunctions.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/aerialPerspective.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/compositeAerialPerspective.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/compositeGlobeAtmosphere.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/compositeSky.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/diffuseSkyIrradiance.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/fullscreenTriangle.vertex.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/multiScattering.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/skyView.fragment.fx", "packages/dev/addons/src/atmosphere/ShadersWGSL/transmittance.fragment.fx", "packages/dev/addons/src/msdfText/shaders/msdf.fragment.fx", "packages/dev/addons/src/msdfText/shaders/msdf.vertex.fx", "packages/dev/addons/src/msdfText/shadersWGSL/msdf.fragment.fx", "packages/dev/addons/src/msdfText/shadersWGSL/msdf.vertex.fx"], "rig": [".prettierignore", "packages/dev/core/src/Cameras/RigModes/index.ts", "packages/dev/core/src/Cameras/RigModes/pure.ts", "packages/dev/core/src/Cameras/RigModes/stereoscopicAnaglyphRigMode.ts", "packages/dev/core/src/Cameras/RigModes/stereoscopicRigMode.ts", "packages/dev/core/src/Cameras/RigModes/vrRigMode.ts", "packages/dev/core/src/Materials/Node/Blocks/trigonometryBlock.pure.ts", "packages/dev/core/src/Materials/Node/Blocks/trigonometryBlock.ts", "packages/dev/core/src/Materials/floatingOriginMatrixOverrides.ts", "packages/dev/core/src/Meshes/Node/Blocks/geometryTrigonometryBlock.pure.ts", "packages/dev/core/src/Meshes/Node/Blocks/geometryTrigonometryBlock.ts", "packages/dev/core/src/Particles/Node/Blocks/Triggers/particleTriggerBlock.pure.ts", "packages/dev/core/src/Particles/Node/Blocks/Triggers/particleTriggerBlock.ts", "packages/dev/core/src/Particles/Node/Blocks/Triggers/pure.ts", "packages/dev/core/src/Particles/Node/Blocks/Triggers/triggerTools.ts", "packages/dev/core/src/Particles/Node/Blocks/particleTrigonometryBlock.pure.ts", "packages/dev/core/src/Particles/Node/Blocks/particleTrigonometryBlock.ts", "packages/dev/loaders/src/FBX/interpreter/rig.ts", "packages/dev/sharedUiComponents/src/imgs/hAlignRightIcon.svg", "packages/tools/flowGraphEditor/test/playwright/fge.utils.ts", "packages/tools/flowGraphEditor/test/playwright/flowGraphEditor.test.ts", "packages/tools/guiEditor/src/imgs/cursor_scaleDiagonalRight.svg", "packages/tools/nodeEditor/src/components/preview/svgs/directionalRight.svg", "packages/tools/nodeEditor/src/graphSystem/display/trigonometryDisplayManager.module.scss", "packages/tools/nodeEditor/src/graphSystem/display/trigonometryDisplayManager.ts", "packages/tools/nodeParticleEditor/src/graphSystem/display/triggerDisplayManager.module.scss", "packages/tools/nodeParticleEditor/src/graphSystem/display/triggerDisplayManager.ts", "packages/tools/nodeRenderGraphEditor/src/components/preview/svgs/directionalRight.svg", "packages/tools/playground/public/textures/Space/space_right.jpg", "packages/tools/tests/playwright.utils.ts"], "face": ["packages/dev/core/src/Animations/animatable.interface.ts", "packages/dev/core/src/Audio/Interfaces/IAudioEngine.ts", "packages/dev/core/src/Audio/Interfaces/IAudioEngineOptions.ts", "packages/dev/core/src/Audio/Interfaces/ISoundOptions.ts", "packages/dev/core/src/Behaviors/Meshes/surfaceMagnetismBehavior.ts", "packages/dev/core/src/DeviceInput/inputInterfaces.ts", "packages/dev/core/src/Engines/Native/nativeInterfaces.ts", "packages/dev/core/src/Materials/GreasedLine/greasedLineMaterialInterfaces.ts", "packages/dev/core/src/Materials/Node/Blocks/PBR/subSurfaceBlock.pure.ts", "packages/dev/core/src/Materials/Node/Blocks/PBR/subSurfaceBlock.ts", "packages/dev/core/src/Materials/PBR/pbrSubSurfaceConfiguration.ts", "packages/dev/core/src/Materials/Textures/Loaders/EXR/exrLoader.interfaces.ts", "packages/dev/core/src/Meshes/Node/Blocks/Instances/instantiateOnFacesBlock.pure.ts", "packages/dev/core/src/Meshes/Node/Blocks/Instances/instantiateOnFacesBlock.ts", "packages/dev/core/src/Meshes/Node/Interfaces/nodeGeometryExecutionContext.ts", "packages/dev/core/src/Meshes/Node/Interfaces/nodeGeometryInstancingContext.ts", "packages/dev/core/src/Meshes/Node/Interfaces/nodeGeometryTextureData.ts", "packages/dev/core/src/Misc/interfaces/iClipPlanesHolder.ts", "packages/dev/core/src/Misc/interfaces/iPerfViewer.ts", "packages/dev/core/src/Misc/interfaces/iWebRequest.ts", "packages/dev/core/src/Misc/interfaces/screenshotSize.ts", "packages/dev/core/src/ObjectModel/objectModelInterfaces.ts", "packages/dev/core/src/PostProcesses/subSurfaceScatteringPostProcess.pure.ts", "packages/dev/core/src/PostProcesses/subSurfaceScatteringPostProcess.ts", "packages/dev/core/src/Rendering/subSurfaceConfiguration.ts", "packages/dev/core/src/Rendering/subSurfaceSceneComponent.pure.ts", "packages/dev/core/src/Rendering/subSurfaceSceneComponent.ts", "packages/dev/core/src/Rendering/subSurfaceSceneComponent.types.ts", "packages/dev/core/src/Shaders/ShadersInclude/openpbrSubsurfaceLayerData.fx", "packages/dev/core/src/Shaders/ShadersInclude/pbrBlockSubSurface.fx"], "lip": ["packages/dev/core/src/Behaviors/Cameras/geospatialClippingBehavior.ts", "packages/dev/core/src/Events/clipboardEvents.ts", "packages/dev/core/src/FlowGraph/Blocks/Execution/ControlFlow/flowGraphFlipFlopBlock.pure.ts", "packages/dev/core/src/FlowGraph/Blocks/Execution/ControlFlow/flowGraphFlipFlopBlock.ts", "packages/dev/core/src/Materials/Node/Blocks/Dual/clipPlanesBlock.pure.ts", "packages/dev/core/src/Materials/Node/Blocks/Dual/clipPlanesBlock.ts", "packages/dev/core/src/Materials/clipPlaneMaterialHelper.ts", "packages/dev/core/src/Misc/interfaces/iClipPlanesHolder.ts", "packages/dev/core/src/Shaders/ShadersInclude/clipPlaneFragment.fx", "packages/dev/core/src/Shaders/ShadersInclude/clipPlaneFragmentDeclaration.fx", "packages/dev/core/src/Shaders/ShadersInclude/clipPlaneFragmentDeclaration2.fx", "packages/dev/core/src/Shaders/ShadersInclude/clipPlaneVertex.fx", "packages/dev/core/src/Shaders/ShadersInclude/clipPlaneVertexDeclaration.fx", "packages/dev/core/src/Shaders/ShadersInclude/clipPlaneVertexDeclaration2.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/clipPlaneFragment.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/clipPlaneFragmentDeclaration.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/clipPlaneVertex.fx", "packages/dev/core/src/ShadersWGSL/ShadersInclude/clipPlaneVertexDeclaration.fx", "packages/dev/gui/src/2D/controls/ellipse.pure.ts", "packages/dev/gui/src/2D/controls/ellipse.ts", "packages/dev/sharedUiComponents/src/copyCommandToClipboard.ts", "packages/dev/sharedUiComponents/src/imgs/clipChildrenIcon.svg", "packages/dev/sharedUiComponents/src/imgs/clipContentsIcon.svg", "packages/dev/sharedUiComponents/src/imgs/ellipseIcon.svg", "packages/dev/sharedUiComponents/src/tabs/propertyGrids/gui/ellipsePropertyGridComponent.tsx", "packages/dev/sharedUiComponents/test/unit/copyCommandToClipboard.test.ts", "packages/tools/guiEditor/src/components/propertyTab/propertyGrids/gui/ellipsePropertyGridComponent.tsx", "packages/tools/guiEditor/src/imgs/ellipseIcon.svg", "packages/tools/guiEditor/src/imgs/ellipseIconDark.svg", "packages/tools/tests/test/visualization/ReferenceImages/clipplanes.png"]}

## Source file: packages/dev/core/src/Rendering/subSurfaceConfiguration.ts

{
  "path": "packages/dev/core/src/Rendering/subSurfaceConfiguration.ts",
  "blob_sha": "8dcfff93ea45b0ee428ad8e1f9c03c5762153e04",
  "sha256": "226d291f91d865002c48eb6d9dda6d8ae2e74eab41825c64309d3d7ff31d355c",
  "lines": 217,
  "bytes": 7763,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 22,
    "sss": 2,
    "shader": 2,
    "face": 22
  },
  "url": "https://github.com/BabylonJS/Babylon.js/blob/02b5a5e79b0140f79dc75f703e82cb79d8b5fa4c/packages/dev/core/src/Rendering/subSurfaceConfiguration.ts",
  "function_signature_candidates": []
}

```text

import { Logger } from "../Misc/logger";
import { type Scene } from "../scene";
import { Color3, Color4 } from "../Maths/math.color.pure";
import { SubSurfaceScatteringPostProcess } from "../PostProcesses/subSurfaceScatteringPostProcess.pure";
import { SceneComponentConstants } from "../sceneComponent";
import { type PrePassEffectConfiguration } from "./prePassEffectConfiguration";
import { _WarnImport } from "../Misc/devTools";
import { Constants } from "../Engines/constants";
import { ShaderLanguage } from "core/Materials/shaderLanguage";
import { RegisterSubSurfaceSceneComponent } from "./subSurfaceSceneComponent.pure";

/**
 * Contains all parameters needed for the prepass to perform
 * screen space subsurface scattering
 */
export class SubSurfaceConfiguration implements PrePassEffectConfiguration {
    /**
     * @internal
     */
    public static _SceneComponentInitialization: (scene: Scene) => void = (_) => {
        throw _WarnImport("SubSurfaceSceneComponent");
    };

    private _ssDiffusionS: number[] = [];
    private _ssFilterRadii: number[] = [];
    private _ssDiffusionD: number[] = [];

    /**
     * Post process to attach for screen space subsurface scattering
     */
    public postProcess: SubSurfaceScatteringPostProcess;

    /**
     * Diffusion profile color for subsurface scattering
     */
    public get ssDiffusionS() {
        return this._ssDiffusionS;
    }

    /**
     * Diffusion profile max color channel value for subsurface scattering
     */
    public get ssDiffusionD() {
        return this._ssDiffusionD;
    }

    /**
     * Diffusion profile filter radius for subsurface scattering
     */
    public get ssFilterRadii() {
        return this._ssFilterRadii;
    }

    /**
     * Is subsurface enabled
     */
    public enabled = false;

    /**
     * Does the output of this prepass need to go through imageprocessing
     */
    public needsImageProcessing = true;

    /**
     * Name of the configuration
     */
    public name = SceneComponentConstants.NAME_SUBSURFACE;

    /**
     * Diffusion profile colors for subsurface scattering
     * You can add one diffusion color using `addDiffusionProfile` on `scene.prePassRenderer`
     * See ...
     * Note that you can only store up to 5 of them
     */
    public ssDiffusionProfileColors: Color3[] = [];

    /**
     * Defines the ratio real world => scene units.
     * Used for subsurface scattering
     */
    public metersPerUnit: number = 1;

    /**
     * Textures that should be present in the MRT for this effect to work
     */
    public readonly texturesRequired: number[] = [
        Constants.PREPASS_DEPTH_TEXTURE_TYPE,
        Constants.PREPASS_ALBEDO_SQRT_TEXTURE_TYPE,
        Constants.PREPASS_COLOR_TEXTURE_TYPE,
        Constants.PREPASS_IRRADIANCE_LEGACY_TEXTURE_TYPE,
    ];

    /**
     * The clear color of the render targets.
     * We need 1 for the alpha channel of the irradiance texture so that we early exit from the SSS post-process if the pixel should not be processed
     */
    public clearColor = new Color4(0, 0, 0, 1);

    private _scene: Scene;

    /**
     * Builds a subsurface configuration object
     * @param scene The scene
     */
    constructor(scene: Scene) {
        // Adding default diffusion profile
        this.addDiffusionProfile(new Color3(1, 1, 1));
        this._scene = scene;

        RegisterSubSurfaceSceneComponent(SubSurfaceConfiguration);
        SubSurfaceConfiguration._SceneComponentInitialization(this._scene);
    }

    /**
     * Adds a new diffusion profile.
     * Useful for more realistic subsurface scattering on diverse materials.
     * @param color The color of the diffusion profile. Should be the average color of the material.
     * @returns The index of the diffusion profile for the material subsurface configuration
     */
    public addDiffusionProfile(color: Color3): number {
        if (this.ssDiffusionD.length >= 5) {
            // We only suppport 5 diffusion profiles
            Logger.Error("You already reached the maximum number of diffusion profiles.");
            return 0; // default profile
        }

        // Do not add doubles
        for (let i = 0; i < this._ssDiffusionS.length / 3; i++) {
            if (this._ssDiffusionS[i * 3] === color.r && this._ssDiffusionS[i * 3 + 1] === color.g && this._ssDiffusionS[i * 3 + 2] === color.b) {
                return i;
            }
        }

        this._ssDiffusionS.push(color.r, color.g, color.b);
        this._ssDiffusionD.push(Math.max(Math.max(color.r, color.b), color.g));
        this._ssFilterRadii.push(this.getDiffusionProfileParameters(color));
        this.ssDiffusionProfileColors.push(color);

        return this._ssDiffusionD.length - 1;
    }

    /**
     * Creates the sss post process
     * @returns The created post process
     */
    public createPostProcess(): SubSurfaceScatteringPostProcess {
        this.postProcess = new SubSurfaceScatteringPostProcess("subSurfaceScattering", this._scene, {
            size: 1,
            engine: this._scene.getEngine(),
            shaderLanguage: this._scene.getEngine().isWebGPU ? ShaderLanguage.WGSL : ShaderLanguage.GLSL,
        });
        this.postProcess.autoClear = false;

        return this.postProcess;
    }

    /**
     * Deletes all diffusion profiles.
     * Note that in order to render subsurface scattering, you should have at least 1 diffusion profile.
     */
    public clearAllDiffusionProfiles() {
        this._ssDiffusionD = [];
        this._ssDiffusionS = [];
        this._ssFilterRadii = [];
        this.ssDiffusionProfileColors = [];
    }

    /**
     * Disposes this object
     */
    public dispose() {
        this.clearAllDiffusionProfiles();
        if (this.postProcess) {
            this.postProcess.dispose();
        }
    }

    /**
     * @internal
     * https://zero-radiance.github.io/post/sampling-diffusion/
     *
     * Importance sample the normalized diffuse reflectance profile for the computed value of 's'.
     * ------------------------------------------------------------------------------------
     * R[r, phi, s]   = s * (Exp[-r * s] + Exp[-r * s / 3]) / (8 * Pi * r)
     * PDF[r, phi, s] = r * R[r, phi, s]
     * CDF[r, s]      = 1 - 1/4 * Exp[-r * s] - 3/4 * Exp[-r * s / 3]
     * ------------------------------------------------------------------------------------
     * We importance sample the color channel with the widest scattering distance.
     */
    public getDiffusionProfileParameters(color: Color3) {
        const cdf = 0.997;
        const maxScatteringDistance = Math.max(color.r, color.g, color.b);

        return this._sampleBurleyDiffusionProfile(cdf, maxScatteringDistance);
    }

    /**
     * Performs sampling of a Normalized Burley diffusion profile in polar coordinates.
     * 'u' is the random number (the value of the CDF): [0, 1).
     * rcp(s) = 1 / ShapeParam = ScatteringDistance.
     * Returns the sampled radial distance, s.t. (u = 0 -> r = 0) and (u = 1 -> r = Inf).
     * @param u
     * @param rcpS
     * @returns The sampled radial distance
     */
    private _sampleBurleyDiffusionProfile(u: number, rcpS: number) {
        u = 1 - u; // Convert CDF to CCDF

        const g = 1 + 4 * u * (2 * u + Math.sqrt(1 + 4 * u * u));
        const n = Math.pow(g, -1.0 / 3.0); // g^(-1/3)
        const p = g * n * n; // g^(+1/3)
        const c = 1 + p + n; // 1 + g^(+1/3) + g^(-1/3)
        const x = 3 * Math.log(c / (4 * u));

        return x * rcpS;
    }
}


```

## Source file: packages/dev/core/src/Rendering/subSurfaceSceneComponent.ts

{
  "path": "packages/dev/core/src/Rendering/subSurfaceSceneComponent.ts",
  "blob_sha": "dd66ad4b182a68d66071b538dac1ab18d69e2bda",
  "sha256": "6e1aa228af27a2afc85904785cd37f219f16c20d8167f21bb365e6210924a4aa",
  "lines": 10,
  "bytes": 472,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 6,
    "face": 6
  },
  "url": "https://github.com/BabylonJS/Babylon.js/blob/02b5a5e79b0140f79dc75f703e82cb79d8b5fa4c/packages/dev/core/src/Rendering/subSurfaceSceneComponent.ts",
  "function_signature_candidates": []
}

```text

export * from "./subSurfaceSceneComponent.types";
/**
 * Re-exports pure implementation and applies runtime side effects.
 * Import subSurfaceSceneComponent.pure for tree-shakeable, side-effect-free usage.
 */
export * from "./subSurfaceSceneComponent.pure";

import { SubSurfaceConfiguration } from "./subSurfaceConfiguration";
import { RegisterSubSurfaceSceneComponent } from "./subSurfaceSceneComponent.pure";
RegisterSubSurfaceSceneComponent(SubSurfaceConfiguration);


```

## Source file: packages/dev/core/src/Rendering/subSurfaceSceneComponent.pure.ts

{
  "path": "packages/dev/core/src/Rendering/subSurfaceSceneComponent.pure.ts",
  "blob_sha": "f4b1d862d99e3c32688960785d8c6371eda47171",
  "sha256": "4e0f8ae478c0f6266bbcdd1062ce26aa5652b8581e1691f3d81ebba0230f3712",
  "lines": 170,
  "bytes": 5894,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 31,
    "face": 31
  },
  "url": "https://github.com/BabylonJS/Babylon.js/blob/02b5a5e79b0140f79dc75f703e82cb79d8b5fa4c/packages/dev/core/src/Rendering/subSurfaceSceneComponent.pure.ts",
  "function_signature_candidates": []
}

```text

/** This file must only contain pure code and pure imports */

import { Scene } from "../scene.pure";
import { SceneComponentConstants, type ISceneSerializableComponent } from "../sceneComponent";

import { type Nullable } from "../types";
import { Color3 } from "../Maths/math.color.pure";
import { AddParser } from "core/Loading/Plugins/babylonFileParser.function";
import { type SubSurfaceConfiguration } from "./subSurfaceConfiguration";

/**
 * Defines the Geometry Buffer scene component responsible to manage a G-Buffer useful
 * in several rendering techniques.
 */
export class SubSurfaceSceneComponent implements ISceneSerializableComponent {
    /**
     * The component name helpful to identify the component in the list of scene components.
     */
    public readonly name = SceneComponentConstants.NAME_SUBSURFACE;

    /**
     * The scene the component belongs to.
     */
    public scene: Scene;

    /**
     * Creates a new instance of the component for the given scene
     * @param scene Defines the scene to register the component in
     */
    constructor(scene: Scene) {
        this.scene = scene;
    }

    /**
     * Registers the component in a given scene
     */
    public register(): void {}

    /**
     * Serializes the component data to the specified json object
     * @param serializationObject The object to serialize to
     */
    public serialize(serializationObject: any): void {
        if (!this.scene.subSurfaceConfiguration) {
            return;
        }

        const ssDiffusionProfileColors = this.scene.subSurfaceConfiguration.ssDiffusionProfileColors;
        serializationObject.ssDiffusionProfileColors = [];

        for (let i = 0; i < ssDiffusionProfileColors.length; i++) {
            serializationObject.ssDiffusionProfileColors.push({
                r: ssDiffusionProfileColors[i].r,
                g: ssDiffusionProfileColors[i].g,
                b: ssDiffusionProfileColors[i].b,
            });
        }
    }

    /**
     * Adds all the elements from the container to the scene
     */
    public addFromContainer(): void {
        // Nothing to do
    }

    /**
     * Removes all the elements in the container from the scene
     */
    public removeFromContainer(): void {
        // Make sure nothing will be serialized
        if (!this.scene.prePassRenderer) {
            return;
        }

        if (this.scene.subSurfaceConfiguration) {
            this.scene.subSurfaceConfiguration.clearAllDiffusionProfiles();
        }
    }

    /**
     * Rebuilds the elements related to this component in case of
     * context lost for instance.
     */
    public rebuild(): void {
        // Nothing to do for this component
    }

    /**
     * Disposes the component and the associated resources
     */
    public dispose(): void {
        // Nothing to do for this component
    }
}

let _Registered = false;
/**
 * Register side effects for subSurfaceSceneComponent.
 * Safe to call multiple times; only the first call has an effect.
 * @param subSurfaceConfigClass The SubSurfaceConfiguration class to register the component for
 */
export function RegisterSubSurfaceSceneComponent(subSurfaceConfigClass: typeof SubSurfaceConfiguration): void {
    if (_Registered) {
        return;
    }
    _Registered = true;

    // Adds the parser to the scene parsers.
    AddParser(SceneComponentConstants.NAME_SUBSURFACE, (parsedData: any, scene: Scene) => {
        // Diffusion profiles
        if (parsedData.ssDiffusionProfileColors !== undefined && parsedData.ssDiffusionProfileColors !== null) {
            scene.enableSubSurfaceForPrePass();
            if (scene.subSurfaceConfiguration) {
                for (let index = 0, cache = parsedData.ssDiffusionProfileColors.length; index < cache; index++) {
                    const color = parsedData.ssDiffusionProfileColors[index];
                    scene.subSurfaceConfiguration.addDiffusionProfile(new Color3(color.r, color.g, color.b));
                }
            }
        }
    });

    Object.defineProperty(Scene.prototype, "subSurfaceConfiguration", {
        get: function (this: Scene) {
            return this._subSurfaceConfiguration;
        },
        set: function (this: Scene, value: Nullable<SubSurfaceConfiguration>) {
            if (value) {
                if (this.enablePrePassRenderer()) {
                    this._subSurfaceConfiguration = value;
                }
            }
        },
        enumerable: true,
        configurable: true,
    });

    Scene.prototype.enableSubSurfaceForPrePass = function (): Nullable<SubSurfaceConfiguration> {
        if (this._subSurfaceConfiguration) {
            return this._subSurfaceConfiguration;
        }

        const prePassRenderer = this.enablePrePassRenderer();
        if (prePassRenderer) {
            this._subSurfaceConfiguration = new subSurfaceConfigClass(this);
            prePassRenderer.addEffectConfiguration(this._subSurfaceConfiguration);
            return this._subSurfaceConfiguration;
        }

        return null;
    };

    Scene.prototype.disableSubSurfaceForPrePass = function (): void {
        if (!this._subSurfaceConfiguration) {
            return;
        }

        this._subSurfaceConfiguration.dispose();
        this._subSurfaceConfiguration = null;
    };

    subSurfaceConfigClass._SceneComponentInitialization = (scene: Scene) => {
        // Register the G Buffer component to the scene.
        let component = scene._getComponent(SceneComponentConstants.NAME_SUBSURFACE) as SubSurfaceSceneComponent;
        if (!component) {
            component = new SubSurfaceSceneComponent(scene);
            scene._addComponent(component);
        }
    };
}


```

## Source file: packages/dev/core/src/Materials/PBR/pbrSubSurfaceConfiguration.ts

{
  "path": "packages/dev/core/src/Materials/PBR/pbrSubSurfaceConfiguration.ts",
  "blob_sha": "1c27fc23653df372e9edf74efcf3db0591cedef0",
  "sha256": "321ad7e63d43a55d7bd01c5baeac655dc50672d3afb65e22620fcca595ed2530",
  "lines": 940,
  "bytes": 42452,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 1,
    "subsurface": 36,
    "sss": 4,
    "shader": 3,
    "rig": 1,
    "face": 50
  },
  "url": "https://github.com/BabylonJS/Babylon.js/blob/02b5a5e79b0140f79dc75f703e82cb79d8b5fa4c/packages/dev/core/src/Materials/PBR/pbrSubSurfaceConfiguration.ts",
  "function_signature_candidates": []
}

```text

/* eslint-disable @typescript-eslint/naming-convention */
import { type Nullable } from "../../types";
import { type IAnimatable } from "../../Animations/animatable.interface";
import { serialize, serializeAsTexture, expandToProperty, serializeAsColor3 } from "../../Misc/decorators";
import { Color3 } from "../../Maths/math.color.pure";
import { type SmartArray } from "../../Misc/smartArray";
import { type BaseTexture } from "../../Materials/Textures/baseTexture";
import { type RenderTargetTexture } from "../../Materials/Textures/renderTargetTexture";
import { MaterialFlags } from "../materialFlags";
import { type UniformBuffer } from "../../Materials/uniformBuffer";
import { type EffectFallbacks } from "../effectFallbacks";
import { type CubeTexture } from "../Textures/cubeTexture";
import { TmpVectors } from "../../Maths/math.vector.pure";
import { type SubMesh } from "../../Meshes/subMesh";
import { MaterialPluginBase } from "../materialPluginBase.pure";
import { Constants } from "../../Engines/constants";
import { MaterialDefines } from "../materialDefines";

import { type Engine } from "../../Engines/engine";
import { type Scene } from "../../scene";
import { type PBRBaseMaterial } from "./pbrBaseMaterial";
import { BindTextureMatrix, PrepareDefinesForMergedUV } from "../materialHelper.functions";

/**
 * @internal
 */
export class MaterialSubSurfaceDefines extends MaterialDefines {
    public SUBSURFACE = false;

    public SS_REFRACTION = false;
    public SS_REFRACTION_USE_INTENSITY_FROM_THICKNESS = false;
    public SS_TRANSLUCENCY = false;
    public SS_TRANSLUCENCY_USE_INTENSITY_FROM_THICKNESS = false;
    public SS_SCATTERING = false;
    public SS_DISPERSION = false;

    public SS_THICKNESSANDMASK_TEXTURE = false;
    public SS_THICKNESSANDMASK_TEXTUREDIRECTUV = 0;
    public SS_HAS_THICKNESS = false;
    public SS_REFRACTIONINTENSITY_TEXTURE = false;
    public SS_REFRACTIONINTENSITY_TEXTUREDIRECTUV = 0;
    public SS_TRANSLUCENCYINTENSITY_TEXTURE = false;
    public SS_TRANSLUCENCYINTENSITY_TEXTUREDIRECTUV = 0;
    public SS_TRANSLUCENCYCOLOR_TEXTURE = false;
    public SS_TRANSLUCENCYCOLOR_TEXTUREDIRECTUV = 0;
    public SS_TRANSLUCENCYCOLOR_TEXTURE_GAMMA = false;

    public SS_REFRACTIONMAP_3D = false;
    public SS_REFRACTIONMAP_OPPOSITEZ = false;
    public SS_LODINREFRACTIONALPHA = false;
    public SS_GAMMAREFRACTION = false;
    public SS_RGBDREFRACTION = false;
    public SS_LINEARSPECULARREFRACTION = false;
    public SS_LINKREFRACTIONTOTRANSPARENCY = false;
    public SS_ALBEDOFORREFRACTIONTINT = false;
    public SS_ALBEDOFORTRANSLUCENCYTINT = false;
    public SS_USE_LOCAL_REFRACTIONMAP_CUBIC = false;
    public SS_USE_THICKNESS_AS_DEPTH = false;

    public SS_USE_GLTF_TEXTURES = false;
    public SS_APPLY_ALBEDO_AFTER_SUBSURFACE = false;
    public SS_TRANSLUCENCY_LEGACY = false;
}

/**
 * Plugin that implements the sub surface component of the PBR material
 */
export class PBRSubSurfaceConfiguration extends MaterialPluginBase {
    /**
     * Default value used for applyAlbedoAfterSubSurface.
     *
     * This property only exists for backward compatibility reasons.
     * Set it to true if your rendering in 8.0+ is different from that in 7 when you use sub-surface properties (transmission, refraction, etc.). Default is false.
     * Note however that the PBR calculation is wrong when this property is set to true, so only use it if you want to mimic the 7.0 behavior.
     */
    public static DEFAULT_APPLY_ALBEDO_AFTERSUBSURFACE = false;

    /**
     * Default value used for legacyTranslucency.
     *
     * This property only exists for backward compatibility reasons.
     * Set it to true if your rendering in 8.0+ is different from that in 7 when you use sub-surface translucency. Default is false.
     */
    public static DEFAULT_LEGACY_TRANSLUCENCY = false;

    declare protected _material: PBRBaseMaterial;

    private _isRefractionEnabled = false;
    /**
     * Defines if the refraction is enabled in the material.
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor isRefractionEnabled = false;

    private _isTranslucencyEnabled = false;
    /**
     * Defines if the translucency is enabled in the material.
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor isTranslucencyEnabled = false;

    private _isDispersionEnabled = false;
    /**
     * Defines if dispersion is enabled in the material.
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor isDispersionEnabled = false;

    private _isScatteringEnabled = false;
    /**
     * Defines if the sub surface scattering is enabled in the material.
     */
    @serialize()
    @expandToProperty("_markScenePrePassDirty")
    public accessor isScatteringEnabled = false;

    @serialize()
    private _scatteringDiffusionProfileIndex = 0;

    /**
     * Diffusion profile for subsurface scattering.
     * Useful for better scattering in the skins or foliages.
     */
    public get scatteringDiffusionProfile(): Nullable<Color3> {
        if (!this._scene.subSurfaceConfiguration) {
            return null;
        }

        return this._scene.subSurfaceConfiguration.ssDiffusionProfileColors[this._scatteringDiffusionProfileIndex];
    }

    public set scatteringDiffusionProfile(c: Nullable<Color3>) {
        if (!this._scene.enableSubSurfaceForPrePass()) {
            // Not supported
            return;
        }

        // addDiffusionProfile automatically checks for doubles
        if (c) {
            this._scatteringDiffusionProfileIndex = this._scene.subSurfaceConfiguration!.addDiffusionProfile(c);
        }
    }

    /**
     * Defines the refraction intensity of the material.
     * The refraction when enabled replaces the Diffuse part of the material.
     * The intensity helps transitioning between diffuse and refraction.
     */
    @serialize()
    public refractionIntensity: number = 1;

    /**
     * Defines the translucency intensity of the material.
     * When translucency has been enabled, this defines how much of the "translucency"
     * is added to the diffuse part of the material.
     */
    @serialize()
    public translucencyIntensity: number = 1;

    private _useAlbedoToTintRefraction = false;
    /**
     * When enabled, transparent surfaces will be tinted with the albedo colour (independent of thickness)
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor useAlbedoToTintRefraction: boolean = false;

    private _useAlbedoToTintTranslucency = false;
    /**
     * When enabled, translucent surfaces will be tinted with the albedo colour (independent of thickness)
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor useAlbedoToTintTranslucency: boolean = false;

    private _thicknessTexture: Nullable<BaseTexture> = null;
    /**
     * Stores the average thickness of a mesh in a texture (The texture is holding the values linearly).
     * The red (or green if useGltfStyleTextures=true) channel of the texture should contain the thickness remapped between 0 and 1.
     * 0 would mean minimumThickness
     * 1 would mean maximumThickness
     * The other channels might be use as a mask to vary the different effects intensity.
     */
    @serializeAsTexture()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor thicknessTexture: Nullable<BaseTexture> = null;

    private _refractionTexture: Nullable<BaseTexture> = null;
    /**
     * Defines the texture to use for refraction.
     */
    @serializeAsTexture()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor refractionTexture: Nullable<BaseTexture> = null;

    /** @internal */
    public _indexOfRefraction = 1.5;
    /**
     * Index of refraction of the material base layer.
     * https://en.wikipedia.org/wiki/List_of_refractive_indices
     *
     * This does not only impact refraction but also the Base F0 of Dielectric Materials.
     *
     * From dielectric fresnel rules: F0 = square((iorT - iorI) / (iorT + iorI))
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor indexOfRefraction = 1.5;

    @serialize()
    private _volumeIndexOfRefraction = -1.0;

    /**
     * Index of refraction of the material's volume.
     * https://en.wikipedia.org/wiki/List_of_refractive_indices
     *
     * This ONLY impacts refraction. If not provided or given a non-valid value,
     * the volume will use the same IOR as the surface.
     */
    public get volumeIndexOfRefraction(): number {
        if (this._volumeIndexOfRefraction >= 1.0) {
            return this._volumeIndexOfRefraction;
        }
        return this._indexOfRefraction;
    }
    public set volumeIndexOfRefraction(value: number) {
        const volumeIndexOfRefraction = value >= 1.0 ? value : -1.0;
        if (this._volumeIndexOfRefraction === volumeIndexOfRefraction) {
            return;
        }
        this._volumeIndexOfRefraction = volumeIndexOfRefraction;
        this._markAllSubMeshesAsTexturesDirty();
    }

    private _invertRefractionY = false;
    /**
     * Controls if refraction needs to be inverted on Y. This could be useful for procedural texture.
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor invertRefractionY = false;

    /** @internal */
    public _linkRefractionWithTransparency = false;
    /**
     * This parameters will make the material used its opacity to control how much it is refracting against not.
     * Materials half opaque for instance using refraction could benefit from this control.
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor linkRefractionWithTransparency = false;

    /**
     * Defines the minimum thickness stored in the thickness map.
     * If no thickness map is defined, this value will be used to simulate thickness.
     */
    @serialize()
    public minimumThickness: number = 0;

    /**
     * Defines the maximum thickness stored in the thickness map.
     */
    @serialize()
    public maximumThickness: number = 1;

    /**
     * Defines that the thickness should be used as a measure of the depth volume.
     */
    @serialize()
    public useThicknessAsDepth = false;

    /**
     * Defines the volume tint of the material.
     * This is used for both translucency and scattering.
     */
    @serializeAsColor3()
    public tintColor = Color3.White();

    /**
     * Defines the distance at which the tint color should be found in the media.
     * This is used for refraction only.
     */
    @serialize()
    public tintColorAtDistance = 1;

    /**
     * Defines the Abbe number for the volume.
     */
    @serialize()
    public dispersion = 0;

    /**
     * Defines how far each channel transmit through the media.
     * It is defined as a color to simplify it selection.
     */
    @serializeAsColor3()
    public diffusionDistance = Color3.White();

    private _useMaskFromThicknessTexture = false;
    /**
     * Stores the intensity of the different subsurface effects in the thickness texture.
     * Note that if refractionIntensityTexture and/or translucencyIntensityTexture is provided it takes precedence over thicknessTexture + useMaskFromThicknessTexture
     * * the green (red if useGltfStyleTextures = true) channel is the refraction intensity.
     * * the blue (alpha if useGltfStyleTextures = true) channel is the translucency intensity.
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor useMaskFromThicknessTexture: boolean = false;

    private _refractionIntensityTexture: Nullable<BaseTexture> = null;
    /**
     * Stores the intensity of the refraction. If provided, it takes precedence over thicknessTexture + useMaskFromThicknessTexture
     * * the green (red if useGltfStyleTextures = true) channel is the refraction intensity.
     */
    @serializeAsTexture()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor refractionIntensityTexture: Nullable<BaseTexture> = null;

    private _translucencyIntensityTexture: Nullable<BaseTexture> = null;
    /**
     * Stores the intensity of the translucency. If provided, it takes precedence over thicknessTexture + useMaskFromThicknessTexture
     * * the blue (alpha if useGltfStyleTextures = true) channel is the translucency intensity.
     */
    @serializeAsTexture()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor translucencyIntensityTexture: Nullable<BaseTexture> = null;

    /**
     * Defines the translucency tint of the material.
     * If not set, the tint color will be used instead.
     */
    @serializeAsColor3()
    public translucencyColor: Nullable<Color3> = null;

    private _translucencyColorTexture: Nullable<BaseTexture> = null;
    /**
     * Defines the translucency tint color of the material as a texture.
     * This is multiplied against the translucency color to add variety and realism to the material.
     * If translucencyColor is not set, the tint color will be used instead.
     */
    @serializeAsTexture()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor translucencyColorTexture: Nullable<BaseTexture> = null;

    private _useGltfStyleTextures = true;
    /**
     * Use channels layout used by glTF:
     * * thicknessTexture: the green (instead of red) channel is the thickness
     * * thicknessTexture/refractionIntensityTexture: the red (instead of green) channel is the refraction intensity
     * * thicknessTexture/translucencyIntensityTexture: the alpha (instead of blue) channel is the translucency intensity
     */
    @serialize()
    @expandToProperty("_markAllSubMeshesAsTexturesDirty")
    public accessor useGltfStyleTextures: boolean = true;

    /**
     * This property only exists for backward compatibility reasons.
     * Set it to true if your rendering in 8.0+ is different from that in 7 when you use sub-surface properties (transmission, refraction, etc.). Default is false.
     * Note however that the PBR calculation is wrong when this property is set to true, so only use it if you want to mimic the 7.0 behavior.
     */
    @serialize()
    public applyAlbedoAfterSubSurface = PBRSubSurfaceConfiguration.DEFAULT_APPLY_ALBEDO_AFTERSUBSURFACE;

    /**
     * This property only exists for backward compatibility reasons.
     * Set it to true if your rendering in 8.0+ is different from that in 7 when you use sub-surface translucency. Default is false.
     */
    @serialize()
    public legacyTranslucency = PBRSubSurfaceConfiguration.DEFAULT_LEGACY_TRANSLUCENCY;

    /**
     * Keeping for backward compatibility... Should not be used anymore. It has been replaced by
     * the property with the correct spelling.
     * @see legacyTranslucency
     */
    public get legacyTransluceny(): boolean {
        return this.legacyTranslucency;
    }
    public set legacyTransluceny(value: boolean) {
        this.legacyTranslucency = value;
    }

    private _scene: Scene;

    /** @internal */
    private _internalMarkAllSubMeshesAsTexturesDirty: () => void;
    private _internalMarkScenePrePassDirty: () => void;

    /** @internal */
    public _markAllSubMeshesAsTexturesDirty(): void {
        this._enable(this._isRefractionEnabled || this._isTranslucencyEnabled || this._isScatteringEnabled);
        this._internalMarkAllSubMeshesAsTexturesDirty();
    }
    /** @internal */
    public _markScenePrePassDirty(): void {
        this._enable(this._isRefractionEnabled || this._isTranslucencyEnabled || this._isScatteringEnabled);
        this._internalMarkAllSubMeshesAsTexturesDirty();
        this._internalMarkScenePrePassDirty();
    }

    /**
     * Gets a boolean indicating that the plugin is compatible with a given shader language.
     * @returns true if the plugin is compatible with the shader language
     */
    public override isCompatible(): boolean {
        return true;
    }

    constructor(material: PBRBaseMaterial, addToPluginList = true) {
        super(material, "PBRSubSurface", 130, new MaterialSubSurfaceDefines(), addToPluginList);

        this._scene = material.getScene();
        this.registerForExtraEvents = true;

        this._internalMarkAllSubMeshesAsTexturesDirty = material._dirtyCallbacks[Constants.MATERIAL_TextureDirtyFlag];
        this._internalMarkScenePrePassDirty = material._dirtyCallbacks[Constants.MATERIAL_PrePassDirtyFlag];
    }

    /**
     * Checks whether the subsurface textures are ready for the sub mesh.
     * @param defines defines the material defines to inspect
     * @param scene defines the scene to use for readiness checks
     * @returns true if subsurface is ready
     */
    public override isReadyForSubMesh(defines: MaterialSubSurfaceDefines, scene: Scene): boolean {
        if (!this._isRefractionEnabled && !this._isTranslucencyEnabled && !this._isScatteringEnabled) {
            return true;
        }

        if (defines._areTexturesDirty) {
            if (scene.texturesEnabled) {
                if (this._thicknessTexture && MaterialFlags.ThicknessTextureEnabled) {
                    if (!this._thicknessTexture.isReadyOrNotBlocking()) {
                        return false;
                    }
                }

                if (this._refractionIntensityTexture && MaterialFlags.RefractionIntensityTextureEnabled) {
                    if (!this._refractionIntensityTexture.isReadyOrNotBlocking()) {
                        return false;
                    }
                }

                if (this._translucencyColorTexture && MaterialFlags.TranslucencyColorTextureEnabled) {
                    if (!this._translucencyColorTexture.isReadyOrNotBlocking()) {
                        return false;
                    }
                }

                if (this._translucencyIntensityTexture && MaterialFlags.TranslucencyIntensityTextureEnabled) {
                    if (!this._translucencyIntensityTexture.isReadyOrNotBlocking()) {
                        return false;
                    }
                }

                const refractionTexture = this._getRefractionTexture(scene);
                if (refractionTexture && MaterialFlags.RefractionTextureEnabled) {
                    if (!refractionTexture.isReadyOrNotBlocking()) {
                        return false;
                    }
                }
            }
        }

        return true;
    }

    /**
     * Updates shader defines for subsurface rendering before attributes are processed.
     * @param defines defines the material defines to update
     * @param scene defines the scene to use for texture checks
     */
    public override prepareDefinesBeforeAttributes(defines: MaterialSubSurfaceDefines, scene: Scene): void {
        if (!this._isRefractionEnabled && !this._isTranslucencyEnabled && !this._isScatteringEnabled) {
            defines.SUBSURFACE = false;
            defines.SS_DISPERSION = false;
            defines.SS_TRANSLUCENCY = false;
            defines.SS_SCATTERING = false;
            defines.SS_REFRACTION = false;
            defines.SS_REFRACTION_USE_INTENSITY_FROM_THICKNESS = false;
            defines.SS_TRANSLUCENCY_USE_INTENSITY_FROM_THICKNESS = false;
            defines.SS_THICKNESSANDMASK_TEXTURE = false;
            defines.SS_THICKNESSANDMASK_TEXTUREDIRECTUV = 0;
            defines.SS_HAS_THICKNESS = false;
            defines.SS_REFRACTIONINTENSITY_TEXTURE = false;
            defines.SS_REFRACTIONINTENSITY_TEXTUREDIRECTUV = 0;
            defines.SS_TRANSLUCENCYINTENSITY_TEXTURE = false;
            defines.SS_TRANSLUCENCYINTENSITY_TEXTUREDIRECTUV = 0;
            defines.SS_REFRACTIONMAP_3D = false;
            defines.SS_REFRACTIONMAP_OPPOSITEZ = false;
            defines.SS_LODINREFRACTIONALPHA = false;
            defines.SS_GAMMAREFRACTION = false;
            defines.SS_RGBDREFRACTION = false;
            defines.SS_LINEARSPECULARREFRACTION = false;
            defines.SS_LINKREFRACTIONTOTRANSPARENCY = false;
            defines.SS_ALBEDOFORREFRACTIONTINT = false;
            defines.SS_ALBEDOFORTRANSLUCENCYTINT = false;
            defines.SS_USE_LOCAL_REFRACTIONMAP_CUBIC = false;
            defines.SS_USE_THICKNESS_AS_DEPTH = false;
            defines.SS_USE_GLTF_TEXTURES = false;
            defines.SS_TRANSLUCENCYCOLOR_TEXTURE = false;
            defines.SS_TRANSLUCENCYCOLOR_TEXTUREDIRECTUV = 0;
            defines.SS_TRANSLUCENCYCOLOR_TEXTURE_GAMMA = false;
            defines.SS_APPLY_ALBEDO_AFTER_SUBSURFACE = false;
            return;
        }

        if (defines._areTexturesDirty) {
            defines.SUBSURFACE = true;

            defines.SS_DISPERSION = this._isDispersionEnabled;
            defines.SS_TRANSLUCENCY = this._isTranslucencyEnabled;
            defines.SS_TRANSLUCENCY_USE_INTENSITY_FROM_THICKNESS = false;
            defines.SS_TRANSLUCENCY_LEGACY = this.legacyTranslucency;
            defines.SS_SCATTERING = this._isScatteringEnabled;
            defines.SS_THICKNESSANDMASK_TEXTURE = false;
            defines.SS_REFRACTIONINTENSITY_TEXTURE = false;
            defines.SS_TRANSLUCENCYINTENSITY_TEXTURE = false;
            defines.SS_HAS_THICKNESS = false;
            defines.SS_USE_GLTF_TEXTURES = false;
            defines.SS_REFRACTION = false;
            defines.SS_REFRACTION_USE_INTENSITY_FROM_THICKNESS = false;
            defines.SS_REFRACTIONMAP_3D = false;
            defines.SS_GAMMAREFRACTION = false;
            defines.SS_RGBDREFRACTION = false;
            defines.SS_LINEARSPECULARREFRACTION = false;
            defines.SS_REFRACTIONMAP_OPPOSITEZ = false;
            defines.SS_LODINREFRACTIONALPHA = false;
            defines.SS_LINKREFRACTIONTOTRANSPARENCY = false;
            defines.SS_ALBEDOFORREFRACTIONTINT = false;
            defines.SS_ALBEDOFORTRANSLUCENCYTINT = false;
            defines.SS_USE_LOCAL_REFRACTIONMAP_CUBIC = false;
            defines.SS_USE_THICKNESS_AS_DEPTH = false;
            defines.SS_TRANSLUCENCYCOLOR_TEXTURE = false;
            defines.SS_APPLY_ALBEDO_AFTER_SUBSURFACE = this.applyAlbedoAfterSubSurface;

            if (defines._areTexturesDirty) {
                if (scene.texturesEnabled) {
                    if (this._thicknessTexture && MaterialFlags.ThicknessTextureEnabled) {
                        PrepareDefinesForMergedUV(this._thicknessTexture, defines, "SS_THICKNESSANDMASK_TEXTURE");
                    }

                    if (this._refractionIntensityTexture && MaterialFlags.RefractionIntensityTextureEnabled) {
                        PrepareDefinesForMergedUV(this._refractionIntensityTexture, defines, "SS_REFRACTIONINTENSITY_TEXTURE");
                    }

                    if (this._translucencyIntensityTexture && MaterialFlags.TranslucencyIntensityTextureEnabled) {
                        PrepareDefinesForMergedUV(this._translucencyIntensityTexture, defines, "SS_TRANSLUCENCYINTENSITY_TEXTURE");
                    }

                    if (this._translucencyColorTexture && MaterialFlags.TranslucencyColorTextureEnabled) {
                        PrepareDefinesForMergedUV(this._translucencyColorTexture, defines, "SS_TRANSLUCENCYCOLOR_TEXTURE");
                        defines.SS_TRANSLUCENCYCOLOR_TEXTURE_GAMMA = this._translucencyColorTexture.gammaSpace;
                    }
                }
            }

            defines.SS_HAS_THICKNESS = this.maximumThickness - this.minimumThickness !== 0.0;
            defines.SS_USE_GLTF_TEXTURES = this._useGltfStyleTextures;
            defines.SS_REFRACTION_USE_INTENSITY_FROM_THICKNESS = this._useMaskFromThicknessTexture && !this._refractionIntensityTexture;
            defines.SS_TRANSLUCENCY_USE_INTENSITY_FROM_THICKNESS = this._useMaskFromThicknessTexture && !this._translucencyIntensityTexture;

            if (this._isRefractionEnabled) {
                if (scene.texturesEnabled) {
                    const refractionTexture = this._getRefractionTexture(scene);
                    if (refractionTexture && MaterialFlags.RefractionTextureEnabled) {
                        defines.SS_REFRACTION = true;
                        defines.SS_REFRACTIONMAP_3D = refractionTexture.isCube;
                        defines.SS_GAMMAREFRACTION = refractionTexture.gammaSpace;
                        defines.SS_RGBDREFRACTION = refractionTexture.isRGBD;
                        defines.SS_LINEARSPECULARREFRACTION = refractionTexture.linearSpecularLOD;
                        defines.SS_REFRACTIONMAP_OPPOSITEZ = this._scene.useRightHandedSystem && refractionTexture.isCube ? !refractionTexture.invertZ : refractionTexture.invertZ;
                        defines.SS_LODINREFRACTIONALPHA = refractionTexture.lodLevelInAlpha;
                        defines.SS_LINKREFRACTIONTOTRANSPARENCY = this._linkRefractionWithTransparency;
                        defines.SS_ALBEDOFORREFRACTIONTINT = this._useAlbedoToTintRefraction;
                        defines.SS_USE_LOCAL_REFRACTIONMAP_CUBIC = refractionTexture.isCube && (<any>refractionTexture).boundingBoxSize;
                        defines.SS_USE_THICKNESS_AS_DEPTH = this.useThicknessAsDepth;
                    }
                }
            }

            if (this._isTranslucencyEnabled) {
                defines.SS_ALBEDOFORTRANSLUCENCYTINT = this._useAlbedoToTintTranslucency;
            }
        }
    }

    /**
     * Binds the material data (this function is called even if mustRebind() returns false)
     * @param uniformBuffer defines the Uniform buffer to fill in.
     * @param scene defines the scene the material belongs to.
     * @param engine defines the engine the material belongs to.
     * @param subMesh the submesh to bind data for
     */
    public override hardBindForSubMesh(uniformBuffer: UniformBuffer, scene: Scene, engine: Engine, subMesh: SubMesh): void {
        if (!this._isRefractionEnabled && !this._isTranslucencyEnabled && !this._isScatteringEnabled) {
            return;
        }

        // If min/max thickness is 0, avoid decompising to determine the scaled thickness (it's always zero).
        if (this.maximumThickness === 0.0 && this.minimumThickness === 0.0) {
            uniformBuffer.updateFloat2("vThicknessParam", 0, 0);
        } else {
            subMesh.getRenderingMesh().getWorldMatrix().decompose(TmpVectors.Vector3[0]);
            const thicknessScale = Math.max(Math.abs(TmpVectors.Vector3[0].x), Math.abs(TmpVectors.Vector3[0].y), Math.abs(TmpVectors.Vector3[0].z));
            uniformBuffer.updateFloat2("vThicknessParam", this.minimumThickness * thicknessScale, (this.maximumThickness - this.minimumThickness) * thicknessScale);
        }
    }

    /**
     * Binds subsurface data for a sub mesh.
     * @param uniformBuffer defines the uniform buffer to update
     * @param scene defines the scene to use for texture binding
     * @param engine defines the engine used for binding
     * @param subMesh defines the sub mesh being rendered
     */
    public override bindForSubMesh(uniformBuffer: UniformBuffer, scene: Scene, engine: Engine, subMesh: SubMesh): void {
        if (!this._isRefractionEnabled && !this._isTranslucencyEnabled && !this._isScatteringEnabled) {
            return;
        }

        const defines = subMesh.materialDefines as unknown as MaterialSubSurfaceDefines;

        const isFrozen = this._material.isFrozen;
        const realTimeFiltering = this._material.realTimeFiltering;
        const lodBasedMicrosurface = defines.LODBASEDMICROSFURACE;

        const refractionTexture = this._getRefractionTexture(scene);

        if (!uniformBuffer.useUbo || !isFrozen || !uniformBuffer.isSync) {
            if (this._thicknessTexture && MaterialFlags.ThicknessTextureEnabled) {
                uniformBuffer.updateFloat2("vThicknessInfos", this._thicknessTexture.coordinatesIndex, this._thicknessTexture.level);
                BindTextureMatrix(this._thicknessTexture, uniformBuffer, "thickness");
            }

            if (this._refractionIntensityTexture && MaterialFlags.RefractionIntensityTextureEnabled && defines.SS_REFRACTIONINTENSITY_TEXTURE) {
                uniformBuffer.updateFloat2("vRefractionIntensityInfos", this._refractionIntensityTexture.coordinatesIndex, this._refractionIntensityTexture.level);
                BindTextureMatrix(this._refractionIntensityTexture, uniformBuffer, "refractionIntensity");
            }

            if (this._translucencyColorTexture && MaterialFlags.TranslucencyColorTextureEnabled && defines.SS_TRANSLUCENCYCOLOR_TEXTURE) {
                uniformBuffer.updateFloat2("vTranslucencyColorInfos", this._translucencyColorTexture.coordinatesIndex, this._translucencyColorTexture.level);
                BindTextureMatrix(this._translucencyColorTexture, uniformBuffer, "translucencyColor");
            }

            if (this._translucencyIntensityTexture && MaterialFlags.TranslucencyIntensityTextureEnabled && defines.SS_TRANSLUCENCYINTENSITY_TEXTURE) {
                uniformBuffer.updateFloat2("vTranslucencyIntensityInfos", this._translucencyIntensityTexture.coordinatesIndex, this._translucencyIntensityTexture.level);
                BindTextureMatrix(this._translucencyIntensityTexture, uniformBuffer, "translucencyIntensity");
            }

            if (refractionTexture && MaterialFlags.RefractionTextureEnabled) {
                uniformBuffer.updateMatrix("refractionMatrix", refractionTexture.getRefractionTextureMatrix());

                let depth = 1.0;
                if (!refractionTexture.isCube) {
                    if ((<any>refractionTexture).depth) {
                        depth = (<any>refractionTexture).depth;
                    }
                }

                const width = refractionTexture.getSize().width;
                const refractionIor = this.volumeIndexOfRefraction;
                uniformBuffer.updateFloat4("vRefractionInfos", refractionTexture.level, 1 / refractionIor, depth, this._invertRefractionY ? -1 : 1);
                uniformBuffer.updateFloat4(
                    "vRefractionMicrosurfaceInfos",
                    width,
                    refractionTexture.lodGenerationScale,
                    refractionTexture.lodGenerationOffset,
                    1.0 / this.indexOfRefraction
                );

                if (realTimeFiltering) {
                    uniformBuffer.updateFloat2("vRefractionFilteringInfo", width, Math.log2(width));
                }

                if ((<any>refractionTexture).boundingBoxSize) {
                    const cubeTexture = <CubeTexture>refractionTexture;

                    uniformBuffer.updateVector3("vRefractionPosition", cubeTexture.boundingBoxPosition);
                    uniformBuffer.updateVector3("vRefractionSize", cubeTexture.boundingBoxSize);
                }
            }

            if (this._isScatteringEnabled) {
                uniformBuffer.updateFloat("scatteringDiffusionProfile", this._scatteringDiffusionProfileIndex);
            }
            uniformBuffer.updateColor3("vDiffusionDistance", this.diffusionDistance);

            uniformBuffer.updateFloat4("vTintColor", this.tintColor.r, this.tintColor.g, this.tintColor.b, Math.max(0.00001, this.tintColorAtDistance));
            uniformBuffer.updateColor4("vTranslucencyColor", this.translucencyColor ?? this.tintColor, 0);

            uniformBuffer.updateFloat3("vSubSurfaceIntensity", this.refractionIntensity, this.translucencyIntensity, 0);

            uniformBuffer.updateFloat("dispersion", this.dispersion);
        }

        // Textures
        if (scene.texturesEnabled) {
            if (this._thicknessTexture && MaterialFlags.ThicknessTextureEnabled) {
                uniformBuffer.setTexture("thicknessSampler", this._thicknessTexture);
            }

            if (this._refractionIntensityTexture && MaterialFlags.RefractionIntensityTextureEnabled && defines.SS_REFRACTIONINTENSITY_TEXTURE) {
                uniformBuffer.setTexture("refractionIntensitySampler", this._refractionIntensityTexture);
            }

            if (this._translucencyIntensityTexture && MaterialFlags.TranslucencyIntensityTextureEnabled && defines.SS_TRANSLUCENCYINTENSITY_TEXTURE) {
                uniformBuffer.setTexture("translucencyIntensitySampler", this._translucencyIntensityTexture);
            }

            if (this._translucencyColorTexture && MaterialFlags.TranslucencyColorTextureEnabled && defines.SS_TRANSLUCENCYCOLOR_TEXTURE) {
                uniformBuffer.setTexture("translucencyColorSampler", this._translucencyColorTexture);
            }

            if (refractionTexture && MaterialFlags.RefractionTextureEnabled) {
                if (lodBasedMicrosurface) {
                    uniformBuffer.setTexture("refractionSampler", refractionTexture);
                } else {
                    uniformBuffer.setTexture("refractionSampler", refractionTexture._lodTextureMid || refractionTexture);
                    uniformBuffer.setTexture("refractionSamplerLow", refractionTexture._lodTextureLow || refractionTexture);
                    uniformBuffer.setTexture("refractionSamplerHigh", refractionTexture._lodTextureHigh || refractionTexture);
                }
            }
        }
    }

    /**
     * Returns the texture used for refraction or null if none is used.
     * @param scene defines the scene the material belongs to.
     * @returns - Refraction texture if present.  If no refraction texture and refraction
     * is linked with transparency, returns environment texture.  Otherwise, returns null.
     */
    private _getRefractionTexture(scene: Scene): Nullable<BaseTexture> {
        if (this._refractionTexture) {
            return this._refractionTexture;
        }

        if (this._isRefractionEnabled) {
            return scene.environmentTexture;
        }

        return null;
    }

    /**
     * Returns true if alpha blending should be disabled.
     */
    public get disableAlphaBlending(): boolean {
        return this._isRefractionEnabled && this._linkRefractionWithTransparency;
    }

    /**
     * Fills the list of render target textures.
     * @param renderTargets the list of render targets to update
     */
    public override fillRenderTargetTextures(renderTargets: SmartArray<RenderTargetTexture>): void {
        if (MaterialFlags.RefractionTextureEnabled && this._refractionTexture && this._refractionTexture.isRenderTarget) {
            renderTargets.push(<RenderTargetTexture>this._refractionTexture);
        }
    }

    /**
     * Checks whether subsurface rendering uses a texture.
     * @param texture defines the texture to check
     * @returns true if the texture is used by subsurface rendering
     */
    public override hasTexture(texture: BaseTexture): boolean {
        if (this._thicknessTexture === texture) {
            return true;
        }

        if (this._refractionTexture === texture) {
            return true;
        }

        if (this._refractionIntensityTexture === texture) {
            return true;
        }

        if (this._translucencyIntensityTexture === texture) {
            return true;
        }

        if (this._translucencyColorTexture === texture) {
            return true;
        }

        return false;
    }

    public override hasRenderTargetTextures(): boolean {
        if (MaterialFlags.RefractionTextureEnabled && this._refractionTexture && this._refractionTexture.isRenderTarget) {
            return true;
        }

        return false;
    }

    /**
     * Adds the active subsurface textures.
     * @param activeTextures defines the list of active textures to update
     */
    public override getActiveTextures(activeTextures: BaseTexture[]): void {
        if (this._thicknessTexture) {
            activeTextures.push(this._thicknessTexture);
        }

        if (this._refractionTexture) {
            activeTextures.push(this._refractionTexture);
        }

        if (this._refractionIntensityTexture) {
            activeTextures.push(this._refractionIntensityTexture);
        }

        if (this._translucencyColorTexture) {
            activeTextures.push(this._translucencyColorTexture);
        }

        if (this._translucencyIntensityTexture) {
            activeTextures.push(this._translucencyIntensityTexture);
        }
    }

    /**
     * Adds the animatable subsurface textures.
     * @param animatables defines the list of animatables to update
     */
    public override getAnimatables(animatables: IAnimatable[]): void {
        if (this._thicknessTexture && this._thicknessTexture.animations && this._thicknessTexture.animations.length > 0) {
            animatables.push(this._thicknessTexture);
        }

        if (this._refractionTexture && this._refractionTexture.animations && this._refractionTexture.animations.length > 0) {
            animatables.push(this._refractionTexture);
        }

        if (this._refractionIntensityTexture && this._refractionIntensityTexture.animations && this._refractionIntensityTexture.animations.length > 0) {
            animatables.push(this._refractionIntensityTexture);
        }

        if (this._translucencyColorTexture && this._translucencyColorTexture.animations && this._translucencyColorTexture.animations.length > 0) {
            animatables.push(this._translucencyColorTexture);
        }

        if (this._translucencyIntensityTexture && this._translucencyIntensityTexture.animations && this._translucencyIntensityTexture.animations.length > 0) {
            animatables.push(this._translucencyIntensityTexture);
        }
    }

    /**
     * Disposes the subsurface textures.
     * @param forceDisposeTextures defines whether to dispose the textures
     */
    public override dispose(forceDisposeTextures?: boolean): void {
        if (forceDisposeTextures) {
            if (this._thicknessTexture) {
                this._thicknessTexture.dispose();
            }

            if (this._refractionTexture) {
                this._refractionTexture.dispose();
            }

            if (this._refractionIntensityTexture) {
                this._refractionIntensityTexture.dispose();
            }

            if (this._translucencyColorTexture) {
                this._translucencyColorTexture.dispose();
            }

            if (this._translucencyIntensityTexture) {
                this._translucencyIntensityTexture.dispose();
            }
        }
    }

    public override getClassName(): string {
        return "PBRSubSurfaceConfiguration";
    }

    public override addFallbacks(defines: MaterialSubSurfaceDefines, fallbacks: EffectFallbacks, currentRank: number): number {
        if (defines.SS_SCATTERING) {
            fallbacks.addFallback(currentRank++, "SS_SCATTERING");
        }
        if (defines.SS_TRANSLUCENCY) {
            fallbacks.addFallback(currentRank++, "SS_TRANSLUCENCY");
        }
        return currentRank;
    }

    /**
     * Adds the subsurface sampler names.
     * @param samplers defines the list of sampler names to update
     */
    public override getSamplers(samplers: string[]): void {
        samplers.push(
            "thicknessSampler",
            "refractionIntensitySampler",
            "translucencyIntensitySampler",
            "refractionSampler",
            "refractionSamplerLow",
            "refractionSamplerHigh",
            "translucencyColorSampler"
        );
    }

    public override getUniforms(): { ubo?: Array<{ name: string; size: number; type: string }>; vertex?: string; fragment?: string } {
        return {
            ubo: [
                { name: "vRefractionMicrosurfaceInfos", size: 4, type: "vec4" },
                { name: "vRefractionFilteringInfo", size: 2, type: "vec2" },
                { name: "vTranslucencyIntensityInfos", size: 2, type: "vec2" },
                { name: "vRefractionInfos", size: 4, type: "vec4" },
                { name: "refractionMatrix", size: 16, type: "mat4" },
                { name: "vThicknessInfos", size: 2, type: "vec2" },
                { name: "vRefractionIntensityInfos", size: 2, type: "vec2" },
                { name: "thicknessMatrix", size: 16, type: "mat4" },
                { name: "refractionIntensityMatrix", size: 16, type: "mat4" },
                { name: "translucencyIntensityMatrix", size: 16, type: "mat4" },
                { name: "vThicknessParam", size: 2, type: "vec2" },
                { name: "vDiffusionDistance", size: 3, type: "vec3" },
                { name: "vTintColor", size: 4, type: "vec4" },
                { name: "vSubSurfaceIntensity", size: 3, type: "vec3" },
                { name: "vRefractionPosition", size: 3, type: "vec3" },
                { name: "vRefractionSize", size: 3, type: "vec3" },
                { name: "scatteringDiffusionProfile", size: 1, type: "float" },
                { name: "dispersion", size: 1, type: "float" },

                { name: "vTranslucencyColor", size: 4, type: "vec4" },
                { name: "vTranslucencyColorInfos", size: 2, type: "vec2" },
                { name: "translucencyColorMatrix", size: 16, type: "mat4" },
            ],
        };
    }
}


```
