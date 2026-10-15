# Facial rendering source scan: jjzhang166/3D-Face-Reconstruction

Repository: https://github.com/jjzhang166/3D-Face-Reconstruction

Pinned source revision: bbab29033aaf9dc3bc76b5968e9fcb6b96a19abe

Discovery topic: face reconstruction 3D

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["3D Face Reconstruction/3D Face Reconstruction/include/dlib/LICENSE.txt", "3D Face Reconstruction/3D Face Reconstruction/include/dlib/external/libpng/LICENSE"]

Topic paths: {"iris": ["3D Face Reconstruction/3D Face Reconstruction/include/dlib/test/create_iris_datafile.cpp", "3D Face Reconstruction/3D Face Reconstruction/include/dlib/test/create_iris_datafile.h"], "rig": ["3D Face Reconstruction/3D Face Reconstruction/include/glm/detail/func_trigonometric.hpp", "3D Face Reconstruction/3D Face Reconstruction/include/glm/detail/func_trigonometric.inl", "3D Face Reconstruction/3D Face Reconstruction/include/glm/detail/intrinsic_trigonometric.hpp", "3D Face Reconstruction/3D Face Reconstruction/include/glm/detail/intrinsic_trigonometric.inl", "3D Face Reconstruction/3D Face Reconstruction/include/glm/gtx/fast_trigonometry.hpp", "3D Face Reconstruction/3D Face Reconstruction/include/glm/gtx/fast_trigonometry.inl", "3D Face Reconstruction/3D Face Reconstruction/include/glm/trigonometric.hpp"], "face": ["3D Face Reconstruction/3D Face Reconstruction.sdf", "3D Face Reconstruction/3D Face Reconstruction.sln", "3D Face Reconstruction/3D Face Reconstruction.v12.suo", "3D Face Reconstruction/3D Face Reconstruction/3D Face Reconstruction.vcxproj", "3D Face Reconstruction/3D Face Reconstruction/3D Face Reconstruction.vcxproj.filters", "3D Face Reconstruction/3D Face Reconstruction/Main.cpp", "3D Face Reconstruction/3D Face Reconstruction/OBJLoader.cpp", "3D Face Reconstruction/3D Face Reconstruction/OBJLoader.h", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/3D Face Reconstruction.lastbuildstate", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/CL.read.1.tlog", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/CL.write.1.tlog", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/cl.command.1.tlog", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/link.command.1.tlog", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/link.read.1.tlog", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face .92CC9A2A.tlog/link.write.1.tlog", "3D Face Reconstruction/3D Face Reconstruction/Release/3D Face Reconstruction.log", "3D Face Reconstruction/3D Face Reconstruction/Release/Main.obj", "3D Face Reconstruction/3D Face Reconstruction/Release/OBJLoader.obj", "3D Face Reconstruction/3D Face Reconstruction/Release/Source.obj", "3D Face Reconstruction/3D Face Reconstruction/Release/vc120.pdb", "3D Face Reconstruction/3D Face Reconstruction/include/GL/glew.h", "3D Face Reconstruction/3D Face Reconstruction/include/GL/glxew.h", "3D Face Reconstruction/3D Face Reconstruction/include/GL/wglew.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_assert.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_atomic.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_audio.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_bits.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_blendmode.h", "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_clipboard.h"], "lip": ["3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_clipboard.h"]}

## Source file: 3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_clipboard.h

{
  "path": "3D Face Reconstruction/3D Face Reconstruction/include/SDL2/SDL_clipboard.h",
  "blob_sha": "a5556f21c4b34a32372d127ee962f24f2c48afc4",
  "sha256": "394b875eb806ce8e3a8dda44d6b4ac54987d0608651ec893fe1ab7d6c2414d64",
  "lines": 71,
  "bytes": 1966,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 4,
    "lip": 14
  },
  "url": "https://github.com/jjzhang166/3D-Face-Reconstruction/blob/bbab29033aaf9dc3bc76b5968e9fcb6b96a19abe/3D%20Face%20Reconstruction/3D%20Face%20Reconstruction/include/SDL2/SDL_clipboard.h",
  "function_signature_candidates": []
}

```text

/*
  Simple DirectMedia Layer
  Copyright (C) 1997-2016 Sam Lantinga <slouken@libsdl.org>

  This software is provided 'as-is', without any express or implied
  warranty.  In no event will the authors be held liable for any damages
  arising from the use of this software.

  Permission is granted to anyone to use this software for any purpose,
  including commercial applications, and to alter it and redistribute it
  freely, subject to the following restrictions:

  1. The origin of this software must not be misrepresented; you must not
     claim that you wrote the original software. If you use this software
     in a product, an acknowledgment in the product documentation would be
     appreciated but is not required.
  2. Altered source versions must be plainly marked as such, and must not be
     misrepresented as being the original software.
  3. This notice may not be removed or altered from any source distribution.
*/

/**
 * \file SDL_clipboard.h
 *
 * Include file for SDL clipboard handling
 */

#ifndef _SDL_clipboard_h
#define _SDL_clipboard_h

#include "SDL_stdinc.h"

#include "begin_code.h"
/* Set up for C function definitions, even when using C++ */
#ifdef __cplusplus
extern "C" {
#endif

/* Function prototypes */

/**
 * \brief Put UTF-8 text into the clipboard
 *
 * \sa SDL_GetClipboardText()
 */
extern DECLSPEC int SDLCALL SDL_SetClipboardText(const char *text);

/**
 * \brief Get UTF-8 text from the clipboard, which must be freed with SDL_free()
 *
 * \sa SDL_SetClipboardText()
 */
extern DECLSPEC char * SDLCALL SDL_GetClipboardText(void);

/**
 * \brief Returns a flag indicating whether the clipboard exists and contains a text string that is non-empty
 *
 * \sa SDL_GetClipboardText()
 */
extern DECLSPEC SDL_bool SDLCALL SDL_HasClipboardText(void);


/* Ends C function definitions when using C++ */
#ifdef __cplusplus
}
#endif
#include "close_code.h"

#endif /* _SDL_clipboard_h */

/* vi: set ts=4 sw=4 expandtab: */


```

## Source file: 3D Face Reconstruction/3D Face Reconstruction/include/glm/trigonometric.hpp

{
  "path": "3D Face Reconstruction/3D Face Reconstruction/include/glm/trigonometric.hpp",
  "blob_sha": "21ba20a194a5835e2a4d74a06828b14c6dba6ba9",
  "sha256": "8bb13f53dbd09817905943a4ce339b858804b69cac8fabfcd88a63e4cdde1013",
  "lines": 35,
  "bytes": 1658,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 6
  },
  "url": "https://github.com/jjzhang166/3D-Face-Reconstruction/blob/bbab29033aaf9dc3bc76b5968e9fcb6b96a19abe/3D%20Face%20Reconstruction/3D%20Face%20Reconstruction/include/glm/trigonometric.hpp",
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

## Source file: 3D Face Reconstruction/3D Face Reconstruction/include/glm/gtx/fast_trigonometry.hpp

{
  "path": "3D Face Reconstruction/3D Face Reconstruction/include/glm/gtx/fast_trigonometry.hpp",
  "blob_sha": "93c38ab28f76bc06ddf734158171af2e2f282cf9",
  "sha256": "7e23c9e1dd12bbb150f1aea652ba11a656f9917872d452a280a24b7f92fc6c59",
  "lines": 104,
  "bytes": 3734,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 20
  },
  "url": "https://github.com/jjzhang166/3D-Face-Reconstruction/blob/bbab29033aaf9dc3bc76b5968e9fcb6b96a19abe/3D%20Face%20Reconstruction/3D%20Face%20Reconstruction/include/glm/gtx/fast_trigonometry.hpp",
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

## Source file: 3D Face Reconstruction/3D Face Reconstruction/include/glm/detail/func_trigonometric.hpp

{
  "path": "3D Face Reconstruction/3D Face Reconstruction/include/glm/detail/func_trigonometric.hpp",
  "blob_sha": "244349775db27b0cdf2144c5bbd9577c89edeff2",
  "sha256": "ec97f41e6337f732b5da5627b94234677e452c6e93f4f12ccab4fa306e4b6a5b",
  "lines": 205,
  "bytes": 11025,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 27
  },
  "url": "https://github.com/jjzhang166/3D-Face-Reconstruction/blob/bbab29033aaf9dc3bc76b5968e9fcb6b96a19abe/3D%20Face%20Reconstruction/3D%20Face%20Reconstruction/include/glm/detail/func_trigonometric.hpp",
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
	GLM_FUNC_DECL GLM_CONSTEXPR vecType<T, P> radians(vecType<T, P> const & degrees);

	/// Converts radians to degrees and returns the result.
	///
	/// @tparam genType Floating-point scalar or vector types.
	/// 
	/// @see <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/degrees.xml">GLSL degrees man page</a>
	/// @see <a href="http://www.opengl.org/registry/doc/GLSLangSpec.4.20.8.pdf">GLSL 4.20.8 specification, section 8.1 Angle and Trigonometry Functions</a>
	template <typename T, precision P, template <typename, precision> class vecType>
	GLM_FUNC_DECL GLM_CONSTEXPR vecType<T, P> degrees(vecType<T, P> const & radians);

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
