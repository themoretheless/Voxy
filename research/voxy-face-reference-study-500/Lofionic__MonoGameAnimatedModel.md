# Facial rendering source scan: Lofionic/MonoGameAnimatedModel

Repository: https://github.com/Lofionic/MonoGameAnimatedModel

Pinned source revision: c1457e9d2bd8649f3fbdce43b7a13757f05d4448

Discovery topic: skin shader

Description: Example implementation of model animation in MonoGame using a skinned effect shader.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["AnimatedModelLib/Effects/Source/Skinning.fxh", "AnimatedModelPipelineExtension/SkinnedModelProcessoor.cs"], "shader": ["AnimatedModelLib/Effects/AnimatedModelShader.dx11.mgfxo", "AnimatedModelLib/Effects/AnimatedModelShader.ogl.mgfxo", "AnimatedModelLib/Effects/Source/AnimatedModelShader.fx"], "lip": ["AnimatedModelLib/AnimationClip.cs"]}

## Source file: AnimatedModelLib/AnimationClip.cs

{
  "path": "AnimatedModelLib/AnimationClip.cs",
  "blob_sha": "22898202dfeb0350bd60ed5716f55ad6c8ddf7d5",
  "sha256": "706060b259efdc42202b13d66151b230584b6757b444bc2397342713d210d217",
  "lines": 58,
  "bytes": 1697,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 9
  },
  "url": "https://github.com/Lofionic/MonoGameAnimatedModel/blob/c1457e9d2bd8649f3fbdce43b7a13757f05d4448/AnimatedModelLib/AnimationClip.cs",
  "function_signature_candidates": []
}

```text

﻿using System;
using Microsoft.Xna.Framework.Content;

namespace Lofionic.AnimatedModel {

    public class AnimationClip {

        private TimeSpan duration;
        private TimeSpan offset = TimeSpan.Zero;

        public AnimationClip(TimeSpan duration, Keyframe[] keyframes) {
            this.duration = duration;
            Keyframes = keyframes;
        }

        public AnimationClip(string name, TimeSpan duration, Keyframe[] keyframes) {
            Name = name;
            this.duration = duration;
            Keyframes = keyframes;
        }

        private AnimationClip(TimeSpan offset, TimeSpan duration, Keyframe[] keyframes) {
            this.offset = offset;
            this.duration = duration;
            Keyframes = keyframes;
        }

        public AnimationClip Divide(int startFrame, int endFrame, int totalFrames) {
            double clipLengthMs = Duration.TotalMilliseconds;
            double frameLengthMs = clipLengthMs / totalFrames;

            return new AnimationClip(
                TimeSpan.FromMilliseconds(frameLengthMs * startFrame),
                TimeSpan.FromMilliseconds(frameLengthMs * (endFrame - startFrame)),
                Keyframes);
        }

        private AnimationClip() { }

        [ContentSerializer]
        public string Name { get; set; }

        [ContentSerializer]
        public TimeSpan Duration {
            get => duration;
            set => duration = value;
        }

        [ContentSerializer(Optional = true)]
        public TimeSpan Offset {
            get => offset;
            set => offset = value;
        }

        [ContentSerializer]
        public Keyframe[] Keyframes { get; set; }
    }
}


```

## Source file: AnimatedModelPipelineExtension/SkinnedModelProcessoor.cs

{
  "path": "AnimatedModelPipelineExtension/SkinnedModelProcessoor.cs",
  "blob_sha": "caf130249616d0208da73b8922869445554fe177",
  "sha256": "144c72997b36c982d1bc600988df1b54f8243ae1e96496c33e8267a59691c1d3",
  "lines": 123,
  "bytes": 4785,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 2,
    "lip": 4
  },
  "url": "https://github.com/Lofionic/MonoGameAnimatedModel/blob/c1457e9d2bd8649f3fbdce43b7a13757f05d4448/AnimatedModelPipelineExtension/SkinnedModelProcessoor.cs",
  "function_signature_candidates": [
    {
      "line": 67,
      "text": "private static void FlattenTransforms(NodeContent node, BoneContent skeleton) {"
    }
  ]
}

```text

﻿using System.Collections.Generic;
using Microsoft.Xna.Framework;
using Microsoft.Xna.Framework.Content.Pipeline;
using Microsoft.Xna.Framework.Content.Pipeline.Graphics;
using Microsoft.Xna.Framework.Content.Pipeline.Processors;

using Lofionic.AnimatedModel;

namespace Lofionic.AnimatedModelPipelineExtension {

    [ContentProcessor(DisplayName = "Skinned Model Processor")]
    public class SkinnedModelProcessor : ModelProcessor {

        public override ModelContent Process(NodeContent input, ContentProcessorContext context) {

            AnimatedModelData animatedModelData = ExtractSkeletonAndAnimations(input, context);

            Dictionary<string, object> dictionary = new Dictionary<string, object> {
                { "AnimatedModelData", animatedModelData }
            };

            ModelContent modelContent = base.Process(input, context);
            modelContent.Tag = dictionary;

            return modelContent;
        }

        private AnimatedModelData ExtractSkeletonAndAnimations(NodeContent input, ContentProcessorContext context) {
            // Find the root node
            BoneContent skeleton = MeshHelper.FindSkeleton(input);

            // Bake local transform into skeleton
            FlattenTransforms(input, skeleton);

            // Flatten bone hierarchy into list
            IList<BoneContent> boneList = MeshHelper.FlattenSkeleton(skeleton);

            // Create arrays bind pose, inverse bind pose, and parent for each bone
            Matrix[] bonesBindPose = new Matrix[boneList.Count];
            Matrix[] bonesInverseBindPose = new Matrix[boneList.Count];
            int[] bonesParentIndex = new int[boneList.Count];

            // Indexed list of bone names
            List<string> boneNameList = new List<string>(boneList.Count);

            for (int i = 0; i < boneList.Count; i++) {
                bonesBindPose[i] = boneList[i].Transform;
                bonesInverseBindPose[i] = Matrix.Invert(boneList[i].AbsoluteTransform);

                // Look up index of parent bone using its name
                bonesParentIndex[i] = boneNameList.IndexOf(boneList[i].Parent.Name);

                boneNameList.Add(boneList[i].Name);
            }

            // Extract all animations
            AnimationClip[] animations = ExtractAnimations(
                skeleton.Animations, boneNameList, context);

            return new AnimatedModelData(
                bonesBindPose,
                bonesInverseBindPose,
                bonesParentIndex,
                animations);
        }

        private static void FlattenTransforms(NodeContent node, BoneContent skeleton) {
            foreach (NodeContent child in node.Children) {
                // Don't process the skeleton, because that is special.
                if (child == skeleton) {
                    continue;
                }

                // Bake the local transform into the actual geometry.
                MeshHelper.TransformScene(child, child.Transform);

                // Having baked it, we can now set the local
                // coordinate system back to identity.
                child.Transform = Matrix.Identity;

                // Recurse.
                FlattenTransforms(child, skeleton);
            }
        }

        private AnimationClip[] ExtractAnimations(
            AnimationContentDictionary animationDictionary,
            List<string> boneNameList,
            ContentProcessorContext context) {
            if (animationDictionary.Count < 1) {
                context.Logger.LogImportantMessage("Warning: No animations found.");
            }

            AnimationClip[] animations = new AnimationClip[animationDictionary.Count];

            int animationCount = 0;
            foreach (AnimationContent animationContent in animationDictionary.Values) {
                List<Keyframe> keyframes = new List<Keyframe>();

                // Each bone has its own channel
                foreach (string bone in animationContent.Channels.Keys) {
                    AnimationChannel animationChannel = animationContent.Channels[bone];
                    int boneIndex = boneNameList.IndexOf(bone);

                    foreach (AnimationKeyframe keyframe in animationChannel) {
                        keyframes.Add(new Keyframe(
                            keyframe.Time, boneIndex, keyframe.Transform));
                    }
                }

                // Sort all animation frames by time
                keyframes.Sort();

                animations[animationCount++] = new AnimationClip(
                    animationContent.Name,
                    animationContent.Duration,
                    keyframes.ToArray());
            }

            return animations;
        }
    }
}


```

## Source file: AnimatedModelLib/Keyframe.cs

{
  "path": "AnimatedModelLib/Keyframe.cs",
  "blob_sha": "9f758e73bba30d10ab94b4b792c6107aa479430f",
  "sha256": "d6b2895c6aa54292705d0084f715f496404b2f527e0e2d0602e88ccd2218bc94",
  "lines": 43,
  "bytes": 1065,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1
  },
  "url": "https://github.com/Lofionic/MonoGameAnimatedModel/blob/c1457e9d2bd8649f3fbdce43b7a13757f05d4448/AnimatedModelLib/Keyframe.cs",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "public int CompareTo(object obj) {"
    }
  ]
}

```text

﻿using System;
using Microsoft.Xna.Framework;
using Microsoft.Xna.Framework.Content;

namespace Lofionic.AnimatedModel {

    public class Keyframe : IComparable {
        private TimeSpan time;
        private Matrix transform;

        [ContentSerializer]
        public TimeSpan Time {
            get => time;
            set => time = value;
        }

        [ContentSerializer]
        public int Bone { get; set; }

        [ContentSerializer]
        public Matrix Transform {
            get => transform;
            set => transform = value;
        }

        public Keyframe(TimeSpan time, int boneIndex, Matrix transform) {
            this.time = time;
            Bone = boneIndex;
            this.transform = transform;
        }

        public int CompareTo(object obj) {
            Keyframe keyframe = obj as Keyframe;
            if (obj == null) {
                throw new ArgumentException("Object is not a Keyframe.");
            }

            return time.CompareTo(keyframe.Time);
        }

        private Keyframe() { }
    }
}


```

## Source file: AnimatedModelLib/AnimatedModel.cs

{
  "path": "AnimatedModelLib/AnimatedModel.cs",
  "blob_sha": "2ffc9846e6a6457fee934b6c2fdd75e7b3aef887",
  "sha256": "0c0e98e2ba702236c4b6097a2075fd617b000fadec1e225abd551aaa98653cab",
  "lines": 25,
  "bytes": 814,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 5
  },
  "url": "https://github.com/Lofionic/MonoGameAnimatedModel/blob/c1457e9d2bd8649f3fbdce43b7a13757f05d4448/AnimatedModelLib/AnimatedModel.cs",
  "function_signature_candidates": []
}

```text

﻿using System.Collections.Generic;
using Microsoft.Xna.Framework.Graphics;

namespace Lofionic.AnimatedModel {

    public sealed class AnimatedModel {

        public readonly Model model;
        public readonly Dictionary<string, AnimationClip> clips;

        internal readonly AnimatedModelData animationData;
        
        internal AnimatedModel(Model model, AnimationClip[] clips, string[] clipNames) {
            this.model = model;

            Dictionary<string, object> tag = model.Tag as Dictionary<string, object>;
            animationData = tag["AnimatedModelData"] as AnimatedModelData;

            this.clips = new Dictionary<string, AnimationClip>();
            for (int i = 0; i < clips.Length; i++) {
                this.clips[clipNames[i]] = clips[i];
            }
        }
    }
}


```
