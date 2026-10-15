# Facial rendering source scan: weijielyu/FaceLift

Repository: https://github.com/weijielyu/FaceLift

Pinned source revision: 0a9420c1480db1e22daa6dee192b76a013295992

Discovery topic: face reconstruction 3D

Description: [ICCV 2025] FaceLift: Learning Generalizable Single Image 3D Face Reconstruction from Synthetic Heads

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["utils_folder/face_utils.py"], "lip": ["mvdiffusion/pipelines/pipeline_mvdiffusion_unclip.py"]}

## Source file: utils_folder/face_utils.py

{
  "path": "utils_folder/face_utils.py",
  "blob_sha": "bb0ae0c6678194ee703c8caa816d64a219bf84c3",
  "sha256": "fb12d7db4951c5c3de319a4d69db4359731ffe921970b068ddd51dd6ac901a4e",
  "lines": 246,
  "bytes": 9414,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 9,
    "face": 54
  },
  "url": "https://github.com/weijielyu/FaceLift/blob/0a9420c1480db1e22daa6dee192b76a013295992/utils_folder/face_utils.py",
  "function_signature_candidates": [
    {
      "line": 53,
      "text": "def select_face(detected_bounding_boxes: Optional[np.ndarray], confidence_scores: Optional[np.ndarray]) -> Optional[np.ndarray]:"
    },
    {
      "line": 79,
      "text": "def crop_face("
    },
    {
      "line": 163,
      "text": "def prepare_foreground_with_rembg(input_image_array: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 190,
      "text": "def preprocess_image("
    },
    {
      "line": 225,
      "text": "def preprocess_image_without_cropping("
    }
  ]
}

```text

# Copyright 2025 Adobe Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
Face detection and cropping utilities for 3D face reconstruction.

This module provides functions for face detection, cropping, and preprocessing
to align faces with training data specifications.
"""

from typing import Tuple, Optional, Dict, Any
import numpy as np
import torch
from PIL import Image
from facenet_pytorch import MTCNN
from rembg import remove

# Training set face parameters (derived from training data statistics)
TRAINING_SET_FACE_SIZE = 194.2749650813705
TRAINING_SET_FACE_CENTER = [251.83270369057132, 280.0133630862363]

# Public constants for external use
FACE_SIZE = TRAINING_SET_FACE_SIZE
FACE_CENTER = TRAINING_SET_FACE_CENTER
DEFAULT_BACKGROUND_COLOR = (255, 255, 255)
DEFAULT_IMG_SIZE = 512

# Device setup
DEVICE = torch.device('cuda' if torch.cuda.is_available() else 'cpu')

# Default face detector instance
FACE_DETECTOR = MTCNN(
    image_size=512, 
    margin=0, 
    min_face_size=20, 
    thresholds=[0.6, 0.7, 0.7], 
    factor=0.709, 
    post_process=True, 
    device=DEVICE
)

def select_face(detected_bounding_boxes: Optional[np.ndarray], confidence_scores: Optional[np.ndarray]) -> Optional[np.ndarray]:
    """
    Select the largest face from detected faces with confidence above threshold.
    
    Args:
        detected_bounding_boxes: Detected bounding boxes in xyxy format
        confidence_scores: Detection confidence probabilities
        
    Returns:
        Selected bounding box or None if no suitable face found
    """
    if detected_bounding_boxes is None or confidence_scores is None:
        return None
        
    # Filter faces with confidence > 0.8
    high_confidence_faces = [
        detected_bounding_boxes[i] for i in range(len(detected_bounding_boxes)) 
        if confidence_scores[i] > 0.8
    ]
    
    if not high_confidence_faces:
        return None

    # Return the largest face (by area)
    return max(high_confidence_faces, key=lambda bbox: (bbox[3] - bbox[1]) * (bbox[2] - bbox[0]))

def crop_face(
    input_image_array: np.ndarray, 
    face_detector: MTCNN = FACE_DETECTOR, 
    target_face_size: float = FACE_SIZE, 
    target_face_center: list = FACE_CENTER, 
    output_image_size: int = 512, 
    background_color: Tuple[int, int, int] = (255, 255, 255)
) -> Tuple[Image.Image, Dict[str, Any]]:
    """
    Crop and align face in image to match training data specifications.
    
    Args:
        input_image_array: Input image as numpy array (H, W, C)
        face_detector: MTCNN face detector instance
        target_face_size: Target face size from training data
        target_face_center: Target face center from training data
        output_image_size: Output image size
        background_color: Background color for padding
        
    Returns:
        Tuple of (cropped_image, crop_parameters)
        
    Raises:
        ValueError: If no face is detected in the image
    """
    image_height, image_width, _ = input_image_array.shape
    
    # Handle RGBA images by compositing with background color
    if input_image_array.shape[2] == 4:
        rgba_pil_image = Image.fromarray(input_image_array)
        background_image = Image.new("RGB", rgba_pil_image.size, background_color)
        rgb_composite_image = Image.alpha_composite(background_image.convert("RGBA"), rgba_pil_image).convert("RGB")
        processed_image_array = np.array(rgb_composite_image)
    else:
        processed_image_array = input_image_array[:, :, :3]  # Ensure RGB format

    # Detect and select face
    detected_bounding_boxes, confidence_scores = face_detector.detect(processed_image_array)
    selected_face_bbox = select_face(detected_bounding_boxes, confidence_scores)
    if selected_face_bbox is None:
        raise ValueError("No face detected in the image")

    # Calculate detected face properties
    detected_face_size = 0.5 * (selected_face_bbox[2] - selected_face_bbox[0] + selected_face_bbox[3] - selected_face_bbox[1])
    detected_face_center = (
        0.5 * (selected_face_bbox[0] + selected_face_bbox[2]), 
        0.5 * (selected_face_bbox[1] + selected_face_bbox[3])
    )

    # Scale image to match training face size
    scale_ratio = target_face_size / detected_face_size
    scaled_width, scaled_height = int(image_width * scale_ratio), int(image_height * scale_ratio)
    scaled_pil_image = Image.fromarray(processed_image_array).resize((scaled_width, scaled_height))
    scaled_face_center = (
        int(detected_face_center[0] * scale_ratio), 
        int(detected_face_center[1] * scale_ratio)
    )

    # Create output image with background
    output_image = Image.new("RGB", (output_image_size, output_image_size), color=background_color)

    # Calculate alignment offsets
    horizontal_offset = target_face_center[0] - scaled_face_center[0]
    vertical_offset = target_face_center[1] - scaled_face_center[1]

    # Calculate crop boundaries
    crop_left_boundary = int(max(0, -horizontal_offset))
    crop_top_boundary = int(max(0, -vertical_offset))
    crop_right_boundary = int(min(scaled_width, output_image_size - horizontal_offset))
    crop_bottom_boundary = int(min(scaled_height, output_image_size - vertical_offset))

    # Crop and paste
    cropped_face_image = scaled_pil_image.crop((crop_left_boundary, crop_top_boundary, crop_right_boundary, crop_bottom_boundary))
    paste_coordinates = (int(max(0, horizontal_offset)), int(max(0, vertical_offset)))
    output_image.paste(cropped_face_image, paste_coordinates)

    crop_parameters = {
        'resize_ratio': scale_ratio,
        'x_offset_left': horizontal_offset,
        'y_offset_top': vertical_offset,
    }

    return output_image, crop_parameters

def prepare_foreground_with_rembg(input_image_array: np.ndarray) -> np.ndarray:
    """
    Prepare foreground image using rembg for background removal.
    
    Args:
        input_image_array: Input image as numpy array (H, W, C)
        
    Returns:
        RGBA image as numpy array with background removed
    """
    pil_image = Image.fromarray(input_image_array)
    background_removed_image = remove(pil_image)
    processed_image_array = np.array(background_removed_image)
    
    # Ensure RGBA format
    if processed_image_array.shape[2] == 4:
        return processed_image_array
    elif processed_image_array.shape[2] == 3:
        height, width = processed_image_array.shape[:2]
        alpha_channel = np.full((height, width), 255, dtype=np.uint8)
        rgba_image = np.zeros((height, width, 4), dtype=np.uint8)
        rgba_image[:, :, :3] = processed_image_array
        rgba_image[:, :, 3] = alpha_channel
        return rgba_image
    
    return processed_image_array

def preprocess_image(
    original_image_array: np.ndarray, 
    target_image_size: int = DEFAULT_IMG_SIZE, 
    background_color: Tuple[int, int, int] = DEFAULT_BACKGROUND_COLOR
) -> Image.Image:
    """
    Preprocess image with background removal and face cropping.
    
    Args:
        original_image_array: Input image as numpy array
        target_image_size: Target image size
        background_color: Background color for compositing
        
    Returns:
        Processed PIL Image
    """
    processed_image_array = prepare_foreground_with_rembg(original_image_array)
    
    # Convert RGBA to RGB with specified background
    if processed_image_array.shape[2] == 4:
        rgba_pil_image = Image.fromarray(processed_image_array)
        background_image = Image.new("RGB", rgba_pil_image.size, background_color)
        rgb_composite_image = Image.alpha_composite(background_image.convert("RGBA"), rgba_pil_image).convert("RGB")
        processed_image_array = np.array(rgb_composite_image)
    
    cropped_image, crop_parameters = crop_face(
        processed_image_array,
        FACE_DETECTOR,
        FACE_SIZE, 
        FACE_CENTER,
        target_image_size, 
        background_color
    )
    return cropped_image

def preprocess_image_without_cropping(
    original_image_array: np.ndarray, 
    target_image_size: int = DEFAULT_IMG_SIZE, 
    background_color: Tuple[int, int, int] = DEFAULT_BACKGROUND_COLOR
) -> Image.Image:
    """
    Preprocess image with background removal, without face cropping.
    
    Args:
        original_image_array: Input image as numpy array
        target_image_size: Target image size
        background_color: Background color for compositing
        
    Returns:
        Processed PIL Image
    """
    processed_image_array = prepare_foreground_with_rembg(original_image_array)
    
    resized_image = Image.fromarray(processed_image_array).resize((target_image_size, target_image_size))
    background_image = Image.new("RGBA", (target_image_size, target_image_size), background_color)
    composite_image = Image.alpha_composite(background_image, resized_image).convert("RGB")
    return composite_image


```

## Source file: mvdiffusion/pipelines/pipeline_mvdiffusion_unclip.py

{
  "path": "mvdiffusion/pipelines/pipeline_mvdiffusion_unclip.py",
  "blob_sha": "b029259ac2113837b7a9110e952e46890edf1c96",
  "sha256": "330a254316665d214b456b20e167a94ce52c89963a62f77b48ab360cc3cb1649",
  "lines": 634,
  "bytes": 31373,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 5,
    "rig": 2,
    "face": 2,
    "lip": 30
  },
  "url": "https://github.com/weijielyu/FaceLift/blob/0a9420c1480db1e22daa6dee192b76a013295992/mvdiffusion/pipelines/pipeline_mvdiffusion_unclip.py",
  "function_signature_candidates": [
    {
      "line": 82,
      "text": "def __init__("
    },
    {
      "line": 116,
      "text": "def enable_vae_slicing(self):"
    },
    {
      "line": 126,
      "text": "def disable_vae_slicing(self):"
    },
    {
      "line": 133,
      "text": "def enable_sequential_cpu_offload(self, gpu_id=0):"
    },
    {
      "line": 159,
      "text": "def _execution_device(self):"
    },
    {
      "line": 177,
      "text": "def _encode_prompt("
    },
    {
      "line": 225,
      "text": "def _encode_image("
    },
    {
      "line": 273,
      "text": "def decode_latents(self, latents):"
    },
    {
      "line": 282,
      "text": "def prepare_extra_step_kwargs(self, generator, eta):"
    },
    {
      "line": 299,
      "text": "def check_inputs("
    },
    {
      "line": 329,
      "text": "def prepare_latents(self, batch_size, num_channels_latents, height, width, dtype, device, generator, latents=None):"
    },
    {
      "line": 347,
      "text": "def noise_image_embeddings("
    },
    {
      "line": 394,
      "text": "def __call__("
    }
  ]
}

```text

# Copyright 2025 Adobe Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# pipeline_mvdiffusion_unclip.py is under the Adobe Research License. Copyright 2025 Adobe Inc.
# Modified from https://github.com/pengHTYX/Era3D/blob/main/mvdiffusion/pipelines/pipeline_mvdiffusion_unclip.py

import inspect
import warnings
from typing import Callable, List, Optional, Union, Dict, Any
import PIL
import torch
from packaging import version
from transformers import CLIPImageProcessor, CLIPVisionModelWithProjection, CLIPFeatureExtractor, CLIPTokenizer, CLIPTextModel
from diffusers.utils.import_utils import is_accelerate_available
from diffusers.configuration_utils import FrozenDict
from diffusers.image_processor import VaeImageProcessor
from diffusers.models import AutoencoderKL, UNet2DConditionModel
from diffusers.models.embeddings import get_timestep_embedding
from diffusers.schedulers import KarrasDiffusionSchedulers
from diffusers.utils import deprecate, logging
from diffusers.utils.torch_utils import randn_tensor
from diffusers.pipelines.pipeline_utils import DiffusionPipeline, ImagePipelineOutput
from diffusers.pipelines.stable_diffusion.stable_unclip_image_normalizer import StableUnCLIPImageNormalizer
import os
import torchvision.transforms.functional as TF
from einops import rearrange
logger = logging.get_logger(__name__)

class StableUnCLIPImg2ImgPipeline(DiffusionPipeline):
    """
    Pipeline for text-guided image to image generation using stable unCLIP.

    This model inherits from [`DiffusionPipeline`]. Check the superclass documentation for the generic methods the
    library implements for all the pipelines (such as downloading or saving, running on a particular device, etc.)

    Args:
        feature_extractor ([`CLIPFeatureExtractor`]):
            Feature extractor for image pre-processing before being encoded.
        image_encoder ([`CLIPVisionModelWithProjection`]):
            CLIP vision model for encoding images.
        image_normalizer ([`StableUnCLIPImageNormalizer`]):
            Used to normalize the predicted image embeddings before the noise is applied and un-normalize the image
            embeddings after the noise has been applied.
        image_noising_scheduler ([`KarrasDiffusionSchedulers`]):
            Noise schedule for adding noise to the predicted image embeddings. The amount of noise to add is determined
            by `noise_level` in `StableUnCLIPPipeline.__call__`.
        tokenizer (`CLIPTokenizer`):
            Tokenizer of class
            [CLIPTokenizer](https://huggingface.co/docs/transformers/v4.21.0/en/model_doc/clip#transformers.CLIPTokenizer).
        text_encoder ([`CLIPTextModel`]):
            Frozen text-encoder.
        unet ([`UNet2DConditionModel`]): Conditional U-Net architecture to denoise the encoded image latents.
        scheduler ([`KarrasDiffusionSchedulers`]):
            A scheduler to be used in combination with `unet` to denoise the encoded image latents.
        vae ([`AutoencoderKL`]):
            Variational Auto-Encoder (VAE) Model to encode and decode images to and from latent representations.
    """
    # image encoding components
    feature_extractor: CLIPFeatureExtractor
    image_encoder: CLIPVisionModelWithProjection
    # image noising components
    image_normalizer: StableUnCLIPImageNormalizer
    image_noising_scheduler: KarrasDiffusionSchedulers
    # regular denoising components
    tokenizer: CLIPTokenizer
    text_encoder: CLIPTextModel
    unet: UNet2DConditionModel
    scheduler: KarrasDiffusionSchedulers
    vae: AutoencoderKL

    def __init__(
        self,
        # image encoding components
        feature_extractor: CLIPFeatureExtractor,
        image_encoder: CLIPVisionModelWithProjection,
        # image noising components
        image_normalizer: StableUnCLIPImageNormalizer,
        image_noising_scheduler: KarrasDiffusionSchedulers,
        # regular denoising components
        tokenizer: CLIPTokenizer,
        text_encoder: CLIPTextModel,
        unet: UNet2DConditionModel,
        scheduler: KarrasDiffusionSchedulers,
        # vae
        vae: AutoencoderKL,
        num_views: int = 6,
    ):
        super().__init__()

        self.register_modules(
            feature_extractor=feature_extractor,
            image_encoder=image_encoder,
            image_normalizer=image_normalizer,
            image_noising_scheduler=image_noising_scheduler,
            tokenizer=tokenizer,
            text_encoder=text_encoder,
            unet=unet,
            scheduler=scheduler,
            vae=vae,
        )
        self.vae_scale_factor = 2 ** (len(self.vae.config.block_out_channels) - 1)
        self.image_processor = VaeImageProcessor(vae_scale_factor=self.vae_scale_factor)
        self.num_views: int = num_views
    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline.enable_vae_slicing
    def enable_vae_slicing(self):
        r"""
        Enable sliced VAE decoding.

        When this option is enabled, the VAE will split the input tensor in slices to compute decoding in several
        steps. This is useful to save some memory and allow larger batch sizes.
        """
        self.vae.enable_slicing()

    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline.disable_vae_slicing
    def disable_vae_slicing(self):
        r"""
        Disable sliced VAE decoding. If `enable_vae_slicing` was previously invoked, this method will go back to
        computing decoding in one step.
        """
        self.vae.disable_slicing()

    def enable_sequential_cpu_offload(self, gpu_id=0):
        r"""
        Offloads all models to CPU using accelerate, significantly reducing memory usage. When called, the pipeline's
        models have their state dicts saved to CPU and then are moved to a `torch.device('meta') and loaded to GPU only
        when their specific submodule has its `forward` method called.
        """
        if is_accelerate_available():
            from accelerate import cpu_offload
        else:
            raise ImportError("Please install accelerate via `pip install accelerate`")

        device = torch.device(f"cuda:{gpu_id}")

        # TODO: self.image_normalizer.{scale,unscale} are not covered by the offload hooks, so they fails if added to the list
        models = [
            self.image_encoder,
            self.text_encoder,
            self.unet,
            self.vae,
        ]
        for cpu_offloaded_model in models:
            if cpu_offloaded_model is not None:
                cpu_offload(cpu_offloaded_model, device)

    @property
    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline._execution_device
    def _execution_device(self):
        r"""
        Returns the device on which the pipeline's models will be executed. After calling
        `pipeline.enable_sequential_cpu_offload()` the execution device can only be inferred from Accelerate's module
        hooks.
        """
        if not hasattr(self.unet, "_hf_hook"):
            return self.device
        for module in self.unet.modules():
            if (
                hasattr(module, "_hf_hook")
                and hasattr(module._hf_hook, "execution_device")
                and module._hf_hook.execution_device is not None
            ):
                return torch.device(module._hf_hook.execution_device)
        return self.device

    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline._encode_prompt
    def _encode_prompt(
        self,
        prompt,
        device,
        num_images_per_prompt,
        do_classifier_free_guidance,
        negative_prompt=None,
        prompt_embeds: Optional[torch.FloatTensor] = None,
        negative_prompt_embeds: Optional[torch.FloatTensor] = None,
        lora_scale: Optional[float] = None,
    ):
        r"""
        Encodes the prompt into text encoder hidden states.

        Args:
             prompt (`str` or `List[str]`, *optional*):
                prompt to be encoded
            device: (`torch.device`):
                torch device
            num_images_per_prompt (`int`):
                number of images that should be generated per prompt
            do_classifier_free_guidance (`bool`):
                whether to use classifier free guidance or not
            negative_prompt (`str` or `List[str]`, *optional*):
                The prompt or prompts not to guide the image generation. If not defined, one has to pass
                `negative_prompt_embeds`. instead. If not defined, one has to pass `negative_prompt_embeds`. instead.
                Ignored when not using guidance (i.e., ignored if `guidance_scale` is less than `1`).
            prompt_embeds (`torch.FloatTensor`, *optional*):
                Pre-generated text embeddings. Can be used to easily tweak text inputs, *e.g.* prompt weighting. If not
                provided, text embeddings will be generated from `prompt` input argument.
            negative_prompt_embeds (`torch.FloatTensor`, *optional*):
                Pre-generated negative text embeddings. Can be used to easily tweak text inputs, *e.g.* prompt
                weighting. If not provided, negative_prompt_embeds will be generated from `negative_prompt` input
                argument.
        """
        prompt_embeds = prompt_embeds.to(dtype=self.text_encoder.dtype, device=device)

        if do_classifier_free_guidance:
            # For classifier free guidance, we need to do two forward passes.
            # Here we concatenate the unconditional and text embeddings into a single batch
            # to avoid doing two forward passes
            # normal_prompt_embeds, color_prompt_embeds = torch.chunk(prompt_embeds, 2, dim=0)
            color_prompt_embeds = prompt_embeds
            
            prompt_embeds = torch.cat([color_prompt_embeds, color_prompt_embeds], 0)

        return prompt_embeds

    def _encode_image(
        self,
        image_pil,
        device,
        num_images_per_prompt,
        do_classifier_free_guidance,
        noise_level: int=0,
        generator: Optional[torch.Generator] = None
    ):
        dtype = next(self.image_encoder.parameters()).dtype
        # ______________________________clip image embedding______________________________ 
        image = self.feature_extractor(images=image_pil, return_tensors="pt").pixel_values
        image = image.to(device=device, dtype=dtype)
        image_embeds = self.image_encoder(image).image_embeds
        
        image_embeds = self.noise_image_embeddings(
            image_embeds=image_embeds,
            noise_level=noise_level,
            generator=generator,
            )
        # duplicate image embeddings for each generation per prompt, using mps friendly method
        # image_embeds = image_embeds.unsqueeze(1)
        # note: the condition input is same
        image_embeds = image_embeds.repeat(num_images_per_prompt, 1)

        if do_classifier_free_guidance:
            # For classifier free guidance, we need to do two forward passes.
            # Here we concatenate the unconditional and text embeddings into a single batch
            # to avoid doing two forward passes
            negative_prompt_embeds = torch.zeros_like(image_embeds)
            image_embeds = torch.cat([negative_prompt_embeds, image_embeds])
            
        # _____________________________vae input latents__________________________________________________
        image_pt = torch.stack([TF.to_tensor(img) for img in image_pil], dim=0).to(device)
        image_pt = image_pt * 2.0 - 1.0
        ###### Fix [RuntimeError: Input type (float) and bias type (c10::Half) should be the same] ######
        image_pt = image_pt.to(torch.float16)
        ###### Fix [RuntimeError: Input type (float) and bias type (c10::Half) should be the same] ######
        image_latents = self.vae.encode(image_pt).latent_dist.mode() * self.vae.config.scaling_factor
        # Note: repeat differently from official pipelines     
        image_latents = image_latents.repeat(num_images_per_prompt, 1, 1, 1)

        if do_classifier_free_guidance:
            image_latents = torch.cat([torch.zeros_like(image_latents), image_latents])

        return image_embeds, image_latents

    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline.decode_latents
    def decode_latents(self, latents):
        latents = 1 / self.vae.config.scaling_factor * latents
        image = self.vae.decode(latents).sample
        image = (image / 2 + 0.5).clamp(0, 1)
        # we always cast to float32 as this does not cause significant overhead and is compatible with bfloat16
        image = image.cpu().permute(0, 2, 3, 1).float().numpy()
        return image

    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline.prepare_extra_step_kwargs
    def prepare_extra_step_kwargs(self, generator, eta):
        # prepare extra kwargs for the scheduler step, since not all schedulers have the same signature
        # eta (η) is only used with the DDIMScheduler, it will be ignored for other schedulers.
        # eta corresponds to η in DDIM paper: https://arxiv.org/abs/2010.02502
        # and should be between [0, 1]

        accepts_eta = "eta" in set(inspect.signature(self.scheduler.step).parameters.keys())
        extra_step_kwargs = {}
        if accepts_eta:
            extra_step_kwargs["eta"] = eta

        # check if the scheduler accepts generator
        accepts_generator = "generator" in set(inspect.signature(self.scheduler.step).parameters.keys())
        if accepts_generator:
            extra_step_kwargs["generator"] = generator
        return extra_step_kwargs

    def check_inputs(
        self,
        prompt,
        image,
        height,
        width,
        callback_steps,
        noise_level,
    ):
        if height % 8 != 0 or width % 8 != 0:
            raise ValueError(f"`height` and `width` have to be divisible by 8 but are {height} and {width}.")

        if (callback_steps is None) or (
            callback_steps is not None and (not isinstance(callback_steps, int) or callback_steps <= 0)
        ):
            raise ValueError(
                f"`callback_steps` has to be a positive integer but is {callback_steps} of type"
                f" {type(callback_steps)}."
            )

        if prompt is not None and (not isinstance(prompt, str) and not isinstance(prompt, list)):
            raise ValueError(f"`prompt` has to be of type `str` or `list` but is {type(prompt)}")


        if noise_level < 0 or noise_level >= self.image_noising_scheduler.config.num_train_timesteps:
            raise ValueError(
                f"`noise_level` must be between 0 and {self.image_noising_scheduler.config.num_train_timesteps - 1}, inclusive."
            )

    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_diffusion.StableDiffusionPipeline.prepare_latents
    def prepare_latents(self, batch_size, num_channels_latents, height, width, dtype, device, generator, latents=None):
        shape = (batch_size, num_channels_latents, height // self.vae_scale_factor, width // self.vae_scale_factor)
        if isinstance(generator, list) and len(generator) != batch_size:
            raise ValueError(
                f"You have passed a list of generators of length {len(generator)}, but requested an effective batch"
                f" size of {batch_size}. Make sure the batch size matches the length of the generators."
            )

        if latents is None:
            latents = randn_tensor(shape, generator=generator, device=device, dtype=dtype)
        else:
            latents = latents.to(device)

        # scale the initial noise by the standard deviation required by the scheduler
        latents = latents * self.scheduler.init_noise_sigma
        return latents

    # Copied from diffusers.pipelines.stable_diffusion.pipeline_stable_unclip.StableUnCLIPPipeline.noise_image_embeddings
    def noise_image_embeddings(
        self,
        image_embeds: torch.Tensor,
        noise_level: int,
        noise: Optional[torch.FloatTensor] = None,
        generator: Optional[torch.Generator] = None,
    ):
        """
        Add noise to the image embeddings. The amount of noise is controlled by a `noise_level` input. A higher
        `noise_level` increases the variance in the final un-noised images.

        The noise is applied in two ways
        1. A noise schedule is applied directly to the embeddings
        2. A vector of sinusoidal time embeddings are appended to the output.

        In both cases, the amount of noise is controlled by the same `noise_level`.

        The embeddings are normalized before the noise is applied and un-normalized after the noise is applied.
        """
        if noise is None:
            noise = randn_tensor(
                image_embeds.shape, generator=generator, device=image_embeds.device, dtype=image_embeds.dtype
            )

        noise_level = torch.tensor([noise_level] * image_embeds.shape[0], device=image_embeds.device)

        image_embeds = self.image_normalizer.scale(image_embeds)

        image_embeds = self.image_noising_scheduler.add_noise(image_embeds, timesteps=noise_level, noise=noise)

        image_embeds = self.image_normalizer.unscale(image_embeds)

        noise_level = get_timestep_embedding(
            timesteps=noise_level, embedding_dim=image_embeds.shape[-1], flip_sin_to_cos=True, downscale_freq_shift=0
        )

        # `get_timestep_embeddings` does not contain any weights and will always return f32 tensors,
        # but we might actually be running in fp16. so we need to cast here.
        # there might be better ways to encapsulate this.
        noise_level = noise_level.to(image_embeds.dtype)

        image_embeds = torch.cat((image_embeds, noise_level), 1)

        return image_embeds

    @torch.no_grad()
    # @replace_example_docstring(EXAMPLE_DOC_STRING)
    def __call__(
        self,
        image: Union[torch.FloatTensor, PIL.Image.Image],
        prompt: Union[str, List[str]],   
        prompt_embeds: torch.FloatTensor = None,
        height: Optional[int] = None,
        width: Optional[int] = None,
        num_inference_steps: int = 20,
        guidance_scale: float = 10,
        negative_prompt: Optional[Union[str, List[str]]] = None,
        num_images_per_prompt: Optional[int] = 1,
        eta: float = 0.0,
        generator: Optional[torch.Generator] = None,
        latents: Optional[torch.FloatTensor] = None,
        negative_prompt_embeds: Optional[torch.FloatTensor] = None,
        output_type: Optional[str] = "pil",
        return_dict: bool = True,
        callback: Optional[Callable[[int, int, torch.FloatTensor], None]] = None,
        callback_steps: int = 1,
        cross_attention_kwargs: Optional[Dict[str, Any]] = None,
        noise_level: int = 0,
        image_embeds: Optional[torch.FloatTensor] = None,
        gt_img_in: Optional[torch.FloatTensor] = None,
    ):
        r"""
        Function invoked when calling the pipeline for generation.

        Args:
            prompt (`str` or `List[str]`, *optional*):
                The prompt or prompts to guide the image generation. If not defined, one has to pass `prompt_embeds`.
                instead.
            image (`torch.FloatTensor` or `PIL.Image.Image`):
                `Image`, or tensor representing an image batch. The image will be encoded to its CLIP embedding which
                the unet will be conditioned on. Note that the image is _not_ encoded by the vae and then used as the
                latents in the denoising process such as in the standard stable diffusion text guided image variation
                process.
            height (`int`, *optional*, defaults to self.unet.config.sample_size * self.vae_scale_factor):
                The height in pixels of the generated image.
            width (`int`, *optional*, defaults to self.unet.config.sample_size * self.vae_scale_factor):
                The width in pixels of the generated image.
            num_inference_steps (`int`, *optional*, defaults to 20):
                The number of denoising steps. More denoising steps usually lead to a higher quality image at the
                expense of slower inference.
            guidance_scale (`float`, *optional*, defaults to 10.0):
                Guidance scale as defined in [Classifier-Free Diffusion Guidance](https://arxiv.org/abs/2207.12598).
                `guidance_scale` is defined as `w` of equation 2. of [Imagen
                Paper](https://arxiv.org/pdf/2205.11487.pdf). Guidance scale is enabled by setting `guidance_scale >
                1`. Higher guidance scale encourages to generate images that are closely linked to the text `prompt`,
                usually at the expense of lower image quality.
            negative_prompt (`str` or `List[str]`, *optional*):
                The prompt or prompts not to guide the image generation. If not defined, one has to pass
                `negative_prompt_embeds`. instead. If not defined, one has to pass `negative_prompt_embeds`. instead.
                Ignored when not using guidance (i.e., ignored if `guidance_scale` is less than `1`).
            num_images_per_prompt (`int`, *optional*, defaults to 1):
                The number of images to generate per prompt.
            eta (`float`, *optional*, defaults to 0.0):
                Corresponds to parameter eta (η) in the DDIM paper: https://arxiv.org/abs/2010.02502. Only applies to
                [`schedulers.DDIMScheduler`], will be ignored for others.
            generator (`torch.Generator` or `List[torch.Generator]`, *optional*):
                One or a list of [torch generator(s)](https://pytorch.org/docs/stable/generated/torch.Generator.html)
                to make generation deterministic.
            latents (`torch.FloatTensor`, *optional*):
                Pre-generated noisy latents, sampled from a Gaussian distribution, to be used as inputs for image
                generation. Can be used to tweak the same generation with different prompts. If not provided, a latents
                tensor will ge generated by sampling using the supplied random `generator`.
            prompt_embeds (`torch.FloatTensor`, *optional*):
                Pre-generated text embeddings. Can be used to easily tweak text inputs, *e.g.* prompt weighting. If not
                provided, text embeddings will be generated from `prompt` input argument.
            negative_prompt_embeds (`torch.FloatTensor`, *optional*):
                Pre-generated negative text embeddings. Can be used to easily tweak text inputs, *e.g.* prompt
                weighting. If not provided, negative_prompt_embeds will be generated from `negative_prompt` input
                argument.
            output_type (`str`, *optional*, defaults to `"pil"`):
                The output format of the generate image. Choose between
                [PIL](https://pillow.readthedocs.io/en/stable/): `PIL.Image.Image` or `np.array`.
            return_dict (`bool`, *optional*, defaults to `True`):
                Whether or not to return a [`~pipelines.stable_diffusion.StableDiffusionPipelineOutput`] instead of a
                plain tuple.
            callback (`Callable`, *optional*):
                A function that will be called every `callback_steps` steps during inference. The function will be
                called with the following arguments: `callback(step: int, timestep: int, latents: torch.FloatTensor)`.
            callback_steps (`int`, *optional*, defaults to 1):
                The frequency at which the `callback` function will be called. If not specified, the callback will be
                called at every step.
            cross_attention_kwargs (`dict`, *optional*):
                A kwargs dictionary that if specified is passed along to the `AttnProcessor` as defined under
                `self.processor` in
                [diffusers.cross_attention](https://github.com/huggingface/diffusers/blob/main/src/diffusers/models/cross_attention.py).
            noise_level (`int`, *optional*, defaults to `0`):
                The amount of noise to add to the image embeddings. A higher `noise_level` increases the variance in
                the final un-noised images. See `StableUnCLIPPipeline.noise_image_embeddings` for details.
            image_embeds (`torch.FloatTensor`, *optional*):
                Pre-generated CLIP embeddings to condition the unet on. Note that these are not latents to be used in
                the denoising process. If you want to provide pre-generated latents, pass them to `__call__` as
                `latents`.

        Examples:

        Returns:
            [`~pipelines.ImagePipelineOutput`] or `tuple`: [`~ pipeline_utils.ImagePipelineOutput`] if `return_dict` is
            True, otherwise a `tuple`. When returning a tuple, the first element is a list with the generated images.
        """
        # 0. Default height and width to unet
        height = height or self.unet.config.sample_size * self.vae_scale_factor
        width = width or self.unet.config.sample_size * self.vae_scale_factor

        # 1. Check inputs. Raise error if not correct
        self.check_inputs(
            prompt=prompt,
            image=image,
            height=height,
            width=width,
            callback_steps=callback_steps,
            noise_level=noise_level
        )

        # 2. Define call parameters
        if isinstance(image, list):
            batch_size = len(image)
        elif isinstance(image, torch.Tensor):
            batch_size = image.shape[0]
            assert batch_size >= self.num_views and batch_size % self.num_views == 0
        elif isinstance(image, PIL.Image.Image):
            image = [image]*self.num_views
            batch_size = self.num_views

        if isinstance(prompt, str):
            prompt = [prompt] * self.num_views

        device = self._execution_device

        # here `guidance_scale` is defined analog to the guidance weight `w` of equation (2)
        # of the Imagen paper: https://arxiv.org/pdf/2205.11487.pdf . `guidance_scale = 1`
        # corresponds to doing no classifier free guidance.
        do_classifier_free_guidance = guidance_scale != 1.0

        # 3. Encode input prompt
        text_encoder_lora_scale = (
            cross_attention_kwargs.get("scale", None) if cross_attention_kwargs is not None else None
        )
        prompt_embeds = self._encode_prompt(
            prompt=prompt,
            device=device,
            num_images_per_prompt=num_images_per_prompt,
            do_classifier_free_guidance=do_classifier_free_guidance,
            negative_prompt=negative_prompt,
            prompt_embeds=prompt_embeds,
            negative_prompt_embeds=negative_prompt_embeds,
            lora_scale=text_encoder_lora_scale,
        )
        

        # 4. Encoder input image
        if isinstance(image, list):
            image_pil = image
        elif isinstance(image, torch.Tensor):
            image_pil = [TF.to_pil_image(image[i]) for i in range(image.shape[0])]
        noise_level = torch.tensor([noise_level], device=device)
        image_embeds, image_latents = self._encode_image(
            image_pil=image_pil,
            device=device,
            num_images_per_prompt=num_images_per_prompt,
            do_classifier_free_guidance=do_classifier_free_guidance,
            noise_level=noise_level,
            generator=generator,
        )

        # 5. Prepare timesteps
        self.scheduler.set_timesteps(num_inference_steps, device=device)
        timesteps = self.scheduler.timesteps

        # 6. Prepare latent variables
        num_channels_latents = self.unet.config.out_channels
        if gt_img_in is not None:
            latents = gt_img_in * self.scheduler.init_noise_sigma
        else:
            latents = self.prepare_latents(
                batch_size=batch_size,
                num_channels_latents=num_channels_latents,
                height=height,
                width=width,
                dtype=prompt_embeds.dtype,
                device=device,
                generator=generator,
                latents=latents,
            )

        # 7. Prepare extra step kwargs. TODO: Logic should ideally just be moved out of the pipeline
        extra_step_kwargs = self.prepare_extra_step_kwargs(generator, eta)

        eles, focals = [], []
        # 8. Denoising loop
        for i, t in enumerate(self.progress_bar(timesteps)):
            if do_classifier_free_guidance:
                latent_model_input = torch.cat([latents, latents], 0)
            else:
                latent_model_input = latents
            latent_model_input = torch.cat([
                    latent_model_input, image_latents
                ], dim=1)
            latent_model_input = self.scheduler.scale_model_input(latent_model_input, t)

            # predict the noise residual
            unet_out = self.unet(
                latent_model_input,
                t,
                encoder_hidden_states=prompt_embeds,
                class_labels=image_embeds,
                cross_attention_kwargs=cross_attention_kwargs,
                return_dict=False)
            
            noise_pred = unet_out
                
            # perform guidance
            if do_classifier_free_guidance:
                noise_pred_uncond, noise_pred_text = noise_pred.chunk(2)
                noise_pred = noise_pred_uncond + guidance_scale * (noise_pred_text - noise_pred_uncond)

            # compute the previous noisy sample x_t -> x_t-1
            latents = self.scheduler.step(noise_pred, t, latents, **extra_step_kwargs, return_dict=False)[0]

            if callback is not None and i % callback_steps == 0:
                callback(i, t, latents)

        # 9. Post-processing
        if not output_type == "latent":
            if num_channels_latents == 8:
                latents = torch.cat([latents[:, :4], latents[:, 4:]], dim=0)
            with torch.no_grad():
                image = self.vae.decode(latents / self.vae.config.scaling_factor, return_dict=False)[0]
        else:
            image = latents

        image = self.image_processor.postprocess(image, output_type=output_type)

        # Offload last model to CPU
        # if hasattr(self, "final_offload_hook") and self.final_offload_hook is not None:
        #     self.final_offload_hook.offload()
        if not return_dict:
            return (image, )
        return ImagePipelineOutput(images=image)


```

## Source file: inference.py

{
  "path": "inference.py",
  "blob_sha": "865cdf6ec31f6f476007b57921ce58d83ebc27d0",
  "sha256": "36d984976a704834a855a27c796aa678e6193346e8f9694b4c7ee6b08a587787",
  "lines": 408,
  "bytes": 15198,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 7,
    "rig": 3,
    "face": 23,
    "lip": 7
  },
  "url": "https://github.com/weijielyu/FaceLift/blob/0a9420c1480db1e22daa6dee192b76a013295992/inference.py",
  "function_signature_candidates": [
    {
      "line": 52,
      "text": "def download_weights_from_hf() -> str:"
    },
    {
      "line": 82,
      "text": "def get_model_paths() -> Tuple[str, str, str]:"
    },
    {
      "line": 92,
      "text": "def initialize_face_detector(device: torch.device) -> MTCNN:"
    },
    {
      "line": 105,
      "text": "def initialize_mvdiffusion_pipeline(mvdiffusion_checkpoint_path: str, device: torch.device):"
    },
    {
      "line": 124,
      "text": "def initialize_gslrm_model(gslrm_checkpoint_path: str, gslrm_config_path: str, device: torch.device):"
    },
    {
      "line": 139,
      "text": "def setup_camera_parameters(device: torch.device) -> Tuple[torch.Tensor, torch.Tensor]:"
    },
    {
      "line": 165,
      "text": "def process_single_image("
    },
    {
      "line": 290,
      "text": "def process_images("
    },
    {
      "line": 323,
      "text": "def main("
    }
  ]
}

```text

# Copyright 2025 Adobe Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
3D Face Reconstruction Inference Pipeline

This module provides a complete pipeline for generating 3D face reconstructions
from single input images using multi-view diffusion and Gaussian splatting.
"""

import os
import yaml
import json
import importlib
import warnings
from typing import List, Tuple, Optional

import torch
import numpy as np
from PIL import Image
from einops import rearrange
from easydict import EasyDict as edict
from rich import print
from rembg import remove
from facenet_pytorch import MTCNN
from huggingface_hub import snapshot_download

from mvdiffusion.pipelines.pipeline_mvdiffusion_unclip import StableUnCLIPImg2ImgPipeline
from gslrm.model.gaussians_renderer import render_turntable, imageseq2video
from utils_folder.face_utils import preprocess_image, preprocess_image_without_cropping

# Suppress FutureWarning from facenet_pytorch
warnings.filterwarnings("ignore", category=FutureWarning, module="facenet_pytorch")

# Configuration constants
DEFAULT_IMG_SIZE = 512
DEFAULT_TURNTABLE_VIEWS = 150
DEFAULT_TURNTABLE_FPS = 30
HF_REPO_ID = "wlyu/OpenFaceLift"

def download_weights_from_hf() -> str:
    """Download model weights from HuggingFace if not already present.
    
    Returns:
        Path to the downloaded repository
    """
    script_directory = os.path.dirname(os.path.abspath(__file__))
    
    # Check if weights already exist locally
    mvdiffusion_path = os.path.join(script_directory, "checkpoints/mvdiffusion/pipeckpts")
    gslrm_path = os.path.join(script_directory, "checkpoints/gslrm/ckpt_0000000000021125.pt")
    prompt_embeds_path = os.path.join(script_directory, "mvdiffusion/data/fixed_prompt_embeds_6view/clr_embeds.pt")
    
    if os.path.exists(mvdiffusion_path) and os.path.exists(gslrm_path) and os.path.exists(prompt_embeds_path):
        print("Using local model weights")
        return script_directory
    
    print(f"Downloading model weights from HuggingFace: {HF_REPO_ID}")
    print("This may take a few minutes on first run...")
    
    # Download to local directory
    cache_dir = snapshot_download(
        repo_id=HF_REPO_ID,
        local_dir=os.path.join(script_directory, "checkpoints"),
        local_dir_use_symlinks=False,
    )
    
    print("Model weights downloaded successfully!")
    return script_directory

def get_model_paths() -> Tuple[str, str, str]:
    """Get paths to model checkpoints and config files."""
    script_directory = download_weights_from_hf()
    mvdiffusion_checkpoint_path = os.path.join(script_directory, "checkpoints/mvdiffusion/pipeckpts")
    gslrm_checkpoint_path = os.path.join(script_directory, "checkpoints/gslrm/ckpt_0000000000021125.pt")
    gslrm_config_path = os.path.join(script_directory, "configs/gslrm.yaml")
    return mvdiffusion_checkpoint_path, gslrm_checkpoint_path, gslrm_config_path



def initialize_face_detector(device: torch.device) -> MTCNN:
    """Initialize face detector."""
    return MTCNN(
        image_size=512, 
        margin=0, 
        min_face_size=20,
        thresholds=[0.6, 0.7, 0.7], 
        factor=0.709,
        post_process=True, 
        device=device
    )


def initialize_mvdiffusion_pipeline(mvdiffusion_checkpoint_path: str, device: torch.device):
    """Initialize MV Diffusion pipeline."""
    script_directory = download_weights_from_hf()
    
    diffusion_pipeline = StableUnCLIPImg2ImgPipeline.from_pretrained(
        mvdiffusion_checkpoint_path,
        torch_dtype=torch.float16,
    )
    diffusion_pipeline.unet.enable_xformers_memory_efficient_attention()
    diffusion_pipeline.to(device)
    random_generator = torch.Generator(device=diffusion_pipeline.unet.device)
    
    color_prompt_embeddings = torch.load(
        os.path.join(script_directory, "mvdiffusion/data/fixed_prompt_embeds_6view/clr_embeds.pt")
    )
    
    return diffusion_pipeline, random_generator, color_prompt_embeddings


def initialize_gslrm_model(gslrm_checkpoint_path: str, gslrm_config_path: str, device: torch.device):
    """Initialize GSLRM model."""
    model_config = edict(yaml.safe_load(open(gslrm_config_path, "r")))
    module_name, class_name = model_config.model.class_name.rsplit(".", 1)
    print(f"Loading model from {module_name} -> {class_name}")
    
    ModelClass = importlib.import_module(module_name).__dict__[class_name]
    gslrm_model = ModelClass(model_config)
    model_checkpoint = torch.load(gslrm_checkpoint_path, map_location="cpu")
    gslrm_model.load_state_dict(model_checkpoint["model"])
    gslrm_model = gslrm_model.to(device)
    
    return gslrm_model


def setup_camera_parameters(device: torch.device) -> Tuple[torch.Tensor, torch.Tensor]:
    """Setup camera parameters for 6 views using local opencv_cameras.json."""
    script_directory = download_weights_from_hf()
    camera_file = os.path.join(script_directory, "utils_folder/opencv_cameras.json")
    
    with open(camera_file, 'r') as f:
        camera_data = json.load(f)["frames"]
    
    # Always use 6 views with indices [2, 1, 0, 5, 4, 3] like in gradio_app.py
    camera_indices = [2, 1, 0, 5, 4, 3]
    selected_cameras = [camera_data[i] for i in camera_indices]

    camera_intrinsics_list, camera_extrinsics_list = [], []
    for camera_frame in selected_cameras:
        camera_intrinsics_list.append(np.array([camera_frame["fx"], camera_frame["fy"], camera_frame["cx"], camera_frame["cy"]]))
        camera_extrinsics_list.append(np.linalg.inv(np.array(camera_frame["w2c"])))
    
    camera_intrinsics_array = np.stack(camera_intrinsics_list, axis=0).astype(np.float32)
    camera_extrinsics_array = np.stack(camera_extrinsics_list, axis=0).astype(np.float32)

    camera_intrinsics_tensor = torch.from_numpy(camera_intrinsics_array).float()[None].to(device)
    camera_extrinsics_tensor = torch.from_numpy(camera_extrinsics_array).float()[None].to(device)
    
    return camera_intrinsics_tensor, camera_extrinsics_tensor


def process_single_image(
    image_file: str,
    input_dir: str,
    output_dir: str,
    auto_crop: bool,
    unclip_pipeline,
    generator: torch.Generator,
    color_prompt_embedding: torch.Tensor,
    gs_lrm_model,
    demo_fxfycxcy: torch.Tensor,
    demo_c2w: torch.Tensor,
    guidance_scale_2D: float,
    step_2D: int,
    face_detector: Optional[MTCNN] = None
) -> None:
    """Process a single image through the 3D reconstruction pipeline."""
    print(f"Processing {image_file}")
    image_name = image_file.split(".")[0]

    input_image = Image.open(os.path.join(input_dir, image_file))
    input_image_np = np.array(input_image)

    demo_output_local_dir = os.path.join(output_dir, image_name)
    os.makedirs(demo_output_local_dir, exist_ok=True)

    # Preprocess image
    try:
        if auto_crop:
            input_image = preprocess_image(input_image_np)
        else:
            input_image = preprocess_image_without_cropping(input_image_np)
    except Exception as e:
        print(f"Failed to process {image_file}: {e}, applying fallback processing")
        try:
            input_image = remove(input_image)
            input_image = input_image.resize((DEFAULT_IMG_SIZE, DEFAULT_IMG_SIZE), Image.LANCZOS)
        except Exception as e2:
            print(f"Background removal also failed: {e2}, using original image")
            input_image = input_image.resize((DEFAULT_IMG_SIZE, DEFAULT_IMG_SIZE), Image.LANCZOS)

    input_image.save(os.path.join(demo_output_local_dir, "input.png"))

    # Generate multi-view images
    mv_imgs = unclip_pipeline(
        input_image, 
        None,
        prompt_embeds=color_prompt_embedding,
        guidance_scale=guidance_scale_2D,
        num_images_per_prompt=1, 
        num_inference_steps=step_2D,
        generator=generator,
        eta=1.0,
    ).images

    # Always use 6 views
    if len(mv_imgs) == 7:
        views = [mv_imgs[i] for i in [1, 2, 3, 4, 5, 6]]
    elif len(mv_imgs) == 6:
        views = [mv_imgs[i] for i in [0, 1, 2, 3, 4, 5]]
    else:
        raise ValueError(f"Unexpected number of views: {len(mv_imgs)}")

    # Save multi-view image
    lrm_input_save = Image.new("RGB", (DEFAULT_IMG_SIZE * len(mv_imgs), DEFAULT_IMG_SIZE))
    for i, view in enumerate(mv_imgs):
        lrm_input_save.paste(view, (DEFAULT_IMG_SIZE * i, 0))
    lrm_input_save.save(os.path.join(demo_output_local_dir, "multiview.png"))

    # Prepare input for 3D reconstruction
    lrm_input = np.stack([np.array(view) for view in views], axis=0)
    lrm_input = torch.from_numpy(lrm_input).float()[None].to(demo_fxfycxcy.device) / 255
    lrm_input = rearrange(lrm_input, "b v h w c -> b v c h w")

    index = torch.stack([
        torch.zeros(lrm_input.size(1)).long(),
        torch.arange(lrm_input.size(1)).long(),
    ], dim=-1)
    demo_index = index[None].to(demo_fxfycxcy.device)

    # Create batch
    batch = edict({
        "image": lrm_input,
        "c2w": demo_c2w,
        "fxfycxcy": demo_fxfycxcy,
        "index": demo_index,
    })

    # 3D reconstruction inference
    with torch.autocast(enabled=True, device_type="cuda", dtype=torch.float16):
        result = gs_lrm_model.forward(batch, create_visual=False, split_data=True)

    # Save Gaussian splatting result
    result.gaussians[0].apply_all_filters(
        opacity_thres=0.04,
        scaling_thres=0.1,
        floater_thres=0.6,
        crop_bbx=[-0.91, 0.91, -0.91, 0.91, -1.0, 1.0],
        cam_origins=None,
        nearfar_percent=(0.0001, 1.0),
    ).save_ply(os.path.join(demo_output_local_dir, "gaussians.ply"))

    # Save rendered output
    comp_image = result.render[0].unsqueeze(0).detach()
    v = comp_image.size(1)
    if v > 10:
        comp_image = comp_image[:, :: v // 10, :, :, :]
    comp_image = rearrange(comp_image, "x v c h w -> (x h) (v w) c")
    comp_image = (comp_image.cpu().numpy() * 255.0).clip(0.0, 255.0).astype(np.uint8)
    Image.fromarray(comp_image).save(os.path.join(demo_output_local_dir, "output.png"))
    
    # Generate turntable video
    vis_image = render_turntable(
        result.gaussians[0],
        rendering_resolution=DEFAULT_IMG_SIZE,
        num_views=DEFAULT_TURNTABLE_VIEWS,
    )
    vis_image = rearrange(vis_image, "h (v w) c -> v h w c", v=DEFAULT_TURNTABLE_VIEWS)
    vis_image = np.ascontiguousarray(vis_image)
    imageseq2video(
        vis_image, 
        os.path.join(demo_output_local_dir, "turntable.mp4"), 
        fps=DEFAULT_TURNTABLE_FPS
    )


def process_images(
    input_dir: str,
    output_dir: str,
    auto_crop: bool,
    unclip_pipeline,
    generator: torch.Generator,
    color_prompt_embedding: torch.Tensor,
    gs_lrm_model,
    demo_fxfycxcy: torch.Tensor,
    demo_c2w: torch.Tensor,
    guidance_scale_2D: float,
    step_2D: int,
    face_detector: Optional[MTCNN] = None
) -> None:
    """Process all images in the input directory."""
    if not os.path.isdir(input_dir):
        raise ValueError(f"Input directory does not exist: {input_dir}")
        
    image_files = sorted(os.listdir(input_dir))
    valid_extensions = ('.png', '.jpg', '.jpeg')
    
    for image_file in image_files:
        if not image_file.lower().endswith(valid_extensions):
            continue
            
        process_single_image(
            image_file, input_dir, output_dir, auto_crop,
            unclip_pipeline, generator, color_prompt_embedding,
            gs_lrm_model, demo_fxfycxcy, demo_c2w,
            guidance_scale_2D, step_2D, face_detector
        )


def main(
    input_dir: str = None,
    output_dir: str = None,
    auto_crop: bool = True,
    seed: int = 4,
    guidance_scale_2D: float = 3.0,
    step_2D: int = 50
) -> None:
    """Main function for 3D face reconstruction inference.
    
    Args:
        input_dir: Input directory containing images (default: examples/)
        output_dir: Output directory for results (default: outputs/)
        auto_crop: Auto crop the face (default: True)
        seed: Random seed for generating multi-view images (default: 4)
        guidance_scale_2D: Guidance scale for generating multi-view images (default: 3.0)
        step_2D: Number of steps for generating multi-view images (default: 50)
    """
    script_directory = os.path.dirname(os.path.abspath(__file__))
    
    # Set default paths if not provided
    if input_dir is None:
        input_dir = os.path.join(script_directory, "examples")
    if output_dir is None:
        output_dir = os.path.join(script_directory, "outputs")
    
    # Setup device and paths
    computation_device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
    mvdiffusion_checkpoint_path, gslrm_checkpoint_path, gslrm_config_path = get_model_paths()
    
    os.makedirs(output_dir, exist_ok=True)

    face_detector = None
    if auto_crop:
        face_detector = initialize_face_detector(computation_device)

    # Initialize models
    diffusion_pipeline, random_generator, color_prompt_embeddings = initialize_mvdiffusion_pipeline(
        mvdiffusion_checkpoint_path, computation_device
    )
    gslrm_model = initialize_gslrm_model(gslrm_checkpoint_path, gslrm_config_path, computation_device)

    # Setup camera parameters (always 6 views)
    camera_intrinsics_tensor, camera_extrinsics_tensor = setup_camera_parameters(computation_device)
    
    # Set random seed
    random_generator.manual_seed(seed)

    # Process images
    process_images(
        input_dir, 
        output_dir, 
        auto_crop,
        diffusion_pipeline,
        random_generator,
        color_prompt_embeddings,
        gslrm_model,
        camera_intrinsics_tensor,
        camera_extrinsics_tensor,
        guidance_scale_2D,
        step_2D,
        face_detector
    )


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="3D Face Reconstruction Inference Pipeline")
    parser.add_argument("--input_dir", "-i", type=str, help="Input directory containing images")
    parser.add_argument("--output_dir", "-o", type=str, help="Output directory for results")
    parser.add_argument("--auto_crop", action="store_true", default=True, help="Auto crop the face")
    parser.add_argument("--seed", type=int, default=4, help="Random seed")
    parser.add_argument("--guidance_scale_2D", type=float, default=3.0, help="Guidance scale")
    parser.add_argument("--step_2D", type=int, default=50, help="Number of diffusion steps")
    
    args = parser.parse_args()
    
    main(
        input_dir=args.input_dir,
        output_dir=args.output_dir,
        auto_crop=args.auto_crop,
        seed=args.seed,
        guidance_scale_2D=args.guidance_scale_2D,
        step_2D=args.step_2D
    )


```

## Source file: gradio_app.py

{
  "path": "gradio_app.py",
  "blob_sha": "64b66495c06945e32c000092416c8fcdc22d7b3a",
  "sha256": "c761c7f4d3c8eea922f9941190f03abcae6b873c490cb5ff4d95425f0872dfc3",
  "lines": 279,
  "bytes": 11274,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 5,
    "face": 16,
    "lip": 3
  },
  "url": "https://github.com/weijielyu/FaceLift/blob/0a9420c1480db1e22daa6dee192b76a013295992/gradio_app.py",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "def download_weights_from_hf() -> Path:"
    },
    {
      "line": 73,
      "text": "def __init__(self):"
    },
    {
      "line": 121,
      "text": "def generate_3d_head(self, image_path, auto_crop=True, guidance_scale=3.0,"
    },
    {
      "line": 235,
      "text": "def main():"
    }
  ]
}

```text

# Copyright 2025 Adobe Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
FaceLift: Single Image 3D Face Reconstruction
Generates 3D head models from single images using multi-view diffusion and GS-LRM.
"""

import json
from pathlib import Path
from datetime import datetime

import gradio as gr
import numpy as np
import torch
import yaml
from easydict import EasyDict as edict
from einops import rearrange
from PIL import Image
from huggingface_hub import snapshot_download

from gslrm.model.gaussians_renderer import render_turntable, imageseq2video
from mvdiffusion.pipelines.pipeline_mvdiffusion_unclip import StableUnCLIPImg2ImgPipeline
from utils_folder.face_utils import preprocess_image, preprocess_image_without_cropping

# HuggingFace repository configuration
HF_REPO_ID = "wlyu/OpenFaceLift"

def download_weights_from_hf() -> Path:
    """Download model weights from HuggingFace if not already present.
    
    Returns:
        Path to the downloaded repository
    """
    workspace_dir = Path(__file__).parent
    
    # Check if weights already exist locally
    mvdiffusion_path = workspace_dir / "checkpoints/mvdiffusion/pipeckpts"
    gslrm_path = workspace_dir / "checkpoints/gslrm/ckpt_0000000000021125.pt"
    prompt_embeds_path = workspace_dir / "mvdiffusion/data/fixed_prompt_embeds_6view/clr_embeds.pt"
    
    if mvdiffusion_path.exists() and gslrm_path.exists() and prompt_embeds_path.exists():
        print("Using local model weights")
        return workspace_dir
    
    print(f"Downloading model weights from HuggingFace: {HF_REPO_ID}")
    print("This may take a few minutes on first run...")
    
    # Download to checkpoints directory
    snapshot_download(
        repo_id=HF_REPO_ID,
        local_dir=str(workspace_dir / "checkpoints"),
        local_dir_use_symlinks=False,
    )
    
    print("Model weights downloaded successfully!")
    return workspace_dir

class FaceLiftPipeline:
    """Pipeline for FaceLift 3D head generation from single images."""
    
    def __init__(self):
        # Download weights from HuggingFace if needed
        workspace_dir = download_weights_from_hf()
        
        # Setup paths
        self.output_dir = workspace_dir / "outputs"
        self.examples_dir = workspace_dir / "examples"
        self.output_dir.mkdir(exist_ok=True)
        
        # Parameters
        self.device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
        self.image_size = 512
        self.camera_indices = [2, 1, 0, 5, 4, 3]
        
        # Load models
        print("Loading models...")
        self.mvdiffusion_pipeline = StableUnCLIPImg2ImgPipeline.from_pretrained(
            str(workspace_dir / "checkpoints/mvdiffusion/pipeckpts"),
            torch_dtype=torch.float16,
        )
        self.mvdiffusion_pipeline.unet.enable_xformers_memory_efficient_attention()
        self.mvdiffusion_pipeline.to(self.device)
        
        with open(workspace_dir / "configs/gslrm.yaml", "r") as f:
            config = edict(yaml.safe_load(f))
        
        module_name, class_name = config.model.class_name.rsplit(".", 1)
        module = __import__(module_name, fromlist=[class_name])
        ModelClass = getattr(module, class_name)
        
        self.gs_lrm_model = ModelClass(config)
        checkpoint = torch.load(
            workspace_dir / "checkpoints/gslrm/ckpt_0000000000021125.pt",
            map_location="cpu"
        )
        self.gs_lrm_model.load_state_dict(checkpoint["model"])
        self.gs_lrm_model.to(self.device)
        
        self.color_prompt_embedding = torch.load(
            workspace_dir / "mvdiffusion/data/fixed_prompt_embeds_6view/clr_embeds.pt",
            map_location=self.device
        )
        
        with open(workspace_dir / "utils_folder/opencv_cameras.json", 'r') as f:
            self.cameras_data = json.load(f)["frames"]
        
        print("Models loaded successfully!")
    
    def generate_3d_head(self, image_path, auto_crop=True, guidance_scale=3.0, 
                         random_seed=4, num_steps=50):
        """Generate 3D head from single image."""
        try:
            # Setup output directory
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            output_dir = self.output_dir / timestamp
            output_dir.mkdir(exist_ok=True)
            
            # Preprocess input
            original_img = np.array(Image.open(image_path))
            input_image = preprocess_image(original_img) if auto_crop else \
                         preprocess_image_without_cropping(original_img)
            
            if input_image.size != (self.image_size, self.image_size):
                input_image = input_image.resize((self.image_size, self.image_size))
            
            input_path = output_dir / "input.png"
            input_image.save(input_path)
            
            # Generate multi-view images
            generator = torch.Generator(device=self.mvdiffusion_pipeline.unet.device)
            generator.manual_seed(random_seed)
            
            result = self.mvdiffusion_pipeline(
                input_image, None,
                prompt_embeds=self.color_prompt_embedding,
                guidance_scale=guidance_scale,
                num_images_per_prompt=1,
                num_inference_steps=num_steps,
                generator=generator,
                eta=1.0,
            )
            
            selected_views = result.images[:6]
            
            # Save multi-view composite
            multiview_image = Image.new("RGB", (self.image_size * 6, self.image_size))
            for i, view in enumerate(selected_views):
                multiview_image.paste(view, (self.image_size * i, 0))
            
            multiview_path = output_dir / "multiview.png"
            multiview_image.save(multiview_path)
            
            # Prepare 3D reconstruction input
            view_arrays = [np.array(view) for view in selected_views]
            lrm_input = torch.from_numpy(np.stack(view_arrays, axis=0)).float()
            lrm_input = lrm_input[None].to(self.device) / 255.0
            lrm_input = rearrange(lrm_input, "b v h w c -> b v c h w")
            
            # Prepare camera parameters
            selected_cameras = [self.cameras_data[i] for i in self.camera_indices]
            fxfycxcy_list = [[c["fx"], c["fy"], c["cx"], c["cy"]] for c in selected_cameras]
            c2w_list = [np.linalg.inv(np.array(c["w2c"])) for c in selected_cameras]
            
            fxfycxcy = torch.from_numpy(np.stack(fxfycxcy_list, axis=0).astype(np.float32))
            c2w = torch.from_numpy(np.stack(c2w_list, axis=0).astype(np.float32))
            fxfycxcy = fxfycxcy[None].to(self.device)
            c2w = c2w[None].to(self.device)
            
            batch_indices = torch.stack([
                torch.zeros(lrm_input.size(1)).long(),
                torch.arange(lrm_input.size(1)).long(),
            ], dim=-1)[None].to(self.device)
            
            batch = edict({
                "image": lrm_input,
                "c2w": c2w,
                "fxfycxcy": fxfycxcy,
                "index": batch_indices,
            })
            
            # Run 3D reconstruction
            with torch.autocast(enabled=True, device_type="cuda", dtype=torch.float16):
                result = self.gs_lrm_model.forward(batch, create_visual=False, split_data=True)
            
            comp_image = result.render[0].unsqueeze(0).detach()
            gaussians = result.gaussians[0]
            
            # Save filtered gaussians
            filtered_gaussians = gaussians.apply_all_filters(
                cam_origins=None,
                opacity_thres=0.04,
                scaling_thres=0.2,
                floater_thres=0.75,
                crop_bbx=[-0.91, 0.91, -0.91, 0.91, -1.0, 1.0],
                nearfar_percent=(0.0001, 1.0),
            )
            
            ply_path = output_dir / "gaussians.ply"
            filtered_gaussians.save_ply(str(ply_path))
            
            # Save output image
            comp_image = rearrange(comp_image, "x v c h w -> (x h) (v w) c")
            comp_image = (comp_image.cpu().numpy() * 255.0).clip(0, 255).astype(np.uint8)
            output_path = output_dir / "output.png"
            Image.fromarray(comp_image).save(output_path)
            
            # Generate turntable video
            turntable_frames = render_turntable(gaussians, rendering_resolution=self.image_size, 
                                               num_views=180)
            turntable_frames = rearrange(turntable_frames, "h (v w) c -> v h w c", v=180)
            turntable_frames = np.ascontiguousarray(turntable_frames)
            
            turntable_path = output_dir / "turntable.mp4"
            imageseq2video(turntable_frames, str(turntable_path), fps=30)
            
            return str(input_path), str(multiview_path), str(output_path), \
                   str(turntable_path), str(ply_path)
            
        except Exception as e:
            raise gr.Error(f"Generation failed: {str(e)}")


def main():
    """Run the FaceLift application."""
    pipeline = FaceLiftPipeline()
    
    # Load examples
    examples = []
    if pipeline.examples_dir.exists():
        examples = [[str(f)] for f in sorted(pipeline.examples_dir.iterdir()) 
                   if f.suffix.lower() in {'.png', '.jpg', '.jpeg'}]
    
    # Create interface
    demo = gr.Interface(
        fn=pipeline.generate_3d_head,
        title="FaceLift: Single Image 3D Face Reconstruction",
        description="""
        Transform a single portrait image into a complete 3D head model.
        
        **Tips:**
        - Use high-quality portrait images with clear facial features
        - If face detection fails, try disabling auto-cropping and manually crop to square
        """,
        inputs=[
            gr.Image(type="filepath", label="Input Portrait Image"),
            gr.Checkbox(value=True, label="Auto Cropping"),
            gr.Slider(1.0, 10.0, 3.0, step=0.1, label="Guidance Scale"),
            gr.Number(value=4, label="Random Seed"),
            gr.Slider(10, 100, 50, step=5, label="Generation Steps"),
        ],
        outputs=[
            gr.Image(label="Processed Input"),
            gr.Image(label="Multi-view Generation"),
            gr.Image(label="3D Reconstruction"),
            gr.Video(label="Turntable Animation"),
            gr.File(label="3D Model (.ply)"),
        ],
        examples=examples,
        allow_flagging="never",
    )
    
    demo.queue(max_size=10)
    demo.launch(share=True, server_name="0.0.0.0", server_port=7860, show_error=True)


if __name__ == "__main__":
    main()

```
