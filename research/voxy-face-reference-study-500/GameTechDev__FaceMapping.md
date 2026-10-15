# Facial rendering source scan: GameTechDev/FaceMapping

Repository: https://github.com/GameTechDev/FaceMapping

Pinned source revision: e51c509c49d0b2fbab0bba83324ee619d91d1ed8

Discovery topic: face model 3D

Description: The face mapping sample uses the 3D Scan module to scan the user's face and then map it onto an existing 3D head model. This technique does a "stone face" mapping that is not rigged or currently capable of animating.

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["CPUT/middleware/stb/LICENSE", "license.txt"]

Topic paths: {"skin": ["Media/System/Material/shadowCastSkinned.mtl", "Media/basicHead_03/Texture/skinVariant.dds"], "shader": ["CPUT/include/directx/CPUTComputeShaderDX11.h", "CPUT/include/directx/CPUTDomainShaderDX11.h", "CPUT/include/directx/CPUTGeometryShaderDX11.h", "CPUT/include/directx/CPUTHullShaderDX11.h", "CPUT/include/directx/CPUTPixelShaderDX11.h", "CPUT/include/directx/CPUTShaderDX11.h", "CPUT/include/directx/CPUTVertexShaderDX11.h", "CPUT/source/directx/CPUTComputeShaderDX11.cpp", "CPUT/source/directx/CPUTDomainShaderDX11.cpp", "CPUT/source/directx/CPUTGeometryShaderDX11.cpp", "CPUT/source/directx/CPUTHullShaderDX11.cpp", "CPUT/source/directx/CPUTPixelShaderDX11.cpp", "CPUT/source/directx/CPUTShaderDX11.cpp", "CPUT/source/directx/CPUTVertexShaderDX11.cpp", "Extras/DirectXTex/DDSView/shaders/ps1D.h", "Extras/DirectXTex/DDSView/shaders/ps1Darray.h", "Extras/DirectXTex/DDSView/shaders/ps2D.h", "Extras/DirectXTex/DDSView/shaders/ps2Darray.h", "Extras/DirectXTex/DDSView/shaders/ps3D.h", "Extras/DirectXTex/DDSView/shaders/psCube.h", "Extras/DirectXTex/DDSView/shaders/vs.h", "Media/GUI/Shader/GUIRenderState.rs", "Media/GUI/Shader/GUI_Element.fx", "Media/GUI/Shader/GUI_Element.glsl", "Media/GUI/Shader/GUI_Sprite.fx", "Media/GUI/Shader/GUI_Sprite.glsl", "Media/GUI/Shader/Slider.fx", "Media/GUI/Shader/Slider.glsl", "Media/GUI/Shader/Visualize.fx", "Media/GUI/Shader/Visualize.glsl"], "face": ["Media/basicHead_03/Material/Face.mtl", "Media/basicHead_03/Shader/FaceMapping.fx", "Media/basicHead_03/Shader/FaceRenderStates.rs", "Media/basicHead_03/Shader/SculptFace.fx", "Media/facedisplace.scene", "facescan/.vs/facescan/v15/.suo", "facescan/.vs/facescan/v15/Browse.VC.db", "facescan/.vs/facescan/v15/Solution.VC.db", "facescan/.vs/facescan/v15/Solution.VC.db-shm", "facescan/.vs/facescan/v15/Solution.VC.db-wal", "facescan/CFaceModel.cpp", "facescan/CFaceModel.h", "facescan/CPUT.ico", "facescan/GameFaceScan.cpp", "facescan/GameFaceScan.h", "facescan/Menu/Controls/Font.cpp", "facescan/Menu/Controls/Font.h", "facescan/Menu/Controls/MenuButton.cpp", "facescan/Menu/Controls/MenuButton.h", "facescan/Menu/Controls/MenuCanvas.cpp", "facescan/Menu/Controls/MenuCanvas.h", "facescan/Menu/Controls/MenuControl.cpp", "facescan/Menu/Controls/MenuControl.h", "facescan/Menu/Controls/MenuTypes.h", "facescan/Menu/MenuBase.h", "facescan/Menu/MenuController.cpp", "facescan/Menu/MenuController.h", "facescan/Menu/MenuGlob.cpp", "facescan/Menu/MenuGlob.h", "facescan/Menu/Menu_FaceMapping.cpp"], "lip": ["Extras/DirectXTex/DirectXTex/DirectXTexFlipRotate.cpp", "Media/hairGeekGirl/Shader/Maya_One_UV_clip.fx", "Media/hairGeekGirl_black/Shader/Maya_One_UV_clip.fx", "Media/hairGeekGirl_blonde/Shader/Maya_One_UV_clip.fx"]}

## Source file: Media/basicHead_03/Shader/FaceRenderStates.rs

{
  "path": "Media/basicHead_03/Shader/FaceRenderStates.rs",
  "blob_sha": "328026389238f9269a8518dec2d1b1d8ad2580b8",
  "sha256": "c7eaebd867814ee8614d256f3dc35b77773af01d859a59c38d55ab52bddd4ad2",
  "lines": 31,
  "bytes": 834,
  "selection_score": 18,
  "keyword_line_counts": {},
  "url": "https://github.com/GameTechDev/FaceMapping/blob/e51c509c49d0b2fbab0bba83324ee619d91d1ed8/Media/basicHead_03/Shader/FaceRenderStates.rs",
  "function_signature_candidates": []
}

```text

[RasterizerStateDX11]

FillMode = D3D11_FILL_SOLID

[DepthStencilStateDX11]
DepthEnable = true
DepthFunc   = D3D11_COMPARISON_GREATER_EQUAL

[SamplerDX11_1]
Filter         = D3D11_FILTER_MIN_MAG_MIP_LINEAR
AddressU       = D3D11_TEXTURE_ADDRESS_CLAMP
AddressV       = D3D11_TEXTURE_ADDRESS_CLAMP
BorderColor0   = 1.0
BorderColor1   = 1.0
BorderColor2   = 1.0
BorderColor3   = 1.0

[SamplerDX11_2]
Filter         = D3D11_FILTER_COMPARISON_MIN_MAG_LINEAR_MIP_POINT
AddressU       = D3D11_TEXTURE_ADDRESS_BORDER
AddressV       = D3D11_TEXTURE_ADDRESS_BORDER
ComparisonFunc = D3D11_COMPARISON_GREATER

[SamplerDX11_3]
Filter         = D3D11_FILTER_MIN_MAG_MIP_LINEAR
AddressU       = D3D11_TEXTURE_ADDRESS_WRAP
AddressV       = D3D11_TEXTURE_ADDRESS_WRAP
BorderColor0   = 1.0
BorderColor1   = 1.0
BorderColor2   = 1.0
BorderColor3   = 1.0

```

## Source file: facescan/CFaceModel.h

{
  "path": "facescan/CFaceModel.h",
  "blob_sha": "acfaa36a8798d1b47461c77c58617ca1631caf54",
  "sha256": "9f9e4a051c1b4464f39dda2736621457fc6beb8a00d22c046463a278d0e68977",
  "lines": 56,
  "bytes": 1619,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 3
  },
  "url": "https://github.com/GameTechDev/FaceMapping/blob/e51c509c49d0b2fbab0bba83324ee619d91d1ed8/facescan/CFaceModel.h",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "void LoadObjFilename(const std::string &filename, bool landmarks = true);"
    },
    {
      "line": 36,
      "text": "bool LoadLandmarks();"
    },
    {
      "line": 52,
      "text": "bool LoadLandmarks( const std::string &filename, std::map<std::string, float3> &outMap, int formatVersion = 0 );"
    }
  ]
}

```text

/////////////////////////////////////////////////////////////////////////////////////////////
// Copyright 2017 Intel Corporation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or imlied.
// See the License for the specific language governing permissions and
// limitations under the License.
/////////////////////////////////////////////////////////////////////////////////////////////
#ifndef __OBJ_MODEL__
#define __OBJ_MODEL__

#include "ObjLoader.h"
#include <map>
#include "CPUTMath.h"

class CPUTModel;
class CPUTTexture;

class CFaceModel
{
public:

	CFaceModel();
	~CFaceModel();

	void LoadObjFilename(const std::string &filename, bool landmarks = true);
	CPUTModel *LoadCPUTModel(CPUTTexture **outColorTexture);

	bool LoadLandmarks();

	std::map<std::string, float3> Landmarks;

	float3 AABBMin;
	float3 AABBMax;

private:
	std::string mObjFilename;
	tObjModel mObjModel;

	// values that were applied to the objmodel after it was loaded
	// in an effort to normalize the mesh
	float mVertScale;
	float3 mVertOffset;
	
	bool LoadLandmarks( const std::string &filename, std::map<std::string, float3> &outMap, int formatVersion = 0 );
	
};

#endif // __OBJ_MODEL__

```

## Source file: facescan/CFaceModel.cpp

{
  "path": "facescan/CFaceModel.cpp",
  "blob_sha": "aed252949ebea2c1901b587ad0fa14084fe3c503",
  "sha256": "579c1077b20019a611688c641d577333a11270a886c28d8c9912e8d29aeb2017",
  "lines": 237,
  "bytes": 7364,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 8,
    "rig": 9,
    "face": 8
  },
  "url": "https://github.com/GameTechDev/FaceMapping/blob/e51c509c49d0b2fbab0bba83324ee619d91d1ed8/facescan/CFaceModel.cpp",
  "function_signature_candidates": []
}

```text

/////////////////////////////////////////////////////////////////////////////////////////////
// Copyright 2017 Intel Corporation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or imlied.
// See the License for the specific language governing permissions and
// limitations under the License.
/////////////////////////////////////////////////////////////////////////////////////////////
#include "CFaceModel.h"
#include "CPUTModel.h"
#include "CPUTMesh.h"
#include "CPUTMaterial.h"
#include "SampleUtil.h"
#include "CPUTTexture.h"

CFaceModel::CFaceModel() :
mObjFilename("")
{

}

CFaceModel::~CFaceModel()
{
}

void CFaceModel::LoadObjFilename(const std::string &filename, bool landmarks)
{
	ProfileBlockScoped block("Load OBJ");

	AABBMin = float3(10000.0f, 10000.0f, 10000.0f);
	AABBMax = -AABBMin;

	mObjFilename = filename;
	mObjModel.m_indices.clear();
	mObjModel.m_segments.clear();
	mObjModel.m_vertices.clear();

	objLoader(filename.c_str(), mObjModel);

	float boxSize = 8.0f;
	if (mObjModel.m_vertices.size() > 0)
	{
		float3 vmin = float3(mObjModel.m_vertices[0].x, mObjModel.m_vertices[0].y, mObjModel.m_vertices[0].z);
		float3 vmax = float3(mObjModel.m_vertices[0].x, mObjModel.m_vertices[0].y, mObjModel.m_vertices[0].z);

		for (std::vector<tVertex>::const_iterator it = mObjModel.m_vertices.begin(); it != mObjModel.m_vertices.end(); ++it)
		{
			vmin.x = floatMin(vmin.x, it->x);
			vmin.y = floatMin(vmin.y, it->y);
			vmin.z = floatMin(vmin.z, it->z);
			vmax.x = floatMax(vmax.x, it->x);
			vmax.y = floatMax(vmax.y, it->y);
			vmax.z = floatMax(vmax.z, it->z);
		}

		float3 center = (vmax + vmin) / 2.0f;
		float3 dim = vmax - vmin;

		float maxDim = floatMax(dim.x, floatMax(dim.y, dim.z));

		mVertOffset = -center;
		mVertScale = boxSize / maxDim;

		// center to origin, scale, and rotate
		// 
		int vertCount = (int)mObjModel.m_vertices.size();
		for (int i = 0; i < vertCount; i++)
		{
			tVertex *it = &mObjModel.m_vertices[i];
			float3 pos = float3(it->x, it->y, it->z);
			pos -= center;
			pos *= mVertScale;

			// save scale and rotate from RSSDK format. Z is up in rssdk
			it->x = pos.x;
			it->y = pos.z;
			it->z = pos.y;
			it->v = 1.0f - it->v;

			AABBMin.x = floatMin(it->x, AABBMin.x);
			AABBMin.y = floatMin(it->y, AABBMin.y);
			AABBMin.z = floatMin(it->z, AABBMin.z);
			AABBMax.x = floatMax(it->x, AABBMax.x);
			AABBMax.y = floatMax(it->y, AABBMax.y);
			AABBMax.z = floatMax(it->z, AABBMax.z);

		}
	}
	LoadLandmarks();
}

CPUTModel *CFaceModel::LoadCPUTModel( CPUTTexture **outColorImage )
{
	ProfileBlockScoped block("LoadCPUTModel");
	
	CPUTModel *model = CPUTModel::Create();
	CPUTMesh *mesh = CPUTMesh::Create();
	CPUTBufferElementInfo pVertexInfo[] = {
			{ "POSITION", 0, 0, CPUT_F32, 3, 3 * sizeof(float), 0 },
			{ "TEXCOORD", 0, 1, CPUT_F32, 2, 2 * sizeof(float), 3 * sizeof(float) },
			{ "NORMAL", 0, 2, CPUT_F32, 3, 3 * sizeof(float), 5 * sizeof(float) },
	};
	static int size = sizeof(tVertex);
	CPUTBufferElementInfo indexElementInfo = { "POSITION", 0, 0, CPUT_U32, 1, sizeof(ObjIndexInt), 0 };
	mesh->SetMeshTopology(CPUT_TOPOLOGY_INDEXED_TRIANGLE_LIST);
	mesh->CreateNativeResources(model, 0, 3, pVertexInfo, (int)mObjModel.m_vertices.size(), &mObjModel.m_vertices[0], NULL, 0, NULL);

	model->mpMaterialCount = new int[1];
	model->mpMaterial = new CPUTMaterial**[1];
	model->mpMaterial[0] = new CPUTMaterial*[1];
	model->mpMaterial[0][0] = NULL;
	model->mpMaterialCount[0] = 1;

	if (outColorImage != NULL)
	{
		std::string fnString = mObjFilename;
		int lastindex = (int)fnString.find_last_of(".");
		std::string rawname = fnString.substr(0, lastindex);
		std::string textureName = rawname.append("image1.png");
		*outColorImage = CPUTTexture::Create(std::string("facetexture"), textureName, false);
	}
	
	model->SetMeshes(&mesh, 1);

	return model;
}

bool CFaceModel::LoadLandmarks(const std::string &filename, std::map<std::string, float3> &outMap, int formatVersion)
{
	char line[1024];
	
	FILE *file = fopen(filename.c_str(), "r");

	if (file == NULL)
		return false;
	
	char toRemove[] = { '\t', ' ', '{', '}', '[', ']', '\"', '\n', ',', 'x', 'y', 'z' };
	const char *toRemoveStrings[] = { "Name", "Confidence", "Position" };
	while (fgets(line, sizeof(line), file))
	{
		bool hasConfidence = strstr(line, "Confidence") != NULL;
		for (int i = 0; i < ARRAYSIZE(toRemoveStrings); i++)
		{
			char *loc = line;
			while (loc = strstr(loc, toRemoveStrings[i]))
			{
				int toRemoveLen = (int)strlen(toRemoveStrings[i]);
				memmove(loc, loc + toRemoveLen, 1 + strlen(loc + toRemoveLen));
			}
		}
		{
			int writeLoc = 0;
			char sLine[1024];
			
			for (int i = 0; i < 1024 && line[i] != 0; i++)
			{
				char c = line[i];
				bool skip = false;
				for (int j = 0; j < ARRAYSIZE(toRemove); j++)
				{
					if (toRemove[j] == c)
						skip = true;
				}
				if (c == ':')
					c = ' ';
				if (!skip)
					sLine[writeLoc++] = c;
			}
			sLine[writeLoc] = 0;

			char landmarkName[256];
			float3 pos;
			int confidence = 100;
			bool scanValid = true;
			if (hasConfidence)
			{
				int result = sscanf_s(sLine, "%s %d %f %f %f", landmarkName, sizeof(landmarkName), &confidence, &pos.x, &pos.y, &pos.z);
				scanValid = result == 5;
			}
			else
			{
				int result = sscanf_s(sLine, "%s %f %f %f", landmarkName, sizeof(landmarkName), &pos.x, &pos.y, &pos.z);
				scanValid = result == 4;
			}
			
			if (scanValid)
			{
				pos += mVertOffset;
				pos *= mVertScale;
				floatSwap(&pos.y, &pos.z);
				outMap[std::string(landmarkName)] = pos;
			}
		}
	}
	fclose(file);

	// Based on the left/right eye position print out suggested roll and yaw
	if (outMap.find("LANDMARK_EYE_LEFT_CENTER") != outMap.end() && outMap.find("LANDMARK_EYE_RIGHT_CENTER") != outMap.end())
	{
		float3 leftEye = outMap["LANDMARK_EYE_LEFT_CENTER"];
		float3 rightEye = outMap["LANDMARK_EYE_RIGHT_CENTER"];
		float3 delta = (leftEye - rightEye);
		float distance = delta.length();
		float xDistance = abs((rightEye.x - leftEye.x));
		float yDistance = abs((rightEye.y - leftEye.y));
		float zDistance = abs((rightEye.z - leftEye.z));
		float suggestedYaw = asin(zDistance / distance);
		float suggestedRoll = asin(yDistance / distance);
		DebugPrintf("Landmark Suggested Yaw: %0.2f\n", RadToDeg( suggestedYaw));
		DebugPrintf("Landmark Suggested Roll: %0.2f\n", RadToDeg(suggestedRoll));
	}
	


	return outMap.size() != 0;
}

bool CFaceModel::LoadLandmarks()
{
	Landmarks.clear();
	std::string fnString = mObjFilename;
	int lastindex = (int)fnString.find_last_of(".");
	std::string rawname = fnString.substr(0, lastindex);
	std::string jsonName = rawname.append(".json");

	if (LoadLandmarks(jsonName, Landmarks, 0))
		return true;
	return LoadLandmarks(jsonName, Landmarks, 1);
}



```

## Source file: facescan/GameFaceScan.h

{
  "path": "facescan/GameFaceScan.h",
  "blob_sha": "99d6f0c3a1e22ecf49a44659f9e2644dadae64e0",
  "sha256": "0eca7064a2cb171ff0e61f7e12b7b883577085273ec55560c506d2e0eb4f5555",
  "lines": 95,
  "bytes": 2364,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3,
    "face": 44
  },
  "url": "https://github.com/GameTechDev/FaceMapping/blob/e51c509c49d0b2fbab0bba83324ee619d91d1ed8/facescan/GameFaceScan.h",
  "function_signature_candidates": [
    {
      "line": 66,
      "text": "void FaceScan_Init();"
    },
    {
      "line": 67,
      "text": "void FaceScan_Shutdown();"
    },
    {
      "line": 69,
      "text": "bool FaceScan_FrameAcquire();"
    },
    {
      "line": 70,
      "text": "void FaceScan_FrameRelease();"
    },
    {
      "line": 75,
      "text": "void FaceScan_StartScan();"
    },
    {
      "line": 76,
      "text": "void FaceScan_CancelScan();"
    },
    {
      "line": 77,
      "text": "void FaceScan_SaveScan(const char *filename);"
    },
    {
      "line": 79,
      "text": "bool FaceScan_IsScanning();"
    },
    {
      "line": 93,
      "text": "void FaceScan_MoveScanData(const char *srcObjFilename, const char *dstObjFilename);"
    }
  ]
}

```text

/////////////////////////////////////////////////////////////////////////////////////////////
// Copyright 2017 Intel Corporation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or imlied.
// See the License for the specific language governing permissions and
// limitations under the License.
/////////////////////////////////////////////////////////////////////////////////////////////
#ifndef __GAME_FACE_SCAN__
#define __GAME_FACE_SCAN__

#ifndef DISABLE_RSSDK

enum FaceStatusDetected
{
	FaceStatusDetected_Detected,
	FaceStatusDetected_NotDetected
};

enum FaceStatusXAxis
{
	FaceStatusXAxis_InRange,
	FaceStatusXAxis_TooFarLeft,
	FaceStatusXAxis_TooFarRight
};

enum FaceStatusYAxis
{
	FaceStatusYAxis_InRange,
	FaceStatusYAxis_TooFarUp,
	FaceStatusYAxis_TooFarDown
};

enum FaceStatusZAxis
{
	FaceStatusZAxis_InRange,
	FaceStatusZAxis_TooClose,
	FaceStatusZAxis_TooFar
};

enum FaceStatusYaw
{
	FaceStatusYaw_InRange,
	FaceStatusYaw_TooFarLeft,
	FaceStatusYaw_TooFarRight,
};

enum FaceStatusPitch
{
	FaceStatusPitch_InRange,
	FaceStatusPitch_TooFarUp,
	FaceStatusPitch_TooFarDown,
};


#include "pxcimage.h"
#include "pxc3dscan.h"

void FaceScan_Init();
void FaceScan_Shutdown();

bool FaceScan_FrameAcquire();
void FaceScan_FrameRelease();

PXCImage *FaceScan_GetColorImage();
PXCImage *FaceScan_GetScanPreviewImage();

void FaceScan_StartScan();
void FaceScan_CancelScan();
void FaceScan_SaveScan(const char *filename);

bool FaceScan_IsScanning();


FaceStatusDetected FaceScan_StatusDetected();
FaceStatusXAxis FaceScan_StatusXAxis();
FaceStatusYAxis FaceScan_StatusYAxis();
FaceStatusZAxis FaceScan_StatusZAxis();
FaceStatusYaw FaceScan_StatusYaw();
FaceStatusPitch FaceScan_StatusPitch();

PXC3DScan::AlertData FaceScan_GetAlertData();

#endif

void FaceScan_MoveScanData(const char *srcObjFilename, const char *dstObjFilename);

#endif // #ifndef __GAME_FACE_SCAN__

```
