# Facial rendering source scan: LanderK/RealtimeFacialAnimation

Repository: https://github.com/LanderK/RealtimeFacialAnimation

Pinned source revision: 2488a67bb287dbff57cd6a6b5936558b5e442029

Discovery topic: facial animation

Description: 3rd Year Project/Dissertation. I created a Real time animation software capable of animating a 3D model of a face by only using a standard RGB webcam.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["Code/OpenGL/LICENSE.txt"]

Topic paths: {"eye": ["Code/OpenGL/models/RightEyeInnerTemplate.jpg"], "blendshape": ["Code/OpenGL/Blendshape.h"], "mouth": ["Code/OpenGL/models/mouth_LowerLipDepress.obj", "Code/OpenGL/models/mouth_PullOpen.obj", "Code/OpenGL/models/mouth_PullOpen_l.obj", "Code/OpenGL/models/mouth_PullOpen_r.obj", "Code/OpenGL/models/mouth_Pull_l.obj", "Code/OpenGL/models/mouth_Pull_r.obj"], "shader": ["Code/OpenGL/Shader.h"], "rig": ["Code/OpenGL/models/RightEyeInnerTemplate.jpg"], "face": ["Code/OpenGL/models/face01_0.png", "Code/OpenGL/shape_predictor_68_face_landmarks.dat"], "lip": ["Code/OpenGL/models/mouth_LowerLipDepress.obj"]}

## Source file: Code/OpenGL/Shader.h

{
  "path": "Code/OpenGL/Shader.h",
  "blob_sha": "33e7375dd73c31ac496fc2cdfb26b1068e18f91a",
  "sha256": "bd61587b9aea508753741c67f77383933b517da7cf22390e692f25e3a137b4f5",
  "lines": 99,
  "bytes": 2913,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 45
  },
  "url": "https://github.com/LanderK/RealtimeFacialAnimation/blob/2488a67bb287dbff57cd6a6b5936558b5e442029/Code/OpenGL/Shader.h",
  "function_signature_candidates": [
    {
      "line": 93,
      "text": "void Use()"
    }
  ]
}

```text

//From https://learnopengl.com/code_viewer.php?type=header&code=shader
#ifndef SHADER_H
#define SHADER_H

#include <string>
#include <fstream>
#include <sstream>
#include <iostream>

#include <GL/glew.h>

class Shader
{
public:
	GLuint Program;
	// Constructor generates the shader on the fly
	Shader(const GLchar* vertexPath, const GLchar* fragmentPath)
	{
		// 1. Retrieve the vertex/fragment source code from filePath
		std::string vertexCode;
		std::string fragmentCode;
		std::ifstream vShaderFile;
		std::ifstream fShaderFile;
		// ensures ifstream objects can throw exceptions:
		vShaderFile.exceptions(std::ifstream::badbit);
		fShaderFile.exceptions(std::ifstream::badbit);
		try
		{
			// Open files
			vShaderFile.open(vertexPath);
			fShaderFile.open(fragmentPath);
			std::stringstream vShaderStream, fShaderStream;
			// Read file's buffer contents into streams
			vShaderStream << vShaderFile.rdbuf();
			fShaderStream << fShaderFile.rdbuf();
			// close file handlers
			vShaderFile.close();
			fShaderFile.close();
			// Convert stream into string
			vertexCode = vShaderStream.str();
			fragmentCode = fShaderStream.str();
		}
		catch (std::ifstream::failure e)
		{
			std::cout << "ERROR::SHADER::FILE_NOT_SUCCESFULLY_READ" << std::endl;
		}
		const GLchar* vShaderCode = vertexCode.c_str();
		const GLchar * fShaderCode = fragmentCode.c_str();
		// 2. Compile shaders
		GLuint vertex, fragment;
		GLint success;
		GLchar infoLog[512];
		// Vertex Shader
		vertex = glCreateShader(GL_VERTEX_SHADER);
		glShaderSource(vertex, 1, &vShaderCode, NULL);
		glCompileShader(vertex);
		// Print compile errors if any
		glGetShaderiv(vertex, GL_COMPILE_STATUS, &success);
		if (!success)
		{
			glGetShaderInfoLog(vertex, 512, NULL, infoLog);
			std::cout << "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n" << infoLog << std::endl;
		}
		// Fragment Shader
		fragment = glCreateShader(GL_FRAGMENT_SHADER);
		glShaderSource(fragment, 1, &fShaderCode, NULL);
		glCompileShader(fragment);
		// Print compile errors if any
		glGetShaderiv(fragment, GL_COMPILE_STATUS, &success);
		if (!success)
		{
			glGetShaderInfoLog(fragment, 512, NULL, infoLog);
			std::cout << "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n" << infoLog << std::endl;
		}
		// Shader Program
		this->Program = glCreateProgram();
		glAttachShader(this->Program, vertex);
		glAttachShader(this->Program, fragment);
		glLinkProgram(this->Program);
		// Print linking errors if any
		glGetProgramiv(this->Program, GL_LINK_STATUS, &success);
		if (!success)
		{
			glGetProgramInfoLog(this->Program, 512, NULL, infoLog);
			std::cout << "ERROR::SHADER::PROGRAM::LINKING_FAILED\n" << infoLog << std::endl;
		}
		// Delete the shaders as they're linked into our program now and no longer necessery
		glDeleteShader(vertex);
		glDeleteShader(fragment);

	}
	// Uses the current shader
	void Use()
	{
		glUseProgram(this->Program);
	}
};

#endif

```

## Source file: Code/OpenGL/Blendshape.h

{
  "path": "Code/OpenGL/Blendshape.h",
  "blob_sha": "a3b97b99bb75fd669148883d0dfba7c6650729ad",
  "sha256": "ea6adf2fd659d2a2b9da95613d15614c986ac83541c371252e41c7e4bec0e9df",
  "lines": 127,
  "bytes": 3495,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 13,
    "lip": 1
  },
  "url": "https://github.com/LanderK/RealtimeFacialAnimation/blob/2488a67bb287dbff57cd6a6b5936558b5e442029/Code/OpenGL/Blendshape.h",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "void addShape(GLchar* path) {"
    },
    {
      "line": 36,
      "text": "void addShape(std::vector<Vertex> blend) {"
    },
    {
      "line": 40,
      "text": "int getNumberVertices(int i) {"
    },
    {
      "line": 52,
      "text": "void loadModel(std::string path)"
    },
    {
      "line": 66,
      "text": "void processNode(aiNode* node, const aiScene* scene) {"
    },
    {
      "line": 78,
      "text": "void loadModel(std::vector<Vertex> blend) {"
    }
  ]
}

```text

#pragma once
// Std. Includes
#include <string>
#include <fstream>
#include <sstream>
#include <iostream>
#include <map>
#include <vector>

#include <assimp/Importer.hpp>
#include <assimp/scene.h>
#include <assimp/postprocess.h>




class Blendshape
{
public:
	std::vector<Vertex> baseModel;
	std::vector<std::vector<Vertex>> blendshapes;
	/*Functions */
	//Define the Blenshape for a given base shape and a path for the first blenshapes
	Blendshape(GLchar* path, std::vector<Vertex> baseModel) {
		this->baseModel = baseModel;
		this->loadModel(path);
	}
	//Define Blendshape with a base shapes
	Blendshape(std::vector<Vertex> baseModel) {
		this->baseModel = baseModel;
	}
	//Add a new model to the blendshape model
	void addShape(GLchar* path) {
		this->loadModel(path);
	}
	void addShape(std::vector<Vertex> blend) {
		this->loadModel(blend);
	}
	//Get the number of Vertices used in the blendshape 
	int getNumberVertices(int i) {
		return blendshapes[i].size();
	}
	//Function to retrieve a specific blendshape model
	std::vector<Vertex> getBlendShape(int i) {
		return blendshapes[i];
	}
private:
	/* Model Data */
	std::string directory;
	/* Functions */
	//Load Model from file using Assimp Importer
	void loadModel(std::string path)
	{
		Assimp::Importer import;
		const aiScene* scene = import.ReadFile(path, aiProcess_Triangulate | aiProcess_FlipUVs | aiProcess_GenNormals);

		if (!scene || scene->mFlags == AI_SCENE_FLAGS_INCOMPLETE || !scene->mRootNode) {
			std::cout << "ERROR::ASSIMP::" << import.GetErrorString() << std::endl;
			return;
		}
		this->directory = path.substr(0, path.find_last_of('/'));

		this->processNode(scene->mRootNode, scene);

	}
	void processNode(aiNode* node, const aiScene* scene) {
		// Process all the node's meshes (if any)
		for (GLuint i = 0; i < node->mNumMeshes; i++) {
			aiMesh* mesh = scene->mMeshes[node->mMeshes[i]];
			this->blendshapes.push_back(this->processMesh(mesh, scene));
		}

		for (GLuint i = 0; i < node->mNumChildren; i++) {
			this->processNode(node->mChildren[i], scene);
		}
	}
	//Define a Model from another
	void loadModel(std::vector<Vertex> blend) {

		//Remove the base model from the model given 
		for (unsigned int i = 0; i < blend.size(); i++) {
			glm::vec3 vector;
			vector.x = blend[i].Position.x - baseModel[i].Position.x;
			vector.x = blend[i].Position.y - baseModel[i].Position.y;
			vector.x = blend[i].Position.z - baseModel[i].Position.z;
			blend[i].Position = vector;
		}

		blendshapes.push_back(blend);
	}
	std::vector<Vertex> processMesh(aiMesh* mesh, const aiScene* scene)
	{
		assert(baseModel.size() == mesh->mNumVertices);
		std::vector<Vertex> vertices;

		for (GLuint i = 0; i < mesh->mNumVertices; i++) {
			Vertex vertex;
			// Process vertex positions
			// Define the vertices as the change from the mean
			glm::vec3 vector;
			vector.x = mesh->mVertices[i].x - baseModel[i].Position.x;
			vector.y = mesh->mVertices[i].y - baseModel[i].Position.y;
			vector.z = mesh->mVertices[i].z - baseModel[i].Position.z;
			vertex.Position = vector;
			//Normals
			vector.x = mesh->mNormals[i].x;
			vector.y = mesh->mNormals[i].y;
			vector.z = mesh->mNormals[i].z;
			vertex.Normal = vector;
			//Texture
			if (mesh->mTextureCoords[0]) {
				glm::vec2 vec;
				vec.x = mesh->mTextureCoords[0][i].x;
				vec.y = mesh->mTextureCoords[0][i].y;
				vertex.TexCoords = vec;
			}
			else
				vertex.TexCoords = glm::vec2(0.0f, 0.0f);

			vertices.push_back(vertex);

		}

		return vertices;
	}
	
};

```

## Source file: Code/OpenGL/Mesh.h

{
  "path": "Code/OpenGL/Mesh.h",
  "blob_sha": "855c05a636057d9bf32ee7c5e106da18032b7bbf",
  "sha256": "e7a5a447df55e0473db03a3e671e5badd3fa67e77b88c32410e44e042738d267",
  "lines": 129,
  "bytes": 3824,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 4
  },
  "url": "https://github.com/LanderK/RealtimeFacialAnimation/blob/2488a67bb287dbff57cd6a6b5936558b5e442029/Code/OpenGL/Mesh.h",
  "function_signature_candidates": [
    {
      "line": 46,
      "text": "void Draw(Shader shader) {"
    },
    {
      "line": 78,
      "text": "void reshapeMesh(std::vector<Vertex> newVertices) {"
    },
    {
      "line": 87,
      "text": "void setupMesh() {"
    },
    {
      "line": 114,
      "text": "void findMeshBoundingVolume() {"
    }
  ]
}

```text

//From https://learnopengl.com/code_viewer.php?code=mesh&type=header
// Std. Includes
#pragma once
#include <string>
#include <fstream>
#include <sstream>
#include <iostream>
#include <vector>
// GL Includes
#include <GL/glew.h> // Contains all the necessery OpenGL includes
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>
#include <assimp/postprocess.h>

#include "Shader.h"

struct Vertex {
	glm::vec3 Position;
	glm::vec3 Normal;
	glm::vec2 TexCoords;
};

struct Texture {
	GLuint id;
	std::string type;
	aiString path;
};

class Mesh {
public:
	/*  Mesh Data  */
	std::vector<Vertex> vertices;
	std::vector<GLuint> indices;
	std::vector<Texture> textures;
	glm::vec3 max, min;
	/*  Functions  */
	Mesh(std::vector<Vertex> vertices, std::vector<GLuint> indices, std::vector<Texture> textures) {
		this->vertices = vertices;
		this->indices = indices;
		this->textures = textures;

		this->setupMesh();
		this->findMeshBoundingVolume();

	}
	void Draw(Shader shader) {
		GLuint diffuseNr = 1;
		GLuint specularNr = 1;
		for (GLuint i = 0; i < this->textures.size(); i++) {
			glActiveTexture(GL_TEXTURE0 + i);
			std::stringstream ss;
			std::string number;
			std::string name = this->textures[i].type;
			if (name == "texture_diffuse")
				ss << diffuseNr++;
			else if (name == "texture_specular")
				ss << specularNr++;
			number = ss.str();
			name = (name + number);
			glUniform1i(glGetUniformLocation(shader.Program, name.c_str()), i);
			glBindTexture(GL_TEXTURE_2D, this->textures[i].id);
		}

		glUniform1f(glGetUniformLocation(shader.Program, "material.shininess"), 16.0f);
		
		//Draw Mesh
		glBindVertexArray(this->VAO);
		glBindBuffer(GL_ARRAY_BUFFER, this->VBO);
		glBufferSubData(GL_ARRAY_BUFFER, 0, this->vertices.size() * sizeof(Vertex), &this->vertices[0]);
		glDrawElements(GL_TRIANGLES, this->indices.size(), GL_UNSIGNED_INT, 0);
		glBindVertexArray(0);

		for (GLuint i = 0; i < this->textures.size(); i++) {
			glActiveTexture(GL_TEXTURE0 + i);
			glBindTexture(GL_TEXTURE_2D, 0);
		}
	}
	void reshapeMesh(std::vector<Vertex> newVertices) {
		this->vertices = newVertices;
		//this->setupMesh();
	}
private:
	/*  Render data  */
	GLuint VAO, VBO, EBO;
	
	/*  Functions   */
	void setupMesh() {
		glGenVertexArrays(1, &this->VAO);
		glGenBuffers(1, &this->VBO);
		glGenBuffers(1, &this->EBO);

		glBindVertexArray(this->VAO);
		glBindBuffer(GL_ARRAY_BUFFER, this->VBO);

		glBufferData(GL_ARRAY_BUFFER, this->vertices.size() * sizeof(Vertex), &this->vertices[0], GL_DYNAMIC_DRAW);

		glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, this->EBO);
		glBufferData(GL_ELEMENT_ARRAY_BUFFER, this->indices.size() * sizeof(GLuint), &this->indices[0], GL_DYNAMIC_DRAW);

		//Vertex Postions
		glEnableVertexAttribArray(0);
		glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, sizeof(Vertex), (GLvoid*)0);

		//Vertex Normals
		glEnableVertexAttribArray(1);
		glVertexAttribPointer(1, 3, GL_FLOAT, GL_FALSE, sizeof(Vertex), (GLvoid*)offsetof(Vertex, Normal));

		//Vertex Texture Coords
		glEnableVertexAttribArray(2);
		glVertexAttribPointer(2, 2, GL_FLOAT, GL_FALSE, sizeof(Vertex), (GLvoid*)offsetof(Vertex, TexCoords));

		glBindVertexArray(0);
	}
	void findMeshBoundingVolume() {
		min = vertices[0].Position;
		max = vertices[0].Position;
		for (GLuint i = 1; i < this->vertices.size(); i++) {
			//min and max x
			if (vertices[i].Position.x < min.x) min.x = vertices[i].Position.x;
			else if (vertices[i].Position.x > max.x) max.x = vertices[i].Position.x;
			//min and max y
			if (vertices[i].Position.y < min.y) min.y = vertices[i].Position.y;
			else if (vertices[i].Position.y > max.y) max.y = vertices[i].Position.y;
			//min and max z
			if (vertices[i].Position.z < min.z) min.z = vertices[i].Position.z;
			else if (vertices[i].Position.z > max.z) max.z = vertices[i].Position.z;
		}
	}
};

```

## Source file: Code/OpenGL/Model.h

{
  "path": "Code/OpenGL/Model.h",
  "blob_sha": "04e49d92ebb381ad1664e4d3fbda66a9814baed5",
  "sha256": "a033b706fde5c847b075cd6f9767f0e2f228ab2caa7ffb2958661104c6ab5d8c",
  "lines": 210,
  "bytes": 5950,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 3,
    "face": 4,
    "lip": 1
  },
  "url": "https://github.com/LanderK/RealtimeFacialAnimation/blob/2488a67bb287dbff57cd6a6b5936558b5e442029/Code/OpenGL/Model.h",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "void Draw(Shader shader) {"
    },
    {
      "line": 35,
      "text": "glm::vec3 getMinValues() {"
    },
    {
      "line": 39,
      "text": "glm::vec3 getMaxValues() {"
    },
    {
      "line": 42,
      "text": "int getNumberVertices(int i) {"
    },
    {
      "line": 64,
      "text": "void loadModel(std::string path)"
    },
    {
      "line": 78,
      "text": "void processNode(aiNode* node, const aiScene* scene) {"
    },
    {
      "line": 167,
      "text": "void findModelBoundingVolume() {"
    }
  ]
}

```text

#pragma once
// Std. Includes
#include <string>
#include <fstream>
#include <sstream>
#include <iostream>
#include <map>
#include <vector>

#include <assimp/Importer.hpp>
#include <assimp/scene.h>
#include <assimp/postprocess.h>
#include <SOIL.h>

#include "Shader.h"
#include "Mesh.h"



GLint TextureFromFile(const char* path, std::string directory);

class Model
{
public:
	/*Functions */
	Model(GLchar* path) {
		this->loadModel(path);
		this->findModelBoundingVolume();
	}
	void Draw(Shader shader) {
		for (GLuint i = 0; i < this->meshes.size(); i++) {
			this->meshes[i].Draw(shader);
		}
	}
	glm::vec3 getMinValues() {
		return min;
	}

	glm::vec3 getMaxValues() {
		return max;
	}
	int getNumberVertices(int i) {
		return meshes[i].vertices.size();
	}
	std::vector<Vertex> getVertices(int i){
		return meshes[i].vertices;
	}
	std::vector<GLuint> getIndices(int i){
		return meshes[i].indices;
	}
	std::vector<Texture> getTextures(int i){
		return meshes[i].textures;
	}
	Mesh getMesh(int i) {
		return meshes[i];
	}
private:
	/* Model Data */
	std::vector<Mesh> meshes;
	std::string directory;
	std::vector<Texture> textures_loaded;
	glm::vec3 min, max;
	/* Functions */
	void loadModel(std::string path) 
	{
		Assimp::Importer import;
		const aiScene* scene = import.ReadFile(path, aiProcess_Triangulate | aiProcess_FlipUVs | aiProcess_GenNormals);

		if (!scene || scene->mFlags == AI_SCENE_FLAGS_INCOMPLETE || !scene->mRootNode) {
			std::cout << "ERROR::ASSIMP::" << import.GetErrorString() << std::endl;
			return;
		}
		this->directory = path.substr(0, path.find_last_of('/'));

		this->processNode(scene->mRootNode, scene);

	}
	void processNode(aiNode* node, const aiScene* scene) {
		// Process all the node's meshes (if any)
		for (GLuint i = 0; i < node->mNumMeshes; i++) {
			aiMesh* mesh = scene->mMeshes[node->mMeshes[i]];
			this->meshes.push_back(this->processMesh(mesh, scene));
		}

		for (GLuint i = 0; i < node->mNumChildren; i++) {
			this->processNode(node->mChildren[i], scene);
		}
	}
	Mesh processMesh(aiMesh* mesh, const aiScene* scene) 
	{
		std::vector<Vertex> vertices;
		std::vector<GLuint> indices;
		std::vector<Texture> textures;

		for (GLuint i = 0; i < mesh->mNumVertices; i++) {
			Vertex vertex;
			// Process vertex positions
			glm::vec3 vector;
			vector.x = mesh->mVertices[i].x;
			vector.y = mesh->mVertices[i].y;
			vector.z = mesh->mVertices[i].z;
			vertex.Position = vector;
			//Normals
			vector.x = mesh->mNormals[i].x;
			vector.y = mesh->mNormals[i].y;
			vector.z = mesh->mNormals[i].z;
			vertex.Normal = vector;
			//Texture
			if (mesh->mTextureCoords[0]) {
				glm::vec2 vec;
				vec.x = mesh->mTextureCoords[0][i].x;
				vec.y = mesh->mTextureCoords[0][i].y;
				vertex.TexCoords = vec;
			}
			else {
				vertex.TexCoords = glm::vec2(0.0f, 0.0f);
			}

			vertices.push_back(vertex);

		}
		// Process indices
		for (GLuint i = 0; i < mesh->mNumFaces; i++) {
			aiFace face = mesh->mFaces[i];
			for (GLuint j = 0; j < face.mNumIndices; j++) {
				indices.push_back(face.mIndices[j]);
			}
		}

		// Process material
		if (mesh->mMaterialIndex > 0) {
			aiMaterial* material = scene->mMaterials[mesh->mMaterialIndex];
			std::vector<Texture> diffuseMaps = this->loadMaterialTextures(material, aiTextureType_DIFFUSE, "texture_diffuse");
			textures.insert(textures.end(), diffuseMaps.begin(), diffuseMaps.end());
			std::vector<Texture> specularMaps = this->loadMaterialTextures(material, aiTextureType_SPECULAR, "texture_specular");
			textures.insert(textures.end(), specularMaps.begin(), specularMaps.end());
		}

		return Mesh(vertices, indices, textures);
	}
	std::vector<Texture> loadMaterialTextures(aiMaterial* mat, aiTextureType type, std::string typeName) {
		
		std::vector<Texture> textures;
		for (GLuint i = 0; i < mat->GetTextureCount(type); i++) {
			aiString str;
			mat->GetTexture(type, i, &str);
			GLboolean skip = false;
			for (GLuint j = 0; j < textures_loaded.size(); j++) {
				if (textures_loaded[j].path == str) {
					textures.push_back(textures_loaded[j]);
					skip = true;
					break;
				}
			}
			if (!skip) {
				Texture texture;
				texture.id = TextureFromFile(str.C_Str(), this->directory);
				texture.type = typeName;
				texture.path = str;
				textures.push_back(texture);
				this->textures_loaded.push_back(texture);
			}
			
		}
		return textures;
	}
	void findModelBoundingVolume() {
		this->min = meshes[0].min;
		this->max = meshes[0].max;
		for (GLuint i = 1; i < this->meshes.size(); i++) {
			//min and max x
			if (meshes[i].min.x < min.x) min.x = meshes[i].min.x;
			else if (meshes[i].max.x > max.x) max.x = meshes[i].max.x;
			//min and max y
			if (meshes[i].min.y < min.y) min.y = meshes[i].min.y;
			else if (meshes[i].max.y > max.y) max.y = meshes[i].max.y;
			//min and max z
			if (meshes[i].min.z < min.z) min.z = meshes[i].min.z;
			else if (meshes[i].max.z > max.z) max.z = meshes[i].max.z;
		}
	
	}
};

GLint TextureFromFile(const char* path, std::string directory) {

	std::string filename = std::string(path);
	filename = directory + "/" + filename;
	std::cout << filename << std::endl; 
	GLuint textureID;
	glGenTextures(1, &textureID);
	int width, height;
	unsigned char* image = SOIL_load_image(filename.c_str(), &width, &height, 0, SOIL_LOAD_RGB);
	std::cout << "width = " << width << std::endl;
	std::cout << "height = "<< height << std::endl;
	//Assign texture to ID
	glBindTexture(GL_TEXTURE_2D, textureID);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, width, height, 0, GL_RGB, GL_UNSIGNED_BYTE, image);
	glGenerateMipmap(GL_TEXTURE_2D);

	//Parameters
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
	glBindTexture(GL_TEXTURE_2D, 0);
	SOIL_free_image_data(image);
	return textureID;

}

```
