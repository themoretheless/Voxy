# Facial rendering source scan: thuhcsi/Crystal.TTVS

Repository: https://github.com/thuhcsi/Crystal.TTVS

Pinned source revision: 9e6c86566bfacf4fc571ad7b1308bcb7382bb213

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["data/talkinghead/Song/eyeblow.tga"], "teeth": ["data/talkinghead/Song/teeth.tga"], "tongue": ["data/talkinghead/Song/tongue.tga"], "throat": ["data/talkinghead/Song/throat.tga"], "deform": ["Xface/XFace/include/XEngine/DeformableGeometry.h", "Xface/XFace/src/XEngine/DeformableGeometry.cpp"], "rig": ["data/talkinghead/Song/pupil_right.tga"], "face": ["Xface/Changes.txt", "Xface/XFace/XFace.sln", "Xface/XFace/XFace.vcxproj", "Xface/XFace/XFace.vcxproj.filters", "Xface/XFace/docs/api/xfacedoc.css", "Xface/XFace/docs/api/xheader.html", "Xface/XFace/docs/xface.doxygen", "Xface/XFace/include/XEngine/BmpFile.h", "Xface/XFace/include/XEngine/Buffer.h", "Xface/XFace/include/XEngine/DeformableGeometry.h", "Xface/XFace/include/XEngine/Drawable.h", "Xface/XFace/include/XEngine/Geometry.h", "Xface/XFace/include/XEngine/IRenderer.h", "Xface/XFace/include/XEngine/ISceneLoader.h", "Xface/XFace/include/XEngine/ITexture.h", "Xface/XFace/include/XEngine/ITextureFile.h", "Xface/XFace/include/XEngine/ITextureLoader.h", "Xface/XFace/include/XEngine/IndexedFaceSet.h", "Xface/XFace/include/XEngine/MeshManager.h", "Xface/XFace/include/XEngine/ModelFileFactory.h", "Xface/XFace/include/XEngine/NamedObj.h", "Xface/XFace/include/XEngine/Node.h", "Xface/XFace/include/XEngine/OBJLoader.h", "Xface/XFace/include/XEngine/RenderList.h", "Xface/XFace/include/XEngine/RendererGL.h", "Xface/XFace/include/XEngine/Texture2D.h", "Xface/XFace/include/XEngine/TextureLoaderGL.h", "Xface/XFace/include/XEngine/TextureManager.h", "Xface/XFace/include/XEngine/TgaFile.h", "Xface/XFace/include/XEngine/TgaImg.h"]}

## Source file: Xface/XFace/src/XEngine/DeformableGeometry.cpp

{
  "path": "Xface/XFace/src/XEngine/DeformableGeometry.cpp",
  "blob_sha": "ef143288b4e374519c4b24a7d09996865fbef221",
  "sha256": "02857e2540f2644069566c5f74c6f8ca12bc1ca1a4cbcc5dad07b247ad78c818",
  "lines": 189,
  "bytes": 7778,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 18,
    "jaw": 3,
    "tongue": 5,
    "deform": 11,
    "rig": 5,
    "face": 2,
    "lip": 22
  },
  "url": "https://github.com/thuhcsi/Crystal.TTVS/blob/9e6c86566bfacf4fc571ad7b1308bcb7382bb213/Xface/XFace/src/XEngine/DeformableGeometry.cpp",
  "function_signature_candidates": []
}

```text

/* ***** BEGIN LICENSE BLOCK *****
 * Version: MPL 1.1
 *
 * The contents of this file are subject to the Mozilla Public License Version
 * 1.1 (the "License"); you may not use this file except in compliance with
 * the License. You may obtain a copy of the License at
 * http://www.mozilla.org/MPL/
 *
 * Software distributed under the License is distributed on an "AS IS" basis,
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License
 * for the specific language governing rights and limitations under the
 * License.
 *
 * The Original Code is XFace::XEngine
 *
 * The Initial Developer of the Original Code is
 * ITC-irst, TCC Division (http://tcc.itc.it) Trento / ITALY.
 * Portions created by the Initial Developer are Copyright (C) 2004
 * the Initial Developer. All Rights Reserved.
 *
 * Contributor(s):
 * - Koray Balci (koraybalci@yahoo.com)
 * ***** END LICENSE BLOCK ***** */

#include <XEngine/DeformableGeometry.h>
#include <cassert>
#include <iterator>

namespace XEngine{

DeformableGeometry::DeformableGeometry(const std::string& name) : IndexedFaceSet(name), m_bNeedUpdate(false)
{
	prepareFAPLookup();
}

DeformableGeometry::~DeformableGeometry(void)
{
}

void DeformableGeometry::setVertices(Vector3* pVert, unsigned int size)
{
	// let base class has to do whatever it has to do
	Geometry::setVertices(pVert, size);
	m_def_vertices = m_vertices;

	clearInfluences();
}

void DeformableGeometry::setVertices(std::vector<Vector3> &vertices)
{
	// let base class has to do whatever it has to do
	Geometry::setVertices(vertices);
	m_def_vertices = vertices;

	clearInfluences();
}

void DeformableGeometry::clearInfluences()
{
	m_weights.clear();
	m_weightIndices.clear();
	DeformationWeights val;
	std::fill_n(std::back_inserter(m_weights), m_vertices.size(), val);
	DeformationIndices ind;
	std::fill_n(std::back_inserter(m_weightIndices), m_vertices.size(), ind);
}

void DeformableGeometry::addInfluence(const std::set<unsigned short>& aoi, 
									const std::vector<float>& w, 
									unsigned short fap)
{
	// set the weight and bone id from the data
	std::set<unsigned short>::const_iterator index_it = aoi.begin();
	for (size_t i = 0; i < w.size(); ++i, ++index_it)
	{
		// get an empty slot in weight (limit is MAXWEIGHT weights per vertex)
		size_t slot = 0;
		while(slot< MAXWEIGHT && m_weights[*index_it][slot])
			slot++;
		if(slot == MAXWEIGHT) // this vertex is full of weights, ignore the rest..
			continue;

		// assign finally!
		m_weights[*index_it][slot] = w[i];
		m_weightIndices[*index_it][slot] = fap;
	}
}

/*!
	\todo computeVertexNormals() ??
*/
void DeformableGeometry::update()
{
	for(size_t i = 0; i < m_def_vertices.size(); ++i)
	{
		m_def_vertices[i] = m_vertices[i];
		for(size_t j = 0; j < MAXWEIGHT; ++j)
		{
			if(m_weightIndices[i][j] == 0)
				continue;
			float fap = m_animationParams[ m_weightIndices[i][j] ];
			if(fap)
				m_def_vertices[i] += m_FAPDirection[ m_weightIndices[i][j] ] * (fap * m_weights[i][j]);
		}
	}

//	computeVertexNormals();
}

void DeformableGeometry::prepareFAPLookup()
{
	// first two faps (high level)
	m_FAPDirection.push_back(Vector3(0, 0, 0));		// 1. viseme
	m_FAPDirection.push_back(Vector3(0, 0, 0));		// 2. emotion
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 3. Open_jaw
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 4. lower_t_midlip
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 5. Raise_b_midlip
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 6. Stretch_l_cornerlip
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 7. Stretch_r_cornerlip
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 8. Lower_t_lip_lm
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 9. Lower_t_lip_rm
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 10. Raise_b_lip_lm
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 11. Raise_b_lip_rm
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 12. Raise_l_cornerlip
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 13. Raise_r_cornerlip
	m_FAPDirection.push_back(Vector3(0, 0, 1));		// 14. Thrust_jaw
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 15. Shift_jaw
	m_FAPDirection.push_back(Vector3(0, 0, 1));		// 16. Push_b_lip
	m_FAPDirection.push_back(Vector3(0, 0, 1));		// 17. Push_t_lip
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 18. Depress_chin
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 19. Close_t_l_eyelid
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 20. Close_t_r_eyelid
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 21. Close_b_l_eyelid
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 22. Close_b_r_eyelid
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 23. Yaw_l_eyeball
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 24. Yaw_r_eyeball
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 25. Pitch_l_eyeball
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 26. Pitch_r_eyeball
	m_FAPDirection.push_back(Vector3(0, 0, 1));		// 27. Thrust_l_eyeball
	m_FAPDirection.push_back(Vector3(0, 0, 1));		// 28. Thrust_r_eyeball
	m_FAPDirection.push_back(Vector3(1, 1, 1));		// 29. Dilate_l_pupil (grow)
	m_FAPDirection.push_back(Vector3(1, 1, 1));		// 30. Dilate_r_pupil (grow)

	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 31. Raise_l_i_eyebrow
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 32. Raise_r_i_eyebrow
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 33. Raise_l_m_eyebrow
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 34. Raise_r_m_eyebrow
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 35. Raise_l_o_eyebrow
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 36. Raise_r_o_eyebrow
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 37. Squeeze_l_eyebrow
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 38. Squeeze_r_eyebrow
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 39. Puff_l_cheek
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 40. Puff_r_cheek
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 41. Lift_l_cheek
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 42. Lift_r_cheek
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 43. Shift_tongue_tip
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 44. Raise_tongue_tip
	m_FAPDirection.push_back(Vector3(0, 0, 1));		// 45. Thrust_tongue_tip
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 46. Raise_tongue
	m_FAPDirection.push_back(Vector3(0, 1, 1));		// 47. Tongue_roll
	
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 48. Head_pitch
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 49. Head_yaw
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 50. Head_roll
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 51. Lower_t_midlip_o
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 52. Raise_b_midlip_o
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 53. Stretch_l_cornerlip_o
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 54. Stretch_r_cornerlip_o
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 55. Lower_t_lip_lm_o
	m_FAPDirection.push_back(Vector3(0, -1, 0));	// 56. Lower_t_lip_rm_o
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 57. Raise_b_lip_lm_o
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 58. Raise_b_lip_rm_o
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 59. Raise_l_cornerlip_o
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 60. Raise_r_cornerlip_o

	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 61. Stretch_l_nose
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 62. Stretch_r_nose
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 63. Raise_nose
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 64. Bend_nose
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 65. Raise_l_ear
	m_FAPDirection.push_back(Vector3(0, 1, 0));		// 66. Raise_r_ear
	m_FAPDirection.push_back(Vector3(1, 0, 0));		// 67. Pull_l_ear
	m_FAPDirection.push_back(Vector3(-1, 0, 0));	// 68. Pull_r_ear
}


} // namespace XEngine




```

## Source file: Xface/XFace/include/XEngine/DeformableGeometry.h

{
  "path": "Xface/XFace/include/XEngine/DeformableGeometry.h",
  "blob_sha": "7e9e760aac5145052408024a03fe61183cecf2e7",
  "sha256": "c5deae73bc9e8093c8b7cbcf576319d894019372ec756c86877d5bcf915cf067",
  "lines": 153,
  "bytes": 7357,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 5,
    "tongue": 1,
    "deform": 29,
    "shader": 4,
    "rig": 5,
    "face": 13
  },
  "url": "https://github.com/thuhcsi/Crystal.TTVS/blob/9e6c86566bfacf4fc571ad7b1308bcb7382bb213/Xface/XFace/include/XEngine/DeformableGeometry.h",
  "function_signature_candidates": [
    {
      "line": 73,
      "text": "void prepareFAPLookup();"
    },
    {
      "line": 82,
      "text": "void setVertices(Vector3* pVert, unsigned int size);"
    },
    {
      "line": 83,
      "text": "void setVertices(std::vector<Vector3> &vertices);"
    },
    {
      "line": 84,
      "text": "void addInfluence(const std::set<unsigned short>& aoi, const std::vector<float>& w, unsigned short fap);"
    },
    {
      "line": 85,
      "text": "void clearInfluences();"
    },
    {
      "line": 86,
      "text": "void updateAnimationParams(const std::vector<float>& aps) {m_animationParams = aps; m_bNeedUpdate = true;}"
    },
    {
      "line": 87,
      "text": "bool needUpdate() const {return m_bNeedUpdate;}"
    },
    {
      "line": 88,
      "text": "void update();"
    }
  ]
}

```text

/* ***** BEGIN LICENSE BLOCK *****
 * Version: MPL 1.1
 *
 * The contents of this file are subject to the Mozilla Public License Version
 * 1.1 (the "License"); you may not use this file except in compliance with
 * the License. You may obtain a copy of the License at
 * http://www.mozilla.org/MPL/
 *
 * Software distributed under the License is distributed on an "AS IS" basis,
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License
 * for the specific language governing rights and limitations under the
 * License.
 *
 * The Original Code is XFace::XEngine
 *
 * The Initial Developer of the Original Code is
 * ITC-irst, TCC Division (http://tcc.itc.it) Trento / ITALY.
 * Portions created by the Initial Developer are Copyright (C) 2004
 * the Initial Developer. All Rights Reserved.
 *
 * Contributor(s):
 * - Koray Balci (koraybalci@yahoo.com)
 * ***** END LICENSE BLOCK ***** */
/*! 
	\file DeformableGeometry.h
	\brief Deformation enabled geometry class.
*/

#pragma once
#include "IndexedFaceSet.h"
#include <cassert>
#include <map>
#include <set>
#include <vector>
#include <algorithm>

namespace XEngine{

#define MAXWEIGHT 8

/*!
	\brief Deformation enabled geometry class.
	\ingroup XEngine
	\author Koray Balci
	\version 1.0
	\date   Sept 2004
			
	Deformation is done on this Geometry derived class.
*/
class DeformableGeometry :
	public IndexedFaceSet
{
	struct DeformationWeights
	{
		DeformationWeights() {memset(&w[0], 0, MAXWEIGHT*sizeof(float));}
		float operator[](size_t i) const {assert(i < MAXWEIGHT); return w[i];}
		float& operator[](size_t i) {assert(i < MAXWEIGHT); return w[i];}
	private:
		float w[MAXWEIGHT];
	};

	struct DeformationIndices
	{
		DeformationIndices() {memset(&id[0], 0, MAXWEIGHT*sizeof(unsigned short));}
		unsigned short operator[](size_t i) const {assert(i < MAXWEIGHT); return id[i];}
		unsigned short& operator[](size_t i) {assert(i < MAXWEIGHT); return id[i];}
	private:
		unsigned short id[MAXWEIGHT];
	};

	// these should be static
	std::vector<Vector3> m_FAPDirection;
	void prepareFAPLookup();
	bool m_bNeedUpdate;
protected:
	std::vector<DeformationWeights> m_weights;
	std::vector<DeformationIndices> m_weightIndices;
	std::vector<Vector3> m_def_vertices;
	std::vector<float> m_animationParams;
public:
	const Vector3* const getDeformedVertices() const {return &m_def_vertices[0];}
	void setVertices(Vector3* pVert, unsigned int size);
	void setVertices(std::vector<Vector3> &vertices);
	void addInfluence(const std::set<unsigned short>& aoi, const std::vector<float>& w, unsigned short fap);
	void clearInfluences();
	void updateAnimationParams(const std::vector<float>& aps) {m_animationParams = aps; m_bNeedUpdate = true;}
	bool needUpdate() const {return m_bNeedUpdate;}
	void update();
	DeformableGeometry(const std::string& name);
	~DeformableGeometry(void);
};

} // namespace XEngine

/*!
	\page DeformationPage Skinning Adventures / Architecture Review

	\section DevDiary Developer Diary
	\date Wednesday, September 15, 2004

	I finally finished working on skinning. First of all, I didn't really implement the conventional vertex 
	skinning, because I realized that I don't have to!

	I was planning to use ARB_vertex_blend OpenGL extension and implement a mid-level path along with hw shader 
	and sw deformer. Unfortunately, I realized that, this path requires some change in the mesh data structure. 
	I needed to have submeshes connected to all deformers, something that I don't want to implement really. 
	Actually, my current structure is much better than that already, and also both hw and sw paths do not need 
	that kind of structuring. Alternatively, you can use ARB_matrix_palette extension along with vertex_blend to 
	overcome this, but it is not supported by any of the card vendors anymore! People say, this is because 
	skinning by extensions is replaced by hw shader versions. So, as a result, I also decided to forget about 
	this path totally.

	As soon as I gave up that path, I realized that I do NOT need to change my architecture that much, I do NOT 
	need to implement skinning in the way it is explained. I don't need to store transformation matrices and bone 
	structures for every deformation point as I planned. Here is my architecture.

	XFace::FaceBase class controls the whole system as before. We have a new class for geometry to handle the 
	deformations, XEngine::DeformableGeometry. Each XEngine::Drawable object now has a XEngine::DeformableGeometry 
	bound to it. As before, XEngine::Drawable's represent the building blocks of the head, i.e. the face, hair, 
	tongue, etc.. FaceBase, has a list of XEngine::Drawable objects, during the initialization phase, the meshes 
	are loaded from disk into XEngine::DeformableGeometry objects and attached to XEngine::Drawable objects. 
	In reality, only the names of XEngine::DeformableGeometry objects are stored in XEngine::Drawable, and they 
	are retrieved through XEngine::MeshManager singleton object. During the initialization phase, also FDP file 
	is loaded and XFace::FDPItems are filled. XFace::FDPItem objects represent MPEG-4 feature points defined by 
	the standard, and store the characteristics of the feature point. Namely, we store, for each feature point;

    * the mesh it is affecting,
    * control point that represents the feature point,
    * the region (vertices) influenced by the control point,
    * a set of influence calculators

	Influence calculators store weights for each vertex in the zone of influence, where weight represents how 
	the coresponding vertex will be affected by the deformation of that feature point. Interface for the influence 
	calculators are defined by XFace::IInfluenceCalculators class. Currently, we have a set of raised cosine based 
	influence calculator concrete classes. XfaceEd automatically detects influence calculators, thanks to pluggable 
	factory pattern implemented inside. See XFace::InfluenceCalculatorMaker for more information on that. 
	Each XFace::IInfluenceCalculator derived object also stores a coefficient to control the deformation limits, 
	and the FAP id that the weights are used for (or in other words, the FAP that the influence calculator 
	responds to).

	So during the initialization, when the FDP file is loaded, first static meshes (OBJ/VRML/X3D or any other 
	supported format) are loaded to XEngine::DeformableGeometry instances, and feature point info are loaded to 
	XFace::FDPItems, and XFace::IInfluenceCalculators are created and the weights for the zone of influence are
	calculated. Then those weights are routed to XEngine::DeformableGeometry objects.

	XEngine::DeformableGeometry objects store two sets of vertices, one for the still (static, no emotion/movement)
	and one for the deformed. During runtime, when playback of FAPs starts, for each new frame, 68 FAP values are
	sent to XEngine::DeformableGeometry::updateAnimationParams method, but the deformation is delayed until the
	rendering. During the rendering process, if a geometry needs an update, its update method is called for the 
	moment, though I plan to move this piece of code out of XEngine::DeformableGeometry. This lazy evaluation 
	(in the rendering pipeline) enables us to call a shader instead of updating on software. Though I haven't 
	implemented the shader pipeline, all the structuring is done.
*/


```

## Source file: Xface/XFace/src/XFace/FaceBase.cpp

{
  "path": "Xface/XFace/src/XFace/FaceBase.cpp",
  "blob_sha": "a6c387a207c7dcdf95d7372e009882d0d7b92b21",
  "sha256": "894fd3bf2ed337a4d850a159bdcd092df1a6ad286be4aac1ff64584c960bb166",
  "lines": 160,
  "bytes": 4287,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 3,
    "deform": 2,
    "rig": 5,
    "face": 17
  },
  "url": "https://github.com/thuhcsi/Crystal.TTVS/blob/9e6c86566bfacf4fc571ad7b1308bcb7382bb213/Xface/XFace/src/XFace/FaceBase.cpp",
  "function_signature_candidates": []
}

```text

/* ***** BEGIN LICENSE BLOCK *****
 * Version: MPL 1.1
 *
 * The contents of this file are subject to the Mozilla Public License Version
 * 1.1 (the "License"); you may not use this file except in compliance with
 * the License. You may obtain a copy of the License at
 * http://www.mozilla.org/MPL/
 *
 * Software distributed under the License is distributed on an "AS IS" basis,
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License
 * for the specific language governing rights and limitations under the
 * License.
 *
 * The Original Code is Xface Core Library; FaceBase imp.
 *
 * The Initial Developer of the Original Code is
 * ITC-irst, TCC Division (http://tcc.itc.it) Trento / ITALY.
 * For info, contact: xface-info@itc.it or http://xface.itc.it
 * Portions created by the Initial Developer are Copyright (C) 2004
 * the Initial Developer. All Rights Reserved.
 *
 * Contributor(s):
 * - Koray Balci (koraybalci@yahoo.com)
 * ***** END LICENSE BLOCK ***** */


#include <XFace/FaceBase.h>

#include <iostream>
#include <XEngine/RendererGL.h>
#include <XFace/RaisedCosInfluence.h>
#include <XEngine/TextureManager.h>

namespace XFace{

FaceBase::FaceBase(void) : m_pRenderer(0), m_pFDP(0), m_rotateX(0), m_rotateY(0), m_rotateZ(0)
{
}

FaceBase::~FaceBase(void)
{
	destroy();
}

void FaceBase::destroy()
{
	// clear the drawables
	std::for_each(m_Drawables.begin(), m_Drawables.end(), deletePointer_functor<Drawable>());
	m_Drawables.clear();
	m_RenderList.clearList();

	delete m_pRenderer;
	m_pRenderer = 0;
	
	delete m_pFDP;
	m_pFDP = 0;
}

bool FaceBase::initMeshes(const std::list<MeshInfo>& filenames, const std::string& path)
{
	// load new ones
	std::list<MeshInfo>::const_iterator it = filenames.begin();
	while (it != filenames.end())
	{
		std::string total_path = path + it->path;
		// we do not use the format, we resolve it inside the loader
		std::list<Drawable*> drawables = ModelFileFactory::loadModelFile(it->file, total_path);
		if(drawables.empty())
		{
			std::cerr << "Error loading mesh file " << it->file << std::endl;
			return false;
		}
		else
		{
			m_Drawables.insert(m_Drawables.end(), drawables.begin(), drawables.end());
			std::for_each(drawables.begin(), drawables.end(), renderListFiller_functor(m_RenderList));
		}

		++it;
	}
	
	return true;
}

bool FaceBase::initAux()
{
	// create the renderer (GL or DX choice here)
	m_pRenderer = new RendererGL;

	// initializes influence calcs
	initInfluenceCalculators();

	return true;
}

bool FaceBase::init(std::istream& input)
{
	FDPFile fdp_file;
	delete m_pFDP;
	m_pFDP = new FDP;
	fdp_file.load(input, m_pFDP);

	// initialize the textures and meshes (asuming path is correct as it is)
	if( !initMeshes(fdp_file.getMeshList(), "./") )
		return false;

	return initAux();
}

bool FaceBase::init(const std::string& filename, const std::string& path)
{
	// load configuration and fdp
	// FDP items load
	FDPFile fdp_file;
	delete m_pFDP;
	m_pFDP = new FDP;
	fdp_file.load(path + filename, m_pFDP);

	// initialize the textures and meshes
	if( !initMeshes(fdp_file.getMeshList(), path) )
		return false;

	return initAux();
}

void FaceBase::initInfluenceCalculators()
{
	// we have to make this function call in order to make static variables initialized in influence site
	RaisedCosInfluenceSph def(0, 5);

	// clean the old (if any) drawable contents (bones)
	std::for_each(m_Drawables.begin(), m_Drawables.end(), skinResetter_functor(MeshManager::getInstance()));

	const FDP::FDPITEMS items = m_pFDP->getItems();
	// calls another for_each inside for initializing weights in influence calc. and transferring them to skinned meshes
	std::for_each(items.begin(), items.end(), deformationInitializer_functor(MeshManager::getInstance()));
}

void FaceBase::render()
{
	m_pRenderer->setRotation(m_rotateX, m_rotateY, m_rotateZ);
	m_RenderList.renderList(m_pRenderer);
}

void FaceBase::update(const std::vector<float>& FAPs)
{
	std::for_each(m_Drawables.begin(), m_Drawables.end(), skinDeformer_functor(MeshManager::getInstance(), FAPs));

	// apply global rotation
	m_rotateX = FAPs[47]*100.001f;
	m_rotateY = FAPs[48]*100.001f;
	m_rotateZ = FAPs[49]*100.001f;
}

void FaceBase::rebindTextures()
{
    TextureManager::getInstance()->rebindTextures();
}

} // namespace XFace


```

## Source file: Xface/XFace/include/XFace/FaceBase.h

{
  "path": "Xface/XFace/include/XFace/FaceBase.h",
  "blob_sha": "9fd859b596af6a3be4b7b414e491c90fdd375ff8",
  "sha256": "4d81b5f4047f10896bb622c61c2150d196407e3f711596badb9ca947199e8e59",
  "lines": 191,
  "bytes": 5633,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 8,
    "deform": 11,
    "rig": 5,
    "face": 25
  },
  "url": "https://github.com/thuhcsi/Crystal.TTVS/blob/9e6c86566bfacf4fc571ad7b1308bcb7382bb213/Xface/XFace/include/XFace/FaceBase.h",
  "function_signature_candidates": [
    {
      "line": 88,
      "text": "void operator() (Drawable* dr) const{"
    },
    {
      "line": 103,
      "text": "void operator()(const Drawable* const dr) const{"
    },
    {
      "line": 118,
      "text": "void operator()(const Drawable* const dr) const{"
    },
    {
      "line": 142,
      "text": "void operator() (const std::pair<IInfluenceCalculator* const, std::string>& inf) const{"
    },
    {
      "line": 151,
      "text": "void operator()(const FDPItem* fdp) const{"
    },
    {
      "line": 164,
      "text": "void operator()(T* p) const {delete p;}"
    },
    {
      "line": 168,
      "text": "bool initAux();"
    },
    {
      "line": 179,
      "text": "void render();"
    },
    {
      "line": 180,
      "text": "void destroy();"
    },
    {
      "line": 181,
      "text": "virtual void initInfluenceCalculators();"
    },
    {
      "line": 182,
      "text": "bool initMeshes(const std::list<MeshInfo>& filenames, const std::string& path);"
    },
    {
      "line": 183,
      "text": "virtual bool init(std::istream& input);"
    },
    {
      "line": 184,
      "text": "virtual bool init(const std::string& filename, const std::string& path = \"./\");"
    },
    {
      "line": 185,
      "text": "void update(const std::vector<float>& FAPs);"
    },
    {
      "line": 188,
      "text": "void rebindTextures();"
    }
  ]
}

```text

/* ***** BEGIN LICENSE BLOCK *****
 * Version: MPL 1.1
 *
 * The contents of this file are subject to the Mozilla Public License Version
 * 1.1 (the "License"); you may not use this file except in compliance with
 * the License. You may obtain a copy of the License at
 * http://www.mozilla.org/MPL/
 *
 * Software distributed under the License is distributed on an "AS IS" basis,
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License
 * for the specific language governing rights and limitations under the
 * License.
 *
 * The Original Code is Xface Core Library.
 *
 * The Initial Developer of the Original Code is
 * ITC-irst, TCC Division (http://tcc.itc.it) Trento / ITALY.
 * For info, contact: xface-info@itc.it or http://xface.itc.it
 * Portions created by the Initial Developer are Copyright (C) 2004
 * the Initial Developer. All Rights Reserved.
 *
 * Contributor(s):
 * - Koray Balci (koraybalci@yahoo.com)
 * ***** END LICENSE BLOCK ***** */

/*! 
	\file	FaceBase.h
	\brief	Main face object.
*/

#ifndef FACEBASE_H_
#define FACEBASE_H_

#include <string>
#include <map>
#include <vector>

#include <XEngine/RenderList.h>
#include <XEngine/Drawable.h>
#include <XEngine/IRenderer.h>
#include <XFace/IInfluenceCalculator.h>
#include <XFace/FDP.h>
#include <XFace/FDPFile.h>
#include <XEngine/MeshManager.h>

using XMath::Vector3;
using namespace XEngine;

namespace XFace{

/*!
	\brief Main XFace class.
	\ingroup XFace
	\author Koray Balci
	\version 1.0
	\date   June 2003

	Main entry point for XFace library related operations. Only exposing this class
	to users should be sufficient. Pass the FDP configuration file for initialization, 
	then call update and render when needed. 
	\code
	    FaceBase myFace;
	    myFace.init(FDPFilename, FDPPath); // load an FDP file
	    
	    // in a loop of all FAP data
	    myFace.update(FAPData);
	    myFace.render();
	    
	    // when done, it's good practice to cleanup, especially if you plan on loading
	    // a new FDP file over the same FaceBase instance.
	    myFace.destroy();    
	\endcode
	
	\note Alternatively, you can use XFaceApp module and use XML tasks as messages 
	(see related documentation) and forget about this class and all other XFace classes.
*/
class FaceBase
{
	/*!
		for each Drawable*, inserts them to RenderList
	*/
	struct renderListFiller_functor
	{
	private:
		RenderList& m_rl;
	public:
		renderListFiller_functor(RenderList& rl): m_rl(rl){}
		void operator() (Drawable* dr) const{
			m_rl.addDrawable(dr);
		}

	};

	/*!
	*/
	struct skinDeformer_functor
	{
	private:
		const MeshManager* const m_pMM;
		const std::vector<float>& m_FAPs;
	public:
		skinDeformer_functor(const MeshManager* const pMM, const std::vector<float>& FAPs) : m_pMM(pMM), m_FAPs(FAPs) {}
		void operator()(const Drawable* const dr) const{
			DeformableGeometry* pMesh = m_pMM->getMesh(dr->getMeshName());
			pMesh->updateAnimationParams(m_FAPs);
		}
	};
	
	/*!
		Cleans the old (if any) drawable contents (bones)
	*/
	struct skinResetter_functor
	{
	private:
		const MeshManager* const m_pMM;
	public:
		skinResetter_functor(const MeshManager* const pMM) : m_pMM(pMM) {}
		void operator()(const Drawable* const dr) const{
			DeformableGeometry* pMesh = m_pMM->getMesh(dr->getMeshName());
			pMesh->clearInfluences();
		}
	};
	
	/*!
		for each FDPItem, retrieves the member IInfluenceCalculator*'s, and executes deformerInitializer_functor on them.
		Then, after accumulating the weights, passes the bone info to DeformableGeometry objects in m_Drawables.
	*/
	struct deformationInitializer_functor
	{
	private:
		/*!
			for each std::pair<IInfluenceCalculator* const, std::string> (actually IInfluenceCalculator*), 
			calculates the weights in IInfluenceCalculator instances. Then transfers these weights to SkinnedMesh obj.
		*/
		struct influenceToSkin_functor
		{
		private:
			DeformableGeometry* const m_pMesh;
			const FDPItem* const m_pFdp;
		public:
			influenceToSkin_functor(DeformableGeometry* const pMesh, const FDPItem* const pFdp) : m_pMesh(pMesh), m_pFdp(pFdp) {}
			void operator() (const std::pair<IInfluenceCalculator* const, std::string>& inf) const{
				inf.first->init(m_pFdp);
				m_pMesh->addInfluence(m_pFdp->getAOI(), inf.first->getWeights(), inf.first->getFapID());
			}
		};
	
		MeshManager* m_pMM;
	public:
		deformationInitializer_functor(MeshManager* pMM) : m_pMM(pMM){}
		void operator()(const FDPItem* fdp) const{
			FDPItem::INFLUENCECALC_MAP infCalcs =  fdp->getInfluenceCalculators();
			DeformableGeometry* pMesh = m_pMM->getMesh(fdp->getAffects());
			std::for_each(infCalcs.begin(), infCalcs.end(), influenceToSkin_functor(pMesh, fdp));
		}
	};

	/*!
		template fonctor to call delete on collection that stores pointers to objects.
	*/
	template <class T>
	struct deletePointer_functor
	{
		void operator()(T* p) const {delete p;}
	};
	
	float m_rotateX, m_rotateY, m_rotateZ;
	bool initAux();
protected:
	FDP* m_pFDP;
	RenderList m_RenderList;
	std::list<Drawable*> m_Drawables;
	IRenderer* m_pRenderer;

public:
	const FAPU& getFAPU() const {return m_pFDP->getFAPU();};
	FDP* getFDP() const {return m_pFDP;}
	const std::list<Drawable*>& getDrawables() const {return m_Drawables;}
	void render();
	void destroy();
	virtual void initInfluenceCalculators();
	bool initMeshes(const std::list<MeshInfo>& filenames, const std::string& path);
	virtual bool init(std::istream& input);
	virtual bool init(const std::string& filename, const std::string& path = "./");
	void update(const std::vector<float>& FAPs);
	FaceBase(void);	
	virtual ~FaceBase(void);
    void rebindTextures();
};
}
#endif // FACEBASE_H_


```
