# Facial rendering source scan: luoxz-ai/ARFaceDemo

Repository: https://github.com/luoxz-ai/ARFaceDemo

Pinned source revision: 30e1d32661f984c15692ffdf5bc340594881d4ac

Discovery topic: ARKit blendshape

Description: Demonstration of ARKit's face detection, with blendShapes.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["ARFaceDemo/Art/RedEye.scnp"], "face": ["ARFaceDemo.xcodeproj/project.pbxproj", "ARFaceDemo.xcodeproj/project.xcworkspace/contents.xcworkspacedata", "ARFaceDemo.xcodeproj/xcuserdata/joshedson.xcuserdatad/xcschemes/xcschememanagement.plist", "ARFaceDemo/AppDelegate.swift", "ARFaceDemo/Art/RedEye.scnp", "ARFaceDemo/Art/halfred.png", "ARFaceDemo/Art/spark.png", "ARFaceDemo/Assets.xcassets/AppIcon.appiconset/Contents.json", "ARFaceDemo/Base.lproj/LaunchScreen.storyboard", "ARFaceDemo/Base.lproj/Main.storyboard", "ARFaceDemo/Info.plist", "ARFaceDemo/Mask.swift", "ARFaceDemo/ViewController.swift", "ARFaceDemo/VirtualContentUpdater.swift", "ARFaceDemo/VirtualFaceContent.swift"]}

## Source file: ARFaceDemo/VirtualFaceContent.swift

{
  "path": "ARFaceDemo/VirtualFaceContent.swift",
  "blob_sha": "b8b3e91724f7302001ccf89436805009b8eafa32",
  "sha256": "1e87dfcb79d395f18a17108fa36c70ed3b003f6c7b2764c5f6f5599fa8e2f31e",
  "lines": 15,
  "bytes": 314,
  "selection_score": 13,
  "keyword_line_counts": {
    "rig": 1,
    "face": 5
  },
  "url": "https://github.com/luoxz-ai/ARFaceDemo/blob/30e1d32661f984c15692ffdf5bc340594881d4ac/ARFaceDemo/VirtualFaceContent.swift",
  "function_signature_candidates": []
}

```text

//
//  VirtualFaceContent.swift
//  ARFaceDemo
//
//  Created by Josh Edson on 2/26/18.
//  Copyright © 2018 Josh Edson. All rights reserved.
//

import Foundation
import ARKit
protocol VirtualFaceContent {
    func update(with faceAnchor:ARFaceAnchor)
}

typealias VirtualFaceNode = SCNNode & VirtualFaceContent


```

## Source file: ARFaceDemo/Mask.swift

{
  "path": "ARFaceDemo/Mask.swift",
  "blob_sha": "0ae29c1f8fa0b222c8150a9bcf88801ff5d3fdb4",
  "sha256": "286086da63bd4f1cb6e7432aea55d7230d83be72b140d5cefe43380a6cc6a845",
  "lines": 79,
  "bytes": 2336,
  "selection_score": 1,
  "keyword_line_counts": {
    "eye": 4,
    "rig": 1,
    "face": 7
  },
  "url": "https://github.com/luoxz-ai/ARFaceDemo/blob/30e1d32661f984c15692ffdf5bc340594881d4ac/ARFaceDemo/Mask.swift",
  "function_signature_candidates": []
}

```text

//
//  Mask.swift
//  ARFaceDemo
//
//  Created by Josh Edson on 2/26/18.
//  Copyright © 2018 Josh Edson. All rights reserved.
//

import Foundation
import SceneKit
import ARKit
class Mask:SCNNode {
    var nose:SCNNode?
    init(geometry:ARSCNFaceGeometry) {
        
        super.init()
        self.geometry = geometry
        material.lightingModel = .physicallyBased
        material.diffuse.contents = #imageLiteral(resourceName: "halfred.png")
        material.specular.contents = UIColor.white
        createNose()
        createEyes()
    }
    init(device:MTLDevice) {
        super.init()
        let maskGeometry = ARSCNFaceGeometry(device: device)
        self.geometry = maskGeometry
    }
    func createEyes() {
        
        let emitterNode1 = SCNNode(geometry: nil)
        let emitterNode2 = SCNNode(geometry: nil)
        emitterNode1.name = "emitter1"
        emitterNode2.name = "emitter2"
        
        let emitter1 = SCNParticleSystem(named: "RedEye.scnp", inDirectory: nil)
        let emitter2 = SCNParticleSystem(named: "RedEye.scnp", inDirectory: nil)
        
        emitterNode1.position = SCNVector3(-0.032, 0.0275, 0.03)
        emitterNode2.position = SCNVector3(0.032, 0.0275, 0.03)
        
        emitterNode1.addParticleSystem(emitter1!)
        emitterNode2.addParticleSystem(emitter2!)

        self.addChildNode(emitterNode1)
        self.addChildNode(emitterNode2)
    }
    
    func createNose() {
        let sphere = SCNSphere(radius: 0.025)
        sphere.materials.first!.diffuse.contents = UIColor.red
        sphere.materials.first!.lightingModel = .physicallyBased
        let node = SCNNode(geometry: sphere)
        node.position.y = 0.00
        node.position.z = 0.06
        node.renderingOrder = -1
        self.nose = node
        
        if let nose = self.nose {
            self.addChildNode(nose)
        }
 
    }
    required init?(coder aDecoder: NSCoder) {
        fatalError("\(#function) has not been implemented")
    }
    var material:SCNMaterial! {
        return geometry!.materials.first!
    }
}

extension Mask:VirtualFaceContent {
    func update(with faceAnchor: ARFaceAnchor) {
        guard let faceGeometry = self.geometry as? ARSCNFaceGeometry else {
            return
        }
        faceGeometry.update(from: faceAnchor.geometry)
    }
}


```

## Source file: ARFaceDemo/AppDelegate.swift

{
  "path": "ARFaceDemo/AppDelegate.swift",
  "blob_sha": "56f361f67a4bf7446550599dbdc5c7329c71734b",
  "sha256": "465207fd66d9efb721c20c1154131f658394ca7956c2b684116d39140313ad39",
  "lines": 24,
  "bytes": 504,
  "selection_score": 1,
  "keyword_line_counts": {
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/luoxz-ai/ARFaceDemo/blob/30e1d32661f984c15692ffdf5bc340594881d4ac/ARFaceDemo/AppDelegate.swift",
  "function_signature_candidates": []
}

```text

//
//  AppDelegate.swift
//  ARFaceDemo
//
//  Created by Josh Edson on 2/26/18.
//  Copyright © 2018 Josh Edson. All rights reserved.
//

import UIKit

@UIApplicationMain
class AppDelegate: UIResponder, UIApplicationDelegate {

    var window: UIWindow?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplicationLaunchOptionsKey: Any]?) -> Bool {
        // Override point for customization after application launch.
        return true
    }




}



```

## Source file: ARFaceDemo/ViewController.swift

{
  "path": "ARFaceDemo/ViewController.swift",
  "blob_sha": "979f0632c6c891acdf4560dd1c927b1e7c446e7f",
  "sha256": "e1e1d187a345f1d882028f6a636f570118e8a9296b8b72cdfdf41b49fbc6c1bd",
  "lines": 70,
  "bytes": 1797,
  "selection_score": 1,
  "keyword_line_counts": {
    "rig": 1,
    "face": 6
  },
  "url": "https://github.com/luoxz-ai/ARFaceDemo/blob/30e1d32661f984c15692ffdf5bc340594881d4ac/ARFaceDemo/ViewController.swift",
  "function_signature_candidates": []
}

```text

//
//  ViewController.swift
//  ARFaceDemo
//
//  Created by Josh Edson on 2/26/18.
//  Copyright © 2018 Josh Edson. All rights reserved.
//

import UIKit
import SceneKit
import ARKit
class ViewController: UIViewController {


    @IBOutlet weak var sceneView: ARSCNView!
    
    let contentUpdater = VirtualContentUpdater()
    
    var session:ARSession {
        return sceneView.session
    }
    
    override func viewDidLoad() {
        
        super.viewDidLoad()
        sceneView.delegate = contentUpdater
        session.delegate = self
        sceneView.automaticallyUpdatesLighting = true
        createFaceGeometry()
        restartExperience()
    
    }

    func restartExperience() {
        
        guard ARFaceTrackingConfiguration.isSupported else { return }
        let configuration = ARFaceTrackingConfiguration()
        configuration.isLightEstimationEnabled = true
        session.run(configuration, options: [.resetTracking, .removeExistingAnchors])
    }
    
    func createFaceGeometry() {
        
        guard let device = sceneView.device else {
            return
        }
        let mask = Mask(device: device)
        contentUpdater.virtualFaceNode = mask

    }


}

extension ViewController:ARSessionDelegate {
    
    func sessionInterruptionEnded(_ session: ARSession) {
        DispatchQueue.main.async {
            self.restartExperience()
        }
    }
    
    func session(_ session: ARSession, didFailWithError error: Error) {
        guard error is ARError else { return }
        let error = error as NSError
        let messages = [error.localizedDescription, error.localizedFailureReason, error.localizedRecoverySuggestion].flatMap { ($0) }
        let errorString = messages.joined(separator: "\n")
        print(errorString)
    }
}


```
