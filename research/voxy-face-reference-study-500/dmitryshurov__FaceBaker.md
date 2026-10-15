# Facial rendering source scan: dmitryshurov/FaceBaker

Repository: https://github.com/dmitryshurov/FaceBaker

Pinned source revision: 0fcb13b62ef94e1740894f636001a56d297a7368

Discovery topic: facial rig

Description: Implementation of FaceBaker: Baking Character Facial Rigs with Machine Learning

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["neural_network/face_baker/__init__.py", "neural_network/face_baker/data.py", "neural_network/face_baker/model.py", "neural_network/face_baker/pca.py"]}

## Source file: neural_network/face_baker/pca.py

{
  "path": "neural_network/face_baker/pca.py",
  "blob_sha": "7dfb814f49ef45ec8ca3ec75501a2c25212f0880",
  "sha256": "1a4342a277c5e8d99ddfff8032b7313b1a23a7a13fb9d5be895a4d12f7df979c",
  "lines": 2,
  "bytes": 30,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/dmitryshurov/FaceBaker/blob/0fcb13b62ef94e1740894f636001a56d297a7368/neural_network/face_baker/pca.py",
  "function_signature_candidates": []
}

```text

class PCACalculator:
    pass


```

## Source file: neural_network/face_baker/data.py

{
  "path": "neural_network/face_baker/data.py",
  "blob_sha": "d45d963d4d6ac52408c0157873e95642f12f5073",
  "sha256": "bf1637a4ffee2047d8ebcb842cb12c14981b229eef1803b8c58342af716f55bf",
  "lines": 2,
  "bytes": 27,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/dmitryshurov/FaceBaker/blob/0fcb13b62ef94e1740894f636001a56d297a7368/neural_network/face_baker/data.py",
  "function_signature_candidates": []
}

```text

class DataLoader:
    pass


```

## Source file: neural_network/face_baker/model.py

{
  "path": "neural_network/face_baker/model.py",
  "blob_sha": "e9854f930ba4ab80376a36879e2fea1b52790890",
  "sha256": "809e38abb2cbf453a177a0974bd5155099b34db0cbb83459bb4b6cae12be2c35",
  "lines": 75,
  "bytes": 2914,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 2,
    "face": 1
  },
  "url": "https://github.com/dmitryshurov/FaceBaker/blob/0fcb13b62ef94e1740894f636001a56d297a7368/neural_network/face_baker/model.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def __init__(self,"
    },
    {
      "line": 18,
      "text": "def dense_layer_name(layer_num):"
    },
    {
      "line": 21,
      "text": "def create_model("
    },
    {
      "line": 54,
      "text": "def _add_dense_layer_with_skip_connections("
    },
    {
      "line": 74,
      "text": "def plot_model_to_file(self, file_path):"
    }
  ]
}

```text

from tensorflow import keras


class ModelBuilder:
    def __init__(self,
                 dense_layers_width=256,
                 num_dense_layers=8,
                 leaky_relu_alpha=0.3,        # Since it's not clear from the paper, the default is 0.3
                 skip_connections_type='add'  # Since it's not clear from the paper, this can be 'add' or 'concat'
                 ):
        self.dense_layers_width = dense_layers_width
        self.num_dense_layers = num_dense_layers
        self.leaky_relu_alpha = leaky_relu_alpha
        self.skip_connections_type = skip_connections_type
        self.model = None

    @staticmethod
    def dense_layer_name(layer_num):
        return f'Dense_{layer_num}'

    def create_model(
            self,
            num_rig_control_variables=100,
            num_output_mesh_vertices=1000,
            num_pca_components=50):

        # Add the flat input layer
        input_layer = keras.Input(shape=(num_rig_control_variables,), name="Input_Rig_Control_Variables")

        # Add the first Dense layer
        dense_layer = keras.layers.Dense(self.dense_layers_width, name=self.dense_layer_name(1))(input_layer)

        # We don't need skip connections after the first Dense layer
        skip_layer = None

        # Add the rest 7 layer blocks
        for layer_num in range(2, self.num_dense_layers + 1):
            dense_layer, skip_layer = self._add_dense_layer_with_skip_connections(layer_num, dense_layer, skip_layer)

        # Add missing Leaky ReLU
        leaky_relu = keras.layers.LeakyReLU(alpha=self.leaky_relu_alpha)(skip_layer)

        # Add PCA Dense layer
        pca_layer = keras.layers.Dense(num_pca_components, name="PCA")(leaky_relu)

        # Add output Dense layer
        output_layer = keras.layers.Dense(num_output_mesh_vertices * 3, name="Output_Mesh_Coordinates")(pca_layer)

        # Create the model
        self.model = keras.Model(inputs=input_layer, outputs=output_layer, name="FaceBaker")

        return self.model

    def _add_dense_layer_with_skip_connections(
            self,
            layer_num,
            dense_layer_prev,
            skip_layer_prev):

        """
        Add a Dense - Leaky ReLU - Skip Connection block as in Figure 2
        """

        # After the first Dense layer we don't have a subsequent Add layer, so use that Dense layer instead
        if skip_layer_prev is None:
            skip_layer_prev = dense_layer_prev

        leaky_relu = keras.layers.LeakyReLU(alpha=self.leaky_relu_alpha)(skip_layer_prev)
        dense_layer_new = keras.layers.Dense(self.dense_layers_width, name=self.dense_layer_name(layer_num))(leaky_relu)
        skip_layer_new = keras.layers.Add()([dense_layer_prev, dense_layer_new])

        return dense_layer_new, skip_layer_new

    def plot_model_to_file(self, file_path):
        keras.utils.plot_model(self.model, file_path, show_shapes=True)


```

## Source file: neural_network/face_baker/__init__.py

{
  "path": "neural_network/face_baker/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/dmitryshurov/FaceBaker/blob/0fcb13b62ef94e1740894f636001a56d297a7368/neural_network/face_baker/__init__.py",
  "function_signature_candidates": []
}

```text



```
