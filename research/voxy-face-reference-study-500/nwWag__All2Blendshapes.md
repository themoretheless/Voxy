# Facial rendering source scan: nwWag/All2Blendshapes

Repository: https://github.com/nwWag/All2Blendshapes

Pinned source revision: a0f1c6db5d25ea8be3ff384423ad6beb00869722

Discovery topic: ARKit blendshape

Description: Generates ARKit Blendshapes from multimodal input with sota transformer encoders.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["arkit/FaceRecorderSwift/FaceRecorderSwift.xcodeproj/project.pbxproj", "arkit/FaceRecorderSwift/FaceRecorderSwift.xcodeproj/project.xcworkspace/contents.xcworkspacedata", "arkit/FaceRecorderSwift/FaceRecorderSwift.xcodeproj/project.xcworkspace/xcshareddata/IDEWorkspaceChecks.plist", "arkit/FaceRecorderSwift/FaceRecorderSwift.xcodeproj/project.xcworkspace/xcuserdata/nicolaswagner.xcuserdatad/UserInterfaceState.xcuserstate", "arkit/FaceRecorderSwift/FaceRecorderSwift.xcodeproj/xcuserdata/nicolaswagner.xcuserdatad/xcdebugger/Breakpoints_v2.xcbkptlist", "arkit/FaceRecorderSwift/FaceRecorderSwift.xcodeproj/xcuserdata/nicolaswagner.xcuserdatad/xcschemes/xcschememanagement.plist", "arkit/FaceRecorderSwift/FaceRecorderSwift/AppDelegate.swift", "arkit/FaceRecorderSwift/FaceRecorderSwift/Assets.xcassets/AccentColor.colorset/Contents.json", "arkit/FaceRecorderSwift/FaceRecorderSwift/Assets.xcassets/AppIcon.appiconset/Contents.json", "arkit/FaceRecorderSwift/FaceRecorderSwift/Assets.xcassets/Contents.json", "arkit/FaceRecorderSwift/FaceRecorderSwift/Base.lproj/LaunchScreen.storyboard", "arkit/FaceRecorderSwift/FaceRecorderSwift/Base.lproj/Main.storyboard", "arkit/FaceRecorderSwift/FaceRecorderSwift/DepthCapture.swift", "arkit/FaceRecorderSwift/FaceRecorderSwift/Info.plist", "arkit/FaceRecorderSwift/FaceRecorderSwift/ViewController.swift", "arkit/FaceRecorderSwift/FaceRecorderSwift/art.scnassets/texture.png"]}

## Source file: task.py

{
  "path": "task.py",
  "blob_sha": "601d94b20d75126f3d819b38d38491bc3221dd2b",
  "sha256": "b10d490ffc9c400ea3cf6fbbb054fc0d1d140a629d160186e10c7ac71e88cab0",
  "lines": 137,
  "bytes": 5630,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 19,
    "face": 1
  },
  "url": "https://github.com/nwWag/All2Blendshapes/blob/a0f1c6db5d25ea8be3ff384423ad6beb00869722/task.py",
  "function_signature_candidates": []
}

```text

import argparse
import logging
from typing_extensions import runtime
import wandb
from huggingface_hub import login
import datetime
logging.basicConfig(level=logging.INFO)
from accelerate import Accelerator

import speech2bs.preprocess as preprocess
import speech2bs.directories as directories
import speech2bs.dataset as dataset
import speech2bs.model as model
import speech2bs.training as training

# ===============================================================================
# Arguments & Login
# ===============================================================================
parser = argparse.ArgumentParser(description="Task Args.")

# Folder
parser.add_argument("-train_folder",
                    type=str,
                    help="Path to the training data folder.")
parser.add_argument("-test_folder",
                    type=str,
                    help="Path to the test data folder.")
parser.add_argument("-output_folder",
                    type=str,
                    default="/data/output/",
                    help="Path to the output data folder.")

# Model
parser.add_argument("-window_size",
                    type=int,
                    default=8,
                    help="Window size for the model input. Default is 8.")

# Training
parser.add_argument("-seed",
                    type=int,
                    default=42,
                    help="Random seed for reproducibility.")
parser.add_argument("-epochs",
                    type=int,
                    default=100,  
                    help="Number of training epochs. Default is 2.")
parser.add_argument("-batch_size",
                    type=int,
                    default=64,
                    help="Batch size for training. Default is 16.")
parser.add_argument("-learning_rate",
                    type=float,
                    default=1e-4,
                    help="Learning rate for the optimizer. Default is 2e-5.")
parser.add_argument("-weight_decay",
                    type=float,
                    default=0.01,   
                    help="Weight decay for the optimizer. Default is 0.01.")
parser.add_argument("--include_video",
                    action='store_true',
                    help="Include video in the training data. Default is False.")
parser.add_argument("--include_text",
                    action='store_true',
                    help="Include text in the training data. Default is False.")
parser.add_argument("--contrastive",
                    action='store_true',
                    help="Use contrastive loss during training. Default is False.")
parser.add_argument("--logging_steps",
                    type=int,
                    default=5,
                    help="Number of steps between logging. Default is 5.")

# Tokens
parser.add_argument("-hf_token",
                    type=str,
                    default=None)      
parser.add_argument("-wandb_token",
                    type=str,
                    default=None)       

# Dataset
parser.add_argument('--recreate', 
                    action='store_true', 
                    help="Do not use cached")
parser.add_argument('-target_size', 
                    type=int, 
                    default=224, 
                    help="Target size for video frames. Default is 224.")



args = parser.parse_args()

if(args.hf_token): login(args.hf_token)
if(args.wandb_token): wandb.login(key=args.wandb_token)

# W&B
run_name = datetime.datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
accelerator = Accelerator()
if accelerator.is_main_process:
    wandb.init(project="speech2bs", config=args, name=run_name)


# ===============================================================================
# Define Vars
# ===============================================================================
train_folder_path, output_train_folder_path, \
    test_folder_path, output_test_folder_path = directories.create_folders(args)

# ===============================================================================
# Preprocessing
# ===============================================================================
train_bs_weights, train_audio, train_video, train_text = preprocess.extract_tensors("train", train_folder_path, output_train_folder_path, args)
test_bs_weights, test_audio, test_video, test_text = preprocess.extract_tensors("test", test_folder_path, output_test_folder_path, args)

# ===============================================================================
# Datasets
# ===============================================================================
training_set = dataset.Speech2BSDataset(train_video, train_audio, train_text, train_bs_weights, window_size=args.window_size)
training_set, validation_set = dataset.get_validation_dataset(training_set, val_size=0.2, seed=args.seed)
test_set = dataset.Speech2BSDataset(test_video, test_audio, test_text, test_bs_weights, window_size=args.window_size)


# ===============================================================================
# Model
# ===============================================================================
sp2bs_model = model.Speech2BsTrans(n_audio_features=train_audio.shape[1],
                            n_output_features=train_bs_weights.shape[1],
                            args=args)
sp2bs_model = model.make_it_qat(sp2bs_model)

# ===============================================================================
# Training
# ===============================================================================
trainer = training.get_trainer(sp2bs_model, training_set, validation_set, output_train_folder_path, args)
trainer.train()

```

## Source file: speech2bs/model.py

{
  "path": "speech2bs/model.py",
  "blob_sha": "ea519760fdc4fa8455c236d606678b5c883c4240",
  "sha256": "10e855654cf540a268757a269dc7b960bfe45df734ce0c3ae858748e84c1eeec",
  "lines": 117,
  "bytes": 4961,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/nwWag/All2Blendshapes/blob/a0f1c6db5d25ea8be3ff384423ad6beb00869722/speech2bs/model.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, n_audio_features, n_output_features, args):"
    },
    {
      "line": 69,
      "text": "def forward(self, x, return_embeddings=False):"
    },
    {
      "line": 114,
      "text": "def make_it_qat(module):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F
import torch.ao.quantization as tq
from transformers import GPT2Config, GPT2Model, VivitConfig, VivitModel

class Speech2BsTrans(nn.Module):
    """Overall model for Speech2Bs, including audio, video and text transformers. Seperate linear regressors for each modality."""
    def __init__(self, n_audio_features, n_output_features, args):
        super().__init__()
        
        # ===============================================================================
        # Audio Transformer
        # ===============================================================================
        # Default gpt2 transformer and linear tokenzier for audio features
        config = GPT2Config(
            vocab_size=2,
            n_positions=args.window_size,
            n_ctx=args.window_size,
            n_embd=64,
            n_layer=2,
            n_head=2
        )
        self.audio_transformer = GPT2Model(config)
        self.tokenizer = nn.Sequential(nn.Linear(n_audio_features, 64),
                                       nn.Sigmoid())
        self.audio_regressor = nn.Sequential(nn.Linear(self.audio_transformer.config.hidden_size, n_output_features),
                                       nn.Sigmoid())
       
        # ===============================================================================
        # Video Transformer
        # ===============================================================================       
        # Default vivit transformer and linear tokenzier for video features to match audio features                         
        self.video_transformer = None
        self.video_hidden_size = 0
        if args.include_video:
            self.video_hidden_size = self.audio_transformer.config.hidden_size
            config = VivitConfig(
                image_size=args.target_size,
                num_frames=args.window_size,
                num_hidden_layers=2,
                num_attention_heads=8,
                hidden_size=self.video_hidden_size
            )
            self.video_transformer = VivitModel(config)
            self.video_regressor = nn.Sequential(nn.Linear(self.video_hidden_size, n_output_features),
                                        nn.Sigmoid())

        # ===============================================================================
        # Text Transformer
        # ===============================================================================
        # Default gpt2 transformer and one hot tokenzier for unicode characters
        self.text_transformer = None
        if args.include_text:
            config = GPT2Config(
                vocab_size=256, 
                n_positions=args.window_size,
                n_ctx=args.window_size,
                n_embd=256,
                n_layer=2,
                n_head=2
            )
            self.text_tokenizer = lambda x: F.one_hot(x, num_classes=256)
            self.text_transformer = GPT2Model(config)
            self.text_down_embed = nn.Linear(256, self.audio_transformer.config.hidden_size)
            self.text_regressor = nn.Sequential(nn.Linear(self.audio_transformer.config.hidden_size, n_output_features),
                                        nn.Sigmoid())
            
    def forward(self, x, return_embeddings=False):
        # Check input modalities
        if len(x) == 2:
            video = x[0]
            audio = x[1]
        elif len(x) == 3:
            video = x[0]
            audio = x[1]
            text = x[2]
        else:
            audio = x


        # Audio
        audio_embeds = self.tokenizer(audio)
        audio_embeds = self.audio_transformer(
            inputs_embeds=audio_embeds
        ).last_hidden_state.mean(dim=1)
        bs_audio = self.audio_regressor(audio_embeds)

        # Video
        bs_video, video_embeds = None, None
        if self.video_transformer is not None:
            video_embeds = self.video_transformer(video).pooler_output
            bs_video = self.video_regressor(video_embeds)

        # Text
        bs_text, text_embeds = None, None
        if self.text_transformer is not None:
            text_embeds = self.text_tokenizer(text)
            text_embeds[text_embeds < 0] = 0
            text_embeds[text_embeds >= 255] = 0
            text_embeds = self.text_transformer(
                inputs_embeds=text_embeds
            ).last_hidden_state.mean(dim=1)
            text_embeds = self.text_down_embed(text_embeds)
            bs_text = self.text_regressor(text_embeds)

        # Return all modalities and set none if not present
        if not return_embeddings:
            return bs_audio, bs_video, bs_text
        else:
            return bs_audio, bs_video, bs_text, audio_embeds, video_embeds, text_embeds


def make_it_qat(module):
    module.qconfig = tq.get_default_qat_qconfig('fbgemm')
    tq.prepare_qat(module, inplace=True)
    return module

```

## Source file: speech2bs/dataset.py

{
  "path": "speech2bs/dataset.py",
  "blob_sha": "5a2c0bec7f0c79da03c704ef4929295bf4f6297b",
  "sha256": "f66f90d32a7a06724926ca982424bd992773efa9c57556a36c41237ab4207947",
  "lines": 59,
  "bytes": 2466,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/nwWag/All2Blendshapes/blob/a0f1c6db5d25ea8be3ff384423ad6beb00869722/speech2bs/dataset.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def __init__(self, video, audio, text, bs_weights, window_size=8):"
    },
    {
      "line": 29,
      "text": "def __len__(self):"
    },
    {
      "line": 32,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 53,
      "text": "def get_validation_dataset(dataset, val_size=0.2, seed=42):"
    }
  ]
}

```text

from torch.utils.data import Dataset, random_split
import torch

class Speech2BSDataset(Dataset):
    def __init__(self, video, audio, text, bs_weights, window_size=8):
        # Three modalities
        self.audio = audio
        self.video = video
        self.text = text
        
        # Check which modalities are present
        self.has_video = False
        self.has_text = False
        if video is not None: self.has_video = True
        if text is not None: self.has_text = True

        # Labels
        self.bs_weights = bs_weights
        # Moving window size
        self.window_size = window_size

        # Dataset must have the same length
        assert len(audio) == len(bs_weights), "Audio and BS weights must have the same length " + str(len(audio)) + " " + str(len(bs_weights))
        if self.has_video:
            assert len(video) == len(audio), "Video and Audio must have the same length " + str(len(video)) + " " + str(len(audio))
        if self.has_text:
            assert len(text) == len(audio), "Text and Audio must have the same length " + str(len(text)) + " " + str(len(audio))
    
    def __len__(self):
        return len(self.bs_weights)
    
    def __getitem__(self, index):
        diff = index - self.window_size
        # Ensure to stay in dataset including window
        if diff < 0:
            index -= diff
        
        # Return depending on available modalities
        if self.has_text and self.has_video:
            return self.video[index-self.window_size:index].float(), \
                   self.audio[index-self.window_size:index].float(), \
                   self.text[index-self.window_size:index].long(), \
                   self.bs_weights[index].float()
        if self.has_video:
            return self.video[index-self.window_size:index].float(), \
                   self.audio[index-self.window_size:index].float(), \
                   self.bs_weights[index].float()
        else:
            return self.audio[index-self.window_size:index].float(), \
                self.bs_weights[index].float()


def get_validation_dataset(dataset, val_size=0.2, seed=42):
    """Create a validation dataset from the given dataset with given seed"""
    generator = torch.Generator().manual_seed(seed)
    total_size = len(dataset)
    val_length = int(total_size * val_size)
    train_length = total_size - val_length
    return random_split(dataset, [train_length, val_length], generator=generator)

```

## Source file: speech2bs/training.py

{
  "path": "speech2bs/training.py",
  "blob_sha": "31e3d1bc0cbf306b6cf3864fdc880a5bdc013561",
  "sha256": "6727d9757e23ed77cc7be57950fb7fdfc9a3f11f750242dfb83f9291af33de4a",
  "lines": 95,
  "bytes": 3743,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "gum": 2
  },
  "url": "https://github.com/nwWag/All2Blendshapes/blob/a0f1c6db5d25ea8be3ff384423ad6beb00869722/speech2bs/training.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self, temperature=0.5):"
    },
    {
      "line": 16,
      "text": "def forward(self, z1, z2):"
    },
    {
      "line": 30,
      "text": "def __init__(self, contrastive, *args, **kwargs):"
    },
    {
      "line": 36,
      "text": "def compute_loss(self, model, inputs, return_outputs=False, num_items_in_batch=None):"
    },
    {
      "line": 65,
      "text": "def prediction_step(self, model, inputs, prediction_loss_only, ignore_keys=None):"
    },
    {
      "line": 70,
      "text": "def get_trainer(model, train_dataset, validation_dataset, output_folder, args):"
    }
  ]
}

```text

from turtle import rt
from transformers import Trainer, TrainingArguments, TrainerCallback
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data.dataloader import default_collate
import wandb


class NTXentLoss(nn.Module):
    """Standard Contrastive loss for self-supervised learning,"""
    def __init__(self, temperature=0.5):
        super().__init__()
        self.temperature = temperature

    def forward(self, z1, z2):
        z1 = F.normalize(z1, dim=1)
        z2 = F.normalize(z2, dim=1)
        batch_size = z1.size(0)
        sim = torch.matmul(z1, z2.T) / self.temperature

        labels = torch.eye(batch_size, device=z1.device)
        
        loss = F.binary_cross_entropy_with_logits(sim, labels)
        return loss

    
class Speech2BsTrainer(Trainer):
    """Custom HF Trainer for Speech2Bs model, including contrastive loss and logging."""
    def __init__(self, contrastive, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.contrastive_loss = NTXentLoss(temperature=0.5) if contrastive else None
        self.trained_steps = 0
        self.logging_steps = kwargs["args"].logging_steps

    def compute_loss(self, model, inputs, return_outputs=False, num_items_in_batch=None):
        """Evaluate model, currently only if all modelitites are present."""
        if len(inputs) == 4:
            outputs_audio, outputs_video, outputs_text, \
                audio_embeds, video_embeds, text_embeds = model(inputs[:3], return_embeddings=True)

        # Reconstruction loss in BS space
        loss = F.mse_loss(outputs_audio, inputs[-1], reduction='mean')
        loss += F.mse_loss(outputs_video, inputs[-1], reduction='mean') if outputs_video is not None else 0
        loss += F.mse_loss(outputs_text, inputs[-1], reduction='mean') if outputs_text is not None else 0
        loss_dict = {"recon_loss": loss}

        # Contrastive loss, only audio pairs
        if self.contrastive_loss is not None:
            contrastive_loss = self.contrastive_loss(audio_embeds, video_embeds) if outputs_video is not None else 0
            contrastive_loss += self.contrastive_loss(audio_embeds, text_embeds) if outputs_text is not None else 0
            loss_dict["contrastive_loss"] = contrastive_loss
            loss += contrastive_loss

        # Log loss
        is_training = self.model.training
        if is_training:
            if self.trained_steps % self.logging_steps == (self.logging_steps - 1): 
                try: wandb.log(loss_dict, step=self.trained_steps+1)
                except: pass
            self.trained_steps += 1

        return (loss, outputs) if return_outputs else loss
    
    def prediction_step(self, model, inputs, prediction_loss_only, ignore_keys=None):
        with torch.no_grad():
            loss, _ = self.compute_loss(model, inputs, return_outputs=True)
        return (loss, None, None)
    
def get_trainer(model, train_dataset, validation_dataset, output_folder, args):
    training_args = TrainingArguments(
    output_dir=output_folder,
    learning_rate=args.learning_rate,
    per_device_train_batch_size=args.batch_size,
    per_device_eval_batch_size=args.batch_size,
    num_train_epochs=args.epochs,
    weight_decay=args.weight_decay,
    eval_strategy="epoch",
    save_strategy="epoch",
    load_best_model_at_end=True,
    save_total_limit=2,
    push_to_hub=False,
    report_to="wandb",
    logging_steps=args.logging_steps
)
    
    trainer = Speech2BsTrainer(
        contrastive=args.contrastive,
        model=model,
        args=training_args,
        train_dataset=train_dataset,
        eval_dataset=validation_dataset,
        data_collator=default_collate     
        )
    return trainer


```
