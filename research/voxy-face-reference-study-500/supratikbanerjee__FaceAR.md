# Facial rendering source scan: supratikbanerjee/FaceAR

Repository: https://github.com/supratikbanerjee/FaceAR

Pinned source revision: 5afa68dfe6a825a837edfe57dfb425bd63c45bdb

Discovery topic: blendshape

Description: Real-Time Blendshape Manipulation with Facial Landmark Detection from 2D Video

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["Face AR_ Real-Time Blendshape Manipulation with Facial Landmark Detection from 2D Video.pdf"], "shader": ["Shader.cpp", "Shader.h"], "face": ["Face AR_ Real-Time Blendshape Manipulation with Facial Landmark Detection from 2D Video.pdf", "FaceAR.cpp"]}

## Source file: Shader.h

{
  "path": "Shader.h",
  "blob_sha": "bbe976e41a8c1088f243951cd1221efe3bf14f5a",
  "sha256": "715359235958606669080c862af47c54f68a060536f8f655dc28218e8959dcce",
  "lines": 33,
  "bytes": 886,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 8
  },
  "url": "https://github.com/supratikbanerjee/FaceAR/blob/5afa68dfe6a825a837edfe57dfb425bd63c45bdb/Shader.h",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "void AddShader(GLuint ShaderProgram, const char* pShaderText, GLenum ShaderType);"
    },
    {
      "line": 19,
      "text": "void CompileShaders(const char* vertexPath, const char* fragmentPath);"
    },
    {
      "line": 21,
      "text": "void use();"
    },
    {
      "line": 22,
      "text": "void setBool(const std::string &name, bool value) const;"
    },
    {
      "line": 23,
      "text": "void setInt(const std::string &name, int value) const;"
    },
    {
      "line": 24,
      "text": "void setFloat(const std::string &name, float value) const;"
    },
    {
      "line": 26,
      "text": "void setMat4(const std::string &name, glm::mat4 value) const;"
    },
    {
      "line": 27,
      "text": "void setVec3(const std::string &name, glm::vec3 value) const;"
    },
    {
      "line": 30,
      "text": "void setAttribVec3(GLuint &, unsigned int &) const;"
    }
  ]
}

```text

#ifndef SHADER_H
#define SHADER_H

#include <glad/glad.h>
#include <glm/glm.hpp>
#include <string>

class Shader
{
public:
	unsigned int shaderProgramID;

	Shader(const char* vertexPath, const char* fragmentPath);

	char* readShaderSource(const char* shaderFile);

	void AddShader(GLuint ShaderProgram, const char* pShaderText, GLenum ShaderType);

	void CompileShaders(const char* vertexPath, const char* fragmentPath);

	void use();
	void setBool(const std::string &name, bool value) const;
	void setInt(const std::string &name, int value) const;
	void setFloat(const std::string &name, float value) const;

	void setMat4(const std::string &name, glm::mat4 value) const;
	void setVec3(const std::string &name, glm::vec3 value) const;

	GLuint getAttribLocation(const std::string &name);
	void setAttribVec3(GLuint &, unsigned int &) const;

};
#endif


```

## Source file: FaceAR.cpp

{
  "path": "FaceAR.cpp",
  "blob_sha": "f61d6f499e370e116f27994607552cc626f46464",
  "sha256": "7c3ed56ed67402c75906a7a80eccfcec9af2c8f428c1a25bfb473f6d2f09880a",
  "lines": 338,
  "bytes": 10669,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 4,
    "blendshape": 22,
    "jaw": 2,
    "wrinkle": 2,
    "shader": 26,
    "face": 17
  },
  "url": "https://github.com/supratikbanerjee/FaceAR/blob/5afa68dfe6a825a837edfe57dfb425bd63c45bdb/FaceAR.cpp",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "void framebuffer_size_callback(GLFWwindow* window, int width, int height)"
    },
    {
      "line": 50,
      "text": "void processInput(GLFWwindow *window)"
    },
    {
      "line": 68,
      "text": "void generateObjectBuffer(Eigen::VectorXf &blendfaces, Eigen::VectorXf &Nblendfaces, Shader &shader)"
    },
    {
      "line": 92,
      "text": "void updateObjectBuffer(Eigen::VectorXf &blendfaces, Eigen::VectorXf &Nblendfaces, Shader &shader)"
    },
    {
      "line": 109,
      "text": "void blendFaces(Eigen::VectorXf &blendfaces, Eigen::MatrixXf &facesXf, Eigen::VectorXf &weightsXf)"
    },
    {
      "line": 114,
      "text": "void NblendFaces(Eigen::VectorXf &Nblendfaces, Eigen::MatrixXf &facesXf, Eigen::VectorXf &weightsXf)"
    },
    {
      "line": 119,
      "text": "void manageScene(float* weight)"
    },
    {
      "line": 130,
      "text": "void display(Shader &shader, float* weight) {"
    },
    {
      "line": 160,
      "text": "void init(Shader &shader, Eigen::VectorXf &out_blendWeights, Eigen::MatrixXf &out_Blendshapes, Eigen::MatrixXf &out_BlendshapesNormals, Eigen::VectorXf& out_Neutral)"
    },
    {
      "line": 269,
      "text": "int main()"
    }
  ]
}

```text

#include <windows.h>
#include <vector>
#include <glad/glad.h>
#include <iostream>
#include <glm\glm.hpp>
#include <glm\gtc\matrix_transform.hpp>
#include <Eigen/Sparse>
#include <Eigen/Dense>
#include <GLFW/glfw3.h>
#include "Calibration.h"
#include "LandmarkDetector.h"
#include "Shader.h"
#include "Camera.h"
#include "ObjectLoader.h"
#include <future>

int width = 800;
int height = 450;

float* weight;

std::vector<unsigned int> indices;
std::vector < float > vertex_count;


GLuint positionID;
GLuint normalID;
GLuint vp_vbo1 = 0;
GLuint vn_vbo1 = 0;
GLuint EBO = 0;
GLuint vao = 0;

unsigned int ModelCount = 0;
std::vector < glm::mat4 > local;
std::vector < glm::mat4 > global;
std::vector < glm::vec3 > objc;

glm::vec3 lpos;

ObjectLoader load;
Calibration calibrate;
Camera camera(glm::vec3(0.0f, 20.0f, 60.0f));


void framebuffer_size_callback(GLFWwindow* window, int width, int height)
{
	glViewport(0, 0, width, height);
}

void processInput(GLFWwindow *window)
{
	if (glfwGetKey(window, GLFW_KEY_ESCAPE) == GLFW_PRESS)
		glfwSetWindowShouldClose(window, true);
	if (glfwGetKey(window, GLFW_KEY_C) == GLFW_PRESS)
	{
		calibrate.isCalibrate = true;
		//calibrate.Calibrated = false;
		printf("Starting Calibration\n");
	}
	if (glfwGetKey(window, GLFW_KEY_V) == GLFW_PRESS)
	{
		calibrate.isCalibrate = false;
		printf("Stoping Calibration\n");
	}
}


void generateObjectBuffer(Eigen::VectorXf &blendfaces, Eigen::VectorXf &Nblendfaces, Shader &shader)
{
	glGenVertexArrays(1, &vao);
	glBindVertexArray(vao);

	glGenBuffers(1, &vp_vbo1);
	glBindBuffer(GL_ARRAY_BUFFER, vp_vbo1);
	glBufferData(GL_ARRAY_BUFFER, blendfaces.size() * sizeof(float), blendfaces.data(), GL_DYNAMIC_DRAW);

	glGenBuffers(1, &vn_vbo1);
	glBindBuffer(GL_ARRAY_BUFFER, vn_vbo1);
	glBufferData(GL_ARRAY_BUFFER, Nblendfaces.size() * sizeof(float), Nblendfaces.data(), GL_DYNAMIC_DRAW);

	glGenBuffers(1, &EBO);
	glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, EBO);
	glBufferData(GL_ELEMENT_ARRAY_BUFFER, indices.size() * sizeof(unsigned int), &indices[0], GL_STATIC_DRAW);

	shader.setAttribVec3(positionID, vp_vbo1);
	shader.setAttribVec3(normalID, vn_vbo1);

	glBindVertexArray(0);
}


void updateObjectBuffer(Eigen::VectorXf &blendfaces, Eigen::VectorXf &Nblendfaces, Shader &shader)
{
	glBindVertexArray(vao);

	glBindBuffer(GL_ARRAY_BUFFER, vp_vbo1);
	glBufferData(GL_ARRAY_BUFFER, blendfaces.size() * sizeof(float), blendfaces.data(), GL_DYNAMIC_DRAW);

	glBindBuffer(GL_ARRAY_BUFFER, vn_vbo1);
	glBufferData(GL_ARRAY_BUFFER, Nblendfaces.size() * sizeof(float), Nblendfaces.data(), GL_DYNAMIC_DRAW);

	glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, EBO);
	glBufferData(GL_ELEMENT_ARRAY_BUFFER, indices.size() * sizeof(unsigned int), &indices[0], GL_STATIC_DRAW);

	shader.setAttribVec3(positionID, vp_vbo1);
	shader.setAttribVec3(normalID, vn_vbo1);
}

void blendFaces(Eigen::VectorXf &blendfaces, Eigen::MatrixXf &facesXf, Eigen::VectorXf &weightsXf) 
{
	blendfaces = facesXf.transpose() * weightsXf;
}

void NblendFaces(Eigen::VectorXf &Nblendfaces, Eigen::MatrixXf &facesXf, Eigen::VectorXf &weightsXf) 
{
	Nblendfaces = facesXf.transpose() * weightsXf;
}

void manageScene(float* weight)
{
	for (int i = 0; i < ModelCount; i++)
		global[i] = local[i];
	//std::cout << weight[7] << std::endl;
	local[0] = glm::rotate(glm::mat4(1.0f), weight[7], glm::vec3(0.0f, 1.0f, 0.0f));
	local[0] = glm::rotate(local[0], weight[8], glm::vec3(1.0f, 0.0f, 0.0f));
	local[0] = glm::rotate(local[0], weight[9], glm::vec3(0.0f, 0.0f, 1.0f));
	global[0] = local[0];
}

void display(Shader &shader, float* weight) {

	glEnable(GL_DEPTH_TEST);
	glDepthFunc(GL_LESS);
	glClearColor(0.15f, 0.15f, 0.15f, 1.0f);
	glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
	shader.use();
	glBindVertexArray(vao);

	float rough = 1.0f;

	glm::mat4 view = camera.GetViewMatrix();
	glm::mat4 persp_proj = glm::perspective(45.0f, (float)width / (float)height, 0.1f, 100.0f);

	shader.setVec3("camView", camera.GetCameraPosition());
	rough = 0.15f;
	shader.setFloat("shine_rough", rough);
	shader.setMat4("proj", persp_proj);
	shader.setMat4("view", view);

	for (int i = 0; i < ModelCount; i++)
	{
		shader.setVec3("objColor", objc[i]);
		manageScene(weight);
		shader.setMat4("model", global[i]);
		glDrawElements(GL_TRIANGLES, indices.size(), GL_UNSIGNED_INT, 0);
		//glDrawArrays(GL_POINT, index_p,0);
	}
}

void init(Shader &shader, Eigen::VectorXf &out_blendWeights, Eigen::MatrixXf &out_Blendshapes, Eigen::MatrixXf &out_BlendshapesNormals, Eigen::VectorXf& out_Neutral)
{
	shader.use();
	positionID = shader.getAttribLocation("vertex_position1");
	normalID = shader.getAttribLocation("vertex_normals1");
	lpos = glm::vec3(0.0f, 20.0f, 20.0f);
	shader.setVec3("position", lpos);
	//GLuint shaderProgramID = CompileShaders();

	const char *c_obj[] = { "cubet.obj" };
	/*const char *obj[] = { "neutral.obj","Mery_jaw_open.obj", "Mery_kiss.obj", "Mery_l_brow_lower.obj", \
  "Mery_l_brow_narrow.obj", "Mery_l_brow_raise.obj", "Mery_l_eye_closed.obj", "Mery_l_eye_lower_open.obj",\
  "Mery_l_eye_upper_open.obj", "Mery_l_nose_wrinkle.obj","Mery_l_puff.obj", "Mery_l_sad.obj", "Mery_l_smile.obj",\
  "Mery_l_suck.obj","Mery_r_brow_lower.obj", "Mery_r_brow_narrow.obj", "Mery_r_brow_raise.obj", \
  "Mery_r_eye_closed.obj", "Mery_r_eye_lower_open.obj","Mery_r_eye_upper_open.obj","Mery_r_nose_wrinkle.obj",\
  "Mery_r_puff.obj","Mery_r_sad.obj","Mery_r_smile.obj","Mery_r_suck.obj" };*/
	const char *obj[] = { "neutral.obj","Mery_l_brow_raise.obj", "Mery_r_brow_raise.obj", "Mery_jaw_open.obj", "Mery_l_smile.obj" , "Mery_r_smile.obj", "Mery_l_eye_closed.obj", "Mery_r_eye_closed.obj"};
	//objc.push_back(glm::vec3(0.95, 0.45, 0.25));
	objc.push_back(glm::vec3(1.0, 1.0, 1.0));
	objc.push_back(glm::vec3(1.0, 0.0, 0.0));
	unsigned int objs = (sizeof(obj) / sizeof(obj[0]));

	ModelCount = 1;

	std::vector <std::vector< glm::vec2 >> uvs;
	std::vector <std::vector< glm::vec3 >> vertices;
	std::vector <std::vector< glm::vec3 >> normals;

	std::vector< glm::vec3 > vert_temp;
	std::vector< glm::vec2 > uv_temp;
	std::vector< glm::vec3 > norm_temp;

	std::vector< glm::vec3 > vert_out;
	std::vector< glm::vec2 > uv_out;
	std::vector< glm::vec3 > norm_out;


	for (int i = 0; i < objs; i++)
	{
		global.push_back(glm::mat4(1.0f));
		local.push_back(glm::mat4(1.0f));
		//local[i] = glm::scale(local[i], glm::vec3(0.05f, 0.05f, 0.05f));
		bool res = load.loadOBJ(obj[i], vert_temp, uv_temp, norm_temp);
		indices.clear();
		load.indexVBO(vert_temp, uv_temp, norm_temp, indices, vert_out, uv_out, norm_out);
		vertices.push_back(vert_out);
		uvs.push_back(uv_out);
		normals.push_back(norm_out);
		// std::cout << vert_temp.size() << std::endl;
		vert_temp.clear();
		uv_temp.clear();
		norm_temp.clear();
		vert_out.clear();
		uv_out.clear();
		norm_out.clear();
		//std::cout << vert_temp.size() << std::endl;
		vertex_count.push_back(vertices[0].size());
		//printf("count %d\n", vertices[i].size());
	}
	printf("%d\n", indices.size());

	Eigen::VectorXf blendWeights(objs);
	Eigen::MatrixXf Blendshapes(objs, (int)vertex_count[0] * 3);
	Eigen::MatrixXf BlendshapesNormals(objs, (int)vertex_count[0] * 3);
	Eigen::VectorXf neutralV((int)vertex_count[0] * 3);
	Eigen::VectorXf neutralN((int)vertex_count[0] * 3);

	for (int i = 0; i < objs; i++)
		blendWeights(i) = 0;
	blendWeights(0) = 1.0;

	int v = 0;

	for (int i = 0; i < vertex_count[0] * 3; i += 3)
	{
		neutralV(i) = Blendshapes(0, i) = vertices[0][v].x;
		neutralV(i + 1) = Blendshapes(0, i + 1) = vertices[0][v].y;
		neutralV(i + 2) = Blendshapes(0, i + 2) = vertices[0][v].z;

		neutralN(i) = BlendshapesNormals(0, i) = normals[0][v].x;
		neutralN(i + 1) = BlendshapesNormals(0, i + 1) = normals[0][v].y;
		neutralN(i + 2) = BlendshapesNormals(0, i + 2) = normals[0][v].z;
		v++;
	}

	for (int k = 1; k < objs; k++)
	{
		v = 0;
		for (int i = 0; i < vertex_count[0] * 3; i += 3)
		{
			Blendshapes(k, i) = vertices[k][v].x - Blendshapes(0, i);
			Blendshapes(k, i + 1) = vertices[k][v].y - Blendshapes(0, i + 1);
			Blendshapes(k, i + 2) = vertices[k][v].z - Blendshapes(0, i + 2);

			BlendshapesNormals(k, i) = normals[k][v].x - BlendshapesNormals(0, i);
			BlendshapesNormals(k, i + 1) = normals[k][v].y - BlendshapesNormals(0, i + 1);
			BlendshapesNormals(k, i + 2) = normals[k][v].z - BlendshapesNormals(0, i + 2);
			v++;
		}
	}

	out_blendWeights = blendWeights;
	out_Blendshapes = Blendshapes;
	out_BlendshapesNormals = BlendshapesNormals;
	out_Neutral = neutralV;
	generateObjectBuffer(neutralV, neutralN, shader);
}


int main()
{
	glfwInit();
	glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
	glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
	glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);

	GLFWwindow* window = glfwCreateWindow(width, height, "Xtreme Render Engine FaceAR Plugin", NULL, NULL);
	if (window == NULL)
	{
		std::cout << "FaceAR Plugin Error" << std::endl;
		glfwTerminate();
		return -1;
	}

	glfwMakeContextCurrent(window);
	glfwSetFramebufferSizeCallback(window, framebuffer_size_callback);

	if (!gladLoadGLLoader((GLADloadproc)glfwGetProcAddress))
	{
		std::cout << "Failed to initialize GLAD" << std::endl;
		return -1;
	}

	glEnable(GL_DEPTH_TEST);

	Shader shader("../Shaders/rta.vs", "../Shaders/oren-nayar_cook-torrance_frag.glsl");
	LandmarkDetector landmark = LandmarkDetector(&calibrate);

	Eigen::MatrixXf Blendshapes;
	Eigen::MatrixXf BlendshapesNormals;
	Eigen::VectorXf blendWeights;

	Eigen::VectorXf blendfaces;
	Eigen::VectorXf Nblendfaces;
	Eigen::VectorXf Neutral;

	glm::vec3 position;
	glm::vec3 first_pos;
	glm::vec3 last_pos;

	init(shader, blendWeights, Blendshapes, BlendshapesNormals, Neutral);

	while (!glfwWindowShouldClose(window))
	{
		//std::thread th1(landmark.Capture(), NULL);
		weight = landmark.Capture();
		//printf("%f %f\n", weight[0], weight[1]);

		glfwPollEvents();
		processInput(window);
		for (int i = 0; i < 7; i++)
		{
			weight[i] = weight[i] < 0 ? 0 : weight[i];
			weight[i] = weight[i] > 1 ? 1 : weight[i];
			blendWeights(i+1) = weight[i];
		}

		display(shader, weight);
		blendFaces(blendfaces, Blendshapes, blendWeights);
		NblendFaces(Nblendfaces, BlendshapesNormals, blendWeights);
		updateObjectBuffer(blendfaces, Nblendfaces, shader);
		glfwSwapInterval(0);
		glfwSwapBuffers(window);
	}

	glfwTerminate();
	getchar();
	return 0;
}

```

## Source file: Shader.cpp

{
  "path": "Shader.cpp",
  "blob_sha": "705dafcd674741b7296bf271b8efeccda150dbef",
  "sha256": "01e5c3ff5835cd4eb10ab62adf4639bf610e76251351846b10c65004dcd475bc",
  "lines": 120,
  "bytes": 3691,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 49
  },
  "url": "https://github.com/supratikbanerjee/FaceAR/blob/5afa68dfe6a825a837edfe57dfb425bd63c45bdb/Shader.cpp",
  "function_signature_candidates": []
}

```text

#include "Shader.h"

char* Shader::readShaderSource(const char* shaderFile) {
	FILE* fp = fopen(shaderFile, "rb"); //!->Why does binary flag "RB" work and not "R"... wierd msvc thing?

	if (fp == NULL) { return NULL; }

	fseek(fp, 0L, SEEK_END);
	long size = ftell(fp);

	fseek(fp, 0L, SEEK_SET);
	char* buf = new char[size + 1];
	fread(buf, 1, size, fp);
	buf[size] = '\0';

	fclose(fp);

	return buf;
}

void Shader::AddShader(GLuint ShaderProgram, const char* pShaderText, GLenum ShaderType)
{
	GLuint ShaderObj = glCreateShader(ShaderType);

	if (ShaderObj == 0) {
		fprintf(stderr, "Error creating shader type %d\n", ShaderType);
		exit(0);
	}
	const char* pShaderSource = readShaderSource(pShaderText);

	// Bind the source code to the shader, this happens before compilation
	glShaderSource(ShaderObj, 1, (const GLchar**)&pShaderSource, NULL);
	// compile the shader and check for errors
	glCompileShader(ShaderObj);
	GLint success;
	// check for shader related errors using glGetShaderiv
	glGetShaderiv(ShaderObj, GL_COMPILE_STATUS, &success);
	if (!success) {
		GLchar InfoLog[1024];
		glGetShaderInfoLog(ShaderObj, 1024, NULL, InfoLog);
		fprintf(stderr, "Error compiling shader type %d: '%s'\n", ShaderType, InfoLog);
		exit(1);
	}
	// Attach the compiled shader object to the program object
	glAttachShader(ShaderProgram, ShaderObj);
}

void Shader::CompileShaders(const char* vertexPath, const char* fragmentPath)
{
	shaderProgramID = glCreateProgram();
	if (shaderProgramID == 0) {
		fprintf(stderr, "Error creating shader program\n");
		exit(1);
	}

	AddShader(shaderProgramID, vertexPath, GL_VERTEX_SHADER);
	AddShader(shaderProgramID, fragmentPath, GL_FRAGMENT_SHADER);

	GLint Success = 0;
	GLchar ErrorLog[1024] = { 0 };
	glLinkProgram(shaderProgramID);
	glGetProgramiv(shaderProgramID, GL_LINK_STATUS, &Success);
	if (Success == 0) {
		glGetProgramInfoLog(shaderProgramID, sizeof(ErrorLog), NULL, ErrorLog);
		fprintf(stderr, "Error linking shader program: '%s'\n", ErrorLog);
		exit(1);
	}

	glValidateProgram(shaderProgramID);
	glGetProgramiv(shaderProgramID, GL_VALIDATE_STATUS, &Success);
	if (!Success) {
		glGetProgramInfoLog(shaderProgramID, sizeof(ErrorLog), NULL, ErrorLog);
		fprintf(stderr, "Invalid shader program: '%s'\n", ErrorLog);
		exit(1);
	}
}

Shader::Shader(const char* vertexPath, const char* fragmentPath)
{
	CompileShaders(vertexPath, fragmentPath);
}

void Shader::use()
{
	glUseProgram(shaderProgramID);
}
void Shader::setBool(const std::string &name, bool value) const
{
	glUniform1i(glGetUniformLocation(shaderProgramID, name.c_str()), (int)value);
}
void Shader::setInt(const std::string &name, int value) const
{
	glUniform1i(glGetUniformLocation(shaderProgramID, name.c_str()), value);
}
void Shader::setFloat(const std::string &name, float value) const
{
	glUniform1fv(glGetUniformLocation(shaderProgramID, name.c_str()), 1, &value);
}

void Shader::setMat4(const std::string &name, glm::mat4 value) const
{
	glUniformMatrix4fv(glGetUniformLocation(shaderProgramID, name.c_str()), 1, GL_FALSE, &value[0][0]);
}

void Shader::setVec3(const std::string &name, glm::vec3 value) const
{
	glUniform3fv(glGetUniformLocation(shaderProgramID, name.c_str()), 1, &value[0]);
}

GLuint Shader::getAttribLocation(const std::string &name)
{
	return glGetAttribLocation(shaderProgramID, name.c_str());
}

void Shader::setAttribVec3(GLuint &AttricLocatin, unsigned int &VBO) const
{
	glEnableVertexAttribArray(AttricLocatin);
	glBindBuffer(GL_ARRAY_BUFFER, VBO);
	glVertexAttribPointer(AttricLocatin, 3, GL_FLOAT, GL_FALSE, 0, NULL);
}

```

## Source file: Camera.h

{
  "path": "Camera.h",
  "blob_sha": "47366c28c617ef18054f2e65feb3bc2a318438ac",
  "sha256": "d342f22f263fab6de68a8ef88e0c36e9e12ca5dcb30bfe739752fed2e5ecef49",
  "lines": 138,
  "bytes": 4094,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 9,
    "lip": 1
  },
  "url": "https://github.com/supratikbanerjee/FaceAR/blob/5afa68dfe6a825a837edfe57dfb425bd63c45bdb/Camera.h",
  "function_signature_candidates": [
    {
      "line": 71,
      "text": "glm::vec3 GetCameraPosition()"
    },
    {
      "line": 77,
      "text": "void ProcessKeyboard(Camera_Movement direction, float deltaTime)"
    },
    {
      "line": 91,
      "text": "void ProcessMouseMovement(float xoffset, float yoffset, GLboolean constrainPitch = true)"
    },
    {
      "line": 113,
      "text": "void ProcessMouseScroll(float yoffset)"
    },
    {
      "line": 125,
      "text": "void updateCameraVectors()"
    }
  ]
}

```text

#ifndef CAMERA_H
#define CAMERA_H

// From Learn OpenGL

#include <glad/glad.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>

#include <vector>

// Defines several possible options for camera movement. Used as abstraction to stay away from window-system specific input methods
enum Camera_Movement {
	FORWARD,
	BACKWARD,
	LEFT,
	RIGHT
};

// Default camera values
const float YAW = -90.0f;
const float PITCH = 0.0f;
const float SPEED = 2.5f;
const float SENSITIVITY = 0.1f;
const float ZOOM = 45.0f;


// An abstract camera class that processes input and calculates the corresponding Euler Angles, Vectors and Matrices for use in OpenGL
class Camera
{
public:
	// Camera Attributes
	glm::vec3 Position;
	glm::vec3 Front;
	glm::vec3 Up;
	glm::vec3 Right;
	glm::vec3 WorldUp;
	// Euler Angles
	float Yaw;
	float Pitch;
	// Camera options
	float MovementSpeed;
	float MouseSensitivity;
	float Zoom;

	// Constructor with vectors
	Camera(glm::vec3 position = glm::vec3(0.0f, 0.0f, 0.0f), glm::vec3 up = glm::vec3(0.0f, 1.0f, 0.0f), float yaw = YAW, float pitch = PITCH) : Front(glm::vec3(0.0f, 0.0f, -1.0f)), MovementSpeed(SPEED), MouseSensitivity(SENSITIVITY), Zoom(ZOOM)
	{
		Position = position;
		WorldUp = up;
		Yaw = yaw;
		Pitch = pitch;
		updateCameraVectors();
	}
	// Constructor with scalar values
	Camera(float posX, float posY, float posZ, float upX, float upY, float upZ, float yaw, float pitch) : Front(glm::vec3(0.0f, 0.0f, -1.0f)), MovementSpeed(SPEED), MouseSensitivity(SENSITIVITY), Zoom(ZOOM)
	{
		Position = glm::vec3(posX, posY, posZ);
		WorldUp = glm::vec3(upX, upY, upZ);
		Yaw = yaw;
		Pitch = pitch;
		updateCameraVectors();
	}

	// Returns the view matrix calculated using Euler Angles and the LookAt Matrix
	glm::mat4 GetViewMatrix()
	{
		return glm::lookAt(Position, Position + Front, Up);
	}

	glm::vec3 GetCameraPosition()
	{
		return Position;
	}

	// Processes input received from any keyboard-like input system. Accepts input parameter in the form of camera defined ENUM (to abstract it from windowing systems)
	void ProcessKeyboard(Camera_Movement direction, float deltaTime)
	{
		float velocity = MovementSpeed * deltaTime;
		if (direction == FORWARD)
			Position += Front * velocity;
		if (direction == BACKWARD)
			Position -= Front * velocity;
		if (direction == LEFT)
			Position -= Right * velocity;
		if (direction == RIGHT)
			Position += Right * velocity;
	}

	// Processes input received from a mouse input system. Expects the offset value in both the x and y direction.
	void ProcessMouseMovement(float xoffset, float yoffset, GLboolean constrainPitch = true)
	{
		xoffset *= MouseSensitivity;
		yoffset *= MouseSensitivity;

		Yaw += xoffset;
		Pitch += yoffset;

		// Make sure that when pitch is out of bounds, screen doesn't get flipped
		if (constrainPitch)
		{
			if (Pitch > 89.0f)
				Pitch = 89.0f;
			if (Pitch < -89.0f)
				Pitch = -89.0f;
		}

		// Update Front, Right and Up Vectors using the updated Euler angles
		updateCameraVectors();
	}

	// Processes input received from a mouse scroll-wheel event. Only requires input on the vertical wheel-axis
	void ProcessMouseScroll(float yoffset)
	{
		if (Zoom >= 1.0f && Zoom <= 45.0f)
			Zoom -= yoffset;
		if (Zoom <= 1.0f)
			Zoom = 1.0f;
		if (Zoom >= 45.0f)
			Zoom = 45.0f;
	}

private:
	// Calculates the front vector from the Camera's (updated) Euler Angles
	void updateCameraVectors()
	{
		// Calculate the new Front vector
		glm::vec3 front;
		front.x = cos(glm::radians(Yaw)) * cos(glm::radians(Pitch));
		front.y = sin(glm::radians(Pitch));
		front.z = sin(glm::radians(Yaw)) * cos(glm::radians(Pitch));
		Front = glm::normalize(front);
		// Also re-calculate the Right and Up vector
		Right = glm::normalize(glm::cross(Front, WorldUp));  // Normalize the vectors, because their length gets closer to 0 the more you look up or down which results in slower movement.
		Up = glm::normalize(glm::cross(Right, Front));
	}
};
#endif

```
