# Facial rendering source scan: cedricfoucault/facial_animation

Repository: https://github.com/cedricfoucault/facial_animation

Pinned source revision: f83f3512d6bd16f6a879394e5a3534f62d3ac71e

Discovery topic: facial animation

Description: OpenGL program that can render and smoothly animate faces representing different human emotions on a human head mesh.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: src/glutil.h

{
  "path": "src/glutil.h",
  "blob_sha": "eb53456ec001b18650ec5620d4f930ca1d95370e",
  "sha256": "d7b8bfe51e82e4b3a70279be2a9d601917f288f6722d4809ebf8ef689221659d",
  "lines": 53,
  "bytes": 1443,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/cedricfoucault/facial_animation/blob/f83f3512d6bd16f6a879394e5a3534f62d3ac71e/src/glutil.h",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "extern void rot_axis_f ();"
    },
    {
      "line": 20,
      "text": "extern void rot_about_axis_f ();"
    },
    {
      "line": 21,
      "text": "extern void rot_omega_f ();"
    },
    {
      "line": 22,
      "text": "extern void urot_axis_f ();"
    },
    {
      "line": 23,
      "text": "extern void urot_about_axis_f ();"
    },
    {
      "line": 24,
      "text": "extern void urot_omega_f ();"
    },
    {
      "line": 27,
      "text": "extern void uview_direction_d ();"
    },
    {
      "line": 28,
      "text": "extern void uview_direction_f ();"
    },
    {
      "line": 29,
      "text": "extern void uviewpoint_d ();"
    },
    {
      "line": 30,
      "text": "extern void uviewpoint_f ();"
    },
    {
      "line": 33,
      "text": "extern void rot_axis_f (float omega, float axis[3]);"
    },
    {
      "line": 34,
      "text": "extern void rot_about_axis_f (float angle, float axis[3]);"
    },
    {
      "line": 35,
      "text": "extern void rot_omega_f (float axis[3]);"
    },
    {
      "line": 36,
      "text": "extern void urot_axis_f (float m[16], float omega, float axis[3]);"
    },
    {
      "line": 37,
      "text": "extern void urot_about_axis_f (float m[16], float angle, float axis[3]);"
    },
    {
      "line": 38,
      "text": "extern void urot_omega_f (float m[16], float axis[3]);"
    },
    {
      "line": 41,
      "text": "extern void uview_direction_f (float m[16],\t\t/* returned */"
    },
    {
      "line": 46,
      "text": "extern void uviewpoint_f (float m[16],\t\t/* returned */"
    }
  ]
}

```text


/*
 * gutil.h
 *
 * FUNCTION:
 * Provide utilities that allow rotation to occur 
 * around any axis.
 * 
 * HISTORY:
 * created by Linas Vepstas  1990
 * added single & double precision, June 1991, Linas Vepstas
 */

#ifndef __GUTIL_H__
#define __GUTIL_H__


/* Rotation Utilities */
extern void rot_axis_f ();
extern void rot_about_axis_f ();
extern void rot_omega_f ();
extern void urot_axis_f ();
extern void urot_about_axis_f ();
extern void urot_omega_f ();

/* viewpoint functions */
extern void uview_direction_d ();
extern void uview_direction_f ();
extern void uviewpoint_d ();
extern void uviewpoint_f ();

/* Rotation Utilities */
extern void rot_axis_f (float omega, float axis[3]);
extern void rot_about_axis_f (float angle, float axis[3]);
extern void rot_omega_f (float axis[3]);
extern void urot_axis_f (float m[16], float omega, float axis[3]);
extern void urot_about_axis_f (float m[16], float angle, float axis[3]);
extern void urot_omega_f (float m[16], float axis[3]);

/* viewpoint functions */
extern void uview_direction_f (float m[16],		/* returned */
                        float v21[3],		/* input */
                        float up[3]);		/* input */


extern void uviewpoint_f (float m[16],		/* returned */
                   float v1[3],		/* input */
                   float v2[3],		/* input */
                   float up[3]);		/* input */


/* ------------------- end of file ---------------------- */
#endif


```

## Source file: src/texpng.h

{
  "path": "src/texpng.h",
  "blob_sha": "78ecfdb07c8461a313258a67638b76e93a6d1e35",
  "sha256": "13ab8b15753b325a0cbd3251d32be8bde824cf3caefc461128bed0768246e750",
  "lines": 18,
  "bytes": 581,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/cedricfoucault/facial_animation/blob/f83f3512d6bd16f6a879394e5a3534f62d3ac71e/src/texpng.h",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "void loadPngTex(char * fname , struct tImagePNG * pImagePNG , int *hasAlpha );"
    },
    {
      "line": 14,
      "text": "void ReverseImage( int width , int height , unsigned char *data , int bytesPerPixel );"
    },
    {
      "line": 15,
      "text": "void CheckImageDimensions( int sizeX , int sizeY , char * strFileName);"
    },
    {
      "line": 16,
      "text": "void CreateTexturePNG(unsigned int *textureId, char * strFileName);"
    },
    {
      "line": 17,
      "text": "int writepng(char *filename, char *description,"
    }
  ]
}

```text

#define PNG_BYTES_TO_CHECK 4
// This stores png data
struct tImagePNG
{
	int bytesPerPixel;
	int sizeX;
	int sizeY;
	unsigned char *data;
};

typedef struct tImagePNG   *PtImagePNG;

void loadPngTex(char * fname , struct tImagePNG * pImagePNG , int *hasAlpha );
void ReverseImage( int width , int height , unsigned char *data , int bytesPerPixel );
void CheckImageDimensions( int sizeX , int sizeY , char * strFileName);
void CreateTexturePNG(unsigned int *textureId, char * strFileName);
int writepng(char *filename, char *description,
	     int x, int y, int width, int height);


```

## Source file: src/texpng.cpp

{
  "path": "src/texpng.cpp",
  "blob_sha": "580d93274a095de1551e9e983ce506e13233c107",
  "sha256": "df2c8820f608df24f1f7ecc1dae6bfdd5ceae3304856b2f4332aaa0eda0791f1",
  "lines": 379,
  "bytes": 12593,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/cedricfoucault/facial_animation/blob/f83f3512d6bd16f6a879394e5a3534f62d3ac71e/src/texpng.cpp",
  "function_signature_candidates": [
    {
      "line": 49,
      "text": "void loadPngTex(char * fname , struct tImagePNG * pImagePNG , int *hasAlpha )"
    },
    {
      "line": 181,
      "text": "void ReverseImage( int width , int height , unsigned char *data , int bytesPerPixel ) {"
    },
    {
      "line": 196,
      "text": "void CheckImageDimensions( int sizeX , int sizeY , char * strFileName) {"
    },
    {
      "line": 231,
      "text": "void CreateTexturePNG(unsigned int *textureId, char * strFileName)"
    },
    {
      "line": 286,
      "text": "int writepng(char *filename, char *description,"
    }
  ]
}

```text

/*****************************************************************************
File: texpng.cpp

Virtual Humans
Master in Computer Science
Christian Jacquemin, University Paris 11

This file is provided without support, instruction, or implied
warranty of any kind.  University Paris 11 makes no guarantee of its
fitness for a particular purpose and is not liable under any
circumstances for any damages or loss whatsoever arising from the use
or inability to use this file or items derived from it.
******************************************************************************/
#include <GL/gl.h>
#include <GL/glu.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <png.h>
#include "texpng.h"
#include "ctype.h"

#define png_infopp_NULL (png_infopp)NULL
#define int_p_NULL (int*)NULL

/* Pixels in this bitmap structure are stored as BGR. */
typedef struct _RGBPixel {
    unsigned char blue;
    unsigned char green;
    unsigned char red;
} RGBPixel;

/* Structure for containing decompressed bitmaps. */
typedef struct _RGBBitmap {
    RGBPixel *pixels;
    size_t width;
    size_t height;
    size_t bytewidth;
    unsigned char bytes_per_pixel;
} RGBBitmap;

///////////////////////////////// LOAD PNG \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\*
/////
/////	This loads the PNG file and returns its data in a tImagePNG struct
/////
///////////////////////////////// LOAD PNG \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\*

// This is the interesting function here. It sets up a PNG image as a texture.
void loadPngTex(char * fname , struct tImagePNG * pImagePNG , int *hasAlpha )
{
  FILE      *fp;
  
  // The header of the file will be saved in here
  char buf[PNG_BYTES_TO_CHECK];
  
  // Open the file and check correct opening
  /* Open the prospective PNG file. */
  if ((fp = fopen(fname, "rb")) == NULL) {
    printf( "Error: Could not open the texture file [%s]!" , fname );
    exit( 0 );
  }
  
  // Read the PNG header, which is 8 bytes long.
  /* Read in some of the signature bytes */
  if (fread(buf, 1, PNG_BYTES_TO_CHECK, fp) != PNG_BYTES_TO_CHECK) {
    printf( "Error: Incorrect PNG texture file [%s]!" , fname ); 
    exit( 0 );
  }

  // Check whether the file is a PNG file
  // png_sig_cmp() checks the given PNG header and returns 0 if it could
  // be the start of a PNG file. We can use 8 bytes at max for
  // this comparison.
  if(png_sig_cmp((png_byte*)buf, (png_size_t)0, PNG_BYTES_TO_CHECK) != 0) {
    fclose( fp );
    printf( "Error: Not a PNG file [%s]!" , fname );
    exit( 0 );
  }
  
  // Create / initialize the png_struct
  // The png_struct isn't directly accessed by the user (us).
  // We will later create a png_info from this to get access to the
  // PNG's infos.
  // The three 0's in the arg list could be pointers to user defined error
  // handling functions. 0 means we don't want to specify them, but use
  // libPNG's default error handling instead.
  png_infop info_ptr;
  png_structp png_ptr
    = png_create_read_struct(PNG_LIBPNG_VER_STRING,
			     NULL , NULL , NULL );
  if(!png_ptr) {
    fclose( fp );
    printf( "Error: Couldn't create PNG read structure [%s]!" , fname );
    exit( 0 );
  }

  // Create / initialize the png_info
  // The png_info grants the user access to the PNG infos.
  info_ptr = png_create_info_struct(png_ptr);
  if(!info_ptr) {
    // We need to destroy the read_struct
    png_destroy_read_struct(&png_ptr, NULL, NULL);
    fclose( fp );
    printf( "Error: Couldn't create PNG info structure [%s]!" , fname );
    exit( 0 );
  }
  
  // Setup error handler
  // This sets the point libPNG jumps back to is an error occurs.
  if (setjmp(png_jmpbuf(png_ptr))) {
    /* Free all of the memory associated with the png_ptr and info_ptr */
    png_destroy_read_struct(&png_ptr, &info_ptr, png_infopp_NULL);
    fclose(fp);
    /* If we get here, we had a problem reading the file */
    printf( "Error: Couldn't setup PNG error handler [%s]!" , fname );
    exit( 0 );
  }
  
  /* Set up the input control if you are using standard C streams */
  png_init_io(png_ptr, fp);
  
  // This tells libPNG that we have already read 8 bytes from the start
  // of the file (for the header check above).
  png_set_sig_bytes(png_ptr, PNG_BYTES_TO_CHECK);

  // This reads the PNG file into the read and info structs
  png_read_png(png_ptr, info_ptr, PNG_TRANSFORM_IDENTITY, NULL);

  // Get some basic infos about the image
  pImagePNG->sizeX = png_get_image_width(png_ptr, info_ptr); // width in px
  pImagePNG->sizeY = png_get_image_height(png_ptr, info_ptr); // height in px
  int bit_depth = png_get_bit_depth(png_ptr, info_ptr); // bits per pixel

  // Color type - we'll handle RGB and RGBA (with Alpha)
  int cType = png_get_color_type(png_ptr, info_ptr);

  // We now calculate the *bytes* per pixel from the color type and the
  // bits per pixel.
  if((cType == PNG_COLOR_TYPE_RGB) && (bit_depth == 8)) {
    pImagePNG->bytesPerPixel = 3;
     *hasAlpha = 0;
  }
  else if((cType == PNG_COLOR_TYPE_RGB_ALPHA) && (bit_depth == 8)) {
    pImagePNG->bytesPerPixel = 4;
    *hasAlpha = 1;
  }
  else {
    /* clean up after the read, and free any memory allocated - REQUIRED */
    png_destroy_read_struct(&png_ptr, &info_ptr, png_infopp_NULL);
    fclose( fp );
    printf( "Error: PNG image [%s] type is unsupported!" , fname );
    exit( 0 );
  }
  
  // Returns a pointer to the array of array of png_bytes that holds the
  // image data.
  png_byte **imageData = png_get_rows(png_ptr, info_ptr);

  // This will hold our image 
  unsigned int rowbytes = png_get_rowbytes(png_ptr, info_ptr);
  pImagePNG->data = (GLubyte*)malloc(pImagePNG->sizeY * rowbytes * sizeof(GLubyte));
  
  int i,j;
  for( i = 0 ; i < pImagePNG->sizeY ; i++ ) {
    memcpy(&pImagePNG->data[(pImagePNG->sizeY - i - 1) * rowbytes],
	   imageData[i], rowbytes);
  }

  // Free the memory we used - we don't need it anymore
  /* clean up after the read, and free any memory allocated - REQUIRED */
  png_destroy_read_struct(&png_ptr, &info_ptr, png_infopp_NULL);
  fclose( fp );
}

///////////////////////////// CREATE TEXTURE \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\*
/////
/////	This creates a texture in OpenGL that we can use as a texture map
/////
///////////////////////////// CREATE TEXTURE \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\*

void ReverseImage( int width , int height , unsigned char *data , int bytesPerPixel ) {
  unsigned char *dataaux = (unsigned char*)malloc(width * height * bytesPerPixel * sizeof(unsigned char));
  int indrow;
  for( indrow = 0 ; indrow < height ; indrow++ ) {
    memcpy(&dataaux[indrow * width * bytesPerPixel],
	   &data[(height - indrow - 1) * width * bytesPerPixel], width * bytesPerPixel);
  }
  for( indrow = 0 ; indrow < height ; indrow++ ) {
    memcpy(&data[indrow * width * bytesPerPixel],
	   &dataaux[indrow * width * bytesPerPixel], width * bytesPerPixel);
  }
  free(dataaux);
}

// The image dimensions must be powers of 2.
void CheckImageDimensions( int sizeX , int sizeY , char * strFileName) {  
  // Check the image width
  int comp = 2;
  int isValid = 0;
  int i;
  for( i = 0; i < 10; i++) {
    comp <<= 1;
    if(sizeX == comp)
      isValid = 1;
  }
  if(!isValid) {
    printf( "Error: The image width is not a power of 2 [%s]!" , strFileName ); 
    exit( 0 );
  }  
  
  // Check the image height
  comp = 2;
  isValid = 0;
  for( i = 0; i < 10; i++) {
    comp <<= 1;
    if(sizeY == comp)
      isValid = 1;
  }
  if(!isValid) {
    printf( "Error: The image height is not a power of 2 [%s]!" , strFileName );
    exit( 0 );
  }  
}

///////////////////////////////// CREATE TEXTURE \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\*
/////
/////	This creates a texture in OpenGL that we can use as a texture map
/////
///////////////////////////////// CREATE TEXTURE \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\*

void CreateTexturePNG(unsigned int *textureId, char * strFileName)
{
  if(!strFileName)			
    // Return from the function if no file name was passed in
    return;
  
  
  printf( "Loading [%s]!\n" , strFileName ); 

  // Load the image and store the data
  struct tImagePNG imagePNG;
  int hasAlpha = 0;
  loadPngTex(strFileName , &imagePNG , &hasAlpha );  
	
  // Generate a texture with the associative texture ID stored in the array
  glGenTextures(1, textureId);

  // Bind the texture to the texture arrays index and init the texture
  glBindTexture(GL_TEXTURE_2D, *textureId);

  // Build Mipmaps (builds different versions of the picture for distances - looks better)
  if( hasAlpha ) {
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER,  
		    GL_LINEAR_MIPMAP_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, 
		    GL_LINEAR);
    gluBuild2DMipmaps(GL_TEXTURE_2D, GL_RGBA8,
		      imagePNG.sizeX, imagePNG.sizeY,  
		      GL_RGBA, 
		      GL_UNSIGNED_BYTE, (const void *)imagePNG.data); 
  }
  else {
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER,  
		    GL_LINEAR_MIPMAP_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, 
		    GL_LINEAR);
    gluBuild2DMipmaps(GL_TEXTURE_2D, GL_RGB8,
		      imagePNG.sizeX, imagePNG.sizeY,  
		      GL_RGB, 
		      GL_UNSIGNED_BYTE, (const void *)imagePNG.data); 
  }

  // Now we need to free the image data that we loaded since OpenGL stored it as a texture

  if (imagePNG.data) 	        // If there is texture data
    {
      free(imagePNG.data);	// Free the texture data, we don't need it anymore
    }
}

//////////////////////////////////////////////////////////////////////
// SAVE IMAGE
//////////////////////////////////////////////////////////////////////

/* Attempts to save PNG to file; returns 0 on success, non-zero on error. */
int writepng(char *filename, char *description,
	     int x, int y, int width, int height) {
  RGBPixel *image;
  int bytes_per_pixel = 3;

  FILE *fp;
  png_structp png_ptr = NULL;
  png_infop info_ptr = NULL;
  png_uint_32 bytes_per_row;
  png_byte **row_pointers = NULL;

  fp = fopen(filename, "wb");
  if (fp == NULL) {
    return 1;
  }
  image = (RGBPixel *) malloc(width * height * sizeof(struct _RGBPixel));

  /* OpenGL's default 4 byte pack alignment would leave extra bytes at the
     end of each image row so that each full row contained a number of bytes
     divisible by 4.  Ie, an RGB row with 3 pixels and 8-bit componets would
     be laid out like "RGBRGBRGBxxx" where the last three "xxx" bytes exist
     just to pad the row out to 12 bytes (12 is divisible by 4). To make sure
     the rows are packed as tight as possible (no row padding), set the pack
     alignment to 1. */
  glPixelStorei(GL_PACK_ALIGNMENT, 1);
  glReadPixels(x, y, width, height, GL_RGB, GL_UNSIGNED_BYTE, (void *)image);

  /* Initialize the write struct. */
  png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
  if (png_ptr == NULL) {
    fclose(fp);
    return -1;
  }

  /* Initialize the info struct. */
  info_ptr = png_create_info_struct(png_ptr);
  if (info_ptr == NULL) {
    png_destroy_write_struct(&png_ptr, NULL);
    fclose(fp);
    return 1;
  }

  /* Set up error handling. */
  if (setjmp(png_jmpbuf(png_ptr))) {
    png_destroy_write_struct(&png_ptr, &info_ptr);
    fclose(fp);
    return 1;
  }

  /* Set image attributes. */
  png_set_IHDR(png_ptr,
	       info_ptr,
	       width,
	       height,
	       8,
	       PNG_COLOR_TYPE_RGB,
	       PNG_INTERLACE_NONE,
	       PNG_COMPRESSION_TYPE_DEFAULT,
	       PNG_FILTER_TYPE_DEFAULT);
  
  /* Initialize rows of PNG. */
  bytes_per_row = width * bytes_per_pixel;
  row_pointers = (png_byte **)png_malloc(png_ptr, height * sizeof(png_byte *));
  unsigned char *pixels = (unsigned char *)image;
  for (int j = height - 1; j >= 0 ; --j) {
    unsigned char *row = (unsigned char *)png_malloc(png_ptr, sizeof(unsigned char) * bytes_per_row);
    row_pointers[j] = (png_byte *)row;
    memcpy(row_pointers[j], pixels, bytes_per_pixel * width);
    pixels += bytes_per_pixel * width;
    // for (int i = 0; i < width; ++i) {
    //   RGBPixel color = *(image + (j * bytes_per_row) + (i * bytes_per_pixel));
    //   *row++ = color.red;
    //   *row++ = color.green;
    //   *row++ = color.blue;
    // }
  }
  
  /* Actually write the image data. */
  png_init_io(png_ptr, fp);
  png_set_rows(png_ptr, info_ptr, row_pointers);
  png_write_png(png_ptr, info_ptr, PNG_TRANSFORM_IDENTITY, NULL);

  /* Cleanup. */
  for (int j = 0; j < height; j++) {
    png_free(png_ptr, row_pointers[j]);
  }
  png_free(png_ptr, row_pointers);

  /* Finish writing. */
  png_destroy_write_struct(&png_ptr, &info_ptr);
  free(image);
  fclose(fp);
  return 0;
}


```

## Source file: src/urotate.cpp

{
  "path": "src/urotate.cpp",
  "blob_sha": "ba4cc028e623f4b2b81ed0163094d021818ffd05",
  "sha256": "5915da9a6c758f0e94021af8359a2dcce53c8e119037f6b7d6078865e7504589",
  "lines": 186,
  "bytes": 5590,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 3,
    "rig": 1
  },
  "url": "https://github.com/cedricfoucault/facial_animation/blob/f83f3512d6bd16f6a879394e5a3534f62d3ac71e/src/urotate.cpp",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "* void urot_about_axis (float m[16],      --- returned"
    },
    {
      "line": 26,
      "text": "* void rot_about_axis (float angle,        --- input"
    },
    {
      "line": 33,
      "text": "* void urot_axis (float m[16],      --- returned"
    },
    {
      "line": 40,
      "text": "* void rot_axis (float omega,        --- input"
    },
    {
      "line": 47,
      "text": "* void urot_omega (float m[16],       --- returned"
    },
    {
      "line": 52,
      "text": "* void rot_omega (float omega[3])      --- input"
    },
    {
      "line": 69,
      "text": "void urot_axis_f (float m[16], \t\t/* returned */"
    },
    {
      "line": 142,
      "text": "void urot_about_axis_f (float m[16], \t\t/* returned */"
    },
    {
      "line": 169,
      "text": "void urot_omega_f (float m[16], \t/* returned */"
    }
  ]
}

```text

/* 
 * MODULE: urotate.c
 *
 * FUNCTION:
 * This module contains three different routines that compute rotation
 * matricies and return these to user.
 * Detailed description is provided below.
 *
 * HISTORY:
 * Developed & written, Linas Vepstas, Septmeber 1991
 * double precision port, March 1993
 *
 * DETAILED DESCRIPTION:
 * This module contains three routines:
 * --------------------------------------------------------------------
 *
 * void urot_about_axis (float m[16],      --- returned
 *                       float angle,        --- input 
 *                       float axis[3])      --- input
 * Computes a rotation matrix.
 * The rotation is around the the direction specified by the argument
 * argument axis[3].  User may specify vector which is not of unit
 * length.  The angle of rotation is specified in degrees, and is in the
 * right-handed direction.
 *
 * void rot_about_axis (float angle,        --- input 
 *                      float axis[3])      --- input
 * Same as above routine, except that the matrix is multiplied into the
 * GL matrix stack.
 *
 * --------------------------------------------------------------------
 *
 * void urot_axis (float m[16],      --- returned
 *                 float omega,        --- input
 *                 float axis[3])      --- input
 * Same as urot_about_axis(), but angle specified in radians.
 * It is assumed that the argument axis[3] is a vector of unit length.
 * If it is not of unit length, the returned matrix will not be correct.
 *
 * void rot_axis (float omega,        --- input
 *                float axis[3])      --- input
 * Same as above routine, except that the matrix is multiplied into the
 * GL matrix stack.
 *
 * --------------------------------------------------------------------
 *
 * void urot_omega (float m[16],       --- returned
 *                  float omega[3])      --- input
 * same as urot_axis(), but the angle is taken as the length of the
 * vector omega[3]
 *
 * void rot_omega (float omega[3])      --- input
 * Same as above routine, except that the matrix is multiplied into the
 * GL matrix stack.
 *
 * --------------------------------------------------------------------
 */

#include <math.h>
#include "glutil.h"

/* Some <math.h> files do not define M_PI... */
#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif
   
/* ========================================================== */

void urot_axis_f (float m[16], 		/* returned */
                  float omega, 		/* input */
                  float axis[3])		/* input */
{
   double s, c, ssq, csq, cts;
   double tmp;

   /*
    * The formula coded up below can be derived by using the 
    * homomorphism between SU(2) and O(3), namely, that the
    * 3x3 rotation matrix R is given by
    *      t.R.v = S(-1) t.v S
    * where
    * t are the Pauli matrices (similar to Quaternions, easier to use)
    * v is an arbitrary 3-vector
    * and S is a 2x2 hermitian matrix:
    *     S = exp ( i omega t.axis / 2 )
    * 
    * (Also, remember that computer graphics uses the transpose of R).
    * 
    * The Pauli matrices are:
    * 
    *  tx = (0 1)          ty = (0 -i)            tz = (1  0)
    *       (1 0)               (i  0)                 (0 -1)
    *
    * Note that no error checking is done -- if the axis vector is not 
    * of unit length, you'll get strange results.
    */

   tmp = (double) omega / 2.0;
   s = sin (tmp);
   c = cos (tmp);

   ssq = s*s;
   csq = c*c;

   m[0*4+0] = m[1*4+1] = m[2*4+2] = csq - ssq;

   ssq *= 2.0;

   /* on-diagonal entries */
   m[0*4+0] += ssq * axis[0]*axis[0];
   m[1*4+1] += ssq * axis[1]*axis[1];
   m[2*4+2] += ssq * axis[2]*axis[2];

   /* off-diagonal entries */
   m[0*4+1] = m[1*4+0] = axis[0] * axis[1] * ssq;
   m[1*4+2] = m[2*4+1] = axis[1] * axis[2] * ssq;
   m[2*4+0] = m[0*4+2] = axis[2] * axis[0] * ssq;

   cts = 2.0 * c * s;

   tmp = cts * axis[2];
   m[0*4+1] += tmp;
   m[1*4+0] -= tmp;

   tmp = cts * axis[0];
   m[1*4+2] += tmp;
   m[2*4+1] -= tmp;

   tmp = cts * axis[1];
   m[2*4+0] += tmp;
   m[0*4+2] -= tmp;

   /* homogeneous entries */
   m[0*4+3] = m[1*4+3] = m[2*4+3] = m[3*4+2] = m[3*4+1] = m[3*4+0] = 0.0;
   m[3*4+3] = 1.0;


}

/* ========================================================== */

void urot_about_axis_f (float m[16], 		/* returned */
                        float angle, 		/* input */
                        float axis[3])		/* input */
{
   float len, ax[3];

   angle *= M_PI/180.0;		/* convert to radians */

   /* renormalize axis vector, if needed */
   len = axis[0]*axis[0] + axis[1]*axis[1] + axis[2]*axis[2];

   /* we can save some machine instructions by normalizing only
    * if needed.  The compiler should be able to schedule in the 
    * if test "for free". */
   if (len != 1.0) {
      len = (float) (1.0 / sqrt ((double) len));
      ax[0] = axis[0] * len;
      ax[1] = axis[1] * len;
      ax[2] = axis[2] * len;
      urot_axis_f (m, angle, ax);
   } else {
      urot_axis_f (m, angle, axis);
   }
}

/* ========================================================== */

void urot_omega_f (float m[16], 	/* returned */
                   float axis[3])		/* input */
{
   float len, ax[3];

   /* normalize axis vector */
   len = axis[0]*axis[0] + axis[1]*axis[1] + axis[2]*axis[2];

   len = (float) (1.0 / sqrt ((double) len));
   ax[0] = axis[0] * len;
   ax[1] = axis[1] * len;
   ax[2] = axis[2] * len;

   /* the amount of rotation is equal to the length, in radians */
   urot_axis_f (m, len, ax);
}

/* ======================= END OF FILE ========================== */


```
