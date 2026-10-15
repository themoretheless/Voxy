# Facial rendering source scan: patrikhuber/eos

Repository: https://github.com/patrikhuber/eos

Pinned source revision: ce984207a51c2edd2e7568a64fec9b98ef61aeed

Discovery topic: face model 3D

Description: A lightweight 3D Morphable Face Model library in modern C++

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["include/eos/fitting/blendshape_fitting.hpp", "include/eos/morphablemodel/Blendshape.hpp", "share/expression_blendshapes_3448.bin"], "mouth": ["share/bfm2017-1_bfm_nomouth_model_contours.json", "share/ibug_to_bfm2017-1_bfm_nomouth.txt"], "shader": ["include/eos/render/FragmentShader.hpp", "include/eos/render/VertexShader.hpp"]}

## Source file: include/eos/render/VertexShader.hpp

{
  "path": "include/eos/render/VertexShader.hpp",
  "blob_sha": "83dc0ad469f530618c2d506cf209852367ccac82",
  "sha256": "b05425c36e6ae7abc88393b8439164e61cb7bba41e545efdb43fc7ef1d71b63a",
  "lines": 58,
  "bytes": 1881,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 6,
    "rig": 1,
    "lip": 3
  },
  "url": "https://github.com/patrikhuber/eos/blob/ce984207a51c2edd2e7568a64fec9b98ef61aeed/include/eos/render/VertexShader.hpp",
  "function_signature_candidates": []
}

```text

/*
 * eos - A 3D Morphable Model fitting library written in modern C++11/14.
 *
 * File: include/eos/render/VertexShader.hpp
 *
 * Copyright 2017, 2023 Patrik Huber
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#pragma once

#ifndef EOS_VERTEX_SHADER_HPP
#define EOS_VERTEX_SHADER_HPP

#include "Eigen/Core"

namespace eos {
namespace render {

/**
 * @brief A simple vertex shader that projects the vertex and returns the vertex in clip-space coordinates.
 */
class VertexShader
{
public:
    /**
     * @brief Projects the given vertex into clip-space and returns it.
     *
     * @param[in] vertex The vertex to project.
     * @param[in] model_view_matrix The model-view matrix.
     * @param[in] projection_matrix The projection matrix.
     * @tparam VertexType Vertex type.
     * @tparam MatrixType Matrix type.
     * @return Vertex projected to clip space.
     */
    template <typename VertexType, typename MatrixType>
    Eigen::Vector4<MatrixType> operator()(const Eigen::Vector4<VertexType>& vertex,
                                          const Eigen::Matrix4<MatrixType>& model_view_matrix,
                                          const Eigen::Matrix4<MatrixType>& projection_matrix)
    {
        return projection_matrix * model_view_matrix * vertex;
    };
};

} /* namespace render */
} /* namespace eos */

#endif /* EOS_VERTEX_SHADER_HPP */


```

## Source file: include/eos/render/FragmentShader.hpp

{
  "path": "include/eos/render/FragmentShader.hpp",
  "blob_sha": "194452a913c3016d83ce8cd6e6571bc0a9e3a4a6",
  "sha256": "e23207a24cf90b2cd7c942f0d40e1514fd50f0316fad1c215120526954f9d337",
  "lines": 200,
  "bytes": 8341,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 16,
    "rig": 1
  },
  "url": "https://github.com/patrikhuber/eos/blob/ce984207a51c2edd2e7568a64fec9b98ef61aeed/include/eos/render/FragmentShader.hpp",
  "function_signature_candidates": []
}

```text

/*
 * eos - A 3D Morphable Model fitting library written in modern C++11/14.
 *
 * File: include/eos/render/FragmentShader.hpp
 *
 * Copyright 2017, 2023 Patrik Huber
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#pragma once

#ifndef EOS_FRAGMENT_SHADER_HPP
#define EOS_FRAGMENT_SHADER_HPP

#include "eos/render/detail/Vertex.hpp"
#include "eos/render/detail/texturing.hpp"
#include "eos/cpp17/optional.hpp"

#include "Eigen/Core"

// Fragment shaders are a more accurate name for the same functionality as Pixel shaders. They aren't pixels
// yet, since the output still has to past several tests (depth, alpha, stencil) as well as the fact that one
// may be using anti-aliasing, which renders one-fragment-to-one-pixel non-true.
// The "pixel" in "pixel shader" is a misnomer because the pixel shader doesn't operate on pixels directly.
// The pixel shader operates on "fragments" which may or may not end up as actual pixels, depending on several
// factors outside of the pixel shader.
// The shaders *can not* depend on any state - they have to be able to run independently and in parallel!
// But can they really? What about the z-test? It happens earlier at the moment - which is good?

namespace eos {
namespace render {

/**
 * @brief A simple fragment shader that does vertex-colouring.
 *
 * Uses the vertex colour data to shade the given fragment / pixel location.
 */
class VertexColoringFragmentShader
{
public:
    /**
     * @brief Todo.
     *
     * Todo.
     * lambda is not perspectively corrected. Note: In our case, it is, as we do it in the raster loop at
     * the moment?
     * The given colour values should be in the range [0, 1]. The returned colour will also be in the range
     * [0, 1]. Note/Todo: Can colour values be <0 and >1? If so, we should document and tell the user to
     * perhaps clamp the values.
     *
     * @param[in] x X.
     * @return RGBA colour in the range [0, 1].
     */
    template <typename T>
    Eigen::Vector4<T> shade_triangle_pixel(int x, int y, const detail::Vertex<T>& point_a,
                                           const detail::Vertex<T>& point_b, const detail::Vertex<T>& point_c,
                                           const Eigen::Vector3<T>& lambda,
                                           const cpp17::optional<Texture>& texture, float dudx, float dudy,
                                           float dvdx, float dvdy)
    {
        // attributes interpolation
        const Eigen::Vector3<T> color_persp =
            lambda[0] * point_a.color + lambda[1] * point_b.color + lambda[2] * point_c.color;
        return Eigen::Vector4<T>(color_persp[0], color_persp[1], color_persp[2], T(1));
    };
};

/**
 * @brief A fragment shader that textures.
 */
class TexturingFragmentShader
{
public:
    /**
     * @brief Todo.
     *
     * See comments above about lambda (persp. corrected?).
     *
     * This shader reads the colour value from the given \p texture.
     * The returned colour will be in the range [0, 1].
     * Note/Todo: Can colour values be <0 and >1? If so, we should document and tell the user to perhaps clamp
     * the values.
     *
     * @param[in] x X.
     * @return RGBA colour in the range [0, 1].
     */
    template <typename T>
    Eigen::Vector4<T> shade_triangle_pixel(int x, int y, const detail::Vertex<T>& point_a,
                                           const detail::Vertex<T>& point_b, const detail::Vertex<T>& point_c,
                                           const Eigen::Vector3<T>& lambda,
                                           const cpp17::optional<Texture>& texture, float dudx, float dudy,
                                           float dvdx, float dvdy)
    {
        Eigen::Vector2<T> texcoords_persp =
            lambda[0] * point_a.texcoords + lambda[1] * point_b.texcoords + lambda[2] * point_c.texcoords;

        // The Texture is in BGR, thus tex2D returns BGR
        // Todo: Think about changing that.
        // tex2d divides the colour values by 255, so that the return value we get here is in the range [0, 1].
        const Eigen::Vector3<T> texture_color =
            detail::tex2d(texcoords_persp, texture.value(), dudx, dudy, dvdx, dvdy); // uses the current texture
        const Eigen::Vector3<T> pixel_color =
            Eigen::Vector3<T>(texture_color[2], texture_color[1], texture_color[0]);
        // other: color.mul(tex2D(texture, texCoord));
        return Eigen::Vector4<T>(pixel_color[0], pixel_color[1], pixel_color[2], T(1));
    };
};

/**
 * @brief Computes inverse perspectively correct lambda.
 *
 * X.
 * Inverts the perspective texture mapping. Can be derived using some tedious algebra.
 * Todo: Probably move to a texturing file, internal/detail one, where we will also put the tex2d, mipmapping
 * etc stuff?
 *
 * @param[in] X X.
 * @return X.
 */
template <typename T>
Eigen::Vector3<T> compute_inverse_perspectively_correct_lambda(const Eigen::Vector3<T>& lambda_world,
                                                               const T& one_over_w0, const T& one_over_w1,
                                                               const T& one_over_w2)
{
    const float w0 = 1 / one_over_w0;
    const float w1 = 1 / one_over_w1;
    const float w2 = 1 / one_over_w2;

    const float d = w0 - (w0 - w1) * lambda_world.y() - (w0 - w2) * lambda_world.z();
    if (d == 0)
        return lambda_world;

    Eigen::Vector3<T> lambda;

    lambda.z() = lambda_world.z() * w2 / d;
    lambda.y() = lambda_world.y() * w1 / d;

    lambda.x() = 1 - lambda.y() - lambda.z();
    return lambda;
};

/**
 * @brief A fragment shader that is used to extract, or remap, texture from an image to a UV map (i.e. the
 * reverse process of texturing).
 */
class ExtractionFragmentShader
{
public:
    /**
     * @brief Todo.
     *
     * Todo.
     * Inverts the perspective texture mapping. Can be derived using some tedious algebra.
     * NOTE: This one actually takes/needs the perspectively corrected lambda I think!
     *
     * Todo: Probably move to a texturing file, internal/detail one, where we will also put the tex2d,
     * mipmapping etc stuff?
     *
     * @param[in] X X.
     * @return X.
     */
    template <typename T>
    Eigen::Vector4<T> shade_triangle_pixel(int x, int y, const detail::Vertex<T>& point_a,
                                           const detail::Vertex<T>& point_b, const detail::Vertex<T>& point_c,
                                           const Eigen::Vector3<T>& lambda,
                                           const cpp17::optional<Texture>& texture, float dudx, float dudy,
                                           float dvdx, float dvdy)
    {
        const auto corrected_lambda = compute_inverse_perspectively_correct_lambda(
            lambda, point_a.position.w(), point_b.position.w(), point_c.position.w());
        const Eigen::Vector2<T> texcoords_persp = corrected_lambda[0] * point_a.texcoords +
                                                  corrected_lambda[1] * point_b.texcoords +
                                                  corrected_lambda[2] * point_c.texcoords;

        // Texturing, no mipmapping:
        Eigen::Vector2<T> image_tex_coords = detail::texcoord_wrap(texcoords_persp);
        image_tex_coords[0] *= texture->mipmaps[0].width();
        image_tex_coords[1] *= texture->mipmaps[0].height();
        const Eigen::Vector3<T> texture_color =
            detail::tex2d_linear(image_tex_coords, 0, texture.value()) / 255.0f;
        const Eigen::Vector3<T> pixel_color =
            Eigen::Vector3<T>(texture_color[2], texture_color[1], texture_color[0]);
        return Eigen::Vector4<T>(pixel_color[0], pixel_color[1], pixel_color[2], T(1));
    };
};

} /* namespace render */
} /* namespace eos */

#endif /* EOS_FRAGMENT_SHADER_HPP */


```

## Source file: include/eos/morphablemodel/Blendshape.hpp

{
  "path": "include/eos/morphablemodel/Blendshape.hpp",
  "blob_sha": "8105d4539816d99740a86741cbe3dbec8148badd",
  "sha256": "c47a76e3c67ecfab1f5399ef324fe8c9e0552feee53d836c01b330b44e1a3f5e",
  "lines": 148,
  "bytes": 4589,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 34,
    "deform": 5,
    "rig": 1
  },
  "url": "https://github.com/patrikhuber/eos/blob/ce984207a51c2edd2e7568a64fec9b98ef61aeed/include/eos/morphablemodel/Blendshape.hpp",
  "function_signature_candidates": [
    {
      "line": 59,
      "text": "void serialize(Archive& archive)"
    },
    {
      "line": 101,
      "text": "inline void save_blendshapes(const std::vector<Blendshape>& blendshapes, std::string filename)"
    }
  ]
}

```text

/*
 * eos - A 3D Morphable Model fitting library written in modern C++11/14.
 *
 * File: include/eos/morphablemodel/Blendshape.hpp
 *
 * Copyright 2015 Patrik Huber
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#pragma once

#ifndef EOS_BLENDSHAPE_HPP
#define EOS_BLENDSHAPE_HPP

#include "cereal/archives/binary.hpp"
#include "cereal/types/string.hpp"
#include "eos/morphablemodel/io/eigen_cerealisation.hpp"

#include "Eigen/Core"

#include <cassert>
#include <fstream>
#include <string>
#include <vector>

namespace eos {
namespace morphablemodel {

/**
 * @brief A class representing a 3D blendshape.
 *
 * A blendshape is a vector of offsets that transform the vertices of
 * a given mesh or shape instance. Usually, a blendshape is associated
 * with a deformation like a particular facial expression or a phoneme.
 */
struct Blendshape
{
    std::string name;            ///< Name of the blendshape.
    Eigen::VectorXf deformation; ///< A 3m x 1 col-vector (xyzxyz...)', where m is the number of
                                 ///< model-vertices. Has the same format as PcaModel::mean.

    friend class cereal::access;
    /**
     * Serialises this class using cereal.
     *
     * @param[in] archive The archive to serialise to (or to serialise from).
     */
    template <class Archive>
    void serialize(Archive& archive)
    {
        archive(CEREAL_NVP(name), CEREAL_NVP(deformation));
    };
};

/**
 * Shorthand notation for an std::vector<Blendshape>.
 */
using Blendshapes = std::vector<Blendshape>;

/**
 * Helper method to load a file with blendshapes from
 * a cereal::BinaryInputArchive from the harddisk.
 *
 * @param[in] filename Filename to a blendshapes-file.
 * @return The loaded blendshapes.
 * @throw std::runtime_error When the file given in \c filename fails to be opened (most likely because the
 * file doesn't exist).
 */
inline std::vector<Blendshape> load_blendshapes(std::string filename)
{
    std::vector<Blendshape> blendshapes;

    std::ifstream file(filename, std::ios::binary);
    if (!file)
    {
        throw std::runtime_error("Error opening given file: " + filename);
    }
    cereal::BinaryInputArchive input_archive(file);
    input_archive(blendshapes);

    return blendshapes;
};

/**
 * Helper method to save a set of blendshapes to the
 * harddisk as a cereal::BinaryOutputArchive.
 *
 * @param[in] blendshapes The blendshapes to be saved.
 * @param[in] filename Filename for the blendshapes.
 */
inline void save_blendshapes(const std::vector<Blendshape>& blendshapes, std::string filename)
{
    std::ofstream file(filename, std::ios::binary);
    if (!file)
    {
        throw std::runtime_error("Error creating given file: " + filename);
    }
    cereal::BinaryOutputArchive output_archive(file);
    output_archive(blendshapes);
};

/**
 * @brief Copies the blendshapes into a matrix, with each column being a blendshape.
 *
 * @param[in] blendshapes Vector of blendshapes.
 * @return The resulting matrix.
 */
inline Eigen::MatrixXf to_matrix(const std::vector<Blendshape>& blendshapes)
{
    assert(blendshapes.size() > 0);
    // Todo: Assert all blendshapes have to have the same number of rows, and one col

    Eigen::MatrixXf blendshapes_as_basis(blendshapes[0].deformation.rows(), blendshapes.size());
    for (int i = 0; i < blendshapes.size(); ++i)
    {
        blendshapes_as_basis.col(i) = blendshapes[i].deformation;
    }
    return blendshapes_as_basis;
};

/**
 * @brief Maps an std::vector of coefficients with Eigen::Map, so it can be multiplied
 * with a blendshapes matrix.
 *
 * Note that the resulting Eigen::Map only lives as long as the data given lives and is in scope.
 *
 * @param[in] coefficients Vector of blendshape coefficients.
 * @return An Eigen::Map pointing to the given coefficients data.
 */
inline Eigen::Map<const Eigen::VectorXf> to_vector(const std::vector<float>& coefficients)
{
    return Eigen::Map<const Eigen::VectorXf>(coefficients.data(), coefficients.size());
};

} /* namespace morphablemodel */
} /* namespace eos */

#endif /* EOS_BLENDSHAPE_HPP */


```

## Source file: include/eos/fitting/blendshape_fitting.hpp

{
  "path": "include/eos/fitting/blendshape_fitting.hpp",
  "blob_sha": "aae7c3a46c48c2cd9e38a0b5807a0f2234b46373",
  "sha256": "bb6a233e1df8a6f31d835fe06983664d4ed050b4ded651f65ae79963cbd5680d",
  "lines": 216,
  "bytes": 10365,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 34,
    "rig": 1,
    "face": 14
  },
  "url": "https://github.com/patrikhuber/eos/blob/ce984207a51c2edd2e7568a64fec9b98ef61aeed/include/eos/fitting/blendshape_fitting.hpp",
  "function_signature_candidates": []
}

```text

/*
 * eos - A 3D Morphable Model fitting library written in modern C++11/14.
 *
 * File: include/eos/fitting/blendshape_fitting.hpp
 *
 * Copyright 2015 Patrik Huber
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#pragma once

#ifndef BLENDSHAPEFITTING_HPP_
#define BLENDSHAPEFITTING_HPP_

#include "eos/morphablemodel/Blendshape.hpp"

#include "Eigen/Core"
#include "Eigen/QR"
#include "Eigen/Sparse"
#include "nnls.h"

#include <vector>
#include <cassert>

namespace eos {
namespace fitting {

/**
 * Fits blendshape coefficients to given 2D landmarks, given a current face shape instance.
 * It's a linear, closed-form solution fitting algorithm, with regularisation (constraining
 * the L2-norm of the coefficients). However, there is no constraint on the coefficients,
 * so negative coefficients are allowed, which, with linear blendshapes (offsets), will most
 * likely not be desired. Thus, prefer the function
 * fit_blendshapes_to_landmarks_nnls(std::vector<eos::morphablemodel::Blendshape>, cv::Mat, cv::Mat, std::vector<cv::Vec2f>, std::vector<int>).
 * 
 * This algorithm is very similar to the shape fitting in fit_shape_to_landmarks_linear.
 * Instead of the PCA basis, the blendshapes are used, and instead of the mean, a current
 * face instance is used to do the fitting from.
 *
 * @param[in] blendshapes A vector with blendshapes to estimate the coefficients for.
 * @param[in] face_instance A shape instance from which the blendshape coefficients should be estimated (i.e. the current mesh without expressions, e.g. estimated from a previous PCA-model fitting). A 3m x 1 matrix.
 * @param[in] affine_camera_matrix A 3x4 affine camera matrix from model to screen-space.
 * @param[in] landmarks 2D landmarks from an image to fit the blendshapes to.
 * @param[in] vertex_ids The vertex ids in the model that correspond to the 2D points.
 * @param[in] lambda A regularisation parameter, constraining the L2-norm of the coefficients.
 * @return The estimated blendshape-coefficients.
 */
inline std::vector<float> fit_blendshapes_to_landmarks_linear(
    const std::vector<morphablemodel::Blendshape>& blendshapes, const Eigen::VectorXf& face_instance,
    Eigen::Matrix<float, 3, 4> affine_camera_matrix, const std::vector<Eigen::Vector2f>& landmarks,
    const std::vector<int>& vertex_ids, float lambda = 500.0f)
{
    assert(landmarks.size() == vertex_ids.size());

    using Eigen::MatrixXf;
    using Eigen::VectorXf;

    const auto num_blendshapes = blendshapes.size();
    const int num_landmarks = static_cast<int>(landmarks.size());

    // Copy all blendshapes into a "basis" matrix with each blendshape being a column:
    const MatrixXf blendshapes_as_basis = morphablemodel::to_matrix(blendshapes);

    // $\hat{V} \in R^{3N\times m-1}$, subselect the rows of the eigenvector matrix $V$ associated with the $N$ feature points
    // And we insert a row of zeros after every third row, resulting in matrix $\hat{V}_h \in R^{4N\times m-1}$:
    MatrixXf V_hat_h = MatrixXf::Zero(4 * num_landmarks, num_blendshapes);
    int row_index = 0;
    for (int i = 0; i < num_landmarks; ++i)
    {
        V_hat_h.block(row_index, 0, 3, V_hat_h.cols()) =
            blendshapes_as_basis.block(vertex_ids[i] * 3, 0, 3, blendshapes_as_basis.cols());
        row_index += 4; // replace 3 rows and skip the 4th one, it has all zeros
    }
    // Form a block diagonal matrix $P \in R^{3N\times 4N}$ in which the camera matrix C (P_Affine, affine_camera_matrix) is placed on the diagonal:
    MatrixXf P = MatrixXf::Zero(3 * num_landmarks, 4 * num_landmarks);
    for (int i = 0; i < num_landmarks; ++i)
    {
        P.block<3, 4>(3 * i, 4 * i) = affine_camera_matrix;
    }

    // The landmarks in matrix notation (in homogeneous coordinates), $3N\times 1$
    VectorXf y = VectorXf::Ones(3 * num_landmarks);
    for (int i = 0; i < num_landmarks; ++i)
    {
        y(3 * i) = landmarks[i][0];
        y((3 * i) + 1) = landmarks[i][1];
        // y((3 * i) + 2) = 1; // already 1, stays (homogeneous coordinate)
    }
    // The mean, with an added homogeneous coordinate (x_1, y_1, z_1, 1, x_2, ...)^t
    VectorXf v_bar = VectorXf::Ones(4 * num_landmarks);
    for (int i = 0; i < num_landmarks; ++i)
    {
        v_bar(4 * i) = face_instance(vertex_ids[i] * 3);
        v_bar((4 * i) + 1) = face_instance(vertex_ids[i] * 3 + 1);
        v_bar((4 * i) + 2) = face_instance(vertex_ids[i] * 3 + 2);
        // v_bar((4 * i) + 3) = 1; // already 1, stays (homogeneous coordinate)
    }

    // Bring into standard regularised quadratic form:
    const MatrixXf A = P * V_hat_h;   // camera matrix times the basis
    const MatrixXf b = P * v_bar - y; // camera matrix times the mean, minus the landmarks

    const MatrixXf AtAReg =
        A.transpose() * A + lambda * Eigen::MatrixXf::Identity(num_blendshapes, num_blendshapes);
    const MatrixXf rhs = -A.transpose() * b;

    const VectorXf coefficients = AtAReg.colPivHouseholderQr().solve(rhs);

    return std::vector<float>(coefficients.data(), coefficients.data() + coefficients.size());
};

/**
 * Fits blendshape coefficients to given 2D landmarks, given a current face shape instance.
 * Uses non-negative least-squares (NNLS) to solve for the coefficients. The NNLS algorithm
 * used doesn't support any regularisation.
 *
 * This algorithm is very similar to the shape fitting in fit_shape_to_landmarks_linear.
 * Instead of the PCA basis, the blendshapes are used, and instead of the mean, a current
 * face instance is used to do the fitting from.
 *
 * @param[in] blendshapes A vector with blendshapes to estimate the coefficients for.
 * @param[in] face_instance A shape instance from which the blendshape coefficients should be estimated (i.e. the current mesh without expressions, e.g. estimated from a previous PCA-model fitting). A 3m x 1 matrix.
 * @param[in] affine_camera_matrix A 3x4 affine camera matrix from model to screen-space.
 * @param[in] landmarks 2D landmarks from an image to fit the blendshapes to.
 * @param[in] vertex_ids The vertex ids in the model that correspond to the 2D points.
 * @return The estimated blendshape-coefficients.
 */
inline std::vector<float> fit_blendshapes_to_landmarks_nnls(
    const std::vector<morphablemodel::Blendshape>& blendshapes, const Eigen::VectorXf& face_instance,
    Eigen::Matrix<float, 3, 4> affine_camera_matrix, const std::vector<Eigen::Vector2f>& landmarks,
    const std::vector<int>& vertex_ids)
{
    assert(landmarks.size() == vertex_ids.size());

    using Eigen::MatrixXf;
    using Eigen::VectorXf;

    const auto num_blendshapes = blendshapes.size();
    const int num_landmarks = static_cast<int>(landmarks.size());

    // Copy all blendshapes into a "basis" matrix with each blendshape being a column:
    const MatrixXf blendshapes_as_basis = morphablemodel::to_matrix(blendshapes);

    // $\hat{V} \in R^{3N\times m-1}$, subselect the rows of the eigenvector matrix $V$ associated with the $N$ feature points
    // And we insert a row of zeros after every third row, resulting in matrix $\hat{V}_h \in R^{4N\times m-1}$:
    MatrixXf V_hat_h = MatrixXf::Zero(4 * num_landmarks, num_blendshapes);
    int row_index = 0;
    for (int i = 0; i < num_landmarks; ++i)
    {
        V_hat_h.block(row_index, 0, 3, V_hat_h.cols()) =
            blendshapes_as_basis.block(vertex_ids[i] * 3, 0, 3, blendshapes_as_basis.cols());
        row_index += 4; // replace 3 rows and skip the 4th one, it has all zeros
    }

    // Form a block diagonal matrix $P \in R^{3N\times 4N}$ in which the camera matrix C (P_Affine, affine_camera_matrix) is placed on the diagonal:
    Eigen::SparseMatrix<float> P(3 * num_landmarks, 4 * num_landmarks);
    std::vector<Eigen::Triplet<float>> P_coefficients; // list of non-zeros coefficients
    for (int i = 0; i < num_landmarks; ++i)
    { // Note: could make this the inner-most loop.
        for (int x = 0; x < affine_camera_matrix.rows(); ++x)
        {
            for (int y = 0; y < affine_camera_matrix.cols(); ++y)
            {
                P_coefficients.push_back(
                    Eigen::Triplet<float>(3 * i + x, 4 * i + y, affine_camera_matrix(x, y)));
            }
        }
    }
    P.setFromTriplets(P_coefficients.begin(), P_coefficients.end());

    // The landmarks in matrix notation (in homogeneous coordinates), $3N\times 1$
    VectorXf y = VectorXf::Ones(3 * num_landmarks);
    for (int i = 0; i < num_landmarks; ++i)
    {
        y(3 * i) = landmarks[i][0];
        y((3 * i) + 1) = landmarks[i][1];
        // y_((3 * i) + 2) = 1; // already 1, stays (homogeneous coordinate)
    }

    // The mean, with an added homogeneous coordinate (x_1, y_1, z_1, 1, x_2, ...)^t
    VectorXf v_bar = VectorXf::Ones(4 * num_landmarks);
    for (int i = 0; i < num_landmarks; ++i)
    {
        v_bar(4 * i) = face_instance(vertex_ids[i] * 3);
        v_bar((4 * i) + 1) = face_instance(vertex_ids[i] * 3 + 1);
        v_bar((4 * i) + 2) = face_instance(vertex_ids[i] * 3 + 2);
        // v_bar((4 * i) + 3) = 1; // already 1, stays (homogeneous coordinate)
    }

    // Bring into standard least squares form:
    const MatrixXf A = P * V_hat_h;   // camera matrix times the basis
    const MatrixXf b = P * v_bar - y; // camera matrix times the mean, minus the landmarks

    // Solve using NNLS:
    VectorXf coefficients;
    Eigen::NNLS<MatrixXf> nnls(A, 100); // In the multi-image fitting, the solver sometimes got stuck. This limits it to 100 iterations. It normaly converges within <10 iterations. This should be fine for the single-image fitting as well.
    bool non_singular = nnls.solve(-b);
    coefficients.noalias() = nnls.x();

    return std::vector<float>(coefficients.data(), coefficients.data() + coefficients.size());
};

} /* namespace fitting */
} /* namespace eos */

#endif /* BLENDSHAPEFITTING_HPP_ */


```
