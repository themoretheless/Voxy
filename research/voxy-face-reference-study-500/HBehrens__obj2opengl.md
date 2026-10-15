# Facial rendering source scan: HBehrens/obj2opengl

Repository: https://github.com/HBehrens/obj2opengl

Pinned source revision: aede13e6b2e020651d251f1f6dc1f841b491a861

Discovery topic: face model 3D

Description: script to convert 3D models of OBJ files to C/C++ float arrays (vertices, faces, texture) compatible with OpenGL ES glDrawArrays compatible with iPhone/iPad

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {}

## Source file: cube.h

{
  "path": "cube.h",
  "blob_sha": "b9e8dc8352f93ee915890decbac401b47e308a03",
  "sha256": "6e28af2497af1007b3259e92dfed89943feb5a3bde0c37f187db77f0d19961ed",
  "lines": 125,
  "bytes": 2155,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/HBehrens/obj2opengl/blob/aede13e6b2e020651d251f1f6dc1f841b491a861/cube.h",
  "function_signature_candidates": []
}

```text

/*
created with obj2opengl.pl

source file    : ./cube.obj
vertices       : 8
faces          : 12
normals        : 6
texture coords : 0


// include generated arrays
#import "./cube.h"

// set input data to arrays
glVertexPointer(3, GL_FLOAT, 0, cubeVerts);
glNormalPointer(GL_FLOAT, 0, cubeNormals);

// draw data
glDrawArrays(GL_TRIANGLES, 0, cubeNumVerts);
*/

unsigned int cubeNumVerts = 36;

float cubeVerts [] = {
  // f  1//2  7//2  5//2
  -0.5, -0.5, -0.5,
  0.5, 0.5, -0.5,
  0.5, -0.5, -0.5,
  // f  1//2  3//2  7//2 
  -0.5, -0.5, -0.5,
  -0.5, 0.5, -0.5,
  0.5, 0.5, -0.5,
  // f  1//6  4//6  3//6 
  -0.5, -0.5, -0.5,
  -0.5, 0.5, 0.5,
  -0.5, 0.5, -0.5,
  // f  1//6  2//6  4//6 
  -0.5, -0.5, -0.5,
  -0.5, -0.5, 0.5,
  -0.5, 0.5, 0.5,
  // f  3//3  8//3  7//3 
  -0.5, 0.5, -0.5,
  0.5, 0.5, 0.5,
  0.5, 0.5, -0.5,
  // f  3//3  4//3  8//3 
  -0.5, 0.5, -0.5,
  -0.5, 0.5, 0.5,
  0.5, 0.5, 0.5,
  // f  5//5  7//5  8//5 
  0.5, -0.5, -0.5,
  0.5, 0.5, -0.5,
  0.5, 0.5, 0.5,
  // f  5//5  8//5  6//5 
  0.5, -0.5, -0.5,
  0.5, 0.5, 0.5,
  0.5, -0.5, 0.5,
  // f  1//4  5//4  6//4 
  -0.5, -0.5, -0.5,
  0.5, -0.5, -0.5,
  0.5, -0.5, 0.5,
  // f  1//4  6//4  2//4 
  -0.5, -0.5, -0.5,
  0.5, -0.5, 0.5,
  -0.5, -0.5, 0.5,
  // f  2//1  6//1  8//1 
  -0.5, -0.5, 0.5,
  0.5, -0.5, 0.5,
  0.5, 0.5, 0.5,
  // f  2//1  8//1  4//1 
  -0.5, -0.5, 0.5,
  0.5, 0.5, 0.5,
  -0.5, 0.5, 0.5,
};

float cubeNormals [] = {
  // f  1//2  7//2  5//2
  0, 0, -1,
  0, 0, -1,
  0, 0, -1,
  // f  1//2  3//2  7//2 
  0, 0, -1,
  0, 0, -1,
  0, 0, -1,
  // f  1//6  4//6  3//6 
  -1, 0, 0,
  -1, 0, 0,
  -1, 0, 0,
  // f  1//6  2//6  4//6 
  -1, 0, 0,
  -1, 0, 0,
  -1, 0, 0,
  // f  3//3  8//3  7//3 
  0, 1, 0,
  0, 1, 0,
  0, 1, 0,
  // f  3//3  4//3  8//3 
  0, 1, 0,
  0, 1, 0,
  0, 1, 0,
  // f  5//5  7//5  8//5 
  1, 0, 0,
  1, 0, 0,
  1, 0, 0,
  // f  5//5  8//5  6//5 
  1, 0, 0,
  1, 0, 0,
  1, 0, 0,
  // f  1//4  5//4  6//4 
  0, -1, 0,
  0, -1, 0,
  0, -1, 0,
  // f  1//4  6//4  2//4 
  0, -1, 0,
  0, -1, 0,
  0, -1, 0,
  // f  2//1  6//1  8//1 
  0, 0, 1,
  0, 0, 1,
  0, 0, 1,
  // f  2//1  8//1  4//1 
  0, 0, 1,
  0, 0, 1,
  0, 0, 1,
};



```
