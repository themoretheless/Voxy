# Facial rendering source scan: jhaberstro/subsurface-scattering

Repository: https://github.com/jhaberstro/subsurface-scattering

Pinned source revision: 2574c54fe16fa1939cea13219c655d9dd307f681

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"subsurface": ["subsurface-scattering.xcodeproj/project.pbxproj", "subsurface-scattering.xcodeproj/project.xcworkspace/contents.xcworkspacedata", "subsurface-scattering.xcodeproj/project.xcworkspace/xcuserdata/jhaberstro.xcuserdatad/UserInterfaceState.xcuserstate", "subsurface-scattering.xcodeproj/xcuserdata/jhaberstro.xcuserdatad/xcschemes/subsurface-scattering.xcscheme", "subsurface-scattering.xcodeproj/xcuserdata/jhaberstro.xcuserdatad/xcschemes/xcschememanagement.plist", "subsurface-scattering/.DS_Store", "subsurface-scattering/glm/.DS_Store", "subsurface-scattering/glm/core/_detail.hpp", "subsurface-scattering/glm/core/_fixes.hpp", "subsurface-scattering/glm/core/_swizzle.hpp", "subsurface-scattering/glm/core/_swizzle.inl", "subsurface-scattering/glm/core/dummy.cpp", "subsurface-scattering/glm/core/func_common.hpp", "subsurface-scattering/glm/core/func_common.inl", "subsurface-scattering/glm/core/func_exponential.hpp", "subsurface-scattering/glm/core/func_exponential.inl", "subsurface-scattering/glm/core/func_geometric.hpp", "subsurface-scattering/glm/core/func_geometric.inl", "subsurface-scattering/glm/core/func_integer.hpp", "subsurface-scattering/glm/core/func_integer.inl", "subsurface-scattering/glm/core/func_matrix.hpp", "subsurface-scattering/glm/core/func_matrix.inl", "subsurface-scattering/glm/core/func_noise.hpp", "subsurface-scattering/glm/core/func_noise.inl", "subsurface-scattering/glm/core/func_packing.hpp", "subsurface-scattering/glm/core/func_packing.inl", "subsurface-scattering/glm/core/func_trigonometric.hpp", "subsurface-scattering/glm/core/func_trigonometric.inl", "subsurface-scattering/glm/core/func_vector_relational.hpp", "subsurface-scattering/glm/core/func_vector_relational.inl"], "sss": ["subsurface-scattering/shaders/sss.fsh", "subsurface-scattering/shaders/sss.vsh"], "shader": ["subsurface-scattering/shaders/normal.fsh", "subsurface-scattering/shaders/normal.vsh", "subsurface-scattering/shaders/sss.fsh", "subsurface-scattering/shaders/sss.vsh", "subsurface-scattering/shaders/thickness.fsh", "subsurface-scattering/shaders/thickness.vsh"], "rig": ["subsurface-scattering/glm/core/func_trigonometric.hpp", "subsurface-scattering/glm/core/func_trigonometric.inl", "subsurface-scattering/glm/core/intrinsic_trigonometric.hpp", "subsurface-scattering/glm/core/intrinsic_trigonometric.inl", "subsurface-scattering/glm/gtx/fast_trigonometry.hpp", "subsurface-scattering/glm/gtx/fast_trigonometry.inl"], "face": ["subsurface-scattering.xcodeproj/project.pbxproj", "subsurface-scattering.xcodeproj/project.xcworkspace/contents.xcworkspacedata", "subsurface-scattering.xcodeproj/project.xcworkspace/xcuserdata/jhaberstro.xcuserdatad/UserInterfaceState.xcuserstate", "subsurface-scattering.xcodeproj/xcuserdata/jhaberstro.xcuserdatad/xcschemes/subsurface-scattering.xcscheme", "subsurface-scattering.xcodeproj/xcuserdata/jhaberstro.xcuserdatad/xcschemes/xcschememanagement.plist", "subsurface-scattering/.DS_Store", "subsurface-scattering/glm/.DS_Store", "subsurface-scattering/glm/core/_detail.hpp", "subsurface-scattering/glm/core/_fixes.hpp", "subsurface-scattering/glm/core/_swizzle.hpp", "subsurface-scattering/glm/core/_swizzle.inl", "subsurface-scattering/glm/core/dummy.cpp", "subsurface-scattering/glm/core/func_common.hpp", "subsurface-scattering/glm/core/func_common.inl", "subsurface-scattering/glm/core/func_exponential.hpp", "subsurface-scattering/glm/core/func_exponential.inl", "subsurface-scattering/glm/core/func_geometric.hpp", "subsurface-scattering/glm/core/func_geometric.inl", "subsurface-scattering/glm/core/func_integer.hpp", "subsurface-scattering/glm/core/func_integer.inl", "subsurface-scattering/glm/core/func_matrix.hpp", "subsurface-scattering/glm/core/func_matrix.inl", "subsurface-scattering/glm/core/func_noise.hpp", "subsurface-scattering/glm/core/func_noise.inl", "subsurface-scattering/glm/core/func_packing.hpp", "subsurface-scattering/glm/core/func_packing.inl", "subsurface-scattering/glm/core/func_trigonometric.hpp", "subsurface-scattering/glm/core/func_trigonometric.inl", "subsurface-scattering/glm/core/func_vector_relational.hpp", "subsurface-scattering/glm/core/func_vector_relational.inl"]}

## Source file: subsurface-scattering/glm/gtx/fast_trigonometry.hpp

{
  "path": "subsurface-scattering/glm/gtx/fast_trigonometry.hpp",
  "blob_sha": "aae9ec2f1f22522fb9a65320b53f2b0796cd62f9",
  "sha256": "5c5a9461c5094eb9ae239523fe83f69aec7b9b67792d05cfd523c46edc7fe42f",
  "lines": 88,
  "bytes": 2945,
  "selection_score": 21,
  "keyword_line_counts": {
    "rig": 20
  },
  "url": "https://github.com/jhaberstro/subsurface-scattering/blob/2574c54fe16fa1939cea13219c655d9dd307f681/subsurface-scattering/glm/gtx/fast_trigonometry.hpp",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "void main_gtx_fast_trigonometry();"
    }
  ]
}

```text

///////////////////////////////////////////////////////////////////////////////////////////////////
// OpenGL Mathematics Copyright (c) 2005 - 2011 G-Truc Creation (www.g-truc.net)
///////////////////////////////////////////////////////////////////////////////////////////////////
// Created : 2006-01-08
// Updated : 2006-11-13
// Licence : This source is under MIT License
// File    : glm/gtx/fast_trigonometry.hpp
///////////////////////////////////////////////////////////////////////////////////////////////////
// Dependency:
// - GLM core
///////////////////////////////////////////////////////////////////////////////////////////////////

#ifndef glm_gtx_fast_trigonometry
#define glm_gtx_fast_trigonometry

// Dependency:
#include "../glm.hpp"

#if(defined(GLM_MESSAGES) && !defined(glm_ext))
#	pragma message("GLM: GLM_GTX_fast_trigonometry extension included")
#endif

namespace glm
{
   	namespace test{
		void main_gtx_fast_trigonometry();
	}//namespace test

    namespace gtx{
	//! GLM_GTX_fast_trigonometry extension: Fast but less accurate implementations of trigonometric functions.
    namespace fast_trigonometry
    {
		/// \addtogroup gtx_fast_trigonometry
		///@{

		//! Faster than the common sin function but less accurate. 
		//! Defined between -2pi and 2pi. 
		//! From GLM_GTX_fast_trigonometry extension.
        template <typename T> 
		T fastSin(const T& angle);

        //! Faster than the common cos function but less accurate.
		//! Defined between -2pi and 2pi.
		//! From GLM_GTX_fast_trigonometry extension.
		template <typename T> 
		T fastCos(const T& angle);

        //! Faster than the common tan function but less accurate. 
		//! Defined between -2pi and 2pi. 
		//! From GLM_GTX_fast_trigonometry extension.
		template <typename T> 
		T fastTan(const T& angle);

        //! Faster than the common asin function but less accurate. 
		//! Defined between -2pi and 2pi.
		//! From GLM_GTX_fast_trigonometry extension.
		template <typename T> 
		T fastAsin(const T& angle);

		//! Faster than the common acos function but less accurate. 
		//! Defined between -2pi and 2pi. 
		//! From GLM_GTX_fast_trigonometry extension.
        template <typename T> 
		T fastAcos(const T& angle);

		//! Faster than the common atan function but less accurate.
		//! Defined between -2pi and 2pi. 
		//! From GLM_GTX_fast_trigonometry extension.
		template <typename T> 
		T fastAtan(const T& y, const T& x);

		//! Faster than the common atan function but less accurate. 
		//! Defined between -2pi and 2pi.
		//! From GLM_GTX_fast_trigonometry extension.
        template <typename T> 
		T fastAtan(const T& angle);

		///@}

    }//namespace fast_trigonometry
    }//namespace gtx
}//namespace glm

#include "fast_trigonometry.inl"

namespace glm{using namespace gtx::fast_trigonometry;}

#endif//glm_gtx_fast_trigonometry


```

## Source file: subsurface-scattering/glm/core/func_trigonometric.hpp

{
  "path": "subsurface-scattering/glm/core/func_trigonometric.hpp",
  "blob_sha": "d97a78bd70adca9908cba281f470fb4d325f9051",
  "sha256": "5e47a3ad1fec9ed6dc5da6c853f3987b13cabecbc71ebd680808e67fac94eb81",
  "lines": 160,
  "bytes": 6325,
  "selection_score": 21,
  "keyword_line_counts": {
    "rig": 14
  },
  "url": "https://github.com/jhaberstro/subsurface-scattering/blob/2574c54fe16fa1939cea13219c655d9dd307f681/subsurface-scattering/glm/core/func_trigonometric.hpp",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "void main_core_func_trigonometric();"
    }
  ]
}

```text

///////////////////////////////////////////////////////////////////////////////////////////////////
// OpenGL Mathematics Copyright (c) 2005 - 2011 G-Truc Creation (www.g-truc.net)
///////////////////////////////////////////////////////////////////////////////////////////////////
// Created : 2008-08-01
// Updated : 2008-09-10
// Licence : This source is under MIT License
// File    : glm/core/func_trigonometric.hpp
///////////////////////////////////////////////////////////////////////////////////////////////////

#ifndef glm_core_func_trigonometric
#define glm_core_func_trigonometric

namespace glm
{
	namespace test{
		void main_core_func_trigonometric();
	}//namespace test

	namespace core{
	namespace function{
	//! Define Angle and trigonometry functions 
	//! from Section 8.1 of GLSL 1.30.8 specification. 
	//! Included in glm namespace.
	namespace trigonometric{

	/// \addtogroup core_funcs
	///@{

	//! Converts degrees to radians and returns the result.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/radians.xml">GLSL radians man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType radians(genType const & degrees);

	//! Converts radians to degrees and returns the result.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/degrees.xml">GLSL degrees man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType degrees(genType const & radians);

	//! The standard trigonometric sine function. 
	//! The values returned by this function will range from [-1, 1].
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/sin.xml">GLSL sin man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType sin(genType const & angle);

	//! The standard trigonometric cosine function. 
	//! The values returned by this function will range from [-1, 1].
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/cos.xml">GLSL cos man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType cos(genType const & angle);

	//! The standard trigonometric tangent function.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/tan.xml">GLSL tan man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType tan(genType const & angle); 

	//! Arc sine. Returns an angle whose sine is x. 
	//! The range of values returned by this function is [-PI/2, PI/2]. 
	//! Results are undefined if |x| > 1.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/asin.xml">GLSL asin man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType asin(genType const & x);

	//! Arc cosine. Returns an angle whose sine is x. 
	//! The range of values returned by this function is [0, PI]. 
	//! Results are undefined if |x| > 1.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/acos.xml">GLSL acos man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType acos(genType const & x);

	//! Arc tangent. Returns an angle whose tangent is y/x. 
	//! The signs of x and y are used to determine what 
	//! quadrant the angle is in. The range of values returned 
	//! by this function is [-PI, PI]. Results are undefined 
	//! if x and y are both 0. 
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/atan.xml">GLSL atan man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType atan(genType const & y, genType const & x);

	//! Arc tangent. Returns an angle whose tangent is y_over_x. 
	//! The range of values returned by this function is [-PI/2, PI/2].
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/atan.xml">GLSL atan man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType atan(genType const & y_over_x);

	//! Returns the hyperbolic sine function, (exp(x) - exp(-x)) / 2
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/sinh.xml">GLSL sinh man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType sinh(genType const & angle);

	//! Returns the hyperbolic cosine function, (exp(x) + exp(-x)) / 2
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/cosh.xml">GLSL cosh man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType cosh(genType const & angle);

	//! Returns the hyperbolic tangent function, sinh(angle) / cosh(angle)
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/tanh.xml">GLSL tanh man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType tanh(genType const & angle);

	//! Arc hyperbolic sine; returns the inverse of sinh.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/asinh.xml">GLSL asinh man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType asinh(genType const & x);
	
	//! Arc hyperbolic cosine; returns the non-negative inverse
	//! of cosh. Results are undefined if x < 1.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/acosh.xml">GLSL acosh man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType acosh(genType const & x);

	//! Arc hyperbolic tangent; returns the inverse of tanh.
	//! Results are undefined if abs(x) >= 1.
    //!
    //! \li <a href="http://www.opengl.org/sdk/docs/manglsl/xhtml/atanh.xml">GLSL atanh man page</a>
    //! \li GLSL 1.30.08 specification, section 8.1	
	template <typename genType> 
	genType atanh(genType const & x);

	///@}

	}//namespace trigonometric
	}//namespace function
	}//namespace core

	using namespace core::function::trigonometric;
}//namespace glm

#include "func_trigonometric.inl"

#endif//glm_core_func_trigonometric




```

## Source file: subsurface-scattering/glm/core/intrinsic_trigonometric.hpp

{
  "path": "subsurface-scattering/glm/core/intrinsic_trigonometric.hpp",
  "blob_sha": "9b69bf749501c93ea9bebdfb682fbc4be89379d1",
  "sha256": "0c9f21aa171654c30b93fde22961376c22e175f103c108c60a5545ce8bd0131c",
  "lines": 29,
  "bytes": 946,
  "selection_score": 21,
  "keyword_line_counts": {
    "rig": 6
  },
  "url": "https://github.com/jhaberstro/subsurface-scattering/blob/2574c54fe16fa1939cea13219c655d9dd307f681/subsurface-scattering/glm/core/intrinsic_trigonometric.hpp",
  "function_signature_candidates": []
}

```text

///////////////////////////////////////////////////////////////////////////////////////////////////
// OpenGL Mathematics Copyright (c) 2005 - 2011 G-Truc Creation (www.g-truc.net)
///////////////////////////////////////////////////////////////////////////////////////////////////
// Created : 2009-06-09
// Updated : 2009-06-09
// Licence : This source is under MIT License
// File    : glm/core/intrinsic_trigonometric.hpp
///////////////////////////////////////////////////////////////////////////////////////////////////

#ifndef glm_detail_intrinsic_trigonometric
#define glm_detail_intrinsic_trigonometric

#include "setup.hpp"

#if((GLM_ARCH & GLM_ARCH_SSE2) != GLM_ARCH_SSE2)
#	error "SSE2 instructions not supported or enabled"
#else

namespace glm{
namespace detail
{

}//namespace detail
}//namespace glm

#include "intrinsic_trigonometric.inl"

#endif//GLM_ARCH
#endif//glm_detail_intrinsic_trigonometric


```

## Source file: subsurface-scattering/main.cpp

{
  "path": "subsurface-scattering/main.cpp",
  "blob_sha": "58a650bb9942f2788ab2c96d76d748ca652c64ac",
  "sha256": "1f9f7a0765c07bc7c5ed8e250e7a594fa765ea8a24398c51de4a6b30b22fefd4",
  "lines": 313,
  "bytes": 12160,
  "selection_score": 6,
  "keyword_line_counts": {
    "sss": 19,
    "shader": 3,
    "rig": 6,
    "face": 3
  },
  "url": "https://github.com/jhaberstro/subsurface-scattering/blob/2574c54fe16fa1939cea13219c655d9dd307f681/subsurface-scattering/main.cpp",
  "function_signature_candidates": [
    {
      "line": 51,
      "text": "void create_fbo(GLuint& fbo, GLuint& depth_texture, GLenum internalFormat, GLenum format) {"
    },
    {
      "line": 102,
      "text": "void draw_quad(GLuint vbo, GLuint pos_attrib, GLuint tex_attrib) {"
    },
    {
      "line": 114,
      "text": "void draw_lighted_mesh(mesh* mesh, glm::vec3 const& ambient, glm::vec3 const& diffuse, glm::mat4 const& mv, glm::mat4 const& p) {"
    },
    {
      "line": 149,
      "text": "void initialize() {"
    },
    {
      "line": 170,
      "text": "void draw() {"
    },
    {
      "line": 208,
      "text": "glm::vec4 transformed_light_pos(0.0f, 0.0f, 0.0f, 1.0f);"
    },
    {
      "line": 273,
      "text": "void PezRender() {"
    },
    {
      "line": 277,
      "text": "void PezUpdate(unsigned int milliseconds) { }"
    },
    {
      "line": 279,
      "text": "void PezHandleMouse(int x, int y, int action) {"
    },
    {
      "line": 289,
      "text": "void PezHandleKeyDown(int key) {"
    },
    {
      "line": 312,
      "text": "void PezHandleKeyUp(int key) { }"
    }
  ]
}

```text

//
//  main.cpp
//  sub-surface scattering
//
//  Created by Jedd Haberstro on 5/17/11.
//  Copyright 2011 Student. All rights reserved.
//

#include <SFML/Window.hpp>
#include "mesh.hpp"
#include "utilities.h"
#include "openctm.h"

#include "glm/glm.hpp"
#include "glm/ext.hpp"

#include "pez/pez.h"

enum
{
    WINDOW_WIDTH = PEZ_VIEWPORT_WIDTH,
    WINDOW_HEIGHT = PEZ_VIEWPORT_HEIGHT
};

enum
{
    ATTRIB_VERTEX,
    ATTRIB_TEXCOORD,
    ATTRIB_NORMAL,
    NUM_ATTRIBUTES
};


GLuint thickness_program;
GLuint sss_program;
GLuint normal_program;
GLuint scene_fbo;
GLuint scene_texture;
GLuint quad_vbo;
float obj_rotation = 45.0f;
float light_rotation = 0.0f;
float obj_scale = 1.0f;
mesh* dragon_mesh;
mesh* buddha_mesh;
mesh* sphere_mesh;
glm::vec3 obj_position;
glm::vec3 light_position;
glm::vec3 light_translation;


void create_fbo(GLuint& fbo, GLuint& depth_texture, GLenum internalFormat, GLenum format) {
    glGenFramebuffers(1, &fbo);
    glBindFramebuffer(GL_FRAMEBUFFER, fbo);
        
    glGenTextures(1, &depth_texture);
    glBindTexture(GL_TEXTURE_2D, depth_texture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexImage2D(GL_TEXTURE_2D, 0, internalFormat, WINDOW_WIDTH, WINDOW_HEIGHT, 0, format, GL_FLOAT, 0);
    //glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, WINDOW_WIDTH, WINDOW_HEIGHT, 0, GL_RGB, GL_UNSIGNED_BYTE, 0);
    assert(GL_NO_ERROR == glGetError());
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, depth_texture, 0);
    
    //GLuint depthRenderbuffer;
    //glGenRenderbuffers(1, &depthRenderbuffer);
    //glBindRenderbuffer(GL_RENDERBUFFER, depthRenderbuffer);
    //glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH_COMPONENT16, WINDOW_WIDTH, WINDOW_HEIGHT);
    //glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, depthRenderbuffer);
    
    GLenum status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
    if(status != GL_FRAMEBUFFER_COMPLETE) {
        printf("failed to make complete framebuffer object %x", status);
        //assert(false);
    }
    
    glBindFramebuffer(GL_FRAMEBUFFER, 0);
    glBindRenderbuffer(GL_RENDERBUFFER, 0);
}

GLuint create_quad(float left, float bottom, float right, float top)
{
    float quad[] = {
        left, bottom, 0, 0,
        left, top, 0, 1,
        right, top, 1, 1,
        right, top, 1, 1,
        right, bottom, 1, 0,
        left, bottom, 0, 0
    };
    
    GLuint handle;
    glGenBuffers(1, &handle);
    glBindBuffer(GL_ARRAY_BUFFER, handle);
    glBufferData(GL_ARRAY_BUFFER, sizeof(quad), &quad[0], GL_STATIC_DRAW);
    glBindBuffer(GL_ARRAY_BUFFER, 0);
    
    return handle;
}

void draw_quad(GLuint vbo, GLuint pos_attrib, GLuint tex_attrib) {
    glBindBuffer(GL_ARRAY_BUFFER, vbo);
    glEnableVertexAttribArray(pos_attrib);
    glEnableVertexAttribArray(tex_attrib);
    glVertexAttribPointer(pos_attrib, 2, GL_FLOAT, GL_FALSE, sizeof(float) * 4, 0);
    glVertexAttribPointer(tex_attrib, 2, GL_FLOAT, GL_FALSE, sizeof(float) * 4, (GLvoid*)(sizeof(float) * 2));
    glDrawArrays(GL_TRIANGLES, 0, 6);
    
    glDisableVertexAttribArray(pos_attrib);
    glDisableVertexAttribArray(tex_attrib);
}

void draw_lighted_mesh(mesh* mesh, glm::vec3 const& ambient, glm::vec3 const& diffuse, glm::mat4 const& mv, glm::mat4 const& p) {
    glUseProgram(normal_program);
    
    glm::mat3 normal_matrix = glm::transpose(glm::inverse(glm::mat3(mv)));
    
    int modelview_uniform = glGetUniformLocation(normal_program, "modelview");
    int projection_uniform = glGetUniformLocation(normal_program, "projection");
    int normal_matrix_uniform = glGetUniformLocation(normal_program, "normal_matrix");
    int light_pos_uniform = glGetUniformLocation(normal_program, "light_pos");
    int diffuse_uniform = glGetUniformLocation(normal_program, "diffuse");
    int ambient_uniform = glGetUniformLocation(normal_program, "ambient");
    int specular_uniform = glGetUniformLocation(normal_program, "specular");
    int shininess_uniform = glGetUniformLocation(normal_program, "shininess");
    
    glUniformMatrix4fv(modelview_uniform, 1, GL_FALSE, &mv[0][0]);
    glUniformMatrix4fv(projection_uniform, 1, GL_FALSE, &p[0][0]);
    glUniformMatrix3fv(normal_matrix_uniform, 1, GL_FALSE, &normal_matrix[0][0]);
    glUniform3f(light_pos_uniform, 0.25f, 0.25f, 1.0f);
    glUniform3f(diffuse_uniform, diffuse.r, diffuse.g, diffuse.b);
    glUniform3f(ambient_uniform, ambient.r, ambient.g, ambient.b);
    glUniform3f(specular_uniform, 0.5f, 0.5f, 0.5f);
    glUniform1f(shininess_uniform, 50.0f);
    
    glEnable(GL_DEPTH_TEST);
    glDisable(GL_BLEND);
    
    draw_mesh(mesh, ATTRIB_VERTEX, ATTRIB_NORMAL);  
    
    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, 0);
    glBindBuffer(GL_ARRAY_BUFFER, 0);
    glDisable(GL_DEPTH_TEST);
    glDisableVertexAttribArray(ATTRIB_VERTEX);
    glDisableVertexAttribArray(ATTRIB_NORMAL);
}

void initialize() {    
    attribute thickness_attribs[] = { { ATTRIB_VERTEX, "position" }, { ATTRIB_NORMAL, "normal" } };
    thickness_program = load_shader("thickness.vsh", "thickness.fsh", thickness_attribs, 2);

    attribute sss_attribs[] = { { ATTRIB_VERTEX, "position" }, { ATTRIB_NORMAL, "normal" } };
    sss_program = load_shader("sss.vsh", "sss.fsh", sss_attribs, 2);
    
    attribute normal_attribs[] = { { ATTRIB_VERTEX, "position" }, { ATTRIB_NORMAL, "normal" } };
    normal_program = load_shader("normal.vsh", "normal.fsh", normal_attribs, 2);
    
    create_fbo(scene_fbo, scene_texture, GL_RGB32F_ARB, GL_RGB);
    quad_vbo = create_quad(-1.0f, -1.0f, 1.0f, 1.0f);
    dragon_mesh = load_mesh_ctm("dragon.ctm");      
    buddha_mesh = load_mesh_ctm("buddha.ctm");
    sphere_mesh = load_mesh_ctm("sphere.ctm");
    
    obj_position = glm::vec3(-0.6f, -0.2f, -2.3f);
    light_position = glm::vec3(-0.6f, 0.3f, -2.7f);
    light_translation = glm::vec3(0.0, 0.0025f, 0.0f);
}

void draw() {
    float x = 0.6f;
    float y = x * WINDOW_HEIGHT / WINDOW_WIDTH;
    glm::mat4 perspective = glm::frustum(-x, x, -y, y, 1.0f, 100.0f);
    glm::mat4 translation1 = glm::translate(obj_position);
    glm::mat4 rot = glm::rotate(obj_rotation, 0.0f, 1.0f, 0.0f);
    rot = glm::rotate(rot, 270.0f, 2.0f, 0.0f, 0.0f);
    
    // Draw thicknss into scene_texture
    {
        glBindFramebuffer(GL_FRAMEBUFFER, scene_fbo);
        glClearColor(0.0f, 0.0f, 0.0f, 0.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        
        glEnable(GL_BLEND);
        glBlendFunc(GL_ONE, GL_ONE);
        
        glUseProgram(thickness_program);
        int modelview_uniform = glGetUniformLocation(thickness_program, "modelview");
        int projection_uniform = glGetUniformLocation(thickness_program, "projection");
        
        glm::mat4 modelview = translation1 * rot;
        modelview = glm::scale(modelview, glm::vec3(1.0f, obj_scale, 1.0f));
        glUniformMatrix4fv(modelview_uniform, 1, GL_FALSE, &modelview[0][0]);
        glUniformMatrix4fv(projection_uniform, 1, GL_FALSE, &perspective[0][0]);
        draw_mesh(buddha_mesh, ATTRIB_VERTEX, ATTRIB_NORMAL);
        
        glDisable(GL_BLEND);
        //save_framebuffer("out.raw");
    }
    
    // Second pass to perform sub-surface scattering 
    {
        glBindFramebuffer(GL_FRAMEBUFFER, 0); // 0 is the default buffer
        glClearColor(0.0f, 0.0f, 0.0f, 0.0f);
        //glClearColor(0.9f, 0.9f, 0.9f, 0.9f);
        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        
        glm::vec4 transformed_light_pos(0.0f, 0.0f, 0.0f, 1.0f);
        // Draw the light
        {
            glm::mat4 model = glm::translate(light_position);
            model = glm::translate(model, obj_position - light_position);
            model = model * glm::rotate(light_rotation, 0.0f, 1.0f, 0.0f);
            model = glm::translate(model, -(obj_position - light_position));
            transformed_light_pos = model * transformed_light_pos;
            draw_lighted_mesh(sphere_mesh, glm::vec3(1.0f), glm::vec3(0.0f), model, perspective);
        }
        
        glUseProgram(sss_program);
        int modelview_uniform = glGetUniformLocation(sss_program, "modelview");
        int projection_uniform = glGetUniformLocation(sss_program, "projection");
        int light_pos_uniform = glGetUniformLocation(sss_program, "light_pos");
        int thickness_texture_uniform = glGetUniformLocation(sss_program, "thickness_texture");
        int mat_diffuse_uniform = glGetUniformLocation(sss_program, "mat_diffuse");
        int light_diffuse_uniform = glGetUniformLocation(sss_program, "light_diffuse");
        int ambient_color_uniform = glGetUniformLocation(sss_program, "ambient_color");
        int specular_uniform = glGetUniformLocation(sss_program, "specular");
        int shininess_uniform = glGetUniformLocation(sss_program, "shininess");
        int ambient_uniform = glGetUniformLocation(sss_program, "ambient");
        int power_uniform = glGetUniformLocation(sss_program, "power");
        int distortion_uniform = glGetUniformLocation(sss_program, "distortion");
        int scale_uniform = glGetUniformLocation(sss_program, "scale");
        int screen_size_uniform = glGetUniformLocation(sss_program, "screen_size");
        int light_radius_uniform = glGetUniformLocation(sss_program, "light_radius");
        
        glm::mat4 modelview = translation1 * rot;
        modelview = glm::scale(modelview, glm::vec3(1.0f, obj_scale, 1.0f));
        glUniformMatrix4fv(modelview_uniform, 1, GL_FALSE, &modelview[0][0]);
        glUniformMatrix4fv(projection_uniform, 1, GL_FALSE, &perspective[0][0]);
        glUniform4f(light_pos_uniform, transformed_light_pos.x, transformed_light_pos.y, transformed_light_pos.z, 1.0f); 
        float factor = 1.0f;
        //glUniform4f(mat_diffuse_uniform, 0.639f * factor, 0.498f * factor, 0.341f * factor, 1.0f);
        glUniform4f(mat_diffuse_uniform, 0.0f, 0.45f, 0.75f, 1.0f);
        glUniform4f(light_diffuse_uniform, 1.0f, 1.0f, 1.0f, 1.0f);
        //glUniform4f(light_diffuse_uniform, 0.3f, 0.8f, 0.2f, 1.0f);
        glUniform3f(ambient_color_uniform, 0.04f, 0.04f, 0.04f);
        glUniform3f(specular_uniform, 0.0f, 0.0f, 0.0f); // (0.5, 0.5, 0.5)
        glUniform1f(shininess_uniform, 200.0f);
        glUniform1f(ambient_uniform, 2.0f); // Play around with this
        glUniform1i(power_uniform, 4.0f);
        glUniform1f(distortion_uniform, 0.2f);
        glUniform1f(scale_uniform, 15.0f);
        glUniform2f(screen_size_uniform, WINDOW_WIDTH, WINDOW_HEIGHT);
        glUniform1f(light_radius_uniform, 1.0f);
        
        glActiveTexture(GL_TEXTURE0);
        glBindTexture(GL_TEXTURE_2D, scene_texture);
        glUniform1i(thickness_texture_uniform, 0);
        
        glEnable(GL_DEPTH_TEST);
        glDisable(GL_BLEND);
        draw_mesh(buddha_mesh, ATTRIB_VERTEX, ATTRIB_NORMAL);
        glBindTexture(GL_TEXTURE_2D, 0);
        glDisable(GL_DEPTH_TEST);         
    }
}

const char* PezInitialize(int width, int height) {
    initialize();
    return "Sub-surface Scattering";
}

void PezRender() {
    draw();
}

void PezUpdate(unsigned int milliseconds) { }

void PezHandleMouse(int x, int y, int action) {
    static int lastx = 0.0f;
    static int lasty = 0.0f;
    int deltax = x - lastx;
    int deltay = y - lasty;
    obj_rotation += deltax;
    lastx = x;
    lasty = y;
}

void PezHandleKeyDown(int key) {
    static float const kTranslation = 0.025f;
    if (key == KEY_UPARROW) {
        light_position.y += kTranslation;
    }
    else if (key == KEY_DOWNARROW) {
        light_position.y -= kTranslation;
    }
    else if (key == KEY_LEFTARROW) {
        light_position.x -= kTranslation;
    }
    else if (key == KEY_RIGHTARROW) {
        light_position.x += kTranslation;
    }
    else if (key == KEY_W) {
        obj_scale += 0.05f;
    }
    else if (key == KEY_S) {
        obj_scale -= 0.05f;
    }
    
}

void PezHandleKeyUp(int key) { }



```
