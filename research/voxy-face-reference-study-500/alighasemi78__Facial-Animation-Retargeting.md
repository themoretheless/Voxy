# Facial rendering source scan: alighasemi78/Facial-Animation-Retargeting

Repository: https://github.com/alighasemi78/Facial-Animation-Retargeting

Pinned source revision: 955424e9a43ef13aae629b621619626ce32b74ad

Discovery topic: facial retargeting

Description: Elective in AI 1 course project

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["assets/blendshape_results.png", "generating_blendshape_weights.py", "generating_images_from_blendshape_weights.py", "training_blendshape_prediction.py"]}

## Source file: generating_blendshape_weights.py

{
  "path": "generating_blendshape_weights.py",
  "blob_sha": "a809b9b724bf158aa7c5c06707e0a57d95b0bba5",
  "sha256": "8b93a9b59ee613199436be1f020d27d6963f736e00275c4aef357408c1852b70",
  "lines": 176,
  "bytes": 5295,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 24,
    "gum": 4,
    "face": 19
  },
  "url": "https://github.com/alighasemi78/Facial-Animation-Retargeting/blob/955424e9a43ef13aae629b621619626ce32b74ad/generating_blendshape_weights.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def get_args():"
    },
    {
      "line": 26,
      "text": "def get_blendshapes(face_dir):"
    },
    {
      "line": 77,
      "text": "def get_blendshape_weight(blendshape, device, saved_images_dir, image_name):"
    },
    {
      "line": 128,
      "text": "def save_weights(name, face, data_path, device, saved_images_dir, saved_models_dir):"
    }
  ]
}

```text

# Generating Blendshape Weights

## Importing the Required Libraries
import argparse
import torch
import os
from utilities import create_mesh, create_image
from tqdm import tqdm
from pytorch3d.io import load_ply
import matplotlib.pyplot as plt


def get_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data_dir", type=str, default="COMA_data")
    parser.add_argument(
        "--saved_images_dir", type=str, default="blendshape_weights_images"
    )
    parser.add_argument("--saved_models_dir", type=str, default="blendshape_weights")
    return parser.parse_args()


## Creating the Blendshapes
### From the notebook "Exploring CoMA Data", we understand that in order to find the max expression,
### we need to find the frame with the most displacement in the vertices.
def get_blendshapes(face_dir):
    neutral_verts = None
    neutral_faces = None
    max_expression_verts = []

    expressions = os.listdir(face_dir)
    expressions.sort()

    for expression in tqdm(expressions):
        expression_dir = os.path.join(face_dir, expression)
        frames = os.listdir(expression_dir)
        frames.sort()

        first_frame_path = os.path.join(expression_dir, frames[0])
        first_verts, faces = load_ply(first_frame_path)
        center = first_verts.mean(0)
        first_verts -= center
        scale = max(first_verts.abs().max(0)[0])
        first_verts /= scale

        max_displacement = 0.0
        max_verts = None

        for frame in tqdm(frames, leave=False):
            frame_path = os.path.join(expression_dir, frame)
            verts, _ = load_ply(frame_path)
            center = verts.mean(0)
            verts -= center
            scale = max(verts.abs().max(0)[0])
            verts /= scale

            displacement = torch.norm(verts - first_verts, dim=1).mean().item()
            if displacement > max_displacement:
                max_displacement = displacement
                max_verts = verts

        if neutral_verts is None:
            neutral_verts = first_verts.view(-1, 1)
            neutral_faces = faces

        max_expression_verts.append(max_verts.view(-1, 1))

    max_expression_verts = torch.cat(max_expression_verts, dim=1)
    return {
        "base_faces": neutral_faces,
        "base_blendshape": neutral_verts,
        "delta_blendshapes": max_expression_verts - neutral_verts,
    }


## Generating Blendshape Weights
def get_blendshape_weight(blendshape, device, saved_images_dir, image_name):
    num_meshes = 10_000

    b0 = blendshape["base_blendshape"]
    B = blendshape["delta_blendshapes"]

    num_blendshapes = B.shape[1]

    w = torch.zeros(num_meshes, num_blendshapes)

    # For each mesh, randomly select 1-3 blendshapes to emphasize
    for i in tqdm(range(num_meshes)):
        num_active = torch.randint(1, 4, (1,)).item()  # Choose 1 to 3 blendshapes
        active_indices = torch.randint(
            0, num_blendshapes, (num_active,)
        )  # Indices of active blendshapes
        w[i, active_indices] = (
            torch.rand(num_active) * 0.5 + 0.5
        )  # Values in [0.5, 1.0]

    # Normalize weights so each row sums to 1
    w /= w.sum(dim=1, keepdim=True)

    new_verts = (b0 + B @ w.T).T.view(num_meshes, -1, 3)

    base_faces = blendshape["base_faces"]

    meshes = [create_mesh(verts, base_faces, device) for verts in new_verts[:4]]

    fig, axes = plt.subplots(2, 2, figsize=(10, 10))
    for i, (mesh, ax) in enumerate(zip(meshes, axes.flatten())):
        image = create_image(device, mesh)
        image = image.permute(1, 2, 0).cpu().numpy()

        ax.imshow(image)
        ax.axis("off")

    plt.tight_layout()

    save_path = os.path.join(saved_images_dir, f"{image_name}.png")
    plt.savefig(save_path, bbox_inches="tight", pad_inches=0)
    plt.close(fig)

    return {
        "b0": b0,
        "B": B,
        "base_faces": blendshape["base_faces"],
        "w": w,
    }


def save_weights(name, face, data_path, device, saved_images_dir, saved_models_dir):
    face_dir = os.path.join(data_path, face)

    blendshapes = get_blendshapes(face_dir)

    blendshape_weights = get_blendshape_weight(
        blendshapes, device, saved_images_dir, name
    )

    blendshape_weights_path = os.path.join(saved_models_dir, f"{name}.pt")
    torch.save(blendshape_weights, blendshape_weights_path)


if __name__ == "__main__":
    ## Initial Settings
    if torch.cuda.is_available():
        device = torch.device("cuda:0")
        torch.cuda.set_device(device)
    else:
        device = torch.device("cpu")
        print("WARNING: CPU only, this will be slow!")

    args = get_args()

    data_path = args.data_dir

    saved_images_dir = args.saved_images_dir
    if not os.path.exists(saved_images_dir):
        os.mkdir(saved_images_dir)

    saved_models_dir = args.saved_models_dir
    if not os.path.exists(saved_models_dir):
        os.mkdir(saved_models_dir)

    source = 0
    target = 1

    faces = os.listdir(data_path)
    faces.sort()

    source_face = faces[source]
    target_face = faces[target]

    save_weights(
        "source", source_face, data_path, device, saved_images_dir, saved_models_dir
    )
    save_weights(
        "target", target_face, data_path, device, saved_images_dir, saved_models_dir
    )


```

## Source file: training_blendshape_prediction.py

{
  "path": "training_blendshape_prediction.py",
  "blob_sha": "c2a6dcd984ec0e759a18fc5c371aac98f5f7f897",
  "sha256": "f8c19b77794bf19a2053f52514d9ebaf4e11faf7f26325fec50512d76e93bb5e",
  "lines": 317,
  "bytes": 9964,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 9,
    "gum": 5,
    "face": 5
  },
  "url": "https://github.com/alighasemi78/Facial-Animation-Retargeting/blob/955424e9a43ef13aae629b621619626ce32b74ad/training_blendshape_prediction.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def get_args():"
    },
    {
      "line": 28,
      "text": "def __init__(self, images, weights):"
    },
    {
      "line": 32,
      "text": "def __len__(self):"
    },
    {
      "line": 35,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 44,
      "text": "def __init__(self, lambda_w, lambda_r):"
    },
    {
      "line": 80,
      "text": "def forward(self, x):"
    },
    {
      "line": 83,
      "text": "def loss(self, w_t, I_t, w_t_bar, I_t_hat):"
    },
    {
      "line": 91,
      "text": "def train(train_dataloader, saved_models_dir, device):"
    },
    {
      "line": 143,
      "text": "def test(test_dataloader, saved_model_path, saved_images_dir, device):"
    },
    {
      "line": 184,
      "text": "def get_metrics(test_dataloader, saved_model_path, device):"
    }
  ]
}

```text

# Training Blenshape Prediction

## Importing the Required Libraries
import argparse
import torch
from utilities import create_mesh, create_image
import os
from tqdm import tqdm
from torch.utils.data import Dataset, random_split, DataLoader
import torch.nn as nn
import matplotlib.pyplot as plt
import torch.nn.functional as F


def get_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--saved_images_dir", type=str, default="bpnet_images")
    parser.add_argument("--saved_models_dir", type=str, default="models")
    parser.add_argument("--images_dir", type=str, default="images")
    parser.add_argument(
        "--blendshape_weights_dir", type=str, default="blendshape_weights"
    )
    return parser.parse_args()


## Creating a Custom Dataset
class MyDataset(Dataset):
    def __init__(self, images, weights):
        self.images = images
        self.w = weights

    def __len__(self):
        return len(self.images)

    def __getitem__(self, idx):
        image = self.images[idx]
        w = self.w[idx]

        return image, w


## Defining the Architecture of the BPNet
class BPNet(nn.Module):
    def __init__(self, lambda_w, lambda_r):
        super(BPNet, self).__init__()

        self.lambda_w = lambda_w
        self.lambda_r = lambda_r
        self.model = nn.Sequential(
            nn.Conv2d(3, 16, kernel_size=4, stride=2, padding=1),
            nn.ReLU(),
            nn.Conv2d(16, 32, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(32),
            nn.ReLU(),
            nn.Conv2d(32, 64, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(64),
            nn.ReLU(),
            nn.Conv2d(64, 128, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(128),
            nn.ReLU(),
            nn.Conv2d(128, 256, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(256),
            nn.ReLU(),
            nn.Conv2d(256, 512, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(512),
            nn.ReLU(),
            nn.Flatten(),
            nn.Linear(512 * 2 * 2, 1024),
            nn.ReLU(),
            nn.Linear(1024, 512),
            nn.ReLU(),
            nn.Linear(512, 256),
            nn.ReLU(),
            nn.Linear(256, 128),
            nn.ReLU(),
            nn.Linear(128, 12),
            nn.Softmax(dim=1),
        )

    def forward(self, x):
        return self.model(x)

    def loss(self, w_t, I_t, w_t_bar, I_t_hat):
        loss_w = torch.abs(w_t - w_t_bar).mean()
        loss_r = torch.abs(I_t - I_t_hat).mean()

        return self.lambda_w * loss_w + self.lambda_r * loss_r


## Training the Model
def train(train_dataloader, saved_models_dir, device):
    model = BPNet(1, 1)
    model.to(device)

    optimizer = torch.optim.Adam(model.parameters(), lr=0.0003)

    num_epochs = 8

    for epoch in tqdm(range(num_epochs)):
        epoch_loss = 0.0

        for batch in tqdm(train_dataloader, leave=False):
            I_t = batch[0].to(device)
            w_t = batch[1].to(device)

            # Zero the gradients
            optimizer.zero_grad()

            # Forward pass
            w_t_bar = model(I_t)

            new_verts = (b0 + B @ w_t_bar.T).T.view(w_t_bar.shape[0], -1, 3)

            I_t_hat = []
            for verts in new_verts:
                mesh = create_mesh(verts, base_faces, device)

                image = create_image(device, mesh, True)
                I_t_hat.append(image)
            I_t_hat = torch.stack(I_t_hat)

            # Compute the reconstruction loss
            loss = model.loss(w_t, I_t, w_t_bar, I_t_hat)

            # Backward pass and optimization
            loss.backward()
            optimizer.step()

            # Accumulate batch loss
            epoch_loss += loss.item()

        # Average loss for the epoch
        epoch_loss /= len(train_dataloader)
        print(f"Epoch [{epoch+1}/{num_epochs}], Loss: {epoch_loss:.4f}")

    saved_model_path = os.path.join(saved_models_dir, "bpnet_model.pt")
    torch.save(model, saved_model_path)

    return saved_model_path


## Testing the Trained Model
def test(test_dataloader, saved_model_path, saved_images_dir, device):
    model = torch.load(saved_model_path)
    model.eval()

    with torch.no_grad():
        for batch in test_dataloader:
            I_t = batch[0].to(device)

            # Forward pass
            w_t_bar = model(I_t)

            new_verts = (b0 + B @ w_t_bar.T).T.view(w_t_bar.shape[0], -1, 3)

            I_t_hat = []
            for verts in new_verts:
                mesh = create_mesh(verts, base_faces, device)

                image = create_image(device, mesh)
                I_t_hat.append(image)
            I_t_hat = torch.stack(I_t_hat)

            I_t = I_t.permute(0, 2, 3, 1)
            I_t_hat = I_t_hat.permute(0, 2, 3, 1)

            for i in range(I_t.shape[0]):
                images = [I_t[i].cpu().numpy(), I_t_hat[i].cpu().numpy()]

                fig, axes = plt.subplots(1, 2, figsize=(5, 10))
                for _, (image, ax) in enumerate(zip(images, axes.flatten())):
                    ax.imshow(image)
                    ax.axis("off")

                plt.tight_layout()

                save_path = os.path.join(saved_images_dir, f"test_result_{i}.png")
                plt.savefig(save_path, bbox_inches="tight", pad_inches=0)
                plt.close(fig)

            break


def get_metrics(test_dataloader, saved_model_path, device):
    model = torch.load(saved_model_path)
    model.eval()

    total_mae_weights = 0
    total_mae_images = 0
    num_samples = 0

    with torch.no_grad():
        for batch in test_dataloader:
            I_t = batch[0].to(device)  # Ground truth image
            w_t = batch[1].to(device)  # Ground truth weights

            # Forward pass
            w_t_bar = model(I_t)

            # Compute weight difference
            mae_weights = F.l1_loss(
                w_t_bar, w_t
            ).item()  # Mean Absolute Error (L1 Loss)

            # Generate new vertices from predicted weights
            new_verts = (b0 + B @ w_t_bar.T).T.view(w_t_bar.shape[0], -1, 3)

            # Reconstruct images
            I_t_hat = []
            for verts in new_verts:
                mesh = create_mesh(verts, base_faces, device)
                image = create_image(device, mesh)
                I_t_hat.append(image)
            I_t_hat = torch.stack(I_t_hat)

            # Compute image difference
            mae_images = torch.abs(I_t - I_t_hat).mean().item()

            # Accumulate results
            total_mae_weights += mae_weights
            total_mae_images += mae_images
            num_samples += 1

    avg_mae_weights = total_mae_weights / num_samples
    avg_mae_images = total_mae_images / num_samples

    print(f"Test Set Evaluation Results:")
    print(f"Weight Prediction - Mean Absolute Error (MAE): {avg_mae_weights:.4f}")
    print(f"Image Reconstruction - Mean Absolute Error (MAE): {avg_mae_images:.4f}")


if __name__ == "__main__":
    ## Initial Settings
    if torch.cuda.is_available():
        device = torch.device("cuda:0")
        torch.cuda.set_device(device)
    else:
        device = torch.device("cpu")
        print("WARNING: CPU only, this will be slow!")

    args = get_args()

    saved_images_dir = args.saved_images_dir
    if not os.path.exists(saved_images_dir):
        os.makedirs(saved_images_dir)

    saved_models_dir = args.saved_models_dir
    if not os.path.exists(saved_models_dir):
        os.makedirs(saved_models_dir)

    ## Loading the Images
    images_dir = args.images_dir
    target_images_dir = os.path.join(images_dir, "target")
    images_names = os.listdir(target_images_dir)
    images_names.sort(key=lambda x: int(x.split(".")[0]))

    images = []
    for image in tqdm(images_names):
        image_path = os.path.join(target_images_dir, image)
        images.append(torch.load(image_path))

    images = torch.stack(images)

    print("Images Shape:")
    print(images.shape)

    ## Loading the Blendshape Weights
    blendshape_weights_dir = args.blendshape_weights_dir
    blendshape_weights_path = os.path.join(blendshape_weights_dir, "target.pt")
    blendshape_weights = torch.load(blendshape_weights_path)

    b0 = blendshape_weights["b0"].to(device)
    B = blendshape_weights["B"].to(device)
    base_faces = blendshape_weights["base_faces"].to(device)
    w = blendshape_weights["w"].to(device)

    dataset = MyDataset(images, w)

    train_size = int(0.8 * len(dataset))
    test_size = len(dataset) - train_size
    train_dataset, test_dataset = random_split(dataset, [train_size, test_size])

    train_dataloader = DataLoader(train_dataset, batch_size=5, shuffle=True)
    test_dataloader = DataLoader(test_dataset, batch_size=5, shuffle=False)

    for batch in train_dataloader:
        I_t = batch[0]
        w_t = batch[1]

        new_verts = (b0 + B @ w_t.T).T.view(w_t.shape[0], -1, 3)
        I_t_from_w = []
        for verts in new_verts:
            mesh = create_mesh(verts, base_faces, device)

            image = create_image(device, mesh)
            I_t_from_w.append(image)
        I_t_from_w = torch.stack(I_t_from_w)

        I_t = I_t.permute(0, 2, 3, 1)
        I_t_from_w = I_t_from_w.permute(0, 2, 3, 1)

        images = [I_t[0].cpu().numpy(), I_t_from_w[0].cpu().numpy()]

        fig, axes = plt.subplots(1, 2, figsize=(5, 10))
        for i, (image, ax) in enumerate(zip(images, axes.flatten())):
            ax.imshow(image)
            ax.axis("off")

        plt.tight_layout()
        save_path = os.path.join(saved_images_dir, f"batch_sample.png")
        plt.savefig(save_path, bbox_inches="tight", pad_inches=0)
        plt.close(fig)
        break

    saved_model_path = train(train_dataloader, saved_models_dir, device)
    test(test_dataloader, saved_model_path, saved_images_dir, device)
    get_metrics(test_dataloader, saved_model_path, device)


```

## Source file: generating_images_from_blendshape_weights.py

{
  "path": "generating_images_from_blendshape_weights.py",
  "blob_sha": "57f2a2e6b2ec9ec6d7391c6404c84497567bc36f",
  "sha256": "8e9a397253d32715bd711a95c14082e76d6d32ea511d5d96c9ae59f2ca44111b",
  "lines": 68,
  "bytes": 2091,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 13,
    "gum": 3,
    "face": 2
  },
  "url": "https://github.com/alighasemi78/Facial-Animation-Retargeting/blob/955424e9a43ef13aae629b621619626ce32b74ad/generating_images_from_blendshape_weights.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def get_args():"
    },
    {
      "line": 21,
      "text": "def save_images(name, saved_images_dir, blendshape_weights_dir, device):"
    }
  ]
}

```text

# Generating Images from Blendshape Weights

## Importing the Required Libraries
import argparse
import torch
import os
import shutil
from tqdm import tqdm
from utilities import create_mesh, create_image


def get_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--saved_images_dir", type=str, default="images")
    parser.add_argument(
        "--blendshape_weights_dir", type=str, default="blendshape_weights"
    )
    return parser.parse_args()


def save_images(name, saved_images_dir, blendshape_weights_dir, device):
    ## Loading the Blendshape Weights
    blendshape_weights_path = os.path.join(blendshape_weights_dir, f"{name}.pt")
    blendshape_weights = torch.load(blendshape_weights_path)

    ## Generating the Images for Source and Target Models
    b0 = blendshape_weights["b0"]
    B = blendshape_weights["B"]
    base_faces = blendshape_weights["base_faces"]
    w = blendshape_weights["w"]

    num_meshes = w.shape[0]

    new_verts = (b0 + B @ w.T).T.view(num_meshes, -1, 3)

    images_dir = os.path.join(saved_images_dir, name)
    if not os.path.exists(images_dir):
        os.mkdir(images_dir)

    for i, verts in tqdm(enumerate(new_verts), total=len(new_verts)):
        image_path = os.path.join(images_dir, f"{i}.pt")

        if not os.path.exists(image_path):
            mesh = create_mesh(verts, base_faces, device)

            image = create_image(device, mesh)
            torch.save(image, image_path)


if __name__ == "__main__":
    ## Initial Settings
    if torch.cuda.is_available():
        device = torch.device("cuda:0")
        torch.cuda.set_device(device)
    else:
        device = torch.device("cpu")
        print("WARNING: CPU only, this will be slow!")

    args = get_args()

    saved_images_dir = args.saved_images_dir
    if not os.path.exists(saved_images_dir):
        os.mkdir(saved_images_dir)

    blendshape_weights_dir = args.blendshape_weights_dir

    save_images("source", saved_images_dir, blendshape_weights_dir, device)
    save_images("target", saved_images_dir, blendshape_weights_dir, device)


```

## Source file: pipeline.py

{
  "path": "pipeline.py",
  "blob_sha": "a5f98dd24e4241dd3e9150e7e94634538c96883e",
  "sha256": "4a9cf2868528f77321a987d5a1731aa1b558dc3aea91fdda80fac3ef12ed7ba2",
  "lines": 311,
  "bytes": 9796,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 7,
    "gum": 5,
    "face": 3
  },
  "url": "https://github.com/alighasemi78/Facial-Animation-Retargeting/blob/955424e9a43ef13aae629b621619626ce32b74ad/pipeline.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def get_args():"
    },
    {
      "line": 28,
      "text": "def __init__(self, images):"
    },
    {
      "line": 31,
      "text": "def __len__(self):"
    },
    {
      "line": 34,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 43,
      "text": "def __init__(self, in_channels):"
    },
    {
      "line": 50,
      "text": "def forward(self, x):"
    },
    {
      "line": 73,
      "text": "def __init__(self):"
    },
    {
      "line": 104,
      "text": "def forward(self, x):"
    },
    {
      "line": 109,
      "text": "def __init__(self):"
    },
    {
      "line": 133,
      "text": "def forward(self, x):"
    },
    {
      "line": 139,
      "text": "def __init__(self):"
    },
    {
      "line": 146,
      "text": "def forward(self, I_s, I_t=None):"
    },
    {
      "line": 161,
      "text": "def loss(self, I_s, I_t, I_s_bar, I_t_bar):"
    },
    {
      "line": 170,
      "text": "def __init__(self, lambda_w, lambda_r):"
    },
    {
      "line": 206,
      "text": "def forward(self, x):"
    },
    {
      "line": 209,
      "text": "def loss(self, w_t, I_t, w_t_bar, I_t_hat):"
    },
    {
      "line": 217,
      "text": "def run_pipeline(models_dir, saved_images_dir):"
    }
  ]
}

```text

# Facial Animation Retargeting

## Importing the Required Libraries
import argparse
import torch
from utilities import create_mesh, create_image
import os
from tqdm import tqdm
from torch.utils.data import Dataset, DataLoader
import torch.nn as nn
import torch.nn.functional as F
import matplotlib.pyplot as plt


def get_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--saved_images_dir", type=str, default="pipeline_images")
    parser.add_argument("--images_dir", type=str, default="images")
    parser.add_argument(
        "--blendshape_weights_dir", type=str, default="blendshape_weights"
    )
    parser.add_argument("--models_dir", type=str, default="models")
    return parser.parse_args()


## Creating a Custom Dataset
class MyDataset(Dataset):
    def __init__(self, images):
        self.images = images

    def __len__(self):
        return len(self.images)

    def __getitem__(self, idx):
        image = self.images[idx]

        return image


## Defining the Architectures
### ReenactNet
class SelfAttention(nn.Module):
    def __init__(self, in_channels):
        super(SelfAttention, self).__init__()
        self.query_conv = nn.Conv2d(in_channels, in_channels // 8, kernel_size=1)
        self.key_conv = nn.Conv2d(in_channels, in_channels // 8, kernel_size=1)
        self.value_conv = nn.Conv2d(in_channels, in_channels, kernel_size=1)
        self.gamma = nn.Parameter(torch.zeros(1))

    def forward(self, x):
        batch_size, C, H, W = x.size()

        # Queries, keys, and values
        query = (
            self.query_conv(x).view(batch_size, -1, H * W).permute(0, 2, 1)
        )  # B x (H*W) x C'
        key = self.key_conv(x).view(batch_size, -1, H * W)  # B x C' x (H*W)
        value = self.value_conv(x).view(batch_size, -1, H * W)  # B x C x (H*W)

        # Attention map (softmax applied to key-query dot product)
        attention = F.softmax(torch.bmm(query, key), dim=-1)  # B x (H*W) x (H*W)

        # Weighted value map
        out = torch.bmm(value, attention.permute(0, 2, 1))  # B x C x (H*W)
        out = out.view(batch_size, C, H, W)  # B x C x H x W

        # Scale output
        out = self.gamma * out + x
        return out


class Encoder(nn.Module):
    def __init__(self):
        super(Encoder, self).__init__()

        self.encoder = nn.Sequential(
            nn.Conv2d(3, 16, kernel_size=3, stride=1, padding=1),
            nn.LeakyReLU(0.2),
            nn.BatchNorm2d(16),
            nn.Conv2d(16, 32, kernel_size=3, stride=2, padding=1),
            nn.LeakyReLU(0.2),
            nn.BatchNorm2d(32),
            nn.Conv2d(32, 64, kernel_size=3, stride=2, padding=1),
            nn.LeakyReLU(0.2),
            nn.BatchNorm2d(64),
            SelfAttention(64),
            nn.Conv2d(64, 128, kernel_size=3, stride=2, padding=1),
            nn.LeakyReLU(0.2),
            nn.BatchNorm2d(128),
            nn.Conv2d(128, 256, kernel_size=3, stride=2, padding=1),
            nn.LeakyReLU(0.2),
            nn.BatchNorm2d(256),
            nn.Conv2d(256, 512, kernel_size=3, stride=2, padding=1),
            nn.LeakyReLU(0.2),
            nn.BatchNorm2d(512),
            nn.Flatten(),
            nn.Linear(512 * 4 * 4, 512),
            nn.Linear(512, 8192),
            nn.Unflatten(1, (512, 4, 4)),
            nn.ConvTranspose2d(512, 512, kernel_size=4, stride=2, padding=1),
            nn.LeakyReLU(0.2),
        )

    def forward(self, x):
        return self.encoder(x)


class Decoder(nn.Module):
    def __init__(self):
        super(Decoder, self).__init__()

        self.decoder = nn.Sequential(
            nn.ConvTranspose2d(512, 512, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(512),
            nn.LeakyReLU(0.2),
            nn.ConvTranspose2d(512, 256, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(256),
            nn.LeakyReLU(0.2),
            nn.ConvTranspose2d(256, 128, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(128),
            nn.LeakyReLU(0.2),
            SelfAttention(128),
            nn.ConvTranspose2d(128, 64, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(64),
            nn.LeakyReLU(0.2),
            nn.ConvTranspose2d(64, 32, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(32),
            nn.LeakyReLU(0.2),
            nn.Conv2d(32, 3, kernel_size=4, stride=2, padding=1),
            nn.Sigmoid(),
        )

    def forward(self, x):
        # print("!!!")
        return self.decoder(x)


class ReenactNet(nn.Module):
    def __init__(self):
        super(ReenactNet, self).__init__()

        self.E = Encoder()
        self.D_s = Decoder()
        self.D_t = Decoder()

    def forward(self, I_s, I_t=None):
        if I_t is None:
            z = self.E(I_s)
            I_t_bar = self.D_t(z)

            return I_t_bar

        z_s = self.E(I_s)
        z_t = self.E(I_t)

        I_s_bar = self.D_s(z_s)
        I_t_bar = self.D_t(z_t)

        return I_s_bar, I_t_bar

    def loss(self, I_s, I_t, I_s_bar, I_t_bar):
        l1_loss_s = torch.abs(I_s - I_s_bar).mean()
        l1_loss_t = torch.abs(I_t - I_t_bar).mean()

        return l1_loss_s + l1_loss_t


### BPNet
class BPNet(nn.Module):
    def __init__(self, lambda_w, lambda_r):
        super(BPNet, self).__init__()

        self.lambda_w = lambda_w
        self.lambda_r = lambda_r
        self.model = nn.Sequential(
            nn.Conv2d(3, 16, kernel_size=4, stride=2, padding=1),
            nn.ReLU(),
            nn.Conv2d(16, 32, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(32),
            nn.ReLU(),
            nn.Conv2d(32, 64, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(64),
            nn.ReLU(),
            nn.Conv2d(64, 128, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(128),
            nn.ReLU(),
            nn.Conv2d(128, 256, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(256),
            nn.ReLU(),
            nn.Conv2d(256, 512, kernel_size=4, stride=2, padding=1),
            nn.BatchNorm2d(512),
            nn.ReLU(),
            nn.Flatten(),
            nn.Linear(512 * 2 * 2, 1024),
            nn.ReLU(),
            nn.Linear(1024, 512),
            nn.ReLU(),
            nn.Linear(512, 256),
            nn.ReLU(),
            nn.Linear(256, 128),
            nn.ReLU(),
            nn.Linear(128, 12),
            nn.Softmax(dim=1),
        )

    def forward(self, x):
        return self.model(x)

    def loss(self, w_t, I_t, w_t_bar, I_t_hat):
        loss_w = torch.abs(w_t - w_t_bar).mean()
        loss_r = torch.abs(I_t - I_t_hat).mean()

        return self.lambda_w * loss_w + self.lambda_r * loss_r


## Performing the Method
def run_pipeline(models_dir, saved_images_dir):
    reenactment_path = os.path.join(models_dir, "reenactment_model.pt")
    reenactNet = torch.load(reenactment_path)
    bpnet_path = os.path.join(models_dir, "bpnet_model.pt")
    bpNet = torch.load(bpnet_path)

    reenactNet.eval()
    bpNet.eval()

    with torch.no_grad():
        for I_s in dataloader:
            I_s = I_s.to(device)

            I_t_bar = reenactNet(I_s)
            w_t_bar = bpNet(I_t_bar)

            new_verts = (b0 + B @ w_t_bar.T).T.view(w_t_bar.shape[0], -1, 3)

            I_t_hat = []
            for verts in new_verts:
                mesh = create_mesh(verts, base_faces, device)

                image = create_image(device, mesh)
                I_t_hat.append(image)
            I_t_hat = torch.stack(I_t_hat)

            I_s = I_s.permute(0, 2, 3, 1)
            I_t_bar = I_t_bar.permute(0, 2, 3, 1)
            I_t_hat = I_t_hat.permute(0, 2, 3, 1)

            for i in range(I_s.shape[0]):
                images = [
                    I_s[i].cpu().numpy(),
                    I_t_bar[i].cpu().numpy(),
                    I_t_hat[i].cpu().numpy(),
                ]

                fig, axes = plt.subplots(1, 3, figsize=(5, 15))
                for _, (image, ax) in enumerate(zip(images, axes.flatten())):
                    ax.imshow(image)
                    ax.axis("off")

                plt.tight_layout()

                save_path = os.path.join(saved_images_dir, f"test_result_{i}.png")
                plt.savefig(save_path, bbox_inches="tight", pad_inches=0)
                plt.close(fig)
            break


if __name__ == "__main__":
    ## Initial Settings
    if torch.cuda.is_available():
        device = torch.device("cuda:0")
        torch.cuda.set_device(device)
    else:
        device = torch.device("cpu")
        print("WARNING: CPU only, this will be slow!")

    args = get_args()

    saved_images_dir = args.saved_images_dir
    if not os.path.exists(saved_images_dir):
        os.makedirs(saved_images_dir)

    ## Loading the Images
    images_dir = args.images_dir
    source_images_dir = os.path.join(images_dir, "source")
    images_names = os.listdir(source_images_dir)

    images = []
    for image in tqdm(images_names):
        image_path = os.path.join(source_images_dir, image)
        images.append(torch.load(image_path))

    images = torch.stack(images)

    print("Images Shape:")
    print(images.shape)

    ## Loading the Target Base Faces
    blendshape_weights_dir = args.blendshape_weights_dir
    blendshape_weights_path = os.path.join(blendshape_weights_dir, "target.pt")
    blendshape_weights = torch.load(blendshape_weights_path)

    b0 = blendshape_weights["b0"].to(device)
    B = blendshape_weights["B"].to(device)
    base_faces = blendshape_weights["base_faces"].to(device)

    dataset = MyDataset(images)

    dataloader = DataLoader(dataset, batch_size=16, shuffle=False)

    models_dir = args.models_dir
    run_pipeline(models_dir, saved_images_dir)


```
