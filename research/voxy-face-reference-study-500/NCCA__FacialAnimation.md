# Facial rendering source scan: NCCA/FacialAnimation

Repository: https://github.com/NCCA/FacialAnimation

Pinned source revision: a9b094d138c5eede6641292fa2d062e8e0c73736

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["models/Eyeball.obj"], "jaw": ["models/FaceJawOpen.obj"], "shader": ["shaders/PerFragASDFrag.glsl", "shaders/PerFragASDVert.glsl"], "rig": ["models/FaceRightBrowDown.obj", "models/FaceRightBrowUp.obj", "models/FaceRightFrown.obj", "models/FaceRightSmile.obj", "models/FaceRightSneer.obj"], "face": ["models/FaceCheekPuff.obj", "models/FaceCheekSuck.obj", "models/FaceDefault.obj", "models/FaceJawOpen.obj", "models/FaceKiss.obj", "models/FaceLeftBrowDown.obj", "models/FaceLeftBrowUp.obj", "models/FaceLeftFrown.obj", "models/FaceLeftSmile.obj", "models/FaceLeftSneer.obj", "models/FaceRightBrowDown.obj", "models/FaceRightBrowUp.obj", "models/FaceRightFrown.obj", "models/FaceRightSmile.obj", "models/FaceRightSneer.obj"]}

## Source file: shaders/PerFragASDFrag.glsl

{
  "path": "shaders/PerFragASDFrag.glsl",
  "blob_sha": "93222377e34e540423e1e2e91c35df859dc8ed46",
  "sha256": "73fb47f2047c15c5cdbb11b93fb8e878a034dd054dca3f29fe647285449c1e05",
  "lines": 79,
  "bytes": 1725,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "shader": 1
  },
  "url": "https://github.com/NCCA/FacialAnimation/blob/a9b094d138c5eede6641292fa2d062e8e0c73736/shaders/PerFragASDFrag.glsl",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "vec3 phongModel( )"
    },
    {
      "line": 56,
      "text": "vec3 phongModelHalfVector( )"
    },
    {
      "line": 73,
      "text": "void main ()"
    }
  ]
}

```text

#version 330 core
// this shader is modified from the OpenGL Shading Language cookbook

struct LightInfo
{
	// Light position in eye coords.
	vec3 position;
	// Ambient light intensity
	vec3 La;
	// Diffuse light intensity
	vec3 Ld;
	// Specular light intensity
	vec3 Ls;
};
uniform LightInfo light;

struct MaterialInfo
{
	// Ambient reflectivity
	vec3 Ka;
	// Diffuse reflectivity
	vec3 Kd;
	// Specular reflectivity
	vec3 Ks;
	// Specular shininess factor
	float shininess;
};

uniform MaterialInfo material;



in vec3 position;
in vec3 normal;
in vec4 debugColour;
/// @brief our output fragment colour
layout (location =0)out vec4 fragColour;


vec3 phongModel( )
{
	vec3 s = normalize(light.position - position);
	vec3 v = normalize(-position);
	vec3 r = reflect( -s, normal );
	vec3 ambient = light.La * material.Ka;
	float sDotN = max( dot(s,normal), 0.0 );
	vec3 diffuse = light.Ld * material.Kd * sDotN;
	vec3 spec = vec3(0.0);
	if( sDotN > 0.0 )
	{
		spec = light.Ls * material.Ks * pow( max( dot(r,v), 0.0 ), material.shininess );
	}
	return ambient + diffuse + spec;
}

vec3 phongModelHalfVector( )
{
	vec3 n = normalize(normal);
	vec3 s = normalize(light.position - position);
	vec3 v = normalize(-position);
	vec3 h = normalize( v + s );
	vec3 ambient = light.La * material.Ka;
	float sDotN = max( dot(s,normal), 0.0 );
	vec3 diffuse = light.Ld * material.Kd * sDotN;
	vec3 spec = vec3(0.0);
	if( sDotN > 0.0 )
	{
		spec = light.Ls * material.Ks * pow( max( dot(h,n), 0.0 ), material.shininess );
	}
	return ambient + diffuse + spec;
}

void main ()
{
	fragColour=vec4(	phongModelHalfVector(),1.0);
	//fragColour=debugColour;
}




```

## Source file: shaders/PerFragASDVert.glsl

{
  "path": "shaders/PerFragASDVert.glsl",
  "blob_sha": "bacec09f35bd7920ef6f92051c5e5cb912268a9b",
  "sha256": "8a4d03cf750840c05437fecdaead3e58f42e0bcbcc4e6de40c496a3ba2809920",
  "lines": 62,
  "bytes": 1288,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "lip": 1
  },
  "url": "https://github.com/NCCA/FacialAnimation/blob/a9b094d138c5eede6641292fa2d062e8e0c73736/shaders/PerFragASDVert.glsl",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "void main()"
    }
  ]
}

```text

#version 330 core
// this is base on http://http.developer.nvidia.com/GPUGems3/gpugems3_ch03.html
layout (location =0) in vec3 baseVert;
layout (location =1) in vec3 baseNormal;

// transform matrix values
uniform mat4 MVP;
uniform mat3 normalMatrix;
uniform mat4 MV;

#define meshOffset 28
#define numWeights 14
uniform float weights[numWeights];



out vec3 position;
out vec3 normal;
out vec4 debugColour;
uniform samplerBuffer TBO;
void main()
{

	vec3  finalN;
	vec3  finalP;
	vec3 weightNorm=vec3(0.0f);
	vec3 weightVert=vec3(0.0f);
	// do the verts first
	int i;
	for (i=0; i<numWeights; ++i)
	{
		weightVert+= (texelFetch(TBO,int(meshOffset*gl_VertexID+i)).xyz*weights[i]);
	}

	finalP= baseVert+weightVert;

	for (; i<meshOffset; ++i)
	{
		weightNorm+= (texelFetch(TBO,int(meshOffset*gl_VertexID+i)).xyz*weights[i-numWeights]);
	}

	finalN= baseNormal+weightNorm;

	// then normalize and mult by normal matrix for shading
	normal = normalize( normalMatrix * finalN);
	// now calculate the eye cord position for the frag stage
	position = vec3(MV * vec4(baseVert,1.0));

	//debugColour=vec4(weight3*poseVert3,1);
	// Convert position to clip coordinates and pass along
	gl_Position = MVP*vec4(finalP,1.0);

}











```

## Source file: src/main.cpp

{
  "path": "src/main.cpp",
  "blob_sha": "36986c8aef4611fd97773642213342bdea0bc7d2",
  "sha256": "fd406cf2840524d19e2285548b1c1bdc56252b9ef8ef053f8a9d2cf853c98548",
  "lines": 47,
  "bytes": 1549,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/NCCA/FacialAnimation/blob/a9b094d138c5eede6641292fa2d062e8e0c73736/src/main.cpp",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "int main(int argc, char **argv)"
    }
  ]
}

```text

/****************************************************************************
basic OpenGL demo modified from http://qt-project.org/doc/qt-5.0/qtgui/openglwindow.html
****************************************************************************/
#include <QtGui/QGuiApplication>
#include <iostream>
#include "NGLScene.h"



int main(int argc, char **argv)
{
  QGuiApplication app(argc, argv);
  // create an OpenGL format specifier
  QSurfaceFormat format;
  // set the number of samples for multisampling
  // will need to enable glEnable(GL_MULTISAMPLE); once we have a context
  format.setSamples(4);
  #if defined(__APPLE__)
    // at present mac osx Mountain Lion only supports GL3.2
    // the new mavericks will have GL 4.x so can change
    format.setMajorVersion(4);
    format.setMinorVersion(1);
  #else
    // with luck we have the latest GL version so set to this
    format.setMajorVersion(4);
    format.setMinorVersion(3);
  #endif
  // now we are going to set to CoreProfile OpenGL so we can't use and old Immediate mode GL
  format.setProfile(QSurfaceFormat::CoreProfile);
  // now set the depth buffer to 24 bits
  format.setDepthBufferSize(24);
  // now we are going to create our scene window
  NGLScene window;
  // and set the OpenGL format
  window.setFormat(format);
  // we can now query the version to see if it worked
  std::cout<<"Profile is "<<format.majorVersion()<<" "<<format.minorVersion()<<"\n";
  // set the window size
  window.resize(1024, 720);
  // and finally show
  window.show();

  return app.exec();
}





```

## Source file: src/NGLScene.cpp

{
  "path": "src/NGLScene.cpp",
  "blob_sha": "a68601e3ee39ed1ca84bfc6917513cb97f4a1522",
  "sha256": "ea8cbdd18df181e3903f0ca3f4bd7f883c8110f3c6b88287d7463cdb732a8e4c",
  "lines": 528,
  "bytes": 16930,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 7,
    "blendshape": 1,
    "shader": 43,
    "rig": 20,
    "face": 15,
    "lip": 1
  },
  "url": "https://github.com/NCCA/FacialAnimation/blob/a9b094d138c5eede6641292fa2d062e8e0c73736/src/NGLScene.cpp",
  "function_signature_candidates": []
}

```text

#include <QMouseEvent>
#include <QGuiApplication>

#include "NGLScene.h"
#include <ngl/NGLInit.h>
#include <ngl/VAOPrimitives.h>
#include <ngl/ShaderLib.h>
#include <ngl/Transformation.h>
#include <ngl/pystring.h>
#include <iostream>

NGLScene::NGLScene()
{
  setTitle("Qt5 Simple NGL Demo");
  m_activeWeight = 0;
}

NGLScene::~NGLScene()
{
  std::cout << "Shutting down NGL, removing VAO's and Shaders\n";
}

void NGLScene::resizeGL(int _w, int _h)
{
  m_project = ngl::perspective(45.0f, static_cast<float>(_w) / _h, 0.05f, 350.0f);
  m_win.width = static_cast<int>(_w * devicePixelRatio());
  m_win.height = static_cast<int>(_h * devicePixelRatio());
}

void NGLScene::initializeGL()
{
  ngl::NGLInit::initialize();

  glClearColor(0.4f, 0.4f, 0.4f, 1.0f); // Grey Background
  // enable depth testing for drawing
  glEnable(GL_DEPTH_TEST);
  // Now we will create a basic Camera from the graphics library
  // This is a static camera so it only needs to be set once
  // First create Values for the camera position
  ngl::Vec3 from(0, 1.5, 15);
  ngl::Vec3 to(0, 1.5, 0);
  ngl::Vec3 up(0, 1, 0);
  m_view = ngl::lookAt(from, to, up);
  // set the shape using FOV 45 Aspect Ratio based on Width and Height
  // The final two are near and far clipping planes of 0.5 and 10
  m_project = ngl::perspective(45, (float)720.0 / 576.0, 0.05, 350);
  // we are creating a shader called PerFragADS
  ngl::ShaderLib::createShaderProgram("PerFragADS");
  // now we are going to create empty shaders for Frag and Vert
  ngl::ShaderLib::attachShader("PerFragADSVertex", ngl::ShaderType::VERTEX);
  ngl::ShaderLib::attachShader("PerFragADSFragment", ngl::ShaderType::FRAGMENT);
  // attach the source
  ngl::ShaderLib::loadShaderSource("PerFragADSVertex", "shaders/PerFragASDVert.glsl");
  ngl::ShaderLib::loadShaderSource("PerFragADSFragment", "shaders/PerFragASDFrag.glsl");
  // compile the shaders
  ngl::ShaderLib::compileShader("PerFragADSVertex");
  ngl::ShaderLib::compileShader("PerFragADSFragment");
  // add them to the program
  ngl::ShaderLib::attachShaderToProgram("PerFragADS", "PerFragADSVertex");
  ngl::ShaderLib::attachShaderToProgram("PerFragADS", "PerFragADSFragment");

  // now we have associated this data we can link the shader
  ngl::ShaderLib::linkProgramObject("PerFragADS");
  // and make it active ready to load values
  ngl::ShaderLib::use("PerFragADS");
  // now we need to set the material and light values
  /*
   *struct MaterialInfo
   {
        // Ambient reflectivity
        vec3 Ka;
        // Diffuse reflectivity
        vec3 Kd;
        // Specular reflectivity
        vec3 Ks;
        // Specular shininess factor
        float shininess;
  };*/
  ngl::ShaderLib::setUniform("material.Ka", 0.1f, 0.1f, 0.1f);
  // red diffuse
  ngl::ShaderLib::setUniform("material.Kd", 0.8f, 0.8f, 0.8f);
  // white spec
  ngl::ShaderLib::setUniform("material.Ks", 1.0f, 1.0f, 1.0f);
  ngl::ShaderLib::setUniform("material.shininess", 800.0f);
  ngl::ShaderLib::setUniform("TBO", 0);
  // now for  the lights values (all set to white)
  /*struct LightInfo
  {
  // Light position in eye coords.
  vec4 position;
  // Ambient light intensity
  vec3 La;
  // Diffuse light intensity
  vec3 Ld;
  // Specular light intensity
  vec3 Ls;
  };*/
  ngl::ShaderLib::setUniform("light.position", from);
  ngl::ShaderLib::setUniform("light.La", 0.1f, 0.1f, 0.1f);
  ngl::ShaderLib::setUniform("light.Ld", 1.0f, 1.0f, 1.0f);
  ngl::ShaderLib::setUniform("light.Ls", 0.9f, 0.9f, 0.9f);

  glEnable(GL_DEPTH_TEST); // for removal of hidden surfaces

  ngl::ShaderLib::use("nglDiffuseShader");
  ngl::ShaderLib::setUniform("Colour", 1.0f, 1.0f, 1.0f, 1.0f);

  ngl::ShaderLib::setUniform("lightPos", from);
  ngl::ShaderLib::setUniform("lightDiffuse", 1.0f, 1.0f, 1.0f, 1.0f);

  // first we create a mesh from an obj passing in the obj file and texture
  m_eyeMesh.reset(new ngl::Obj("models/Eyeball.obj"));
  m_eyeMesh->createVAO();
  parseModelFile();

  m_text = std::make_unique<ngl::Text>("fonts/Arial.ttf", 16);
  createMorphMesh();
  glViewport(0, 0, 1024, 720);
  m_text->setScreenSize(width(), height());
}

// a simple structure to hold our vertex data
struct vertData
{
  ngl::Vec3 p1;
  ngl::Vec3 n1;
};
void NGLScene::createMorphMesh()
{
  // texture buffers have to be vec4 unless using GL 4.x so mac is out for now
  // just use Vec4 and waste data see http://www.opengl.org/wiki/Buffer_Texture
  std::vector<ngl::Vec4> targets;
  // base pose is mesh 1 stored in m_meshes[0]
  // get the obj data so we can process it locally
  std::vector<ngl::Vec3> baseVert = m_baseMesh->getVertexList();
  std::vector<ngl::Vec3> baseNormal = m_baseMesh->getNormalList();

  // should really check to see if the poses match if we were doing this properly
  auto numMeshes = m_meshes.size();
  std::cout << "num meshes " << numMeshes;
  std::vector<std::vector<ngl::Vec3>> verts;
  std::vector<std::vector<ngl::Vec3>> normals;
  // faces will be the same for each mesh so only need one
  // really we should check that this conforms, however it will
  // just break if it's not right so should know!
  std::vector<ngl::Face> faces = m_baseMesh->getFaceList();

  for (size_t i = 0; i < numMeshes; ++i)
  {
    verts.push_back(m_meshes[i]->getVertexList());
    normals.push_back(m_meshes[i]->getNormalList());
  }

  // now we are going to process and pack the mesh into an ngl::VertexArrayObject
  std::vector<vertData> vboMesh;
  vertData d;
  auto nFaces = faces.size();
  // loop for each of the faces
  ngl::Vec3 current;

  for (size_t i = 0; i < nFaces; ++i)
  {
    // now for each triangle in the face (remember we ensured tri above)
    for (size_t j = 0; j < 3; ++j)
    {
      // pack in the vertex data first

      d.p1 = baseVert[faces[i].m_vert[j]];
      // the blend meshes are just the differences so we subtract the base mesh
      // from the current one (could do this on GPU but this saves processing time)
      for (size_t vi = 0; vi < numMeshes; ++vi)
      {
        current = verts[vi][faces[i].m_vert[j]] - d.p1;
        targets.push_back(ngl::Vec4(current.m_x, current.m_y, current.m_z, 1.0));
      }

      // now do the normals
      d.n1 = baseNormal[faces[i].m_norm[j]];
      for (size_t ni = 0; ni < numMeshes; ++ni)
      {
        current = normals[ni][faces[i].m_norm[j]] - d.n1;
        targets.push_back(ngl::Vec4(current.m_x, current.m_y, current.m_z, 1.0));
      }

      // finally add it to our mesh VAO structure
      vboMesh.push_back(d);
    }
  }

  // generate and bind our matrix buffer this is going to be fed to the feedback shader to
  // generate our model position data for later, if we update how many instances we use
  // this will need to be re-generated (done in the draw routine)
  GLuint morphTarget;
  glGenBuffers(1, &morphTarget);

  glBindBuffer(GL_TEXTURE_BUFFER, morphTarget);
  glBufferData(GL_TEXTURE_BUFFER, targets.size() * sizeof(ngl::Vec4), NULL, GL_STATIC_DRAW);
  glBufferSubData(GL_TEXTURE_BUFFER, 0, targets.size() * sizeof(ngl::Vec4), &targets[0].m_x); // Fill

  glGenTextures(1, &m_tboID);
  glActiveTexture(GL_TEXTURE0);
  glBindTexture(GL_TEXTURE_BUFFER, m_tboID);

  glTexBuffer(GL_TEXTURE_BUFFER, GL_RGBA32F, morphTarget);

  // first we grab an instance of our VOA class as a TRIANGLE_STRIP
  m_vaoMesh = ngl::VAOFactory::createVAO("simpleVAO", GL_TRIANGLES);
  // next we bind it so it's active for setting data
  m_vaoMesh->bind();
  auto meshSize = vboMesh.size();
  // now we have our data add it to the VAO, we need to tell the VAO the following
  // how much (in bytes) data we are copying
  // a pointer to the first element of data (in this case the address of the first element of the
  // std::vector
  m_vaoMesh->setData(ngl::AbstractVAO::VertexData(meshSize * sizeof(vertData), vboMesh[0].p1.m_x));

  // so data is Vert / Normal for each mesh
  m_vaoMesh->setVertexAttributePointer(0, 3, GL_FLOAT, sizeof(vertData), 0);
  m_vaoMesh->setVertexAttributePointer(1, 3, GL_FLOAT, sizeof(vertData), 3);

  // now we have set the vertex attributes we tell the VAO class how many indices to draw when
  // glDrawArrays is called, in this case we use buffSize (but if we wished less of the sphere to be drawn we could
  // specify less (in steps of 3))
  m_vaoMesh->setNumIndices(meshSize);
  // finally we have finished for now so time to unbind the VAO
  m_vaoMesh->unbind();

  // now set all the weights
  for (unsigned int i = 0; i < m_meshes.size(); ++i)
  {
    m_weights.push_back(0.0f);
  }
}

void NGLScene::resetWeights()
{
  for (unsigned int i = 0; i < m_weights.size(); ++i)
    m_weights[i] = 0.0;
}

void NGLScene::changeActiveWeight(Direction _d)
{
  switch (_d)
  {
  case UP:
    if (++m_activeWeight >= static_cast<int>(m_meshes.size()))
      m_activeWeight = m_meshes.size() - 1;
    break;
  case DOWN:
    if (--m_activeWeight <= 0)
      m_activeWeight = 0;
    break;
  }
  std::cout << m_activeWeight << "\n";
}

void NGLScene::changeWeight(Direction _d)
{

  if (_d == UP)
    m_weights[m_activeWeight] += 0.05;
  else
    m_weights[m_activeWeight] -= 0.05;
  // clamp to 0.0 -> 1.0 range
  m_weights[m_activeWeight] = std::min(1.0f, std::max(0.0f, m_weights[m_activeWeight]));
}

void NGLScene::parseModelFile()
{
  // open the file to parse
  std::fstream fileIn;
  fileIn.open("models.txt");
  if (!fileIn.is_open())
  {
    std::cout << "File : models.txt Not found Exiting " << std::endl;
    exit(EXIT_FAILURE);
  }

  // this is the line we wish to parse
  std::string lineBuffer;
  // say which separators should be used in this
  // case Spaces, Tabs and return \ new line
  // I'm also doing , seperated list this time
  namespace ps = pystring;
  // loop through the file
  while (!fileIn.eof())
  {
    // grab a line from the input
    getline(fileIn, lineBuffer, '\n');
    // make sure it's not an empty line
    if (lineBuffer.size() != 0)
    {
      // now tokenize the line using pystring (included in ngl)
      std::vector<std::string> tokens;
      // strip leading spaces
      lineBuffer = ps::lstrip(lineBuffer);
      // now split on ,
      ps::split(lineBuffer, tokens, ",");
      // now see if it's a valid one and call the correct function
      if (tokens[0] == "BaseMesh")
      {
        std::cout << "found base mesh loading " << tokens[1] << '\n';
        m_baseMesh.reset(new ngl::Obj(tokens[1]));
      }
      else if (tokens[0] == "BlendShape")
      {
        std::cout << "Found " << tokens[1] << '\n';
        m_meshNames.push_back(tokens[1]);
        std::unique_ptr<ngl::Obj> mesh(new ngl::Obj(tokens[2]));
        m_meshes.push_back(std::move(mesh));
      }
    } // end zero loop
  }
}

void NGLScene::loadMatricesToShader()
{
  ngl::ShaderLib::use("PerFragADS");
  ngl::Mat4 MV;
  ngl::Mat4 MVP;
  ngl::Mat3 normalMatrix;
  MV = m_view * m_mouseGlobalTX;
  MVP = m_project * MV;
  normalMatrix = MV;
  normalMatrix.inverse().transpose();
  ngl::ShaderLib::setUniform("MVP", MVP);
  ngl::ShaderLib::setUniform("MV", MV);
  ngl::ShaderLib::setUniform("normalMatrix", normalMatrix);
  for (unsigned int i = 0; i < m_weights.size(); ++i)
  {
    ngl::ShaderLib::setUniform(fmt::format("weights[{}]", i), m_weights[i]);
  }
}

void NGLScene::paintGL()
{
  // clear the screen and depth buffer
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
  // Rotation based on the mouse position for our global transform
  ngl::Transformation trans;
  auto rotX = ngl::Mat4::rotateX(m_win.spinXFace);
  auto rotY = ngl::Mat4::rotateY(m_win.spinYFace);
  // multiply the rotations
  m_mouseGlobalTX = rotY * rotX;
  // add the translations
  m_mouseGlobalTX.m_m[3][0] = m_modelPos.m_x;
  m_mouseGlobalTX.m_m[3][1] = m_modelPos.m_y;
  m_mouseGlobalTX.m_m[3][2] = m_modelPos.m_z;

  loadMatricesToShader();
  // draw the mesh
  m_vaoMesh->bind();
  glBindTexture(GL_TEXTURE_BUFFER, m_tboID);
  m_vaoMesh->draw();
  m_vaoMesh->unbind();

  ngl::ShaderLib::use("nglDiffuseShader");
  // left Eye

  ngl::Mat4 MV;
  ngl::Mat4 MVP;
  ngl::Mat4 M;
  ngl::Mat3 normalMatrix;
  ngl::Transformation t;
  t.setScale(0.685f, 0.583f, 0.583f);
  t.setPosition(-1.276f, 3.209f, 2.271f);
  M = m_mouseGlobalTX * t.getMatrix();
  MV = m_view * M;
  MVP = m_project * MV;
  normalMatrix = MV;
  normalMatrix.inverse().transpose();

  ngl::ShaderLib::setUniform("MVP", MVP);
  ngl::ShaderLib::setUniform("normalMatrix", normalMatrix);
  m_eyeMesh->draw();

  t.setPosition(1.276f, 3.209f, 2.271f);
  M = m_mouseGlobalTX * t.getMatrix();
  MV = m_view * M;
  MVP = m_project * MV;
  normalMatrix = MV;
  normalMatrix.inverse();
  ngl::ShaderLib::setUniform("MVP", MVP);
  ngl::ShaderLib::setUniform("normalMatrix", normalMatrix);
  m_eyeMesh->draw();
  m_text->setColour(1.0f, 1.0f, 1.0f);
  m_text->renderText(10, 700, fmt::format("Current Mesh {} value {}", m_meshNames[m_activeWeight], m_weights[m_activeWeight]));
  m_text->renderText(10, 680, "Q-W change Pose Arrows to swap weights");
}

//----------------------------------------------------------------------------------------------------------------------
void NGLScene::mouseMoveEvent(QMouseEvent *_event)
{
// note the method buttons() is the button state when event was called
// that is different from button() which is used to check which button was
// pressed when the mousePress/Release event is generated
#if QT_VERSION > QT_VERSION_CHECK(6, 0, 0)
  auto position = _event->position();
#else
  auto position = _event->pos();
#endif
  if (m_win.rotate && _event->buttons() == Qt::LeftButton)
  {
    int diffx = position.x() - m_win.origX;
    int diffy = position.y() - m_win.origY;
    m_win.spinXFace += static_cast<int>(0.5f * diffy);
    m_win.spinYFace += static_cast<int>(0.5f * diffx);
    m_win.origX = position.x();
    m_win.origY = position.y();
    update();
  }
  // right mouse translate code
  else if (m_win.translate && _event->buttons() == Qt::RightButton)
  {
    int diffX = static_cast<int>(position.x() - m_win.origXPos);
    int diffY = static_cast<int>(position.y() - m_win.origYPos);
    m_win.origXPos = position.x();
    m_win.origYPos = position.y();
    m_modelPos.m_x += INCREMENT * diffX;
    m_modelPos.m_y -= INCREMENT * diffY;
    update();
  }
}

//----------------------------------------------------------------------------------------------------------------------
void NGLScene::mousePressEvent(QMouseEvent *_event)
{
// that method is called when the mouse button is pressed in this case we
// store the value where the maouse was clicked (x,y) and set the Rotate flag to true
#if QT_VERSION > QT_VERSION_CHECK(6, 0, 0)
  auto position = _event->position();
#else
  auto position = _event->pos();
#endif
  if (_event->button() == Qt::LeftButton)
  {
    m_win.origX = position.x();
    m_win.origY = position.y();
    m_win.rotate = true;
  }
  // right mouse translate mode
  else if (_event->button() == Qt::RightButton)
  {
    m_win.origXPos = position.x();
    m_win.origYPos = position.y();
    m_win.translate = true;
  }
}

//----------------------------------------------------------------------------------------------------------------------
void NGLScene::mouseReleaseEvent(QMouseEvent *_event)
{
  // that event is called when the mouse button is released
  // we then set Rotate to false
  if (_event->button() == Qt::LeftButton)
  {
    m_win.rotate = false;
  }
  // right mouse translate mode
  if (_event->button() == Qt::RightButton)
  {
    m_win.translate = false;
  }
}

//----------------------------------------------------------------------------------------------------------------------
void NGLScene::wheelEvent(QWheelEvent *_event)
{

  // check the diff of the wheel position (0 means no change)
  if (_event->angleDelta().x() > 0)
  {
    m_modelPos.m_z += ZOOM;
  }
  else if (_event->angleDelta().x() < 0)
  {
    m_modelPos.m_z -= ZOOM;
  }
  update();
}
//----------------------------------------------------------------------------------------------------------------------

void NGLScene::keyPressEvent(QKeyEvent *_event)
{
  // this method is called every time the main window recives a key event.
  // we then switch on the key value and set the camera in the GLWindow
  switch (_event->key())
  {
  // escape key to quite
  case Qt::Key_Escape:
    QGuiApplication::exit(EXIT_SUCCESS);
    break;
  case Qt::Key_1:
    glPolygonMode(GL_FRONT_AND_BACK, GL_LINE);
    break;
  case Qt::Key_2:
    glPolygonMode(GL_FRONT_AND_BACK, GL_FILL);
    break;
  case Qt::Key_F:
    showFullScreen();
    break;
  case Qt::Key_N:
    showNormal();
    break;
  case Qt::Key_Q:
    changeWeight(DOWN);
    break;
  case Qt::Key_W:
    changeWeight(UP);
    break;

  case Qt::Key_Left:
    changeActiveWeight(DOWN);
    break;
  case Qt::Key_Right:
    changeActiveWeight(UP);
    break;

  case Qt::Key_Space:
    resetWeights();
    break;
  default:
    break;
  }
  // finally update the GLWindow and re-draw
  // if (isExposed())
  update();
}


```
