# Facial rendering source scan: mitsuba-renderer/mitsuba3

Repository: https://github.com/mitsuba-renderer/mitsuba3

Pinned source revision: fd9c9ceb38deb511a5c3f8479c8b14c0b552167e

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "docs/exts/sphinxtr/LICENSE"]

Topic paths: {"face": ["include/mitsuba/render/microfacet.h", "src/render/microfacet.cpp", "src/render/python/microfacet.cpp", "src/render/python/microfacet_v.cpp", "src/render/tests/test_microfacet.py"], "lip": ["src/shapes/ellipsoids.cpp", "src/shapes/ellipsoids.h", "src/shapes/ellipsoidsmesh.cpp", "src/shapes/tests/test_ellipsoidsmesh.py"]}

## Source file: src/shapes/ellipsoids.h

{
  "path": "src/shapes/ellipsoids.h",
  "blob_sha": "af4d6236986acee865b9665b546a2ab1b5851862",
  "sha256": "53176201d081f240e9d52dca07b30032d4cc836b7f260e588d2b5869e75b243a",
  "lines": 497,
  "bytes": 21918,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 3,
    "lip": 59
  },
  "url": "https://github.com/mitsuba-renderer/mitsuba3/blob/fd9c9ceb38deb511a5c3f8479c8b14c0b552167e/src/shapes/ellipsoids.h",
  "function_signature_candidates": [
    {
      "line": 295,
      "text": "void traverse(TraversalCallback *cb) {"
    },
    {
      "line": 304,
      "text": "void parameters_changed() {"
    },
    {
      "line": 316,
      "text": "void compute_extents() {"
    },
    {
      "line": 331,
      "text": "bool has_attribute(std::string_view name) const {"
    }
  ]
}

```text

#pragma once

#include <mitsuba/render/fwd.h>
#include <mitsuba/core/fwd.h>
#include <mitsuba/core/properties.h>
#include <mitsuba/core/fresolver.h>
#include <mitsuba/core/profiler.h>
#include <drjit/tensor.h>
#include <drjit/quaternion.h>
#include <tsl/robin_map.h>

#include "ply.h"

NAMESPACE_BEGIN(mitsuba)

template <typename T>
struct Ellipsoid {
    Point<T, 3> center;     // Center values
    Vector<T, 3> scale;     // Scale values
    dr::Quaternion<T> quat; // To world rotation quaternion values
};

constexpr uint32_t EllipsoidStructSize = 10u;

/**
 * Generic container class for ellipsoids.
 *
 * This is a convenience data structure meant to hold ellipsoids shape
 * data (centers, scales, rotation) and its extra attributes.
 */
template <typename Float, typename Spectrum>
class EllipsoidsData {
public:
    MI_IMPORT_TYPES()
    using FloatStorage  = DynamicBuffer<dr::replace_scalar_t<Float, float>>;
    using BoolStorage   = DynamicBuffer<Mask>;
    using UInt32Storage = DynamicBuffer<UInt32>;
    using ArrayXf       = dr::DynamicArray<Float>;
    using AttributesMap = tsl::robin_map<std::string, FloatStorage, std::hash<std::string_view>, std::equal_to<>>;

    EllipsoidsData() { }


    /// Constructor from a single PLY file or multiple separate tensors
    EllipsoidsData(const Properties &props) {
        if (props.has_property("filename")) {
            if (props.has_property("data"))
                Throw("Cannot specify both \"data\" and \"filename\".");
            if (props.has_property("centers"))
                Throw("Cannot specify both \"centers\" and \"filename\".");

            FileResolver* fs = mitsuba::file_resolver();
            fs::path file_path = fs->resolve(props.get<std::string_view>("filename"));
            std::string name = file_path.filename().string();

            auto fail = [&](const char *descr) {
                Throw("Error while loading PLY file \"%s\": %s!", name, descr);
            };

            Log(Debug, "Loading ellipsoids from \"%s\" ..", name);
            if (!fs::exists(file_path))
                fail("file not found");

            ref<FileStream> file = new FileStream(file_path);
            ref<Stream> stream = file.get();
            ScopedPhase phase(ProfilerPhase::LoadGeometry);

            PLYHeader header;
            try {
                header = parse_ply_header(stream, name);
                if (header.ascii) {
                    if (stream->size() > 100 * 1024)
                        Log(Warn,
                            "\"%s\": performance warning -- this file uses the ASCII PLY format, which "
                            "is slow to parse. Consider converting it to the binary PLY format.",
                            name);
                    stream = parse_ascii(file.get(), header.elements, name);
                }
            } catch (const std::exception &e) {
                fail(e.what());
            }

            auto &el = header.elements[0];

            // Check that the PLY files are structured as follow:
            // x, y, z
            // nx, ny, nz
            //  ... extras ...
            // scale_0, scale_1, scale_2
            // rot_0, rot_1, rot_2, rot_3

            bool failure = false;
            size_t i1 = 0;
            for (auto &f : { "x", "y", "z", "nx", "ny", "nz" })
                failure |= (el.struct_[i1++].name != f);

            size_t i2 = el.struct_.size() - 7;
            for (auto &f : { "scale_0", "scale_1", "scale_2", "rot_0", "rot_1", "rot_2", "rot_3" })
                failure |= (el.struct_[i2++].name != f);

            if (failure) {
                std::cout << "el.struct: " << el.struct_ << std::endl;
                Throw("Invalid structure in PLY file!");
            }

            size_t extras_count = el.struct_.size() - 13;
            std::vector<std::pair<std::string, uint32_t>> extras;

            bool is_3dg = (el.struct_[6].name == "f_dc_0" && el.struct_[el.struct_.size() - 8].name == "opacity");

            if (!is_3dg) {
                size_t i = 0;
                std::string prefix = "";
                size_t count = 0;
                while (i < extras_count) {
                    auto name2 = el.struct_[6 + i].name;
                    auto current_prefix = name2.substr(0, name2.find_last_of("_"));
                    if (prefix == current_prefix) {
                        count++;
                    } else {
                        if (count > 0)
                            extras.push_back({ prefix, (uint32_t) count });
                        prefix = current_prefix;
                        count = 1;
                    }
                    i++;
                }
                extras.push_back({ prefix, (uint32_t) count });
            } else {
                extras.push_back({ "sh_coeffs", (uint32_t) extras_count - 1 });
                extras.push_back({ "opacity", 1 });
            }

            size_t scale_offset = el.struct_.size() - 7;
            size_t quat_offset  = el.struct_.size() - 4;

            std::unique_ptr<float[]> buf(new float[el.struct_.size()]);
            std::unique_ptr<float[]> ellipsoid_data(new float[el.count * EllipsoidStructSize]);

            std::vector<std::unique_ptr<float[]>> extras_data;
            for (auto [name2, dim] : extras)
                extras_data.push_back(std::unique_ptr<float[]>(new float[el.count * dim]));

            ScalarFloat scale_factor = props.get<ScalarFloat>("scale_factor", 1.f);

            auto to_world = props.get<ScalarAffineTransform4f>("to_world", ScalarAffineTransform4f());
            auto [to_world_S, to_world_Q, to_world_T] = transform_decompose(to_world.matrix, 25);
            float to_world_scale = dr::mean(dr::diag(to_world_S));

            size_t count = 0;
            for (size_t i = 0; i < el.count; i++) {
                stream->read(buf.get(), el.struct_.nbytes());

                ScalarPoint3f center = dr::load<ScalarPoint3f>(buf.get());
                center = to_world * center;

                ScalarPoint3f scale  = dr::load<ScalarPoint3f>(buf.get() + scale_offset);
                scale = dr::exp(scale); // Scaling activation (exponential)
                scale = dr::maximum(scale, 1e-6f);
                scale *= scale_factor;
                scale *= to_world_scale;

                ScalarQuaternion4f quat = ScalarQuaternion4f(
                    buf.get()[quat_offset + 1], // i
                    buf.get()[quat_offset + 2], // j
                    buf.get()[quat_offset + 3], // k
                    buf.get()[quat_offset + 0]  // r
                );
                quat = to_world_Q * quat;
                quat = dr::normalize(quat);

                dr::store(ellipsoid_data.get() + EllipsoidStructSize * count + 0, center);
                dr::store(ellipsoid_data.get() + EllipsoidStructSize * count + 3, scale);
                dr::store(ellipsoid_data.get() + EllipsoidStructSize * count + 6, quat);

                if (is_3dg) {
                    size_t sh_coeffs_count = extras_count - 1;
                    size_t sh_n = sh_coeffs_count / 3;
                    extras_data[0].get()[count * sh_coeffs_count + 0] = buf.get()[6 + 0];
                    extras_data[0].get()[count * sh_coeffs_count + 1] = buf.get()[6 + 1];
                    extras_data[0].get()[count * sh_coeffs_count + 2] = buf.get()[6 + 2];
                    for (size_t j = 1; j < sh_n; j++) { // SH coefficients are stored in a strange order!?
                        extras_data[0].get()[count * sh_coeffs_count + j * 3 + 0] = buf.get()[6 + (j - 1) + 3];
                        extras_data[0].get()[count * sh_coeffs_count + j * 3 + 1] = buf.get()[6 + (j - 1) + sh_n + 2];
                        extras_data[0].get()[count * sh_coeffs_count + j * 3 + 2] = buf.get()[6 + (j - 1) + 2 * sh_n + 1];
                    }

                    float opacity = buf.get()[el.struct_.size() - 8];
                    opacity = 1.f / (1.f + dr::exp(-opacity)); // Opacity activation (sigmoid)
                    opacity = dr::clip(opacity, 1e-8f, 1.f - 1e-8f);
                    extras_data[1].get()[count] = opacity;
                } else {
                    size_t offset = 0;
                    for (size_t j = 0; j < extras.size(); j++) {
                        size_t dim = extras[j].second;
                        for (size_t k = 0; k < dim; k++) {
                            extras_data[j].get()[count * dim + k] = buf.get()[6 + offset++];
                        }
                    }
                }

                count++;
            }

            file->close();

            m_data = dr::load<FloatStorage>(ellipsoid_data.get(), count * EllipsoidStructSize);

            if (is_3dg) {
                m_attributes.insert({ "sh_coeffs", dr::load<FloatStorage>(extras_data[0].get(), count * (extras_count - 1)) });
                m_attributes.insert({ "opacities", dr::load<FloatStorage>(extras_data[1].get(), count) });
            } else {
                for (size_t i = 0; i < extras.size(); i++)
                    m_attributes.insert({ extras[i].first, dr::load<FloatStorage>(extras_data[i].get(), count * extras[i].second) });
            }
        } else if (props.has_property("data")) {
            if (props.has_property("filename"))
                Throw("Cannot specify both \"data\" and \"filename\".");
            if (props.has_property("centers"))
                Throw("Cannot specify both \"centers\" and \"data\".");
            if (props.has_property("scale_factor"))
                Throw("\"scale_factor\" parameter is only supported with PLY files!");
            if (props.has_property("to_world"))
                Throw("\"to_world\" is only supported when loading PLY file!");

            const TensorXf &data = props.get_any<TensorXf>("data");
            if (data.ndim() > 1 && data.shape(1) != EllipsoidStructSize)
                Throw("TensorXf data must have shape (N, EllipsoidStructSize) or (N * EllipsoidStructSize)!");

            if (data.ndim() == 1 && data.shape(0) % EllipsoidStructSize != 0)
                Throw("Flat TensorXf data width must be a multiple of EllipsoidStructSize!");

            m_data = data.array();
        } else if (props.has_property("centers")) {
            const TensorXf32 &centers = props.get_any<TensorXf32>("centers");
            const TensorXf32 &scales  = props.get_any<TensorXf32>("scales");
            const TensorXf32 &quats   = props.get_any<TensorXf32>("quaternions");

            if (props.has_property("to_world"))
                Throw("\"to_world\" is only supported when loading PLY file!");
            if (centers.shape(1) != 3)
                Throw("TensorXf centers must have shape (N, 3)!");
            if (quats.shape(1) != 4)
                Throw("TensorXf quats must have shape (N, 4)!");
            if (scales.shape(1) != 3)
                Throw("TensorXf scales must have shape (N, 3)!");
            if (props.has_property("scale_factor"))
                Throw("\"scale_factor\" parameter is only supported with PLY files!");

            if (centers.shape(0) != quats.shape(0) || centers.shape(0) != scales.shape(0))
                Throw("TensorXf centers, quaternions and scales must have the same number of rows!");

            m_data = dr::zeros<FloatStorage>(centers.shape(0) * EllipsoidStructSize);
            UInt32Storage idx = dr::arange<UInt32Storage>(centers.shape(0));
            for (int i = 0; i < 3; i++)
                dr::scatter(m_data, dr::gather<FloatStorage>(centers.array(), idx * 3 + i), idx * EllipsoidStructSize + i, true, ReduceMode::NoConflicts);
            for (int i = 0; i < 3; i++)
                dr::scatter(m_data, dr::gather<FloatStorage>(scales.array(), idx * 3 + i), idx * EllipsoidStructSize + 3 + i, true, ReduceMode::NoConflicts);
            for (int i = 0; i < 4; i++)
                dr::scatter(m_data, dr::gather<FloatStorage>(quats.array(), idx * 4 + i), idx * EllipsoidStructSize + 6 + i, true, ReduceMode::NoConflicts);
            dr::eval(m_data);
        } else {
            Throw("Must specify either \"data\" or \"filename\" or \"centers\".");
        }

        m_data_pointer = m_data.data();

        m_extent_multiplier = props.get<ScalarFloat>("extent", 3.f);
        m_extent_adaptive_clamping = props.get<bool>("extent_adaptive_clamping", false);

        // Load any other ellipsoids attributes
        auto unqueried = props.unqueried();
        if (!unqueried.empty()) {
            for (auto &key : unqueried) {
                if (key == "shell")
                    continue;
                const TensorXf &tensor = props.get_any<TensorXf>(key);
                if (tensor.ndim() != 2)
                    Throw("Ellipsoids attribute \"%s\" must be a 2 dimensional tensor!", key);
                if (tensor.shape(0) != count())
                    Throw("Ellipsoids attribute \"%s\" must have the same number of entries as ellipsoids (%u vs %u)", key, tensor.shape(0), count());

                m_attributes.insert({ key , tensor.array() });
            }
        }

        if (m_extent_adaptive_clamping && (!has_attribute("opacities"))) {
            Log(Warn, "Ellipsoids must have attribute \"opacities\" to use adaptive clamping! Disabling adaptive clamping.");
            m_extent_adaptive_clamping = false;
        }

        compute_extents();
    }

    void traverse(TraversalCallback *cb) {
        cb->put("data", m_data, ParamFlags::Differentiable);
        for (auto it = m_attributes.begin(); it != m_attributes.end(); ++it)
            cb->put(it.key(), it.value(), ParamFlags::Differentiable);

        cb->put("extent",                   m_extent_multiplier,        ParamFlags::ReadOnly);
        cb->put("extent_adaptive_clamping", m_extent_adaptive_clamping, ParamFlags::ReadOnly);
    }

    void parameters_changed() {
        if constexpr (!dr::is_cuda_v<Float>)
            m_data_pointer = m_data.data();

        for (auto& [name, attr]: m_attributes) {
            if (dr::width(attr) % count() != 0)
                Throw("Attribute \"%s\" must have the same number of entries as ellipsoids (%u vs %u)", name, dr::width(attr), count());
        }

        compute_extents();
    }

    void compute_extents() {
        if (m_extent_adaptive_clamping) {
            auto indices = dr::arange<UInt32Storage>(count());
            FloatStorage opacities = dr::gather<FloatStorage>(m_attributes["opacities"], indices);
            float alpha = 0.01f; // minimum response of the Gaussian
            m_extents = dr::sqrt(2.f * dr::log(opacities / alpha)) * m_extent_multiplier / 3.f;
        } else {
            m_extents = dr::full<FloatStorage>(m_extent_multiplier, count());
        }

        m_extents_pointer = m_extents.data();
    }

    size_t count() const { return dr::width(m_data) / EllipsoidStructSize; }

    bool has_attribute(std::string_view name) const {
        if (m_attributes.find(name) != m_attributes.end())
            return true;
        if (name == "center" || name == "quaternion" || name == "scale")
            return true;
        return false;
    }

    Float eval_attribute_1(std::string_view name,
                           const SurfaceInteraction3f &si,
                           Mask active) const {
        const auto& it = m_attributes.find(name);
        if (it != m_attributes.end()) {
            const auto& attr = it->second;
            if (dr::width(attr) == count())
                return dr::gather<Float>(attr, si.prim_index, active);
        }

        if (name == "extent")
            return extents<Float>(si.prim_index, active);

        Throw("Unknown attribute %s!", name);
    }

    Color3f eval_attribute_3(std::string_view name,
                             const SurfaceInteraction3f &si,
                             Mask active) const {
        const auto& it = m_attributes.find(name);
        if (it != m_attributes.end()) {
            const auto& attr = it->second;
            if (dr::width(attr) / count() == 3)
                return dr::gather<Color3f>(attr, si.prim_index, active);
        }

        if (name == "center" || name == "scale") {
            auto ellipsoid = get_ellipsoid<Float>(si.prim_index, active);
            if (name == "center")
                return ellipsoid.center;
            else if(name == "scale")
                return ellipsoid.scale;
        }

        Throw("Unknown attribute %s!", name);
    }

    /// Helper meta-template function to call the templated Float::gather_packet_ with a runtime size
    template <int N = 256, typename Index, typename Mask>
    ArrayXf gather_packet_dynamic(FloatStorage attr, Index index, Mask active) const {
        if constexpr (N <= 1) {
            return ArrayXf();
        } else {
            uint32_t dim = (uint32_t) (dr::width(attr) / count());
            if (dim > 256)
                Throw("Over the maximum number of dimensions for attributes! %u vs 256", dim);
            if (N == dim) {
                auto tmp = Float::template gather_packet_<N>(attr, index, active, ReduceMode::Auto);
                ArrayXf res = dr::zeros<ArrayXf>((size_t) N);
                for (int i = 0; i < N; i++)
                    res.entry(i) = tmp[i];
                return res;
            } else {
                return gather_packet_dynamic<N - 1>(attr, index, active);
            }
        }
    }

    ArrayXf eval_attribute_x(std::string_view name,
                             const SurfaceInteraction3f &si,
                             Mask active) const {
        const auto& it = m_attributes.find(name);
        if (it != m_attributes.end()) {
            const auto& attr = it->second;
            if constexpr (!dr::is_jit_v<Float>) {
                uint32_t dim = (uint32_t) (dr::width(attr) / count());
                ArrayXf res = dr::zeros<ArrayXf>(dim);
                for (uint32_t i = 0; i < dim; i++)
                    res.entry(i) = dr::gather<Float>(attr, si.prim_index * dim + i, active);
                return res;
            } else {
                return gather_packet_dynamic(attr, si.prim_index, active);
            }
        }

        if (name == "ellipsoid" || name == "center" || name == "quaternion" || name == "scale") {
            auto ellipsoid = get_ellipsoid<Float>(si.prim_index, active);
            if (name == "ellipsoid") {
                ArrayXf res = dr::zeros<ArrayXf>(EllipsoidStructSize);
                for (int i = 0; i < 3; ++i)
                    res.entry(i) = ellipsoid.center[i];
                for (int i = 0; i < 3; ++i)
                    res.entry(i + 3) = ellipsoid.scale[i];
                for (int i = 0; i < 4; ++i)
                    res.entry(i + 6) = ellipsoid.quat[i];
                return res;
            } else if (name == "center") {
                return ArrayXf({ ellipsoid.center.x(), ellipsoid.center.y(), ellipsoid.center.z() });
            } else if(name == "quaterion") {
                return ArrayXf({ ellipsoid.quat.x(), ellipsoid.quat.y(), ellipsoid.quat.z(), ellipsoid.quat.w() });
            } else if(name == "scale") {
                return ArrayXf({ ellipsoid.scale.x(), ellipsoid.scale.y(), ellipsoid.scale.z() });
            }
        }

        Throw("Unknown attribute %s!", name);
    }

    /// Helper routine to extract the data for a given ellipsoid.
    template <typename Float_, typename Index_, typename Mask_ = dr::mask_t<Float>>
    Ellipsoid<Float_> get_ellipsoid(Index_ index, Mask_ active = true) const {
        Index_ idx = index * EllipsoidStructSize;
        Ellipsoid<Float_> ellipsoid;
        if constexpr (!dr::is_jit_v<Index_>) {
            DRJIT_MARK_USED(active);
            auto tmp = dr::load<dr::Array<float, EllipsoidStructSize>>(&m_data_pointer[idx]);
            ellipsoid.center = Point<Float_, 3>((Float_) tmp[0], (Float_) tmp[1], (Float_) tmp[2]);
            ellipsoid.scale  = Vector<Float_, 3>((Float_) tmp[3], (Float_) tmp[4], (Float_) tmp[5]);
            ellipsoid.quat   = dr::Quaternion<Float_>((Float_) tmp[6], (Float_) tmp[7], (Float_) tmp[8], (Float_) tmp[9]);
        } else {
            auto tmp = Float_::template gather_packet_<EllipsoidStructSize>(m_data, index, active, ReduceMode::Auto);
            ellipsoid.center = Point<Float_, 3>(tmp[0], tmp[1], tmp[2]);
            ellipsoid.scale  = Vector<Float_, 3>(tmp[3], tmp[4], tmp[5]);
            ellipsoid.quat   = dr::Quaternion<Float_>(tmp[6], tmp[7], tmp[8], tmp[9]);
        }
        return ellipsoid;
    }

    template <typename Float_, typename Index_, typename Mask_ = dr::mask_t<Index_>>
    Float_ extents(Index_ index, Mask_ active = true) const {
        DRJIT_MARK_USED(active);
        if constexpr (!dr::is_jit_v<Index_>)
            return (Float_) m_extents_pointer[index];
        else
            return dr::gather<Float_>(m_extents, index, active);
    }

    FloatStorage& extents_data() { return m_extents; }

    const FloatStorage& extents_data() const { return m_extents; }

    FloatStorage& data() { return m_data; }

    const FloatStorage& data() const { return m_data; }

    AttributesMap& attributes() { return m_attributes; }

    const AttributesMap& attributes() const { return m_attributes; }

private:
    /// The buffer for the ellipsoid data: centers, scales, and quaternions.
    FloatStorage m_data;

    /// The pointer to the ellipsoid data above (used in Embree's kernel)
    float *m_data_pointer;

    /// The extent of the ellipsoid support defined by its shell
    ScalarFloat m_extent_multiplier;
    bool m_extent_adaptive_clamping;
    FloatStorage m_extents;

    /// The pointer to the ellipsoid extents data above (used in Embree's kernel)
    float *m_extents_pointer;

    /// Arbitrary attributes for ellipsoids
    AttributesMap m_attributes;
};

NAMESPACE_END(mitsuba)


```

## Source file: src/render/microfacet.cpp

{
  "path": "src/render/microfacet.cpp",
  "blob_sha": "107af2057cfcb8d40c96001cd729f5b0ca9bb6a4",
  "sha256": "dd2a48e2e7392a2bd59ae6f7797e653f903bb1ff2ee39fa2e63ae288be4cd7b3",
  "lines": 7,
  "bytes": 135,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/mitsuba-renderer/mitsuba3/blob/fd9c9ceb38deb511a5c3f8479c8b14c0b552167e/src/render/microfacet.cpp",
  "function_signature_candidates": []
}

```text

#include <mitsuba/render/microfacet.h>

NAMESPACE_BEGIN(mitsuba)

MI_INSTANTIATE_CLASS(MicrofacetDistribution)

NAMESPACE_END(mitsuba)


```

## Source file: src/shapes/ellipsoids.cpp

{
  "path": "src/shapes/ellipsoids.cpp",
  "blob_sha": "2a4e0cdfdf188f2311d99ff43617e462a6578172",
  "sha256": "8ebf29fb8e3096270437228e3d5415c1747cfbe54ec1f46eee9e674870a8f2e7",
  "lines": 524,
  "bytes": 20269,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 6,
    "rig": 3,
    "face": 9,
    "lip": 98
  },
  "url": "https://github.com/mitsuba-renderer/mitsuba3/blob/fd9c9ceb38deb511a5c3f8479c8b14c0b552167e/src/shapes/ellipsoids.cpp",
  "function_signature_candidates": [
    {
      "line": 163,
      "text": "void traverse(TraversalCallback *cb) override {"
    },
    {
      "line": 167,
      "text": "void parameters_changed(const std::vector<std::string> &keys) override {"
    },
    {
      "line": 373,
      "text": "void describe(ShapeIR &g) const override {"
    },
    {
      "line": 398,
      "text": "void traverse_cb(void *payload, const dr::TraverseVisitor &cb) override {"
    },
    {
      "line": 416,
      "text": "void recompute_bbox() {"
    }
  ]
}

```text

#include <mitsuba/core/fwd.h>
#include <mitsuba/core/math.h>
#include <mitsuba/core/properties.h>
#include <mitsuba/core/transform.h>
#include <mitsuba/core/util.h>
#include <mitsuba/core/fstream.h>
#include <mitsuba/core/mstream.h>
#include <mitsuba/core/warp.h>
#include <mitsuba/core/fresolver.h>
#include <mitsuba/core/mmap.h>
#include <mitsuba/core/timer.h>
#include <mitsuba/render/fwd.h>
#include <mitsuba/render/interaction.h>
#include <mitsuba/render/shape.h>
#include <mitsuba/render/scene_ir.h>

#include "ellipsoids.h"

NAMESPACE_BEGIN(mitsuba)

/**!

.. _shape-ellipsoids:

Ellipsoids (:monosp:`ellipsoids`)
-------------------------------------------------

.. pluginparameters::

 * - filename
   - |string|
   - Specifies the PLY file containing the ellipsoid centers, scales, and quaternions.
     This parameter cannot be used if `data` or `centers` are provided.

 * - data
   - |tensor|
   - A tensor of shape (N, 10) or (N * 10) that defines the ellipsoid centers, scales, and quaternions.
     This parameter cannot be used if `filename` or `centers` are provided.

 * - centers
   - |tensor|
   - A tensor of shape (N, 3) specifying the ellipsoid centers.
     This parameter cannot be used if `filename` or `data` are provided.

 * - scales
   - |tensor|
   - A tensor of shape (N, 3) specifying the ellipsoid scales.
     This parameter cannot be used if `filename` or `data` are provided.

 * - quaternions
   - |tensor|
   - A tensor of shape (N, 3) specifying the ellipsoid quaternions.
     This parameter cannot be used if `filename` or `data` are provided.

 * - scale_factor
   - |float|
   - A scaling factor applied to all ellipsoids when loading from a PLY file. (Default: 1.0)

 * - extent
   - |float|
   - Specifies the extent of the ellipsoid. This effectively acts as an
     extra scaling factor on the ellipsoid, without having to alter the scale
     parameters. (Default: 3.0)
   - |readonly|

 * - extent_adaptive_clamping
   - |float|
   - If True, use adaptive extent values based on the `opacities` attribute of
     the volumetric primitives. (Default: False)
   - |readonly|

 * - to_world
   - |transform|
   - Specifies an optional linear object-to-world transformation to apply to
     all ellipsoids.
   - |exposed|, |differentiable|, |discontinuous|

 * - (Nested plugin)
   - |tensor|
   - Specifies arbitrary ellipsoids attribute as a tensor of shape (N, D) with D
     the dimensionality of the attribute. For instance this can be used to define
     an opacity value for each ellipsoids, or a set of spherical harmonic coefficients
     as used in the :monosp:`volprim_rf_basic` integrator.

This shape plugin defines a point cloud of anisotropic ellipsoid primitives
using specified centers, scales, and quaternions. It employs a closed-form
ray-intersection formula with backface culling. Although it is slower than the
:monosp:`ellipsoidsmesh` shape plugin, which uses tessellated ellipsoids for
ray-triangle intersection hardware acceleration, it offers greater flexibility
in computing intersections.

This shape also exposes an `extent` parameter, it acts as an extra scaling
factor for the ellipsoids' scales. Typically, this is used to define the support
of a kernel function defined within the ellipsoid. For example, the
`scale` parmaters of the ellipsoid will define the variances of a gaussian and
the `extent` will multiple those value to define the effictive radius of the
ellipsoid. When `extent_adaptive_clamping` is enabled, the extent is
additionally multiplied by an opacity-dependent factor::math:`\sqrt{2 * \log(opacity / 0.01)} / 3`

It is designed for use with volumetric primitive integrators, as detailed in
:cite:`Condor2024Gaussians`.

.. tabs::
    .. code-tab:: xml
        :name: ellipsoids-sphere

        <shape type="ellipsoids">
            <string name="filename" value="my_primitives.ply"/>
        </shape>

    .. code-tab:: python

        'primitives': {
            'type': 'ellipsoids',
            'filename': 'my_primitives.ply'
        }
 */

template <typename Float, typename Spectrum>
class Ellipsoids final : public Shape<Float, Spectrum> {
public:
    MI_IMPORT_BASE(Shape, m_to_world, m_is_instance, m_shape_type, initialize,
                   mark_dirty, get_children_string)
    MI_IMPORT_TYPES()

    using typename Base::ScalarSize;
    using typename Base::ScalarIndex;
    using FloatStorage  = DynamicBuffer<dr::replace_scalar_t<Float, float>>;
    using BoolStorage   = DynamicBuffer<Mask>;
    using UInt32Storage = DynamicBuffer<UInt32>;
    using ArrayXf       = dr::DynamicArray<Float>;

    /// Box layout of the GPU acceleration structure builders
    struct PackedBoundingBox { float min[3], max[3]; };

    static constexpr bool IsGPU = dr::is_cuda_v<Float> || dr::is_metal_v<Float>;
    using BoundingBoxType =
        std::conditional_t<IsGPU, PackedBoundingBox, ScalarBoundingBox3f>;

    Ellipsoids(const Properties &props) : Base(props) {
        m_shape_type = ShapeType::Ellipsoids;

        Timer timer;

        m_ellipsoids = EllipsoidsData<Float, Spectrum>(props);

        size_t ellipsoids_count_bytes = EllipsoidStructSize * sizeof(float);
        Log(Debug, "Read %i ellipsoids (%s in %s)",
            m_ellipsoids.count(),
            util::mem_string(m_ellipsoids.count() * ellipsoids_count_bytes),
            util::time_string((float) timer.value())
        );

        recompute_bbox();
        initialize();
    }

    ~Ellipsoids() {
        jit_free(m_device_bboxes);
        jit_free(m_host_bboxes);
    }

    void traverse(TraversalCallback *cb) override {
        m_ellipsoids.traverse(cb);
    }

    void parameters_changed(const std::vector<std::string> &keys) override {
        m_ellipsoids.parameters_changed();

        if (keys.empty() || string::contains(keys, "data")) {
            recompute_bbox();
            mark_dirty();
        }

        Base::parameters_changed(keys);
    }

    ScalarSize primitive_count() const override { return (ScalarSize) m_ellipsoids.count(); }

    ScalarBoundingBox3f bbox() const override { return m_bbox; }

    ScalarBoundingBox3f bbox(ScalarIndex prim_index) const override {
        if constexpr (IsGPU)
            Throw("bbox(ScalarIndex) is not available on the GPU backends!");
        Assert(prim_index <= primitive_count());
        auto bbox = ((BoundingBoxType*) m_host_bboxes)[prim_index];

        return ScalarBoundingBox3f(
            ScalarPoint3f(bbox.min[0], bbox.min[1], bbox.min[2]),
            ScalarPoint3f(bbox.max[0], bbox.max[1], bbox.max[2])
        );
    }

    // =============================================================

    // =============================================================
    // Sampling routines (not implemented!)
    // =============================================================

    PositionSample3f sample_position(Float, const Point2f &, Mask) const override { return dr::zeros<PositionSample3f>(); }

    Float pdf_position(const PositionSample3f &, Mask) const override { return 0; }

    DirectionSample3f sample_direction(const Interaction3f &, const Point2f &, Mask) const override { return dr::zeros<DirectionSample3f>(); }

    Float pdf_direction(const Interaction3f &, const DirectionSample3f &, Mask) const override { return 0; }

    SurfaceInteraction3f eval_parameterization(const Point2f &, uint32_t, Mask) const override { return dr::zeros<SurfaceInteraction3f>(); }

    // =============================================================

    // =============================================================
    // Attribute routines
    // =============================================================

    Mask has_attribute(std::string_view name, Mask active) const override {
        if (m_ellipsoids.has_attribute(name))
            return true;

        return Base::has_attribute(name, active);
    }

    Float eval_attribute_1(std::string_view name,
                           const SurfaceInteraction3f &si,
                           Mask active) const override {
        MI_MASK_ARGUMENT(active);
        try {
            return m_ellipsoids.eval_attribute_1(name, si, active);
        } catch (...) {
            return Base::eval_attribute_1(name, si, active);
        }
    }

    Color3f eval_attribute_3(std::string_view name,
                             const SurfaceInteraction3f &si,
                             Mask active) const override {
        MI_MASK_ARGUMENT(active);
        try {
            return m_ellipsoids.eval_attribute_3(name, si, active);
        } catch (...) {
            return Base::eval_attribute_3(name, si, active);
        }
    }

    ArrayXf eval_attribute_x(std::string_view name,
                             const SurfaceInteraction3f &si,
                             Mask active) const override {
        MI_MASK_ARGUMENT(active);
        try {
            return m_ellipsoids.eval_attribute_x(name, si, active);
        } catch (...) {
            return Base::eval_attribute_x(name, si, active);
        }
    }

    // =============================================================

    // =============================================================
    // Ray tracing routines
    // =============================================================

    template <typename FloatP, typename Ray3fP, typename Ellipsoid>
    std::pair<FloatP, dr::mask_t<FloatP>>
    ray_ellipsoid_intersection(const Ray3fP &ray,
                               Ellipsoid ellipsoid,
                               dr::mask_t<FloatP> active) const {
        using Value = dr::float32_array_t<FloatP>;
        using Value3 = Vector<Value, 3>;

        auto rot = dr::quat_to_matrix<dr::Matrix<Value, 3>>(ellipsoid.quat);

        // Transform space such that the ellipsoid is now a unit sphere centered
        // at the origin
        Value3 l = transpose(rot) * (ray.o - ellipsoid.center);
        Value3 d = transpose(rot) * ray.d;

        Value3 scale_rcp = dr::rcp(ellipsoid.scale);
        l *= scale_rcp;
        d *= scale_rcp;

        // Shift the ray origin to the point closest to the center (see sphere.cpp)
        Value A = dr::squared_norm(d),
              t_offset = -dr::dot(l, d) / A;
        Value3 o = dr::fmadd(d, t_offset, l);

        Value B = dr::scalar_t<Value>(2) * dr::dot(o, d),
              C = dr::squared_norm(o) - dr::scalar_t<Value>(1);

        auto [solution_found, near_t, far_t] = math::solve_quadratic(A, B, C);

        // Undo the origin shift
        near_t += t_offset;

        // Only the nearest root counts: a ray that starts inside the
        // ellipsoid sees its backface, which is culled
        Value maxt = Value(ray.maxt);
        active &= dr::mask_t<FloatP>(solution_found && near_t >= Value(0) &&
                                     near_t <= maxt);
        FloatP t = FloatP(near_t);

        return { dr::select(active, t, dr::Infinity<FloatP>), active };
    }

    template <typename FloatP, typename Ray3fP>
    std::tuple<dr::mask_t<FloatP>, FloatP, Point<FloatP, 2>,
               dr::uint32_array_t<FloatP>, dr::uint32_array_t<FloatP>>
    ray_intersect_preliminary_impl(const Ray3fP &ray,
                                   dr::uint32_array_t<FloatP> prim_index,
                                   dr::mask_t<FloatP> active) const {
        MI_MASK_ARGUMENT(active);
        using Value = dr::float32_array_t<FloatP>;
        auto ellipsoid = m_ellipsoids.template get_ellipsoid<Value>(prim_index, active);
        ellipsoid.scale *= m_ellipsoids.template extents<Value>(prim_index, active);
        auto [t, valid] = ray_ellipsoid_intersection<FloatP, Ray3fP>(ray, ellipsoid, active);
        return { valid, t, dr::zeros<Point<FloatP, 2>>(), ((uint32_t) -1), prim_index };
    }

    template <typename FloatP, typename Ray3fP>
    dr::mask_t<FloatP> ray_test_impl(const Ray3fP &ray,
                                     dr::uint32_array_t<FloatP> prim_index,
                                     dr::mask_t<FloatP> active) const {
        MI_MASK_ARGUMENT(active);
        return std::get<0>(ray_intersect_preliminary_impl<FloatP>(ray, prim_index, active));
    }

    MI_SHAPE_DEFINE_RAY_INTERSECT_METHODS()

    SurfaceInteraction3f compute_surface_interaction(const Ray3f &ray,
                                                     const PreliminaryIntersection3f &pi,
                                                     uint32_t ray_flags,
                                                     Mask active) const override {
        MI_MASK_ARGUMENT(active);

        bool detach_shape = has_flag(ray_flags, RayFlags::DetachShape);
        // If necessary, temporally suspend gradient tracking for all shape
        // parameters to construct a surface interaction completely detach from
        // the shape.
        dr::suspend_grad<Float> scope(detach_shape, m_ellipsoids.data());
        SurfaceInteraction3f si = dr::zeros<SurfaceInteraction3f>();

        auto ellipsoid = m_ellipsoids.template get_ellipsoid<Float>(pi.prim_index, active);
        ellipsoid.scale *= m_ellipsoids.template extents<Float>(pi.prim_index, active);
        auto rot = dr::quat_to_matrix<Matrix3f>(ellipsoid.quat);

        // The local coordinates are static as the ellipsoid moves
        si.t = pi.t;
        si.p = dr::detach(ray(pi.t));
        Point3f local_d =
            dr::detach(dr::transpose(rot) * (si.p - ellipsoid.center));

        si.n = dr::detach(
            dr::normalize(rot * (local_d / dr::square(ellipsoid.scale))));
        si.p_err = ray_error(ray, pi.t, si.n);

        Point3f p_att = ellipsoid.center +
                        rot * (ellipsoid.scale *
                               (local_d / dr::detach(ellipsoid.scale)));

        si.attach_motion(ray, p_att, ray_flags);

        Point3f local = dr::transpose(rot) * (si.p - ellipsoid.center);
        si.sh_frame.n = dr::normalize(rot * (local / dr::square(ellipsoid.scale)));

        si.n = si.sh_frame.n;

        si.prim_index = pi.prim_index;
        si.shape      = this;
        si.instance_index = 0;

        return si;
    }

    void describe(ShapeIR &g) const override {
        Base::describe(g);
        g.aabb_buffer = m_device_bboxes;
    }

    // =============================================================

    std::string to_string() const override {
        std::ostringstream oss;
        oss << "Ellipsoids[" << std::endl
            << "  bbox = " << string::indent(m_bbox) << "," << std::endl
            << "  ellipsoid_count = " << primitive_count() << "," << std::endl
            << "  " << string::indent(get_children_string()) << std::endl;

        if (m_ellipsoids.attributes().size() > 0) {
                oss << "  Ellipsoid attributes = {";
            for (auto& [name, attr]: m_ellipsoids.attributes())
                oss << " " << name << "[" << attr << "],";
            oss << "  }," << std::endl;
        }

        oss << "]";
        return oss.str();
    }

    void traverse_cb(void *payload, const dr::TraverseVisitor &cb) override {
        if (cb.role != dr::TraverseRole::Freeze)
            return;

        Base::traverse_cb(payload, cb);
        dr::traverse_fn(m_ellipsoids.data(), payload, cb, "m_ellipsoids");
        dr::traverse_fn(m_ellipsoids.extents_data(), payload, cb, "m_ellipsoids_extents");
        auto &attr_map = m_ellipsoids.attributes();
        for (auto it = attr_map.begin(); it != attr_map.end(); ++it) {
            dr::traverse_fn(it.value(), payload, cb, "m_ellipsoids_attributes");
        }
    }

    MI_DECLARE_CLASS(Ellipsoids)

private:

    /// Helper routine to recompute the bounding boxes of all ellipsoids.
    void recompute_bbox() {
        Timer timer;
        size_t ellipsoid_count = primitive_count();

        Log(Debug, "Recomputing bounding boxes for \"%s\" ellipsoids", ellipsoid_count);

        if constexpr (dr::is_jit_v<Float>) {
            UInt32 idx = dr::arange<UInt32>(m_ellipsoids.count());
            auto ellipsoid = m_ellipsoids.template get_ellipsoid<Float>(idx);
            ellipsoid.scale *= m_ellipsoids.extents_data();
            auto rot = dr::quat_to_matrix<Matrix3f>(ellipsoid.quat);

            Vector3f delta = Vector3f(
                dr::norm(rot[0] * ellipsoid.scale),
                dr::norm(rot[1] * ellipsoid.scale),
                dr::norm(rot[2] * ellipsoid.scale)
            );

            BoundingBox3f bbox(ellipsoid.center - delta, ellipsoid.center + delta);

            uint32_t size = (uint32_t) sizeof(BoundingBoxType) / sizeof(ScalarFloat);
            uint32_t stride = (uint32_t) offsetof(BoundingBoxType, max) / sizeof(ScalarFloat);

            Float data = dr::empty<Float>(ellipsoid_count * size);
            for (int i = 0; i < 3; i++) {
                dr::scatter(data, bbox.min[i], idx * size + i, true, ReduceMode::NoConflicts);
                dr::scatter(data, bbox.max[i], idx * size + i + stride, true, ReduceMode::NoConflicts);
            }

            if constexpr (IsGPU) {
                constexpr JitBackend backend = dr::backend_v<Float>;
                jit_free(m_device_bboxes);
                m_device_bboxes = jit_malloc(backend, sizeof(BoundingBoxType) * ellipsoid_count);
                jit_memcpy(backend, m_device_bboxes, data.data(),
                           sizeof(BoundingBoxType) * ellipsoid_count);
            }

            if constexpr (dr::is_llvm_v<Float>) {
                jit_free(m_host_bboxes);
                m_host_bboxes = jit_malloc(JitBackend::None, sizeof(BoundingBoxType) * ellipsoid_count);
                jit_memcpy(JitBackend::LLVM, m_host_bboxes, data.data(),
                           sizeof(BoundingBoxType) * ellipsoid_count);
            }

            m_bbox = ScalarBoundingBox3f(
                ScalarVector3f(
                    dr::slice<ScalarFloat>(dr::min(bbox.min[0])),
                    dr::slice<ScalarFloat>(dr::min(bbox.min[1])),
                    dr::slice<ScalarFloat>(dr::min(bbox.min[2]))
                ),
                ScalarVector3f(
                    dr::slice<ScalarFloat>(dr::max(bbox.max[0])),
                    dr::slice<ScalarFloat>(dr::max(bbox.max[1])),
                    dr::slice<ScalarFloat>(dr::max(bbox.max[2]))
                )
            );
        } else {
            const float *ptr = m_ellipsoids.data().data();
            const float *ptr_extents = m_ellipsoids.extents_data().data();

            BoundingBoxType *host_bboxes = (BoundingBoxType *) jit_malloc(
                JitBackend::None, sizeof(BoundingBoxType) * ellipsoid_count);

            m_bbox.reset();

            for (size_t i = 0; i < ellipsoid_count; ++i) {
                size_t idx = i * EllipsoidStructSize;
                ScalarPoint3f center(ptr[idx + 0], ptr[idx + 1], ptr[idx + 2]);
                ScalarVector3f scale(ptr[idx + 3], ptr[idx + 4], ptr[idx + 5]);
                ScalarQuaternion4f quat(ptr[idx + 6], ptr[idx + 7], ptr[idx + 8], ptr[idx + 9]);
                scale *= ptr_extents[i];
                auto rot = dr::quat_to_matrix<ScalarMatrix3f>(quat);

                ScalarVector3f delta = ScalarVector3f(
                    dr::norm(rot[0] * scale),
                    dr::norm(rot[1] * scale),
                    dr::norm(rot[2] * scale)
                );
                auto prim_bbox = ScalarBoundingBox3f(center - delta, center + delta);

                // Append the ellipsoid bounding box to the list
                host_bboxes[i] = BoundingBoxType(prim_bbox);

                // Expand the shape's bounding box
                m_bbox.expand(prim_bbox);
            }

            jit_free(m_host_bboxes);
            m_host_bboxes = host_bboxes;
        }

        Log(Debug, "Finished recomputing bounding boxes in %s", util::time_string((float) timer.value()));
    }

private:
    /// Object holding the ellipsoids data and attributes
    EllipsoidsData<Float, Spectrum> m_ellipsoids;

    /// The bounding box of the overall shape
    ScalarBoundingBox3f m_bbox;

    /// Per-ellipsoid boxes on the host (CPU backends) or the device (GPU
    /// backends, see describe())
    void *m_host_bboxes   = nullptr;
    void *m_device_bboxes = nullptr;
};

MI_EXPORT_PLUGIN(Ellipsoids)
NAMESPACE_END(mitsuba)


```

## Source file: src/shapes/ellipsoidsmesh.cpp

{
  "path": "src/shapes/ellipsoidsmesh.cpp",
  "blob_sha": "0e2fbcac2cf3eb06ec1570c8792543d8638f960c",
  "sha256": "b0d98ea342c036e0c0b2dc0acff560b199ea79439b792749af8e34d4ccd642c4",
  "lines": 567,
  "bytes": 24789,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 3,
    "rig": 1,
    "face": 61,
    "lip": 65
  },
  "url": "https://github.com/mitsuba-renderer/mitsuba3/blob/fd9c9ceb38deb511a5c3f8479c8b14c0b552167e/src/shapes/ellipsoidsmesh.cpp",
  "function_signature_candidates": [
    {
      "line": 237,
      "text": "void traverse(TraversalCallback *cb) override {"
    },
    {
      "line": 241,
      "text": "void parameters_changed(const std::vector<std::string> &keys) override {"
    },
    {
      "line": 327,
      "text": "void traverse_cb(void *payload, const dr::TraverseVisitor &cb) override {"
    },
    {
      "line": 343,
      "text": "void recompute_mesh() {"
    },
    {
      "line": 467,
      "text": "void describe(ShapeIR &g) const override {"
    }
  ]
}

```text

#include <mitsuba/core/fwd.h>
#include <mitsuba/core/fresolver.h>
#include <mitsuba/core/properties.h>
#include <mitsuba/core/util.h>
#include <mitsuba/core/timer.h>
#include <mitsuba/render/mesh.h>
#include <mitsuba/render/scene_ir.h>

#include "ellipsoids.h"

#if defined(MI_ENABLE_EMBREE)
    #include <embree3/rtcore.h>
#endif

NAMESPACE_BEGIN(mitsuba)

/// Forward declaration of template shell mesh data buffers
extern std::vector<dr::Array<float, 3>>    box_vertices, uv_sphere_72_vertices, ico_sphere_20_vertices;
extern std::vector<dr::Array<uint32_t, 3>> box_faces,    uv_sphere_72_faces,    ico_sphere_20_faces;

/**!

.. _shape-EllipsoidsMesh:

Mesh ellipsoids (:monosp:`ellipsoidsmesh`)
------------------------------------------

.. pluginparameters::

 * - filename
   - |string|
   - Specifies the PLY file containing the ellipsoid centers, scales, and quaternions.
     This parameter cannot be used if `data` or `centers` are provided.

 * - data
   - |tensor|
   - A tensor of shape (N, 10) or (N * 10) that defines the ellipsoid centers, scales, and quaternions.
     This parameter cannot be used if `filename` or `centers` are provided.

 * - centers
   - |tensor|
   - A tensor of shape (N, 3) specifying the ellipsoid centers.
     This parameter cannot be used if `filename` or `data` are provided.

 * - scales
   - |tensor|
   - A tensor of shape (N, 3) specifying the ellipsoid scales.
     This parameter cannot be used if `filename` or `data` are provided.

 * - quaternions
   - |tensor|
   - A tensor of shape (N, 3) specifying the ellipsoid quaternions.
     This parameter cannot be used if `filename` or `data` are provided.

 * - scale_factor
   - |float|
   - A scaling factor applied to all ellipsoids when loading from a PLY file. (Default: 1.0)

 * - extent
   - |float|
   - Specifies the extent of the ellipsoid. This effectively acts as an
     extra scaling factor on the ellipsoid, without having to alter the scale
     parameters. (Default: 3.0)
   - |readonly|

 * - extent_adaptive_clamping
   - |float|
   - If True, use adaptive extent values based on the `opacities` attribute of the volumetric primitives. (Default: False)
   - |readonly|

 * - shell
   - |string| or |mesh|
   - Specifies the shell type. Could be one of :monosp:`box`, :monosp:`ico_sphere`,
     or :monosp:`uv_sphere`, as well as a custom child mesh object. (Default: ``ico_sphere``)

 * - to_world
   - |transform|
   - Specifies an optional linear object-to-world transformation to apply to all ellipsoids.
   - |exposed|, |differentiable|, |discontinuous|

 * - (Nested plugin)
   - |tensor|
   - Specifies arbitrary ellipsoids attribute as a tensor of shape (N, D) with D
     the dimensionality of the attribute. For instance this can be used to define
     an opacity value for each ellipsoids, or a set of spherical harmonic coefficients
     as used in the :monosp:`volprim_rf_basic` integrator.

This shape plugin defines a point cloud of anisotropic ellipsoid primitives
given centers, scales, and quaternions, using a mesh-based representation with
backface culling. This plugin is designed to leverage hardware acceleration for
ray-triangle intersections, providing a performance advantage over analytical
ellipsoid representations.

This shape also exposes an `extent` parameter, it acts as an extra scaling
factor for the ellipsoids' scales. Typically, this is used to define the support
of a kernel function defined within the ellipsoid. For example, the
`scale` parmaters of the ellipsoid will define the variances of a gaussian and
the `extent` will multiple those value to define the effictive radius of the
ellipsoid. When `extent_adaptive_clamping` is enabled, the extent is
additionally multiplied by an opacity-dependent factor::math:`\sqrt{2 * \log(opacity / 0.01)} / 3`

This shape is designed for use with volumetric primitive integrators, as
detailed in :cite:`Condor2024Gaussians`.

.. tabs::
    .. code-tab:: xml
        :name: ellipsoidsmesh-sphere

        <shape type="ellipsoidsmesh">
            <string name="filename" value="my_primitives.ply"/>
        </shape>

    .. code-tab:: python

        'primitives': {
            'type': 'ellipsoidsmesh',
            'filename': 'my_primitives.ply'
        }
 */

template <typename Float, typename Spectrum>
class EllipsoidsMesh final : public Mesh<Float, Spectrum> {
public:
    MI_IMPORT_BASE(Mesh, m_vertex_count, m_face_count, m_packed_faces, mark_dirty,
                   m_packed_vertices, m_position_count, m_normal_count, m_bbox,
                   recompute_bbox, get_children_string, m_shape_type)
    MI_IMPORT_TYPES(Shape)

    using typename Base::ScalarSize;
    using typename Base::ScalarIndex;
    using typename Base::InputFloat;
    using typename Base::InputPoint3f;
    using StoredFloat = DynamicBuffer<dr::replace_scalar_t<Float, InputFloat>>;
    using IndexStorage = DynamicBuffer<dr::replace_scalar_t<Float, ScalarIndex>>;
    using ArrayXf      = dr::DynamicArray<Float>;

#if defined(MI_ENABLE_LLVM) && !defined(MI_ENABLE_EMBREE)
    using Base::m_packed_vertices_ptr;
    using Base::m_packed_faces_ptr;
#endif

    EllipsoidsMesh(const Properties &props) : Base(props) {
        m_shape_type = ShapeType::EllipsoidsMesh;

        Timer timer;

        m_ellipsoids = EllipsoidsData<Float, Spectrum>(props);

        std::string_view shell_type = "default";
        if (props.has_property("shell")) {
            if (props.type("shell") == Properties::Type::String) {
                shell_type = props.get<std::string_view>("shell");
                if (shell_type != "box" && shell_type != "default" && shell_type != "ico_sphere" && shell_type != "uv_sphere")
                    Throw("Shell type '%s' is not supported. Should be one of: [\"default\", \"box\", \"ico_sphere\", \"uv_sphere\"]", shell_type);
            } else {
                shell_type = "mesh";
            }
        }

        if (shell_type == "box") {
            Log(Debug, "Load box shell template (12 triangles)");
            m_shell_vertices = box_vertices;
            m_shell_faces    = box_faces;
        } else if (shell_type == "default" || shell_type == "ico_sphere") {
            Log(Debug, "Load default ICO sphere shell template (20 triangles)");
            m_shell_vertices = ico_sphere_20_vertices;
            m_shell_faces    = ico_sphere_20_faces;
        } else if (shell_type == "uv_sphere") {
            Log(Debug, "Load default UV sphere shell template (72 triangles)");
            m_shell_vertices = uv_sphere_72_vertices;
            m_shell_faces    = uv_sphere_72_faces;
        } else {
            Log(Debug, "Load shell template from nested mesh object.");
            ref<Base> mesh(dynamic_cast<Base *>(props.get<ref<Object>>("shell").get()));

            // Consume the shell through the geometric topology
            auto geometric_faces = mesh->geometric_faces();

            auto&& pos = dr::migrate(mesh->positions().array(),
                                     JitBackend::None);
            auto&& faces = dr::migrate(geometric_faces.array(),
                                       JitBackend::None);

            if constexpr (dr::is_jit_v<Float>)
                dr::sync_thread();

            const ScalarIndex *faces_ptr = faces.data();
            const InputFloat *pos_ptr = pos.data();

            m_shell_vertices.clear();
            for (size_t i = 0; i < dr::width(pos) / 3; ++i)
                m_shell_vertices.push_back({ pos_ptr[3 * i], pos_ptr[3 * i + 1],
                                             pos_ptr[3 * i + 2] });

            m_shell_faces.clear();
            for (size_t i = 0; i < dr::width(faces) / 3; ++i) {
                m_shell_faces.push_back({
                    faces_ptr[3 * i + 0],
                    faces_ptr[3 * i + 1],
                    faces_ptr[3 * i + 2],
                });
            }
        }

        // Scale vertex positions of shell to ensure that it full encapsulates the unit sphere
        {
            InputFloat scaling = dr::Largest<InputFloat>;
            for (size_t i = 0; i < m_shell_faces.size(); ++i) {
                InputPoint3f v0 = m_shell_vertices[m_shell_faces[i][0]];
                InputPoint3f v1 = m_shell_vertices[m_shell_faces[i][1]];
                InputPoint3f v2 = m_shell_vertices[m_shell_faces[i][2]];

                // Ensures that all vertices are outside of unit sphere
                scaling = std::min(scaling, dr::norm(v0));
                scaling = std::min(scaling, dr::norm(v1));
                scaling = std::min(scaling, dr::norm(v2));

                // Ensures that face center is outside of unit sphere
                scaling = std::min(scaling, dr::norm((v0 + v1 + v2) / 3.f));
            }

            for (size_t i = 0; i < m_shell_vertices.size(); ++i)
                m_shell_vertices[i] /= scaling;
        }

        Log(Debug, "Template mesh shell contains %zu faces and %zu vertices", m_shell_faces.size(), m_shell_vertices.size());

        recompute_mesh();

        ScalarSize shell_count_bytes = 15 * sizeof(InputFloat) + 24 * sizeof(InputFloat) + 12 * sizeof(ScalarIndex);
        Log(Debug, "Initialize %i mesh ellipsoid shells (%s in %s)",
            m_ellipsoids.count(),
            util::mem_string(m_ellipsoids.count() * shell_count_bytes),
            util::time_string((float) timer.value()));
    }

    void traverse(TraversalCallback *cb) override {
        m_ellipsoids.traverse(cb);
    }

    void parameters_changed(const std::vector<std::string> &keys) override {
        m_ellipsoids.parameters_changed();

        if (keys.empty() || string::contains(keys, "data")) {
            recompute_mesh();
        }

        // Don't call Mesh::parameters_changed() as it will initialize
        // data-structures that are not needed for this plugin!
    }

    Mask has_attribute(std::string_view name, Mask active) const override {
        if (m_ellipsoids.has_attribute(name))
            return true;
        return Base::has_attribute(name, active);
    }

    Float eval_attribute_1(std::string_view name,
                           const SurfaceInteraction3f &si,
                           Mask active) const override {
        MI_MASK_ARGUMENT(active);
        try {
            return m_ellipsoids.eval_attribute_1(name, si, active);
        } catch (...) {
            return Base::eval_attribute_1(name, si, active);
        }
    }

    Color3f eval_attribute_3(std::string_view name,
                             const SurfaceInteraction3f &si,
                             Mask active) const override {
        MI_MASK_ARGUMENT(active);
        try {
            return m_ellipsoids.eval_attribute_3(name, si, active);
        } catch (...) {
            return Base::eval_attribute_3(name, si, active);
        }
    }

    ArrayXf eval_attribute_x(std::string_view name,
                             const SurfaceInteraction3f &si,
                             Mask active) const override {
        MI_MASK_ARGUMENT(active);
        try {
            return m_ellipsoids.eval_attribute_x(name, si, active);
        } catch (...) {
            return Base::eval_attribute_x(name, si, active);
        }
    }

    SurfaceInteraction3f compute_surface_interaction(const Ray3f &ray,
                                                     const PreliminaryIntersection3f &pi,
                                                     uint32_t ray_flags,
                                                     Mask active) const override {
        auto si = Base::compute_surface_interaction(ray, pi, ray_flags, active);

        // Divide by the number of faces per Gaussians
        si.prim_index /= (uint32_t) m_shell_faces.size();

        // Embree (custom filter) and OptiX (TrianglesCulled) cull backfaces so
        // the volprim integrator sees one entry hit per primitive. Metal reports
        // both faces, which would double-count (~2x brightness), so reject here.
        if constexpr (dr::is_metal_v<Float>)
            this->cull_backface(si, ray, active);

        return si;
    }

    std::string to_string() const override {
        std::ostringstream oss;
        oss << "EllipsoidsMesh[" << std::endl
            << "  bbox = " << string::indent(m_bbox) << "," << std::endl
            << "  ellipsoid_count = " << m_ellipsoids.count() << "," << std::endl
            << "  " << string::indent(get_children_string()) << std::endl;

        if (m_ellipsoids.attributes().size() > 0) {
                oss << "  Ellipsoid attributes = {";
            for (auto& [name, attr]: m_ellipsoids.attributes())
                oss << " " << name << "[" << attr << "],";
            oss << "  }," << std::endl;
        }

        oss << "]";
        return oss.str();
    }

    void traverse_cb(void *payload, const dr::TraverseVisitor &cb) override {
        if (cb.role != dr::TraverseRole::Freeze)
            return;

        Base::traverse_cb(payload, cb);
        dr::traverse_fn(m_ellipsoids.data(), payload, cb, "m_ellipsoids");
        dr::traverse_fn(m_ellipsoids.extents_data(), payload, cb, "m_ellipsoids_extents");
        auto &attr_map = m_ellipsoids.attributes();
        for (auto it = attr_map.begin(); it != attr_map.end(); ++it) {
            dr::traverse_fn(it.value(), payload, cb, "m_ellipsoids_attributes");
        }
    }

    MI_DECLARE_CLASS(EllipsoidsMesh)

private:
    void recompute_mesh() {
        if constexpr (dr::is_jit_v<Float>) {
            UInt32 idx = dr::arange<UInt32>(m_ellipsoids.count());

            auto ellipsoid = m_ellipsoids.template get_ellipsoid<Float>(idx);
            auto rot = dr::quat_to_matrix<Matrix3f>(ellipsoid.quat);

            const AffineTransform4f& to_world = AffineTransform4f::translate(ellipsoid.center) *
                                          AffineTransform4f(rot) *
                                          AffineTransform4f::scale(ellipsoid.scale) *
                                          AffineTransform4f::scale(m_ellipsoids.template extents<Float>(idx));

            int nb_vertices = (int) m_shell_vertices.size();
            int nb_faces    = (int) m_shell_faces.size();

            uint32_t vertex_count = (uint32_t) (m_ellipsoids.count() * nb_vertices);
            uint32_t face_count   = (uint32_t) (m_ellipsoids.count() * nb_faces);

            bool initialized = (vertex_count != m_vertex_count) || (face_count != m_face_count);

            m_vertex_count = vertex_count;
            m_face_count   = face_count;
            // The shells reference their vertices directly, without maps
            m_position_count = m_normal_count = vertex_count;

            // The shell vertices carry positions only: the remaining lanes
            // of the packed records stay zero, and this plugin never runs
            // Mesh::pack().
            m_packed_vertices = dr::zeros<StoredFloat>(
                (size_t) MeshVertexStride * m_vertex_count);
            m_packed_faces    = dr::empty<IndexStorage>(4 * m_face_count);

            for (int i = 0; i < nb_vertices; ++i) {
                Point3f v = to_world * Point3f(m_shell_vertices[i]);
                // Convert to 32-bit precision
                using JitInputPoint3f = Point<dr::replace_scalar_t<Float, InputFloat>, 3>;
                JitInputPoint3f v32(v);
                UInt32 base = (idx * uint32_t(nb_vertices) + uint32_t(i)) *
                              MeshVertexStride;
                for (uint32_t k = 0; k < 3; ++k)
                    dr::scatter(m_packed_vertices, v32[k], base + k, true,
                                ReduceMode::NoConflicts);
            }

            UInt32 offset = idx * uint32_t(nb_vertices);
            for (int i = 0; i < nb_faces; ++i) {
                Vector3u face = m_shell_faces[i];
                // 4-word face records; the fourth word is derived below
                Vector4u rec(face[0] + offset, face[1] + offset,
                             face[2] + offset, 0u);
                dr::scatter(m_packed_faces, rec, idx * nb_faces + i, true,
                            ReduceMode::NoConflicts);
            }

            // This plugin writes the packed records directly and skips
            // Mesh::refresh(), which would build data structures that it
            // does not need. The field views now describe stale records
            // and rebuild on demand.
            this->update_face_state();
            this->drop_views();
#if defined(MI_ENABLE_LLVM) && !defined(MI_ENABLE_EMBREE)
            m_packed_vertices_ptr = m_packed_vertices.data();
            m_packed_faces_ptr = m_packed_faces.data();
#endif
            if (initialized)
                recompute_bbox();
            mark_dirty();
        } else {
            size_t nb_vertices = m_shell_vertices.size();
            size_t nb_faces    = m_shell_faces.size();

            uint32_t vertex_count = (uint32_t) (m_ellipsoids.count() * nb_vertices);
            uint32_t face_count   = (uint32_t) (m_ellipsoids.count() * nb_faces);

            bool initialized = (vertex_count != m_vertex_count) || (face_count != m_face_count);

            m_vertex_count = vertex_count;
            m_face_count   = face_count;
            // The shells reference their vertices directly, without maps
            m_position_count = m_normal_count = vertex_count;

            m_packed_vertices = dr::zeros<StoredFloat>(
                (size_t) MeshVertexStride * m_vertex_count);
            m_packed_faces    = dr::zeros<IndexStorage>(4 * m_face_count);

            for (size_t i = 0; i < m_ellipsoids.count(); ++i) {
                auto ellipsoid = m_ellipsoids.template get_ellipsoid<Float>(i);
                auto rot = dr::quat_to_matrix<Matrix3f>(ellipsoid.quat);

                const ScalarAffineTransform4f &to_world =
                    ScalarAffineTransform4f::translate(ellipsoid.center) *
                    ScalarAffineTransform4f(rot) *
                    ScalarAffineTransform4f::scale(ellipsoid.scale) *
                    ScalarAffineTransform4f::scale(m_ellipsoids.template extents<Float>(i));

                for (size_t j = 0; j < nb_vertices; ++j) {
                    Point3f v = to_world * Point3f(m_shell_vertices[j]);
                    for (size_t k = 0; k < 3; ++k)
                        m_packed_vertices[(i * nb_vertices + j) *
                                          MeshVertexStride + k] = float(v[k]);
                }

                UInt32 offset = (uint32_t) i * uint32_t(nb_vertices);
                for (size_t j = 0; j < nb_faces; ++j) {
                    Vector3u face = m_shell_faces[j];
                    // 4-word face records; the fourth word is derived below
                    for (size_t k = 0; k < 3; ++k)
                        m_packed_faces[i * nb_faces * 4 + j * 4 + k] = face[k] + offset;
                }
            }

            // Skips Mesh::refresh(), see above
            this->update_face_state();
            this->drop_views();
#if defined(MI_ENABLE_LLVM) && !defined(MI_ENABLE_EMBREE)
            m_packed_vertices_ptr = m_packed_vertices.data();
            m_packed_faces_ptr = m_packed_faces.data();
#endif
            if (initialized)
                recompute_bbox();
            mark_dirty();
        }
    }

    void describe(ShapeIR &g) const override {
        Base::describe(g);
        // Mesh::describe() tags this as plain Triangles. Refine it to the
        // back-face-culled bucket (the only kind both GPU backends cull).
        g.kind = ShapeIR::Kind::TrianglesCulled;
    }

private:
    /// Object holding the ellipsoid data and attributes
    EllipsoidsData<Float, Spectrum> m_ellipsoids;

    // Template mesh for the shell
    std::vector<dr::Array<float, 3>> m_shell_vertices;
    std::vector<dr::Array<uint32_t, 3>> m_shell_faces;

    // Store constructor properties to expand to other plugin in LLVM mode.
    Properties m_props;
};

MI_EXPORT_PLUGIN(EllipsoidsMesh)

// =============================================================
// Hardcoded mesh shell template data
// =============================================================

std::vector<dr::Array<float, 3>> box_vertices = {
    {  1, -1, -1 }, {  1, -1,  1 }, { -1, -1,  1 }, { -1, -1, -1 },
    {  1,  1, -1 }, { -1,  1, -1 }, { -1,  1,  1 }, {  1,  1,  1 },
    {  1, -1, -1 }, {  1,  1, -1 }, {  1,  1,  1 }, {  1, -1,  1 },
    {  1, -1,  1 }, {  1,  1,  1 }, { -1,  1,  1 }, { -1, -1,  1 },
    { -1, -1,  1 }, { -1,  1,  1 }, { -1,  1, -1 }, { -1, -1, -1 },
    {  1,  1, -1 }, {  1, -1, -1 }, { -1, -1, -1 }, { -1,  1, -1 }
};

std::vector<dr::Array<uint32_t, 3>> box_faces = {
    {  0,  1,  2 }, {  3,  0,  2 }, {  4,  5,  6 }, {  7,  4,  6 },
    {  8,  9, 10 }, { 11,  8, 10 }, { 12, 13, 14 }, { 15, 12, 14 },
    { 16, 17, 18 }, { 19, 16, 18 }, { 20, 21, 22 }, { 23, 20, 22 }
};

std::vector<dr::Array<float, 3>> uv_sphere_72_vertices = {
    { 0.377821f, 0.809017f, -0.450270f },   { 0.611327f, 0.309017f, -0.728552f },   { 0.611327f, -0.309017f, -0.728552f },
    { 0.377821f, -0.809017f, -0.450270f },  { 0.578855f, 0.809017f, -0.102068f },   { 0.936608f, 0.309017f, -0.165149f },
    { 0.936608f, -0.309017f, -0.165149f },  { 0.578855f, -0.809017f, -0.102068f },  { 0.509037f, 0.809017f, 0.293893f },
    { 0.823639f, 0.309017f, 0.475528f },    { 0.823639f, -0.309017f, 0.475528f },   { 0.509037f, -0.809017f, 0.293893f },
    { 0.201034f, 0.809017f, 0.552337f },    { 0.325280f, 0.309017f, 0.893701f },    { 0.325280f, -0.309017f, 0.893701f },
    { 0.201034f, -0.809017f, 0.552337f },   { -0.201034f, 0.809017f, 0.552337f },   { -0.325280f, 0.309017f, 0.893701f },
    { -0.325280f, -0.309017f, 0.893701f },  { -0.201034f, -0.809017f, 0.552337f },  { 0.000000f, 1.000000f, 0.000000f },
    { -0.509037f, 0.809017f, 0.293893f },   { -0.823639f, 0.309017f, 0.475528f },   { -0.823639f, -0.309017f, 0.475528f },
    { -0.509037f, -0.809017f, 0.293893f },  { -0.578855f, 0.809017f, -0.102068f },  { -0.936608f, 0.309017f, -0.165149f },
    { -0.936608f, -0.309017f, -0.165149f }, { -0.578855f, -0.809017f, -0.102068f }, { -0.377821f, 0.809017f, -0.450269f },
    { -0.611327f, 0.309017f, -0.728551f },  { -0.611327f, -0.309017f, -0.728551f }, { -0.377821f, -0.809017f, -0.450269f },
    { -0.000000f, 0.809017f, -0.587785f },  { -0.000000f, 0.309017f, -0.951056f },  { -0.000000f, -0.309017f, -0.951056f },
    { -0.000000f, -0.809017f, -0.587785f }, { 0.000000f, -1.000000f, 0.000000f }
};

std::vector<dr::Array<uint32_t, 3>> uv_sphere_72_faces = {
    { 35, 3, 36 },  { 34, 0, 1 },   { 37, 36, 3 },
    { 34, 2, 35 },  { 33, 20, 0 },  { 37, 3, 7 },
    { 1, 6, 2 },    { 0, 20, 4 },   { 2, 7, 3 },
    { 1, 4, 5 },    { 37, 7, 11 },  { 5, 10, 6 },
    { 4, 20, 8 },   { 6, 11, 7 },   { 4, 9, 5 },
    { 37, 11, 15 }, { 9, 14, 10 },  { 8, 20, 12 },
    { 11, 14, 15 }, { 8, 13, 9 },   { 37, 15, 19 },
    { 13, 18, 14 }, { 12, 20, 16 }, { 14, 19, 15 },
    { 13, 16, 17 }, { 37, 19, 24 }, { 17, 23, 18 },
    { 16, 20, 21 }, { 18, 24, 19 }, { 17, 21, 22 },
    { 37, 24, 28 }, { 22, 27, 23 }, { 21, 20, 25 },
    { 23, 28, 24 }, { 22, 25, 26 }, { 37, 28, 32 },
    { 26, 31, 27 }, { 25, 20, 29 }, { 27, 32, 28 },
    { 26, 29, 30 }, { 37, 32, 36 }, { 30, 35, 31 },
    { 29, 20, 33 }, { 32, 35, 36 }, { 29, 34, 30 },
    { 35, 2, 3 },   { 34, 33, 0 },  { 34, 1, 2 },
    { 1, 5, 6 },    { 2, 6, 7 },    { 1, 0, 4 },
    { 5, 9, 10 },   { 6, 10, 11 },  { 4, 8, 9 },
    { 9, 13, 14 },  { 11, 10, 14 }, { 8, 12, 13 },
    { 13, 17, 18 }, { 14, 18, 19 }, { 13, 12, 16 },
    { 17, 22, 23 }, { 18, 23, 24 }, { 17, 16, 21 },
    { 22, 26, 27 }, { 23, 27, 28 }, { 22, 21, 25 },
    { 26, 30, 31 }, { 27, 31, 32 }, { 26, 25, 29 },
    { 30, 34, 35 }, { 32, 31, 35 }, { 29, 33, 34 },
};

std::vector<dr::Array<float, 3>> ico_sphere_20_vertices = {
    { 0.000000f, -1.000000f, 0.000000f },   { 0.723600f, -0.447215f, 0.525720f },   { -0.276385f, -0.447215f, 0.850640f },
    { -0.894425f, -0.447215f, 0.000000f },  { -0.276385f, -0.447215f, -0.85064f },  { 0.723600f, -0.447215f, -0.525720f },
    { 0.276385f, 0.447215f, 0.850640f },    { -0.723600f, 0.447215f, 0.525720f },   { -0.723600f, 0.447215f, -0.525720f },
    { 0.276385f, 0.447215f, -0.850640f },   { 0.894425f, 0.447215f, 0.000000f },    { 0.000000f, 1.000000f, 0.000000f }
};

std::vector<dr::Array<uint32_t, 3>> ico_sphere_20_faces = {
    { 0, 1, 2 }, { 1, 0, 5 }, { 0, 2, 3 }, { 0, 3, 4 }, { 0, 4, 5 },
    { 1, 5, 10 }, { 2, 1, 6 }, { 3, 2, 7 }, { 4, 3, 8 }, { 5, 4, 9 },
    { 1, 10, 6 }, { 2, 6, 7 }, { 3, 7, 8 }, { 4, 8, 9 }, { 5, 9, 10 },
    { 6, 10, 11 }, { 7, 6, 11 }, { 8, 7, 11 }, { 9, 8, 11 }, { 10, 9, 11 }
};

//
// =============================================================

NAMESPACE_END(mitsuba)


```
