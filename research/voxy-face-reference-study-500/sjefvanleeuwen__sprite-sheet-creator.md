# Facial rendering source scan: sjefvanleeuwen/sprite-sheet-creator

Repository: https://github.com/sjefvanleeuwen/sprite-sheet-creator

Pinned source revision: eceafa51596850082f2e377ab4c37d8ef875c9c5

Discovery topic: character creator 3D

Description: A comprehensive tool for viewing 3D character models, creating sprite sheets, and exporting animations.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: js/fbxViewer.js

{
  "path": "js/fbxViewer.js",
  "blob_sha": "93a976b1257de005d0f395bd16e83033efd82443",
  "sha256": "77fe6d0cc46ec3ad98f5ef140c5161a60c1c488ac9edadfa0ee94b9f88b12466",
  "lines": 1143,
  "bytes": 44156,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 28,
    "face": 3
  },
  "url": "https://github.com/sjefvanleeuwen/sprite-sheet-creator/blob/eceafa51596850082f2e377ab4c37d8ef875c9c5/js/fbxViewer.js",
  "function_signature_candidates": []
}

```text

document.addEventListener('DOMContentLoaded', async function() {
    // Get references to HTML elements
    const canvas = document.getElementById('renderCanvas');
    const animationList = document.getElementById('animationList');
    const playBtn = document.getElementById('playBtn');
    const pauseBtn = document.getElementById('pauseBtn');
    const statusElement = document.getElementById('statusMessage');
    const recordBtn = document.getElementById('recordBtn');
    const framesInput = document.getElementById('framesInput');
    const columnsInput = document.getElementById('columnsInput');
    const cellWidthInput = document.getElementById('cellWidthInput');
    const cellHeightInput = document.getElementById('cellHeightInput');
    
    // Initialize BabylonJS
    const engine = new BABYLON.Engine(canvas, true);
    let scene, camera, character, animationGroups = [];
    let currentAnimationGroup = null;
    
    // Sprite sheet recording variables
    let isRecording = false;
    let recordedFrames = [];
    let frameCount = 0;
    let totalFramesToCapture = 0;
    const captureCanvas = document.createElement('canvas');
    const captureContext = captureCanvas.getContext('2d');
    
    // Fix the scroll wheel to zoom functionality with a direct implementation
    let zoomFactor = 5; // Initial orthographic zoom level
    
    // Display status messages to the user
    function showStatus(message, isError = false) {
        statusElement.textContent = message;
        statusElement.className = isError ? 'error' : 'info';
        console.log(isError ? 'ERROR: ' : 'INFO: ', message);
    }
    
    // Add camera adjustment variables
    let cameraOffsetX = 0;
    let cameraOffsetY = 0;
    const cameraAdjustStep = 0.25; // How much to move the camera per adjustment
    
    /**
     * Updates orthographic camera settings based on zoom factor
     * This keeps the orthographic view consistent with zoom level
     */
    function updateOrthoCamera() {
        if (!camera) return;
        
        // Force a 1:1 square aspect ratio regardless of canvas dimensions
        camera.mode = BABYLON.Camera.ORTHOGRAPHIC_CAMERA;
        
        // Use the same value for both horizontal and vertical dimensions
        // This ensures a perfect square view
        camera.orthoTop = zoomFactor;
        camera.orthoBottom = -zoomFactor;
        camera.orthoLeft = -zoomFactor; // Same as vertical for 1:1 ratio
        camera.orthoRight = zoomFactor;  // Same as vertical for 1:1 ratio
        
        // Apply camera position offsets by moving the target
        if (camera.target) {
            // Create a new target vector with the offsets applied
            const newTarget = new BABYLON.Vector3(
                cameraOffsetX,
                1 + cameraOffsetY, // Add 1 to keep at approximate character height
                0
            );
            camera.setTarget(newTarget);
        }
    }
    
    // Add these functions for camera adjustment
    function adjustCameraPosition(directionX, directionY) {
        cameraOffsetX += directionX * cameraAdjustStep;
        cameraOffsetY += directionY * cameraAdjustStep;
        updateOrthoCamera();
        
        // Update status to show camera position
        showStatus(`Camera offset: X=${cameraOffsetX.toFixed(2)}, Y=${cameraOffsetY.toFixed(2)}`);
    }
    
    function resetCameraPosition() {
        cameraOffsetX = 0;
        cameraOffsetY = 0;
        updateOrthoCamera();
        showStatus("Camera position reset");
    }
    
    // Create the scene
    const createScene = async function() {
        const scene = new BABYLON.Scene(engine);
        window.scene = scene; // Make scene globally accessible
        
        // Set transparent background by setting alpha to 0
        scene.clearColor = new BABYLON.Color4(0, 0, 0, 0);
        
        // Enable alpha blending (required for transparent backgrounds)
        engine.setHardwareScalingLevel(1);
        engine.alpha = true;
        engine.alphaMode = BABYLON.Engine.ALPHA_COMBINE;
        
        // Add a light
        const light = new BABYLON.HemisphericLight('light', new BABYLON.Vector3(0, 1, 0), scene);
        light.intensity = 0.7;
        
        // Add directional light for better lighting from the side
        const dirLight = new BABYLON.DirectionalLight('dirLight', new BABYLON.Vector3(-1, -0.5, 0), scene);
        dirLight.position = new BABYLON.Vector3(10, 10, 0);
        dirLight.intensity = 0.8;
        
        // Create a 2D side-view camera with elevated position
        // Use Math.PI/2 for the alpha value to position camera on the side
        camera = new BABYLON.ArcRotateCamera(
            'camera', 
            Math.PI/2,    // Alpha: side view
            Math.PI/3,    // Beta: elevated angle
            15,           // Initial radius
            new BABYLON.Vector3(0, 1, 0),  // Target point
            scene
        );
        camera.mode = BABYLON.Camera.ORTHOGRAPHIC_CAMERA;
        
        // Enable wheel (remove any existing wheel limitations)
        camera.useFramingBehavior = false;
        camera.panningSensibility = 100; // Lower = more sensitive panning
        
        // Set initial orthographic view size
        updateOrthoCamera();
        
        // Add this simple direct event listener for wheel scrolling
        canvas.addEventListener("wheel", function(event) {
            event.preventDefault();
            
            // Adjust zoom factor based on wheel direction
            if (event.deltaY < 0) {
                // Zoom in
                zoomFactor *= 0.9; 
            } else {
                // Zoom out
                zoomFactor *= 1.1;
            }
            
            // Clamp zoom limits
            zoomFactor = Math.max(0.001, Math.min(50, zoomFactor));
            
            // Apply the new zoom level
            updateOrthoCamera();
            
            // Display the zoom level for debugging
            showStatus(`Zoom level: ${zoomFactor.toFixed(1)}x`);
        });
        
        // Remove the rigid locking and instead allow zooming:
        camera.lowerRadiusLimit = 5;    // Minimum zoom-in distance
        camera.upperRadiusLimit = 50;   // Maximum zoom-out distance
        
        // Set orthographic scale based on screen size
        const aspectRatio = engine.getAspectRatio(camera);
        const orthoSize = 5;
        camera.orthoTop = orthoSize;
        camera.orthoBottom = -orthoSize;
        camera.orthoLeft = -orthoSize * aspectRatio;
        camera.orthoRight = orthoSize * aspectRatio;
        
        // Allow limited panning for better viewing
        camera.panningSensibility = 50;
        camera.panningAxis = new BABYLON.Vector3(1, 1, 0); // Only allow panning in X and Y
        
        // Create a simple ground
        // const ground = BABYLON.MeshBuilder.CreateGround('ground', {width: 50, height: 50}, scene);
        // const groundMaterial = new BABYLON.StandardMaterial('groundMat', scene);
        // groundMaterial.diffuseColor = new BABYLON.Color3(0.2, 0.2, 0.2);
        // groundMaterial.specularColor = new BABYLON.Color3(0, 0, 0);
        // groundMaterial.alpha = 0; // Make the ground transparent
        // ground.material = groundMaterial;
        // ground.position.y = 0;
        
        // Try to load the character model
      //  await loadCharacterModel();
        
        // Create camera adjustment UI
        createCameraControls();
        
        // Dispatch event when scene is ready
        window.dispatchEvent(new CustomEvent('sceneReady'));
        
        return scene;
    };
    
    // Add this after the createScene function but before the event listeners

    function replaceCharacter(container) {
        if (!scene) return;

        // Clear previous character completely
        if (window.currentCharacter) {
            // Stop and dispose all animations
            scene.animationGroups.slice().forEach(group => {
                group.stop();
                group.dispose();
            });
            
            // Remove all meshes related to current character
            scene.meshes.slice().forEach(mesh => {
                if (mesh !== scene.ground) { // Keep the ground
                    mesh.dispose();
                }
            });

            // Clear any skeletons
            scene.skeletons.slice().forEach(skeleton => {
                skeleton.dispose();
            });

            window.currentCharacter = null;
            window.currentAnimationGroup = null;
        }

        // Add new meshes and animations
        container.addAllToScene();
        window.currentCharacter = container.meshes[0];
        window.animationGroups = container.animationGroups;
        
        // Position the new character
        window.currentCharacter.position = new BABYLON.Vector3(0, 0, 0);
        window.currentCharacter.rotation = new BABYLON.Vector3(0, Math.PI/2, 0);
        window.currentCharacter.scaling = new BABYLON.Vector3(1, 1, 1);

        // Update animation list
        const animationList = document.getElementById('animationList');
        animationList.innerHTML = '';
        container.animationGroups.forEach((group, index) => {
            const option = document.createElement('option');
            option.value = index;
            option.text = group.name;
            animationList.appendChild(option);
        });

        // Play first animation if available
        if (container.animationGroups.length > 0) {
            window.currentAnimationGroup = container.animationGroups[0];
            window.currentAnimationGroup.start(true);
        }

        // Update animation list and play first animation
        if (container.animationGroups.length > 0) {
            // Store animation groups both locally and globally
            animationGroups = container.animationGroups;
            window.animationGroups = animationGroups;
            
            // Update the animation list UI
            populateAnimationList();
            
            // Play the first animation
            playAnimation(0);
        }

        return window.currentCharacter;
    }

    // Make the function globally accessible
    window.replaceCharacter = replaceCharacter;

    // Function to try loading the character model with fallbacks
    async function loadCharacterModel() {
        showStatus("Loading character model...");
        
        // Check if file exists first (using fetch to check)
        try {
            // Try GLB first (changed order - GLB first, then FBX)
            await tryLoadGLB();
        }
        catch (glbError) {
            console.error("Error loading GLB:", glbError);
            
            try {
                // Try FBX as fallback
                await tryLoadFBX();
            }
            catch (fbxError) {
                console.error("Error loading FBX:", fbxError);
                
                // Final fallback - create a simple character
                createFallbackCharacter();
                showStatus("Using fallback character. Please place a valid model in the models folder.", true);
            }
        }
    }
    
    // Try loading GLB file
    async function tryLoadGLB() {
        // Check if file exists first
        const response = await fetch('models/character.glb', { method: 'HEAD' });
        if (!response.ok) {
            throw new Error("GLB file not found");
        }
        
        showStatus("Loading GLB model...");
        
        const result = await BABYLON.SceneLoader.ImportMeshAsync('', 'models/', 'character.glb', scene);
        
        // Process the loaded model
        processLoadedModel(result);
        showStatus("GLB model loaded successfully!");
    }
    
    // Try loading FBX file
    async function tryLoadFBX() {
        // Check if file exists first
        const response = await fetch('models/character.fbx', { method: 'HEAD' });
        if (!response.ok) {
            throw new Error("FBX file not found");
        }
        
        showStatus("Loading FBX model...");
        
        const result = await BABYLON.SceneLoader.ImportMeshAsync('', 'models/', 'character.fbx', scene);
        
        // Process the loaded model
        processLoadedModel(result);
        showStatus("FBX model loaded successfully!");
    }
    
    // Create a fallback character if model loading fails
    function createFallbackCharacter() {
        // Create a simple character made of shapes
        const body = BABYLON.MeshBuilder.CreateBox("body", {height: 2, width: 0.75, depth: 0.5}, scene);
        body.position.y = 1;
        
        const head = BABYLON.MeshBuilder.CreateSphere("head", {diameter: 0.7}, scene);
        head.position.y = 2.4;
        head.parent = body;
        
        const leftArm = BABYLON.MeshBuilder.CreateBox("leftArm", {height: 1.2, width: 0.25, depth: 0.25}, scene);
        leftArm.position.x = -0.5;
        leftArm.position.y = 1;
        leftArm.parent = body;
        
        const rightArm = BABYLON.MeshBuilder.CreateBox("rightArm", {height: 1.2, width: 0.25, depth: 0.25}, scene);
        rightArm.position.x = 0.5;
        rightArm.position.y = 1;
        rightArm.parent = body;
        
        const leftLeg = BABYLON.MeshBuilder.CreateBox("leftLeg", {height: 1.5, width: 0.3, depth: 0.3}, scene);
        leftLeg.position.x = -0.2;
        leftLeg.position.y = -0.75;
        leftLeg.parent = body;
        
        const rightLeg = BABYLON.MeshBuilder.CreateBox("rightLeg", {height: 1.5, width: 0.3, depth: 0.3}, scene);
        rightLeg.position.x = 0.2;
        rightLeg.position.y = -0.75;
        rightLeg.parent = body;
        
        // Create material for the character
        const characterMaterial = new BABYLON.StandardMaterial("charMaterial", scene);
        characterMaterial.diffuseColor = new BABYLON.Color3(0.5, 0.5, 1.0);
        
        // Apply material to all parts
        body.material = characterMaterial;
        head.material = characterMaterial;
        leftArm.material = characterMaterial;
        rightArm.material = characterMaterial;
        leftLeg.material = characterMaterial;
        rightLeg.material = characterMaterial;
        
        // Create a simple animation
        const walkAnim = new BABYLON.Animation(
            "walkAnimation", 
            "rotation.x", 
            30,
            BABYLON.Animation.ANIMATIONTYPE_FLOAT,
            BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
        );
        
        const walkKeys = [];
        walkKeys.push({ frame: 0, value: -Math.PI / 10 });
        walkKeys.push({ frame: 15, value: Math.PI / 10 });
        walkKeys.push({ frame: 30, value: -Math.PI / 10 });
        walkAnim.setKeys(walkKeys);
        
        const armAnim = new BABYLON.Animation(
            "armAnimation", 
            "rotation.x", 
            30,
            BABYLON.Animation.ANIMATIONTYPE_FLOAT,
            BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
        );
        
        const armKeys = [];
        armKeys.push({ frame: 0, value: Math.PI / 4 });
        armKeys.push({ frame: 15, value: -Math.PI / 4 });
        armKeys.push({ frame: 30, value: Math.PI / 4 });
        armAnim.setKeys(armKeys);
        
        // Create animation groups
        const walkAnimGroup = new BABYLON.AnimationGroup("Walk");
        walkAnimGroup.addTargetedAnimation(walkAnim, leftLeg);
        walkAnimGroup.addTargetedAnimation(new BABYLON.Animation(walkAnim), rightLeg);
        walkAnimGroup.addTargetedAnimation(armAnim, leftArm);
        walkAnimGroup.addTargetedAnimation(armAnim, rightArm);
        
        const idleAnimGroup = new BABYLON.AnimationGroup("Idle");
        const idleAnim = new BABYLON.Animation(
            "idleAnimation", 
            "position.y", 
            30,
            BABYLON.Animation.ANIMATIONTYPE_FLOAT,
            BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
        );
        
        const idleKeys = [];
        idleKeys.push({ frame: 0, value: body.position.y });
        idleKeys.push({ frame: 15, value: body.position.y + 0.05 });
        idleKeys.push({ frame: 30, value: body.position.y });
        idleAnim.setKeys(idleKeys);
        
        idleAnimGroup.addTargetedAnimation(idleAnim, body);
        
        // Store animations and character
        animationGroups = [idleAnimGroup, walkAnimGroup];
        character = body;
        
        // Populate animation list
        populateAnimationList();
        
        // For a side view, adjust the orientation of the fallback character
        body.rotation.y = Math.PI/2; // Rotate to face the side camera
    }
    
    // Process loaded character model
    function processLoadedModel(result) {
        // Get the loaded character (root mesh)
        character = result.meshes[0];
        
        // Center the character
        character.position = new BABYLON.Vector3(0, 0, 0);
        
        // Adjust character rotation to face correctly for side view
        // Rotate 90 degrees so character faces the camera from the side
        character.rotation = new BABYLON.Vector3(0, Math.PI/2, 0);
        
        // Store animation groups both locally and globally
        animationGroups = result.animationGroups;
        window.animationGroups = animationGroups;
        
        // Handle no animations case
        if (animationGroups.length === 0) {
            showStatus("The model has no animations. Creating a simple animation.", true);
            createDefaultAnimation();
        }
        
        // Reset camera position when loading a new model
        cameraOffsetX = 0;
        cameraOffsetY = 0;
        
        // Set the camera to look at the character's upper body
        if (camera) {
            camera.setTarget(new BABYLON.Vector3(0, 1, 0));
        }
        
        // Populate animation dropdown
        populateAnimationList();
        
        // Make sure to play the first animation
        if (animationGroups.length > 0) {
            playAnimation(0);
        }
        
        console.log(`Loaded character with ${animationGroups.length} animations`);
    }
    
    // Create a default animation if none exists
    function createDefaultAnimation() {
        const anim = new BABYLON.Animation(
            "defaultAnim", 
            "position.y", 
            30,
            BABYLON.Animation.ANIMATIONTYPE_FLOAT,
            BABYLON.Animation.ANIMATIONLOOPMODE_CYCLE
        );
        
        const keys = [];
        keys.push({ frame: 0, value: character.position.y });
        keys.push({ frame: 30, value: character.position.y + 0.2 });
        keys.push({ frame: 60, value: character.position.y });
        
        anim.setKeys(keys);
        
        const animGroup = new BABYLON.AnimationGroup("Default");
        animGroup.addTargetedAnimation(anim, character);
        
        animationGroups = [animGroup];
    }
    
    // Populate animation dropdown
    function populateAnimationList() {
        // Clear existing options
        animationList.innerHTML = '';
        
        // Add each animation to the dropdown
        animationGroups.forEach((group, index) => {
            const option = document.createElement('option');
            option.value = index;
            option.text = group.name;
            animationList.appendChild(option);
        });
        
        // Select and play the first animation
        if (animationGroups.length > 0) {
            animationList.value = "0";
            playAnimation(0);
        }
    }
    
    // Play selected animation
    function playAnimation(index) {
        // Stop all animations first
        if (scene) {
            scene.animationGroups.forEach(group => group.stop());
        }
        
        // Play selected animation
        if (index >= 0 && index < animationGroups.length) {
            currentAnimationGroup = animationGroups[index];
            window.currentAnimationGroup = currentAnimationGroup; // Ensure global access
            if (currentAnimationGroup) {
                currentAnimationGroup.start(true); // true for looping
                showStatus(`Playing animation: ${currentAnimationGroup.name}`);
                // Update dropdown selection to match
                animationList.value = index.toString();
            }
        }
    }
    
    // Recording sprite sheet functions
    function startRecording() {
        // Get the currently selected animation
        const selectedIndex = parseInt(animationList.value);
        
        // Ensure we have a valid animation
        if (!animationGroups || animationGroups.length === 0) {
            showStatus("No animations available to record", true);
            return;
        }
        
        // Force play the selected animation if it's not already playing
        if (!currentAnimationGroup || currentAnimationGroup !== animationGroups[selectedIndex]) {
            playAnimation(selectedIndex);
        }
        
        // Double check we have a valid animation now
        if (!currentAnimationGroup) {
            showStatus("Failed to start animation for recording", true);
            return;
        }
        
        // Get parameters from inputs with fallback values
        totalFramesToCapture = Math.max(1, parseInt(framesInput.value) || 16);
        
        // Reset recording state
        recordedFrames = [];
        frameCount = 0;
        isRecording = true;
        
        // Restart the animation from the beginning
        currentAnimationGroup.stop();
        currentAnimationGroup.reset();
        currentAnimationGroup.start(true);
        
        showStatus(`Recording sprite sheet: frame 0/${totalFramesToCapture}`);
        
        // Disable record button during recording
        recordBtn.disabled = true;
        recordBtn.textContent = "Recording...";
        
        // Start the recording loop
        setTimeout(() => {
            recordingLoop();
        }, 100);
    }
    
    // Update the recordingLoop function to ensure proper frame timing
    async function recordingLoop() {
        if (!isRecording || !currentAnimationGroup) return;
        
        // Capture the current frame
        captureFrame();
        
        // Wait for animation to advance
        if (frameCount < totalFramesToCapture) {
            // Calculate delay based on animation speed and length
            const animSpeed = currentAnimationGroup.speedRatio || 1.0;
            const animTotalFrames = currentAnimationGroup.to - currentAnimationGroup.from;
            
            // Calculate how many animation frames to advance per sprite frame
            const framesPerCapture = animTotalFrames / totalFramesToCapture;
            
            // Calculate delay needed based on animation speed
            // Use a more precise timing method to ensure consistent frames
            const delay = (1000 / 30) * framesPerCapture / animSpeed;
            
            // Use setTimeout for next frame
            setTimeout(recordingLoop, delay);
        }
    }
    
    // Update captureFrame to include better error handling and ensure proper image loading
    function captureFrame() {
        if (!isRecording || !currentAnimationGroup) return;
        
        try {
            // Update status every few frames
            if (frameCount % 5 === 0) {
                showStatus(`Recording sprite sheet: frame ${frameCount}/${totalFramesToCapture}`);
            }
            
            // Get desired cell dimensions from inputs
            const cellWidth = parseInt(cellWidthInput.value) || 256;
            const cellHeight = parseInt(cellHeightInput.value) || 256;
            
            // Force transparent background during capture
            scene.clearColor = new BABYLON.Color4(0, 0, 0, 0);
            
            // Take the screenshot
            BABYLON.Tools.CreateScreenshot(engine, camera, {
                width: cellWidth,
                height: cellHeight,
                precision: 1
            }, function(data) {
                if (!data || data.length < 100) {
                    // Something went wrong with the capture
                    console.error("Invalid screenshot data received");
                    captureRetry();
                    return;
                }
                
                // Store the image data with proper loading handling
                const img = new Image();
                img.onload = function() {
                    recordedFrames.push(img);
                    
                    // Check if we've captured all frames
                    frameCount++;
                    showStatus(`Recording sprite sheet: frame ${frameCount}/${totalFramesToCapture}`);
                    
                    if (frameCount >= totalFramesToCapture) {
                        finishRecording();
                    }
                };
                
                img.onerror = function(err) {
                    console.error("Error loading captured frame:", err);
                    captureRetry();
                };
                
                // Set the source after attaching event handlers
                img.src = data;
            });
        } catch (err) {
            console.error("Error in captureFrame:", err);
            captureRetry();
        }
    }
    
    // Add a retry mechanism for failed captures
    function captureRetry() {
        // If too many retries, abort
        if (frameCount > totalFramesToCapture * 1.5) {
            showStatus("Too many retries, aborting recording", true);
            finishRecording(true);
            return;
        }
        
        // Try again after a short delay
        setTimeout(() => {
            if (isRecording) {
                captureFrame();
            }
        }, 200);
    }
    
    // Add a proper finish function to ensure complete recording
    function finishRecording(hasError = false) {
        isRecording = false;
        
        if (hasError) {
            // Handle error case
            recordBtn.disabled = false;
            recordBtn.textContent = "Record Sprite Sheet";
            return;
        }
        
        // Validate we have all frames
        if (recordedFrames.length !== totalFramesToCapture) {
            showStatus(`Warning: Captured ${recordedFrames.length} frames instead of ${totalFramesToCapture}`, true);
        }
        
        // Create the sprite sheet
        if (recordedFrames.length > 0) {
            showStatus(`Creating sprite sheet with ${recordedFrames.length} frames...`);
            createSpriteSheet();
        } else {
            showStatus("Failed to capture any frames", true);
            recordBtn.disabled = false;
            recordBtn.textContent = "Record Sprite Sheet";
        }
    }
    
    // Add this helper function at the bottom of the file to improve animation timing
    function getAnimationFrameRate(animGroup) {
        // Try to determine the animation frame rate
        if (animGroup && animGroup.animatables.length > 0) {
            const anim = animGroup.animatables[0].animations[0];
            if (anim) return anim.framePerSecond || 30;
        }
        return 30; // Default to 30fps if not determinable
    }
    
    /**
     * Analyze a frame to find its content bounds (non-transparent areas)
     * @param {HTMLImageElement} frame - The frame image to analyze
     * @returns {Object} - The content bounds {left, top, right, bottom, width, height}
     */
    function analyzeFrameContent(frame) {
        // Create a temporary canvas for analysis
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        tempCanvas.width = frame.width;
        tempCanvas.height = frame.height;
        
        // Draw the frame on the temp canvas
        tempCtx.drawImage(frame, 0, 0);
        
        // Get the pixel data
        const pixelData = tempCtx.getImageData(0, 0, tempCanvas.width, tempCanvas.height).data;
        
        // Find content boundaries
        let left = frame.width;
        let top = frame.height;
        let right = 0;
        let bottom = 0;
        let foundContent = false;
        
        // Analyze the pixel data to find content bounds
        for (let y = 0; y < frame.height; y++) {
            for (let x = 0; x < frame.width; x++) {
                const pixelIndex = (y * frame.width + x) * 4;
                // Check pixel alpha (non-transparent)
                if (pixelData[pixelIndex + 3] > 10) { // Alpha threshold
                    left = Math.min(left, x);
                    top = Math.min(top, y);
                    right = Math.max(right, x);
                    bottom = Math.max(bottom, y);
                    foundContent = true;
                }
            }
        }
        
        // Fallback if no content found
        if (!foundContent) {
            return {
                left: 0,
                top: 0,
                right: frame.width - 1,
                bottom: frame.height - 1,
                width: frame.width,
                height: frame.height
            };
        }
        
        // Add asymmetric margins - less on bottom, more on sides and top
        const marginX = Math.ceil(frame.width * 0.025); // 2.5% of width for horizontal margin
        const marginTopY = Math.ceil(frame.height * 0.025); // 2.5% of height for top margin
        const marginBottomY = Math.ceil(frame.height * 0.01); // 1% of height for bottom margin
        
        left = Math.max(0, left - marginX);
        top = Math.max(0, top - marginTopY);
        right = Math.min(frame.width - 1, right + marginX);
        bottom = Math.min(frame.height - 1, bottom + marginBottomY);
        
        return {
            left: left,
            top: top,
            right: right,
            bottom: bottom,
            width: right - left + 1,
            height: bottom - top + 1
        };
    }
    
    // Modify createSpriteSheet function to respect exact cell dimensions from user input

    function createSpriteSheet() {
        showStatus("Creating transparent sprite sheet...");
        
        // Get parameters
        const numFrames = recordedFrames.length;
        const numColumns = parseInt(columnsInput.value) || 4;
        const numRows = Math.ceil(numFrames / numColumns);
        
        // Get exact cell dimensions from user input
        const cellWidth = parseInt(cellWidthInput.value) || 256;
        const cellHeight = parseInt(cellHeightInput.value) || 256;
        
        // Check if optimization is enabled
        const shouldOptimize = document.getElementById('optimizeInput').checked;
        
        // Set the capture canvas size for the entire sprite sheet
        // Use exact dimensions based on user input
        captureCanvas.width = cellWidth * numColumns;
        captureCanvas.height = cellHeight * numRows;
        
        // Clear the canvas with transparent background
        captureContext.clearRect(0, 0, captureCanvas.width, captureCanvas.height);
        
        if (shouldOptimize) {
            // With optimization: analyze frames but maintain exact cell size
            const frameBounds = recordedFrames.map(frame => analyzeFrameContent(frame));
            
            for (let i = 0; i < numFrames; i++) {
                const row = Math.floor(i / numColumns);
                const col = i % numColumns;
                
                // Calculate target position on sprite sheet using exact cell dimensions
                const targetX = col * cellWidth;
                const targetY = row * cellHeight;
                
                const frame = recordedFrames[i];
                const bounds = frameBounds[i];
                
                // Calculate centering offsets within the fixed cell
                const offsetX = Math.floor((cellWidth - bounds.width) / 2);
                const offsetY = Math.floor((cellHeight - bounds.height) / 2);
                
                // Draw the content portion of the frame centered in its fixed-size cell
                captureContext.drawImage(
                    frame,
                    bounds.left, bounds.top, bounds.width, bounds.height,
                    targetX + offsetX, targetY + offsetY, bounds.width, bounds.height
                );
                
                // Draw cell borders for visualization (optional)
                // captureContext.strokeStyle = 'rgba(255, 0, 0, 0.3)';
                // captureContext.strokeRect(targetX, targetY, cellWidth, cellHeight);
            }
        } else {
            // Without optimization: just place each frame directly in a cell of exact size
            for (let i = 0; i < numFrames; i++) {
                const row = Math.floor(i / numColumns);
                const col = i % numColumns;
                
                // Calculate target position based on exact cell dimensions
                const targetX = col * cellWidth;
                const targetY = row * cellHeight;
                
                // Draw the full frame scaled to fit the exact cell dimensions
                captureContext.drawImage(
                    recordedFrames[i],
                    0, 0, recordedFrames[i].width, recordedFrames[i].height,
                    targetX, targetY, cellWidth, cellHeight
                );
            }
        }
        
        // Update the metadata to reflect exact cell dimensions
        const metadata = {
            frameWidth: cellWidth,
            frameHeight: cellHeight,
            frames: numFrames,
            columns: numColumns,
            rows: numRows,
            animationName: currentAnimationGroup.name.replace(/[^\w\s-]/g, '_'),  // Sanitize for filename
            optimized: shouldOptimize,
            date: new Date().toISOString(),
            transparent: true,
            cameraPosition: {
                offsetX: cameraOffsetX,
                offsetY: cameraOffsetY
            }
        };
        
        // If optimized, add optimization data
        if (shouldOptimize) {
            metadata.optimizationInfo = {
                note: "Frames are centered within fixed-size cells",
                maintainsExactDimensions: true
            };
        }
        
        // Create downloadable sprite sheet image
        const dataURL = captureCanvas.toDataURL('image/png');
        const downloadLink = document.createElement('a');
        downloadLink.href = dataURL;
        downloadLink.download = `${currentAnimationGroup.name.replace(/[^\w\s-]/g, '_')}_spritesheet.png`;
        document.body.appendChild(downloadLink);
        downloadLink.click();
        document.body.removeChild(downloadLink);
        
        // Create downloadable metadata JSON
        const metadataBlob = new Blob([JSON.stringify(metadata, null, 2)], {type: 'application/json'});
        const metadataURL = URL.createObjectURL(metadataBlob);
        const metadataLink = document.createElement('a');
        metadataLink.href = metadataURL;
        metadataLink.download = `${currentAnimationGroup.name.replace(/[^\w\s-]/g, '_')}_metadata.json`;
        document.body.appendChild(metadataLink);
        metadataLink.click();
        document.body.removeChild(metadataLink);
        URL.revokeObjectURL(metadataURL);
        
        // Re-enable record button
        recordBtn.disabled = false;
        recordBtn.textContent = "Record Sprite Sheet";
        
        // Update status message with exact dimensions used
        showStatus(`Sprite sheet created with ${numFrames} frames - Exact cell size: ${cellWidth}x${cellHeight}px`);
    }
    
    // Create camera adjustment UI in the createScene function
    function createCameraControls() {
        // Create a container for camera controls
        const cameraControlPanel = document.createElement('div');
        cameraControlPanel.id = 'cameraControlPanel';
        cameraControlPanel.className = 'control-panel';
        cameraControlPanel.style.position = 'absolute';
        cameraControlPanel.style.left = '10px';
        cameraControlPanel.style.top = '10px';
        cameraControlPanel.style.backgroundColor = 'rgba(0,0,0,0.7)';
        cameraControlPanel.style.borderRadius = '5px';
        cameraControlPanel.style.padding = '10px';
        cameraControlPanel.style.color = 'white';
        cameraControlPanel.style.zIndex = '100';
        cameraControlPanel.style.display = 'flex';
        cameraControlPanel.style.flexDirection = 'column';
        cameraControlPanel.style.alignItems = 'center';
        
        // Add a title
        const title = document.createElement('div');
        title.textContent = 'Camera Position';
        title.style.marginBottom = '10px';
        title.style.fontWeight = 'bold';
        cameraControlPanel.appendChild(title);
        
        // Create a grid for direction buttons
        const controlGrid = document.createElement('div');
        controlGrid.style.display = 'grid';
        controlGrid.style.gridTemplateColumns = '1fr 1fr 1fr';
        controlGrid.style.gridTemplateRows = '1fr 1fr 1fr';
        controlGrid.style.gap = '3px';
        
        // Create buttons for different directions
        const buttonStyle = 'width:32px; height:32px; background:#444; border:none; color:white; cursor:pointer; border-radius:4px; font-weight:bold;';
        
        // Top row
        const btnEmpty1 = document.createElement('button');
        btnEmpty1.style = buttonStyle;
        btnEmpty1.style.visibility = 'hidden';
        
        const btnUp = document.createElement('button');
        btnUp.innerHTML = '▲';
        btnUp.style = buttonStyle;
        btnUp.onclick = () => adjustCameraPosition(0, 1);
        
        const btnEmpty2 = document.createElement('button');
        btnEmpty2.style = buttonStyle;
        btnEmpty2.style.visibility = 'hidden';
        
        // Middle row
        const btnLeft = document.createElement('button');
        btnLeft.innerHTML = '◀';
        btnLeft.style = buttonStyle;
        btnLeft.onclick = () => adjustCameraPosition(-1, 0);
        
        const btnReset = document.createElement('button');
        btnReset.innerHTML = '⌂';
        btnReset.style = buttonStyle;
        btnReset.onclick = resetCameraPosition;
        
        const btnRight = document.createElement('button');
        btnRight.innerHTML = '▶';
        btnRight.style = buttonStyle;
        btnRight.onclick = () => adjustCameraPosition(1, 0);
        
        // Bottom row
        const btnEmpty3 = document.createElement('button');
        btnEmpty3.style = buttonStyle;
        btnEmpty3.style.visibility = 'hidden';
        
        const btnDown = document.createElement('button');
        btnDown.innerHTML = '▼';
        btnDown.style = buttonStyle;
        btnDown.onclick = () => adjustCameraPosition(0, -1);
        
        const btnEmpty4 = document.createElement('button');
        btnEmpty4.style = buttonStyle;
        btnEmpty4.style.visibility = 'hidden';
        
        // Add all buttons to the grid
        controlGrid.appendChild(btnEmpty1);
        controlGrid.appendChild(btnUp);
        controlGrid.appendChild(btnEmpty2);
        
        controlGrid.appendChild(btnLeft);
        controlGrid.appendChild(btnReset);
        controlGrid.appendChild(btnRight);
        
        controlGrid.appendChild(btnEmpty3);
        controlGrid.appendChild(btnDown);
        controlGrid.appendChild(btnEmpty4);
        
        cameraControlPanel.appendChild(controlGrid);
        
        // Add step size control
        const stepControl = document.createElement('div');
        stepControl.style.marginTop = '10px';
        stepControl.style.width = '100%';
        stepControl.style.textAlign = 'center';
        
        const stepLabel = document.createElement('label');
        stepLabel.textContent = 'Step Size:';
        stepLabel.style.display = 'block';
        stepLabel.style.marginBottom = '5px';
        stepLabel.style.fontSize = '12px';
        stepControl.appendChild(stepLabel);
        
        const stepSelect = document.createElement('select');
        stepSelect.style.width = '100%';
        stepSelect.style.backgroundColor = '#333';
        stepSelect.style.color = 'white';
        stepSelect.style.border = 'none';
        stepSelect.style.padding = '3px';
        
        [0.1, 0.25, 0.5, 1.0].forEach(step => {
            const option = document.createElement('option');
            option.value = step;
            option.textContent = step;
            if (step === cameraAdjustStep) {
                option.selected = true;
            }
            stepSelect.appendChild(option);
        });
        
        stepSelect.onchange = (e) => {
            window.cameraAdjustStep = parseFloat(e.target.value);
            showStatus(`Camera step size set to ${window.cameraAdjustStep}`);
        };
        
        stepControl.appendChild(stepSelect);
        cameraControlPanel.appendChild(stepControl);
        
        // Add keyboard instructions
        const keyboardHelp = document.createElement('div');
        keyboardHelp.style.marginTop = '10px';
        keyboardHelp.style.fontSize = '11px';
        keyboardHelp.style.color = '#aaa';
        keyboardHelp.textContent = 'Use arrow keys to adjust camera position';
        cameraControlPanel.appendChild(keyboardHelp);
        
        // Add to document
        document.body.appendChild(cameraControlPanel);
        
        // Add keyboard controls
        document.addEventListener('keydown', (e) => {
            switch(e.key) {
                case 'ArrowUp':
                    adjustCameraPosition(0, 1);
                    break;
                case 'ArrowDown':
                    adjustCameraPosition(0, -1);
                    break;
                case 'ArrowLeft':
                    adjustCameraPosition(-1, 0);
                    break;
                case 'ArrowRight':
                    adjustCameraPosition(1, 0);
                    break;
                case 'Home':
                    resetCameraPosition();
                    break;
            }
        });
        
        return cameraControlPanel;
    }
    
    // Create and set up the scene
    try {
        scene = await createScene();
        
        // Only start render loop after scene is created
        engine.runRenderLoop(() => {
            if (scene) {
                scene.render();
            }
        });

        // Handle window resize
        window.addEventListener('resize', () => {
            engine.resize();
        });
    } catch (error) {
        console.error("Error creating scene:", error);
    }

    // Register event listeners
    playBtn.addEventListener('click', () => {
        const selectedIndex = parseInt(animationList.value);
        playAnimation(selectedIndex);
    });
    
    pauseBtn.addEventListener('click', () => {
        if (currentAnimationGroup) {
            currentAnimationGroup.pause();
            showStatus(`Paused animation: ${currentAnimationGroup.name}`);
        }
    });
    
    animationList.addEventListener('change', () => {
        const selectedIndex = parseInt(animationList.value);
        playAnimation(selectedIndex);
    });
    
    // Add sprite sheet recording event listener
    recordBtn.addEventListener('click', startRecording);
    
    // Run the render loop with frame capture REMOVED from here
    engine.runRenderLoop(() => {
        scene.render();
        // We'll handle frame capture separately with the recordingLoop function
    });
    
    // Handle window resize
    window.addEventListener('resize', () => {
        // Resize the engine
        engine.resize();
        
        // Ensure camera maintains square ratio
        updateOrthoCamera();
        
        // Update canvas size to remain square
        const size = Math.min(window.innerHeight, window.innerWidth);
        canvas.style.width = `${size}px`;
        canvas.style.height = `${size}px`;
    });
});


```

## Source file: js/gif.worker.js

{
  "path": "js/gif.worker.js",
  "blob_sha": "f1fd8de79461d0cfe5cd4dffb7e44c00cdc64fc5",
  "sha256": "e72c66d202722714cc113d8660a4fc5ec9fe62748d46ef231f91d3d21dda2ba4",
  "lines": 3,
  "bytes": 16635,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/sjefvanleeuwen/sprite-sheet-creator/blob/eceafa51596850082f2e377ab4c37d8ef875c9c5/js/gif.worker.js",
  "function_signature_candidates": []
}

```text

// gif.worker.js 0.2.0 - https://github.com/jnordberg/gif.js
(function e(t,n,r){function s(o,u){if(!n[o]){if(!t[o]){var a=typeof require=="function"&&require;if(!u&&a)return a(o,!0);if(i)return i(o,!0);var f=new Error("Cannot find module '"+o+"'");throw f.code="MODULE_NOT_FOUND",f}var l=n[o]={exports:{}};t[o][0].call(l.exports,function(e){var n=t[o][1][e];return s(n?n:e)},l,l.exports,e,t,n,r)}return n[o].exports}var i=typeof require=="function"&&require;for(var o=0;o<r.length;o++)s(r[o]);return s})({1:[function(require,module,exports){var NeuQuant=require("./TypedNeuQuant.js");var LZWEncoder=require("./LZWEncoder.js");function ByteArray(){this.page=-1;this.pages=[];this.newPage()}ByteArray.pageSize=4096;ByteArray.charMap={};for(var i=0;i<256;i++)ByteArray.charMap[i]=String.fromCharCode(i);ByteArray.prototype.newPage=function(){this.pages[++this.page]=new Uint8Array(ByteArray.pageSize);this.cursor=0};ByteArray.prototype.getData=function(){var rv="";for(var p=0;p<this.pages.length;p++){for(var i=0;i<ByteArray.pageSize;i++){rv+=ByteArray.charMap[this.pages[p][i]]}}return rv};ByteArray.prototype.writeByte=function(val){if(this.cursor>=ByteArray.pageSize)this.newPage();this.pages[this.page][this.cursor++]=val};ByteArray.prototype.writeUTFBytes=function(string){for(var l=string.length,i=0;i<l;i++)this.writeByte(string.charCodeAt(i))};ByteArray.prototype.writeBytes=function(array,offset,length){for(var l=length||array.length,i=offset||0;i<l;i++)this.writeByte(array[i])};function GIFEncoder(width,height){this.width=~~width;this.height=~~height;this.transparent=null;this.transIndex=0;this.repeat=-1;this.delay=0;this.image=null;this.pixels=null;this.indexedPixels=null;this.colorDepth=null;this.colorTab=null;this.neuQuant=null;this.usedEntry=new Array;this.palSize=7;this.dispose=-1;this.firstFrame=true;this.sample=10;this.dither=false;this.globalPalette=false;this.out=new ByteArray}GIFEncoder.prototype.setDelay=function(milliseconds){this.delay=Math.round(milliseconds/10)};GIFEncoder.prototype.setFrameRate=function(fps){this.delay=Math.round(100/fps)};GIFEncoder.prototype.setDispose=function(disposalCode){if(disposalCode>=0)this.dispose=disposalCode};GIFEncoder.prototype.setRepeat=function(repeat){this.repeat=repeat};GIFEncoder.prototype.setTransparent=function(color){this.transparent=color};GIFEncoder.prototype.addFrame=function(imageData){this.image=imageData;this.colorTab=this.globalPalette&&this.globalPalette.slice?this.globalPalette:null;this.getImagePixels();this.analyzePixels();if(this.globalPalette===true)this.globalPalette=this.colorTab;if(this.firstFrame){this.writeLSD();this.writePalette();if(this.repeat>=0){this.writeNetscapeExt()}}this.writeGraphicCtrlExt();this.writeImageDesc();if(!this.firstFrame&&!this.globalPalette)this.writePalette();this.writePixels();this.firstFrame=false};GIFEncoder.prototype.finish=function(){this.out.writeByte(59)};GIFEncoder.prototype.setQuality=function(quality){if(quality<1)quality=1;this.sample=quality};GIFEncoder.prototype.setDither=function(dither){if(dither===true)dither="FloydSteinberg";this.dither=dither};GIFEncoder.prototype.setGlobalPalette=function(palette){this.globalPalette=palette};GIFEncoder.prototype.getGlobalPalette=function(){return this.globalPalette&&this.globalPalette.slice&&this.globalPalette.slice(0)||this.globalPalette};GIFEncoder.prototype.writeHeader=function(){this.out.writeUTFBytes("GIF89a")};GIFEncoder.prototype.analyzePixels=function(){if(!this.colorTab){this.neuQuant=new NeuQuant(this.pixels,this.sample);this.neuQuant.buildColormap();this.colorTab=this.neuQuant.getColormap()}if(this.dither){this.ditherPixels(this.dither.replace("-serpentine",""),this.dither.match(/-serpentine/)!==null)}else{this.indexPixels()}this.pixels=null;this.colorDepth=8;this.palSize=7;if(this.transparent!==null){this.transIndex=this.findClosest(this.transparent,true)}};GIFEncoder.prototype.indexPixels=function(imgq){var nPix=this.pixels.length/3;this.indexedPixels=new Uint8Array(nPix);var k=0;for(var j=0;j<nPix;j++){var index=this.findClosestRGB(this.pixels[k++]&255,this.pixels[k++]&255,this.pixels[k++]&255);this.usedEntry[index]=true;this.indexedPixels[j]=index}};GIFEncoder.prototype.ditherPixels=function(kernel,serpentine){var kernels={FalseFloydSteinberg:[[3/8,1,0],[3/8,0,1],[2/8,1,1]],FloydSteinberg:[[7/16,1,0],[3/16,-1,1],[5/16,0,1],[1/16,1,1]],Stucki:[[8/42,1,0],[4/42,2,0],[2/42,-2,1],[4/42,-1,1],[8/42,0,1],[4/42,1,1],[2/42,2,1],[1/42,-2,2],[2/42,-1,2],[4/42,0,2],[2/42,1,2],[1/42,2,2]],Atkinson:[[1/8,1,0],[1/8,2,0],[1/8,-1,1],[1/8,0,1],[1/8,1,1],[1/8,0,2]]};if(!kernel||!kernels[kernel]){throw"Unknown dithering kernel: "+kernel}var ds=kernels[kernel];var index=0,height=this.height,width=this.width,data=this.pixels;var direction=serpentine?-1:1;this.indexedPixels=new Uint8Array(this.pixels.length/3);for(var y=0;y<height;y++){if(serpentine)direction=direction*-1;for(var x=direction==1?0:width-1,xend=direction==1?width:0;x!==xend;x+=direction){index=y*width+x;var idx=index*3;var r1=data[idx];var g1=data[idx+1];var b1=data[idx+2];idx=this.findClosestRGB(r1,g1,b1);this.usedEntry[idx]=true;this.indexedPixels[index]=idx;idx*=3;var r2=this.colorTab[idx];var g2=this.colorTab[idx+1];var b2=this.colorTab[idx+2];var er=r1-r2;var eg=g1-g2;var eb=b1-b2;for(var i=direction==1?0:ds.length-1,end=direction==1?ds.length:0;i!==end;i+=direction){var x1=ds[i][1];var y1=ds[i][2];if(x1+x>=0&&x1+x<width&&y1+y>=0&&y1+y<height){var d=ds[i][0];idx=index+x1+y1*width;idx*=3;data[idx]=Math.max(0,Math.min(255,data[idx]+er*d));data[idx+1]=Math.max(0,Math.min(255,data[idx+1]+eg*d));data[idx+2]=Math.max(0,Math.min(255,data[idx+2]+eb*d))}}}}};GIFEncoder.prototype.findClosest=function(c,used){return this.findClosestRGB((c&16711680)>>16,(c&65280)>>8,c&255,used)};GIFEncoder.prototype.findClosestRGB=function(r,g,b,used){if(this.colorTab===null)return-1;if(this.neuQuant&&!used){return this.neuQuant.lookupRGB(r,g,b)}var c=b|g<<8|r<<16;var minpos=0;var dmin=256*256*256;var len=this.colorTab.length;for(var i=0,index=0;i<len;index++){var dr=r-(this.colorTab[i++]&255);var dg=g-(this.colorTab[i++]&255);var db=b-(this.colorTab[i++]&255);var d=dr*dr+dg*dg+db*db;if((!used||this.usedEntry[index])&&d<dmin){dmin=d;minpos=index}}return minpos};GIFEncoder.prototype.getImagePixels=function(){var w=this.width;var h=this.height;this.pixels=new Uint8Array(w*h*3);var data=this.image;var srcPos=0;var count=0;for(var i=0;i<h;i++){for(var j=0;j<w;j++){this.pixels[count++]=data[srcPos++];this.pixels[count++]=data[srcPos++];this.pixels[count++]=data[srcPos++];srcPos++}}};GIFEncoder.prototype.writeGraphicCtrlExt=function(){this.out.writeByte(33);this.out.writeByte(249);this.out.writeByte(4);var transp,disp;if(this.transparent===null){transp=0;disp=0}else{transp=1;disp=2}if(this.dispose>=0){disp=dispose&7}disp<<=2;this.out.writeByte(0|disp|0|transp);this.writeShort(this.delay);this.out.writeByte(this.transIndex);this.out.writeByte(0)};GIFEncoder.prototype.writeImageDesc=function(){this.out.writeByte(44);this.writeShort(0);this.writeShort(0);this.writeShort(this.width);this.writeShort(this.height);if(this.firstFrame||this.globalPalette){this.out.writeByte(0)}else{this.out.writeByte(128|0|0|0|this.palSize)}};GIFEncoder.prototype.writeLSD=function(){this.writeShort(this.width);this.writeShort(this.height);this.out.writeByte(128|112|0|this.palSize);this.out.writeByte(0);this.out.writeByte(0)};GIFEncoder.prototype.writeNetscapeExt=function(){this.out.writeByte(33);this.out.writeByte(255);this.out.writeByte(11);this.out.writeUTFBytes("NETSCAPE2.0");this.out.writeByte(3);this.out.writeByte(1);this.writeShort(this.repeat);this.out.writeByte(0)};GIFEncoder.prototype.writePalette=function(){this.out.writeBytes(this.colorTab);var n=3*256-this.colorTab.length;for(var i=0;i<n;i++)this.out.writeByte(0)};GIFEncoder.prototype.writeShort=function(pValue){this.out.writeByte(pValue&255);this.out.writeByte(pValue>>8&255)};GIFEncoder.prototype.writePixels=function(){var enc=new LZWEncoder(this.width,this.height,this.indexedPixels,this.colorDepth);enc.encode(this.out)};GIFEncoder.prototype.stream=function(){return this.out};module.exports=GIFEncoder},{"./LZWEncoder.js":2,"./TypedNeuQuant.js":3}],2:[function(require,module,exports){var EOF=-1;var BITS=12;var HSIZE=5003;var masks=[0,1,3,7,15,31,63,127,255,511,1023,2047,4095,8191,16383,32767,65535];function LZWEncoder(width,height,pixels,colorDepth){var initCodeSize=Math.max(2,colorDepth);var accum=new Uint8Array(256);var htab=new Int32Array(HSIZE);var codetab=new Int32Array(HSIZE);var cur_accum,cur_bits=0;var a_count;var free_ent=0;var maxcode;var clear_flg=false;var g_init_bits,ClearCode,EOFCode;function char_out(c,outs){accum[a_count++]=c;if(a_count>=254)flush_char(outs)}function cl_block(outs){cl_hash(HSIZE);free_ent=ClearCode+2;clear_flg=true;output(ClearCode,outs)}function cl_hash(hsize){for(var i=0;i<hsize;++i)htab[i]=-1}function compress(init_bits,outs){var fcode,c,i,ent,disp,hsize_reg,hshift;g_init_bits=init_bits;clear_flg=false;n_bits=g_init_bits;maxcode=MAXCODE(n_bits);ClearCode=1<<init_bits-1;EOFCode=ClearCode+1;free_ent=ClearCode+2;a_count=0;ent=nextPixel();hshift=0;for(fcode=HSIZE;fcode<65536;fcode*=2)++hshift;hshift=8-hshift;hsize_reg=HSIZE;cl_hash(hsize_reg);output(ClearCode,outs);outer_loop:while((c=nextPixel())!=EOF){fcode=(c<<BITS)+ent;i=c<<hshift^ent;if(htab[i]===fcode){ent=codetab[i];continue}else if(htab[i]>=0){disp=hsize_reg-i;if(i===0)disp=1;do{if((i-=disp)<0)i+=hsize_reg;if(htab[i]===fcode){ent=codetab[i];continue outer_loop}}while(htab[i]>=0)}output(ent,outs);ent=c;if(free_ent<1<<BITS){codetab[i]=free_ent++;htab[i]=fcode}else{cl_block(outs)}}output(ent,outs);output(EOFCode,outs)}function encode(outs){outs.writeByte(initCodeSize);remaining=width*height;curPixel=0;compress(initCodeSize+1,outs);outs.writeByte(0)}function flush_char(outs){if(a_count>0){outs.writeByte(a_count);outs.writeBytes(accum,0,a_count);a_count=0}}function MAXCODE(n_bits){return(1<<n_bits)-1}function nextPixel(){if(remaining===0)return EOF;--remaining;var pix=pixels[curPixel++];return pix&255}function output(code,outs){cur_accum&=masks[cur_bits];if(cur_bits>0)cur_accum|=code<<cur_bits;else cur_accum=code;cur_bits+=n_bits;while(cur_bits>=8){char_out(cur_accum&255,outs);cur_accum>>=8;cur_bits-=8}if(free_ent>maxcode||clear_flg){if(clear_flg){maxcode=MAXCODE(n_bits=g_init_bits);clear_flg=false}else{++n_bits;if(n_bits==BITS)maxcode=1<<BITS;else maxcode=MAXCODE(n_bits)}}if(code==EOFCode){while(cur_bits>0){char_out(cur_accum&255,outs);cur_accum>>=8;cur_bits-=8}flush_char(outs)}}this.encode=encode}module.exports=LZWEncoder},{}],3:[function(require,module,exports){var ncycles=100;var netsize=256;var maxnetpos=netsize-1;var netbiasshift=4;var intbiasshift=16;var intbias=1<<intbiasshift;var gammashift=10;var gamma=1<<gammashift;var betashift=10;var beta=intbias>>betashift;var betagamma=intbias<<gammashift-betashift;var initrad=netsize>>3;var radiusbiasshift=6;var radiusbias=1<<radiusbiasshift;var initradius=initrad*radiusbias;var radiusdec=30;var alphabiasshift=10;var initalpha=1<<alphabiasshift;var alphadec;var radbiasshift=8;var radbias=1<<radbiasshift;var alpharadbshift=alphabiasshift+radbiasshift;var alpharadbias=1<<alpharadbshift;var prime1=499;var prime2=491;var prime3=487;var prime4=503;var minpicturebytes=3*prime4;function NeuQuant(pixels,samplefac){var network;var netindex;var bias;var freq;var radpower;function init(){network=[];netindex=new Int32Array(256);bias=new Int32Array(netsize);freq=new Int32Array(netsize);radpower=new Int32Array(netsize>>3);var i,v;for(i=0;i<netsize;i++){v=(i<<netbiasshift+8)/netsize;network[i]=new Float64Array([v,v,v,0]);freq[i]=intbias/netsize;bias[i]=0}}function unbiasnet(){for(var i=0;i<netsize;i++){network[i][0]>>=netbiasshift;network[i][1]>>=netbiasshift;network[i][2]>>=netbiasshift;network[i][3]=i}}function altersingle(alpha,i,b,g,r){network[i][0]-=alpha*(network[i][0]-b)/initalpha;network[i][1]-=alpha*(network[i][1]-g)/initalpha;network[i][2]-=alpha*(network[i][2]-r)/initalpha}function alterneigh(radius,i,b,g,r){var lo=Math.abs(i-radius);var hi=Math.min(i+radius,netsize);var j=i+1;var k=i-1;var m=1;var p,a;while(j<hi||k>lo){a=radpower[m++];if(j<hi){p=network[j++];p[0]-=a*(p[0]-b)/alpharadbias;p[1]-=a*(p[1]-g)/alpharadbias;p[2]-=a*(p[2]-r)/alpharadbias}if(k>lo){p=network[k--];p[0]-=a*(p[0]-b)/alpharadbias;p[1]-=a*(p[1]-g)/alpharadbias;p[2]-=a*(p[2]-r)/alpharadbias}}}function contest(b,g,r){var bestd=~(1<<31);var bestbiasd=bestd;var bestpos=-1;var bestbiaspos=bestpos;var i,n,dist,biasdist,betafreq;for(i=0;i<netsize;i++){n=network[i];dist=Math.abs(n[0]-b)+Math.abs(n[1]-g)+Math.abs(n[2]-r);if(dist<bestd){bestd=dist;bestpos=i}biasdist=dist-(bias[i]>>intbiasshift-netbiasshift);if(biasdist<bestbiasd){bestbiasd=biasdist;bestbiaspos=i}betafreq=freq[i]>>betashift;freq[i]-=betafreq;bias[i]+=betafreq<<gammashift}freq[bestpos]+=beta;bias[bestpos]-=betagamma;return bestbiaspos}function inxbuild(){var i,j,p,q,smallpos,smallval,previouscol=0,startpos=0;for(i=0;i<netsize;i++){p=network[i];smallpos=i;smallval=p[1];for(j=i+1;j<netsize;j++){q=network[j];if(q[1]<smallval){smallpos=j;smallval=q[1]}}q=network[smallpos];if(i!=smallpos){j=q[0];q[0]=p[0];p[0]=j;j=q[1];q[1]=p[1];p[1]=j;j=q[2];q[2]=p[2];p[2]=j;j=q[3];q[3]=p[3];p[3]=j}if(smallval!=previouscol){netindex[previouscol]=startpos+i>>1;for(j=previouscol+1;j<smallval;j++)netindex[j]=i;previouscol=smallval;startpos=i}}netindex[previouscol]=startpos+maxnetpos>>1;for(j=previouscol+1;j<256;j++)netindex[j]=maxnetpos}function inxsearch(b,g,r){var a,p,dist;var bestd=1e3;var best=-1;var i=netindex[g];var j=i-1;while(i<netsize||j>=0){if(i<netsize){p=network[i];dist=p[1]-g;if(dist>=bestd)i=netsize;else{i++;if(dist<0)dist=-dist;a=p[0]-b;if(a<0)a=-a;dist+=a;if(dist<bestd){a=p[2]-r;if(a<0)a=-a;dist+=a;if(dist<bestd){bestd=dist;best=p[3]}}}}if(j>=0){p=network[j];dist=g-p[1];if(dist>=bestd)j=-1;else{j--;if(dist<0)dist=-dist;a=p[0]-b;if(a<0)a=-a;dist+=a;if(dist<bestd){a=p[2]-r;if(a<0)a=-a;dist+=a;if(dist<bestd){bestd=dist;best=p[3]}}}}}return best}function learn(){var i;var lengthcount=pixels.length;var alphadec=30+(samplefac-1)/3;var samplepixels=lengthcount/(3*samplefac);var delta=~~(samplepixels/ncycles);var alpha=initalpha;var radius=initradius;var rad=radius>>radiusbiasshift;if(rad<=1)rad=0;for(i=0;i<rad;i++)radpower[i]=alpha*((rad*rad-i*i)*radbias/(rad*rad));var step;if(lengthcount<minpicturebytes){samplefac=1;step=3}else if(lengthcount%prime1!==0){step=3*prime1}else if(lengthcount%prime2!==0){step=3*prime2}else if(lengthcount%prime3!==0){step=3*prime3}else{step=3*prime4}var b,g,r,j;var pix=0;i=0;while(i<samplepixels){b=(pixels[pix]&255)<<netbiasshift;g=(pixels[pix+1]&255)<<netbiasshift;r=(pixels[pix+2]&255)<<netbiasshift;j=contest(b,g,r);altersingle(alpha,j,b,g,r);if(rad!==0)alterneigh(rad,j,b,g,r);pix+=step;if(pix>=lengthcount)pix-=lengthcount;i++;if(delta===0)delta=1;if(i%delta===0){alpha-=alpha/alphadec;radius-=radius/radiusdec;rad=radius>>radiusbiasshift;if(rad<=1)rad=0;for(j=0;j<rad;j++)radpower[j]=alpha*((rad*rad-j*j)*radbias/(rad*rad))}}}function buildColormap(){init();learn();unbiasnet();inxbuild()}this.buildColormap=buildColormap;function getColormap(){var map=[];var index=[];for(var i=0;i<netsize;i++)index[network[i][3]]=i;var k=0;for(var l=0;l<netsize;l++){var j=index[l];map[k++]=network[j][0];map[k++]=network[j][1];map[k++]=network[j][2]}return map}this.getColormap=getColormap;this.lookupRGB=inxsearch}module.exports=NeuQuant},{}],4:[function(require,module,exports){var GIFEncoder,renderFrame;GIFEncoder=require("./GIFEncoder.js");renderFrame=function(frame){var encoder,page,stream,transfer;encoder=new GIFEncoder(frame.width,frame.height);if(frame.index===0){encoder.writeHeader()}else{encoder.firstFrame=false}encoder.setTransparent(frame.transparent);encoder.setRepeat(frame.repeat);encoder.setDelay(frame.delay);encoder.setQuality(frame.quality);encoder.setDither(frame.dither);encoder.setGlobalPalette(frame.globalPalette);encoder.addFrame(frame.data);if(frame.last){encoder.finish()}if(frame.globalPalette===true){frame.globalPalette=encoder.getGlobalPalette()}stream=encoder.stream();frame.data=stream.pages;frame.cursor=stream.cursor;frame.pageSize=stream.constructor.pageSize;if(frame.canTransfer){transfer=function(){var i,len,ref,results;ref=frame.data;results=[];for(i=0,len=ref.length;i<len;i++){page=ref[i];results.push(page.buffer)}return results}();return self.postMessage(frame,transfer)}else{return self.postMessage(frame)}};self.onmessage=function(event){return renderFrame(event.data)}},{"./GIFEncoder.js":1}]},{},[4]);
//# sourceMappingURL=gif.worker.js.map

```

## Source file: js/GifExporter.js

{
  "path": "js/GifExporter.js",
  "blob_sha": "e10ee380cf15c019bd0e40e254114df0c009225b",
  "sha256": "81bde550f5e9061cae5e3707e343825358f0551e2b5a0b9d6e1382806d490400",
  "lines": 165,
  "bytes": 6700,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/sjefvanleeuwen/sprite-sheet-creator/blob/eceafa51596850082f2e377ab4c37d8ef875c9c5/js/GifExporter.js",
  "function_signature_candidates": []
}

```text

/**
 * GifExporter - A utility for converting sprite sheets to GIF animations
 * Works with the animation player and sprite sheets created by the FBX Viewer
 */
class GifExporter {
    /**
     * Create a new GifExporter
     * @param {Object} options - Configuration options
     */
    constructor(options = {}) {
        this.quality = options.quality || 10;
        this.workers = options.workers || 4;
        this.onProgress = options.onProgress || (() => {});
        this.onComplete = options.onComplete || (() => {});
        this.onError = options.onError || (() => {});
        
        // Check if gif.js is available
        if (typeof GIF !== 'function') {
            throw new Error('GIF.js library is required. Please include gif.js script in your HTML.');
        }
    }
    
    /**
     * Export a sprite sheet to a GIF animation
     * @param {Object} options - Export options
     * @param {HTMLImageElement} options.spriteSheet - The sprite sheet image
     * @param {Object} options.metadata - The sprite sheet metadata
     * @param {number} options.fps - Frames per second (default: from metadata or 24)
     * @param {number} options.scale - Scale factor (default: 1)
     * @param {boolean} options.loop - Whether the animation should loop (default: true)
     * @returns {Promise<Blob>} - A promise that resolves with the GIF blob
     */
    exportGif(options) {
        return new Promise((resolve, reject) => {
            const spriteSheet = options.spriteSheet;
            const metadata = options.metadata;
            
            if (!spriteSheet || !metadata) {
                return reject(new Error('Sprite sheet and metadata are required'));
            }
            
            const fps = options.fps || metadata.fps || 24;
            const scale = options.scale || 1;
            const loop = options.loop !== undefined ? options.loop : true;
            
            // Calculate frame delay in milliseconds
            const frameDelay = 1000 / fps;
            
            // Create GIF encoder with appropriate settings
            const gif = new GIF({
                workers: this.workers,
                quality: this.quality,
                width: metadata.frameWidth * scale,
                height: metadata.frameHeight * scale,
                workerScript: options.workerScript || 'https://cdn.jsdelivr.net/gh/jnordberg/gif.js@0.2.0/dist/gif.worker.js',
                transparent: 'rgba(0,0,0,0)',
                repeat: loop ? 0 : -1 // 0 = loop forever, -1 = no repeat
            });
            
            // Set up a temporary canvas for frames
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            canvas.width = metadata.frameWidth;
            canvas.height = metadata.frameHeight;
            
            // Set up a canvas for scaled frames if needed
            let scaledCanvas, scaledCtx;
            if (scale !== 1) {
                scaledCanvas = document.createElement('canvas');
                scaledCtx = scaledCanvas.getContext('2d');
                scaledCanvas.width = metadata.frameWidth * scale;
                scaledCanvas.height = metadata.frameHeight * scale;
            }
            
            // Add each frame to the GIF
            let framesProcessed = 0;
            const totalFrames = metadata.frames;
            
            for (let i = 0; i < totalFrames; i++) {
                // Calculate frame position in the sprite sheet
                const row = Math.floor(i / metadata.columns);
                const col = i % metadata.columns;
                const x = col * metadata.frameWidth;
                const y = row * metadata.frameHeight;
                
                // Draw the frame to the canvas
                ctx.clearRect(0, 0, canvas.width, canvas.height);
                ctx.drawImage(
                    spriteSheet, 
                    x, y, metadata.frameWidth, metadata.frameHeight,
                    0, 0, canvas.width, canvas.height
                );
                
                // Use scaled canvas if needed
                if (scale !== 1) {
                    scaledCtx.clearRect(0, 0, scaledCanvas.width, scaledCanvas.height);
                    scaledCtx.drawImage(
                        canvas, 
                        0, 0, canvas.width, canvas.height,
                        0, 0, scaledCanvas.width, scaledCanvas.height
                    );
                    gif.addFrame(scaledCanvas, {delay: frameDelay, copy: true});
                } else {
                    gif.addFrame(canvas, {delay: frameDelay, copy: true});
                }
                
                framesProcessed++;
                this.onProgress(framesProcessed / totalFrames * 0.5); // First 50% is adding frames
            }
            
            // Handle rendering progress and completion
            gif.on('progress', progress => {
                // Second 50% is rendering the GIF
                this.onProgress(0.5 + progress * 0.5); 
            });
            
            gif.on('finished', blob => {
                this.onComplete(blob);
                resolve(blob);
            });
            
            // Render the GIF
            try {
                gif.render();
            } catch (error) {
                this.onError(error);
                reject(error);
            }
        });
    }
    
    /**
     * Create a download link for a GIF blob
     * @param {Blob} blob - The GIF blob
     * @param {string} filename - The filename for the download
     * @returns {HTMLAnchorElement} - The download link element
     */
    createDownloadLink(blob, filename = 'animation.gif') {
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        link.textContent = 'Download GIF';
        link.className = 'download-btn';
        link.style = 'background-color: #27ae60; color: white; padding: 8px 16px; border-radius: 4px; text-decoration: none; display: inline-block;';
        
        // Clean up object URL after download
        link.addEventListener('click', () => {
            setTimeout(() => {
                URL.revokeObjectURL(url);
            }, 100);
        });
        
        return link;
    }
}

// Export for use in module systems or global scope
if (typeof module !== 'undefined' && typeof module.exports !== 'undefined') {
    module.exports = GifExporter;
} else if (typeof define === 'function' && define.amd) {
    define([], function() { return GifExporter; });
} else {
    window.GifExporter = GifExporter;
}


```

## Source file: characterLoader.js

{
  "path": "characterLoader.js",
  "blob_sha": "a73219182b7c967acfd1cd0a389134b4d75c0665",
  "sha256": "6e5748317460b705fc24c5df50e3d13792618e26326a26939adb0b8444d2631a",
  "lines": 30,
  "bytes": 1245,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/sjefvanleeuwen/sprite-sheet-creator/blob/eceafa51596850082f2e377ab4c37d8ef875c9c5/characterLoader.js",
  "function_signature_candidates": []
}

```text

window.addEventListener('sceneReady', () => {
    document.getElementById('characterUpload').addEventListener('change', async (event) => {
        const file = event.target.files[0];
        if (!file) return;

        const statusMessage = document.getElementById('statusMessage');
        statusMessage.textContent = 'Loading character...';

        try {
            const scene = window.scene;
            if (!scene) throw new Error('Scene not initialized');

            const arrayBuffer = await file.arrayBuffer();
            const blob = new Blob([arrayBuffer], { type: 'model/gltf-binary' });
            const blobUrl = URL.createObjectURL(blob);

            BABYLON.SceneLoader.LoadAssetContainer("", blobUrl, scene, function (container) {
                window.replaceCharacter(container);
                statusMessage.textContent = 'Character loaded successfully!';
                URL.revokeObjectURL(blobUrl);
            }, null, function (error) {
                statusMessage.textContent = 'Error loading character: ' + error.message;
                URL.revokeObjectURL(blobUrl);
            }, ".glb");

        } catch (error) {
            statusMessage.textContent = 'Error: ' + error.message;
        }
    });
});


```
