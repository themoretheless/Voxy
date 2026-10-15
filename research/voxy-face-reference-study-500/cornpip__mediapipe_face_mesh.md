# Facial rendering source scan: cornpip/mediapipe_face_mesh

Repository: https://github.com/cornpip/mediapipe_face_mesh

Pinned source revision: f2d5d1b9d00f5a9463703a9873e3b8e3dce0b658

Discovery topic: ARKit blendshape

Description: Real-time MediaPipe face mesh for Flutter: 478 landmarks with iris, multi-face tracking, ARKit-style blendshapes, and 3D head pose.

Reported license: BSD-3-Clause

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["assets/models/face_blendshapes.tflite", "ios/mediapipe_face_mesh/Sources/mediapipe_face_mesh/mediapipe_face_blendshapes.mm", "macos/mediapipe_face_mesh/Sources/mediapipe_face_mesh/mediapipe_face_blendshapes.mm", "src/mediapipe_face_blendshapes.cc"], "gum": ["src/include/tensorflow/lite/delegates/gpu/cl/cl_arguments.cc", "src/include/tensorflow/lite/delegates/gpu/cl/cl_arguments.h", "src/include/tensorflow/lite/delegates/gpu/cl/cl_arguments_test.cc", "src/include/tensorflow/lite/delegates/gpu/common/task/arguments.cc", "src/include/tensorflow/lite/delegates/gpu/common/task/arguments.h", "src/include/tensorflow/lite/delegates/gpu/metal/metal_arguments.cc", "src/include/tensorflow/lite/delegates/gpu/metal/metal_arguments.h"], "iris": ["assets/models/iris_landmark.tflite"], "shader": ["src/include/tensorflow/lite/delegates/gpu/gl/compiler/shader_code.h", "src/include/tensorflow/lite/delegates/gpu/gl/compiler/shader_codegen.cc", "src/include/tensorflow/lite/delegates/gpu/gl/compiler/shader_codegen.h", "src/include/tensorflow/lite/delegates/gpu/gl/gl_shader.cc", "src/include/tensorflow/lite/delegates/gpu/gl/gl_shader.h", "src/include/tensorflow/lite/delegates/gpu/gl/node_shader.h"], "rig": ["src/include/tensorflow/lite/kernels/right_shift.cc", "src/include/tensorflow/lite/kernels/right_shift_test.cc"], "face": ["assets/models/face_blendshapes.tflite", "assets/models/face_detection_full_range.tflite", "assets/models/face_detection_full_range_sparse.tflite", "assets/models/face_detection_short_range.tflite", "assets/models/face_landmark_with_attention.tflite", "assets/models/face_mesh_v2.tflite", "assets/models/mediapipe_face_mesh.tflite", "bench/assets/bench_face_10s.mp4", "bench/mine/integration_test/multiface_profile_test.dart", "example/android/app/src/main/kotlin/com/cornpip/mediapipe_face_mesh_example/MainActivity.kt", "example/lib/utils/face_mesh_camera_image_adapter.dart", "ios/mediapipe_face_mesh.podspec", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/Info.plist", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/TensorFlowLiteC.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/builtin_ops.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/c_api.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/c_api_experimental.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/c_api_opaque.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/c_api_types.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/common.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/gpu_delegate.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/gpu_delegate_options.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/operator.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/profiler.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/telemetry_setting.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/types.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/Headers/TensorFlowLiteC/xnnpack_delegate.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64/libTensorFlowLiteC.a", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64_x86_64-simulator/Headers/TensorFlowLiteC/TensorFlowLiteC.h", "ios/mediapipe_face_mesh/Frameworks/TensorFlowLiteC.xcframework/ios-arm64_x86_64-simulator/Headers/TensorFlowLiteC/builtin_ops.h"]}

## Source file: src/mediapipe_face_blendshapes.cc

{
  "path": "src/mediapipe_face_blendshapes.cc",
  "blob_sha": "3932d8b8b3fe5efbfbeb8dfaf123556d6e16516b",
  "sha256": "6abe0f0e28174363c2b9d263d8bd3dfc49bbadddd22543d9c48c40fd5bc1ef37",
  "lines": 472,
  "bytes": 16279,
  "selection_score": 30,
  "keyword_line_counts": {
    "blendshape": 66,
    "iris": 2,
    "face": 1
  },
  "url": "https://github.com/cornpip/mediapipe_face_mesh/blob/f2d5d1b9d00f5a9463703a9873e3b8e3dce0b658/src/mediapipe_face_blendshapes.cc",
  "function_signature_candidates": [
    {
      "line": 55,
      "text": "bool Initialize(const std::string& model_path,"
    },
    {
      "line": 276,
      "text": "void RecordError(const std::string& message) { SetError(message); }"
    },
    {
      "line": 283,
      "text": "void operator()(TfLiteModel* model) const {"
    },
    {
      "line": 292,
      "text": "void operator()(TfLiteInterpreterOptions* options) const {"
    },
    {
      "line": 301,
      "text": "void operator()(TfLiteInterpreter* interpreter) const {"
    },
    {
      "line": 311,
      "text": "void operator()(TfLiteDelegate* delegate) const {"
    },
    {
      "line": 318,
      "text": "void Shutdown() {"
    },
    {
      "line": 335,
      "text": "void SetError(const std::string& message) {"
    },
    {
      "line": 361,
      "text": "void SetGlobalBlendshapesError(const std::string& message) {"
    },
    {
      "line": 422,
      "text": "FFI_PLUGIN_EXPORT void mp_blendshapes_destroy(MpBlendshapesContext* context) {"
    },
    {
      "line": 442,
      "text": "FFI_PLUGIN_EXPORT void mp_blendshapes_release_result("
    }
  ]
}

```text

#include "mediapipe_face.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <string>
#include <vector>

#include "tflite_runtime.h"

#if defined(__ANDROID__)
#include <android/log.h>
#define MP_BS_LOG_TAG "MediapipeBlendshapes"
#define MP_BS_LOGI(...) \
  __android_log_print(ANDROID_LOG_INFO, MP_BS_LOG_TAG, __VA_ARGS__)
#define MP_BS_LOGE(...) \
  __android_log_print(ANDROID_LOG_ERROR, MP_BS_LOG_TAG, __VA_ARGS__)
#else
#define MP_BS_LOGI(...) std::fprintf(stdout, "[INFO] " __VA_ARGS__)
#define MP_BS_LOGE(...) std::fprintf(stderr, "[ERROR] " __VA_ARGS__)
#endif

namespace {

constexpr int kBlendshapeCount = 52;
constexpr int kBlendshapeInputLandmarkCount = 146;
// The blendshapes model consumes a subset that reaches into the iris landmarks
// (468..477), so the source result must contain the full 478 landmarks.
constexpr int kMinRequiredLandmarks = 478;

// Subset of the 478 landmarks required by the blendshapes model, in the exact
// order expected by the model input. Mirrors MediaPipe's kLandmarksSubsetIdxs.
constexpr int kBlendshapeLandmarkSubset[kBlendshapeInputLandmarkCount] = {
    0,   1,   4,   5,   6,   7,   8,   10,  13,  14,  17,  21,  33,  37,  39,
    40,  46,  52,  53,  54,  55,  58,  61,  63,  65,  66,  67,  70,  78,  80,
    81,  82,  84,  87,  88,  91,  93,  95,  103, 105, 107, 109, 127, 132, 133,
    136, 144, 145, 146, 148, 149, 150, 152, 153, 154, 155, 157, 158, 159, 160,
    161, 162, 163, 168, 172, 173, 176, 178, 181, 185, 191, 195, 197, 234, 246,
    249, 251, 263, 267, 269, 270, 276, 282, 283, 284, 285, 288, 291, 293, 295,
    296, 297, 300, 308, 310, 311, 312, 314, 317, 318, 321, 323, 324, 332, 334,
    336, 338, 356, 361, 362, 365, 373, 374, 375, 377, 378, 379, 380, 381, 382,
    384, 385, 386, 387, 388, 389, 390, 397, 398, 400, 402, 405, 409, 415, 454,
    466, 468, 469, 470, 471, 472, 473, 474, 475, 476, 477};

class BlendshapesContext {
 public:
  BlendshapesContext() = default;
  ~BlendshapesContext() { Shutdown(); }

  bool Initialize(const std::string& model_path,
                  const MpBlendshapesCreateOptions* options) {
    threads_ = (options && options->threads > 0) ? options->threads : 2;

    const char* runtime_path = (options && options->tflite_library_path)
                                   ? options->tflite_library_path
                                   : nullptr;
    if (!runtime_.Load(runtime_path)) {
      SetError("Failed to load TensorFlow Lite runtime: " + runtime_.error());
      return false;
    }

    model_.reset(runtime_.ModelCreateFromFile(model_path.c_str()));
    if (!model_) {
      SetError("Unable to load blendshapes model file: " + model_path);
      return false;
    }

    options_.reset(runtime_.InterpreterOptionsCreate());
    if (!options_) {
      SetError("Failed to allocate interpreter options.");
      return false;
    }
    runtime_.InterpreterOptionsSetThreads(options_.get(), threads_);

    const MpDelegateType delegate_choice =
        options ? static_cast<MpDelegateType>(options->delegate)
                : MP_DELEGATE_CPU;
    const bool allow_delegate_fallback =
        !options || options->disable_delegate_fallback == 0;
    active_delegate_ = MP_DELEGATE_CPU;
    auto AttachDelegate = [&](TfLiteDelegate* created,
                              TfLiteDelegateDeleter::DeleteFn deleter,
                              const char* name, MpDelegateType delegate_type) {
      if (!created) {
        return false;
      }
      delegate_.get_deleter().deleter = deleter;
      delegate_.reset(created);
      runtime_.InterpreterOptionsAddDelegate(
          options_.get(),
          reinterpret_cast<TfLiteOpaqueDelegate*>(delegate_.get()));
      active_delegate_ = delegate_type;
      MP_BS_LOGI("Blendshapes %s delegate enabled.\n", name);
      return true;
    };

    switch (delegate_choice) {
      case MP_DELEGATE_XNNPACK: {
        if (runtime_.InterpreterOptionsAddDelegate &&
            runtime_.XnnpackDelegateOptionsDefault &&
            runtime_.XnnpackDelegateCreate && runtime_.XnnpackDelegateDelete) {
          TfLiteXNNPackDelegateOptions xnnpack_options =
              runtime_.XnnpackDelegateOptionsDefault();
          xnnpack_options.num_threads = threads_;
          AttachDelegate(runtime_.XnnpackDelegateCreate(&xnnpack_options),
                         runtime_.XnnpackDelegateDelete, "XNNPACK",
                         MP_DELEGATE_XNNPACK);
        } else if (!allow_delegate_fallback) {
          SetError("XNNPACK delegate is unavailable for blendshapes model and "
                   "delegate fallback is disabled.");
          return false;
        }
        break;
      }
      case MP_DELEGATE_GPU_V2: {
        if (runtime_.InterpreterOptionsAddDelegate &&
            runtime_.GpuDelegateV2OptionsDefault &&
            runtime_.GpuDelegateV2Create && runtime_.GpuDelegateV2Delete) {
          TfLiteGpuDelegateOptionsV2 gpu_options =
              runtime_.GpuDelegateV2OptionsDefault();
          gpu_options.experimental_flags |=
              TFLITE_GPU_EXPERIMENTAL_FLAGS_ENABLE_QUANT;
          AttachDelegate(runtime_.GpuDelegateV2Create(&gpu_options),
                         runtime_.GpuDelegateV2Delete, "GPU V2",
                         MP_DELEGATE_GPU_V2);
        } else if (!allow_delegate_fallback) {
          SetError("GPU delegate (V2) is unavailable for blendshapes model and "
                   "delegate fallback is disabled.");
          return false;
        }
        break;
      }
      case MP_DELEGATE_CPU:
      default:
        break;
    }

    if (!allow_delegate_fallback && delegate_choice != MP_DELEGATE_CPU &&
        active_delegate_ != delegate_choice) {
      SetError("Failed to create requested delegate for blendshapes model "
               "because delegate fallback is disabled.");
      return false;
    }

    interpreter_.reset(runtime_.InterpreterCreate(model_.get(), options_.get()));
    bool tensors_ready =
        interpreter_ &&
        runtime_.InterpreterAllocateTensors(interpreter_.get()) == kTfLiteOk;
    // A delegate can also fail after it is attached, while the interpreter
    // builds or allocates tensors (e.g. the GPU delegate rejecting a graph
    // with custom ops). Honor delegate fallback for that stage too.
    if (!tensors_ready && active_delegate_ != MP_DELEGATE_CPU &&
        allow_delegate_fallback) {
      MP_BS_LOGE(
          "Blendshapes interpreter creation with the requested delegate "
          "failed. Falling back to CPU.\n");
      interpreter_.reset();
      options_.reset(runtime_.InterpreterOptionsCreate());
      delegate_.reset();
      if (!options_) {
        SetError("Failed to allocate interpreter options.");
        return false;
      }
      runtime_.InterpreterOptionsSetThreads(options_.get(), threads_);
      active_delegate_ = MP_DELEGATE_CPU;
      interpreter_.reset(
          runtime_.InterpreterCreate(model_.get(), options_.get()));
      tensors_ready =
          interpreter_ &&
          runtime_.InterpreterAllocateTensors(interpreter_.get()) == kTfLiteOk;
    }
    if (!interpreter_) {
      SetError("Failed to create blendshapes interpreter.");
      return false;
    }
    if (!tensors_ready) {
      SetError("Blendshapes tensor allocation failed.");
      return false;
    }
    if (runtime_.InterpreterGetInputTensorCount(interpreter_.get()) < 1) {
      SetError("Blendshapes interpreter input tensor missing.");
      return false;
    }
    input_tensor_ = runtime_.InterpreterGetInputTensor(interpreter_.get(), 0);
    if (!input_tensor_ ||
        runtime_.TensorType(input_tensor_) != kTfLiteFloat32) {
      SetError("Blendshapes model input must be float32.");
      return false;
    }
    if (TensorElementCount(input_tensor_) !=
        static_cast<size_t>(kBlendshapeInputLandmarkCount * 2)) {
      SetError("Blendshapes model expects a 146x2 landmark input.");
      return false;
    }
    input_buffer_.resize(static_cast<size_t>(kBlendshapeInputLandmarkCount * 2));

    if (runtime_.InterpreterGetOutputTensorCount(interpreter_.get()) < 1) {
      SetError("Blendshapes model output missing.");
      return false;
    }
    output_tensor_ = runtime_.InterpreterGetOutputTensor(interpreter_.get(), 0);
    if (!output_tensor_ ||
        runtime_.TensorType(output_tensor_) != kTfLiteFloat32) {
      SetError("Blendshapes output tensor must be float32.");
      return false;
    }
    if (TensorElementCount(output_tensor_) !=
        static_cast<size_t>(kBlendshapeCount)) {
      SetError("Blendshapes model must output 52 coefficients.");
      return false;
    }
    output_buffer_.resize(static_cast<size_t>(kBlendshapeCount));
    MP_BS_LOGI("Blendshapes initialize success\n");
    return true;
  }

  MpBlendshapesResult* Process(const MpLandmark* landmarks,
                               int landmarks_count, int image_width,
                               int image_height) {
    if (!interpreter_) {
      SetError("Blendshapes interpreter is not initialized.");
      return nullptr;
    }
    if (!landmarks || landmarks_count < kMinRequiredLandmarks) {
      SetError("Blendshapes require at least 478 landmarks (enable iris).");
      return nullptr;
    }
    if (image_width <= 0 || image_height <= 0) {
      SetError("Invalid image size for blendshapes.");
      return nullptr;
    }

    const float width = static_cast<float>(image_width);
    const float height = static_cast<float>(image_height);
    for (int i = 0; i < kBlendshapeInputLandmarkCount; ++i) {
      const MpLandmark& landmark = landmarks[kBlendshapeLandmarkSubset[i]];
      input_buffer_[i * 2] = landmark.x * width;
      input_buffer_[i * 2 + 1] = landmark.y * height;
    }

    if (runtime_.TensorCopyFromBuffer(input_tensor_, input_buffer_.data(),
                                      input_buffer_.size() * sizeof(float)) !=
        kTfLiteOk) {
      SetError("Failed to copy blendshapes input buffer.");
      return nullptr;
    }
    if (runtime_.InterpreterInvoke(interpreter_.get()) != kTfLiteOk) {
      SetError("Blendshapes invocation failed.");
      return nullptr;
    }
    if (runtime_.TensorCopyToBuffer(output_tensor_, output_buffer_.data(),
                                    output_buffer_.size() * sizeof(float)) !=
        kTfLiteOk) {
      SetError("Unable to read blendshapes output.");
      return nullptr;
    }

    auto* result = new MpBlendshapesResult();
    result->scores = new float[kBlendshapeCount];
    result->scores_count = kBlendshapeCount;
    for (int i = 0; i < kBlendshapeCount; ++i) {
      result->scores[i] = output_buffer_[i];
    }
    return result;
  }

  const char* last_error() const { return last_error_.c_str(); }

  // Records an error raised outside the class, e.g. a C++ exception caught
  // at the FFI boundary.
  void RecordError(const std::string& message) { SetError(message); }

  MpDelegateType active_delegate() const { return active_delegate_; }

 private:
  struct TfLiteModelDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteModel* model) const {
      if (runtime && model) {
        runtime->ModelDelete(model);
      }
    }
  };

  struct TfLiteOptionsDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteInterpreterOptions* options) const {
      if (runtime && options) {
        runtime->InterpreterOptionsDelete(options);
      }
    }
  };

  struct TfLiteInterpreterDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteInterpreter* interpreter) const {
      if (runtime && interpreter) {
        runtime->InterpreterDelete(interpreter);
      }
    }
  };

  struct TfLiteDelegateDeleter {
    using DeleteFn = void (*)(TfLiteDelegate*);
    DeleteFn deleter = nullptr;
    void operator()(TfLiteDelegate* delegate) const {
      if (deleter && delegate) {
        deleter(delegate);
      }
    }
  };

  void Shutdown() {
    interpreter_.reset();
    options_.reset();
    model_.reset();
    delegate_.reset();
    runtime_.Release();
  }

  size_t TensorElementCount(const TfLiteTensor* tensor) const {
    int total = 1;
    const int dims = runtime_.TensorNumDims(tensor);
    for (int i = 0; i < dims; ++i) {
      total *= runtime_.TensorDim(tensor, i);
    }
    return static_cast<size_t>(total);
  }

  void SetError(const std::string& message) {
    last_error_ = message;
    MP_BS_LOGE("%s\n", message.c_str());
  }

  TfLiteRuntime runtime_;
  std::unique_ptr<TfLiteModel, TfLiteModelDeleter> model_{nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreterOptions, TfLiteOptionsDeleter> options_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreter, TfLiteInterpreterDeleter> interpreter_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteDelegate, TfLiteDelegateDeleter> delegate_{nullptr, {}};

  TfLiteTensor* input_tensor_ = nullptr;
  const TfLiteTensor* output_tensor_ = nullptr;

  int threads_ = 2;
  MpDelegateType active_delegate_ = MP_DELEGATE_CPU;

  std::vector<float> input_buffer_;
  std::vector<float> output_buffer_;
  std::string last_error_;
};

thread_local std::string g_bs_last_global_error;

void SetGlobalBlendshapesError(const std::string& message) {
  g_bs_last_global_error = message;
}

}  // namespace

struct MpBlendshapesContext {
  BlendshapesContext impl;
};

namespace {

// No C++ exception may unwind across the FFI boundary into Dart (the process
// would abort), so pointer-returning exports run their body through this
// guard, which converts an escaped exception (std::bad_alloc included) into
// a null return with the error recorded on the context, when there is one,
// and globally.
template <typename Fn>
auto GuardedNative(MpBlendshapesContext* context, Fn&& body)
    -> decltype(body()) {
  try {
    return body();
  } catch (const std::exception& e) {
    const std::string message =
        std::string("Unhandled native exception: ") + e.what();
    if (context) {
      context->impl.RecordError(message);
    }
    SetGlobalBlendshapesError(message);
    return nullptr;
  } catch (...) {
    const std::string message = "Unhandled native exception.";
    if (context) {
      context->impl.RecordError(message);
    }
    SetGlobalBlendshapesError(message);
    return nullptr;
  }
}

}  // namespace

extern "C" {

FFI_PLUGIN_EXPORT MpBlendshapesContext* mp_blendshapes_create(
    const char* model_path, const MpBlendshapesCreateOptions* options) {
  return GuardedNative(nullptr, [&]() -> MpBlendshapesContext* {
    if (!model_path) {
      SetGlobalBlendshapesError("Model path is null.");
      return nullptr;
    }
    auto* context = new MpBlendshapesContext();
    if (!context->impl.Initialize(model_path, options)) {
      SetGlobalBlendshapesError(context->impl.last_error());
      delete context;
      return nullptr;
    }
    return context;
  });
}

FFI_PLUGIN_EXPORT void mp_blendshapes_destroy(MpBlendshapesContext* context) {
  try {
    delete context;
  } catch (...) {
  }
}

FFI_PLUGIN_EXPORT MpBlendshapesResult* mp_blendshapes_process(
    MpBlendshapesContext* context, const MpLandmark* landmarks,
    int32_t landmarks_count, int32_t image_width, int32_t image_height) {
  return GuardedNative(context, [&]() -> MpBlendshapesResult* {
    if (!context) {
      SetGlobalBlendshapesError("Context is null.");
      return nullptr;
    }
    return context->impl.Process(landmarks, landmarks_count, image_width,
                                 image_height);
  });
}

FFI_PLUGIN_EXPORT void mp_blendshapes_release_result(
    MpBlendshapesResult* result) {
  if (!result) {
    return;
  }
  delete[] result->scores;
  result->scores = nullptr;
  delete result;
}

FFI_PLUGIN_EXPORT const char* mp_blendshapes_last_error(
    const MpBlendshapesContext* context) {
  if (!context) {
    return nullptr;
  }
  return context->impl.last_error();
}

FFI_PLUGIN_EXPORT const char* mp_blendshapes_last_global_error(void) {
  return g_bs_last_global_error.c_str();
}

FFI_PLUGIN_EXPORT MpDelegateType mp_blendshapes_active_delegate(
    const MpBlendshapesContext* context) {
  if (!context) {
    return MP_DELEGATE_CPU;
  }
  return context->impl.active_delegate();
}

}  // extern "C"


```

## Source file: src/mediapipe_face.h

{
  "path": "src/mediapipe_face.h",
  "blob_sha": "cab4b19326a1b9c436f8d2592e0c0d0fdd00c0b7",
  "sha256": "182cb0cc74102b6152af3f9aede0c27afbb9dd0f24065089ae9507a150ec60a8",
  "lines": 325,
  "bytes": 10269,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 20,
    "iris": 6,
    "rig": 3,
    "face": 74,
    "lip": 2
  },
  "url": "https://github.com/cornpip/mediapipe_face_mesh/blob/f2d5d1b9d00f5a9463703a9873e3b8e3dce0b658/src/mediapipe_face.h",
  "function_signature_candidates": [
    {
      "line": 168,
      "text": "FFI_PLUGIN_EXPORT void mp_face_mesh_destroy(MpFaceMeshContext* context);"
    },
    {
      "line": 205,
      "text": "FFI_PLUGIN_EXPORT void mp_face_mesh_release_result(MpFaceMeshResult* result);"
    },
    {
      "line": 207,
      "text": "FFI_PLUGIN_EXPORT void mp_face_mesh_release_multi_result("
    },
    {
      "line": 235,
      "text": "FFI_PLUGIN_EXPORT void mp_face_geometry_release_result("
    },
    {
      "line": 266,
      "text": "FFI_PLUGIN_EXPORT void mp_blendshapes_destroy(MpBlendshapesContext* context);"
    },
    {
      "line": 278,
      "text": "FFI_PLUGIN_EXPORT void mp_blendshapes_release_result("
    },
    {
      "line": 292,
      "text": "FFI_PLUGIN_EXPORT void mp_face_detector_destroy(MpFaceDetectorContext* context);"
    },
    {
      "line": 310,
      "text": "FFI_PLUGIN_EXPORT void mp_face_detector_release_result("
    }
  ]
}

```text

#ifndef MEDIAPIPE_FACE_MESH_H_
#define MEDIAPIPE_FACE_MESH_H_

#include <stdint.h>

#if _WIN32
#define FFI_PLUGIN_EXPORT __declspec(dllexport)
#else
#define FFI_PLUGIN_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MpFaceMeshContext MpFaceMeshContext;
typedef struct MpFaceDetectorContext MpFaceDetectorContext;

typedef enum {
  MP_PIXEL_FORMAT_RGBA = 0,
  MP_PIXEL_FORMAT_BGRA = 1,
} MpPixelFormat;

typedef enum {
  MP_DELEGATE_CPU = 0,
  MP_DELEGATE_XNNPACK = 1,
  MP_DELEGATE_GPU_V2 = 2,
} MpDelegateType;

typedef struct {
  const uint8_t* data;
  int32_t width;
  int32_t height;
  int32_t bytes_per_row;
  MpPixelFormat format;
} MpImage;

// Android camera NV21 input (Y plane + interleaved VU plane).
typedef struct {
  const uint8_t* y;
  const uint8_t* vu;
  int32_t width;
  int32_t height;
  int32_t y_bytes_per_row;
  int32_t vu_bytes_per_row;
} MpNv21Image;

typedef struct {
  float x_center;
  float y_center;
  float width;
  float height;
  float rotation;
} MpNormalizedRect;

typedef struct {
  float x;
  float y;
  float z;
} MpLandmark;

typedef struct {
  MpLandmark* landmarks;
  int32_t landmarks_count;
  MpNormalizedRect rect;
  float score;
  int32_t image_width;
  int32_t image_height;
} MpFaceMeshResult;

typedef struct {
  // One entry per input ROI, in input order. Entries whose face presence
  // score fell below the threshold have landmarks_count == 0, matching
  // mp_face_mesh_process.
  MpFaceMeshResult* results;
  int32_t results_count;
} MpFaceMeshMultiResult;

typedef struct {
  // Vertical field of view of the virtual camera in degrees.
  float vertical_fov_degrees;
  // Near and far clip planes of the virtual frustum in the same units as the
  // output metric landmarks (centimeters).
  float near_plane;
  float far_plane;
  // Non-zero when input normalized landmarks use a top-left image origin.
  // This matches FaceMeshResult landmarks returned by this plugin.
  uint8_t origin_top_left;
} MpFaceGeometryOptions;

typedef struct {
  // Landmarks reprojected into centimeter-scale metric space.
  MpLandmark* metric_landmarks;
  int32_t metric_landmarks_count;
  // Row-major 4x4 similarity transform from canonical face space to camera
  // space. Encodes rotation, translation, and uniform scale.
  float pose_transform_matrix[16];
  float yaw_degrees;
  float pitch_degrees;
  float roll_degrees;
  // Uniform scale factor from the Procrustes fit: ratio of the observed face
  // size to the canonical model size in centimeters.
  float scale;
} MpFaceGeometryResult;

typedef struct {
  const char* tflite_library_path;
  const char* iris_model_path;
  int32_t threads;
  float min_detection_confidence;
  float min_tracking_confidence;
  float min_face_presence_confidence;
  MpDelegateType delegate;
  uint8_t enable_roi_tracking;
  uint8_t enable_iris;
  // When non-zero, [model_path] points to the unified
  // `face_landmark_with_attention` model, which refines lips, eyes, and irises
  // in a single inference and outputs 478 landmarks directly. Requires a
  // libtensorflowlite_c that registers the MediaPipe custom ops. When set, the
  // separate iris pass ([enable_iris]/[iris_model_path]) is not used; iris is
  // always included in the result.
  uint8_t enable_attention_mesh;
  // When non-zero, fail creation instead of falling back to CPU if the
  // requested delegate is unavailable or cannot be created.
  uint8_t disable_delegate_fallback;
} MpFaceMeshCreateOptions;

typedef struct {
  float left;
  float top;
  float right;
  float bottom;
  float score;
  float keypoints[12];
  MpNormalizedRect face_rect;
  MpNormalizedRect expanded_face_rect;
} MpDetection;

typedef struct {
  MpDetection* detections;
  int32_t detections_count;
  int32_t image_width;
  int32_t image_height;
} MpFaceDetectorResult;

typedef struct {
  const char* tflite_library_path;
  int32_t threads;
  float min_detection_confidence;
  float min_suppression_threshold;
  int32_t max_results;
  MpDelegateType delegate;
  // When non-zero, fail creation instead of falling back to CPU if the
  // requested delegate is unavailable or cannot be created.
  uint8_t disable_delegate_fallback;
} MpFaceDetectorCreateOptions;

typedef struct {
  float scale_x;
  float scale_y;
  float shift_x;
  float shift_y;
} MpRoiTransformOptions;

FFI_PLUGIN_EXPORT MpFaceMeshContext* mp_face_mesh_create(
    const char* model_path, const MpFaceMeshCreateOptions* options);

FFI_PLUGIN_EXPORT void mp_face_mesh_destroy(MpFaceMeshContext* context);

FFI_PLUGIN_EXPORT MpFaceMeshResult* mp_face_mesh_process(
    MpFaceMeshContext* context,
    const MpImage* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal);

FFI_PLUGIN_EXPORT MpFaceMeshResult* mp_face_mesh_process_nv21(
    MpFaceMeshContext* context,
    const MpNv21Image* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal);

// Runs one mesh inference per ROI on the given frame, equivalent to calling
// mp_face_mesh_process once per ROI. The batch form exists so a language
// binding can copy the frame across its boundary once per frame instead of
// once per face. Returns null and sets the context error if any ROI fails.
// Release the result with mp_face_mesh_release_multi_result.
FFI_PLUGIN_EXPORT MpFaceMeshMultiResult* mp_face_mesh_process_rois(
    MpFaceMeshContext* context,
    const MpImage* image,
    const MpNormalizedRect* rois,
    int32_t rois_count,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal);

FFI_PLUGIN_EXPORT MpFaceMeshMultiResult* mp_face_mesh_process_rois_nv21(
    MpFaceMeshContext* context,
    const MpNv21Image* image,
    const MpNormalizedRect* rois,
    int32_t rois_count,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal);

FFI_PLUGIN_EXPORT void mp_face_mesh_release_result(MpFaceMeshResult* result);

FFI_PLUGIN_EXPORT void mp_face_mesh_release_multi_result(
    MpFaceMeshMultiResult* result);

// Non-zero while the internal tracked ROI follows a face, meaning it was seeded
// from landmarks and has not been dropped by a face-presence or
// tracking-confidence failure, or an input-geometry change. Always zero when
// the context was created with enable_roi_tracking off.
FFI_PLUGIN_EXPORT uint8_t mp_face_mesh_is_tracking(
    const MpFaceMeshContext* context);

FFI_PLUGIN_EXPORT const char* mp_face_mesh_last_error(
    const MpFaceMeshContext* context);

FFI_PLUGIN_EXPORT const char* mp_face_mesh_last_global_error(void);

FFI_PLUGIN_EXPORT MpDelegateType mp_face_mesh_active_delegate(
    const MpFaceMeshContext* context);

FFI_PLUGIN_EXPORT MpDelegateType mp_face_mesh_active_iris_delegate(
    const MpFaceMeshContext* context);

FFI_PLUGIN_EXPORT MpFaceGeometryResult* mp_face_geometry_estimate(
    const MpLandmark* landmarks,
    int32_t landmarks_count,
    int32_t image_width,
    int32_t image_height,
    const MpFaceGeometryOptions* options);

FFI_PLUGIN_EXPORT void mp_face_geometry_release_result(
    MpFaceGeometryResult* result);

FFI_PLUGIN_EXPORT const char* mp_face_geometry_last_error(void);

// ---- Face blendshapes ------------------------------------------------------
// Post-processing module that turns face landmarks into 52 ARKit-style
// blendshape coefficients. Runs as its own context, separate from the face
// mesh context, on the landmarks it is given.

typedef struct MpBlendshapesContext MpBlendshapesContext;

typedef struct {
  const char* tflite_library_path;
  int32_t threads;
  MpDelegateType delegate;
  // When non-zero, fail creation instead of falling back to CPU if the
  // requested delegate is unavailable or cannot be created.
  uint8_t disable_delegate_fallback;
} MpBlendshapesCreateOptions;

typedef struct {
  // 52 blendshape coefficients in [0, 1], ordered to match the MediaPipe face
  // blendshapes model (index 0 is `_neutral`).
  float* scores;
  int32_t scores_count;
} MpBlendshapesResult;

FFI_PLUGIN_EXPORT MpBlendshapesContext* mp_blendshapes_create(
    const char* model_path, const MpBlendshapesCreateOptions* options);

FFI_PLUGIN_EXPORT void mp_blendshapes_destroy(MpBlendshapesContext* context);

// Runs the blendshapes model on the provided face landmarks. Requires at least
// 478 landmarks (iris). [image_width]/[image_height] are the dimensions the
// normalized landmarks are expressed against. Returns null on error.
FFI_PLUGIN_EXPORT MpBlendshapesResult* mp_blendshapes_process(
    MpBlendshapesContext* context,
    const MpLandmark* landmarks,
    int32_t landmarks_count,
    int32_t image_width,
    int32_t image_height);

FFI_PLUGIN_EXPORT void mp_blendshapes_release_result(
    MpBlendshapesResult* result);

FFI_PLUGIN_EXPORT const char* mp_blendshapes_last_error(
    const MpBlendshapesContext* context);

FFI_PLUGIN_EXPORT const char* mp_blendshapes_last_global_error(void);

FFI_PLUGIN_EXPORT MpDelegateType mp_blendshapes_active_delegate(
    const MpBlendshapesContext* context);

FFI_PLUGIN_EXPORT MpFaceDetectorContext* mp_face_detector_create(
    const char* model_path, const MpFaceDetectorCreateOptions* options);

FFI_PLUGIN_EXPORT void mp_face_detector_destroy(MpFaceDetectorContext* context);

FFI_PLUGIN_EXPORT MpFaceDetectorResult* mp_face_detector_process(
    MpFaceDetectorContext* context,
    const MpImage* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal,
    const MpRoiTransformOptions* roi_transform);

FFI_PLUGIN_EXPORT MpFaceDetectorResult* mp_face_detector_process_nv21(
    MpFaceDetectorContext* context,
    const MpNv21Image* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal,
    const MpRoiTransformOptions* roi_transform);

FFI_PLUGIN_EXPORT void mp_face_detector_release_result(
    MpFaceDetectorResult* result);

FFI_PLUGIN_EXPORT const char* mp_face_detector_last_error(
    const MpFaceDetectorContext* context);

FFI_PLUGIN_EXPORT const char* mp_face_detector_last_global_error(void);

FFI_PLUGIN_EXPORT MpDelegateType mp_face_detector_active_delegate(
    const MpFaceDetectorContext* context);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // MEDIAPIPE_FACE_MESH_H_


```

## Source file: src/mediapipe_face_mesh.cc

{
  "path": "src/mediapipe_face_mesh.cc",
  "blob_sha": "80ad2b0e214e700a85bc02bdf1b5b33c665e942f",
  "sha256": "06997b4e5742e2c2a81343f44c3f60fa216706786d988cab1ad76a4f4f163b59",
  "lines": 2204,
  "bytes": 82116,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 69,
    "iris": 191,
    "rig": 36,
    "face": 109,
    "lip": 13
  },
  "url": "https://github.com/cornpip/mediapipe_face_mesh/blob/f2d5d1b9d00f5a9463703a9873e3b8e3dce0b658/src/mediapipe_face_mesh.cc",
  "function_signature_candidates": [
    {
      "line": 91,
      "text": "float Clamp(float value, float min_value, float max_value) {"
    },
    {
      "line": 95,
      "text": "int ClampInt(int value, int min_value, int max_value) {"
    },
    {
      "line": 99,
      "text": "float NormalizeAngle(float radians) {"
    },
    {
      "line": 119,
      "text": "bool IsPixelSpaceLandmarkBuffer(const float* buffer, int landmark_count) {"
    },
    {
      "line": 135,
      "text": "bool Initialize(const std::string& model_path,"
    },
    {
      "line": 674,
      "text": "void RecordError(const std::string& message) { SetError(message); }"
    },
    {
      "line": 679,
      "text": "bool is_tracking() const { return tracking_active_; }"
    },
    {
      "line": 688,
      "text": "void operator()(TfLiteModel* model) const {"
    },
    {
      "line": 697,
      "text": "void operator()(TfLiteInterpreterOptions* options) const {"
    },
    {
      "line": 706,
      "text": "void operator()(TfLiteInterpreter* interpreter) const {"
    },
    {
      "line": 716,
      "text": "void operator()(TfLiteDelegate* delegate) const {"
    },
    {
      "line": 723,
      "text": "void Shutdown() {"
    },
    {
      "line": 735,
      "text": "bool InitializeIris(const std::string& iris_model_path,"
    },
    {
      "line": 940,
      "text": "static int NormalizeRotationDegrees(int rotation_degrees) {"
    },
    {
      "line": 952,
      "text": "bool Preprocess(const MpImage& image, const MpNormalizedRect& rect) {"
    },
    {
      "line": 990,
      "text": "bool PreprocessRotated(const MpImage& image,"
    },
    {
      "line": 1036,
      "text": "bool PreprocessNv21(const MpNv21Image& image, const MpNormalizedRect& rect) {"
    },
    {
      "line": 1075,
      "text": "bool PreprocessNv21Rotated(const MpNv21Image& image,"
    },
    {
      "line": 1290,
      "text": "static inline void MapRotatedToRaw(int x_rot,"
    },
    {
      "line": 1422,
      "text": "bool BindAttentionOutputs(int output_count) {"
    },
    {
      "line": 1465,
      "text": "bool ReadAttentionLandmarks(float* raw_score_out) {"
    },
    {
      "line": 1536,
      "text": "float AverageContourZ(int offset) const {"
    },
    {
      "line": 1554,
      "text": "bool AugmentResultWithIris(const MpImage& image,"
    },
    {
      "line": 1574,
      "text": "bool AugmentResultWithIrisNv21(const MpNv21Image& image,"
    },
    {
      "line": 1595,
      "text": "bool AugmentResultWithIrisFromSampler(MpFaceMeshResult* result,"
    },
    {
      "line": 1641,
      "text": "bool RunIrisForEye(const MpFaceMeshResult* result,"
    },
    {
      "line": 1711,
      "text": "void DecodeIrisLandmarks(const std::vector<float>& buffer,"
    },
    {
      "line": 1798,
      "text": "void UpdateTrackingState(const MpFaceMeshResult& result, float score) {"
    },
    {
      "line": 1858,
      "text": "float EstimateRotation(const MpLandmark* landmarks,"
    },
    {
      "line": 1878,
      "text": "void SetError(const std::string& message) {"
    },
    {
      "line": 1954,
      "text": "void SetGlobalError(const std::string& message) {"
    },
    {
      "line": 2043,
      "text": "FFI_PLUGIN_EXPORT void mp_face_mesh_destroy(MpFaceMeshContext* context) {"
    },
    {
      "line": 2146,
      "text": "FFI_PLUGIN_EXPORT void mp_face_mesh_release_result(MpFaceMeshResult* result) {"
    },
    {
      "line": 2155,
      "text": "FFI_PLUGIN_EXPORT void mp_face_mesh_release_multi_result("
    }
  ]
}

```text

#include "mediapipe_face.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <cstdio>

#include <exception>
#include <memory>
#include <string>
#include <utility>
#include <vector>
#include "tflite_runtime.h"

#if defined(__ANDROID__)
#include <android/log.h>
#define MP_LOG_TAG "MediapipeFaceMesh"
#define MP_LOGI(...) __android_log_print(ANDROID_LOG_INFO, MP_LOG_TAG, __VA_ARGS__)
#define MP_LOGE(...) __android_log_print(ANDROID_LOG_ERROR, MP_LOG_TAG, __VA_ARGS__)
#else
#define MP_LOGI(...) std::fprintf(stdout, "[INFO] " __VA_ARGS__)
#define MP_LOGE(...) std::fprintf(stderr, "[ERROR] " __VA_ARGS__)
#endif

namespace {

struct RectInPixels {
  float center_x = 0.0f;
  float center_y = 0.0f;
  float width = 0.0f;
  float height = 0.0f;
  float rotation = 0.0f;
};

struct RgbPixel {
  float r = 0.0f;
  float g = 0.0f;
  float b = 0.0f;
};

constexpr int kFaceLandmarkCount = 468;
constexpr int kEyeLandmarkCount = 71;
constexpr int kIrisLandmarkCount = 5;
constexpr int kIrisInputSize = 64;
constexpr float kIrisRoiScale = 2.3f;

constexpr int kEyeLandmarkIndicesInFaceLandmarks[kEyeLandmarkCount * 2] = {
    33,  7,   163, 144, 145, 153, 154, 155, 133, 246, 161, 160,
    159, 158, 157, 173, 130, 25,  110, 24,  23,  22,  26,  112,
    243, 247, 30,  29,  27,  28,  56,  190, 226, 31,  228, 229,
    230, 231, 232, 233, 244, 113, 225, 224, 223, 222, 221, 189,
    35,  124, 46,  53,  52,  65,  143, 111, 117, 118, 119, 120,
    121, 128, 245, 156, 70,  63,  105, 66,  107, 55,  193,
    263, 249, 390, 373, 374, 380, 381, 382, 362, 466, 388, 387,
    386, 385, 384, 398, 359, 255, 339, 254, 253, 252, 256, 341,
    463, 467, 260, 259, 257, 258, 286, 414, 446, 261, 448, 449,
    450, 451, 452, 453, 464, 342, 445, 444, 443, 442, 441, 413,
    265, 353, 276, 283, 282, 295, 372, 340, 346, 347, 348, 349,
    350, 357, 465, 383, 300, 293, 334, 296, 336, 285, 417};

// --- Attention (face_landmark_with_attention) mesh assembly ------------------
// The attention model emits 7 output tensors that map onto the 478-landmark
// layout via LandmarksRefinementCalculator (see MediaPipe
// tensors_to_face_landmarks_with_attention.pbtxt):
//   - mesh (468x3)  -> indices 0..467 (x,y,z)
//   - lips (80x2)   -> kLipsLandmarkIndices  (x,y only; z kept from mesh)
//   - left  eye (71x2) -> kEyeLandmarkIndicesInFaceLandmarks[0..70]  (x,y only)
//   - right eye (71x2) -> kEyeLandmarkIndicesInFaceLandmarks[71..141] (x,y only)
//   - left  iris (5x2) -> indices 468..472 (x,y; z = avg of left-eye contour z)
//   - right iris (5x2) -> indices 473..477 (x,y; z = avg of right-eye contour z)
constexpr int kAttentionLandmarkCount = 478;
constexpr int kLipsLandmarkCount = 80;
constexpr int kLipsLandmarkIndicesInFaceLandmarks[kLipsLandmarkCount] = {
    // Lower outer / Upper outer (excluding corners).
    61,  146, 91,  181, 84,  17,  314, 405, 321, 375, 291,
    185, 40,  39,  37,  0,   267, 269, 270, 409,
    // Lower inner / Upper inner (excluding corners).
    78,  95,  88,  178, 87,  14,  317, 402, 318, 324, 308,
    191, 80,  81,  82,  13,  312, 311, 310, 415,
    // Lower semi-outer / Upper semi-outer (excluding corners).
    76,  77,  90,  180, 85,  16,  315, 404, 320, 307, 306,
    184, 74,  73,  72,  11,  302, 303, 304, 408,
    // Lower semi-inner / Upper semi-inner (excluding corners).
    62,  96,  89,  179, 86,  15,  316, 403, 319, 325, 292,
    183, 42,  41,  38,  12,  268, 271, 272, 407};
// Number of eye-contour points averaged to assign each iris landmark's z.
constexpr int kIrisZAverageCount = 16;

float Clamp(float value, float min_value, float max_value) {
  return std::max(min_value, std::min(max_value, value));
}

int ClampInt(int value, int min_value, int max_value) {
  return std::max(min_value, std::min(max_value, value));
}

float NormalizeAngle(float radians) {
  constexpr float kPi = 3.14159265358979323846f;
  constexpr float kTwoPi = kPi * 2.0f;
  float angle = radians;
  while (angle > kPi) {
    angle -= kTwoPi;
  }
  while (angle < -kPi) {
    angle += kTwoPi;
  }
  return angle;
}

// Whether a landmark tensor holds pixel coordinates in input-tensor
// resolution rather than normalized [0,1] values. Decided over the whole
// buffer, not per landmark: normalized outputs stay near [0,1] while
// pixel-space outputs put most landmarks far outside it, so a single
// coordinate beyond a generous margin means the buffer is pixel-space. A
// per-landmark test would misread a pixel-space landmark that legitimately
// lands within one pixel of the crop origin as already normalized.
bool IsPixelSpaceLandmarkBuffer(const float* buffer, int landmark_count) {
  for (int i = 0; i < landmark_count; ++i) {
    const float x = buffer[i * 3];
    const float y = buffer[i * 3 + 1];
    if (x > 2.0f || y > 2.0f || x < -1.0f || y < -1.0f) {
      return true;
    }
  }
  return false;
}

class FaceMeshContext {
 public:
  FaceMeshContext() = default;
  ~FaceMeshContext() { Shutdown(); }

  bool Initialize(const std::string& model_path,
                  const MpFaceMeshCreateOptions* options) {
    threads_ = 2;
    if (options && options->threads > 0) {
      threads_ = options->threads;
    }
    // Negative means "use the default"; an explicit 0.0 is honored so the
    // native threshold always matches what the Dart processor reports.
    min_detection_confidence_ =
        (options && options->min_detection_confidence >= 0.f)
            ? options->min_detection_confidence
            : 0.5f;
    min_tracking_confidence_ =
        (options && options->min_tracking_confidence >= 0.f)
            ? options->min_tracking_confidence
            : 0.5f;
    min_face_presence_confidence_ =
        (options && options->min_face_presence_confidence >= 0.f)
            ? options->min_face_presence_confidence
            : 0.5f;
    roi_tracking_enabled_ = !options || options->enable_roi_tracking != 0;
    attention_enabled_ = options && options->enable_attention_mesh != 0;
    // The attention model already refines and outputs irises; the separate iris
    // pass is skipped, but iris is always present in the 478-landmark result.
    iris_enabled_ = !attention_enabled_ && options && options->enable_iris != 0;

    MP_LOGI("Initialize start: model=%s threads=%d\n", model_path.c_str(),
            threads_);

    const char* runtime_path =
        (options && options->tflite_library_path)
            ? options->tflite_library_path
            : nullptr;

    if (!runtime_.Load(runtime_path)) {
      SetError("Failed to load TensorFlow Lite runtime: " + runtime_.error());
      return false;
    }

    model_.reset(runtime_.ModelCreateFromFile(model_path.c_str()));
    if (!model_) {
      SetError("Unable to load model file: " + model_path);
      return false;
    }

    options_.reset(runtime_.InterpreterOptionsCreate());
    if (!options_) {
      SetError("Failed to allocate interpreter options.");
      return false;
    }
    runtime_.InterpreterOptionsSetThreads(options_.get(), threads_);

    const MpDelegateType delegate_choice =
        options ? static_cast<MpDelegateType>(options->delegate)
                : MP_DELEGATE_CPU;
    const bool allow_delegate_fallback =
        !options || options->disable_delegate_fallback == 0;
    active_delegate_ = MP_DELEGATE_CPU;
    auto AttachDelegate = [&](TfLiteDelegate* created,
                              TfLiteDelegateDeleter::DeleteFn deleter,
                              const char* name,
                              MpDelegateType delegate_type) {
      if (!created) {
        return false;
      }
      delegate_.get_deleter().deleter = deleter;
      delegate_.reset(created);
      runtime_.InterpreterOptionsAddDelegate(
          options_.get(),
          reinterpret_cast<TfLiteOpaqueDelegate*>(delegate_.get()));
      active_delegate_ = delegate_type;
      MP_LOGI("%s delegate enabled.\n", name);
      return true;
    };
    switch (delegate_choice) {
      case MP_DELEGATE_XNNPACK: {
        if (!runtime_.InterpreterOptionsAddDelegate ||
            !runtime_.XnnpackDelegateOptionsDefault ||
            !runtime_.XnnpackDelegateCreate || !runtime_.XnnpackDelegateDelete) {
          if (!allow_delegate_fallback) {
            SetError("XNNPACK delegate is unavailable for face mesh and "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_LOGI("XNNPACK delegate requested but not available in runtime.\n");
          break;
        }
        TfLiteXNNPackDelegateOptions xnnpack_options =
            runtime_.XnnpackDelegateOptionsDefault();
        xnnpack_options.num_threads = threads_;
        TfLiteDelegate* created_delegate =
            runtime_.XnnpackDelegateCreate(&xnnpack_options);
        if (!AttachDelegate(created_delegate, runtime_.XnnpackDelegateDelete,
                            "XNNPACK", MP_DELEGATE_XNNPACK)) {
          if (!allow_delegate_fallback) {
            SetError("Failed to create XNNPACK delegate for face mesh because "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_LOGE("Failed to create XNNPACK delegate. Falling back to CPU.\n");
        }
        break;
      }
      case MP_DELEGATE_GPU_V2: {
        if (!runtime_.InterpreterOptionsAddDelegate ||
            !runtime_.GpuDelegateV2OptionsDefault ||
            !runtime_.GpuDelegateV2Create || !runtime_.GpuDelegateV2Delete) {
          if (!allow_delegate_fallback) {
            SetError("GPU delegate (V2) is unavailable for face mesh and "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_LOGI("GPU delegate (V2) requested but not available in runtime.\n");
          break;
        }
        TfLiteGpuDelegateOptionsV2 gpu_options =
            runtime_.GpuDelegateV2OptionsDefault();
        gpu_options.experimental_flags |= TFLITE_GPU_EXPERIMENTAL_FLAGS_ENABLE_QUANT;
        TfLiteDelegate* created_delegate =
            runtime_.GpuDelegateV2Create(&gpu_options);
        if (!AttachDelegate(created_delegate, runtime_.GpuDelegateV2Delete,
                            "GPU V2", MP_DELEGATE_GPU_V2)) {
          if (!allow_delegate_fallback) {
            SetError("Failed to create GPU delegate for face mesh because "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_LOGE("Failed to create GPU delegate. Falling back to CPU.\n");
        }
        break;
      }
      case MP_DELEGATE_CPU:
      default:
        break;
    }

    interpreter_.reset(runtime_.InterpreterCreate(model_.get(), options_.get()));
    bool tensors_ready =
        interpreter_ &&
        runtime_.InterpreterAllocateTensors(interpreter_.get()) == kTfLiteOk;
    // A delegate can also fail after it is attached, while the interpreter
    // builds or allocates tensors (e.g. the GPU delegate rejecting a graph
    // with custom ops). Honor delegate fallback for that stage too.
    if (!tensors_ready && active_delegate_ != MP_DELEGATE_CPU &&
        allow_delegate_fallback) {
      MP_LOGE("Interpreter creation with the requested delegate failed. "
              "Falling back to CPU.\n");
      interpreter_.reset();
      options_.reset(runtime_.InterpreterOptionsCreate());
      delegate_.reset();
      if (!options_) {
        SetError("Failed to allocate interpreter options.");
        return false;
      }
      runtime_.InterpreterOptionsSetThreads(options_.get(), threads_);
      active_delegate_ = MP_DELEGATE_CPU;
      interpreter_.reset(
          runtime_.InterpreterCreate(model_.get(), options_.get()));
      tensors_ready =
          interpreter_ &&
          runtime_.InterpreterAllocateTensors(interpreter_.get()) == kTfLiteOk;
    }
    if (!interpreter_) {
      SetError("Failed to create interpreter.");
      return false;
    }
    if (!tensors_ready) {
      SetError("Tensor allocation failed.");
      return false;
    }

    if (runtime_.InterpreterGetInputTensorCount(interpreter_.get()) < 1) {
      SetError("Interpreter input tensor missing.");
      return false;
    }
    input_tensor_ = runtime_.InterpreterGetInputTensor(interpreter_.get(), 0);
    if (!input_tensor_) {
      SetError("Input tensor unavailable.");
      return false;
    }
    if (runtime_.TensorType(input_tensor_) != kTfLiteFloat32) {
      SetError("Model input must be float32.");
      return false;
    }
    if (runtime_.TensorNumDims(input_tensor_) != 4) {
      SetError("Expected NHWC tensor layout.");
      return false;
    }
    const int batch = runtime_.TensorDim(input_tensor_, 0);
    input_height_ = runtime_.TensorDim(input_tensor_, 1);
    input_width_ = runtime_.TensorDim(input_tensor_, 2);
    const int channels = runtime_.TensorDim(input_tensor_, 3);
    if (batch != 1 || channels != 3) {
      SetError("Model expects 1xHxWx3 input.");
      return false;
    }
    input_buffer_.resize(static_cast<size_t>(input_height_ * input_width_ * channels));

    const int output_count =
        runtime_.InterpreterGetOutputTensorCount(interpreter_.get());
    if (output_count < 1) {
      SetError("Model outputs are missing.");
      return false;
    }
    if (attention_enabled_) {
      if (!BindAttentionOutputs(output_count)) {
        return false;
      }
    } else {
      output_landmarks_tensor_ =
          runtime_.InterpreterGetOutputTensor(interpreter_.get(), 0);
      if (!output_landmarks_tensor_) {
        SetError("Landmark tensor missing.");
        return false;
      }
      if (runtime_.TensorType(output_landmarks_tensor_) != kTfLiteFloat32) {
        SetError("Landmark tensor must be float32.");
        return false;
      }
      int total = 1;
      const int dims = runtime_.TensorNumDims(output_landmarks_tensor_);
      for (int i = 0; i < dims; ++i) {
        total *= runtime_.TensorDim(output_landmarks_tensor_, i);
      }
      if (total % 3 != 0) {
        SetError("Unexpected landmark size.");
        return false;
      }
      output_landmark_count_ = total / 3;
      landmarks_buffer_.resize(static_cast<size_t>(total));

      if (output_count > 1) {
        output_score_tensor_ =
            runtime_.InterpreterGetOutputTensor(interpreter_.get(), 1);
        if (output_score_tensor_ &&
            runtime_.TensorType(output_score_tensor_) != kTfLiteFloat32) {
          output_score_tensor_ = nullptr;
        }
      }
    }

    if (iris_enabled_) {
      const char* iris_model_path =
          options ? options->iris_model_path : nullptr;
      if (!iris_model_path || std::strlen(iris_model_path) == 0) {
        SetError("Iris model path is required when iris is enabled.");
        return false;
      }
      if (!InitializeIris(iris_model_path, delegate_choice,
                          allow_delegate_fallback)) {
        return false;
      }
    }

    roi_ = DefaultRect();
    has_valid_rect_ = roi_tracking_enabled_;
    tracking_active_ = false;
    MP_LOGI("Initialize success\n");
    return true;
  }

  /// RGBA/BGRA
  MpFaceMeshResult* Process(const MpImage& image,
                            const MpNormalizedRect* override_rect,
                            int rotation_degrees = 0,
                            bool mirror_horizontal = false) {
    if (!interpreter_) {
      SetError("Interpreter is not initialized.");
      return nullptr;
    }
    if (!image.data || image.width <= 0 || image.height <= 0 ||
        image.bytes_per_row <= 0) {
      SetError("Invalid image buffer.");
      return nullptr;
    }
    if (image.format != MP_PIXEL_FORMAT_RGBA &&
        image.format != MP_PIXEL_FORMAT_BGRA) {
      SetError("Unsupported pixel format. Use RGBA/BGRA.");
      return nullptr;
    }

    const int rot = NormalizeRotationDegrees(rotation_degrees);
    if (rot < 0) {
      SetError("rotation_degrees must be one of 0, 90, 180, 270.");
      return nullptr;
    }

    if (rot != last_rotation_degrees_ ||
        mirror_horizontal != last_mirror_horizontal_) {
      if (roi_tracking_enabled_) {
        has_valid_rect_ = false;
        tracking_active_ = false;
      }
      last_rotation_degrees_ = rot;
      last_mirror_horizontal_ = mirror_horizontal;
    }

    const int logical_width = (rot == 90 || rot == 270) ? image.height : image.width;
    const int logical_height =
        (rot == 90 || rot == 270) ? image.width : image.height;

    MpNormalizedRect rect;
    if (override_rect) {
      rect = SanitizeRect(*override_rect);
    } else if (roi_tracking_enabled_ && has_valid_rect_) {
      rect = roi_;
    } else {
      rect = DefaultRect();
    }

    const bool needs_transform = rot != 0 || mirror_horizontal;
    if (needs_transform) {
      if (!PreprocessRotated(image, rect, rot, mirror_horizontal,
                             logical_width, logical_height)) {
        return nullptr;
      }
    } else {
      if (!Preprocess(image, rect)) {
        return nullptr;
      }
    }

    const size_t bytes = input_buffer_.size() * sizeof(float);
    if (runtime_.TensorCopyFromBuffer(input_tensor_, input_buffer_.data(),
                                      bytes) != kTfLiteOk) {
      SetError("Failed to copy input buffer.");
      return nullptr;
    }

    if (runtime_.InterpreterInvoke(interpreter_.get()) != kTfLiteOk) {
      SetError("Interpreter invocation failed.");
      return nullptr;
    }

    float raw_score = 0.0f;
    if (attention_enabled_) {
      if (!ReadAttentionLandmarks(&raw_score)) {
        return nullptr;
      }
    } else {
      if (runtime_.TensorCopyToBuffer(
              output_landmarks_tensor_, landmarks_buffer_.data(),
              landmarks_buffer_.size() * sizeof(float)) != kTfLiteOk) {
        SetError("Unable to read landmark output.");
        return nullptr;
      }
      if (output_score_tensor_) {
        if (runtime_.TensorCopyToBuffer(output_score_tensor_, &raw_score,
                                        sizeof(float)) != kTfLiteOk) {
          SetError("Unable to read confidence output.");
          return nullptr;
        }
      }
    }
    const float face_presence_score =
        1.0f / (1.0f + std::exp(-raw_score));

    MpFaceMeshResult* result =
        BuildResultFromSize(logical_width, logical_height, rect,
                            face_presence_score);
    if (!result) {
      return nullptr;
    }

    if (face_presence_score < min_face_presence_confidence_) {
      result->landmarks_count = 0;
      delete[] result->landmarks;
      result->landmarks = nullptr;
      if (roi_tracking_enabled_ && !override_rect) {
        has_valid_rect_ = false;
        tracking_active_ = false;
      }
      return result;
    }

    if (iris_enabled_ &&
        !AugmentResultWithIris(image, result, rot, mirror_horizontal,
                               logical_width, logical_height)) {
      delete[] result->landmarks;
      delete result;
      return nullptr;
    }

    if (roi_tracking_enabled_) {
      if (!override_rect) {
        UpdateTrackingState(*result, face_presence_score);
      } else {
        // Seed tracking from the just-computed landmarks rather than the
        // override (detector) rect so the next tracked frame crops the
        // actual face extent instead of the imprecise detection box.
        roi_ = SanitizeRect(RectFromLandmarks(
            result->landmarks, result->landmarks_count,
            result->image_width, result->image_height));
        has_valid_rect_ = true;
        tracking_active_ = true;
      }
    }

    return result;
  }

  MpFaceMeshResult* ProcessNv21(const MpNv21Image& image,
                               const MpNormalizedRect* override_rect,
                               int rotation_degrees = 0,
                               bool mirror_horizontal = false) {
    if (!interpreter_) {
      SetError("Interpreter is not initialized.");
      return nullptr;
    }
    if (!image.y || !image.vu || image.width <= 0 || image.height <= 0 ||
        image.y_bytes_per_row <= 0 || image.vu_bytes_per_row <= 0) {
      SetError("Invalid NV21 image buffer.");
      return nullptr;
    }

    const int rot = NormalizeRotationDegrees(rotation_degrees);
    if (rot < 0) {
      SetError("rotation_degrees must be one of 0, 90, 180, 270.");
      return nullptr;
    }

    // Reset tracking state when the logical coordinate system changes.
    if (rot != last_rotation_degrees_ ||
        mirror_horizontal != last_mirror_horizontal_) {
      if (roi_tracking_enabled_) {
        has_valid_rect_ = false;
        tracking_active_ = false;
      }
      last_rotation_degrees_ = rot;
      last_mirror_horizontal_ = mirror_horizontal;
    }

    const int logical_width = (rot == 90 || rot == 270) ? image.height : image.width;
    const int logical_height = (rot == 90 || rot == 270) ? image.width : image.height;

    MpNormalizedRect rect;
    if (override_rect) {
      rect = SanitizeRect(*override_rect);
    } else if (roi_tracking_enabled_ && has_valid_rect_) {
      rect = roi_;
    } else {
      rect = DefaultRect();
    }

    const bool needs_transform = rot != 0 || mirror_horizontal;
    if (needs_transform) {
      if (!PreprocessNv21Rotated(image, rect, rot, mirror_horizontal,
                                 logical_width, logical_height)) {
        return nullptr;
      }
    } else {
      if (!PreprocessNv21(image, rect)) {
        return nullptr;
      }
    }

    const size_t bytes = input_buffer_.size() * sizeof(float);
    if (runtime_.TensorCopyFromBuffer(input_tensor_, input_buffer_.data(),
                                      bytes) != kTfLiteOk) {
      SetError("Failed to copy input buffer.");
      return nullptr;
    }

    if (runtime_.InterpreterInvoke(interpreter_.get()) != kTfLiteOk) {
      SetError("Interpreter invocation failed.");
      return nullptr;
    }

    float raw_score = 0.0f;
    if (attention_enabled_) {
      if (!ReadAttentionLandmarks(&raw_score)) {
        return nullptr;
      }
    } else {
      if (runtime_.TensorCopyToBuffer(
              output_landmarks_tensor_, landmarks_buffer_.data(),
              landmarks_buffer_.size() * sizeof(float)) != kTfLiteOk) {
        SetError("Unable to read landmark output.");
        return nullptr;
      }
      if (output_score_tensor_) {
        if (runtime_.TensorCopyToBuffer(output_score_tensor_, &raw_score,
                                        sizeof(float)) != kTfLiteOk) {
          SetError("Unable to read confidence output.");
          return nullptr;
        }
      }
    }
    const float face_presence_score =
        1.0f / (1.0f + std::exp(-raw_score));

    MpFaceMeshResult* result =
        BuildResultFromSize(logical_width, logical_height, rect,
                            face_presence_score);
    if (!result) {
      return nullptr;
    }

    if (face_presence_score < min_face_presence_confidence_) {
      result->landmarks_count = 0;
      delete[] result->landmarks;
      result->landmarks = nullptr;
      if (roi_tracking_enabled_ && !override_rect) {
        has_valid_rect_ = false;
        tracking_active_ = false;
      }
      return result;
    }

    if (iris_enabled_ &&
        !AugmentResultWithIrisNv21(image, result, rot, mirror_horizontal,
                                   logical_width, logical_height)) {
      delete[] result->landmarks;
      delete result;
      return nullptr;
    }

    if (roi_tracking_enabled_) {
      if (!override_rect) {
        UpdateTrackingState(*result, face_presence_score);
      } else {
        // Seed tracking from the just-computed landmarks rather than the
        // override (detector) rect so the next tracked frame crops the
        // actual face extent instead of the imprecise detection box.
        roi_ = SanitizeRect(RectFromLandmarks(
            result->landmarks, result->landmarks_count,
            result->image_width, result->image_height));
        has_valid_rect_ = true;
        tracking_active_ = true;
      }
    }

    return result;
  }

  const char* last_error() const { return last_error_.c_str(); }

  // Records an error raised outside the class, e.g. a C++ exception caught
  // at the FFI boundary.
  void RecordError(const std::string& message) { SetError(message); }

  // Whether the internal ROI currently follows a face (was seeded from
  // landmarks and has not been dropped by a confidence failure, tracking
  // loss, or an input-geometry change).
  bool is_tracking() const { return tracking_active_; }

  MpDelegateType active_delegate() const { return active_delegate_; }

  MpDelegateType active_iris_delegate() const { return active_iris_delegate_; }

 private:
  struct TfLiteModelDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteModel* model) const {
      if (runtime && model) {
        runtime->ModelDelete(model);
      }
    }
  };

  struct TfLiteOptionsDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteInterpreterOptions* options) const {
      if (runtime && options) {
        runtime->InterpreterOptionsDelete(options);
      }
    }
  };

  struct TfLiteInterpreterDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteInterpreter* interpreter) const {
      if (runtime && interpreter) {
        runtime->InterpreterDelete(interpreter);
      }
    }
  };

  struct TfLiteDelegateDeleter {
    using DeleteFn = void (*)(TfLiteDelegate*);
    DeleteFn deleter = nullptr;
    void operator()(TfLiteDelegate* delegate) const {
      if (deleter && delegate) {
        deleter(delegate);
      }
    }
  };

  void Shutdown() {
    iris_interpreter_.reset();
    iris_options_.reset();
    iris_model_.reset();
    iris_delegate_.reset();
    interpreter_.reset();
    options_.reset();
    model_.reset();
    delegate_.reset();
    runtime_.Release();
  }

  bool InitializeIris(const std::string& iris_model_path,
                      MpDelegateType delegate_choice,
                      bool allow_delegate_fallback) {
    iris_model_.reset(runtime_.ModelCreateFromFile(iris_model_path.c_str()));
    if (!iris_model_) {
      SetError("Unable to load iris model file: " + iris_model_path);
      return false;
    }

    iris_options_.reset(runtime_.InterpreterOptionsCreate());
    if (!iris_options_) {
      SetError("Failed to allocate iris interpreter options.");
      return false;
    }
    runtime_.InterpreterOptionsSetThreads(iris_options_.get(), threads_);

    active_iris_delegate_ = MP_DELEGATE_CPU;
    auto AttachIrisDelegate = [&](TfLiteDelegate* created,
                                  TfLiteDelegateDeleter::DeleteFn deleter,
                                  const char* name,
                                  MpDelegateType delegate_type) {
      if (!created) {
        return false;
      }
      iris_delegate_.get_deleter().deleter = deleter;
      iris_delegate_.reset(created);
      runtime_.InterpreterOptionsAddDelegate(
          iris_options_.get(),
          reinterpret_cast<TfLiteOpaqueDelegate*>(iris_delegate_.get()));
      active_iris_delegate_ = delegate_type;
      MP_LOGI("Iris %s delegate enabled.\n", name);
      return true;
    };

    switch (delegate_choice) {
      case MP_DELEGATE_XNNPACK: {
        if (runtime_.InterpreterOptionsAddDelegate &&
            runtime_.XnnpackDelegateOptionsDefault &&
            runtime_.XnnpackDelegateCreate && runtime_.XnnpackDelegateDelete) {
          TfLiteXNNPackDelegateOptions xnnpack_options =
              runtime_.XnnpackDelegateOptionsDefault();
          xnnpack_options.num_threads = threads_;
          AttachIrisDelegate(runtime_.XnnpackDelegateCreate(&xnnpack_options),
                             runtime_.XnnpackDelegateDelete, "XNNPACK",
                             MP_DELEGATE_XNNPACK);
        } else if (!allow_delegate_fallback) {
          SetError("XNNPACK delegate is unavailable for iris model and "
                   "delegate fallback is disabled.");
          return false;
        }
        break;
      }
      case MP_DELEGATE_GPU_V2: {
        if (runtime_.InterpreterOptionsAddDelegate &&
            runtime_.GpuDelegateV2OptionsDefault &&
            runtime_.GpuDelegateV2Create && runtime_.GpuDelegateV2Delete) {
          TfLiteGpuDelegateOptionsV2 gpu_options =
              runtime_.GpuDelegateV2OptionsDefault();
          gpu_options.experimental_flags |=
              TFLITE_GPU_EXPERIMENTAL_FLAGS_ENABLE_QUANT;
          AttachIrisDelegate(runtime_.GpuDelegateV2Create(&gpu_options),
                             runtime_.GpuDelegateV2Delete, "GPU V2",
                             MP_DELEGATE_GPU_V2);
        } else if (!allow_delegate_fallback) {
          SetError("GPU delegate (V2) is unavailable for iris model and "
                   "delegate fallback is disabled.");
          return false;
        }
        break;
      }
      case MP_DELEGATE_CPU:
      default:
        break;
    }

    if (!allow_delegate_fallback && delegate_choice != MP_DELEGATE_CPU &&
        active_iris_delegate_ != delegate_choice) {
      SetError("Failed to create requested delegate for iris model because "
               "delegate fallback is disabled.");
      return false;
    }

    iris_interpreter_.reset(
        runtime_.InterpreterCreate(iris_model_.get(), iris_options_.get()));
    bool iris_tensors_ready =
        iris_interpreter_ &&
        runtime_.InterpreterAllocateTensors(iris_interpreter_.get()) ==
            kTfLiteOk;
    // A delegate can also fail after it is attached, while the interpreter
    // builds or allocates tensors (e.g. the GPU delegate rejecting a graph
    // with custom ops). Honor delegate fallback for that stage too.
    if (!iris_tensors_ready && active_iris_delegate_ != MP_DELEGATE_CPU &&
        allow_delegate_fallback) {
      MP_LOGE("Iris interpreter creation with the requested delegate failed. "
              "Falling back to CPU.\n");
      iris_interpreter_.reset();
      iris_options_.reset(runtime_.InterpreterOptionsCreate());
      iris_delegate_.reset();
      if (!iris_options_) {
        SetError("Failed to allocate iris interpreter options.");
        return false;
      }
      runtime_.InterpreterOptionsSetThreads(iris_options_.get(), threads_);
      active_iris_delegate_ = MP_DELEGATE_CPU;
      iris_interpreter_.reset(
          runtime_.InterpreterCreate(iris_model_.get(), iris_options_.get()));
      iris_tensors_ready =
          iris_interpreter_ &&
          runtime_.InterpreterAllocateTensors(iris_interpreter_.get()) ==
              kTfLiteOk;
    }
    if (!iris_interpreter_) {
      SetError("Failed to create iris interpreter.");
      return false;
    }
    if (!iris_tensors_ready) {
      SetError("Iris tensor allocation failed.");
      return false;
    }
    if (runtime_.InterpreterGetInputTensorCount(iris_interpreter_.get()) < 1) {
      SetError("Iris interpreter input tensor missing.");
      return false;
    }
    iris_input_tensor_ =
        runtime_.InterpreterGetInputTensor(iris_interpreter_.get(), 0);
    if (!iris_input_tensor_ ||
        runtime_.TensorType(iris_input_tensor_) != kTfLiteFloat32 ||
        runtime_.TensorNumDims(iris_input_tensor_) != 4) {
      SetError("Iris model input must be float32 NHWC.");
      return false;
    }
    iris_input_height_ = runtime_.TensorDim(iris_input_tensor_, 1);
    iris_input_width_ = runtime_.TensorDim(iris_input_tensor_, 2);
    const int iris_channels = runtime_.TensorDim(iris_input_tensor_, 3);
    if (runtime_.TensorDim(iris_input_tensor_, 0) != 1 || iris_channels != 3) {
      SetError("Iris model expects 1xHxWx3 input.");
      return false;
    }
    if (iris_input_width_ != kIrisInputSize ||
        iris_input_height_ != kIrisInputSize) {
      MP_LOGI("Iris input is %dx%d, expected 64x64.\n", iris_input_width_,
              iris_input_height_);
    }
    iris_input_buffer_.resize(
        static_cast<size_t>(iris_input_width_ * iris_input_height_ * 3));

    if (runtime_.InterpreterGetOutputTensorCount(iris_interpreter_.get()) < 2) {
      SetError("Iris model must expose eye contour and iris outputs.");
      return false;
    }
    iris_eye_tensor_ =
        runtime_.InterpreterGetOutputTensor(iris_interpreter_.get(), 0);
    iris_landmarks_tensor_ =
        runtime_.InterpreterGetOutputTensor(iris_interpreter_.get(), 1);
    if (!iris_eye_tensor_ || !iris_landmarks_tensor_ ||
        runtime_.TensorType(iris_eye_tensor_) != kTfLiteFloat32 ||
        runtime_.TensorType(iris_landmarks_tensor_) != kTfLiteFloat32) {
      SetError("Iris output tensors must be float32.");
      return false;
    }
    iris_eye_buffer_.resize(TensorElementCount(iris_eye_tensor_));
    iris_landmarks_buffer_.resize(TensorElementCount(iris_landmarks_tensor_));
    if (iris_eye_buffer_.size() < kEyeLandmarkCount * 3 ||
        iris_landmarks_buffer_.size() < kIrisLandmarkCount * 3) {
      SetError("Unexpected iris output tensor sizes.");
      return false;
    }
    MP_LOGI("Iris initialize success\n");
    return true;
  }

  MpNormalizedRect DefaultRect() const {
    MpNormalizedRect rect;
    rect.x_center = 0.5f;
    rect.y_center = 0.5f;
    rect.width = 1.0f;
    rect.height = 1.0f;
    rect.rotation = 0.0f;
    return rect;
  }

  MpNormalizedRect SanitizeRect(MpNormalizedRect rect) const {
    if (!(rect.width > 0.f) || !(rect.height > 0.f)) {
      return DefaultRect();
    }
    rect.x_center = Clamp(rect.x_center, 0.0f, 1.0f);
    rect.y_center = Clamp(rect.y_center, 0.0f, 1.0f);
    // Clamp the ROI size with one scale factor so the width:height ratio
    // (pixel-space squareness) survives; independent per-axis clamps would
    // stretch small or very large ROIs anisotropically.
    const float long_dim = std::max(rect.width, rect.height);
    const float short_dim = std::min(rect.width, rect.height);
    float scale = 1.0f;
    if (long_dim > 2.0f) {
      scale = 2.0f / long_dim;
    }
    if (short_dim * scale < 0.1f) {
      scale = 0.1f / short_dim;
    }
    rect.width *= scale;
    rect.height *= scale;
    rect.rotation = NormalizeAngle(rect.rotation);
    return rect;
  }

  static int NormalizeRotationDegrees(int rotation_degrees) {
    switch (rotation_degrees) {
      case 0:
      case 90:
      case 180:
      case 270:
        return rotation_degrees;
      default:
        return -1;
    }
  }

  bool Preprocess(const MpImage& image, const MpNormalizedRect& rect) {
    const RectInPixels roi = ToPixelRect(rect, image.width, image.height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;

    float* dst = input_buffer_.data();
    const int target_w = input_width_;
    const int target_h = input_height_;

    size_t offset = 0;
    for (int y = 0; y < target_h; ++y) {
      const float ny =
          ((static_cast<float>(y) + 0.5f) / static_cast<float>(target_h) - 0.5f) *
          2.0f;
      for (int x = 0; x < target_w; ++x) {
        const float nx = ((static_cast<float>(x) + 0.5f) /
                              static_cast<float>(target_w) -
                          0.5f) *
                         2.0f;
        const float rx = nx * half_w;
        const float ry = ny * half_h;
        const float source_x = cos_r * rx - sin_r * ry + roi.center_x;
        const float source_y = sin_r * rx + cos_r * ry + roi.center_y;
        const RgbPixel pixel = BilinearSample(image, source_x, source_y);
        dst[offset++] = pixel.r / 255.0f;
        dst[offset++] = pixel.g / 255.0f;
        dst[offset++] = pixel.b / 255.0f;
      }
    }
    return true;
  }

  bool PreprocessRotated(const MpImage& image,
                         const MpNormalizedRect& rect,
                         int rotation_degrees,
                         bool mirror_horizontal,
                         int rotated_width,
                         int rotated_height) {
    const RectInPixels roi = ToPixelRect(rect, rotated_width, rotated_height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;

    float* dst = input_buffer_.data();
    const int target_w = input_width_;
    const int target_h = input_height_;

    size_t offset = 0;
    for (int y = 0; y < target_h; ++y) {
      const float ny =
          ((static_cast<float>(y) + 0.5f) / static_cast<float>(target_h) -
           0.5f) *
          2.0f;
      for (int x = 0; x < target_w; ++x) {
        const float nx =
            ((static_cast<float>(x) + 0.5f) / static_cast<float>(target_w) -
             0.5f) *
            2.0f;
        const float rx = nx * half_w;
        const float ry = ny * half_h;
        const float source_x = cos_r * rx - sin_r * ry + roi.center_x;
        const float source_y = sin_r * rx + cos_r * ry + roi.center_y;
        const RgbPixel pixel = BilinearSampleRotated(
            image, source_x, source_y, rotation_degrees, mirror_horizontal,
            rotated_width, rotated_height);
        dst[offset++] = pixel.r / 255.0f;
        dst[offset++] = pixel.g / 255.0f;
        dst[offset++] = pixel.b / 255.0f;
      }
    }
    return true;
  }

  bool PreprocessNv21(const MpNv21Image& image, const MpNormalizedRect& rect) {
    const RectInPixels roi = ToPixelRect(rect, image.width, image.height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;

    float* dst = input_buffer_.data();
    const int target_w = input_width_;
    const int target_h = input_height_;

    size_t offset = 0;
    for (int y = 0; y < target_h; ++y) {
      const float ny =
          ((static_cast<float>(y) + 0.5f) / static_cast<float>(target_h) -
           0.5f) *
          2.0f;
      for (int x = 0; x < target_w; ++x) {
        const float nx =
            ((static_cast<float>(x) + 0.5f) / static_cast<float>(target_w) -
             0.5f) *
            2.0f;
        const float rx = nx * half_w;
        const float ry = ny * half_h;
        const float source_x = cos_r * rx - sin_r * ry + roi.center_x;
        const float source_y = sin_r * rx + cos_r * ry + roi.center_y;
        const RgbPixel pixel = BilinearSampleNv21(image, source_x, source_y);
        dst[offset++] = pixel.r / 255.0f;
        dst[offset++] = pixel.g / 255.0f;
        dst[offset++] = pixel.b / 255.0f;
      }
    }
    return true;
  }

  bool PreprocessNv21Rotated(const MpNv21Image& image,
                             const MpNormalizedRect& rect,
                             int rotation_degrees,
                             bool mirror_horizontal,
                             int rotated_width,
                             int rotated_height) {
    const RectInPixels roi = ToPixelRect(rect, rotated_width, rotated_height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;

    float* dst = input_buffer_.data();
    const int target_w = input_width_;
    const int target_h = input_height_;

    size_t offset = 0;
    for (int y = 0; y < target_h; ++y) {
      const float ny =
          ((static_cast<float>(y) + 0.5f) / static_cast<float>(target_h) -
           0.5f) *
          2.0f;
      for (int x = 0; x < target_w; ++x) {
        const float nx =
            ((static_cast<float>(x) + 0.5f) / static_cast<float>(target_w) -
             0.5f) *
            2.0f;
        const float rx = nx * half_w;
        const float ry = ny * half_h;
        const float source_x = cos_r * rx - sin_r * ry + roi.center_x;
        const float source_y = sin_r * rx + cos_r * ry + roi.center_y;
        const RgbPixel pixel = BilinearSampleNv21Rotated(
            image, source_x, source_y, rotation_degrees, mirror_horizontal,
            rotated_width, rotated_height);
        dst[offset++] = pixel.r / 255.0f;
        dst[offset++] = pixel.g / 255.0f;
        dst[offset++] = pixel.b / 255.0f;
      }
    }
    return true;
  }

  RgbPixel BilinearSample(const MpImage& image, float x, float y) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(image.width - 1) ||
        y > static_cast<float>(image.height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, image.width - 1);
    const int y1 = std::min(y0 + 1, image.height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixel(image, x0, y0);
    const RgbPixel p10 = ReadPixel(image, x1, y0);
    const RgbPixel p01 = ReadPixel(image, x0, y1);
    const RgbPixel p11 = ReadPixel(image, x1, y1);

    const RgbPixel top = Lerp(p00, p10, dx);
    const RgbPixel bottom = Lerp(p01, p11, dx);
    return Lerp(top, bottom, dy);
  }

  RgbPixel BilinearSampleRotated(const MpImage& image,
                                 float x,
                                 float y,
                                 int rotation_degrees,
                                 bool mirror_horizontal,
                                 int rotated_width,
                                 int rotated_height) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(rotated_width - 1) ||
        y > static_cast<float>(rotated_height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, rotated_width - 1);
    const int y1 = std::min(y0 + 1, rotated_height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixelRotated(
        image, x0, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p10 = ReadPixelRotated(
        image, x1, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p01 = ReadPixelRotated(
        image, x0, y1, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p11 = ReadPixelRotated(
        image, x1, y1, rotation_degrees, mirror_horizontal, rotated_width);

    const RgbPixel top = Lerp(p00, p10, dx);
    const RgbPixel bottom = Lerp(p01, p11, dx);
    return Lerp(top, bottom, dy);
  }

  RgbPixel BilinearSampleNv21(const MpNv21Image& image,
                              float x,
                              float y) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(image.width - 1) ||
        y > static_cast<float>(image.height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, image.width - 1);
    const int y1 = std::min(y0 + 1, image.height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixelNv21(image, x0, y0);
    const RgbPixel p10 = ReadPixelNv21(image, x1, y0);
    const RgbPixel p01 = ReadPixelNv21(image, x0, y1);
    const RgbPixel p11 = ReadPixelNv21(image, x1, y1);

    const RgbPixel top = Lerp(p00, p10, dx);
    const RgbPixel bottom = Lerp(p01, p11, dx);
    return Lerp(top, bottom, dy);
  }

  RgbPixel BilinearSampleNv21Rotated(const MpNv21Image& image,
                                     float x,
                                     float y,
                                     int rotation_degrees,
                                     bool mirror_horizontal,
                                     int rotated_width,
                                     int rotated_height) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(rotated_width - 1) ||
        y > static_cast<float>(rotated_height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, rotated_width - 1);
    const int y1 = std::min(y0 + 1, rotated_height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixelNv21Rotated(
        image, x0, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p10 = ReadPixelNv21Rotated(
        image, x1, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p01 = ReadPixelNv21Rotated(
        image, x0, y1, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p11 = ReadPixelNv21Rotated(
        image, x1, y1, rotation_degrees, mirror_horizontal, rotated_width);

    const RgbPixel top = Lerp(p00, p10, dx);
    const RgbPixel bottom = Lerp(p01, p11, dx);
    return Lerp(top, bottom, dy);
  }

  RgbPixel ReadPixel(const MpImage& image, int x, int y) const {
    const uint8_t* row =
        image.data + static_cast<size_t>(y) * image.bytes_per_row;
    const uint8_t* ptr = row + static_cast<size_t>(x) * 4;
    RgbPixel pixel;
    if (image.format == MP_PIXEL_FORMAT_RGBA) {
      pixel.r = static_cast<float>(ptr[0]);
      pixel.g = static_cast<float>(ptr[1]);
      pixel.b = static_cast<float>(ptr[2]);
    } else {
      pixel.r = static_cast<float>(ptr[2]);
      pixel.g = static_cast<float>(ptr[1]);
      pixel.b = static_cast<float>(ptr[0]);
    }
    return pixel;
  }

  RgbPixel ReadPixelNv21(const MpNv21Image& image, int x, int y) const {
    const uint8_t* y_row =
        image.y + static_cast<size_t>(y) * image.y_bytes_per_row;
    const uint8_t Y = y_row[static_cast<size_t>(x)];

    const int uv_x = x >> 1;
    const int uv_y = y >> 1;
    const uint8_t* vu_row =
        image.vu + static_cast<size_t>(uv_y) * image.vu_bytes_per_row;
    const size_t vu_index = static_cast<size_t>(uv_x) * 2;
    const uint8_t V = vu_row[vu_index];
    const uint8_t U = vu_row[vu_index + 1];

    const int C = static_cast<int>(Y) - 16;
    const int D = static_cast<int>(U) - 128;
    const int E = static_cast<int>(V) - 128;
    const int c = C < 0 ? 0 : C;

    const int r = (298 * c + 409 * E + 128) >> 8;
    const int g = (298 * c - 100 * D - 208 * E + 128) >> 8;
    const int b = (298 * c + 516 * D + 128) >> 8;

    RgbPixel pixel;
    pixel.r = static_cast<float>(ClampInt(r, 0, 255));
    pixel.g = static_cast<float>(ClampInt(g, 0, 255));
    pixel.b = static_cast<float>(ClampInt(b, 0, 255));
    return pixel;
  }

  RgbPixel ReadPixelRotated(const MpImage& image,
                            int x_rot,
                            int y_rot,
                            int rotation_degrees,
                            bool mirror_horizontal,
                            int rotated_width) const {
    int x_raw = 0;
    int y_raw = 0;
    MapRotatedToRaw(x_rot, y_rot, rotation_degrees, mirror_horizontal,
                    image.width, image.height, rotated_width, x_raw, y_raw);
    return ReadPixel(image, x_raw, y_raw);
  }

  static inline void MapRotatedToRaw(int x_rot,
                                     int y_rot,
                                     int rotation_degrees,
                                     bool mirror_horizontal,
                                     int raw_width,
                                     int raw_height,
                                     int rotated_width,
                                     int& out_x,
                                     int& out_y) {
    int xr = x_rot;
    int yr = y_rot;
    if (mirror_horizontal) {
      xr = (rotated_width - 1) - xr;
    }
    switch (rotation_degrees) {
      case 90:
        out_x = yr;
        out_y = (raw_height - 1) - xr;
        break;
      case 180:
        out_x = (raw_width - 1) - xr;
        out_y = (raw_height - 1) - yr;
        break;
      case 270:
        out_x = (raw_width - 1) - yr;
        out_y = xr;
        break;
      case 0:
      default:
        out_x = xr;
        out_y = yr;
        break;
    }
    out_x = ClampInt(out_x, 0, raw_width - 1);
    out_y = ClampInt(out_y, 0, raw_height - 1);
  }

  RgbPixel ReadPixelNv21Rotated(const MpNv21Image& image,
                                int x_rot,
                                int y_rot,
                                int rotation_degrees,
                                bool mirror_horizontal,
                                int rotated_width) const {
    int x_raw = 0;
    int y_raw = 0;
    MapRotatedToRaw(x_rot, y_rot, rotation_degrees, mirror_horizontal,
                    image.width, image.height, rotated_width,
                    x_raw, y_raw);
    return ReadPixelNv21(image, x_raw, y_raw);
  }

  RgbPixel Lerp(const RgbPixel& a, const RgbPixel& b, float t) const {
    const float blend = Clamp(t, 0.0f, 1.0f);
    RgbPixel out;
    out.r = a.r + (b.r - a.r) * blend;
    out.g = a.g + (b.g - a.g) * blend;
    out.b = a.b + (b.b - a.b) * blend;
    return out;
  }

  MpFaceMeshResult* BuildResult(const MpImage& image,
                                const MpNormalizedRect& rect,
                                float score) {
    return BuildResultFromSize(image.width, image.height, rect, score);
  }

  MpFaceMeshResult* BuildResultFromSize(int width,
                                        int height,
                                        const MpNormalizedRect& rect,
                                        float score) {
    auto* result = new MpFaceMeshResult();
    if (!result) {
      SetError("Unable to allocate result.");
      return nullptr;
    }
    result->landmarks_count = output_landmark_count_;
    result->landmarks = new MpLandmark[output_landmark_count_];
    if (!result->landmarks) {
      SetError("Unable to allocate landmarks buffer.");
      delete result;
      return nullptr;
    }
    result->rect = rect;
    result->score = score;
    result->image_width = width;
    result->image_height = height;

    const RectInPixels roi = ToPixelRect(rect, width, height);
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;
    const float input_w = std::max(1, input_width_);
    const float input_h = std::max(1, input_height_);
    // Some models emit normalized [0,1], others emit pixel coordinates in
    // input resolution; the whole buffer is one or the other.
    const bool pixel_space =
        IsPixelSpaceLandmarkBuffer(landmarks_buffer_.data(),
                                   output_landmark_count_);

    for (int i = 0; i < output_landmark_count_; ++i) {
      float raw_x = landmarks_buffer_[i * 3];
      float raw_y = landmarks_buffer_[i * 3 + 1];
      float raw_z = landmarks_buffer_[i * 3 + 2];

      if (pixel_space) {
        raw_x = raw_x / input_w;
        raw_y = raw_y / input_h;
        raw_z = raw_z / input_w;
      }

      const float nx = (raw_x - 0.5f) * 2.0f;
      const float ny = (raw_y - 0.5f) * 2.0f;
      const float rx = nx * half_w;
      const float ry = ny * half_h;

      const float abs_x = cos_r * rx - sin_r * ry + roi.center_x;
      const float abs_y = sin_r * rx + cos_r * ry + roi.center_y;
      const float abs_z = raw_z * roi.width;

      MpLandmark& landmark = result->landmarks[i];
      landmark.x =
          Clamp(abs_x / static_cast<float>(width), -0.5f, 1.5f);
      landmark.y =
          Clamp(abs_y / static_cast<float>(height), -0.5f, 1.5f);
      landmark.z = abs_z / static_cast<float>(width);
    }
    return result;
  }

  // Binds and validates the 7 output tensors of the attention model by their
  // fixed output-index order and expected element counts.
  bool BindAttentionOutputs(int output_count) {
    if (output_count < 7) {
      SetError("Attention model expects 7 output tensors.");
      return false;
    }
    struct Bind {
      const TfLiteTensor** dst;
      int index;
      int expected;
      const char* name;
    };
    const Bind binds[] = {
        {&attn_mesh_tensor_, 0, kFaceLandmarkCount * 3, "mesh"},
        {&attn_lips_tensor_, 1, kLipsLandmarkCount * 2, "lips"},
        {&attn_left_eye_tensor_, 2, kEyeLandmarkCount * 2, "left_eye"},
        {&attn_right_eye_tensor_, 3, kEyeLandmarkCount * 2, "right_eye"},
        {&attn_left_iris_tensor_, 4, kIrisLandmarkCount * 2, "left_iris"},
        {&attn_right_iris_tensor_, 5, kIrisLandmarkCount * 2, "right_iris"},
        {&attn_faceflag_tensor_, 6, 1, "faceflag"},
    };
    for (const Bind& b : binds) {
      const TfLiteTensor* t =
          runtime_.InterpreterGetOutputTensor(interpreter_.get(), b.index);
      if (!t || runtime_.TensorType(t) != kTfLiteFloat32 ||
          static_cast<int>(TensorElementCount(t)) != b.expected) {
        SetError(std::string("Attention output tensor mismatch: ") + b.name);
        return false;
      }
      *b.dst = t;
    }
    output_landmark_count_ = kAttentionLandmarkCount;
    landmarks_buffer_.resize(static_cast<size_t>(kAttentionLandmarkCount * 3));
    attn_lips_buffer_.resize(static_cast<size_t>(kLipsLandmarkCount * 2));
    attn_left_eye_buffer_.resize(static_cast<size_t>(kEyeLandmarkCount * 2));
    attn_right_eye_buffer_.resize(static_cast<size_t>(kEyeLandmarkCount * 2));
    attn_left_iris_buffer_.resize(static_cast<size_t>(kIrisLandmarkCount * 2));
    attn_right_iris_buffer_.resize(static_cast<size_t>(kIrisLandmarkCount * 2));
    return true;
  }

  // Reads the attention output tensors and assembles the 478-landmark buffer in
  // the model's input-pixel units (matching the mesh path so BuildResultFromSize
  // applies the same transform). Also returns the raw face-presence logit.
  bool ReadAttentionLandmarks(float* raw_score_out) {
    if (runtime_.TensorCopyToBuffer(attn_mesh_tensor_, landmarks_buffer_.data(),
                                    kFaceLandmarkCount * 3 * sizeof(float)) !=
        kTfLiteOk) {
      SetError("Unable to read attention mesh output.");
      return false;
    }
    struct Sub {
      const TfLiteTensor* tensor;
      std::vector<float>* buffer;
      const char* name;
    };
    const Sub subs[] = {
        {attn_lips_tensor_, &attn_lips_buffer_, "lips"},
        {attn_left_eye_tensor_, &attn_left_eye_buffer_, "left_eye"},
        {attn_right_eye_tensor_, &attn_right_eye_buffer_, "right_eye"},
        {attn_left_iris_tensor_, &attn_left_iris_buffer_, "left_iris"},
        {attn_right_iris_tensor_, &attn_right_iris_buffer_, "right_iris"},
    };
    for (const Sub& s : subs) {
      if (runtime_.TensorCopyToBuffer(s.tensor, s.buffer->data(),
                                      s.buffer->size() * sizeof(float)) !=
          kTfLiteOk) {
        SetError(std::string("Unable to read attention output: ") + s.name);
        return false;
      }
    }

    // Refined lips overwrite x,y of their mesh indices; z is kept from the mesh.
    for (int k = 0; k < kLipsLandmarkCount; ++k) {
      const int dst = kLipsLandmarkIndicesInFaceLandmarks[k];
      landmarks_buffer_[dst * 3] = attn_lips_buffer_[k * 2];
      landmarks_buffer_[dst * 3 + 1] = attn_lips_buffer_[k * 2 + 1];
    }
    // Refined eye contours (left = first half, right = second half of the table).
    for (int k = 0; k < kEyeLandmarkCount; ++k) {
      const int ldst = kEyeLandmarkIndicesInFaceLandmarks[k];
      landmarks_buffer_[ldst * 3] = attn_left_eye_buffer_[k * 2];
      landmarks_buffer_[ldst * 3 + 1] = attn_left_eye_buffer_[k * 2 + 1];
      const int rdst =
          kEyeLandmarkIndicesInFaceLandmarks[kEyeLandmarkCount + k];
      landmarks_buffer_[rdst * 3] = attn_right_eye_buffer_[k * 2];
      landmarks_buffer_[rdst * 3 + 1] = attn_right_eye_buffer_[k * 2 + 1];
    }
    // Irises appended at 468..472 / 473..477; z is the average z of the
    // corresponding eye contour (assign_average in the reference graph).
    const float left_iris_z = AverageContourZ(0);
    const float right_iris_z = AverageContourZ(kEyeLandmarkCount);
    for (int k = 0; k < kIrisLandmarkCount; ++k) {
      const int ldst = kFaceLandmarkCount + k;
      landmarks_buffer_[ldst * 3] = attn_left_iris_buffer_[k * 2];
      landmarks_buffer_[ldst * 3 + 1] = attn_left_iris_buffer_[k * 2 + 1];
      landmarks_buffer_[ldst * 3 + 2] = left_iris_z;
      const int rdst = kFaceLandmarkCount + kIrisLandmarkCount + k;
      landmarks_buffer_[rdst * 3] = attn_right_iris_buffer_[k * 2];
      landmarks_buffer_[rdst * 3 + 1] = attn_right_iris_buffer_[k * 2 + 1];
      landmarks_buffer_[rdst * 3 + 2] = right_iris_z;
    }

    float raw = 0.0f;
    if (runtime_.TensorCopyToBuffer(attn_faceflag_tensor_, &raw,
                                    sizeof(float)) != kTfLiteOk) {
      SetError("Unable to read attention face-flag output.");
      return false;
    }
    *raw_score_out = raw;
    return true;
  }

  // Average z over the first kIrisZAverageCount contour points starting at
  // [offset] within kEyeLandmarkIndicesInFaceLandmarks.
  float AverageContourZ(int offset) const {
    float sum = 0.0f;
    for (int i = 0; i < kIrisZAverageCount; ++i) {
      const int idx = kEyeLandmarkIndicesInFaceLandmarks[offset + i];
      sum += landmarks_buffer_[idx * 3 + 2];
    }
    return sum / static_cast<float>(kIrisZAverageCount);
  }

  size_t TensorElementCount(const TfLiteTensor* tensor) const {
    int total = 1;
    const int dims = runtime_.TensorNumDims(tensor);
    for (int i = 0; i < dims; ++i) {
      total *= runtime_.TensorDim(tensor, i);
    }
    return static_cast<size_t>(total);
  }

  bool AugmentResultWithIris(const MpImage& image,
                             MpFaceMeshResult* result,
                             int rotation_degrees,
                             bool mirror_horizontal,
                             int logical_width,
                             int logical_height) {
    auto sampler = [&](float x, float y) {
      x = Clamp(x, 0.0f, static_cast<float>(logical_width - 1));
      y = Clamp(y, 0.0f, static_cast<float>(logical_height - 1));
      if (rotation_degrees == 0 && !mirror_horizontal) {
        return BilinearSample(image, x, y);
      }
      return BilinearSampleRotated(image, x, y, rotation_degrees,
                                   mirror_horizontal, logical_width,
                                   logical_height);
    };
    return AugmentResultWithIrisFromSampler(result, logical_width,
                                            logical_height, sampler);
  }

  bool AugmentResultWithIrisNv21(const MpNv21Image& image,
                                 MpFaceMeshResult* result,
                                 int rotation_degrees,
                                 bool mirror_horizontal,
                                 int logical_width,
                                 int logical_height) {
    auto sampler = [&](float x, float y) {
      x = Clamp(x, 0.0f, static_cast<float>(logical_width - 1));
      y = Clamp(y, 0.0f, static_cast<float>(logical_height - 1));
      if (rotation_degrees == 0 && !mirror_horizontal) {
        return BilinearSampleNv21(image, x, y);
      }
      return BilinearSampleNv21Rotated(image, x, y, rotation_degrees,
                                       mirror_horizontal, logical_width,
                                       logical_height);
    };
    return AugmentResultWithIrisFromSampler(result, logical_width,
                                            logical_height, sampler);
  }

  template <typename Sampler>
  bool AugmentResultWithIrisFromSampler(MpFaceMeshResult* result,
                                        int width,
                                        int height,
                                        Sampler sampler) {
    if (!result || !result->landmarks ||
        result->landmarks_count < kFaceLandmarkCount) {
      return true;
    }

    std::vector<MpLandmark> left_eye;
    std::vector<MpLandmark> right_eye;
    std::vector<MpLandmark> left_iris;
    std::vector<MpLandmark> right_iris;
    if (!RunIrisForEye(result, width, height, 33, 133, false, sampler,
                       left_eye, left_iris) ||
        !RunIrisForEye(result, width, height, 362, 263, true, sampler,
                       right_eye, right_iris)) {
      return false;
    }

    constexpr int kOutputCount = kFaceLandmarkCount + kIrisLandmarkCount * 2;
    MpLandmark* updated = new MpLandmark[kOutputCount];
    if (!updated) {
      SetError("Unable to allocate iris-augmented landmarks buffer.");
      return false;
    }
    for (int i = 0; i < kFaceLandmarkCount; ++i) {
      updated[i] = result->landmarks[i];
    }
    for (int i = 0; i < kEyeLandmarkCount; ++i) {
      updated[kEyeLandmarkIndicesInFaceLandmarks[i]] = left_eye[i];
      updated[kEyeLandmarkIndicesInFaceLandmarks[kEyeLandmarkCount + i]] =
          right_eye[i];
    }
    for (int i = 0; i < kIrisLandmarkCount; ++i) {
      updated[kFaceLandmarkCount + i] = left_iris[i];
      updated[kFaceLandmarkCount + kIrisLandmarkCount + i] = right_iris[i];
    }

    delete[] result->landmarks;
    result->landmarks = updated;
    result->landmarks_count = kOutputCount;
    return true;
  }

  template <typename Sampler>
  bool RunIrisForEye(const MpFaceMeshResult* result,
                     int width,
                     int height,
                     int start_index,
                     int end_index,
                     bool is_right_eye,
                     Sampler sampler,
                     std::vector<MpLandmark>& eye_landmarks,
                     std::vector<MpLandmark>& iris_landmarks) {
    const MpNormalizedRect eye_rect =
        IrisRectFromEyeCorners(result->landmarks[start_index],
                               result->landmarks[end_index], width, height);
    if (eye_rect.width <= 0.0f || eye_rect.height <= 0.0f) {
      SetError("Invalid iris ROI.");
      return false;
    }
    const RectInPixels roi = ToPixelRect(eye_rect, width, height);
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;

    size_t offset = 0;
    for (int y = 0; y < iris_input_height_; ++y) {
      const float model_y =
          (static_cast<float>(y) + 0.5f) / static_cast<float>(iris_input_height_);
      const float ny = (model_y - 0.5f) * 2.0f;
      for (int x = 0; x < iris_input_width_; ++x) {
        const int source_model_x = is_right_eye ? (iris_input_width_ - 1 - x) : x;
        const float model_x = (static_cast<float>(source_model_x) + 0.5f) /
                              static_cast<float>(iris_input_width_);
        const float nx = (model_x - 0.5f) * 2.0f;
        const float rx = nx * half_w;
        const float ry = ny * half_h;
        const float source_x = cos_r * rx - sin_r * ry + roi.center_x;
        const float source_y = sin_r * rx + cos_r * ry + roi.center_y;
        const RgbPixel pixel = sampler(source_x, source_y);
        iris_input_buffer_[offset++] = pixel.r / 255.0f;
        iris_input_buffer_[offset++] = pixel.g / 255.0f;
        iris_input_buffer_[offset++] = pixel.b / 255.0f;
      }
    }

    if (runtime_.TensorCopyFromBuffer(
            iris_input_tensor_, iris_input_buffer_.data(),
            iris_input_buffer_.size() * sizeof(float)) != kTfLiteOk) {
      SetError("Failed to copy iris input buffer.");
      return false;
    }
    if (runtime_.InterpreterInvoke(iris_interpreter_.get()) != kTfLiteOk) {
      SetError("Iris interpreter invocation failed.");
      return false;
    }
    if (runtime_.TensorCopyToBuffer(
            iris_eye_tensor_, iris_eye_buffer_.data(),
            iris_eye_buffer_.size() * sizeof(float)) != kTfLiteOk ||
        runtime_.TensorCopyToBuffer(
            iris_landmarks_tensor_, iris_landmarks_buffer_.data(),
            iris_landmarks_buffer_.size() * sizeof(float)) != kTfLiteOk) {
      SetError("Unable to read iris outputs.");
      return false;
    }

    DecodeIrisLandmarks(iris_eye_buffer_, kEyeLandmarkCount, eye_rect, width,
                        height, is_right_eye, eye_landmarks);
    DecodeIrisLandmarks(iris_landmarks_buffer_, kIrisLandmarkCount, eye_rect,
                        width, height, is_right_eye, iris_landmarks);
    return true;
  }

  void DecodeIrisLandmarks(const std::vector<float>& buffer,
                           int count,
                           const MpNormalizedRect& rect,
                           int width,
                           int height,
                           bool is_right_eye,
                           std::vector<MpLandmark>& out) const {
    out.resize(static_cast<size_t>(count));
    const RectInPixels roi = ToPixelRect(rect, width, height);
    const float cos_r = std::cos(roi.rotation);
    const float sin_r = std::sin(roi.rotation);
    const float half_w = roi.width * 0.5f;
    const float half_h = roi.height * 0.5f;
    const float input_w = std::max(1, iris_input_width_);
    const float input_h = std::max(1, iris_input_height_);
    const bool pixel_space = IsPixelSpaceLandmarkBuffer(buffer.data(), count);

    for (int i = 0; i < count; ++i) {
      float raw_x = buffer[static_cast<size_t>(i) * 3];
      float raw_y = buffer[static_cast<size_t>(i) * 3 + 1];
      float raw_z = buffer[static_cast<size_t>(i) * 3 + 2];
      if (pixel_space) {
        raw_x /= input_w;
        raw_y /= input_h;
        raw_z /= input_w;
      }
      if (is_right_eye) {
        raw_x = 1.0f - raw_x;
      }
      const float nx = (raw_x - 0.5f) * 2.0f;
      const float ny = (raw_y - 0.5f) * 2.0f;
      const float rx = nx * half_w;
      const float ry = ny * half_h;
      const float abs_x = cos_r * rx - sin_r * ry + roi.center_x;
      const float abs_y = sin_r * rx + cos_r * ry + roi.center_y;
      const float abs_z = raw_z * roi.width;

      out[static_cast<size_t>(i)].x =
          Clamp(abs_x / static_cast<float>(width), -0.5f, 1.5f);
      out[static_cast<size_t>(i)].y =
          Clamp(abs_y / static_cast<float>(height), -0.5f, 1.5f);
      out[static_cast<size_t>(i)].z = abs_z / static_cast<float>(width);
    }
  }

  MpNormalizedRect IrisRectFromEyeCorners(const MpLandmark& start,
                                          const MpLandmark& end,
                                          int width,
                                          int height) const {
    const float start_x = start.x * static_cast<float>(width);
    const float start_y = start.y * static_cast<float>(height);
    const float end_x = end.x * static_cast<float>(width);
    const float end_y = end.y * static_cast<float>(height);
    const float dx = end_x - start_x;
    const float dy = end_y - start_y;
    const float distance = std::sqrt(dx * dx + dy * dy);
    MpNormalizedRect rect{};
    if (distance < 1.0f || width <= 0 || height <= 0) {
      return rect;
    }
    const float side = distance * kIrisRoiScale;
    rect.x_center = ((start_x + end_x) * 0.5f) / static_cast<float>(width);
    rect.y_center = ((start_y + end_y) * 0.5f) / static_cast<float>(height);
    rect.width = side / static_cast<float>(width);
    rect.height = side / static_cast<float>(height);
    rect.rotation = std::atan2(dy, dx);
    return rect;
  }

  RectInPixels ToPixelRect(const MpNormalizedRect& rect,
                           int width,
                           int height) const {
    RectInPixels roi;
    roi.center_x = rect.x_center * static_cast<float>(width);
    roi.center_y = rect.y_center * static_cast<float>(height);
    roi.width = rect.width * static_cast<float>(width);
    roi.height = rect.height * static_cast<float>(height);
    roi.rotation = rect.rotation;
    if (roi.width <= 0.0f) {
      roi.width = static_cast<float>(width);
    }
    if (roi.height <= 0.0f) {
      roi.height = static_cast<float>(height);
    }
    return roi;
  }

  void UpdateTrackingState(const MpFaceMeshResult& result, float score) {
    const float threshold =
        tracking_active_ ? min_tracking_confidence_ : min_detection_confidence_;
    if (score < threshold) {
      // Official graph semantics: a tracked face whose confidence falls below
      // the threshold is released so the caller re-acquires it via the
      // detector, instead of freezing the last ROI.
      has_valid_rect_ = false;
      tracking_active_ = false;
      return;
    }
    roi_ = SanitizeRect(RectFromLandmarks(result.landmarks,
                                          result.landmarks_count,
                                          result.image_width,
                                          result.image_height));
    has_valid_rect_ = true;
    tracking_active_ = true;
  }

  // Mirrored in Dart as FaceMeshResult.trackingRoi()
  // (lib/mediapipe_face_mesh.dart) for the multi-face tracking flow. Keep
  // the two implementations (including SanitizeRect) in sync.
  MpNormalizedRect RectFromLandmarks(const MpLandmark* landmarks,
                                     int count,
                                     int image_width,
                                     int image_height) const {
    if (!landmarks || count <= 0 || image_width <= 0 || image_height <= 0) {
      return DefaultRect();
    }
    float min_x = 1.0f;
    float min_y = 1.0f;
    float max_x = 0.0f;
    float max_y = 0.0f;
    for (int i = 0; i < count; ++i) {
      min_x = std::min(min_x, landmarks[i].x);
      min_y = std::min(min_y, landmarks[i].y);
      max_x = std::max(max_x, landmarks[i].x);
      max_y = std::max(max_y, landmarks[i].y);
    }
    // square_long must be computed in pixel space, like the detector and the
    // official graph; a square in normalized space stretches vertically on
    // portrait frames.
    const float width_px = (max_x - min_x) * image_width;
    const float height_px = (max_y - min_y) * image_height;
    if (width_px < 1e-1f || height_px < 1e-1f) {
      return DefaultRect();
    }
    const float long_side_px = std::max(width_px, height_px) * 1.5f;
    MpNormalizedRect rect;
    rect.x_center = (min_x + max_x) * 0.5f;
    rect.y_center = (min_y + max_y) * 0.5f;
    // Bounds are enforced by SanitizeRect at the call sites; clamping the two
    // axes independently here would break the pixel-space squareness.
    rect.width = long_side_px / image_width;
    rect.height = long_side_px / image_height;
    rect.rotation =
        EstimateRotation(landmarks, count, image_width, image_height);
    return rect;
  }

  float EstimateRotation(const MpLandmark* landmarks,
                         int count,
                         int image_width,
                         int image_height) const {
    const int left_eye_index = 263;
    const int right_eye_index = 33;
    if (count <= left_eye_index || count <= right_eye_index) {
      return 0.0f;
    }
    const MpLandmark& left = landmarks[left_eye_index];
    const MpLandmark& right = landmarks[right_eye_index];
    // Aspect-correct the deltas so the angle matches pixel space.
    const float dx = (left.x - right.x) * image_width;
    const float dy = (left.y - right.y) * image_height;
    if (std::abs(dx) < 1e-5f && std::abs(dy) < 1e-5f) {
      return 0.0f;
    }
    return std::atan2(dy, dx);
  }

  void SetError(const std::string& message) {
    last_error_ = message;
    MP_LOGE("%s\n", message.c_str());
  }

  TfLiteRuntime runtime_;
  std::unique_ptr<TfLiteModel, TfLiteModelDeleter> model_{nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreterOptions, TfLiteOptionsDeleter> options_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreter, TfLiteInterpreterDeleter> interpreter_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteDelegate, TfLiteDelegateDeleter> delegate_{nullptr, {}};
  std::unique_ptr<TfLiteModel, TfLiteModelDeleter> iris_model_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreterOptions, TfLiteOptionsDeleter> iris_options_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreter, TfLiteInterpreterDeleter> iris_interpreter_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteDelegate, TfLiteDelegateDeleter> iris_delegate_{
      nullptr, {}};

  TfLiteTensor* input_tensor_ = nullptr;
  const TfLiteTensor* output_landmarks_tensor_ = nullptr;
  const TfLiteTensor* output_score_tensor_ = nullptr;
  TfLiteTensor* iris_input_tensor_ = nullptr;
  const TfLiteTensor* iris_eye_tensor_ = nullptr;
  const TfLiteTensor* iris_landmarks_tensor_ = nullptr;

  // Attention model output tensors (face_landmark_with_attention).
  const TfLiteTensor* attn_mesh_tensor_ = nullptr;
  const TfLiteTensor* attn_lips_tensor_ = nullptr;
  const TfLiteTensor* attn_left_eye_tensor_ = nullptr;
  const TfLiteTensor* attn_right_eye_tensor_ = nullptr;
  const TfLiteTensor* attn_left_iris_tensor_ = nullptr;
  const TfLiteTensor* attn_right_iris_tensor_ = nullptr;
  const TfLiteTensor* attn_faceflag_tensor_ = nullptr;

  int input_width_ = 0;
  int input_height_ = 0;
  int output_landmark_count_ = 0;
  int iris_input_width_ = 0;
  int iris_input_height_ = 0;

  int threads_ = 2;
  float min_detection_confidence_ = 0.5f;
  float min_tracking_confidence_ = 0.5f;
  float min_face_presence_confidence_ = 0.5f;
  bool roi_tracking_enabled_ = true;
  bool iris_enabled_ = false;
  bool attention_enabled_ = false;
  MpDelegateType active_delegate_ = MP_DELEGATE_CPU;
  MpDelegateType active_iris_delegate_ = MP_DELEGATE_CPU;

  std::vector<float> input_buffer_;
  std::vector<float> landmarks_buffer_;
  std::vector<float> attn_lips_buffer_;
  std::vector<float> attn_left_eye_buffer_;
  std::vector<float> attn_right_eye_buffer_;
  std::vector<float> attn_left_iris_buffer_;
  std::vector<float> attn_right_iris_buffer_;
  std::vector<float> iris_input_buffer_;
  std::vector<float> iris_eye_buffer_;
  std::vector<float> iris_landmarks_buffer_;

  MpNormalizedRect roi_;
  bool has_valid_rect_ = false;
  // True only while roi_ was derived from face landmarks; has_valid_rect_
  // also covers the initial full-frame DefaultRect seed.
  bool tracking_active_ = false;
  int last_rotation_degrees_ = 0;
  bool last_mirror_horizontal_ = false;
  std::string last_error_;
};

thread_local std::string g_last_global_error;

void SetGlobalError(const std::string& message) {
  g_last_global_error = message;
}

}  // namespace

struct MpFaceMeshContext {
  FaceMeshContext impl;
};

namespace {

// Shared body of mp_face_mesh_process_rois / mp_face_mesh_process_rois_nv21:
// runs [process_one] per ROI and packs the results into one heap array,
// taking ownership of each result's landmarks. An ROI whose inference fails
// yields an empty entry (no landmarks) instead of failing the whole batch,
// so one bad ROI cannot drop every healthy face; the failing ROI's error
// stays readable via mp_face_mesh_last_error.
template <typename ProcessOneFn>
MpFaceMeshMultiResult* ProcessRoisImpl(const MpNormalizedRect* rois,
                                       int32_t rois_count,
                                       ProcessOneFn&& process_one) {
  auto* multi = new MpFaceMeshMultiResult();
  multi->results_count = rois_count;
  multi->results =
      rois_count > 0 ? new MpFaceMeshResult[rois_count]() : nullptr;
  for (int32_t i = 0; i < rois_count; ++i) {
    MpFaceMeshResult* result = process_one(rois[i]);
    if (!result) {
      // Value-initialized entry: no landmarks, score 0. Keep the input ROI
      // so the caller can still see which region the failure belongs to.
      multi->results[i].rect = rois[i];
      continue;
    }
    multi->results[i] = *result;  // Takes ownership of result->landmarks.
    delete result;
  }
  return multi;
}

// No C++ exception may unwind across the FFI boundary into Dart (the process
// would abort), so pointer-returning exports run their body through this
// guard, which converts an escaped exception (std::bad_alloc included) into
// a null return with the error recorded on the context, when there is one,
// and globally.
template <typename Fn>
auto GuardedNative(MpFaceMeshContext* context, Fn&& body) -> decltype(body()) {
  try {
    return body();
  } catch (const std::exception& e) {
    const std::string message =
        std::string("Unhandled native exception: ") + e.what();
    if (context) {
      context->impl.RecordError(message);
    }
    SetGlobalError(message);
    return nullptr;
  } catch (...) {
    const std::string message = "Unhandled native exception.";
    if (context) {
      context->impl.RecordError(message);
    }
    SetGlobalError(message);
    return nullptr;
  }
}

}  // namespace

extern "C" {

FFI_PLUGIN_EXPORT MpFaceMeshContext* mp_face_mesh_create(
    const char* model_path,
    const MpFaceMeshCreateOptions* options) {
  return GuardedNative(nullptr, [&]() -> MpFaceMeshContext* {
    if (!model_path) {
      SetGlobalError("Model path is null.");
      return nullptr;
    }
    auto* context = new MpFaceMeshContext();
    if (!context->impl.Initialize(model_path, options)) {
      SetGlobalError(context->impl.last_error());
      delete context;
      return nullptr;
    }
    return context;
  });
}

FFI_PLUGIN_EXPORT void mp_face_mesh_destroy(MpFaceMeshContext* context) {
  try {
    delete context;
  } catch (...) {
  }
}

FFI_PLUGIN_EXPORT MpFaceMeshResult* mp_face_mesh_process(
    MpFaceMeshContext* context,
    const MpImage* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal) {
  return GuardedNative(context, [&]() -> MpFaceMeshResult* {
    if (!context) {
      SetGlobalError("Context is null.");
      return nullptr;
    }
    if (!image) {
      SetGlobalError("Image is null.");
      return nullptr;
    }
    return context->impl.Process(*image, override_rect, rotation_degrees,
                                 mirror_horizontal != 0);
  });
}

FFI_PLUGIN_EXPORT MpFaceMeshResult* mp_face_mesh_process_nv21(
    MpFaceMeshContext* context,
    const MpNv21Image* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal) {
  return GuardedNative(context, [&]() -> MpFaceMeshResult* {
    if (!context) {
      SetGlobalError("Context is null.");
      return nullptr;
    }
    if (!image) {
      SetGlobalError("Image is null.");
      return nullptr;
    }
    return context->impl.ProcessNv21(*image, override_rect, rotation_degrees,
                                     mirror_horizontal != 0);
  });
}

FFI_PLUGIN_EXPORT MpFaceMeshMultiResult* mp_face_mesh_process_rois(
    MpFaceMeshContext* context,
    const MpImage* image,
    const MpNormalizedRect* rois,
    int32_t rois_count,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal) {
  if (!context) {
    SetGlobalError("Context is null.");
    return nullptr;
  }
  if (!image) {
    SetGlobalError("Image is null.");
    return nullptr;
  }
  if (rois_count < 0 || (rois_count > 0 && !rois)) {
    SetGlobalError("Invalid ROI list.");
    return nullptr;
  }
  return GuardedNative(context, [&]() -> MpFaceMeshMultiResult* {
    return ProcessRoisImpl(
        rois, rois_count, [&](const MpNormalizedRect& roi) {
          return context->impl.Process(*image, &roi, rotation_degrees,
                                       mirror_horizontal != 0);
        });
  });
}

FFI_PLUGIN_EXPORT MpFaceMeshMultiResult* mp_face_mesh_process_rois_nv21(
    MpFaceMeshContext* context,
    const MpNv21Image* image,
    const MpNormalizedRect* rois,
    int32_t rois_count,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal) {
  if (!context) {
    SetGlobalError("Context is null.");
    return nullptr;
  }
  if (!image) {
    SetGlobalError("Image is null.");
    return nullptr;
  }
  if (rois_count < 0 || (rois_count > 0 && !rois)) {
    SetGlobalError("Invalid ROI list.");
    return nullptr;
  }
  return GuardedNative(context, [&]() -> MpFaceMeshMultiResult* {
    return ProcessRoisImpl(
        rois, rois_count, [&](const MpNormalizedRect& roi) {
          return context->impl.ProcessNv21(*image, &roi, rotation_degrees,
                                           mirror_horizontal != 0);
        });
  });
}

FFI_PLUGIN_EXPORT void mp_face_mesh_release_result(MpFaceMeshResult* result) {
  if (!result) {
    return;
  }
  delete[] result->landmarks;
  result->landmarks = nullptr;
  delete result;
}

FFI_PLUGIN_EXPORT void mp_face_mesh_release_multi_result(
    MpFaceMeshMultiResult* result) {
  if (!result) {
    return;
  }
  for (int32_t i = 0; i < result->results_count; ++i) {
    delete[] result->results[i].landmarks;
  }
  delete[] result->results;
  result->results = nullptr;
  delete result;
}

FFI_PLUGIN_EXPORT uint8_t mp_face_mesh_is_tracking(
    const MpFaceMeshContext* context) {
  if (!context) {
    return 0;
  }
  return context->impl.is_tracking() ? 1 : 0;
}

FFI_PLUGIN_EXPORT const char* mp_face_mesh_last_error(
    const MpFaceMeshContext* context) {
  if (!context) {
    return nullptr;
  }
  return context->impl.last_error();
}

FFI_PLUGIN_EXPORT const char* mp_face_mesh_last_global_error(void) {
  return g_last_global_error.c_str();
}

FFI_PLUGIN_EXPORT MpDelegateType mp_face_mesh_active_delegate(
    const MpFaceMeshContext* context) {
  if (!context) {
    return MP_DELEGATE_CPU;
  }
  return context->impl.active_delegate();
}

FFI_PLUGIN_EXPORT MpDelegateType mp_face_mesh_active_iris_delegate(
    const MpFaceMeshContext* context) {
  if (!context) {
    return MP_DELEGATE_CPU;
  }
  return context->impl.active_iris_delegate();
}

}  // extern "C"


```

## Source file: src/mediapipe_face_detector.cc

{
  "path": "src/mediapipe_face_detector.cc",
  "blob_sha": "8b3ae996ec763087c98a5d9cbb68b8cd8aafa965",
  "sha256": "f3d2c085a6814f838d77333987a279ec415b976c8d665a8673d50a4b8fe23b09",
  "lines": 1666,
  "bytes": 59807,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 6,
    "rig": 17,
    "face": 61
  },
  "url": "https://github.com/cornpip/mediapipe_face_mesh/blob/f2d5d1b9d00f5a9463703a9873e3b8e3dce0b658/src/mediapipe_face_detector.cc",
  "function_signature_candidates": [
    {
      "line": 84,
      "text": "float Clamp(float value, float min_value, float max_value) {"
    },
    {
      "line": 88,
      "text": "int ClampInt(int value, int min_value, int max_value) {"
    },
    {
      "line": 92,
      "text": "float NormalizeAngle(float radians) {"
    },
    {
      "line": 105,
      "text": "float Sigmoid(float value) {"
    },
    {
      "line": 110,
      "text": "float IntersectionOverUnion(const DecodedDetection& a,"
    },
    {
      "line": 263,
      "text": "bool Initialize(const std::string& model_path,"
    },
    {
      "line": 611,
      "text": "void RecordError(const std::string& message) { SetError(message); }"
    },
    {
      "line": 618,
      "text": "void operator()(TfLiteModel* model) const {"
    },
    {
      "line": 627,
      "text": "void operator()(TfLiteInterpreterOptions* options) const {"
    },
    {
      "line": 636,
      "text": "void operator()(TfLiteInterpreter* interpreter) const {"
    },
    {
      "line": 646,
      "text": "void operator()(TfLiteDelegate* delegate) const {"
    },
    {
      "line": 653,
      "text": "void Shutdown() {"
    },
    {
      "line": 683,
      "text": "static int NormalizeRotationDegrees(int rotation_degrees) {"
    },
    {
      "line": 907,
      "text": "bool ConfigureDecoder() {"
    },
    {
      "line": 1017,
      "text": "void UpdateProjectionMatrix() {"
    },
    {
      "line": 1053,
      "text": "bool ProjectTensorPoint(float x_norm,"
    },
    {
      "line": 1069,
      "text": "bool Preprocess(const MpImage& image, const MpNormalizedRect& rect) {"
    },
    {
      "line": 1114,
      "text": "bool PreprocessRotated(const MpImage& image,"
    },
    {
      "line": 1166,
      "text": "bool PreprocessNv21(const MpNv21Image& image, const MpNormalizedRect& rect) {"
    },
    {
      "line": 1211,
      "text": "bool PreprocessNv21Rotated(const MpNv21Image& image,"
    },
    {
      "line": 1416,
      "text": "static inline void MapRotatedToRaw(int x_rot,"
    },
    {
      "line": 1475,
      "text": "void SetError(const std::string& message) {"
    },
    {
      "line": 1583,
      "text": "void mp_face_detector_destroy(MpFaceDetectorContext* context) {"
    },
    {
      "line": 1638,
      "text": "void mp_face_detector_release_result(MpFaceDetectorResult* result) {"
    }
  ]
}

```text

#include "mediapipe_face.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <array>
#include <exception>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "tflite_runtime.h"

#if defined(__ANDROID__)
#include <android/log.h>
#define MP_DETECT_LOG_TAG "MediapipeFaceDetector"
#define MP_DETECT_LOGI(...) \
  __android_log_print(ANDROID_LOG_INFO, MP_DETECT_LOG_TAG, __VA_ARGS__)
#define MP_DETECT_LOGE(...) \
  __android_log_print(ANDROID_LOG_ERROR, MP_DETECT_LOG_TAG, __VA_ARGS__)
#else
#define MP_DETECT_LOGI(...) std::fprintf(stdout, "[INFO] " __VA_ARGS__)
#define MP_DETECT_LOGE(...) std::fprintf(stderr, "[ERROR] " __VA_ARGS__)
#endif

namespace {

struct RectInPixels {
  float center_x = 0.0f;
  float center_y = 0.0f;
  float width = 0.0f;
  float height = 0.0f;
  float rotation = 0.0f;
};

struct TensorTransform {
  float center_x = 0.0f;
  float center_y = 0.0f;
  float roi_width = 0.0f;
  float roi_height = 0.0f;
  float rotation = 0.0f;
  float scale = 1.0f;
  float pad_x = 0.0f;
  float pad_y = 0.0f;
  float scaled_width = 0.0f;
  float scaled_height = 0.0f;
};

struct ProjectionMatrix {
  float m00 = 1.0f;
  float m01 = 0.0f;
  float m02 = 0.0f;
  float m10 = 0.0f;
  float m11 = 1.0f;
  float m12 = 0.0f;
};

struct RgbPixel {
  float r = 0.0f;
  float g = 0.0f;
  float b = 0.0f;
};

struct Anchor {
  float x_center = 0.0f;
  float y_center = 0.0f;
  float width = 1.0f;
  float height = 1.0f;
};

struct DecodedDetection {
  float left = 0.0f;
  float top = 0.0f;
  float right = 0.0f;
  float bottom = 0.0f;
  float score = 0.0f;
  std::array<float, 12> keypoints{};
};

float Clamp(float value, float min_value, float max_value) {
  return std::max(min_value, std::min(max_value, value));
}

int ClampInt(int value, int min_value, int max_value) {
  return std::max(min_value, std::min(max_value, value));
}

float NormalizeAngle(float radians) {
  constexpr float kPi = 3.14159265358979323846f;
  constexpr float kTwoPi = kPi * 2.0f;
  float angle = radians;
  while (angle > kPi) {
    angle -= kTwoPi;
  }
  while (angle < -kPi) {
    angle += kTwoPi;
  }
  return angle;
}

float Sigmoid(float value) {
  const float clamped = Clamp(value, -100.0f, 100.0f);
  return 1.0f / (1.0f + std::exp(-clamped));
}

float IntersectionOverUnion(const DecodedDetection& a,
                            const DecodedDetection& b) {
  const float left = std::max(a.left, b.left);
  const float top = std::max(a.top, b.top);
  const float right = std::min(a.right, b.right);
  const float bottom = std::min(a.bottom, b.bottom);
  const float width = std::max(0.0f, right - left);
  const float height = std::max(0.0f, bottom - top);
  const float intersection = width * height;
  const float area_a =
      std::max(0.0f, a.right - a.left) * std::max(0.0f, a.bottom - a.top);
  const float area_b =
      std::max(0.0f, b.right - b.left) * std::max(0.0f, b.bottom - b.top);
  const float denominator = area_a + area_b - intersection;
  if (denominator <= 1e-6f) {
    return 0.0f;
  }
  return intersection / denominator;
}

DecodedDetection WeightedMergeDetections(
    const std::vector<const DecodedDetection*>& group) {
  DecodedDetection merged;
  if (group.empty()) {
    return merged;
  }

  float total_weight = 0.0f;
  float max_score = 0.0f;
  for (const DecodedDetection* detection : group) {
    const float weight = std::max(detection->score, 1e-6f);
    total_weight += weight;
    max_score = std::max(max_score, detection->score);
    merged.left += detection->left * weight;
    merged.top += detection->top * weight;
    merged.right += detection->right * weight;
    merged.bottom += detection->bottom * weight;
    for (size_t i = 0; i < merged.keypoints.size(); ++i) {
      merged.keypoints[i] += detection->keypoints[i] * weight;
    }
  }

  if (total_weight > 0.0f) {
    const float inv_total_weight = 1.0f / total_weight;
    merged.left *= inv_total_weight;
    merged.top *= inv_total_weight;
    merged.right *= inv_total_weight;
    merged.bottom *= inv_total_weight;
    for (float& value : merged.keypoints) {
      value *= inv_total_weight;
    }
  }
  merged.score = max_score;
  return merged;
}

MpNormalizedRect ToNormalizedRect(const RectInPixels& rect,
                                  int image_width,
                                  int image_height) {
  MpNormalizedRect normalized{};
  if (image_width <= 0 || image_height <= 0) {
    return normalized;
  }
  normalized.x_center = rect.center_x / image_width;
  normalized.y_center = rect.center_y / image_height;
  normalized.width = rect.width / image_width;
  normalized.height = rect.height / image_height;
  normalized.rotation = rect.rotation;
  return normalized;
}

std::vector<Anchor> GenerateShortRangeAnchors() {
  constexpr int kInputSize = 128;
  constexpr float kMinScale = 0.1484375f;
  constexpr float kMaxScale = 0.75f;
  constexpr int kNumLayers = 4;
  constexpr int kStrides[kNumLayers] = {8, 16, 16, 16};

  std::vector<Anchor> anchors;
  anchors.reserve(896);

  for (int layer = 0; layer < kNumLayers;) {
    std::vector<float> scales;
    int last_same_stride_layer = layer;
    while (last_same_stride_layer < kNumLayers &&
           kStrides[last_same_stride_layer] == kStrides[layer]) {
      const float scale =
          kMinScale + (kMaxScale - kMinScale) * last_same_stride_layer /
                          static_cast<float>(kNumLayers - 1);
      scales.push_back(scale);
      const float next_scale =
          (last_same_stride_layer == kNumLayers - 1)
              ? 1.0f
              : (kMinScale + (kMaxScale - kMinScale) *
                                 (last_same_stride_layer + 1) /
                                 static_cast<float>(kNumLayers - 1));
      scales.push_back(std::sqrt(scale * next_scale));
      ++last_same_stride_layer;
    }

    const int stride = kStrides[layer];
    const int feature_map_size = static_cast<int>(
        std::ceil(static_cast<float>(kInputSize) / stride));
    for (int y = 0; y < feature_map_size; ++y) {
      for (int x = 0; x < feature_map_size; ++x) {
        const float anchor_x =
            (static_cast<float>(x) + 0.5f) / feature_map_size;
        const float anchor_y =
            (static_cast<float>(y) + 0.5f) / feature_map_size;
        for (size_t scale_index = 0; scale_index < scales.size();
             ++scale_index) {
          Anchor anchor;
          anchor.x_center = anchor_x;
          anchor.y_center = anchor_y;
          anchor.width = 1.0f;
          anchor.height = 1.0f;
          anchors.push_back(anchor);
        }
      }
    }
    layer = last_same_stride_layer;
  }

  return anchors;
}

std::vector<Anchor> GenerateFullRangeAnchors() {
  constexpr int kInputSize = 192;
  constexpr int kStride = 4;
  constexpr int kFeatureMapSize = kInputSize / kStride;

  std::vector<Anchor> anchors;
  anchors.reserve(kFeatureMapSize * kFeatureMapSize);
  for (int y = 0; y < kFeatureMapSize; ++y) {
    for (int x = 0; x < kFeatureMapSize; ++x) {
      Anchor anchor;
      anchor.x_center =
          (static_cast<float>(x) + 0.5f) / kFeatureMapSize;
      anchor.y_center =
          (static_cast<float>(y) + 0.5f) / kFeatureMapSize;
      anchor.width = 1.0f;
      anchor.height = 1.0f;
      anchors.push_back(anchor);
    }
  }
  return anchors;
}

class FaceDetectorContext {
 public:
  FaceDetectorContext() = default;
  ~FaceDetectorContext() { Shutdown(); }

  bool Initialize(const std::string& model_path,
                  const MpFaceDetectorCreateOptions* options) {
    threads_ = 2;
    if (options && options->threads > 0) {
      threads_ = options->threads;
    }
    // Negative means "use the default"; an explicit 0.0 is honored so the
    // native threshold always matches what the Dart processor reports.
    min_detection_confidence_ =
        (options && options->min_detection_confidence >= 0.0f)
            ? options->min_detection_confidence
            : 0.5f;
    min_suppression_threshold_ =
        (options && options->min_suppression_threshold >= 0.0f)
            ? options->min_suppression_threshold
            : 0.3f;
    max_results_ = (options && options->max_results > 0) ? options->max_results
                                                         : 1;

    const char* runtime_path =
        (options && options->tflite_library_path)
            ? options->tflite_library_path
            : nullptr;
    if (!runtime_.Load(runtime_path)) {
      SetError("Failed to load TensorFlow Lite runtime: " + runtime_.error());
      return false;
    }

    model_.reset(runtime_.ModelCreateFromFile(model_path.c_str()));
    if (!model_) {
      SetError("Unable to load model file: " + model_path);
      return false;
    }

    options_.reset(runtime_.InterpreterOptionsCreate());
    if (!options_) {
      SetError("Failed to allocate interpreter options.");
      return false;
    }
    runtime_.InterpreterOptionsSetThreads(options_.get(), threads_);

    const MpDelegateType delegate_choice =
        options ? static_cast<MpDelegateType>(options->delegate)
                : MP_DELEGATE_CPU;
    const bool allow_delegate_fallback =
        !options || options->disable_delegate_fallback == 0;
    active_delegate_ = MP_DELEGATE_CPU;
    auto AttachDelegate = [&](TfLiteDelegate* created,
                              TfLiteDelegateDeleter::DeleteFn deleter,
                              const char* name,
                              MpDelegateType delegate_type) {
      if (!created) {
        return false;
      }
      delegate_.get_deleter().deleter = deleter;
      delegate_.reset(created);
      runtime_.InterpreterOptionsAddDelegate(
          options_.get(),
          reinterpret_cast<TfLiteOpaqueDelegate*>(delegate_.get()));
      active_delegate_ = delegate_type;
      MP_DETECT_LOGI("%s delegate enabled.\n", name);
      return true;
    };

    switch (delegate_choice) {
      case MP_DELEGATE_XNNPACK: {
        if (!runtime_.InterpreterOptionsAddDelegate ||
            !runtime_.XnnpackDelegateOptionsDefault ||
            !runtime_.XnnpackDelegateCreate ||
            !runtime_.XnnpackDelegateDelete) {
          if (!allow_delegate_fallback) {
            SetError("XNNPACK delegate is unavailable for face detector and "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_DETECT_LOGI(
              "XNNPACK delegate requested but unavailable in runtime.\n");
          break;
        }
        TfLiteXNNPackDelegateOptions delegate_options =
            runtime_.XnnpackDelegateOptionsDefault();
        delegate_options.num_threads = threads_;
        TfLiteDelegate* created =
            runtime_.XnnpackDelegateCreate(&delegate_options);
        if (!AttachDelegate(created, runtime_.XnnpackDelegateDelete,
                            "XNNPACK", MP_DELEGATE_XNNPACK)) {
          if (!allow_delegate_fallback) {
            SetError("Failed to create XNNPACK delegate for face detector "
                     "because delegate fallback is disabled.");
            return false;
          }
          MP_DETECT_LOGE(
              "Failed to create XNNPACK delegate. Falling back to CPU.\n");
        }
        break;
      }
      case MP_DELEGATE_GPU_V2: {
        if (!runtime_.InterpreterOptionsAddDelegate ||
            !runtime_.GpuDelegateV2OptionsDefault ||
            !runtime_.GpuDelegateV2Create || !runtime_.GpuDelegateV2Delete) {
          if (!allow_delegate_fallback) {
            SetError("GPU delegate (V2) is unavailable for face detector and "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_DETECT_LOGI(
              "GPU delegate (V2) requested but unavailable in runtime.\n");
          break;
        }
        TfLiteGpuDelegateOptionsV2 delegate_options =
            runtime_.GpuDelegateV2OptionsDefault();
        delegate_options.experimental_flags |=
            TFLITE_GPU_EXPERIMENTAL_FLAGS_ENABLE_QUANT;
        TfLiteDelegate* created =
            runtime_.GpuDelegateV2Create(&delegate_options);
        if (!AttachDelegate(created, runtime_.GpuDelegateV2Delete, "GPU V2",
                            MP_DELEGATE_GPU_V2)) {
          if (!allow_delegate_fallback) {
            SetError("Failed to create GPU delegate for face detector because "
                     "delegate fallback is disabled.");
            return false;
          }
          MP_DETECT_LOGE(
              "Failed to create GPU delegate. Falling back to CPU.\n");
        }
        break;
      }
      case MP_DELEGATE_CPU:
      default:
        break;
    }

    interpreter_.reset(runtime_.InterpreterCreate(model_.get(), options_.get()));
    bool tensors_ready =
        interpreter_ &&
        runtime_.InterpreterAllocateTensors(interpreter_.get()) == kTfLiteOk;
    // A delegate can also fail after it is attached, while the interpreter
    // builds or allocates tensors (e.g. the GPU delegate rejecting a graph
    // with custom ops). Honor delegate fallback for that stage too.
    if (!tensors_ready && active_delegate_ != MP_DELEGATE_CPU &&
        allow_delegate_fallback) {
      MP_DETECT_LOGE(
          "Interpreter creation with the requested delegate failed. "
          "Falling back to CPU.\n");
      interpreter_.reset();
      options_.reset(runtime_.InterpreterOptionsCreate());
      delegate_.reset();
      if (!options_) {
        SetError("Failed to allocate interpreter options.");
        return false;
      }
      runtime_.InterpreterOptionsSetThreads(options_.get(), threads_);
      active_delegate_ = MP_DELEGATE_CPU;
      interpreter_.reset(
          runtime_.InterpreterCreate(model_.get(), options_.get()));
      tensors_ready =
          interpreter_ &&
          runtime_.InterpreterAllocateTensors(interpreter_.get()) == kTfLiteOk;
    }
    if (!interpreter_) {
      SetError("Failed to create interpreter.");
      return false;
    }
    if (!tensors_ready) {
      SetError("Tensor allocation failed.");
      return false;
    }

    if (runtime_.InterpreterGetInputTensorCount(interpreter_.get()) < 1) {
      SetError("Interpreter input tensor missing.");
      return false;
    }
    input_tensor_ = runtime_.InterpreterGetInputTensor(interpreter_.get(), 0);
    if (!input_tensor_) {
      SetError("Input tensor unavailable.");
      return false;
    }
    if (runtime_.TensorType(input_tensor_) != kTfLiteFloat32) {
      SetError("Face detector input must be float32.");
      return false;
    }
    if (runtime_.TensorNumDims(input_tensor_) != 4) {
      SetError("Expected NHWC input tensor layout.");
      return false;
    }
    const int batch = runtime_.TensorDim(input_tensor_, 0);
    input_height_ = runtime_.TensorDim(input_tensor_, 1);
    input_width_ = runtime_.TensorDim(input_tensor_, 2);
    const int channels = runtime_.TensorDim(input_tensor_, 3);
    if (batch != 1 || channels != 3) {
      SetError("Model expects 1xHxWx3 input.");
      return false;
    }
    input_buffer_.resize(
        static_cast<size_t>(input_height_ * input_width_ * channels));

    const int output_count =
        runtime_.InterpreterGetOutputTensorCount(interpreter_.get());
    if (output_count < 2) {
      SetError("Face detector expects at least 2 output tensors.");
      return false;
    }

    output_boxes_tensor_ = runtime_.InterpreterGetOutputTensor(interpreter_.get(), 0);
    output_scores_tensor_ =
        runtime_.InterpreterGetOutputTensor(interpreter_.get(), 1);
    if (!output_boxes_tensor_ || !output_scores_tensor_) {
      SetError("Detector outputs are unavailable.");
      return false;
    }
    if (runtime_.TensorType(output_boxes_tensor_) != kTfLiteFloat32 ||
        runtime_.TensorType(output_scores_tensor_) != kTfLiteFloat32) {
      SetError("Detector outputs must be float32.");
      return false;
    }

    boxes_count_ = 1;
    const int boxes_dims = runtime_.TensorNumDims(output_boxes_tensor_);
    for (int i = 0; i < boxes_dims; ++i) {
      boxes_count_ *= runtime_.TensorDim(output_boxes_tensor_, i);
    }
    scores_count_ = 1;
    const int scores_dims = runtime_.TensorNumDims(output_scores_tensor_);
    for (int i = 0; i < scores_dims; ++i) {
      scores_count_ *= runtime_.TensorDim(output_scores_tensor_, i);
    }
    if (boxes_count_ <= 0 || scores_count_ <= 0) {
      SetError("Detector outputs are empty.");
      return false;
    }

    boxes_buffer_.resize(static_cast<size_t>(boxes_count_));
    scores_buffer_.resize(static_cast<size_t>(scores_count_));
    if (boxes_count_ % 16 != 0) {
      SetError("Unexpected detector box tensor size.");
      return false;
    }
    num_boxes_ = boxes_count_ / 16;
    if (scores_count_ % num_boxes_ != 0) {
      SetError("Detector score tensor does not match box tensor.");
      return false;
    }
    num_classes_ = scores_count_ / num_boxes_;
    if (!ConfigureDecoder()) {
      return false;
    }
    if (static_cast<int>(anchors_.size()) != num_boxes_) {
      SetError("Anchor count does not match detector output tensor.");
      return false;
    }

    MP_DETECT_LOGI("Face detector initialized: input=%dx%d boxes=%d classes=%d\n",
                   input_width_, input_height_, num_boxes_, num_classes_);
    return true;
  }

  MpFaceDetectorResult* Process(const MpImage& image,
                                const MpNormalizedRect* override_rect,
                                int rotation_degrees = 0,
                                bool mirror_horizontal = false,
                                const MpRoiTransformOptions* roi_transform = nullptr) {
    if (!interpreter_) {
      SetError("Interpreter is not initialized.");
      return nullptr;
    }
    if (!image.data || image.width <= 0 || image.height <= 0 ||
        image.bytes_per_row <= 0) {
      SetError("Invalid image buffer.");
      return nullptr;
    }
    if (image.format != MP_PIXEL_FORMAT_RGBA &&
        image.format != MP_PIXEL_FORMAT_BGRA) {
      SetError("Unsupported pixel format. Use RGBA/BGRA.");
      return nullptr;
    }

    const int rot = NormalizeRotationDegrees(rotation_degrees);
    if (rot < 0) {
      SetError("rotation_degrees must be one of 0, 90, 180, 270.");
      return nullptr;
    }

    const int logical_width = (rot == 90 || rot == 270) ? image.height : image.width;
    const int logical_height = (rot == 90 || rot == 270) ? image.width : image.height;
    const MpNormalizedRect rect =
        override_rect ? SanitizeRect(*override_rect) : DefaultRect();

    const bool needs_transform = rot != 0 || mirror_horizontal;
    if (needs_transform) {
      if (!PreprocessRotated(image, rect, rot, mirror_horizontal, logical_width,
                             logical_height)) {
        return nullptr;
      }
    } else {
      if (!Preprocess(image, rect)) {
        return nullptr;
      }
    }

    return InvokeAndDecode(logical_width, logical_height, rect, roi_transform);
  }

  MpFaceDetectorResult* ProcessNv21(const MpNv21Image& image,
                                    const MpNormalizedRect* override_rect,
                                    int rotation_degrees = 0,
                                    bool mirror_horizontal = false,
                                    const MpRoiTransformOptions* roi_transform = nullptr) {
    if (!interpreter_) {
      SetError("Interpreter is not initialized.");
      return nullptr;
    }
    if (!image.y || !image.vu || image.width <= 0 || image.height <= 0 ||
        image.y_bytes_per_row <= 0 || image.vu_bytes_per_row <= 0) {
      SetError("Invalid NV21 image buffer.");
      return nullptr;
    }

    const int rot = NormalizeRotationDegrees(rotation_degrees);
    if (rot < 0) {
      SetError("rotation_degrees must be one of 0, 90, 180, 270.");
      return nullptr;
    }

    const int logical_width = (rot == 90 || rot == 270) ? image.height : image.width;
    const int logical_height =
        (rot == 90 || rot == 270) ? image.width : image.height;
    const MpNormalizedRect rect =
        override_rect ? SanitizeRect(*override_rect) : DefaultRect();

    const bool needs_transform = rot != 0 || mirror_horizontal;
    if (needs_transform) {
      if (!PreprocessNv21Rotated(image, rect, rot, mirror_horizontal,
                                 logical_width, logical_height)) {
        return nullptr;
      }
    } else {
      if (!PreprocessNv21(image, rect)) {
        return nullptr;
      }
    }

    return InvokeAndDecode(logical_width, logical_height, rect, roi_transform);
  }

  const char* last_error() const { return last_error_.c_str(); }

  // Records an error raised outside the class, e.g. a C++ exception caught
  // at the FFI boundary.
  void RecordError(const std::string& message) { SetError(message); }

  MpDelegateType active_delegate() const { return active_delegate_; }

 private:
  struct TfLiteModelDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteModel* model) const {
      if (runtime && model) {
        runtime->ModelDelete(model);
      }
    }
  };

  struct TfLiteOptionsDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteInterpreterOptions* options) const {
      if (runtime && options) {
        runtime->InterpreterOptionsDelete(options);
      }
    }
  };

  struct TfLiteInterpreterDeleter {
    TfLiteRuntime* runtime;
    void operator()(TfLiteInterpreter* interpreter) const {
      if (runtime && interpreter) {
        runtime->InterpreterDelete(interpreter);
      }
    }
  };

  struct TfLiteDelegateDeleter {
    using DeleteFn = void (*)(TfLiteDelegate*);
    DeleteFn deleter = nullptr;
    void operator()(TfLiteDelegate* delegate) const {
      if (deleter && delegate) {
        deleter(delegate);
      }
    }
  };

  void Shutdown() {
    interpreter_.reset();
    options_.reset();
    model_.reset();
    delegate_.reset();
    runtime_.Release();
  }

  MpNormalizedRect DefaultRect() const {
    MpNormalizedRect rect;
    rect.x_center = 0.5f;
    rect.y_center = 0.5f;
    rect.width = 1.0f;
    rect.height = 1.0f;
    rect.rotation = 0.0f;
    return rect;
  }

  MpNormalizedRect SanitizeRect(MpNormalizedRect rect) const {
    if (!(rect.width > 0.f) || !(rect.height > 0.f)) {
      return DefaultRect();
    }
    rect.x_center = Clamp(rect.x_center, 0.0f, 1.0f);
    rect.y_center = Clamp(rect.y_center, 0.0f, 1.0f);
    rect.width = Clamp(rect.width, 0.1f, 2.0f);
    rect.height = Clamp(rect.height, 0.1f, 2.0f);
    rect.rotation = NormalizeAngle(rect.rotation);
    return rect;
  }

  static int NormalizeRotationDegrees(int rotation_degrees) {
    switch (rotation_degrees) {
      case 0:
      case 90:
      case 180:
      case 270:
        return rotation_degrees;
      default:
        return -1;
    }
  }

  MpFaceDetectorResult* InvokeAndDecode(int logical_width,
                                        int logical_height,
                                        const MpNormalizedRect& rect,
                                        const MpRoiTransformOptions* roi_transform = nullptr) {
    const size_t input_bytes = input_buffer_.size() * sizeof(float);
    if (runtime_.TensorCopyFromBuffer(input_tensor_, input_buffer_.data(),
                                      input_bytes) != kTfLiteOk) {
      SetError("Failed to copy detector input buffer.");
      return nullptr;
    }
    if (runtime_.InterpreterInvoke(interpreter_.get()) != kTfLiteOk) {
      SetError("Detector invocation failed.");
      return nullptr;
    }
    if (runtime_.TensorCopyToBuffer(output_boxes_tensor_, boxes_buffer_.data(),
                                    boxes_buffer_.size() * sizeof(float)) !=
        kTfLiteOk) {
      SetError("Unable to read detector box output.");
      return nullptr;
    }
    if (runtime_.TensorCopyToBuffer(output_scores_tensor_, scores_buffer_.data(),
                                    scores_buffer_.size() * sizeof(float)) !=
        kTfLiteOk) {
      SetError("Unable to read detector score output.");
      return nullptr;
    }

    std::vector<DecodedDetection> detections = DecodeDetections(rect);
    if (detections.empty()) {
      return BuildResult(logical_width, logical_height, detections, roi_transform);
    }

    std::sort(detections.begin(), detections.end(),
              [](const DecodedDetection& a, const DecodedDetection& b) {
                return a.score > b.score;
              });

    std::vector<DecodedDetection> selected;
    selected.reserve(
        std::min<int>(max_results_, static_cast<int>(detections.size())));
    std::vector<bool> merged(detections.size(), false);
    for (size_t i = 0; i < detections.size(); ++i) {
      if (merged[i]) {
        continue;
      }

      std::vector<const DecodedDetection*> group;
      group.push_back(&detections[i]);
      merged[i] = true;
      for (size_t j = i + 1; j < detections.size(); ++j) {
        if (merged[j]) {
          continue;
        }
        if (IntersectionOverUnion(detections[i], detections[j]) >
            min_suppression_threshold_) {
          group.push_back(&detections[j]);
          merged[j] = true;
        }
      }

      selected.push_back(WeightedMergeDetections(group));
      if (static_cast<int>(selected.size()) >= max_results_) {
        break;
      }
    }

    return BuildResult(logical_width, logical_height, selected, roi_transform);
  }

  std::vector<DecodedDetection> DecodeDetections(
      const MpNormalizedRect& /*rect*/) const {
    std::vector<DecodedDetection> detections;
    detections.reserve(num_boxes_);

    for (int box_index = 0; box_index < num_boxes_; ++box_index) {
      float max_score = scores_buffer_[box_index * num_classes_];
      for (int class_index = 1; class_index < num_classes_; ++class_index) {
        max_score = std::max(
            max_score, scores_buffer_[box_index * num_classes_ + class_index]);
      }
      const float score = Sigmoid(max_score);
      if (score < min_detection_confidence_) {
        continue;
      }

      const Anchor& anchor = anchors_[box_index];
      const float* raw = boxes_buffer_.data() + box_index * 16;

      // MediaPipe configures BlazeFace with reverse_output_order=true, which
      // means boxes/keypoints are decoded as x/y/w/h rather than y/x/h/w.
      const float x_center =
          raw[0] / x_scale_ * anchor.width + anchor.x_center;
      const float y_center =
          raw[1] / y_scale_ * anchor.height + anchor.y_center;
      const float width = raw[2] / w_scale_ * anchor.width;
      const float height = raw[3] / h_scale_ * anchor.height;
      if (!(width > 0.0f) || !(height > 0.0f)) {
        continue;
      }

      const float x_min = x_center - width * 0.5f;
      const float y_min = y_center - height * 0.5f;
      const float x_max = x_center + width * 0.5f;
      const float y_max = y_center + height * 0.5f;

      DecodedDetection detection;
      float min_x = static_cast<float>(last_image_width_);
      float min_y = static_cast<float>(last_image_height_);
      float max_x = 0.0f;
      float max_y = 0.0f;
      const float kBoxCorners[4][2] = {
          {x_min, y_min},
          {x_max, y_min},
          {x_min, y_max},
          {x_max, y_max},
      };
      for (const auto& corner : kBoxCorners) {
        float image_x = 0.0f;
        float image_y = 0.0f;
        if (!ProjectTensorPoint(corner[0], corner[1], &image_x, &image_y)) {
          continue;
        }
        min_x = std::min(min_x, image_x);
        min_y = std::min(min_y, image_y);
        max_x = std::max(max_x, image_x);
        max_y = std::max(max_y, image_y);
      }
      detection.left = Clamp(min_x, 0.0f, static_cast<float>(last_image_width_));
      detection.top = Clamp(min_y, 0.0f, static_cast<float>(last_image_height_));
      detection.right =
          Clamp(max_x, 0.0f, static_cast<float>(last_image_width_));
      detection.bottom =
          Clamp(max_y, 0.0f, static_cast<float>(last_image_height_));
      if (detection.right <= detection.left ||
          detection.bottom <= detection.top) {
        continue;
      }
      detection.left = Clamp(detection.left / last_image_width_, 0.0f, 1.0f);
      detection.top = Clamp(detection.top / last_image_height_, 0.0f, 1.0f);
      detection.right = Clamp(detection.right / last_image_width_, 0.0f, 1.0f);
      detection.bottom =
          Clamp(detection.bottom / last_image_height_, 0.0f, 1.0f);
      for (int keypoint_index = 0; keypoint_index < 6; ++keypoint_index) {
        const int coord_index = 4 + keypoint_index * 2;
        const float keypoint_x =
            raw[coord_index] / x_scale_ * anchor.width + anchor.x_center;
        const float keypoint_y =
            raw[coord_index + 1] / y_scale_ * anchor.height +
            anchor.y_center;
        float image_x = 0.0f;
        float image_y = 0.0f;
        if (ProjectTensorPoint(keypoint_x, keypoint_y, &image_x, &image_y)) {
          detection.keypoints[keypoint_index * 2] =
              Clamp(image_x / last_image_width_, 0.0f, 1.0f);
          detection.keypoints[keypoint_index * 2 + 1] =
              Clamp(image_y / last_image_height_, 0.0f, 1.0f);
        }
      }
      detection.score = score;
      detections.push_back(detection);
    }

    return detections;
  }

  MpFaceDetectorResult* BuildResult(
      int image_width,
      int image_height,
      const std::vector<DecodedDetection>& detections,
      const MpRoiTransformOptions* roi_transform = nullptr) {
    MpFaceDetectorResult* result = new MpFaceDetectorResult();
    if (!result) {
      SetError("Unable to allocate detector result.");
      return nullptr;
    }
    result->image_width = image_width;
    result->image_height = image_height;
    result->detections_count = static_cast<int32_t>(detections.size());
    result->detections = nullptr;
    if (detections.empty()) {
      return result;
    }

    result->detections = new MpDetection[detections.size()];
    if (!result->detections) {
      delete result;
      SetError("Unable to allocate detector detections buffer.");
      return nullptr;
    }
    for (size_t i = 0; i < detections.size(); ++i) {
      result->detections[i].left = detections[i].left;
      result->detections[i].top = detections[i].top;
      result->detections[i].right = detections[i].right;
      result->detections[i].bottom = detections[i].bottom;
      result->detections[i].score = detections[i].score;
      std::memcpy(result->detections[i].keypoints, detections[i].keypoints.data(),
                  sizeof(result->detections[i].keypoints));
      const RectInPixels face_rect = DetectionToRect(detections[i]);
      const float roi_sx = roi_transform ? roi_transform->scale_x : 1.0f;
      const float roi_sy = roi_transform ? roi_transform->scale_y : 1.0f;
      const float roi_dx = roi_transform ? roi_transform->shift_x : 0.0f;
      const float roi_dy = roi_transform ? roi_transform->shift_y : 0.0f;
      const RectInPixels expanded_face_rect =
          TransformRect(face_rect, roi_sx, roi_sy, true, roi_dx, roi_dy);
      result->detections[i].face_rect =
          ToNormalizedRect(face_rect, image_width, image_height);
      result->detections[i].expanded_face_rect =
          ToNormalizedRect(expanded_face_rect, image_width, image_height);
    }
    return result;
  }

  bool ConfigureDecoder() {
    if (input_width_ == 128 && input_height_ == 128 && num_boxes_ == 896) {
      anchors_ = GenerateShortRangeAnchors();
      x_scale_ = 128.0f;
      y_scale_ = 128.0f;
      w_scale_ = 128.0f;
      h_scale_ = 128.0f;
    } else if (input_width_ == 192 && input_height_ == 192 &&
               num_boxes_ == 2304) {
      anchors_ = GenerateFullRangeAnchors();
      x_scale_ = 192.0f;
      y_scale_ = 192.0f;
      w_scale_ = 192.0f;
      h_scale_ = 192.0f;
    } else {
      SetError("Unsupported face detector model shape.");
      return false;
    }
    if (anchors_.empty()) {
      SetError("Failed to generate detector anchors.");
      return false;
    }
    return true;
  }

  RectInPixels ToPixelRect(const MpNormalizedRect& rect,
                           int width,
                           int height) const {
    RectInPixels roi;
    roi.center_x = rect.x_center * static_cast<float>(width);
    roi.center_y = rect.y_center * static_cast<float>(height);
    roi.width = rect.width * static_cast<float>(width);
    roi.height = rect.height * static_cast<float>(height);
    roi.rotation = rect.rotation;
    if (roi.width <= 0.0f) {
      roi.width = static_cast<float>(width);
    }
    if (roi.height <= 0.0f) {
      roi.height = static_cast<float>(height);
    }
    return roi;
  }

  RectInPixels DetectionToRect(const DecodedDetection& detection) const {
    RectInPixels rect;
    rect.center_x =
        (detection.left + detection.right) * 0.5f * last_image_width_;
    rect.center_y =
        (detection.top + detection.bottom) * 0.5f * last_image_height_;
    rect.width = (detection.right - detection.left) * last_image_width_;
    rect.height = (detection.bottom - detection.top) * last_image_height_;

    const float left_eye_x = detection.keypoints[0] * last_image_width_;
    const float left_eye_y = detection.keypoints[1] * last_image_height_;
    const float right_eye_x = detection.keypoints[2] * last_image_width_;
    const float right_eye_y = detection.keypoints[3] * last_image_height_;
    const float dx = right_eye_x - left_eye_x;
    const float dy = right_eye_y - left_eye_y;
    if (std::abs(dx) > 1e-6f || std::abs(dy) > 1e-6f) {
      rect.rotation = NormalizeAngle(std::atan2(dy, dx));
    }
    return rect;
  }

  RectInPixels TransformRect(const RectInPixels& rect,
                             float scale_x,
                             float scale_y,
                             bool square_long = false,
                             float shift_x = 0.0f,
                             float shift_y = 0.0f) const {
    RectInPixels transformed = rect;
    if (square_long) {
      const float long_side =
          std::max(rect.width, rect.height);
      transformed.width = long_side;
      transformed.height = long_side;
    }

    const float cos_r = std::cos(rect.rotation);
    const float sin_r = std::sin(rect.rotation);
    transformed.center_x =
        rect.center_x + rect.width * shift_x * cos_r -
        rect.height * shift_y * sin_r;
    transformed.center_y =
        rect.center_y + rect.width * shift_x * sin_r +
        rect.height * shift_y * cos_r;
    transformed.width *= scale_x;
    transformed.height *= scale_y;
    return transformed;
  }

  TensorTransform CreateTensorTransform(const RectInPixels& roi) const {
    TensorTransform transform;
    transform.center_x = roi.center_x;
    transform.center_y = roi.center_y;
    transform.roi_width = roi.width;
    transform.roi_height = roi.height;
    transform.rotation = roi.rotation;
    transform.scale =
        std::min(static_cast<float>(input_width_) / roi.width,
                 static_cast<float>(input_height_) / roi.height);
    transform.scaled_width = roi.width * transform.scale;
    transform.scaled_height = roi.height * transform.scale;
    transform.pad_x =
        (static_cast<float>(input_width_) - transform.scaled_width) * 0.5f;
    transform.pad_y =
        (static_cast<float>(input_height_) - transform.scaled_height) * 0.5f;
    return transform;
  }

  void UpdateProjectionMatrix() {
    if (last_transform_.scale <= 0.0f || last_image_width_ <= 0 ||
        last_image_height_ <= 0) {
      last_projection_ = ProjectionMatrix{};
      return;
    }

    const float inv_image_width = 1.0f / last_image_width_;
    const float inv_image_height = 1.0f / last_image_height_;
    const float inv_scale = 1.0f / last_transform_.scale;
    const float cos_r = std::cos(last_transform_.rotation);
    const float sin_r = std::sin(last_transform_.rotation);
    const float half_roi_width = last_transform_.roi_width * 0.5f;
    const float half_roi_height = last_transform_.roi_height * 0.5f;

    const float bx =
        (-last_transform_.pad_x) * inv_scale - half_roi_width;
    const float by =
        (-last_transform_.pad_y) * inv_scale - half_roi_height;
    const float cx =
        cos_r * bx - sin_r * by + last_transform_.center_x;
    const float cy =
        sin_r * bx + cos_r * by + last_transform_.center_y;

    last_projection_.m00 =
        cos_r * input_width_ * inv_scale * inv_image_width;
    last_projection_.m01 =
        -sin_r * input_height_ * inv_scale * inv_image_width;
    last_projection_.m02 = cx * inv_image_width;
    last_projection_.m10 =
        sin_r * input_width_ * inv_scale * inv_image_height;
    last_projection_.m11 =
        cos_r * input_height_ * inv_scale * inv_image_height;
    last_projection_.m12 = cy * inv_image_height;
  }

  bool ProjectTensorPoint(float x_norm,
                          float y_norm,
                          float* image_x,
                          float* image_y) const {
    if (!image_x || !image_y) {
      return false;
    }
    *image_x = (last_projection_.m00 * x_norm + last_projection_.m01 * y_norm +
                last_projection_.m02) *
               last_image_width_;
    *image_y = (last_projection_.m10 * x_norm + last_projection_.m11 * y_norm +
                last_projection_.m12) *
               last_image_height_;
    return true;
  }

  bool Preprocess(const MpImage& image, const MpNormalizedRect& rect) {
    const RectInPixels roi = ToPixelRect(rect, image.width, image.height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    last_transform_ = CreateTensorTransform(roi);
    last_image_width_ = image.width;
    last_image_height_ = image.height;
    UpdateProjectionMatrix();
    const float cos_r = std::cos(last_transform_.rotation);
    const float sin_r = std::sin(last_transform_.rotation);

    float* dst = input_buffer_.data();
    size_t offset = 0;
    for (int y = 0; y < input_height_; ++y) {
      const float tensor_y = static_cast<float>(y) + 0.5f;
      for (int x = 0; x < input_width_; ++x) {
        const float tensor_x = static_cast<float>(x) + 0.5f;
        const float roi_x =
            (tensor_x - last_transform_.pad_x) / last_transform_.scale;
        const float roi_y =
            (tensor_y - last_transform_.pad_y) / last_transform_.scale;
        if (roi_x < 0.0f || roi_y < 0.0f || roi_x > last_transform_.roi_width ||
            roi_y > last_transform_.roi_height) {
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          continue;
        }
        const float local_x = roi_x - last_transform_.roi_width * 0.5f;
        const float local_y = roi_y - last_transform_.roi_height * 0.5f;
        const float source_x =
            cos_r * local_x - sin_r * local_y + last_transform_.center_x;
        const float source_y =
            sin_r * local_x + cos_r * local_y + last_transform_.center_y;
        const RgbPixel pixel = BilinearSample(image, source_x, source_y);
        dst[offset++] = pixel.r / 127.5f - 1.0f;
        dst[offset++] = pixel.g / 127.5f - 1.0f;
        dst[offset++] = pixel.b / 127.5f - 1.0f;
      }
    }
    return true;
  }

  bool PreprocessRotated(const MpImage& image,
                         const MpNormalizedRect& rect,
                         int rotation_degrees,
                         bool mirror_horizontal,
                         int rotated_width,
                         int rotated_height) {
    const RectInPixels roi = ToPixelRect(rect, rotated_width, rotated_height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    last_transform_ = CreateTensorTransform(roi);
    last_image_width_ = rotated_width;
    last_image_height_ = rotated_height;
    UpdateProjectionMatrix();
    const float cos_r = std::cos(last_transform_.rotation);
    const float sin_r = std::sin(last_transform_.rotation);

    float* dst = input_buffer_.data();
    size_t offset = 0;
    for (int y = 0; y < input_height_; ++y) {
      const float tensor_y = static_cast<float>(y) + 0.5f;
      for (int x = 0; x < input_width_; ++x) {
        const float tensor_x = static_cast<float>(x) + 0.5f;
        const float roi_x =
            (tensor_x - last_transform_.pad_x) / last_transform_.scale;
        const float roi_y =
            (tensor_y - last_transform_.pad_y) / last_transform_.scale;
        if (roi_x < 0.0f || roi_y < 0.0f || roi_x > last_transform_.roi_width ||
            roi_y > last_transform_.roi_height) {
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          continue;
        }
        const float local_x = roi_x - last_transform_.roi_width * 0.5f;
        const float local_y = roi_y - last_transform_.roi_height * 0.5f;
        const float source_x =
            cos_r * local_x - sin_r * local_y + last_transform_.center_x;
        const float source_y =
            sin_r * local_x + cos_r * local_y + last_transform_.center_y;
        const RgbPixel pixel = BilinearSampleRotated(
            image, source_x, source_y, rotation_degrees, mirror_horizontal,
            rotated_width, rotated_height);
        dst[offset++] = pixel.r / 127.5f - 1.0f;
        dst[offset++] = pixel.g / 127.5f - 1.0f;
        dst[offset++] = pixel.b / 127.5f - 1.0f;
      }
    }
    return true;
  }

  bool PreprocessNv21(const MpNv21Image& image, const MpNormalizedRect& rect) {
    const RectInPixels roi = ToPixelRect(rect, image.width, image.height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    last_transform_ = CreateTensorTransform(roi);
    last_image_width_ = image.width;
    last_image_height_ = image.height;
    UpdateProjectionMatrix();
    const float cos_r = std::cos(last_transform_.rotation);
    const float sin_r = std::sin(last_transform_.rotation);

    float* dst = input_buffer_.data();
    size_t offset = 0;
    for (int y = 0; y < input_height_; ++y) {
      const float tensor_y = static_cast<float>(y) + 0.5f;
      for (int x = 0; x < input_width_; ++x) {
        const float tensor_x = static_cast<float>(x) + 0.5f;
        const float roi_x =
            (tensor_x - last_transform_.pad_x) / last_transform_.scale;
        const float roi_y =
            (tensor_y - last_transform_.pad_y) / last_transform_.scale;
        if (roi_x < 0.0f || roi_y < 0.0f || roi_x > last_transform_.roi_width ||
            roi_y > last_transform_.roi_height) {
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          continue;
        }
        const float local_x = roi_x - last_transform_.roi_width * 0.5f;
        const float local_y = roi_y - last_transform_.roi_height * 0.5f;
        const float source_x =
            cos_r * local_x - sin_r * local_y + last_transform_.center_x;
        const float source_y =
            sin_r * local_x + cos_r * local_y + last_transform_.center_y;
        const RgbPixel pixel = BilinearSampleNv21(image, source_x, source_y);
        dst[offset++] = pixel.r / 127.5f - 1.0f;
        dst[offset++] = pixel.g / 127.5f - 1.0f;
        dst[offset++] = pixel.b / 127.5f - 1.0f;
      }
    }
    return true;
  }

  bool PreprocessNv21Rotated(const MpNv21Image& image,
                             const MpNormalizedRect& rect,
                             int rotation_degrees,
                             bool mirror_horizontal,
                             int rotated_width,
                             int rotated_height) {
    const RectInPixels roi = ToPixelRect(rect, rotated_width, rotated_height);
    if (roi.width <= 0.f || roi.height <= 0.f) {
      SetError("Invalid ROI dimension.");
      return false;
    }
    last_transform_ = CreateTensorTransform(roi);
    last_image_width_ = rotated_width;
    last_image_height_ = rotated_height;
    UpdateProjectionMatrix();
    const float cos_r = std::cos(last_transform_.rotation);
    const float sin_r = std::sin(last_transform_.rotation);

    float* dst = input_buffer_.data();
    size_t offset = 0;
    for (int y = 0; y < input_height_; ++y) {
      const float tensor_y = static_cast<float>(y) + 0.5f;
      for (int x = 0; x < input_width_; ++x) {
        const float tensor_x = static_cast<float>(x) + 0.5f;
        const float roi_x =
            (tensor_x - last_transform_.pad_x) / last_transform_.scale;
        const float roi_y =
            (tensor_y - last_transform_.pad_y) / last_transform_.scale;
        if (roi_x < 0.0f || roi_y < 0.0f || roi_x > last_transform_.roi_width ||
            roi_y > last_transform_.roi_height) {
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          dst[offset++] = -1.0f;
          continue;
        }
        const float local_x = roi_x - last_transform_.roi_width * 0.5f;
        const float local_y = roi_y - last_transform_.roi_height * 0.5f;
        const float source_x =
            cos_r * local_x - sin_r * local_y + last_transform_.center_x;
        const float source_y =
            sin_r * local_x + cos_r * local_y + last_transform_.center_y;
        const RgbPixel pixel = BilinearSampleNv21Rotated(
            image, source_x, source_y, rotation_degrees, mirror_horizontal,
            rotated_width, rotated_height);
        dst[offset++] = pixel.r / 127.5f - 1.0f;
        dst[offset++] = pixel.g / 127.5f - 1.0f;
        dst[offset++] = pixel.b / 127.5f - 1.0f;
      }
    }
    return true;
  }

  RgbPixel BilinearSample(const MpImage& image, float x, float y) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(image.width - 1) ||
        y > static_cast<float>(image.height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, image.width - 1);
    const int y1 = std::min(y0 + 1, image.height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixel(image, x0, y0);
    const RgbPixel p10 = ReadPixel(image, x1, y0);
    const RgbPixel p01 = ReadPixel(image, x0, y1);
    const RgbPixel p11 = ReadPixel(image, x1, y1);
    return Lerp(Lerp(p00, p10, dx), Lerp(p01, p11, dx), dy);
  }

  RgbPixel BilinearSampleRotated(const MpImage& image,
                                 float x,
                                 float y,
                                 int rotation_degrees,
                                 bool mirror_horizontal,
                                 int rotated_width,
                                 int rotated_height) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(rotated_width - 1) ||
        y > static_cast<float>(rotated_height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, rotated_width - 1);
    const int y1 = std::min(y0 + 1, rotated_height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixelRotated(
        image, x0, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p10 = ReadPixelRotated(
        image, x1, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p01 = ReadPixelRotated(
        image, x0, y1, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p11 = ReadPixelRotated(
        image, x1, y1, rotation_degrees, mirror_horizontal, rotated_width);
    return Lerp(Lerp(p00, p10, dx), Lerp(p01, p11, dx), dy);
  }

  RgbPixel BilinearSampleNv21(const MpNv21Image& image,
                              float x,
                              float y) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(image.width - 1) ||
        y > static_cast<float>(image.height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, image.width - 1);
    const int y1 = std::min(y0 + 1, image.height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixelNv21(image, x0, y0);
    const RgbPixel p10 = ReadPixelNv21(image, x1, y0);
    const RgbPixel p01 = ReadPixelNv21(image, x0, y1);
    const RgbPixel p11 = ReadPixelNv21(image, x1, y1);
    return Lerp(Lerp(p00, p10, dx), Lerp(p01, p11, dx), dy);
  }

  RgbPixel BilinearSampleNv21Rotated(const MpNv21Image& image,
                                     float x,
                                     float y,
                                     int rotation_degrees,
                                     bool mirror_horizontal,
                                     int rotated_width,
                                     int rotated_height) const {
    if (x < 0.0f || y < 0.0f || x > static_cast<float>(rotated_width - 1) ||
        y > static_cast<float>(rotated_height - 1)) {
      return {};
    }
    const int x0 = static_cast<int>(std::floor(x));
    const int y0 = static_cast<int>(std::floor(y));
    const int x1 = std::min(x0 + 1, rotated_width - 1);
    const int y1 = std::min(y0 + 1, rotated_height - 1);
    const float dx = x - static_cast<float>(x0);
    const float dy = y - static_cast<float>(y0);

    const RgbPixel p00 = ReadPixelNv21Rotated(
        image, x0, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p10 = ReadPixelNv21Rotated(
        image, x1, y0, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p01 = ReadPixelNv21Rotated(
        image, x0, y1, rotation_degrees, mirror_horizontal, rotated_width);
    const RgbPixel p11 = ReadPixelNv21Rotated(
        image, x1, y1, rotation_degrees, mirror_horizontal, rotated_width);
    return Lerp(Lerp(p00, p10, dx), Lerp(p01, p11, dx), dy);
  }

  RgbPixel ReadPixel(const MpImage& image, int x, int y) const {
    const uint8_t* row =
        image.data + static_cast<size_t>(y) * image.bytes_per_row;
    const uint8_t* ptr = row + static_cast<size_t>(x) * 4;
    RgbPixel pixel;
    if (image.format == MP_PIXEL_FORMAT_RGBA) {
      pixel.r = static_cast<float>(ptr[0]);
      pixel.g = static_cast<float>(ptr[1]);
      pixel.b = static_cast<float>(ptr[2]);
    } else {
      pixel.r = static_cast<float>(ptr[2]);
      pixel.g = static_cast<float>(ptr[1]);
      pixel.b = static_cast<float>(ptr[0]);
    }
    return pixel;
  }

  RgbPixel ReadPixelNv21(const MpNv21Image& image, int x, int y) const {
    const uint8_t* y_row =
        image.y + static_cast<size_t>(y) * image.y_bytes_per_row;
    const uint8_t Y = y_row[static_cast<size_t>(x)];
    const int uv_x = x >> 1;
    const int uv_y = y >> 1;
    const uint8_t* vu_row =
        image.vu + static_cast<size_t>(uv_y) * image.vu_bytes_per_row;
    const size_t vu_index = static_cast<size_t>(uv_x) * 2;
    const uint8_t V = vu_row[vu_index];
    const uint8_t U = vu_row[vu_index + 1];

    const int C = static_cast<int>(Y) - 16;
    const int D = static_cast<int>(U) - 128;
    const int E = static_cast<int>(V) - 128;
    const int c = C < 0 ? 0 : C;

    RgbPixel pixel;
    pixel.r = static_cast<float>(ClampInt((298 * c + 409 * E + 128) >> 8, 0, 255));
    pixel.g = static_cast<float>(
        ClampInt((298 * c - 100 * D - 208 * E + 128) >> 8, 0, 255));
    pixel.b = static_cast<float>(ClampInt((298 * c + 516 * D + 128) >> 8, 0, 255));
    return pixel;
  }

  RgbPixel ReadPixelRotated(const MpImage& image,
                            int x_rot,
                            int y_rot,
                            int rotation_degrees,
                            bool mirror_horizontal,
                            int rotated_width) const {
    int x_raw = 0;
    int y_raw = 0;
    MapRotatedToRaw(x_rot, y_rot, rotation_degrees, mirror_horizontal,
                    image.width, image.height, rotated_width, x_raw, y_raw);
    return ReadPixel(image, x_raw, y_raw);
  }

  static inline void MapRotatedToRaw(int x_rot,
                                     int y_rot,
                                     int rotation_degrees,
                                     bool mirror_horizontal,
                                     int raw_width,
                                     int raw_height,
                                     int rotated_width,
                                     int& out_x,
                                     int& out_y) {
    int xr = x_rot;
    int yr = y_rot;
    if (mirror_horizontal) {
      xr = (rotated_width - 1) - xr;
    }
    switch (rotation_degrees) {
      case 90:
        out_x = yr;
        out_y = (raw_height - 1) - xr;
        break;
      case 180:
        out_x = (raw_width - 1) - xr;
        out_y = (raw_height - 1) - yr;
        break;
      case 270:
        out_x = (raw_width - 1) - yr;
        out_y = xr;
        break;
      case 0:
      default:
        out_x = xr;
        out_y = yr;
        break;
    }
    out_x = ClampInt(out_x, 0, raw_width - 1);
    out_y = ClampInt(out_y, 0, raw_height - 1);
  }

  RgbPixel ReadPixelNv21Rotated(const MpNv21Image& image,
                                int x_rot,
                                int y_rot,
                                int rotation_degrees,
                                bool mirror_horizontal,
                                int rotated_width) const {
    int x_raw = 0;
    int y_raw = 0;
    MapRotatedToRaw(x_rot, y_rot, rotation_degrees, mirror_horizontal,
                    image.width, image.height, rotated_width, x_raw, y_raw);
    return ReadPixelNv21(image, x_raw, y_raw);
  }

  RgbPixel Lerp(const RgbPixel& a, const RgbPixel& b, float t) const {
    const float blend = Clamp(t, 0.0f, 1.0f);
    RgbPixel out;
    out.r = a.r + (b.r - a.r) * blend;
    out.g = a.g + (b.g - a.g) * blend;
    out.b = a.b + (b.b - a.b) * blend;
    return out;
  }

  void SetError(const std::string& message) {
    last_error_ = message;
    MP_DETECT_LOGE("%s\n", message.c_str());
  }

  TfLiteRuntime runtime_;
  std::unique_ptr<TfLiteModel, TfLiteModelDeleter> model_{nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreterOptions, TfLiteOptionsDeleter> options_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteInterpreter, TfLiteInterpreterDeleter> interpreter_{
      nullptr, {&runtime_}};
  std::unique_ptr<TfLiteDelegate, TfLiteDelegateDeleter> delegate_{nullptr, {}};

  TfLiteTensor* input_tensor_ = nullptr;
  const TfLiteTensor* output_boxes_tensor_ = nullptr;
  const TfLiteTensor* output_scores_tensor_ = nullptr;

  int input_width_ = 0;
  int input_height_ = 0;
  int threads_ = 2;
  int boxes_count_ = 0;
  int scores_count_ = 0;
  int num_boxes_ = 0;
  int num_classes_ = 0;
  int max_results_ = 1;
  int last_image_width_ = 0;
  int last_image_height_ = 0;
  float min_detection_confidence_ = 0.5f;
  float min_suppression_threshold_ = 0.3f;
  float x_scale_ = 128.0f;
  float y_scale_ = 128.0f;
  float w_scale_ = 128.0f;
  float h_scale_ = 128.0f;
  MpDelegateType active_delegate_ = MP_DELEGATE_CPU;

  std::string last_error_;
  ProjectionMatrix last_projection_;
  TensorTransform last_transform_;
  std::vector<float> input_buffer_;
  std::vector<float> boxes_buffer_;
  std::vector<float> scores_buffer_;
  std::vector<Anchor> anchors_;
};

std::string& GlobalFaceDetectorError() {
  static thread_local std::string error;
  return error;
}

}  // namespace

struct MpFaceDetectorContext {
  FaceDetectorContext impl;
};

namespace {

// No C++ exception may unwind across the FFI boundary into Dart (the process
// would abort), so pointer-returning exports run their body through this
// guard, which converts an escaped exception (std::bad_alloc included) into
// a null return with the error recorded on the context, when there is one,
// and globally.
template <typename Fn>
auto GuardedNative(MpFaceDetectorContext* context, Fn&& body)
    -> decltype(body()) {
  try {
    return body();
  } catch (const std::exception& e) {
    const std::string message =
        std::string("Unhandled native exception: ") + e.what();
    if (context) {
      context->impl.RecordError(message);
    }
    GlobalFaceDetectorError() = message;
    return nullptr;
  } catch (...) {
    const std::string message = "Unhandled native exception.";
    if (context) {
      context->impl.RecordError(message);
    }
    GlobalFaceDetectorError() = message;
    return nullptr;
  }
}

}  // namespace

extern "C" {

MpFaceDetectorContext* mp_face_detector_create(
    const char* model_path,
    const MpFaceDetectorCreateOptions* options) {
  return GuardedNative(nullptr, [&]() -> MpFaceDetectorContext* {
    if (!model_path || std::strlen(model_path) == 0) {
      GlobalFaceDetectorError() = "Model path is empty.";
      return nullptr;
    }
    auto* context = new MpFaceDetectorContext();
    if (!context->impl.Initialize(model_path, options)) {
      GlobalFaceDetectorError() = context->impl.last_error();
      delete context;
      return nullptr;
    }
    GlobalFaceDetectorError().clear();
    return context;
  });
}

void mp_face_detector_destroy(MpFaceDetectorContext* context) {
  try {
    delete context;
  } catch (...) {
  }
}

MpFaceDetectorResult* mp_face_detector_process(
    MpFaceDetectorContext* context,
    const MpImage* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal,
    const MpRoiTransformOptions* roi_transform) {
  return GuardedNative(context, [&]() -> MpFaceDetectorResult* {
    if (!context || !image) {
      GlobalFaceDetectorError() = "Detector context or image is null.";
      return nullptr;
    }
    MpFaceDetectorResult* result =
        context->impl.Process(*image, override_rect, rotation_degrees,
                              mirror_horizontal != 0, roi_transform);
    if (!result) {
      GlobalFaceDetectorError() = context->impl.last_error();
    } else {
      GlobalFaceDetectorError().clear();
    }
    return result;
  });
}

MpFaceDetectorResult* mp_face_detector_process_nv21(
    MpFaceDetectorContext* context,
    const MpNv21Image* image,
    const MpNormalizedRect* override_rect,
    int32_t rotation_degrees,
    uint8_t mirror_horizontal,
    const MpRoiTransformOptions* roi_transform) {
  return GuardedNative(context, [&]() -> MpFaceDetectorResult* {
    if (!context || !image) {
      GlobalFaceDetectorError() = "Detector context or image is null.";
      return nullptr;
    }
    MpFaceDetectorResult* result =
        context->impl.ProcessNv21(*image, override_rect, rotation_degrees,
                                  mirror_horizontal != 0, roi_transform);
    if (!result) {
      GlobalFaceDetectorError() = context->impl.last_error();
    } else {
      GlobalFaceDetectorError().clear();
    }
    return result;
  });
}

void mp_face_detector_release_result(MpFaceDetectorResult* result) {
  if (!result) {
    return;
  }
  delete[] result->detections;
  result->detections = nullptr;
  delete result;
}

const char* mp_face_detector_last_error(const MpFaceDetectorContext* context) {
  if (!context) {
    return "";
  }
  return context->impl.last_error();
}

const char* mp_face_detector_last_global_error(void) {
  return GlobalFaceDetectorError().c_str();
}

MpDelegateType mp_face_detector_active_delegate(
    const MpFaceDetectorContext* context) {
  if (!context) {
    return MP_DELEGATE_CPU;
  }
  return context->impl.active_delegate();
}

}  // extern "C"


```
