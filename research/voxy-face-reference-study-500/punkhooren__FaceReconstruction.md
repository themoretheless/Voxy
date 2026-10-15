# Facial rendering source scan: punkhooren/FaceReconstruction

Repository: https://github.com/punkhooren/FaceReconstruction

Pinned source revision: 03b4e83a8287b97e320ccabb4645fc7d09dfabca

Discovery topic: face reconstruction 3D

Description: Facial Landmark Detection and head pose compute use dlib, Real time Face Reconstruction use 3D Morphable Face Model fitting

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["3rdparty/cereal-1.1.1/LICENSE", "3rdparty/cereal-1.1.1/include/cereal/external/rapidjson/license.txt", "3rdparty/cereal-1.1.1/include/cereal/external/rapidxml/license.txt", "3rdparty/dlib-18.17/dlib/LICENSE.txt", "3rdparty/dlib-18.17/dlib/external/libpng/LICENSE", "3rdparty/glm-0.9.7.0/copying.txt", "LICENSE"]

Topic paths: {"iris": ["3rdparty/dlib-18.17/dlib/test/create_iris_datafile.cpp", "3rdparty/dlib-18.17/dlib/test/create_iris_datafile.h"], "rig": ["3rdparty/glm-0.9.7.0/glm/detail/func_trigonometric.hpp", "3rdparty/glm-0.9.7.0/glm/detail/func_trigonometric.inl", "3rdparty/glm-0.9.7.0/glm/detail/intrinsic_trigonometric.hpp", "3rdparty/glm-0.9.7.0/glm/detail/intrinsic_trigonometric.inl", "3rdparty/glm-0.9.7.0/glm/gtx/fast_trigonometry.hpp", "3rdparty/glm-0.9.7.0/glm/gtx/fast_trigonometry.inl", "3rdparty/glm-0.9.7.0/glm/trigonometric.hpp"], "face": ["3rdparty/dlib-18.17/dlib/image_processing/frontal_face_detector.h", "3rdparty/dlib-18.17/dlib/image_processing/frontal_face_detector_abstract.h", "3rdparty/dlib-18.17/dlib/image_processing/render_face_detections.h", "3rdparty/dlib-18.17/dlib/image_processing/render_face_detections_abstract.h", "3rdparty/dlib-18.17/dlib/interfaces/cmd_line_parser_option.h", "3rdparty/dlib-18.17/dlib/interfaces/enumerable.h", "3rdparty/dlib-18.17/dlib/interfaces/map_pair.h", "3rdparty/dlib-18.17/dlib/interfaces/remover.h", "3rdparty/dlib-18.17/dlib/test/face.cpp", "3rdparty/dlib-18.17/dlib/xml_parser/xml_parser_kernel_interfaces.h", "bin/face_landmark_detection_ex", "bin/webcam_face_fit_model_keegan", "bin/webcam_face_pose_ex", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/CXX.includecache", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/DependInfo.cmake", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/build.make", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/cmake_clean.cmake", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/depend.internal", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/depend.make", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/flags.make", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/link.txt", "build/examples/CMakeFiles/face_landmark_detection_ex.dir/progress.make", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/CXX.includecache", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/DependInfo.cmake", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/build.make", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/cmake_clean.cmake", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/depend.internal", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/depend.make", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/flags.make", "build/examples/CMakeFiles/webcam_face_fit_model_keegan.dir/link.txt"]}

## Source file: 3rdparty/glm-0.9.7.0/glm/trigonometric.hpp

{
  "path": "3rdparty/glm-0.9.7.0/glm/trigonometric.hpp",
  "blob_sha": "21ba20a194a5835e2a4d74a06828b14c6dba6ba9",
  "sha256": "8bb13f53dbd09817905943a4ce339b858804b69cac8fabfcd88a63e4cdde1013",
  "lines": 35,
  "bytes": 1658,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 6
  },
  "url": "https://github.com/punkhooren/FaceReconstruction/blob/03b4e83a8287b97e320ccabb4645fc7d09dfabca/3rdparty/glm-0.9.7.0/glm/trigonometric.hpp",
  "function_signature_candidates": []
}

```text

///////////////////////////////////////////////////////////////////////////////////
/// OpenGL Mathematics (glm.g-truc.net)
///
/// Copyright (c) 2005 - 2015 G-Truc Creation (www.g-truc.net)
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
/// 
/// The above copyright notice and this permission notice shall be included in
/// all copies or substantial portions of the Software.
/// 
/// Restrictions:
///		By making use of the Software for military purposes, you choose to make
///		a Bunny unhappy.
/// 
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
/// THE SOFTWARE.
///
/// @ref core
/// @file glm/trigonometric.hpp
/// @date 2013-12-24 / 2013-12-24
/// @author Christophe Riccio
///////////////////////////////////////////////////////////////////////////////////

#pragma once

#include "detail/func_trigonometric.hpp"


```

## Source file: 3rdparty/glm-0.9.7.0/glm/gtx/fast_trigonometry.hpp

{
  "path": "3rdparty/glm-0.9.7.0/glm/gtx/fast_trigonometry.hpp",
  "blob_sha": "93c38ab28f76bc06ddf734158171af2e2f282cf9",
  "sha256": "7e23c9e1dd12bbb150f1aea652ba11a656f9917872d452a280a24b7f92fc6c59",
  "lines": 104,
  "bytes": 3734,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 20
  },
  "url": "https://github.com/punkhooren/FaceReconstruction/blob/03b4e83a8287b97e320ccabb4645fc7d09dfabca/3rdparty/glm-0.9.7.0/glm/gtx/fast_trigonometry.hpp",
  "function_signature_candidates": []
}

```text

///////////////////////////////////////////////////////////////////////////////////
/// OpenGL Mathematics (glm.g-truc.net)
///
/// Copyright (c) 2005 - 2015 G-Truc Creation (www.g-truc.net)
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
/// 
/// The above copyright notice and this permission notice shall be included in
/// all copies or substantial portions of the Software.
/// 
/// Restrictions:
///		By making use of the Software for military purposes, you choose to make
///		a Bunny unhappy.
/// 
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
/// THE SOFTWARE.
///
/// @ref gtx_fast_trigonometry
/// @file glm/gtx/fast_trigonometry.hpp
/// @date 2006-01-08 / 2011-06-07
/// @author Christophe Riccio
///
/// @see core (dependence)
///
/// @defgroup gtx_fast_trigonometry GLM_GTX_fast_trigonometry
/// @ingroup gtx
/// 
/// @brief Fast but less accurate implementations of trigonometric functions.
/// 
/// <glm/gtx/fast_trigonometry.hpp> need to be included to use these functionalities.
///////////////////////////////////////////////////////////////////////////////////

#pragma once

// Dependency:
#include "../gtc/constants.hpp"

#if(defined(GLM_MESSAGES) && !defined(GLM_EXT_INCLUDED))
#	pragma message("GLM: GLM_GTX_fast_trigonometry extension included")
#endif

namespace glm
{
	/// @addtogroup gtx_fast_trigonometry
	/// @{

	/// Wrap an angle to [0 2pi[
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T wrapAngle(T angle);

	/// Faster than the common sin function but less accurate.
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T>
	GLM_FUNC_DECL T fastSin(T angle);

	/// Faster than the common cos function but less accurate.
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T fastCos(T angle);

	/// Faster than the common tan function but less accurate. 
	/// Defined between -2pi and 2pi. 
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T fastTan(T angle);

	/// Faster than the common asin function but less accurate. 
	/// Defined between -2pi and 2pi.
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T fastAsin(T angle);

	/// Faster than the common acos function but less accurate. 
	/// Defined between -2pi and 2pi. 
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T fastAcos(T angle);

	/// Faster than the common atan function but less accurate.
	/// Defined between -2pi and 2pi. 
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T fastAtan(T y, T x);

	/// Faster than the common atan function but less accurate. 
	/// Defined between -2pi and 2pi.
	/// From GLM_GTX_fast_trigonometry extension.
	template <typename T> 
	GLM_FUNC_DECL T fastAtan(T angle);

	/// @}
}//namespace glm

#include "fast_trigonometry.inl"


```

## Source file: 3rdparty/glm-0.9.7.0/glm/detail/func_trigonometric.hpp

{
  "path": "3rdparty/glm-0.9.7.0/glm/detail/func_trigonometric.hpp",
  "blob_sha": "d43961bacfc70f9f69a5fd8dc1f6c9e976c1c097",
  "sha256": "64089c025082d6b98a9c1c1d49af729b46167bd861abc9e134b6640b90434d59",
  "lines": 205,
  "bytes": 10997,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 27
  },
  "url": "https://github.com/punkhooren/FaceReconstruction/blob/03b4e83a8287b97e320ccabb4645fc7d09dfabca/3rdparty/glm-0.9.7.0/glm/detail/func_trigonometric.hpp",
  "function_signature_candidates": []
}

```text

///////////////////////////////////////////////////////////////////////////////////
/// OpenGL Mathematics (glm.g-truc.net)
///
/// Copyright (c) 2005 - 2015 G-Truc Creation (www.g-truc.net)
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
/// 
/// The above copyright notice and this permission notice shall be included in
/// all copies or substantial portions of the Software.
/// 
/// Restrictions:
///		By making use of the Software for military purposes, you choose to make
///		a Bunny unhappy.
/// 
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
/// THE SOFTWARE.
///
/// @ref core
/// @file glm/detail/func_trigonometric.hpp
/// @date 2008-08-01 / 2011-06-15
/// @author Christophe Riccio
///
/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
/// 
/// @defgroup core_func_trigonometric Angle and Trigonometry Functions
/// @ingroup core
/// 
/// Function parameters specified as angle are assumed to be in units of radians. 
/// In no case will any of these functions result in a divide by zero error. If 
/// the divisor of a ratio is 0, then results will be undefined.
/// 
/// These all operate component-wise. The description is per component.
///////////////////////////////////////////////////////////////////////////////////

#pragma once

#include "setup.hpp"
#include "precision.hpp"

namespace glm
{
	/// @addtogroup core_func_trigonometric
	/// @{

	/// Converts degrees to radians and returns the result.
	///
	/// @tparam genType Floating-point scalar or vector types.
	///
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/radians.xml">GLSL radians man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> radians(vecType<T, P> const & degrees);

	/// Converts radians to degrees and returns the result.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/degrees.xml">GLSL degrees man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> degrees(vecType<T, P> const & radians);

	/// The standard trigonometric sine function. 
	/// The values returned by this function will range from [-1, 1].
	/// 
	/// @tparam genType Floating-point scalar or vector types.
	///
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/sin.xml">GLSL sin man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> sin(vecType<T, P> const & angle);

	/// The standard trigonometric cosine function. 
	/// The values returned by this function will range from [-1, 1].
	/// 
	/// @tparam genType Floating-point scalar or vector types.
	///
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/cos.xml">GLSL cos man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> cos(vecType<T, P> const & angle);

	/// The standard trigonometric tangent function.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/tan.xml">GLSL tan man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> tan(vecType<T, P> const & angle); 

	/// Arc sine. Returns an angle whose sine is x. 
	/// The range of values returned by this function is [-PI/2, PI/2]. 
	/// Results are undefined if |x| > 1.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/asin.xml">GLSL asin man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> asin(vecType<T, P> const & x);

	/// Arc cosine. Returns an angle whose sine is x. 
	/// The range of values returned by this function is [0, PI]. 
	/// Results are undefined if |x| > 1.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/acos.xml">GLSL acos man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> acos(vecType<T, P> const & x);

	/// Arc tangent. Returns an angle whose tangent is y/x. 
	/// The signs of x and y are used to determine what 
	/// quadrant the angle is in. The range of values returned 
	/// by this function is [-PI, PI]. Results are undefined 
	/// if x and y are both 0. 
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/atan.xml">GLSL atan man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> atan(vecType<T, P> const & y, vecType<T, P> const & x);

	/// Arc tangent. Returns an angle whose tangent is y_over_x. 
	/// The range of values returned by this function is [-PI/2, PI/2].
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/atan.xml">GLSL atan man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> atan(vecType<T, P> const & y_over_x);

	/// Returns the hyperbolic sine function, (exp(x) - exp(-x)) / 2
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/sinh.xml">GLSL sinh man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> sinh(vecType<T, P> const & angle);

	/// Returns the hyperbolic cosine function, (exp(x) + exp(-x)) / 2
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/cosh.xml">GLSL cosh man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> cosh(vecType<T, P> const & angle);

	/// Returns the hyperbolic tangent function, sinh(angle) / cosh(angle)
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/tanh.xml">GLSL tanh man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> tanh(vecType<T, P> const & angle);

	/// Arc hyperbolic sine; returns the inverse of sinh.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/asinh.xml">GLSL asinh man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> asinh(vecType<T, P> const & x);
	
	/// Arc hyperbolic cosine; returns the non-negative inverse
	/// of cosh. Results are undefined if x < 1.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/acosh.xml">GLSL acosh man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> acosh(vecType<T, P> const & x);

	/// Arc hyperbolic tangent; returns the inverse of tanh.
	/// Results are undefined if abs(x) >= 1.
	///
	/// @tparam genType Floating-point scalar or vector types.
	///
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/atanh.xml">GLSL atanh man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL vecType<T, P> atanh(vecType<T, P> const & x);

	/// @}
}//namespace glm

#include "func_trigonometric.inl"


```

## Source file: 3rdparty/glm-0.9.7.0/glm/detail/intrinsic_trigonometric.hpp

{
  "path": "3rdparty/glm-0.9.7.0/glm/detail/intrinsic_trigonometric.hpp",
  "blob_sha": "bf2c9e224255c145b83cb2f0a20708c656e484f1",
  "sha256": "d56cfb80334d9b39874d1dccbb53b19ad61c88c328aa1ac7e37926a927aff281",
  "lines": 46,
  "bytes": 1757,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 6
  },
  "url": "https://github.com/punkhooren/FaceReconstruction/blob/03b4e83a8287b97e320ccabb4645fc7d09dfabca/3rdparty/glm-0.9.7.0/glm/detail/intrinsic_trigonometric.hpp",
  "function_signature_candidates": []
}

```text

///////////////////////////////////////////////////////////////////////////////////
/// OpenGL Mathematics (glm.g-truc.net)
///
/// Copyright (c) 2005 - 2015 G-Truc Creation (www.g-truc.net)
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
/// 
/// The above copyright notice and this permission notice shall be included in
/// all copies or substantial portions of the Software.
/// 
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
/// THE SOFTWARE.
///
/// @ref core
/// @file glm/detail/intrinsic_trigonometric.hpp
/// @date 2009-06-09 / 2011-06-15
/// @author Christophe Riccio
///////////////////////////////////////////////////////////////////////////////////

#pragma once

#include "setup.hpp"

#if(!(GLM_ARCH & GLM_ARCH_SSE2))
#	error "SSE2 instructions not supported or enabled"
#else

namespace glm{
namespace detail
{

}//namespace detail
}//namespace glm

#include "intrinsic_trigonometric.inl"

#endif//GLM_ARCH


```
