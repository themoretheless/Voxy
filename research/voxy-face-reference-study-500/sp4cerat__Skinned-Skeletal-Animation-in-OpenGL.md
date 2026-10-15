# Facial rendering source scan: sp4cerat/Skinned-Skeletal-Animation-in-OpenGL

Repository: https://github.com/sp4cerat/Skinned-Skeletal-Animation-in-OpenGL

Pinned source revision: 2436debd125b6ef946c99b8ca9f7f5b846aa8e60

Discovery topic: skin shader

Description: Skeletal Animation Tutorial using Shaders to Transform the Vertices. Based on the OGRE Mesh Format

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["shader/mesh/frag.txt", "shader/mesh/vs.txt", "shader/validation_tool/c.conf", "shader/validation_tool/glslangValidator.exe"]}

## Source file: include/IL/il.h

{
  "path": "include/IL/il.h",
  "blob_sha": "540a56eb3c1ba9ef435b6d583a1c1236478a196a",
  "sha256": "7ac71b076ab682f872bcd5236fbdd972cf41593c6ee8d444283f15f7ea829af3",
  "lines": 644,
  "bytes": 24718,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 11,
    "face": 9,
    "lip": 1
  },
  "url": "https://github.com/sp4cerat/Skinned-Skeletal-Animation-in-OpenGL/blob/2436debd125b6ef946c99b8ca9f7f5b846aa8e60/include/IL/il.h",
  "function_signature_candidates": []
}

```text

//-----------------------------------------------------------------------------
//
// ImageLib Sources
// Copyright (C) 2000-2009 by Denton Woods
// Last modified: 03/07/2009
//
// Filename: IL/il.h
//
// Description: The main include file for DevIL
//
//-----------------------------------------------------------------------------

// Doxygen comment
/*! \file il.h
    The main include file for DevIL
*/

#ifndef __il_h_
#ifndef __IL_H__

#define __il_h_
#define __IL_H__

#ifdef __cplusplus
extern "C" {
#endif

//this define controls if floats and doubles are clampled to [0..1]
//during conversion. It takes a little more time, but it is the correct
//way of doing this. If you are sure your floats are always valid,
//you can undefine this value...
#define CLAMP_HALF		1
#define CLAMP_FLOATS	1
#define CLAMP_DOUBLES	1

/*#ifdef _WIN32_WCE
	#define IL_NO_EXR
	#define IL_NO_GIF
	#define IL_NO_JP2
	#define IL_NO_JPG
	#define IL_NO_MNG
	#define IL_NO_PNG
	#define IL_NO_TIF
	#define IL_NO_LCMS
#endif //_WIN32_WCE

#ifdef DJGPP
	#define IL_NO_EXR
	#define IL_NO_GIF
	#define IL_NO_JP2
	#define IL_NO_JPG
	#define IL_NO_MNG
	#define IL_NO_PNG
	#define IL_NO_TIF
	#define IL_NO_LCMS
#endif //DJGPP*/

#ifdef _WIN32
	#if (defined(IL_USE_PRAGMA_LIBS)) && (!defined(_IL_BUILD_LIBRARY))
		#if defined(_MSC_VER) || defined(__BORLANDC__)
			#pragma comment(lib, "DevIL.lib")
		#endif
	#endif
#endif

#ifdef RESTRICT_KEYWORD
#define RESTRICT restrict
#define CONST_RESTRICT const restrict
#else
#define RESTRICT
#define CONST_RESTRICT const
#endif

#include <stdio.h>

typedef unsigned int   ILenum;
typedef unsigned char  ILboolean;
typedef unsigned int   ILbitfield;
typedef signed char    ILbyte;
typedef signed short   ILshort;
typedef int     	   ILint;
typedef size_t         ILsizei;
typedef unsigned char  ILubyte;
typedef unsigned short ILushort;
typedef unsigned int   ILuint;
typedef float          ILfloat;
typedef float          ILclampf;
typedef double         ILdouble;
typedef double         ILclampd;

#ifdef _MSC_VER
typedef __int64          ILint64;
typedef unsigned __int64 ILuint64;
#else
typedef long long int          ILint64;
typedef long long unsigned int ILuint64;
#endif


#include <limits.h>
#ifdef _UNICODE
	#ifndef _WIN32_WCE
		#include <wchar.h>
	#endif
	//if we use a define instead of a typedef,
	//ILconst_string works as intended
	#define ILchar wchar_t
	#define ILstring wchar_t*
	#define ILconst_string  wchar_t const *
#else
	//if we use a define instead of a typedef,
	//ILconst_string works as intended
	#define ILchar char
	#define ILstring char*
	#define ILconst_string char const *
#endif //_UNICODE

#define IL_FALSE			0
#define IL_TRUE				1

//  Matches OpenGL's right now.
//! Data formats \link Formats Formats\endlink
#define IL_COLOUR_INDEX     0x1900
#define IL_COLOR_INDEX      0x1900
#define IL_ALPHA			0x1906
#define IL_RGB              0x1907
#define IL_RGBA             0x1908
#define IL_BGR              0x80E0
#define IL_BGRA             0x80E1
#define IL_LUMINANCE        0x1909
#define IL_LUMINANCE_ALPHA  0x190A

//! Data types \link Types Types\endlink
#define IL_BYTE           0x1400
#define IL_UNSIGNED_BYTE  0x1401
#define IL_SHORT          0x1402
#define IL_UNSIGNED_SHORT 0x1403
#define IL_INT            0x1404
#define IL_UNSIGNED_INT   0x1405
#define IL_FLOAT          0x1406
#define IL_DOUBLE         0x140A
#define IL_HALF           0x140B


#define IL_MAX_BYTE		  		SCHAR_MAX
#define IL_MAX_UNSIGNED_BYTE  	UCHAR_MAX
#define IL_MAX_SHORT	  		SHRT_MAX
#define IL_MAX_UNSIGNED_SHORT 	USHRT_MAX
#define IL_MAX_INT		  		INT_MAX
#define IL_MAX_UNSIGNED_INT   	UINT_MAX

#define IL_LIMIT(x,m,M)		(x<m?m:(x>M?M:x))
#define IL_CLAMP(x) 		IL_LIMIT(x,0,1)

#define IL_VENDOR   0x1F00
#define IL_LOAD_EXT 0x1F01
#define IL_SAVE_EXT 0x1F02


//
// IL-specific #define's
//

#define IL_VERSION_1_7_8 1
#define IL_VERSION       178


// Attribute Bits
#define IL_ORIGIN_BIT          0x00000001
#define IL_FILE_BIT            0x00000002
#define IL_PAL_BIT             0x00000004
#define IL_FORMAT_BIT          0x00000008
#define IL_TYPE_BIT            0x00000010
#define IL_COMPRESS_BIT        0x00000020
#define IL_LOADFAIL_BIT        0x00000040
#define IL_FORMAT_SPECIFIC_BIT 0x00000080
#define IL_ALL_ATTRIB_BITS     0x000FFFFF


// Palette types
#define IL_PAL_NONE   0x0400
#define IL_PAL_RGB24  0x0401
#define IL_PAL_RGB32  0x0402
#define IL_PAL_RGBA32 0x0403
#define IL_PAL_BGR24  0x0404
#define IL_PAL_BGR32  0x0405
#define IL_PAL_BGRA32 0x0406


// Image types
#define IL_TYPE_UNKNOWN 0x0000
#define IL_BMP          0x0420  //!< Microsoft Windows Bitmap - .bmp extension
#define IL_CUT          0x0421  //!< Dr. Halo - .cut extension
#define IL_DOOM         0x0422  //!< DooM walls - no specific extension
#define IL_DOOM_FLAT    0x0423  //!< DooM flats - no specific extension
#define IL_ICO          0x0424  //!< Microsoft Windows Icons and Cursors - .ico and .cur extensions
#define IL_JPG          0x0425  //!< JPEG - .jpg, .jpe and .jpeg extensions
#define IL_JFIF         0x0425  //!<
#define IL_ILBM         0x0426  //!< Amiga IFF (FORM ILBM) - .iff, .ilbm, .lbm extensions
#define IL_PCD          0x0427  //!< Kodak PhotoCD - .pcd extension
#define IL_PCX          0x0428  //!< ZSoft PCX - .pcx extension
#define IL_PIC          0x0429  //!< PIC - .pic extension
#define IL_PNG          0x042A  //!< Portable Network Graphics - .png extension
#define IL_PNM          0x042B  //!< Portable Any Map - .pbm, .pgm, .ppm and .pnm extensions
#define IL_SGI          0x042C  //!< Silicon Graphics - .sgi, .bw, .rgb and .rgba extensions
#define IL_TGA          0x042D  //!< TrueVision Targa File - .tga, .vda, .icb and .vst extensions
#define IL_TIF          0x042E  //!< Tagged Image File Format - .tif and .tiff extensions
#define IL_CHEAD        0x042F  //!< C-Style Header - .h extension
#define IL_RAW          0x0430  //!< Raw Image Data - any extension
#define IL_MDL          0x0431  //!< Half-Life Model Texture - .mdl extension
#define IL_WAL          0x0432  //!< Quake 2 Texture - .wal extension
#define IL_LIF          0x0434  //!< Homeworld Texture - .lif extension
#define IL_MNG          0x0435  //!< Multiple-image Network Graphics - .mng extension
#define IL_JNG          0x0435  //!< 
#define IL_GIF          0x0436  //!< Graphics Interchange Format - .gif extension
#define IL_DDS          0x0437  //!< DirectDraw Surface - .dds extension
#define IL_DCX          0x0438  //!< ZSoft Multi-PCX - .dcx extension
#define IL_PSD          0x0439  //!< Adobe PhotoShop - .psd extension
#define IL_EXIF         0x043A  //!< 
#define IL_PSP          0x043B  //!< PaintShop Pro - .psp extension
#define IL_PIX          0x043C  //!< PIX - .pix extension
#define IL_PXR          0x043D  //!< Pixar - .pxr extension
#define IL_XPM          0x043E  //!< X Pixel Map - .xpm extension
#define IL_HDR          0x043F  //!< Radiance High Dynamic Range - .hdr extension
#define IL_ICNS			0x0440  //!< Macintosh Icon - .icns extension
#define IL_JP2			0x0441  //!< Jpeg 2000 - .jp2 extension
#define IL_EXR			0x0442  //!< OpenEXR - .exr extension
#define IL_WDP			0x0443  //!< Microsoft HD Photo - .wdp and .hdp extension
#define IL_VTF			0x0444  //!< Valve Texture Format - .vtf extension
#define IL_WBMP			0x0445  //!< Wireless Bitmap - .wbmp extension
#define IL_SUN			0x0446  //!< Sun Raster - .sun, .ras, .rs, .im1, .im8, .im24 and .im32 extensions
#define IL_IFF			0x0447  //!< Interchange File Format - .iff extension
#define IL_TPL			0x0448  //!< Gamecube Texture - .tpl extension
#define IL_FITS			0x0449  //!< Flexible Image Transport System - .fit and .fits extensions
#define IL_DICOM		0x044A  //!< Digital Imaging and Communications in Medicine (DICOM) - .dcm and .dicom extensions
#define IL_IWI			0x044B  //!< Call of Duty Infinity Ward Image - .iwi extension
#define IL_BLP			0x044C  //!< Blizzard Texture Format - .blp extension
#define IL_FTX			0x044D  //!< Heavy Metal: FAKK2 Texture - .ftx extension
#define IL_ROT			0x044E  //!< Homeworld 2 - Relic Texture - .rot extension
#define IL_TEXTURE		0x044F  //!< Medieval II: Total War Texture - .texture extension
#define IL_DPX			0x0450  //!< Digital Picture Exchange - .dpx extension
#define IL_UTX			0x0451  //!< Unreal (and Unreal Tournament) Texture - .utx extension
#define IL_MP3			0x0452  //!< MPEG-1 Audio Layer 3 - .mp3 extension


#define IL_JASC_PAL     0x0475  //!< PaintShop Pro Palette


// Error Types
#define IL_NO_ERROR             0x0000
#define IL_INVALID_ENUM         0x0501
#define IL_OUT_OF_MEMORY        0x0502
#define IL_FORMAT_NOT_SUPPORTED 0x0503
#define IL_INTERNAL_ERROR       0x0504
#define IL_INVALID_VALUE        0x0505
#define IL_ILLEGAL_OPERATION    0x0506
#define IL_ILLEGAL_FILE_VALUE   0x0507
#define IL_INVALID_FILE_HEADER  0x0508
#define IL_INVALID_PARAM        0x0509
#define IL_COULD_NOT_OPEN_FILE  0x050A
#define IL_INVALID_EXTENSION    0x050B
#define IL_FILE_ALREADY_EXISTS  0x050C
#define IL_OUT_FORMAT_SAME      0x050D
#define IL_STACK_OVERFLOW       0x050E
#define IL_STACK_UNDERFLOW      0x050F
#define IL_INVALID_CONVERSION   0x0510
#define IL_BAD_DIMENSIONS       0x0511
#define IL_FILE_READ_ERROR      0x0512  // 05/12/2002: Addition by Sam.
#define IL_FILE_WRITE_ERROR     0x0512

#define IL_LIB_GIF_ERROR  0x05E1
#define IL_LIB_JPEG_ERROR 0x05E2
#define IL_LIB_PNG_ERROR  0x05E3
#define IL_LIB_TIFF_ERROR 0x05E4
#define IL_LIB_MNG_ERROR  0x05E5
#define IL_LIB_JP2_ERROR  0x05E6
#define IL_LIB_EXR_ERROR  0x05E7
#define IL_UNKNOWN_ERROR  0x05FF


// Origin Definitions
#define IL_ORIGIN_SET        0x0600
#define IL_ORIGIN_LOWER_LEFT 0x0601
#define IL_ORIGIN_UPPER_LEFT 0x0602
#define IL_ORIGIN_MODE       0x0603


// Format and Type Mode Definitions
#define IL_FORMAT_SET  0x0610
#define IL_FORMAT_MODE 0x0611
#define IL_TYPE_SET    0x0612
#define IL_TYPE_MODE   0x0613


// File definitions
#define IL_FILE_OVERWRITE	0x0620
#define IL_FILE_MODE		0x0621


// Palette definitions
#define IL_CONV_PAL			0x0630


// Load fail definitions
#define IL_DEFAULT_ON_FAIL	0x0632


// Key colour and alpha definitions
#define IL_USE_KEY_COLOUR	0x0635
#define IL_USE_KEY_COLOR	0x0635
#define IL_BLIT_BLEND		0x0636


// Interlace definitions
#define IL_SAVE_INTERLACED	0x0639
#define IL_INTERLACE_MODE	0x063A


// Quantization definitions
#define IL_QUANTIZATION_MODE 0x0640
#define IL_WU_QUANT          0x0641
#define IL_NEU_QUANT         0x0642
#define IL_NEU_QUANT_SAMPLE  0x0643
#define IL_MAX_QUANT_INDEXS  0x0644 //XIX : ILint : Maximum number of colors to reduce to, default of 256. and has a range of 2-256
#define IL_MAX_QUANT_INDICES 0x0644 // Redefined, since the above #define is misspelled


// Hints
#define IL_FASTEST          0x0660
#define IL_LESS_MEM         0x0661
#define IL_DONT_CARE        0x0662
#define IL_MEM_SPEED_HINT   0x0665
#define IL_USE_COMPRESSION  0x0666
#define IL_NO_COMPRESSION   0x0667
#define IL_COMPRESSION_HINT 0x0668


// Compression
#define IL_NVIDIA_COMPRESS	0x0670
#define IL_SQUISH_COMPRESS	0x0671


// Subimage types
#define IL_SUB_NEXT   0x0680
#define IL_SUB_MIPMAP 0x0681
#define IL_SUB_LAYER  0x0682


// Compression definitions
#define IL_COMPRESS_MODE 0x0700
#define IL_COMPRESS_NONE 0x0701
#define IL_COMPRESS_RLE  0x0702
#define IL_COMPRESS_LZO  0x0703
#define IL_COMPRESS_ZLIB 0x0704


// File format-specific values
#define IL_TGA_CREATE_STAMP        0x0710
#define IL_JPG_QUALITY             0x0711
#define IL_PNG_INTERLACE           0x0712
#define IL_TGA_RLE                 0x0713
#define IL_BMP_RLE                 0x0714
#define IL_SGI_RLE                 0x0715
#define IL_TGA_ID_STRING           0x0717
#define IL_TGA_AUTHNAME_STRING     0x0718
#define IL_TGA_AUTHCOMMENT_STRING  0x0719
#define IL_PNG_AUTHNAME_STRING     0x071A
#define IL_PNG_TITLE_STRING        0x071B
#define IL_PNG_DESCRIPTION_STRING  0x071C
#define IL_TIF_DESCRIPTION_STRING  0x071D
#define IL_TIF_HOSTCOMPUTER_STRING 0x071E
#define IL_TIF_DOCUMENTNAME_STRING 0x071F
#define IL_TIF_AUTHNAME_STRING     0x0720
#define IL_JPG_SAVE_FORMAT         0x0721
#define IL_CHEAD_HEADER_STRING     0x0722
#define IL_PCD_PICNUM              0x0723
#define IL_PNG_ALPHA_INDEX 0x0724 //XIX : ILint : the color in the palette at this index value (0-255) is considered transparent, -1 for no trasparent color
#define IL_JPG_PROGRESSIVE         0x0725
#define IL_VTF_COMP                0x0726


// DXTC definitions
#define IL_DXTC_FORMAT      0x0705
#define IL_DXT1             0x0706
#define IL_DXT2             0x0707
#define IL_DXT3             0x0708
#define IL_DXT4             0x0709
#define IL_DXT5             0x070A
#define IL_DXT_NO_COMP      0x070B
#define IL_KEEP_DXTC_DATA   0x070C
#define IL_DXTC_DATA_FORMAT 0x070D
#define IL_3DC              0x070E
#define IL_RXGB             0x070F
#define IL_ATI1N            0x0710
#define IL_DXT1A            0x0711  // Normally the same as IL_DXT1, except for nVidia Texture Tools.

// Environment map definitions
#define IL_CUBEMAP_POSITIVEX 0x00000400
#define IL_CUBEMAP_NEGATIVEX 0x00000800
#define IL_CUBEMAP_POSITIVEY 0x00001000
#define IL_CUBEMAP_NEGATIVEY 0x00002000
#define IL_CUBEMAP_POSITIVEZ 0x00004000
#define IL_CUBEMAP_NEGATIVEZ 0x00008000
#define IL_SPHEREMAP         0x00010000


// Values
#define IL_VERSION_NUM           0x0DE2
#define IL_IMAGE_WIDTH           0x0DE4
#define IL_IMAGE_HEIGHT          0x0DE5
#define IL_IMAGE_DEPTH           0x0DE6
#define IL_IMAGE_SIZE_OF_DATA    0x0DE7
#define IL_IMAGE_BPP             0x0DE8
#define IL_IMAGE_BYTES_PER_PIXEL 0x0DE8
#define IL_IMAGE_BPP             0x0DE8
#define IL_IMAGE_BITS_PER_PIXEL  0x0DE9
#define IL_IMAGE_FORMAT          0x0DEA
#define IL_IMAGE_TYPE            0x0DEB
#define IL_PALETTE_TYPE          0x0DEC
#define IL_PALETTE_SIZE          0x0DED
#define IL_PALETTE_BPP           0x0DEE
#define IL_PALETTE_NUM_COLS      0x0DEF
#define IL_PALETTE_BASE_TYPE     0x0DF0
#define IL_NUM_FACES             0x0DE1
#define IL_NUM_IMAGES            0x0DF1
#define IL_NUM_MIPMAPS           0x0DF2
#define IL_NUM_LAYERS            0x0DF3
#define IL_ACTIVE_IMAGE          0x0DF4
#define IL_ACTIVE_MIPMAP         0x0DF5
#define IL_ACTIVE_LAYER          0x0DF6
#define IL_ACTIVE_FACE           0x0E00
#define IL_CUR_IMAGE             0x0DF7
#define IL_IMAGE_DURATION        0x0DF8
#define IL_IMAGE_PLANESIZE       0x0DF9
#define IL_IMAGE_BPC             0x0DFA
#define IL_IMAGE_OFFX            0x0DFB
#define IL_IMAGE_OFFY            0x0DFC
#define IL_IMAGE_CUBEFLAGS       0x0DFD
#define IL_IMAGE_ORIGIN          0x0DFE
#define IL_IMAGE_CHANNELS        0x0DFF

# if defined __GNUC__ && (__GNUC__ > 3 || (__GNUC__ == 3 && __GNUC_MINOR__ > 0))
// __attribute__((deprecated)) is supported by GCC 3.1 and later.
#  define DEPRECATED(D) D __attribute__((deprecated))
# elif defined _MSC_VER && _MSC_VER >= 1300
// __declspec(deprecated) is supported by MSVC 7.0 and later.
#  define DEPRECATED(D) __declspec(deprecated) D
# else
#  define DEPRECATED (D) D
# endif

//
// Section shamelessly modified from the glut header.
//

// This is from Win32's <windef.h>
#if (_MSC_VER >= 800) || defined(_STDCALL_SUPPORTED) || defined(__BORLANDC__) || defined(__LCC__)
	#define ILAPIENTRY __stdcall 
	#define IL_PACKSTRUCT
//#elif defined(linux) || defined(MACOSX) || defined(__CYGWIN__) //fix bug 840364
#elif defined( __GNUC__ )
  // this should work for any of the above commented platforms 
  // plus any platform using GCC
	#ifdef __MINGW32__
		#define ILAPIENTRY __stdcall
	#else
		#define ILAPIENTRY
	#endif
	#define IL_PACKSTRUCT __attribute__ ((packed))
#else
	#define ILAPIENTRY
	#define IL_PACKSTRUCT
#endif

// This is from Win32's <wingdi.h> and <winnt.h>
#if defined(__LCC__)
	#define ILAPI __stdcall
#elif defined(_WIN32) //changed 20031221 to fix bug 840421
	#ifdef IL_STATIC_LIB
		#define ILAPI
	#else
		#ifdef _IL_BUILD_LIBRARY
			#define ILAPI __declspec(dllexport)
		#else
			#define ILAPI __declspec(dllimport)
		#endif
	#endif
#elif __APPLE__
	#define ILAPI extern
#else
	#define ILAPI
#endif


#define IL_SEEK_SET	0
#define IL_SEEK_CUR	1
#define IL_SEEK_END	2
#define IL_EOF		-1


// Callback functions for file reading
typedef void* ILHANDLE;
typedef void      (ILAPIENTRY *fCloseRProc)(ILHANDLE);
typedef ILboolean (ILAPIENTRY *fEofProc)   (ILHANDLE);
typedef ILint     (ILAPIENTRY *fGetcProc)  (ILHANDLE);
typedef ILHANDLE  (ILAPIENTRY *fOpenRProc) (ILconst_string);
typedef ILint     (ILAPIENTRY *fReadProc)  (void*, ILuint, ILuint, ILHANDLE);
typedef ILint     (ILAPIENTRY *fSeekRProc) (ILHANDLE, ILint, ILint);
typedef ILint     (ILAPIENTRY *fTellRProc) (ILHANDLE);

// Callback functions for file writing
typedef void     (ILAPIENTRY *fCloseWProc)(ILHANDLE);
typedef ILHANDLE (ILAPIENTRY *fOpenWProc) (ILconst_string);
typedef ILint    (ILAPIENTRY *fPutcProc)  (ILubyte, ILHANDLE);
typedef ILint    (ILAPIENTRY *fSeekWProc) (ILHANDLE, ILint, ILint);
typedef ILint    (ILAPIENTRY *fTellWProc) (ILHANDLE);
typedef ILint    (ILAPIENTRY *fWriteProc) (const void*, ILuint, ILuint, ILHANDLE);

// Callback functions for allocation and deallocation
typedef void* (ILAPIENTRY *mAlloc)(const ILsizei);
typedef void  (ILAPIENTRY *mFree) (const void* CONST_RESTRICT);

// Registered format procedures
typedef ILenum (ILAPIENTRY *IL_LOADPROC)(ILconst_string);
typedef ILenum (ILAPIENTRY *IL_SAVEPROC)(ILconst_string);


// ImageLib Functions
ILAPI ILboolean ILAPIENTRY ilActiveFace(ILuint Number);
ILAPI ILboolean ILAPIENTRY ilActiveImage(ILuint Number);
ILAPI ILboolean ILAPIENTRY ilActiveLayer(ILuint Number);
ILAPI ILboolean ILAPIENTRY ilActiveMipmap(ILuint Number);
ILAPI ILboolean ILAPIENTRY ilApplyPal(ILconst_string FileName);
ILAPI ILboolean ILAPIENTRY ilApplyProfile(ILstring InProfile, ILstring OutProfile);
ILAPI void		ILAPIENTRY ilBindImage(ILuint Image);
ILAPI ILboolean ILAPIENTRY ilBlit(ILuint Source, ILint DestX, ILint DestY, ILint DestZ, ILuint SrcX, ILuint SrcY, ILuint SrcZ, ILuint Width, ILuint Height, ILuint Depth);
ILAPI ILboolean ILAPIENTRY ilClampNTSC(void);
ILAPI void		ILAPIENTRY ilClearColour(ILclampf Red, ILclampf Green, ILclampf Blue, ILclampf Alpha);
ILAPI ILboolean ILAPIENTRY ilClearImage(void);
ILAPI ILuint    ILAPIENTRY ilCloneCurImage(void);
ILAPI ILubyte*	ILAPIENTRY ilCompressDXT(ILubyte *Data, ILuint Width, ILuint Height, ILuint Depth, ILenum DXTCFormat, ILuint *DXTCSize);
ILAPI ILboolean ILAPIENTRY ilCompressFunc(ILenum Mode);
ILAPI ILboolean ILAPIENTRY ilConvertImage(ILenum DestFormat, ILenum DestType);
ILAPI ILboolean ILAPIENTRY ilConvertPal(ILenum DestFormat);
ILAPI ILboolean ILAPIENTRY ilCopyImage(ILuint Src);
ILAPI ILuint    ILAPIENTRY ilCopyPixels(ILuint XOff, ILuint YOff, ILuint ZOff, ILuint Width, ILuint Height, ILuint Depth, ILenum Format, ILenum Type, void *Data);
ILAPI ILuint    ILAPIENTRY ilCreateSubImage(ILenum Type, ILuint Num);
ILAPI ILboolean ILAPIENTRY ilDefaultImage(void);
ILAPI void		ILAPIENTRY ilDeleteImage(const ILuint Num);
ILAPI void      ILAPIENTRY ilDeleteImages(ILsizei Num, const ILuint *Images);
ILAPI ILenum	ILAPIENTRY ilDetermineType(ILconst_string FileName);
ILAPI ILenum	ILAPIENTRY ilDetermineTypeF(ILHANDLE File);
ILAPI ILenum	ILAPIENTRY ilDetermineTypeL(const void *Lump, ILuint Size);
ILAPI ILboolean ILAPIENTRY ilDisable(ILenum Mode);
ILAPI ILboolean ILAPIENTRY ilDxtcDataToImage(void);
ILAPI ILboolean ILAPIENTRY ilDxtcDataToSurface(void);
ILAPI ILboolean ILAPIENTRY ilEnable(ILenum Mode);
ILAPI void		ILAPIENTRY ilFlipSurfaceDxtcData(void);
ILAPI ILboolean ILAPIENTRY ilFormatFunc(ILenum Mode);
ILAPI void	    ILAPIENTRY ilGenImages(ILsizei Num, ILuint *Images);
ILAPI ILuint	ILAPIENTRY ilGenImage(void);
ILAPI ILubyte*  ILAPIENTRY ilGetAlpha(ILenum Type);
ILAPI ILboolean ILAPIENTRY ilGetBoolean(ILenum Mode);
ILAPI void      ILAPIENTRY ilGetBooleanv(ILenum Mode, ILboolean *Param);
ILAPI ILubyte*  ILAPIENTRY ilGetData(void);
ILAPI ILuint    ILAPIENTRY ilGetDXTCData(void *Buffer, ILuint BufferSize, ILenum DXTCFormat);
ILAPI ILenum    ILAPIENTRY ilGetError(void);
ILAPI ILint     ILAPIENTRY ilGetInteger(ILenum Mode);
ILAPI void      ILAPIENTRY ilGetIntegerv(ILenum Mode, ILint *Param);
ILAPI ILuint    ILAPIENTRY ilGetLumpPos(void);
ILAPI ILubyte*  ILAPIENTRY ilGetPalette(void);
ILAPI ILconst_string  ILAPIENTRY ilGetString(ILenum StringName);
ILAPI void      ILAPIENTRY ilHint(ILenum Target, ILenum Mode);
ILAPI ILboolean	ILAPIENTRY ilInvertSurfaceDxtcDataAlpha(void);
ILAPI void      ILAPIENTRY ilInit(void);
ILAPI ILboolean ILAPIENTRY ilImageToDxtcData(ILenum Format);
ILAPI ILboolean ILAPIENTRY ilIsDisabled(ILenum Mode);
ILAPI ILboolean ILAPIENTRY ilIsEnabled(ILenum Mode);
ILAPI ILboolean ILAPIENTRY ilIsImage(ILuint Image);
ILAPI ILboolean ILAPIENTRY ilIsValid(ILenum Type, ILconst_string FileName);
ILAPI ILboolean ILAPIENTRY ilIsValidF(ILenum Type, ILHANDLE File);
ILAPI ILboolean ILAPIENTRY ilIsValidL(ILenum Type, void *Lump, ILuint Size);
ILAPI void      ILAPIENTRY ilKeyColour(ILclampf Red, ILclampf Green, ILclampf Blue, ILclampf Alpha);
ILAPI ILboolean ILAPIENTRY ilLoad(ILenum Type, ILconst_string FileName);
ILAPI ILboolean ILAPIENTRY ilLoadF(ILenum Type, ILHANDLE File);
ILAPI ILboolean ILAPIENTRY ilLoadImage(ILconst_string FileName);
ILAPI ILboolean ILAPIENTRY ilLoadL(ILenum Type, const void *Lump, ILuint Size);
ILAPI ILboolean ILAPIENTRY ilLoadPal(ILconst_string FileName);
ILAPI void      ILAPIENTRY ilModAlpha(ILdouble AlphaValue);
ILAPI ILboolean ILAPIENTRY ilOriginFunc(ILenum Mode);
ILAPI ILboolean ILAPIENTRY ilOverlayImage(ILuint Source, ILint XCoord, ILint YCoord, ILint ZCoord);
ILAPI void      ILAPIENTRY ilPopAttrib(void);
ILAPI void      ILAPIENTRY ilPushAttrib(ILuint Bits);
ILAPI void      ILAPIENTRY ilRegisterFormat(ILenum Format);
ILAPI ILboolean ILAPIENTRY ilRegisterLoad(ILconst_string Ext, IL_LOADPROC Load);
ILAPI ILboolean ILAPIENTRY ilRegisterMipNum(ILuint Num);
ILAPI ILboolean ILAPIENTRY ilRegisterNumFaces(ILuint Num);
ILAPI ILboolean ILAPIENTRY ilRegisterNumImages(ILuint Num);
ILAPI void      ILAPIENTRY ilRegisterOrigin(ILenum Origin);
ILAPI void      ILAPIENTRY ilRegisterPal(void *Pal, ILuint Size, ILenum Type);
ILAPI ILboolean ILAPIENTRY ilRegisterSave(ILconst_string Ext, IL_SAVEPROC Save);
ILAPI void      ILAPIENTRY ilRegisterType(ILenum Type);
ILAPI ILboolean ILAPIENTRY ilRemoveLoad(ILconst_string Ext);
ILAPI ILboolean ILAPIENTRY ilRemoveSave(ILconst_string Ext);
ILAPI void      ILAPIENTRY ilResetMemory(void); // Deprecated
ILAPI void      ILAPIENTRY ilResetRead(void);
ILAPI void      ILAPIENTRY ilResetWrite(void);
ILAPI ILboolean ILAPIENTRY ilSave(ILenum Type, ILconst_string FileName);
ILAPI ILuint    ILAPIENTRY ilSaveF(ILenum Type, ILHANDLE File);
ILAPI ILboolean ILAPIENTRY ilSaveImage(ILconst_string FileName);
ILAPI ILuint    ILAPIENTRY ilSaveL(ILenum Type, void *Lump, ILuint Size);
ILAPI ILboolean ILAPIENTRY ilSavePal(ILconst_string FileName);
ILAPI ILboolean ILAPIENTRY ilSetAlpha(ILdouble AlphaValue);
ILAPI ILboolean ILAPIENTRY ilSetData(void *Data);
ILAPI ILboolean ILAPIENTRY ilSetDuration(ILuint Duration);
ILAPI void      ILAPIENTRY ilSetInteger(ILenum Mode, ILint Param);
ILAPI void      ILAPIENTRY ilSetMemory(mAlloc, mFree);
ILAPI void      ILAPIENTRY ilSetPixels(ILint XOff, ILint YOff, ILint ZOff, ILuint Width, ILuint Height, ILuint Depth, ILenum Format, ILenum Type, void *Data);
ILAPI void      ILAPIENTRY ilSetRead(fOpenRProc, fCloseRProc, fEofProc, fGetcProc, fReadProc, fSeekRProc, fTellRProc);
ILAPI void      ILAPIENTRY ilSetString(ILenum Mode, const char *String);
ILAPI void      ILAPIENTRY ilSetWrite(fOpenWProc, fCloseWProc, fPutcProc, fSeekWProc, fTellWProc, fWriteProc);
ILAPI void      ILAPIENTRY ilShutDown(void);
ILAPI ILboolean ILAPIENTRY ilSurfaceToDxtcData(ILenum Format);
ILAPI ILboolean ILAPIENTRY ilTexImage(ILuint Width, ILuint Height, ILuint Depth, ILubyte NumChannels, ILenum Format, ILenum Type, void *Data);
ILAPI ILboolean ILAPIENTRY ilTexImageDxtc(ILint w, ILint h, ILint d, ILenum DxtFormat, const ILubyte* data);
ILAPI ILenum    ILAPIENTRY ilTypeFromExt(ILconst_string FileName);
ILAPI ILboolean ILAPIENTRY ilTypeFunc(ILenum Mode);
ILAPI ILboolean ILAPIENTRY ilLoadData(ILconst_string FileName, ILuint Width, ILuint Height, ILuint Depth, ILubyte Bpp);
ILAPI ILboolean ILAPIENTRY ilLoadDataF(ILHANDLE File, ILuint Width, ILuint Height, ILuint Depth, ILubyte Bpp);
ILAPI ILboolean ILAPIENTRY ilLoadDataL(void *Lump, ILuint Size, ILuint Width, ILuint Height, ILuint Depth, ILubyte Bpp);
ILAPI ILboolean ILAPIENTRY ilSaveData(ILconst_string FileName);

// For all those weirdos that spell "colour" without the 'u'.
#define ilClearColor	ilClearColour
#define ilKeyColor      ilKeyColour

#define imemclear(x,y) memset(x,0,y);

#ifdef __cplusplus
}
#endif

#endif // __IL_H__
#endif // __il_h__


```

## Source file: include/glxew.h

{
  "path": "include/glxew.h",
  "blob_sha": "33ec893d1374d359183e7f98012a4eb42231d58e",
  "sha256": "416f74ea4d6cfe35919e8290bc2c5bda8e46df6369fe27bb67379cc11bac9110",
  "lines": 1510,
  "bytes": 59279,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 29,
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/sp4cerat/Skinned-Skeletal-Animation-in-OpenGL/blob/2436debd125b6ef946c99b8ca9f7f5b846aa8e60/include/glxew.h",
  "function_signature_candidates": [
    {
      "line": 148,
      "text": "extern int glXGetConfig (Display *dpy, XVisualInfo *vis, int attrib, int *value);"
    },
    {
      "line": 151,
      "text": "extern void glXDestroyGLXPixmap (Display *dpy, GLXPixmap pix);"
    },
    {
      "line": 153,
      "text": "extern void glXDestroyContext (Display *dpy, GLXContext ctx);"
    },
    {
      "line": 155,
      "text": "extern void glXCopyContext (Display *dpy, GLXContext src, GLXContext dst, GLulong mask);"
    },
    {
      "line": 159,
      "text": "extern void glXWaitGL (void);"
    },
    {
      "line": 160,
      "text": "extern void glXWaitX (void);"
    },
    {
      "line": 161,
      "text": "extern void glXSwapBuffers (Display *dpy, GLXDrawable drawable);"
    },
    {
      "line": 162,
      "text": "extern void glXUseXFont (Font font, int first, int count, int listBase);"
    }
  ]
}

```text

/*
** The OpenGL Extension Wrangler Library
** Copyright (C) 2002-2008, Milan Ikits <milan ikits[]ieee org>
** Copyright (C) 2002-2008, Marcelo E. Magallon <mmagallo[]debian org>
** Copyright (C) 2002, Lev Povalahev
** All rights reserved.
** 
** Redistribution and use in source and binary forms, with or without 
** modification, are permitted provided that the following conditions are met:
** 
** * Redistributions of source code must retain the above copyright notice, 
**   this list of conditions and the following disclaimer.
** * Redistributions in binary form must reproduce the above copyright notice, 
**   this list of conditions and the following disclaimer in the documentation 
**   and/or other materials provided with the distribution.
** * The name of the author may be used to endorse or promote products 
**   derived from this software without specific prior written permission.
**
** THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" 
** AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE 
** IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
** ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE 
** LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR 
** CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF 
** SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
** INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
** CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
** ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF
** THE POSSIBILITY OF SUCH DAMAGE.
*/

/*
 * Mesa 3-D graphics library
 * Version:  7.0
 *
 * Copyright (C) 1999-2007  Brian Paul   All Rights Reserved.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included
 * in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
 * OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
 * BRIAN PAUL BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN
 * AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

/*
** Copyright (c) 2007 The Khronos Group Inc.
** 
** Permission is hereby granted, free of charge, to any person obtaining a
** copy of this software and/or associated documentation files (the
** "Materials"), to deal in the Materials without restriction, including
** without limitation the rights to use, copy, modify, merge, publish,
** distribute, sublicense, and/or sell copies of the Materials, and to
** permit persons to whom the Materials are furnished to do so, subject to
** the following conditions:
** 
** The above copyright notice and this permission notice shall be included
** in all copies or substantial portions of the Materials.
** 
** THE MATERIALS ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
** EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
** MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
** IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
** CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
** TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
** MATERIALS OR THE USE OR OTHER DEALINGS IN THE MATERIALS.
*/

#ifndef __glxew_h__
#define __glxew_h__
#define __GLXEW_H__

#ifdef __glxext_h_
#error glxext.h included before glxew.h
#endif

#if defined(GLX_H) || defined(__GLX_glx_h__) || defined(__glx_h__)
#error glx.h included before glxew.h
#endif

#define __glxext_h_

#define GLX_H
#define __GLX_glx_h__
#define __glx_h__

#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/Xmd.h>
#include <glew.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---------------------------- GLX_VERSION_1_0 --------------------------- */

#ifndef GLX_VERSION_1_0
#define GLX_VERSION_1_0 1

#define GLX_USE_GL 1
#define GLX_BUFFER_SIZE 2
#define GLX_LEVEL 3
#define GLX_RGBA 4
#define GLX_DOUBLEBUFFER 5
#define GLX_STEREO 6
#define GLX_AUX_BUFFERS 7
#define GLX_RED_SIZE 8
#define GLX_GREEN_SIZE 9
#define GLX_BLUE_SIZE 10
#define GLX_ALPHA_SIZE 11
#define GLX_DEPTH_SIZE 12
#define GLX_STENCIL_SIZE 13
#define GLX_ACCUM_RED_SIZE 14
#define GLX_ACCUM_GREEN_SIZE 15
#define GLX_ACCUM_BLUE_SIZE 16
#define GLX_ACCUM_ALPHA_SIZE 17
#define GLX_BAD_SCREEN 1
#define GLX_BAD_ATTRIBUTE 2
#define GLX_NO_EXTENSION 3
#define GLX_BAD_VISUAL 4
#define GLX_BAD_CONTEXT 5
#define GLX_BAD_VALUE 6
#define GLX_BAD_ENUM 7

typedef XID GLXDrawable;
typedef XID GLXPixmap;
#ifdef __sun
typedef struct __glXContextRec *GLXContext;
#else
typedef struct __GLXcontextRec *GLXContext;
#endif

typedef unsigned int GLXVideoDeviceNV; 

extern Bool glXQueryExtension (Display *dpy, int *errorBase, int *eventBase);
extern Bool glXQueryVersion (Display *dpy, int *major, int *minor);
extern int glXGetConfig (Display *dpy, XVisualInfo *vis, int attrib, int *value);
extern XVisualInfo* glXChooseVisual (Display *dpy, int screen, int *attribList);
extern GLXPixmap glXCreateGLXPixmap (Display *dpy, XVisualInfo *vis, Pixmap pixmap);
extern void glXDestroyGLXPixmap (Display *dpy, GLXPixmap pix);
extern GLXContext glXCreateContext (Display *dpy, XVisualInfo *vis, GLXContext shareList, Bool direct);
extern void glXDestroyContext (Display *dpy, GLXContext ctx);
extern Bool glXIsDirect (Display *dpy, GLXContext ctx);
extern void glXCopyContext (Display *dpy, GLXContext src, GLXContext dst, GLulong mask);
extern Bool glXMakeCurrent (Display *dpy, GLXDrawable drawable, GLXContext ctx);
extern GLXContext glXGetCurrentContext (void);
extern GLXDrawable glXGetCurrentDrawable (void);
extern void glXWaitGL (void);
extern void glXWaitX (void);
extern void glXSwapBuffers (Display *dpy, GLXDrawable drawable);
extern void glXUseXFont (Font font, int first, int count, int listBase);

#define GLXEW_VERSION_1_0 GLXEW_GET_VAR(__GLXEW_VERSION_1_0)

#endif /* GLX_VERSION_1_0 */

/* ---------------------------- GLX_VERSION_1_1 --------------------------- */

#ifndef GLX_VERSION_1_1
#define GLX_VERSION_1_1

#define GLX_VENDOR 0x1
#define GLX_VERSION 0x2
#define GLX_EXTENSIONS 0x3

extern const char* glXQueryExtensionsString (Display *dpy, int screen);
extern const char* glXGetClientString (Display *dpy, int name);
extern const char* glXQueryServerString (Display *dpy, int screen, int name);

#define GLXEW_VERSION_1_1 GLXEW_GET_VAR(__GLXEW_VERSION_1_1)

#endif /* GLX_VERSION_1_1 */

/* ---------------------------- GLX_VERSION_1_2 ---------------------------- */

#ifndef GLX_VERSION_1_2
#define GLX_VERSION_1_2 1

typedef Display* ( * PFNGLXGETCURRENTDISPLAYPROC) (void);

#define glXGetCurrentDisplay GLXEW_GET_FUN(__glewXGetCurrentDisplay)

#define GLXEW_VERSION_1_2 GLXEW_GET_VAR(__GLXEW_VERSION_1_2)

#endif /* GLX_VERSION_1_2 */

/* ---------------------------- GLX_VERSION_1_3 ---------------------------- */

#ifndef GLX_VERSION_1_3
#define GLX_VERSION_1_3 1

#define GLX_RGBA_BIT 0x00000001
#define GLX_FRONT_LEFT_BUFFER_BIT 0x00000001
#define GLX_WINDOW_BIT 0x00000001
#define GLX_COLOR_INDEX_BIT 0x00000002
#define GLX_PIXMAP_BIT 0x00000002
#define GLX_FRONT_RIGHT_BUFFER_BIT 0x00000002
#define GLX_BACK_LEFT_BUFFER_BIT 0x00000004
#define GLX_PBUFFER_BIT 0x00000004
#define GLX_BACK_RIGHT_BUFFER_BIT 0x00000008
#define GLX_AUX_BUFFERS_BIT 0x00000010
#define GLX_CONFIG_CAVEAT 0x20
#define GLX_DEPTH_BUFFER_BIT 0x00000020
#define GLX_X_VISUAL_TYPE 0x22
#define GLX_TRANSPARENT_TYPE 0x23
#define GLX_TRANSPARENT_INDEX_VALUE 0x24
#define GLX_TRANSPARENT_RED_VALUE 0x25
#define GLX_TRANSPARENT_GREEN_VALUE 0x26
#define GLX_TRANSPARENT_BLUE_VALUE 0x27
#define GLX_TRANSPARENT_ALPHA_VALUE 0x28
#define GLX_STENCIL_BUFFER_BIT 0x00000040
#define GLX_ACCUM_BUFFER_BIT 0x00000080
#define GLX_NONE 0x8000
#define GLX_SLOW_CONFIG 0x8001
#define GLX_TRUE_COLOR 0x8002
#define GLX_DIRECT_COLOR 0x8003
#define GLX_PSEUDO_COLOR 0x8004
#define GLX_STATIC_COLOR 0x8005
#define GLX_GRAY_SCALE 0x8006
#define GLX_STATIC_GRAY 0x8007
#define GLX_TRANSPARENT_RGB 0x8008
#define GLX_TRANSPARENT_INDEX 0x8009
#define GLX_VISUAL_ID 0x800B
#define GLX_SCREEN 0x800C
#define GLX_NON_CONFORMANT_CONFIG 0x800D
#define GLX_DRAWABLE_TYPE 0x8010
#define GLX_RENDER_TYPE 0x8011
#define GLX_X_RENDERABLE 0x8012
#define GLX_FBCONFIG_ID 0x8013
#define GLX_RGBA_TYPE 0x8014
#define GLX_COLOR_INDEX_TYPE 0x8015
#define GLX_MAX_PBUFFER_WIDTH 0x8016
#define GLX_MAX_PBUFFER_HEIGHT 0x8017
#define GLX_MAX_PBUFFER_PIXELS 0x8018
#define GLX_PRESERVED_CONTENTS 0x801B
#define GLX_LARGEST_PBUFFER 0x801C
#define GLX_WIDTH 0x801D
#define GLX_HEIGHT 0x801E
#define GLX_EVENT_MASK 0x801F
#define GLX_DAMAGED 0x8020
#define GLX_SAVED 0x8021
#define GLX_WINDOW 0x8022
#define GLX_PBUFFER 0x8023
#define GLX_PBUFFER_HEIGHT 0x8040
#define GLX_PBUFFER_WIDTH 0x8041
#define GLX_PBUFFER_CLOBBER_MASK 0x08000000
#define GLX_DONT_CARE 0xFFFFFFFF

typedef XID GLXFBConfigID;
typedef XID GLXWindow;
typedef XID GLXPbuffer;
typedef struct __GLXFBConfigRec *GLXFBConfig;

typedef struct {
  int event_type; 
  int draw_type; 
  unsigned long serial; 
  Bool send_event; 
  Display *display; 
  GLXDrawable drawable; 
  unsigned int buffer_mask; 
  unsigned int aux_buffer; 
  int x, y; 
  int width, height; 
  int count; 
} GLXPbufferClobberEvent;
typedef union __GLXEvent {
  GLXPbufferClobberEvent glxpbufferclobber; 
  long pad[24]; 
} GLXEvent;

typedef GLXFBConfig* ( * PFNGLXCHOOSEFBCONFIGPROC) (Display *dpy, int screen, const int *attrib_list, int *nelements);
typedef GLXContext ( * PFNGLXCREATENEWCONTEXTPROC) (Display *dpy, GLXFBConfig config, int render_type, GLXContext share_list, Bool direct);
typedef GLXPbuffer ( * PFNGLXCREATEPBUFFERPROC) (Display *dpy, GLXFBConfig config, const int *attrib_list);
typedef GLXPixmap ( * PFNGLXCREATEPIXMAPPROC) (Display *dpy, GLXFBConfig config, Pixmap pixmap, const int *attrib_list);
typedef GLXWindow ( * PFNGLXCREATEWINDOWPROC) (Display *dpy, GLXFBConfig config, Window win, const int *attrib_list);
typedef void ( * PFNGLXDESTROYPBUFFERPROC) (Display *dpy, GLXPbuffer pbuf);
typedef void ( * PFNGLXDESTROYPIXMAPPROC) (Display *dpy, GLXPixmap pixmap);
typedef void ( * PFNGLXDESTROYWINDOWPROC) (Display *dpy, GLXWindow win);
typedef GLXDrawable ( * PFNGLXGETCURRENTREADDRAWABLEPROC) (void);
typedef int ( * PFNGLXGETFBCONFIGATTRIBPROC) (Display *dpy, GLXFBConfig config, int attribute, int *value);
typedef GLXFBConfig* ( * PFNGLXGETFBCONFIGSPROC) (Display *dpy, int screen, int *nelements);
typedef void ( * PFNGLXGETSELECTEDEVENTPROC) (Display *dpy, GLXDrawable draw, unsigned long *event_mask);
typedef XVisualInfo* ( * PFNGLXGETVISUALFROMFBCONFIGPROC) (Display *dpy, GLXFBConfig config);
typedef Bool ( * PFNGLXMAKECONTEXTCURRENTPROC) (Display *display, GLXDrawable draw, GLXDrawable read, GLXContext ctx);
typedef int ( * PFNGLXQUERYCONTEXTPROC) (Display *dpy, GLXContext ctx, int attribute, int *value);
typedef void ( * PFNGLXQUERYDRAWABLEPROC) (Display *dpy, GLXDrawable draw, int attribute, unsigned int *value);
typedef void ( * PFNGLXSELECTEVENTPROC) (Display *dpy, GLXDrawable draw, unsigned long event_mask);

#define glXChooseFBConfig GLXEW_GET_FUN(__glewXChooseFBConfig)
#define glXCreateNewContext GLXEW_GET_FUN(__glewXCreateNewContext)
#define glXCreatePbuffer GLXEW_GET_FUN(__glewXCreatePbuffer)
#define glXCreatePixmap GLXEW_GET_FUN(__glewXCreatePixmap)
#define glXCreateWindow GLXEW_GET_FUN(__glewXCreateWindow)
#define glXDestroyPbuffer GLXEW_GET_FUN(__glewXDestroyPbuffer)
#define glXDestroyPixmap GLXEW_GET_FUN(__glewXDestroyPixmap)
#define glXDestroyWindow GLXEW_GET_FUN(__glewXDestroyWindow)
#define glXGetCurrentReadDrawable GLXEW_GET_FUN(__glewXGetCurrentReadDrawable)
#define glXGetFBConfigAttrib GLXEW_GET_FUN(__glewXGetFBConfigAttrib)
#define glXGetFBConfigs GLXEW_GET_FUN(__glewXGetFBConfigs)
#define glXGetSelectedEvent GLXEW_GET_FUN(__glewXGetSelectedEvent)
#define glXGetVisualFromFBConfig GLXEW_GET_FUN(__glewXGetVisualFromFBConfig)
#define glXMakeContextCurrent GLXEW_GET_FUN(__glewXMakeContextCurrent)
#define glXQueryContext GLXEW_GET_FUN(__glewXQueryContext)
#define glXQueryDrawable GLXEW_GET_FUN(__glewXQueryDrawable)
#define glXSelectEvent GLXEW_GET_FUN(__glewXSelectEvent)

#define GLXEW_VERSION_1_3 GLXEW_GET_VAR(__GLXEW_VERSION_1_3)

#endif /* GLX_VERSION_1_3 */

/* ---------------------------- GLX_VERSION_1_4 ---------------------------- */

#ifndef GLX_VERSION_1_4
#define GLX_VERSION_1_4 1

#define GLX_SAMPLE_BUFFERS 100000
#define GLX_SAMPLES 100001

extern void ( * glXGetProcAddress (const GLubyte *procName)) (void);

#define GLXEW_VERSION_1_4 GLXEW_GET_VAR(__GLXEW_VERSION_1_4)

#endif /* GLX_VERSION_1_4 */

/* -------------------------- GLX_3DFX_multisample ------------------------- */

#ifndef GLX_3DFX_multisample
#define GLX_3DFX_multisample 1

#define GLX_SAMPLE_BUFFERS_3DFX 0x8050
#define GLX_SAMPLES_3DFX 0x8051

#define GLXEW_3DFX_multisample GLXEW_GET_VAR(__GLXEW_3DFX_multisample)

#endif /* GLX_3DFX_multisample */

/* ------------------------ GLX_AMD_gpu_association ------------------------ */

#ifndef GLX_AMD_gpu_association
#define GLX_AMD_gpu_association 1

#define GLX_GPU_VENDOR_AMD 0x1F00
#define GLX_GPU_RENDERER_STRING_AMD 0x1F01
#define GLX_GPU_OPENGL_VERSION_STRING_AMD 0x1F02
#define GLX_GPU_FASTEST_TARGET_GPUS_AMD 0x21A2
#define GLX_GPU_RAM_AMD 0x21A3
#define GLX_GPU_CLOCK_AMD 0x21A4
#define GLX_GPU_NUM_PIPES_AMD 0x21A5
#define GLX_GPU_NUM_SIMD_AMD 0x21A6
#define GLX_GPU_NUM_RB_AMD 0x21A7
#define GLX_GPU_NUM_SPI_AMD 0x21A8

#define GLXEW_AMD_gpu_association GLXEW_GET_VAR(__GLXEW_AMD_gpu_association)

#endif /* GLX_AMD_gpu_association */

/* ------------------------- GLX_ARB_create_context ------------------------ */

#ifndef GLX_ARB_create_context
#define GLX_ARB_create_context 1

#define GLX_CONTEXT_DEBUG_BIT_ARB 0x0001
#define GLX_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB 0x0002
#define GLX_CONTEXT_MAJOR_VERSION_ARB 0x2091
#define GLX_CONTEXT_MINOR_VERSION_ARB 0x2092
#define GLX_CONTEXT_FLAGS_ARB 0x2094

typedef GLXContext ( * PFNGLXCREATECONTEXTATTRIBSARBPROC) (Display* dpy, GLXFBConfig config, GLXContext share_context, Bool direct, const int *attrib_list);

#define glXCreateContextAttribsARB GLXEW_GET_FUN(__glewXCreateContextAttribsARB)

#define GLXEW_ARB_create_context GLXEW_GET_VAR(__GLXEW_ARB_create_context)

#endif /* GLX_ARB_create_context */

/* --------------------- GLX_ARB_create_context_profile -------------------- */

#ifndef GLX_ARB_create_context_profile
#define GLX_ARB_create_context_profile 1

#define GLX_CONTEXT_CORE_PROFILE_BIT_ARB 0x00000001
#define GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB 0x00000002
#define GLX_CONTEXT_PROFILE_MASK_ARB 0x9126

#define GLXEW_ARB_create_context_profile GLXEW_GET_VAR(__GLXEW_ARB_create_context_profile)

#endif /* GLX_ARB_create_context_profile */

/* ------------------------- GLX_ARB_fbconfig_float ------------------------ */

#ifndef GLX_ARB_fbconfig_float
#define GLX_ARB_fbconfig_float 1

#define GLX_RGBA_FLOAT_BIT 0x00000004
#define GLX_RGBA_FLOAT_TYPE 0x20B9

#define GLXEW_ARB_fbconfig_float GLXEW_GET_VAR(__GLXEW_ARB_fbconfig_float)

#endif /* GLX_ARB_fbconfig_float */

/* ------------------------ GLX_ARB_framebuffer_sRGB ----------------------- */

#ifndef GLX_ARB_framebuffer_sRGB
#define GLX_ARB_framebuffer_sRGB 1

#define GLX_FRAMEBUFFER_SRGB_CAPABLE_ARB 0x20B2

#define GLXEW_ARB_framebuffer_sRGB GLXEW_GET_VAR(__GLXEW_ARB_framebuffer_sRGB)

#endif /* GLX_ARB_framebuffer_sRGB */

/* ------------------------ GLX_ARB_get_proc_address ----------------------- */

#ifndef GLX_ARB_get_proc_address
#define GLX_ARB_get_proc_address 1

extern void ( * glXGetProcAddressARB (const GLubyte *procName)) (void);

#define GLXEW_ARB_get_proc_address GLXEW_GET_VAR(__GLXEW_ARB_get_proc_address)

#endif /* GLX_ARB_get_proc_address */

/* -------------------------- GLX_ARB_multisample -------------------------- */

#ifndef GLX_ARB_multisample
#define GLX_ARB_multisample 1

#define GLX_SAMPLE_BUFFERS_ARB 100000
#define GLX_SAMPLES_ARB 100001

#define GLXEW_ARB_multisample GLXEW_GET_VAR(__GLXEW_ARB_multisample)

#endif /* GLX_ARB_multisample */

/* ---------------------- GLX_ARB_vertex_buffer_object --------------------- */

#ifndef GLX_ARB_vertex_buffer_object
#define GLX_ARB_vertex_buffer_object 1

#define GLX_CONTEXT_ALLOW_BUFFER_BYTE_ORDER_MISMATCH_ARB 0x2095

#define GLXEW_ARB_vertex_buffer_object GLXEW_GET_VAR(__GLXEW_ARB_vertex_buffer_object)

#endif /* GLX_ARB_vertex_buffer_object */

/* ----------------------- GLX_ATI_pixel_format_float ---------------------- */

#ifndef GLX_ATI_pixel_format_float
#define GLX_ATI_pixel_format_float 1

#define GLX_RGBA_FLOAT_ATI_BIT 0x00000100

#define GLXEW_ATI_pixel_format_float GLXEW_GET_VAR(__GLXEW_ATI_pixel_format_float)

#endif /* GLX_ATI_pixel_format_float */

/* ------------------------- GLX_ATI_render_texture ------------------------ */

#ifndef GLX_ATI_render_texture
#define GLX_ATI_render_texture 1

#define GLX_BIND_TO_TEXTURE_RGB_ATI 0x9800
#define GLX_BIND_TO_TEXTURE_RGBA_ATI 0x9801
#define GLX_TEXTURE_FORMAT_ATI 0x9802
#define GLX_TEXTURE_TARGET_ATI 0x9803
#define GLX_MIPMAP_TEXTURE_ATI 0x9804
#define GLX_TEXTURE_RGB_ATI 0x9805
#define GLX_TEXTURE_RGBA_ATI 0x9806
#define GLX_NO_TEXTURE_ATI 0x9807
#define GLX_TEXTURE_CUBE_MAP_ATI 0x9808
#define GLX_TEXTURE_1D_ATI 0x9809
#define GLX_TEXTURE_2D_ATI 0x980A
#define GLX_MIPMAP_LEVEL_ATI 0x980B
#define GLX_CUBE_MAP_FACE_ATI 0x980C
#define GLX_TEXTURE_CUBE_MAP_POSITIVE_X_ATI 0x980D
#define GLX_TEXTURE_CUBE_MAP_NEGATIVE_X_ATI 0x980E
#define GLX_TEXTURE_CUBE_MAP_POSITIVE_Y_ATI 0x980F
#define GLX_TEXTURE_CUBE_MAP_NEGATIVE_Y_ATI 0x9810
#define GLX_TEXTURE_CUBE_MAP_POSITIVE_Z_ATI 0x9811
#define GLX_TEXTURE_CUBE_MAP_NEGATIVE_Z_ATI 0x9812
#define GLX_FRONT_LEFT_ATI 0x9813
#define GLX_FRONT_RIGHT_ATI 0x9814
#define GLX_BACK_LEFT_ATI 0x9815
#define GLX_BACK_RIGHT_ATI 0x9816
#define GLX_AUX0_ATI 0x9817
#define GLX_AUX1_ATI 0x9818
#define GLX_AUX2_ATI 0x9819
#define GLX_AUX3_ATI 0x981A
#define GLX_AUX4_ATI 0x981B
#define GLX_AUX5_ATI 0x981C
#define GLX_AUX6_ATI 0x981D
#define GLX_AUX7_ATI 0x981E
#define GLX_AUX8_ATI 0x981F
#define GLX_AUX9_ATI 0x9820
#define GLX_BIND_TO_TEXTURE_LUMINANCE_ATI 0x9821
#define GLX_BIND_TO_TEXTURE_INTENSITY_ATI 0x9822

typedef void ( * PFNGLXBINDTEXIMAGEATIPROC) (Display *dpy, GLXPbuffer pbuf, int buffer);
typedef void ( * PFNGLXDRAWABLEATTRIBATIPROC) (Display *dpy, GLXDrawable draw, const int *attrib_list);
typedef void ( * PFNGLXRELEASETEXIMAGEATIPROC) (Display *dpy, GLXPbuffer pbuf, int buffer);

#define glXBindTexImageATI GLXEW_GET_FUN(__glewXBindTexImageATI)
#define glXDrawableAttribATI GLXEW_GET_FUN(__glewXDrawableAttribATI)
#define glXReleaseTexImageATI GLXEW_GET_FUN(__glewXReleaseTexImageATI)

#define GLXEW_ATI_render_texture GLXEW_GET_VAR(__GLXEW_ATI_render_texture)

#endif /* GLX_ATI_render_texture */

/* --------------------- GLX_EXT_fbconfig_packed_float --------------------- */

#ifndef GLX_EXT_fbconfig_packed_float
#define GLX_EXT_fbconfig_packed_float 1

#define GLX_RGBA_UNSIGNED_FLOAT_BIT_EXT 0x00000008
#define GLX_RGBA_UNSIGNED_FLOAT_TYPE_EXT 0x20B1

#define GLXEW_EXT_fbconfig_packed_float GLXEW_GET_VAR(__GLXEW_EXT_fbconfig_packed_float)

#endif /* GLX_EXT_fbconfig_packed_float */

/* ------------------------ GLX_EXT_framebuffer_sRGB ----------------------- */

#ifndef GLX_EXT_framebuffer_sRGB
#define GLX_EXT_framebuffer_sRGB 1

#define GLX_FRAMEBUFFER_SRGB_CAPABLE_EXT 0x20B2

#define GLXEW_EXT_framebuffer_sRGB GLXEW_GET_VAR(__GLXEW_EXT_framebuffer_sRGB)

#endif /* GLX_EXT_framebuffer_sRGB */

/* ------------------------- GLX_EXT_import_context ------------------------ */

#ifndef GLX_EXT_import_context
#define GLX_EXT_import_context 1

#define GLX_SHARE_CONTEXT_EXT 0x800A
#define GLX_VISUAL_ID_EXT 0x800B
#define GLX_SCREEN_EXT 0x800C

typedef XID GLXContextID;

typedef void ( * PFNGLXFREECONTEXTEXTPROC) (Display* dpy, GLXContext context);
typedef GLXContextID ( * PFNGLXGETCONTEXTIDEXTPROC) (const GLXContext context);
typedef GLXContext ( * PFNGLXIMPORTCONTEXTEXTPROC) (Display* dpy, GLXContextID contextID);
typedef int ( * PFNGLXQUERYCONTEXTINFOEXTPROC) (Display* dpy, GLXContext context, int attribute,int *value);

#define glXFreeContextEXT GLXEW_GET_FUN(__glewXFreeContextEXT)
#define glXGetContextIDEXT GLXEW_GET_FUN(__glewXGetContextIDEXT)
#define glXImportContextEXT GLXEW_GET_FUN(__glewXImportContextEXT)
#define glXQueryContextInfoEXT GLXEW_GET_FUN(__glewXQueryContextInfoEXT)

#define GLXEW_EXT_import_context GLXEW_GET_VAR(__GLXEW_EXT_import_context)

#endif /* GLX_EXT_import_context */

/* -------------------------- GLX_EXT_scene_marker ------------------------- */

#ifndef GLX_EXT_scene_marker
#define GLX_EXT_scene_marker 1

#define GLXEW_EXT_scene_marker GLXEW_GET_VAR(__GLXEW_EXT_scene_marker)

#endif /* GLX_EXT_scene_marker */

/* -------------------------- GLX_EXT_swap_control ------------------------- */

#ifndef GLX_EXT_swap_control
#define GLX_EXT_swap_control 1

#define GLX_SWAP_INTERVAL_EXT 0x20F1
#define GLX_MAX_SWAP_INTERVAL_EXT 0x20F2

typedef void ( * PFNGLXSWAPINTERVALEXTPROC) (Display* dpy, GLXDrawable drawable, int interval);

#define glXSwapIntervalEXT GLXEW_GET_FUN(__glewXSwapIntervalEXT)

#define GLXEW_EXT_swap_control GLXEW_GET_VAR(__GLXEW_EXT_swap_control)

#endif /* GLX_EXT_swap_control */

/* ---------------------- GLX_EXT_texture_from_pixmap ---------------------- */

#ifndef GLX_EXT_texture_from_pixmap
#define GLX_EXT_texture_from_pixmap 1

#define GLX_TEXTURE_1D_BIT_EXT 0x00000001
#define GLX_TEXTURE_2D_BIT_EXT 0x00000002
#define GLX_TEXTURE_RECTANGLE_BIT_EXT 0x00000004
#define GLX_BIND_TO_TEXTURE_RGB_EXT 0x20D0
#define GLX_BIND_TO_TEXTURE_RGBA_EXT 0x20D1
#define GLX_BIND_TO_MIPMAP_TEXTURE_EXT 0x20D2
#define GLX_BIND_TO_TEXTURE_TARGETS_EXT 0x20D3
#define GLX_Y_INVERTED_EXT 0x20D4
#define GLX_TEXTURE_FORMAT_EXT 0x20D5
#define GLX_TEXTURE_TARGET_EXT 0x20D6
#define GLX_MIPMAP_TEXTURE_EXT 0x20D7
#define GLX_TEXTURE_FORMAT_NONE_EXT 0x20D8
#define GLX_TEXTURE_FORMAT_RGB_EXT 0x20D9
#define GLX_TEXTURE_FORMAT_RGBA_EXT 0x20DA
#define GLX_TEXTURE_1D_EXT 0x20DB
#define GLX_TEXTURE_2D_EXT 0x20DC
#define GLX_TEXTURE_RECTANGLE_EXT 0x20DD
#define GLX_FRONT_LEFT_EXT 0x20DE
#define GLX_FRONT_RIGHT_EXT 0x20DF
#define GLX_BACK_LEFT_EXT 0x20E0
#define GLX_BACK_RIGHT_EXT 0x20E1
#define GLX_AUX0_EXT 0x20E2
#define GLX_AUX1_EXT 0x20E3
#define GLX_AUX2_EXT 0x20E4
#define GLX_AUX3_EXT 0x20E5
#define GLX_AUX4_EXT 0x20E6
#define GLX_AUX5_EXT 0x20E7
#define GLX_AUX6_EXT 0x20E8
#define GLX_AUX7_EXT 0x20E9
#define GLX_AUX8_EXT 0x20EA
#define GLX_AUX9_EXT 0x20EB

typedef void ( * PFNGLXBINDTEXIMAGEEXTPROC) (Display* display, GLXDrawable drawable, int buffer, const int *attrib_list);
typedef void ( * PFNGLXRELEASETEXIMAGEEXTPROC) (Display* display, GLXDrawable drawable, int buffer);

#define glXBindTexImageEXT GLXEW_GET_FUN(__glewXBindTexImageEXT)
#define glXReleaseTexImageEXT GLXEW_GET_FUN(__glewXReleaseTexImageEXT)

#define GLXEW_EXT_texture_from_pixmap GLXEW_GET_VAR(__GLXEW_EXT_texture_from_pixmap)

#endif /* GLX_EXT_texture_from_pixmap */

/* -------------------------- GLX_EXT_visual_info -------------------------- */

#ifndef GLX_EXT_visual_info
#define GLX_EXT_visual_info 1

#define GLX_X_VISUAL_TYPE_EXT 0x22
#define GLX_TRANSPARENT_TYPE_EXT 0x23
#define GLX_TRANSPARENT_INDEX_VALUE_EXT 0x24
#define GLX_TRANSPARENT_RED_VALUE_EXT 0x25
#define GLX_TRANSPARENT_GREEN_VALUE_EXT 0x26
#define GLX_TRANSPARENT_BLUE_VALUE_EXT 0x27
#define GLX_TRANSPARENT_ALPHA_VALUE_EXT 0x28
#define GLX_NONE_EXT 0x8000
#define GLX_TRUE_COLOR_EXT 0x8002
#define GLX_DIRECT_COLOR_EXT 0x8003
#define GLX_PSEUDO_COLOR_EXT 0x8004
#define GLX_STATIC_COLOR_EXT 0x8005
#define GLX_GRAY_SCALE_EXT 0x8006
#define GLX_STATIC_GRAY_EXT 0x8007
#define GLX_TRANSPARENT_RGB_EXT 0x8008
#define GLX_TRANSPARENT_INDEX_EXT 0x8009

#define GLXEW_EXT_visual_info GLXEW_GET_VAR(__GLXEW_EXT_visual_info)

#endif /* GLX_EXT_visual_info */

/* ------------------------- GLX_EXT_visual_rating ------------------------- */

#ifndef GLX_EXT_visual_rating
#define GLX_EXT_visual_rating 1

#define GLX_VISUAL_CAVEAT_EXT 0x20
#define GLX_SLOW_VISUAL_EXT 0x8001
#define GLX_NON_CONFORMANT_VISUAL_EXT 0x800D

#define GLXEW_EXT_visual_rating GLXEW_GET_VAR(__GLXEW_EXT_visual_rating)

#endif /* GLX_EXT_visual_rating */

/* -------------------------- GLX_INTEL_swap_event ------------------------- */

#ifndef GLX_INTEL_swap_event
#define GLX_INTEL_swap_event 1

#define GLX_EXCHANGE_COMPLETE_INTEL 0x8180
#define GLX_COPY_COMPLETE_INTEL 0x8181
#define GLX_FLIP_COMPLETE_INTEL 0x8182
#define GLX_BUFFER_SWAP_COMPLETE_INTEL_MASK 0x04000000

#define GLXEW_INTEL_swap_event GLXEW_GET_VAR(__GLXEW_INTEL_swap_event)

#endif /* GLX_INTEL_swap_event */

/* -------------------------- GLX_MESA_agp_offset -------------------------- */

#ifndef GLX_MESA_agp_offset
#define GLX_MESA_agp_offset 1

typedef unsigned int ( * PFNGLXGETAGPOFFSETMESAPROC) (const void* pointer);

#define glXGetAGPOffsetMESA GLXEW_GET_FUN(__glewXGetAGPOffsetMESA)

#define GLXEW_MESA_agp_offset GLXEW_GET_VAR(__GLXEW_MESA_agp_offset)

#endif /* GLX_MESA_agp_offset */

/* ------------------------ GLX_MESA_copy_sub_buffer ----------------------- */

#ifndef GLX_MESA_copy_sub_buffer
#define GLX_MESA_copy_sub_buffer 1

typedef void ( * PFNGLXCOPYSUBBUFFERMESAPROC) (Display* dpy, GLXDrawable drawable, int x, int y, int width, int height);

#define glXCopySubBufferMESA GLXEW_GET_FUN(__glewXCopySubBufferMESA)

#define GLXEW_MESA_copy_sub_buffer GLXEW_GET_VAR(__GLXEW_MESA_copy_sub_buffer)

#endif /* GLX_MESA_copy_sub_buffer */

/* ------------------------ GLX_MESA_pixmap_colormap ----------------------- */

#ifndef GLX_MESA_pixmap_colormap
#define GLX_MESA_pixmap_colormap 1

typedef GLXPixmap ( * PFNGLXCREATEGLXPIXMAPMESAPROC) (Display* dpy, XVisualInfo *visual, Pixmap pixmap, Colormap cmap);

#define glXCreateGLXPixmapMESA GLXEW_GET_FUN(__glewXCreateGLXPixmapMESA)

#define GLXEW_MESA_pixmap_colormap GLXEW_GET_VAR(__GLXEW_MESA_pixmap_colormap)

#endif /* GLX_MESA_pixmap_colormap */

/* ------------------------ GLX_MESA_release_buffers ----------------------- */

#ifndef GLX_MESA_release_buffers
#define GLX_MESA_release_buffers 1

typedef Bool ( * PFNGLXRELEASEBUFFERSMESAPROC) (Display* dpy, GLXDrawable d);

#define glXReleaseBuffersMESA GLXEW_GET_FUN(__glewXReleaseBuffersMESA)

#define GLXEW_MESA_release_buffers GLXEW_GET_VAR(__GLXEW_MESA_release_buffers)

#endif /* GLX_MESA_release_buffers */

/* ------------------------- GLX_MESA_set_3dfx_mode ------------------------ */

#ifndef GLX_MESA_set_3dfx_mode
#define GLX_MESA_set_3dfx_mode 1

#define GLX_3DFX_WINDOW_MODE_MESA 0x1
#define GLX_3DFX_FULLSCREEN_MODE_MESA 0x2

typedef GLboolean ( * PFNGLXSET3DFXMODEMESAPROC) (GLint mode);

#define glXSet3DfxModeMESA GLXEW_GET_FUN(__glewXSet3DfxModeMESA)

#define GLXEW_MESA_set_3dfx_mode GLXEW_GET_VAR(__GLXEW_MESA_set_3dfx_mode)

#endif /* GLX_MESA_set_3dfx_mode */

/* --------------------------- GLX_NV_copy_image --------------------------- */

#ifndef GLX_NV_copy_image
#define GLX_NV_copy_image 1

typedef void ( * PFNGLXCOPYIMAGESUBDATANVPROC) (Display *dpy, GLXContext srcCtx, GLuint srcName, GLenum srcTarget, GLint srcLevel, GLint srcX, GLint srcY, GLint srcZ, GLXContext dstCtx, GLuint dstName, GLenum dstTarget, GLint dstLevel, GLint dstX, GLint dstY, GLint dstZ, GLsizei width, GLsizei height, GLsizei depth);

#define glXCopyImageSubDataNV GLXEW_GET_FUN(__glewXCopyImageSubDataNV)

#define GLXEW_NV_copy_image GLXEW_GET_VAR(__GLXEW_NV_copy_image)

#endif /* GLX_NV_copy_image */

/* -------------------------- GLX_NV_float_buffer -------------------------- */

#ifndef GLX_NV_float_buffer
#define GLX_NV_float_buffer 1

#define GLX_FLOAT_COMPONENTS_NV 0x20B0

#define GLXEW_NV_float_buffer GLXEW_GET_VAR(__GLXEW_NV_float_buffer)

#endif /* GLX_NV_float_buffer */

/* ---------------------- GLX_NV_multisample_coverage ---------------------- */

#ifndef GLX_NV_multisample_coverage
#define GLX_NV_multisample_coverage 1

#define GLX_COLOR_SAMPLES_NV 0x20B3
#define GLX_COVERAGE_SAMPLES_NV 100001

#define GLXEW_NV_multisample_coverage GLXEW_GET_VAR(__GLXEW_NV_multisample_coverage)

#endif /* GLX_NV_multisample_coverage */

/* -------------------------- GLX_NV_present_video ------------------------- */

#ifndef GLX_NV_present_video
#define GLX_NV_present_video 1

#define GLX_NUM_VIDEO_SLOTS_NV 0x20F0

typedef int ( * PFNGLXBINDVIDEODEVICENVPROC) (Display* dpy, unsigned int video_slot, unsigned int video_device, const int *attrib_list);
typedef unsigned int* ( * PFNGLXENUMERATEVIDEODEVICESNVPROC) (Display *dpy, int screen, int *nelements);

#define glXBindVideoDeviceNV GLXEW_GET_FUN(__glewXBindVideoDeviceNV)
#define glXEnumerateVideoDevicesNV GLXEW_GET_FUN(__glewXEnumerateVideoDevicesNV)

#define GLXEW_NV_present_video GLXEW_GET_VAR(__GLXEW_NV_present_video)

#endif /* GLX_NV_present_video */

/* --------------------------- GLX_NV_swap_group --------------------------- */

#ifndef GLX_NV_swap_group
#define GLX_NV_swap_group 1

typedef Bool ( * PFNGLXBINDSWAPBARRIERNVPROC) (Display* dpy, GLuint group, GLuint barrier);
typedef Bool ( * PFNGLXJOINSWAPGROUPNVPROC) (Display* dpy, GLXDrawable drawable, GLuint group);
typedef Bool ( * PFNGLXQUERYFRAMECOUNTNVPROC) (Display* dpy, int screen, GLuint *count);
typedef Bool ( * PFNGLXQUERYMAXSWAPGROUPSNVPROC) (Display* dpy, int screen, GLuint *maxGroups, GLuint *maxBarriers);
typedef Bool ( * PFNGLXQUERYSWAPGROUPNVPROC) (Display* dpy, GLXDrawable drawable, GLuint *group, GLuint *barrier);
typedef Bool ( * PFNGLXRESETFRAMECOUNTNVPROC) (Display* dpy, int screen);

#define glXBindSwapBarrierNV GLXEW_GET_FUN(__glewXBindSwapBarrierNV)
#define glXJoinSwapGroupNV GLXEW_GET_FUN(__glewXJoinSwapGroupNV)
#define glXQueryFrameCountNV GLXEW_GET_FUN(__glewXQueryFrameCountNV)
#define glXQueryMaxSwapGroupsNV GLXEW_GET_FUN(__glewXQueryMaxSwapGroupsNV)
#define glXQuerySwapGroupNV GLXEW_GET_FUN(__glewXQuerySwapGroupNV)
#define glXResetFrameCountNV GLXEW_GET_FUN(__glewXResetFrameCountNV)

#define GLXEW_NV_swap_group GLXEW_GET_VAR(__GLXEW_NV_swap_group)

#endif /* GLX_NV_swap_group */

/* ----------------------- GLX_NV_vertex_array_range ----------------------- */

#ifndef GLX_NV_vertex_array_range
#define GLX_NV_vertex_array_range 1

typedef void * ( * PFNGLXALLOCATEMEMORYNVPROC) (GLsizei size, GLfloat readFrequency, GLfloat writeFrequency, GLfloat priority);
typedef void ( * PFNGLXFREEMEMORYNVPROC) (void *pointer);

#define glXAllocateMemoryNV GLXEW_GET_FUN(__glewXAllocateMemoryNV)
#define glXFreeMemoryNV GLXEW_GET_FUN(__glewXFreeMemoryNV)

#define GLXEW_NV_vertex_array_range GLXEW_GET_VAR(__GLXEW_NV_vertex_array_range)

#endif /* GLX_NV_vertex_array_range */

/* -------------------------- GLX_NV_video_output -------------------------- */

#ifndef GLX_NV_video_output
#define GLX_NV_video_output 1

#define GLX_VIDEO_OUT_COLOR_NV 0x20C3
#define GLX_VIDEO_OUT_ALPHA_NV 0x20C4
#define GLX_VIDEO_OUT_DEPTH_NV 0x20C5
#define GLX_VIDEO_OUT_COLOR_AND_ALPHA_NV 0x20C6
#define GLX_VIDEO_OUT_COLOR_AND_DEPTH_NV 0x20C7
#define GLX_VIDEO_OUT_FRAME_NV 0x20C8
#define GLX_VIDEO_OUT_FIELD_1_NV 0x20C9
#define GLX_VIDEO_OUT_FIELD_2_NV 0x20CA
#define GLX_VIDEO_OUT_STACKED_FIELDS_1_2_NV 0x20CB
#define GLX_VIDEO_OUT_STACKED_FIELDS_2_1_NV 0x20CC

typedef int ( * PFNGLXBINDVIDEOIMAGENVPROC) (Display* dpy, GLXVideoDeviceNV VideoDevice, GLXPbuffer pbuf, int iVideoBuffer);
typedef int ( * PFNGLXGETVIDEODEVICENVPROC) (Display* dpy, int screen, int numVideoDevices, GLXVideoDeviceNV *pVideoDevice);
typedef int ( * PFNGLXGETVIDEOINFONVPROC) (Display* dpy, int screen, GLXVideoDeviceNV VideoDevice, unsigned long *pulCounterOutputPbuffer, unsigned long *pulCounterOutputVideo);
typedef int ( * PFNGLXRELEASEVIDEODEVICENVPROC) (Display* dpy, int screen, GLXVideoDeviceNV VideoDevice);
typedef int ( * PFNGLXRELEASEVIDEOIMAGENVPROC) (Display* dpy, GLXPbuffer pbuf);
typedef int ( * PFNGLXSENDPBUFFERTOVIDEONVPROC) (Display* dpy, GLXPbuffer pbuf, int iBufferType, unsigned long *pulCounterPbuffer, GLboolean bBlock);

#define glXBindVideoImageNV GLXEW_GET_FUN(__glewXBindVideoImageNV)
#define glXGetVideoDeviceNV GLXEW_GET_FUN(__glewXGetVideoDeviceNV)
#define glXGetVideoInfoNV GLXEW_GET_FUN(__glewXGetVideoInfoNV)
#define glXReleaseVideoDeviceNV GLXEW_GET_FUN(__glewXReleaseVideoDeviceNV)
#define glXReleaseVideoImageNV GLXEW_GET_FUN(__glewXReleaseVideoImageNV)
#define glXSendPbufferToVideoNV GLXEW_GET_FUN(__glewXSendPbufferToVideoNV)

#define GLXEW_NV_video_output GLXEW_GET_VAR(__GLXEW_NV_video_output)

#endif /* GLX_NV_video_output */

/* -------------------------- GLX_OML_swap_method -------------------------- */

#ifndef GLX_OML_swap_method
#define GLX_OML_swap_method 1

#define GLX_SWAP_METHOD_OML 0x8060
#define GLX_SWAP_EXCHANGE_OML 0x8061
#define GLX_SWAP_COPY_OML 0x8062
#define GLX_SWAP_UNDEFINED_OML 0x8063

#define GLXEW_OML_swap_method GLXEW_GET_VAR(__GLXEW_OML_swap_method)

#endif /* GLX_OML_swap_method */

/* -------------------------- GLX_OML_sync_control ------------------------- */

#if !defined(GLX_OML_sync_control) && defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 199901L)
#include <inttypes.h>
#define GLX_OML_sync_control 1

typedef Bool ( * PFNGLXGETMSCRATEOMLPROC) (Display* dpy, GLXDrawable drawable, int32_t* numerator, int32_t* denominator);
typedef Bool ( * PFNGLXGETSYNCVALUESOMLPROC) (Display* dpy, GLXDrawable drawable, int64_t* ust, int64_t* msc, int64_t* sbc);
typedef int64_t ( * PFNGLXSWAPBUFFERSMSCOMLPROC) (Display* dpy, GLXDrawable drawable, int64_t target_msc, int64_t divisor, int64_t remainder);
typedef Bool ( * PFNGLXWAITFORMSCOMLPROC) (Display* dpy, GLXDrawable drawable, int64_t target_msc, int64_t divisor, int64_t remainder, int64_t* ust, int64_t* msc, int64_t* sbc);
typedef Bool ( * PFNGLXWAITFORSBCOMLPROC) (Display* dpy, GLXDrawable drawable, int64_t target_sbc, int64_t* ust, int64_t* msc, int64_t* sbc);

#define glXGetMscRateOML GLXEW_GET_FUN(__glewXGetMscRateOML)
#define glXGetSyncValuesOML GLXEW_GET_FUN(__glewXGetSyncValuesOML)
#define glXSwapBuffersMscOML GLXEW_GET_FUN(__glewXSwapBuffersMscOML)
#define glXWaitForMscOML GLXEW_GET_FUN(__glewXWaitForMscOML)
#define glXWaitForSbcOML GLXEW_GET_FUN(__glewXWaitForSbcOML)

#define GLXEW_OML_sync_control GLXEW_GET_VAR(__GLXEW_OML_sync_control)

#endif /* GLX_OML_sync_control */

/* ------------------------ GLX_SGIS_blended_overlay ----------------------- */

#ifndef GLX_SGIS_blended_overlay
#define GLX_SGIS_blended_overlay 1

#define GLX_BLENDED_RGBA_SGIS 0x8025

#define GLXEW_SGIS_blended_overlay GLXEW_GET_VAR(__GLXEW_SGIS_blended_overlay)

#endif /* GLX_SGIS_blended_overlay */

/* -------------------------- GLX_SGIS_color_range ------------------------- */

#ifndef GLX_SGIS_color_range
#define GLX_SGIS_color_range 1

#define GLX_MIN_RED_SGIS 0
#define GLX_MAX_GREEN_SGIS 0
#define GLX_MIN_BLUE_SGIS 0
#define GLX_MAX_ALPHA_SGIS 0
#define GLX_MIN_GREEN_SGIS 0
#define GLX_MIN_ALPHA_SGIS 0
#define GLX_MAX_RED_SGIS 0
#define GLX_EXTENDED_RANGE_SGIS 0
#define GLX_MAX_BLUE_SGIS 0

#define GLXEW_SGIS_color_range GLXEW_GET_VAR(__GLXEW_SGIS_color_range)

#endif /* GLX_SGIS_color_range */

/* -------------------------- GLX_SGIS_multisample ------------------------- */

#ifndef GLX_SGIS_multisample
#define GLX_SGIS_multisample 1

#define GLX_SAMPLE_BUFFERS_SGIS 100000
#define GLX_SAMPLES_SGIS 100001

#define GLXEW_SGIS_multisample GLXEW_GET_VAR(__GLXEW_SGIS_multisample)

#endif /* GLX_SGIS_multisample */

/* ---------------------- GLX_SGIS_shared_multisample ---------------------- */

#ifndef GLX_SGIS_shared_multisample
#define GLX_SGIS_shared_multisample 1

#define GLX_MULTISAMPLE_SUB_RECT_WIDTH_SGIS 0x8026
#define GLX_MULTISAMPLE_SUB_RECT_HEIGHT_SGIS 0x8027

#define GLXEW_SGIS_shared_multisample GLXEW_GET_VAR(__GLXEW_SGIS_shared_multisample)

#endif /* GLX_SGIS_shared_multisample */

/* --------------------------- GLX_SGIX_fbconfig --------------------------- */

#ifndef GLX_SGIX_fbconfig
#define GLX_SGIX_fbconfig 1

#define GLX_WINDOW_BIT_SGIX 0x00000001
#define GLX_RGBA_BIT_SGIX 0x00000001
#define GLX_PIXMAP_BIT_SGIX 0x00000002
#define GLX_COLOR_INDEX_BIT_SGIX 0x00000002
#define GLX_SCREEN_EXT 0x800C
#define GLX_DRAWABLE_TYPE_SGIX 0x8010
#define GLX_RENDER_TYPE_SGIX 0x8011
#define GLX_X_RENDERABLE_SGIX 0x8012
#define GLX_FBCONFIG_ID_SGIX 0x8013
#define GLX_RGBA_TYPE_SGIX 0x8014
#define GLX_COLOR_INDEX_TYPE_SGIX 0x8015

typedef XID GLXFBConfigIDSGIX;
typedef struct __GLXFBConfigRec *GLXFBConfigSGIX;

typedef GLXFBConfigSGIX* ( * PFNGLXCHOOSEFBCONFIGSGIXPROC) (Display *dpy, int screen, const int *attrib_list, int *nelements);
typedef GLXContext ( * PFNGLXCREATECONTEXTWITHCONFIGSGIXPROC) (Display* dpy, GLXFBConfig config, int render_type, GLXContext share_list, Bool direct);
typedef GLXPixmap ( * PFNGLXCREATEGLXPIXMAPWITHCONFIGSGIXPROC) (Display* dpy, GLXFBConfig config, Pixmap pixmap);
typedef int ( * PFNGLXGETFBCONFIGATTRIBSGIXPROC) (Display* dpy, GLXFBConfigSGIX config, int attribute, int *value);
typedef GLXFBConfigSGIX ( * PFNGLXGETFBCONFIGFROMVISUALSGIXPROC) (Display* dpy, XVisualInfo *vis);
typedef XVisualInfo* ( * PFNGLXGETVISUALFROMFBCONFIGSGIXPROC) (Display *dpy, GLXFBConfig config);

#define glXChooseFBConfigSGIX GLXEW_GET_FUN(__glewXChooseFBConfigSGIX)
#define glXCreateContextWithConfigSGIX GLXEW_GET_FUN(__glewXCreateContextWithConfigSGIX)
#define glXCreateGLXPixmapWithConfigSGIX GLXEW_GET_FUN(__glewXCreateGLXPixmapWithConfigSGIX)
#define glXGetFBConfigAttribSGIX GLXEW_GET_FUN(__glewXGetFBConfigAttribSGIX)
#define glXGetFBConfigFromVisualSGIX GLXEW_GET_FUN(__glewXGetFBConfigFromVisualSGIX)
#define glXGetVisualFromFBConfigSGIX GLXEW_GET_FUN(__glewXGetVisualFromFBConfigSGIX)

#define GLXEW_SGIX_fbconfig GLXEW_GET_VAR(__GLXEW_SGIX_fbconfig)

#endif /* GLX_SGIX_fbconfig */

/* --------------------------- GLX_SGIX_hyperpipe -------------------------- */

#ifndef GLX_SGIX_hyperpipe
#define GLX_SGIX_hyperpipe 1

#define GLX_HYPERPIPE_DISPLAY_PIPE_SGIX 0x00000001
#define GLX_PIPE_RECT_SGIX 0x00000001
#define GLX_PIPE_RECT_LIMITS_SGIX 0x00000002
#define GLX_HYPERPIPE_RENDER_PIPE_SGIX 0x00000002
#define GLX_HYPERPIPE_STEREO_SGIX 0x00000003
#define GLX_HYPERPIPE_PIXEL_AVERAGE_SGIX 0x00000004
#define GLX_HYPERPIPE_PIPE_NAME_LENGTH_SGIX 80
#define GLX_BAD_HYPERPIPE_CONFIG_SGIX 91
#define GLX_BAD_HYPERPIPE_SGIX 92
#define GLX_HYPERPIPE_ID_SGIX 0x8030

typedef struct {
  char pipeName[GLX_HYPERPIPE_PIPE_NAME_LENGTH_SGIX]; 
  int  networkId; 
} GLXHyperpipeNetworkSGIX;
typedef struct {
  char pipeName[GLX_HYPERPIPE_PIPE_NAME_LENGTH_SGIX]; 
  int XOrigin; 
  int YOrigin; 
  int maxHeight; 
  int maxWidth; 
} GLXPipeRectLimits;
typedef struct {
  char pipeName[GLX_HYPERPIPE_PIPE_NAME_LENGTH_SGIX]; 
  int channel; 
  unsigned int participationType; 
  int timeSlice; 
} GLXHyperpipeConfigSGIX;
typedef struct {
  char pipeName[GLX_HYPERPIPE_PIPE_NAME_LENGTH_SGIX]; 
  int srcXOrigin; 
  int srcYOrigin; 
  int srcWidth; 
  int srcHeight; 
  int destXOrigin; 
  int destYOrigin; 
  int destWidth; 
  int destHeight; 
} GLXPipeRect;

typedef int ( * PFNGLXBINDHYPERPIPESGIXPROC) (Display *dpy, int hpId);
typedef int ( * PFNGLXDESTROYHYPERPIPECONFIGSGIXPROC) (Display *dpy, int hpId);
typedef int ( * PFNGLXHYPERPIPEATTRIBSGIXPROC) (Display *dpy, int timeSlice, int attrib, int size, void *attribList);
typedef int ( * PFNGLXHYPERPIPECONFIGSGIXPROC) (Display *dpy, int networkId, int npipes, GLXHyperpipeConfigSGIX *cfg, int *hpId);
typedef int ( * PFNGLXQUERYHYPERPIPEATTRIBSGIXPROC) (Display *dpy, int timeSlice, int attrib, int size, void *returnAttribList);
typedef int ( * PFNGLXQUERYHYPERPIPEBESTATTRIBSGIXPROC) (Display *dpy, int timeSlice, int attrib, int size, void *attribList, void *returnAttribList);
typedef GLXHyperpipeConfigSGIX * ( * PFNGLXQUERYHYPERPIPECONFIGSGIXPROC) (Display *dpy, int hpId, int *npipes);
typedef GLXHyperpipeNetworkSGIX * ( * PFNGLXQUERYHYPERPIPENETWORKSGIXPROC) (Display *dpy, int *npipes);

#define glXBindHyperpipeSGIX GLXEW_GET_FUN(__glewXBindHyperpipeSGIX)
#define glXDestroyHyperpipeConfigSGIX GLXEW_GET_FUN(__glewXDestroyHyperpipeConfigSGIX)
#define glXHyperpipeAttribSGIX GLXEW_GET_FUN(__glewXHyperpipeAttribSGIX)
#define glXHyperpipeConfigSGIX GLXEW_GET_FUN(__glewXHyperpipeConfigSGIX)
#define glXQueryHyperpipeAttribSGIX GLXEW_GET_FUN(__glewXQueryHyperpipeAttribSGIX)
#define glXQueryHyperpipeBestAttribSGIX GLXEW_GET_FUN(__glewXQueryHyperpipeBestAttribSGIX)
#define glXQueryHyperpipeConfigSGIX GLXEW_GET_FUN(__glewXQueryHyperpipeConfigSGIX)
#define glXQueryHyperpipeNetworkSGIX GLXEW_GET_FUN(__glewXQueryHyperpipeNetworkSGIX)

#define GLXEW_SGIX_hyperpipe GLXEW_GET_VAR(__GLXEW_SGIX_hyperpipe)

#endif /* GLX_SGIX_hyperpipe */

/* ---------------------------- GLX_SGIX_pbuffer --------------------------- */

#ifndef GLX_SGIX_pbuffer
#define GLX_SGIX_pbuffer 1

#define GLX_FRONT_LEFT_BUFFER_BIT_SGIX 0x00000001
#define GLX_FRONT_RIGHT_BUFFER_BIT_SGIX 0x00000002
#define GLX_PBUFFER_BIT_SGIX 0x00000004
#define GLX_BACK_LEFT_BUFFER_BIT_SGIX 0x00000004
#define GLX_BACK_RIGHT_BUFFER_BIT_SGIX 0x00000008
#define GLX_AUX_BUFFERS_BIT_SGIX 0x00000010
#define GLX_DEPTH_BUFFER_BIT_SGIX 0x00000020
#define GLX_STENCIL_BUFFER_BIT_SGIX 0x00000040
#define GLX_ACCUM_BUFFER_BIT_SGIX 0x00000080
#define GLX_SAMPLE_BUFFERS_BIT_SGIX 0x00000100
#define GLX_MAX_PBUFFER_WIDTH_SGIX 0x8016
#define GLX_MAX_PBUFFER_HEIGHT_SGIX 0x8017
#define GLX_MAX_PBUFFER_PIXELS_SGIX 0x8018
#define GLX_OPTIMAL_PBUFFER_WIDTH_SGIX 0x8019
#define GLX_OPTIMAL_PBUFFER_HEIGHT_SGIX 0x801A
#define GLX_PRESERVED_CONTENTS_SGIX 0x801B
#define GLX_LARGEST_PBUFFER_SGIX 0x801C
#define GLX_WIDTH_SGIX 0x801D
#define GLX_HEIGHT_SGIX 0x801E
#define GLX_EVENT_MASK_SGIX 0x801F
#define GLX_DAMAGED_SGIX 0x8020
#define GLX_SAVED_SGIX 0x8021
#define GLX_WINDOW_SGIX 0x8022
#define GLX_PBUFFER_SGIX 0x8023
#define GLX_BUFFER_CLOBBER_MASK_SGIX 0x08000000

typedef XID GLXPbufferSGIX;
typedef struct { int type; unsigned long serial; Bool send_event; Display *display; GLXDrawable drawable; int event_type; int draw_type; unsigned int mask; int x, y; int width, height; int count; } GLXBufferClobberEventSGIX;

typedef GLXPbuffer ( * PFNGLXCREATEGLXPBUFFERSGIXPROC) (Display* dpy, GLXFBConfig config, unsigned int width, unsigned int height, int *attrib_list);
typedef void ( * PFNGLXDESTROYGLXPBUFFERSGIXPROC) (Display* dpy, GLXPbuffer pbuf);
typedef void ( * PFNGLXGETSELECTEDEVENTSGIXPROC) (Display* dpy, GLXDrawable drawable, unsigned long *mask);
typedef void ( * PFNGLXQUERYGLXPBUFFERSGIXPROC) (Display* dpy, GLXPbuffer pbuf, int attribute, unsigned int *value);
typedef void ( * PFNGLXSELECTEVENTSGIXPROC) (Display* dpy, GLXDrawable drawable, unsigned long mask);

#define glXCreateGLXPbufferSGIX GLXEW_GET_FUN(__glewXCreateGLXPbufferSGIX)
#define glXDestroyGLXPbufferSGIX GLXEW_GET_FUN(__glewXDestroyGLXPbufferSGIX)
#define glXGetSelectedEventSGIX GLXEW_GET_FUN(__glewXGetSelectedEventSGIX)
#define glXQueryGLXPbufferSGIX GLXEW_GET_FUN(__glewXQueryGLXPbufferSGIX)
#define glXSelectEventSGIX GLXEW_GET_FUN(__glewXSelectEventSGIX)

#define GLXEW_SGIX_pbuffer GLXEW_GET_VAR(__GLXEW_SGIX_pbuffer)

#endif /* GLX_SGIX_pbuffer */

/* ------------------------- GLX_SGIX_swap_barrier ------------------------- */

#ifndef GLX_SGIX_swap_barrier
#define GLX_SGIX_swap_barrier 1

typedef void ( * PFNGLXBINDSWAPBARRIERSGIXPROC) (Display *dpy, GLXDrawable drawable, int barrier);
typedef Bool ( * PFNGLXQUERYMAXSWAPBARRIERSSGIXPROC) (Display *dpy, int screen, int *max);

#define glXBindSwapBarrierSGIX GLXEW_GET_FUN(__glewXBindSwapBarrierSGIX)
#define glXQueryMaxSwapBarriersSGIX GLXEW_GET_FUN(__glewXQueryMaxSwapBarriersSGIX)

#define GLXEW_SGIX_swap_barrier GLXEW_GET_VAR(__GLXEW_SGIX_swap_barrier)

#endif /* GLX_SGIX_swap_barrier */

/* -------------------------- GLX_SGIX_swap_group -------------------------- */

#ifndef GLX_SGIX_swap_group
#define GLX_SGIX_swap_group 1

typedef void ( * PFNGLXJOINSWAPGROUPSGIXPROC) (Display *dpy, GLXDrawable drawable, GLXDrawable member);

#define glXJoinSwapGroupSGIX GLXEW_GET_FUN(__glewXJoinSwapGroupSGIX)

#define GLXEW_SGIX_swap_group GLXEW_GET_VAR(__GLXEW_SGIX_swap_group)

#endif /* GLX_SGIX_swap_group */

/* ------------------------- GLX_SGIX_video_resize ------------------------- */

#ifndef GLX_SGIX_video_resize
#define GLX_SGIX_video_resize 1

#define GLX_SYNC_FRAME_SGIX 0x00000000
#define GLX_SYNC_SWAP_SGIX 0x00000001

typedef int ( * PFNGLXBINDCHANNELTOWINDOWSGIXPROC) (Display* display, int screen, int channel, Window window);
typedef int ( * PFNGLXCHANNELRECTSGIXPROC) (Display* display, int screen, int channel, int x, int y, int w, int h);
typedef int ( * PFNGLXCHANNELRECTSYNCSGIXPROC) (Display* display, int screen, int channel, GLenum synctype);
typedef int ( * PFNGLXQUERYCHANNELDELTASSGIXPROC) (Display* display, int screen, int channel, int *x, int *y, int *w, int *h);
typedef int ( * PFNGLXQUERYCHANNELRECTSGIXPROC) (Display* display, int screen, int channel, int *dx, int *dy, int *dw, int *dh);

#define glXBindChannelToWindowSGIX GLXEW_GET_FUN(__glewXBindChannelToWindowSGIX)
#define glXChannelRectSGIX GLXEW_GET_FUN(__glewXChannelRectSGIX)
#define glXChannelRectSyncSGIX GLXEW_GET_FUN(__glewXChannelRectSyncSGIX)
#define glXQueryChannelDeltasSGIX GLXEW_GET_FUN(__glewXQueryChannelDeltasSGIX)
#define glXQueryChannelRectSGIX GLXEW_GET_FUN(__glewXQueryChannelRectSGIX)

#define GLXEW_SGIX_video_resize GLXEW_GET_VAR(__GLXEW_SGIX_video_resize)

#endif /* GLX_SGIX_video_resize */

/* ---------------------- GLX_SGIX_visual_select_group --------------------- */

#ifndef GLX_SGIX_visual_select_group
#define GLX_SGIX_visual_select_group 1

#define GLX_VISUAL_SELECT_GROUP_SGIX 0x8028

#define GLXEW_SGIX_visual_select_group GLXEW_GET_VAR(__GLXEW_SGIX_visual_select_group)

#endif /* GLX_SGIX_visual_select_group */

/* ---------------------------- GLX_SGI_cushion ---------------------------- */

#ifndef GLX_SGI_cushion
#define GLX_SGI_cushion 1

typedef void ( * PFNGLXCUSHIONSGIPROC) (Display* dpy, Window window, float cushion);

#define glXCushionSGI GLXEW_GET_FUN(__glewXCushionSGI)

#define GLXEW_SGI_cushion GLXEW_GET_VAR(__GLXEW_SGI_cushion)

#endif /* GLX_SGI_cushion */

/* ----------------------- GLX_SGI_make_current_read ----------------------- */

#ifndef GLX_SGI_make_current_read
#define GLX_SGI_make_current_read 1

typedef GLXDrawable ( * PFNGLXGETCURRENTREADDRAWABLESGIPROC) (void);
typedef Bool ( * PFNGLXMAKECURRENTREADSGIPROC) (Display* dpy, GLXDrawable draw, GLXDrawable read, GLXContext ctx);

#define glXGetCurrentReadDrawableSGI GLXEW_GET_FUN(__glewXGetCurrentReadDrawableSGI)
#define glXMakeCurrentReadSGI GLXEW_GET_FUN(__glewXMakeCurrentReadSGI)

#define GLXEW_SGI_make_current_read GLXEW_GET_VAR(__GLXEW_SGI_make_current_read)

#endif /* GLX_SGI_make_current_read */

/* -------------------------- GLX_SGI_swap_control ------------------------- */

#ifndef GLX_SGI_swap_control
#define GLX_SGI_swap_control 1

typedef int ( * PFNGLXSWAPINTERVALSGIPROC) (int interval);

#define glXSwapIntervalSGI GLXEW_GET_FUN(__glewXSwapIntervalSGI)

#define GLXEW_SGI_swap_control GLXEW_GET_VAR(__GLXEW_SGI_swap_control)

#endif /* GLX_SGI_swap_control */

/* --------------------------- GLX_SGI_video_sync -------------------------- */

#ifndef GLX_SGI_video_sync
#define GLX_SGI_video_sync 1

typedef int ( * PFNGLXGETVIDEOSYNCSGIPROC) (unsigned int* count);
typedef int ( * PFNGLXWAITVIDEOSYNCSGIPROC) (int divisor, int remainder, unsigned int* count);

#define glXGetVideoSyncSGI GLXEW_GET_FUN(__glewXGetVideoSyncSGI)
#define glXWaitVideoSyncSGI GLXEW_GET_FUN(__glewXWaitVideoSyncSGI)

#define GLXEW_SGI_video_sync GLXEW_GET_VAR(__GLXEW_SGI_video_sync)

#endif /* GLX_SGI_video_sync */

/* --------------------- GLX_SUN_get_transparent_index --------------------- */

#ifndef GLX_SUN_get_transparent_index
#define GLX_SUN_get_transparent_index 1

typedef Status ( * PFNGLXGETTRANSPARENTINDEXSUNPROC) (Display* dpy, Window overlay, Window underlay, unsigned long *pTransparentIndex);

#define glXGetTransparentIndexSUN GLXEW_GET_FUN(__glewXGetTransparentIndexSUN)

#define GLXEW_SUN_get_transparent_index GLXEW_GET_VAR(__GLXEW_SUN_get_transparent_index)

#endif /* GLX_SUN_get_transparent_index */

/* -------------------------- GLX_SUN_video_resize ------------------------- */

#ifndef GLX_SUN_video_resize
#define GLX_SUN_video_resize 1

#define GLX_VIDEO_RESIZE_SUN 0x8171
#define GL_VIDEO_RESIZE_COMPENSATION_SUN 0x85CD

typedef int ( * PFNGLXGETVIDEORESIZESUNPROC) (Display* display, GLXDrawable window, float* factor);
typedef int ( * PFNGLXVIDEORESIZESUNPROC) (Display* display, GLXDrawable window, float factor);

#define glXGetVideoResizeSUN GLXEW_GET_FUN(__glewXGetVideoResizeSUN)
#define glXVideoResizeSUN GLXEW_GET_FUN(__glewXVideoResizeSUN)

#define GLXEW_SUN_video_resize GLXEW_GET_VAR(__GLXEW_SUN_video_resize)

#endif /* GLX_SUN_video_resize */

/* ------------------------------------------------------------------------- */

#ifdef GLEW_MX
#define GLXEW_EXPORT
#else
#define GLXEW_EXPORT extern
#endif /* GLEW_MX */

extern PFNGLXGETCURRENTDISPLAYPROC __glewXGetCurrentDisplay;

extern PFNGLXCHOOSEFBCONFIGPROC __glewXChooseFBConfig;
extern PFNGLXCREATENEWCONTEXTPROC __glewXCreateNewContext;
extern PFNGLXCREATEPBUFFERPROC __glewXCreatePbuffer;
extern PFNGLXCREATEPIXMAPPROC __glewXCreatePixmap;
extern PFNGLXCREATEWINDOWPROC __glewXCreateWindow;
extern PFNGLXDESTROYPBUFFERPROC __glewXDestroyPbuffer;
extern PFNGLXDESTROYPIXMAPPROC __glewXDestroyPixmap;
extern PFNGLXDESTROYWINDOWPROC __glewXDestroyWindow;
extern PFNGLXGETCURRENTREADDRAWABLEPROC __glewXGetCurrentReadDrawable;
extern PFNGLXGETFBCONFIGATTRIBPROC __glewXGetFBConfigAttrib;
extern PFNGLXGETFBCONFIGSPROC __glewXGetFBConfigs;
extern PFNGLXGETSELECTEDEVENTPROC __glewXGetSelectedEvent;
extern PFNGLXGETVISUALFROMFBCONFIGPROC __glewXGetVisualFromFBConfig;
extern PFNGLXMAKECONTEXTCURRENTPROC __glewXMakeContextCurrent;
extern PFNGLXQUERYCONTEXTPROC __glewXQueryContext;
extern PFNGLXQUERYDRAWABLEPROC __glewXQueryDrawable;
extern PFNGLXSELECTEVENTPROC __glewXSelectEvent;

extern PFNGLXCREATECONTEXTATTRIBSARBPROC __glewXCreateContextAttribsARB;

extern PFNGLXBINDTEXIMAGEATIPROC __glewXBindTexImageATI;
extern PFNGLXDRAWABLEATTRIBATIPROC __glewXDrawableAttribATI;
extern PFNGLXRELEASETEXIMAGEATIPROC __glewXReleaseTexImageATI;

extern PFNGLXFREECONTEXTEXTPROC __glewXFreeContextEXT;
extern PFNGLXGETCONTEXTIDEXTPROC __glewXGetContextIDEXT;
extern PFNGLXIMPORTCONTEXTEXTPROC __glewXImportContextEXT;
extern PFNGLXQUERYCONTEXTINFOEXTPROC __glewXQueryContextInfoEXT;

extern PFNGLXSWAPINTERVALEXTPROC __glewXSwapIntervalEXT;

extern PFNGLXBINDTEXIMAGEEXTPROC __glewXBindTexImageEXT;
extern PFNGLXRELEASETEXIMAGEEXTPROC __glewXReleaseTexImageEXT;

extern PFNGLXGETAGPOFFSETMESAPROC __glewXGetAGPOffsetMESA;

extern PFNGLXCOPYSUBBUFFERMESAPROC __glewXCopySubBufferMESA;

extern PFNGLXCREATEGLXPIXMAPMESAPROC __glewXCreateGLXPixmapMESA;

extern PFNGLXRELEASEBUFFERSMESAPROC __glewXReleaseBuffersMESA;

extern PFNGLXSET3DFXMODEMESAPROC __glewXSet3DfxModeMESA;

extern PFNGLXCOPYIMAGESUBDATANVPROC __glewXCopyImageSubDataNV;

extern PFNGLXBINDVIDEODEVICENVPROC __glewXBindVideoDeviceNV;
extern PFNGLXENUMERATEVIDEODEVICESNVPROC __glewXEnumerateVideoDevicesNV;

extern PFNGLXBINDSWAPBARRIERNVPROC __glewXBindSwapBarrierNV;
extern PFNGLXJOINSWAPGROUPNVPROC __glewXJoinSwapGroupNV;
extern PFNGLXQUERYFRAMECOUNTNVPROC __glewXQueryFrameCountNV;
extern PFNGLXQUERYMAXSWAPGROUPSNVPROC __glewXQueryMaxSwapGroupsNV;
extern PFNGLXQUERYSWAPGROUPNVPROC __glewXQuerySwapGroupNV;
extern PFNGLXRESETFRAMECOUNTNVPROC __glewXResetFrameCountNV;

extern PFNGLXALLOCATEMEMORYNVPROC __glewXAllocateMemoryNV;
extern PFNGLXFREEMEMORYNVPROC __glewXFreeMemoryNV;

extern PFNGLXBINDVIDEOIMAGENVPROC __glewXBindVideoImageNV;
extern PFNGLXGETVIDEODEVICENVPROC __glewXGetVideoDeviceNV;
extern PFNGLXGETVIDEOINFONVPROC __glewXGetVideoInfoNV;
extern PFNGLXRELEASEVIDEODEVICENVPROC __glewXReleaseVideoDeviceNV;
extern PFNGLXRELEASEVIDEOIMAGENVPROC __glewXReleaseVideoImageNV;
extern PFNGLXSENDPBUFFERTOVIDEONVPROC __glewXSendPbufferToVideoNV;

#ifdef GLX_OML_sync_control
extern PFNGLXGETMSCRATEOMLPROC __glewXGetMscRateOML;
extern PFNGLXGETSYNCVALUESOMLPROC __glewXGetSyncValuesOML;
extern PFNGLXSWAPBUFFERSMSCOMLPROC __glewXSwapBuffersMscOML;
extern PFNGLXWAITFORMSCOMLPROC __glewXWaitForMscOML;
extern PFNGLXWAITFORSBCOMLPROC __glewXWaitForSbcOML;
#endif

extern PFNGLXCHOOSEFBCONFIGSGIXPROC __glewXChooseFBConfigSGIX;
extern PFNGLXCREATECONTEXTWITHCONFIGSGIXPROC __glewXCreateContextWithConfigSGIX;
extern PFNGLXCREATEGLXPIXMAPWITHCONFIGSGIXPROC __glewXCreateGLXPixmapWithConfigSGIX;
extern PFNGLXGETFBCONFIGATTRIBSGIXPROC __glewXGetFBConfigAttribSGIX;
extern PFNGLXGETFBCONFIGFROMVISUALSGIXPROC __glewXGetFBConfigFromVisualSGIX;
extern PFNGLXGETVISUALFROMFBCONFIGSGIXPROC __glewXGetVisualFromFBConfigSGIX;

extern PFNGLXBINDHYPERPIPESGIXPROC __glewXBindHyperpipeSGIX;
extern PFNGLXDESTROYHYPERPIPECONFIGSGIXPROC __glewXDestroyHyperpipeConfigSGIX;
extern PFNGLXHYPERPIPEATTRIBSGIXPROC __glewXHyperpipeAttribSGIX;
extern PFNGLXHYPERPIPECONFIGSGIXPROC __glewXHyperpipeConfigSGIX;
extern PFNGLXQUERYHYPERPIPEATTRIBSGIXPROC __glewXQueryHyperpipeAttribSGIX;
extern PFNGLXQUERYHYPERPIPEBESTATTRIBSGIXPROC __glewXQueryHyperpipeBestAttribSGIX;
extern PFNGLXQUERYHYPERPIPECONFIGSGIXPROC __glewXQueryHyperpipeConfigSGIX;
extern PFNGLXQUERYHYPERPIPENETWORKSGIXPROC __glewXQueryHyperpipeNetworkSGIX;

extern PFNGLXCREATEGLXPBUFFERSGIXPROC __glewXCreateGLXPbufferSGIX;
extern PFNGLXDESTROYGLXPBUFFERSGIXPROC __glewXDestroyGLXPbufferSGIX;
extern PFNGLXGETSELECTEDEVENTSGIXPROC __glewXGetSelectedEventSGIX;
extern PFNGLXQUERYGLXPBUFFERSGIXPROC __glewXQueryGLXPbufferSGIX;
extern PFNGLXSELECTEVENTSGIXPROC __glewXSelectEventSGIX;

extern PFNGLXBINDSWAPBARRIERSGIXPROC __glewXBindSwapBarrierSGIX;
extern PFNGLXQUERYMAXSWAPBARRIERSSGIXPROC __glewXQueryMaxSwapBarriersSGIX;

extern PFNGLXJOINSWAPGROUPSGIXPROC __glewXJoinSwapGroupSGIX;

extern PFNGLXBINDCHANNELTOWINDOWSGIXPROC __glewXBindChannelToWindowSGIX;
extern PFNGLXCHANNELRECTSGIXPROC __glewXChannelRectSGIX;
extern PFNGLXCHANNELRECTSYNCSGIXPROC __glewXChannelRectSyncSGIX;
extern PFNGLXQUERYCHANNELDELTASSGIXPROC __glewXQueryChannelDeltasSGIX;
extern PFNGLXQUERYCHANNELRECTSGIXPROC __glewXQueryChannelRectSGIX;

extern PFNGLXCUSHIONSGIPROC __glewXCushionSGI;

extern PFNGLXGETCURRENTREADDRAWABLESGIPROC __glewXGetCurrentReadDrawableSGI;
extern PFNGLXMAKECURRENTREADSGIPROC __glewXMakeCurrentReadSGI;

extern PFNGLXSWAPINTERVALSGIPROC __glewXSwapIntervalSGI;

extern PFNGLXGETVIDEOSYNCSGIPROC __glewXGetVideoSyncSGI;
extern PFNGLXWAITVIDEOSYNCSGIPROC __glewXWaitVideoSyncSGI;

extern PFNGLXGETTRANSPARENTINDEXSUNPROC __glewXGetTransparentIndexSUN;

extern PFNGLXGETVIDEORESIZESUNPROC __glewXGetVideoResizeSUN;
extern PFNGLXVIDEORESIZESUNPROC __glewXVideoResizeSUN;

#if defined(GLEW_MX)
struct GLXEWContextStruct
{
#endif /* GLEW_MX */

GLXEW_EXPORT GLboolean __GLXEW_VERSION_1_0;
GLXEW_EXPORT GLboolean __GLXEW_VERSION_1_1;
GLXEW_EXPORT GLboolean __GLXEW_VERSION_1_2;
GLXEW_EXPORT GLboolean __GLXEW_VERSION_1_3;
GLXEW_EXPORT GLboolean __GLXEW_VERSION_1_4;
GLXEW_EXPORT GLboolean __GLXEW_3DFX_multisample;
GLXEW_EXPORT GLboolean __GLXEW_AMD_gpu_association;
GLXEW_EXPORT GLboolean __GLXEW_ARB_create_context;
GLXEW_EXPORT GLboolean __GLXEW_ARB_create_context_profile;
GLXEW_EXPORT GLboolean __GLXEW_ARB_fbconfig_float;
GLXEW_EXPORT GLboolean __GLXEW_ARB_framebuffer_sRGB;
GLXEW_EXPORT GLboolean __GLXEW_ARB_get_proc_address;
GLXEW_EXPORT GLboolean __GLXEW_ARB_multisample;
GLXEW_EXPORT GLboolean __GLXEW_ARB_vertex_buffer_object;
GLXEW_EXPORT GLboolean __GLXEW_ATI_pixel_format_float;
GLXEW_EXPORT GLboolean __GLXEW_ATI_render_texture;
GLXEW_EXPORT GLboolean __GLXEW_EXT_fbconfig_packed_float;
GLXEW_EXPORT GLboolean __GLXEW_EXT_framebuffer_sRGB;
GLXEW_EXPORT GLboolean __GLXEW_EXT_import_context;
GLXEW_EXPORT GLboolean __GLXEW_EXT_scene_marker;
GLXEW_EXPORT GLboolean __GLXEW_EXT_swap_control;
GLXEW_EXPORT GLboolean __GLXEW_EXT_texture_from_pixmap;
GLXEW_EXPORT GLboolean __GLXEW_EXT_visual_info;
GLXEW_EXPORT GLboolean __GLXEW_EXT_visual_rating;
GLXEW_EXPORT GLboolean __GLXEW_INTEL_swap_event;
GLXEW_EXPORT GLboolean __GLXEW_MESA_agp_offset;
GLXEW_EXPORT GLboolean __GLXEW_MESA_copy_sub_buffer;
GLXEW_EXPORT GLboolean __GLXEW_MESA_pixmap_colormap;
GLXEW_EXPORT GLboolean __GLXEW_MESA_release_buffers;
GLXEW_EXPORT GLboolean __GLXEW_MESA_set_3dfx_mode;
GLXEW_EXPORT GLboolean __GLXEW_NV_copy_image;
GLXEW_EXPORT GLboolean __GLXEW_NV_float_buffer;
GLXEW_EXPORT GLboolean __GLXEW_NV_multisample_coverage;
GLXEW_EXPORT GLboolean __GLXEW_NV_present_video;
GLXEW_EXPORT GLboolean __GLXEW_NV_swap_group;
GLXEW_EXPORT GLboolean __GLXEW_NV_vertex_array_range;
GLXEW_EXPORT GLboolean __GLXEW_NV_video_output;
GLXEW_EXPORT GLboolean __GLXEW_OML_swap_method;
GLXEW_EXPORT GLboolean __GLXEW_OML_sync_control;
GLXEW_EXPORT GLboolean __GLXEW_SGIS_blended_overlay;
GLXEW_EXPORT GLboolean __GLXEW_SGIS_color_range;
GLXEW_EXPORT GLboolean __GLXEW_SGIS_multisample;
GLXEW_EXPORT GLboolean __GLXEW_SGIS_shared_multisample;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_fbconfig;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_hyperpipe;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_pbuffer;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_swap_barrier;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_swap_group;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_video_resize;
GLXEW_EXPORT GLboolean __GLXEW_SGIX_visual_select_group;
GLXEW_EXPORT GLboolean __GLXEW_SGI_cushion;
GLXEW_EXPORT GLboolean __GLXEW_SGI_make_current_read;
GLXEW_EXPORT GLboolean __GLXEW_SGI_swap_control;
GLXEW_EXPORT GLboolean __GLXEW_SGI_video_sync;
GLXEW_EXPORT GLboolean __GLXEW_SUN_get_transparent_index;
GLXEW_EXPORT GLboolean __GLXEW_SUN_video_resize;

#ifdef GLEW_MX
}; /* GLXEWContextStruct */
#endif /* GLEW_MX */

/* ------------------------------------------------------------------------ */

#ifdef GLEW_MX

typedef struct GLXEWContextStruct GLXEWContext;
extern GLenum glxewContextInit (GLXEWContext* ctx);
extern GLboolean glxewContextIsSupported (GLXEWContext* ctx, const char* name);

#define glxewInit() glxewContextInit(glxewGetContext())
#define glxewIsSupported(x) glxewContextIsSupported(glxewGetContext(), x)

#define GLXEW_GET_VAR(x) (*(const GLboolean*)&(glxewGetContext()->x))
#define GLXEW_GET_FUN(x) x

#else /* GLEW_MX */

#define GLXEW_GET_VAR(x) (*(const GLboolean*)&x)
#define GLXEW_GET_FUN(x) x

extern GLboolean glxewIsSupported (const char* name);

#endif /* GLEW_MX */

extern GLboolean glxewGetExtension (const char* name);

#ifdef __cplusplus
}
#endif

#endif /* __glxew_h__ */


```

## Source file: include/wglew.h

{
  "path": "include/wglew.h",
  "blob_sha": "e0ff5a77760115d0c5c141978f1baf0d97da4b39",
  "sha256": "edb836b6f2ff5f576247d13c0e2ad5e39405420fcaba2cebdaeaab9061d4f542",
  "lines": 1260,
  "bytes": 54222,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 14,
    "face": 1
  },
  "url": "https://github.com/sp4cerat/Skinned-Skeletal-Animation-in-OpenGL/blob/2436debd125b6ef946c99b8ca9f7f5b846aa8e60/include/wglew.h",
  "function_signature_candidates": []
}

```text

/*
** The OpenGL Extension Wrangler Library
** Copyright (C) 2002-2008, Milan Ikits <milan ikits[]ieee org>
** Copyright (C) 2002-2008, Marcelo E. Magallon <mmagallo[]debian org>
** Copyright (C) 2002, Lev Povalahev
** All rights reserved.
** 
** Redistribution and use in source and binary forms, with or without 
** modification, are permitted provided that the following conditions are met:
** 
** * Redistributions of source code must retain the above copyright notice, 
**   this list of conditions and the following disclaimer.
** * Redistributions in binary form must reproduce the above copyright notice, 
**   this list of conditions and the following disclaimer in the documentation 
**   and/or other materials provided with the distribution.
** * The name of the author may be used to endorse or promote products 
**   derived from this software without specific prior written permission.
**
** THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" 
** AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE 
** IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
** ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE 
** LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR 
** CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF 
** SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
** INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
** CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
** ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF
** THE POSSIBILITY OF SUCH DAMAGE.
*/

/*
** Copyright (c) 2007 The Khronos Group Inc.
** 
** Permission is hereby granted, free of charge, to any person obtaining a
** copy of this software and/or associated documentation files (the
** "Materials"), to deal in the Materials without restriction, including
** without limitation the rights to use, copy, modify, merge, publish,
** distribute, sublicense, and/or sell copies of the Materials, and to
** permit persons to whom the Materials are furnished to do so, subject to
** the following conditions:
** 
** The above copyright notice and this permission notice shall be included
** in all copies or substantial portions of the Materials.
** 
** THE MATERIALS ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
** EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
** MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
** IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
** CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
** TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
** MATERIALS OR THE USE OR OTHER DEALINGS IN THE MATERIALS.
*/

#ifndef __wglew_h__
#define __wglew_h__
#define __WGLEW_H__

#ifdef __wglext_h_
#error wglext.h included before wglew.h
#endif

#define __wglext_h_

#if !defined(WINAPI)
#  ifndef WIN32_LEAN_AND_MEAN
#    define WIN32_LEAN_AND_MEAN 1
#  endif
#include <windows.h>
#  undef WIN32_LEAN_AND_MEAN
#endif

/*
 * GLEW_STATIC needs to be set when using the static version.
 * GLEW_BUILD is set when building the DLL version.
 */
#ifdef GLEW_STATIC
#  define GLEWAPI extern
#else
#  ifdef GLEW_BUILD
#    define GLEWAPI extern __declspec(dllexport)
#  else
#    define GLEWAPI extern __declspec(dllimport)
#  endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* -------------------------- WGL_3DFX_multisample ------------------------- */

#ifndef WGL_3DFX_multisample
#define WGL_3DFX_multisample 1

#define WGL_SAMPLE_BUFFERS_3DFX 0x2060
#define WGL_SAMPLES_3DFX 0x2061

#define WGLEW_3DFX_multisample WGLEW_GET_VAR(__WGLEW_3DFX_multisample)

#endif /* WGL_3DFX_multisample */

/* ------------------------- WGL_3DL_stereo_control ------------------------ */

#ifndef WGL_3DL_stereo_control
#define WGL_3DL_stereo_control 1

#define WGL_STEREO_EMITTER_ENABLE_3DL 0x2055
#define WGL_STEREO_EMITTER_DISABLE_3DL 0x2056
#define WGL_STEREO_POLARITY_NORMAL_3DL 0x2057
#define WGL_STEREO_POLARITY_INVERT_3DL 0x2058

typedef BOOL (WINAPI * PFNWGLSETSTEREOEMITTERSTATE3DLPROC) (HDC hDC, UINT uState);

#define wglSetStereoEmitterState3DL WGLEW_GET_FUN(__wglewSetStereoEmitterState3DL)

#define WGLEW_3DL_stereo_control WGLEW_GET_VAR(__WGLEW_3DL_stereo_control)

#endif /* WGL_3DL_stereo_control */

/* ------------------------ WGL_AMD_gpu_association ------------------------ */

#ifndef WGL_AMD_gpu_association
#define WGL_AMD_gpu_association 1

#define WGL_GPU_VENDOR_AMD 0x1F00
#define WGL_GPU_RENDERER_STRING_AMD 0x1F01
#define WGL_GPU_OPENGL_VERSION_STRING_AMD 0x1F02
#define WGL_GPU_FASTEST_TARGET_GPUS_AMD 0x21A2
#define WGL_GPU_RAM_AMD 0x21A3
#define WGL_GPU_CLOCK_AMD 0x21A4
#define WGL_GPU_NUM_PIPES_AMD 0x21A5
#define WGL_GPU_NUM_SIMD_AMD 0x21A6
#define WGL_GPU_NUM_RB_AMD 0x21A7
#define WGL_GPU_NUM_SPI_AMD 0x21A8

typedef VOID (WINAPI * PFNWGLBLITCONTEXTFRAMEBUFFERAMDPROC) (HGLRC dstCtx, GLint srcX0, GLint srcY0, GLint srcX1, GLint srcY1, GLint dstX0, GLint dstY0, GLint dstX1, GLint dstY1, GLbitfield mask, GLenum filter);
typedef HGLRC (WINAPI * PFNWGLCREATEASSOCIATEDCONTEXTAMDPROC) (UINT id);
typedef HGLRC (WINAPI * PFNWGLCREATEASSOCIATEDCONTEXTATTRIBSAMDPROC) (UINT id, HGLRC hShareContext, const int* attribList);
typedef BOOL (WINAPI * PFNWGLDELETEASSOCIATEDCONTEXTAMDPROC) (HGLRC hglrc);
typedef UINT (WINAPI * PFNWGLGETCONTEXTGPUIDAMDPROC) (HGLRC hglrc);
typedef HGLRC (WINAPI * PFNWGLGETCURRENTASSOCIATEDCONTEXTAMDPROC) (void);
typedef UINT (WINAPI * PFNWGLGETGPUIDSAMDPROC) (UINT maxCount, UINT* ids);
typedef INT (WINAPI * PFNWGLGETGPUINFOAMDPROC) (UINT id, INT property, GLenum dataType, UINT size, void* data);
typedef BOOL (WINAPI * PFNWGLMAKEASSOCIATEDCONTEXTCURRENTAMDPROC) (HGLRC hglrc);

#define wglBlitContextFramebufferAMD WGLEW_GET_FUN(__wglewBlitContextFramebufferAMD)
#define wglCreateAssociatedContextAMD WGLEW_GET_FUN(__wglewCreateAssociatedContextAMD)
#define wglCreateAssociatedContextAttribsAMD WGLEW_GET_FUN(__wglewCreateAssociatedContextAttribsAMD)
#define wglDeleteAssociatedContextAMD WGLEW_GET_FUN(__wglewDeleteAssociatedContextAMD)
#define wglGetContextGPUIDAMD WGLEW_GET_FUN(__wglewGetContextGPUIDAMD)
#define wglGetCurrentAssociatedContextAMD WGLEW_GET_FUN(__wglewGetCurrentAssociatedContextAMD)
#define wglGetGPUIDsAMD WGLEW_GET_FUN(__wglewGetGPUIDsAMD)
#define wglGetGPUInfoAMD WGLEW_GET_FUN(__wglewGetGPUInfoAMD)
#define wglMakeAssociatedContextCurrentAMD WGLEW_GET_FUN(__wglewMakeAssociatedContextCurrentAMD)

#define WGLEW_AMD_gpu_association WGLEW_GET_VAR(__WGLEW_AMD_gpu_association)

#endif /* WGL_AMD_gpu_association */

/* ------------------------- WGL_ARB_buffer_region ------------------------- */

#ifndef WGL_ARB_buffer_region
#define WGL_ARB_buffer_region 1

#define WGL_FRONT_COLOR_BUFFER_BIT_ARB 0x00000001
#define WGL_BACK_COLOR_BUFFER_BIT_ARB 0x00000002
#define WGL_DEPTH_BUFFER_BIT_ARB 0x00000004
#define WGL_STENCIL_BUFFER_BIT_ARB 0x00000008

typedef HANDLE (WINAPI * PFNWGLCREATEBUFFERREGIONARBPROC) (HDC hDC, int iLayerPlane, UINT uType);
typedef VOID (WINAPI * PFNWGLDELETEBUFFERREGIONARBPROC) (HANDLE hRegion);
typedef BOOL (WINAPI * PFNWGLRESTOREBUFFERREGIONARBPROC) (HANDLE hRegion, int x, int y, int width, int height, int xSrc, int ySrc);
typedef BOOL (WINAPI * PFNWGLSAVEBUFFERREGIONARBPROC) (HANDLE hRegion, int x, int y, int width, int height);

#define wglCreateBufferRegionARB WGLEW_GET_FUN(__wglewCreateBufferRegionARB)
#define wglDeleteBufferRegionARB WGLEW_GET_FUN(__wglewDeleteBufferRegionARB)
#define wglRestoreBufferRegionARB WGLEW_GET_FUN(__wglewRestoreBufferRegionARB)
#define wglSaveBufferRegionARB WGLEW_GET_FUN(__wglewSaveBufferRegionARB)

#define WGLEW_ARB_buffer_region WGLEW_GET_VAR(__WGLEW_ARB_buffer_region)

#endif /* WGL_ARB_buffer_region */

/* ------------------------- WGL_ARB_create_context ------------------------ */

#ifndef WGL_ARB_create_context
#define WGL_ARB_create_context 1

#define WGL_CONTEXT_DEBUG_BIT_ARB 0x0001
#define WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB 0x0002
#define WGL_CONTEXT_MAJOR_VERSION_ARB 0x2091
#define WGL_CONTEXT_MINOR_VERSION_ARB 0x2092
#define WGL_CONTEXT_LAYER_PLANE_ARB 0x2093
#define WGL_CONTEXT_FLAGS_ARB 0x2094

typedef HGLRC (WINAPI * PFNWGLCREATECONTEXTATTRIBSARBPROC) (HDC hDC, HGLRC hShareContext, const int* attribList);

#define wglCreateContextAttribsARB WGLEW_GET_FUN(__wglewCreateContextAttribsARB)

#define WGLEW_ARB_create_context WGLEW_GET_VAR(__WGLEW_ARB_create_context)

#endif /* WGL_ARB_create_context */

/* --------------------- WGL_ARB_create_context_profile -------------------- */

#ifndef WGL_ARB_create_context_profile
#define WGL_ARB_create_context_profile 1

#define WGL_CONTEXT_CORE_PROFILE_BIT_ARB 0x00000001
#define WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB 0x00000002
#define WGL_CONTEXT_PROFILE_MASK_ARB 0x9126

#define WGLEW_ARB_create_context_profile WGLEW_GET_VAR(__WGLEW_ARB_create_context_profile)

#endif /* WGL_ARB_create_context_profile */

/* ----------------------- WGL_ARB_extensions_string ----------------------- */

#ifndef WGL_ARB_extensions_string
#define WGL_ARB_extensions_string 1

typedef const char* (WINAPI * PFNWGLGETEXTENSIONSSTRINGARBPROC) (HDC hdc);

#define wglGetExtensionsStringARB WGLEW_GET_FUN(__wglewGetExtensionsStringARB)

#define WGLEW_ARB_extensions_string WGLEW_GET_VAR(__WGLEW_ARB_extensions_string)

#endif /* WGL_ARB_extensions_string */

/* ------------------------ WGL_ARB_framebuffer_sRGB ----------------------- */

#ifndef WGL_ARB_framebuffer_sRGB
#define WGL_ARB_framebuffer_sRGB 1

#define WGL_FRAMEBUFFER_SRGB_CAPABLE_ARB 0x20A9

#define WGLEW_ARB_framebuffer_sRGB WGLEW_GET_VAR(__WGLEW_ARB_framebuffer_sRGB)

#endif /* WGL_ARB_framebuffer_sRGB */

/* ----------------------- WGL_ARB_make_current_read ----------------------- */

#ifndef WGL_ARB_make_current_read
#define WGL_ARB_make_current_read 1

#define ERROR_INVALID_PIXEL_TYPE_ARB 0x2043
#define ERROR_INCOMPATIBLE_DEVICE_CONTEXTS_ARB 0x2054

typedef HDC (WINAPI * PFNWGLGETCURRENTREADDCARBPROC) (VOID);
typedef BOOL (WINAPI * PFNWGLMAKECONTEXTCURRENTARBPROC) (HDC hDrawDC, HDC hReadDC, HGLRC hglrc);

#define wglGetCurrentReadDCARB WGLEW_GET_FUN(__wglewGetCurrentReadDCARB)
#define wglMakeContextCurrentARB WGLEW_GET_FUN(__wglewMakeContextCurrentARB)

#define WGLEW_ARB_make_current_read WGLEW_GET_VAR(__WGLEW_ARB_make_current_read)

#endif /* WGL_ARB_make_current_read */

/* -------------------------- WGL_ARB_multisample -------------------------- */

#ifndef WGL_ARB_multisample
#define WGL_ARB_multisample 1

#define WGL_SAMPLE_BUFFERS_ARB 0x2041
#define WGL_SAMPLES_ARB 0x2042

#define WGLEW_ARB_multisample WGLEW_GET_VAR(__WGLEW_ARB_multisample)

#endif /* WGL_ARB_multisample */

/* ---------------------------- WGL_ARB_pbuffer ---------------------------- */

#ifndef WGL_ARB_pbuffer
#define WGL_ARB_pbuffer 1

#define WGL_DRAW_TO_PBUFFER_ARB 0x202D
#define WGL_MAX_PBUFFER_PIXELS_ARB 0x202E
#define WGL_MAX_PBUFFER_WIDTH_ARB 0x202F
#define WGL_MAX_PBUFFER_HEIGHT_ARB 0x2030
#define WGL_PBUFFER_LARGEST_ARB 0x2033
#define WGL_PBUFFER_WIDTH_ARB 0x2034
#define WGL_PBUFFER_HEIGHT_ARB 0x2035
#define WGL_PBUFFER_LOST_ARB 0x2036

DECLARE_HANDLE(HPBUFFERARB);

typedef HPBUFFERARB (WINAPI * PFNWGLCREATEPBUFFERARBPROC) (HDC hDC, int iPixelFormat, int iWidth, int iHeight, const int* piAttribList);
typedef BOOL (WINAPI * PFNWGLDESTROYPBUFFERARBPROC) (HPBUFFERARB hPbuffer);
typedef HDC (WINAPI * PFNWGLGETPBUFFERDCARBPROC) (HPBUFFERARB hPbuffer);
typedef BOOL (WINAPI * PFNWGLQUERYPBUFFERARBPROC) (HPBUFFERARB hPbuffer, int iAttribute, int* piValue);
typedef int (WINAPI * PFNWGLRELEASEPBUFFERDCARBPROC) (HPBUFFERARB hPbuffer, HDC hDC);

#define wglCreatePbufferARB WGLEW_GET_FUN(__wglewCreatePbufferARB)
#define wglDestroyPbufferARB WGLEW_GET_FUN(__wglewDestroyPbufferARB)
#define wglGetPbufferDCARB WGLEW_GET_FUN(__wglewGetPbufferDCARB)
#define wglQueryPbufferARB WGLEW_GET_FUN(__wglewQueryPbufferARB)
#define wglReleasePbufferDCARB WGLEW_GET_FUN(__wglewReleasePbufferDCARB)

#define WGLEW_ARB_pbuffer WGLEW_GET_VAR(__WGLEW_ARB_pbuffer)

#endif /* WGL_ARB_pbuffer */

/* -------------------------- WGL_ARB_pixel_format ------------------------- */

#ifndef WGL_ARB_pixel_format
#define WGL_ARB_pixel_format 1

#define WGL_NUMBER_PIXEL_FORMATS_ARB 0x2000
#define WGL_DRAW_TO_WINDOW_ARB 0x2001
#define WGL_DRAW_TO_BITMAP_ARB 0x2002
#define WGL_ACCELERATION_ARB 0x2003
#define WGL_NEED_PALETTE_ARB 0x2004
#define WGL_NEED_SYSTEM_PALETTE_ARB 0x2005
#define WGL_SWAP_LAYER_BUFFERS_ARB 0x2006
#define WGL_SWAP_METHOD_ARB 0x2007
#define WGL_NUMBER_OVERLAYS_ARB 0x2008
#define WGL_NUMBER_UNDERLAYS_ARB 0x2009
#define WGL_TRANSPARENT_ARB 0x200A
#define WGL_SHARE_DEPTH_ARB 0x200C
#define WGL_SHARE_STENCIL_ARB 0x200D
#define WGL_SHARE_ACCUM_ARB 0x200E
#define WGL_SUPPORT_GDI_ARB 0x200F
#define WGL_SUPPORT_OPENGL_ARB 0x2010
#define WGL_DOUBLE_BUFFER_ARB 0x2011
#define WGL_STEREO_ARB 0x2012
#define WGL_PIXEL_TYPE_ARB 0x2013
#define WGL_COLOR_BITS_ARB 0x2014
#define WGL_RED_BITS_ARB 0x2015
#define WGL_RED_SHIFT_ARB 0x2016
#define WGL_GREEN_BITS_ARB 0x2017
#define WGL_GREEN_SHIFT_ARB 0x2018
#define WGL_BLUE_BITS_ARB 0x2019
#define WGL_BLUE_SHIFT_ARB 0x201A
#define WGL_ALPHA_BITS_ARB 0x201B
#define WGL_ALPHA_SHIFT_ARB 0x201C
#define WGL_ACCUM_BITS_ARB 0x201D
#define WGL_ACCUM_RED_BITS_ARB 0x201E
#define WGL_ACCUM_GREEN_BITS_ARB 0x201F
#define WGL_ACCUM_BLUE_BITS_ARB 0x2020
#define WGL_ACCUM_ALPHA_BITS_ARB 0x2021
#define WGL_DEPTH_BITS_ARB 0x2022
#define WGL_STENCIL_BITS_ARB 0x2023
#define WGL_AUX_BUFFERS_ARB 0x2024
#define WGL_NO_ACCELERATION_ARB 0x2025
#define WGL_GENERIC_ACCELERATION_ARB 0x2026
#define WGL_FULL_ACCELERATION_ARB 0x2027
#define WGL_SWAP_EXCHANGE_ARB 0x2028
#define WGL_SWAP_COPY_ARB 0x2029
#define WGL_SWAP_UNDEFINED_ARB 0x202A
#define WGL_TYPE_RGBA_ARB 0x202B
#define WGL_TYPE_COLORINDEX_ARB 0x202C
#define WGL_TRANSPARENT_RED_VALUE_ARB 0x2037
#define WGL_TRANSPARENT_GREEN_VALUE_ARB 0x2038
#define WGL_TRANSPARENT_BLUE_VALUE_ARB 0x2039
#define WGL_TRANSPARENT_ALPHA_VALUE_ARB 0x203A
#define WGL_TRANSPARENT_INDEX_VALUE_ARB 0x203B

typedef BOOL (WINAPI * PFNWGLCHOOSEPIXELFORMATARBPROC) (HDC hdc, const int* piAttribIList, const FLOAT *pfAttribFList, UINT nMaxFormats, int *piFormats, UINT *nNumFormats);
typedef BOOL (WINAPI * PFNWGLGETPIXELFORMATATTRIBFVARBPROC) (HDC hdc, int iPixelFormat, int iLayerPlane, UINT nAttributes, const int* piAttributes, FLOAT *pfValues);
typedef BOOL (WINAPI * PFNWGLGETPIXELFORMATATTRIBIVARBPROC) (HDC hdc, int iPixelFormat, int iLayerPlane, UINT nAttributes, const int* piAttributes, int *piValues);

#define wglChoosePixelFormatARB WGLEW_GET_FUN(__wglewChoosePixelFormatARB)
#define wglGetPixelFormatAttribfvARB WGLEW_GET_FUN(__wglewGetPixelFormatAttribfvARB)
#define wglGetPixelFormatAttribivARB WGLEW_GET_FUN(__wglewGetPixelFormatAttribivARB)

#define WGLEW_ARB_pixel_format WGLEW_GET_VAR(__WGLEW_ARB_pixel_format)

#endif /* WGL_ARB_pixel_format */

/* ----------------------- WGL_ARB_pixel_format_float ---------------------- */

#ifndef WGL_ARB_pixel_format_float
#define WGL_ARB_pixel_format_float 1

#define WGL_TYPE_RGBA_FLOAT_ARB 0x21A0

#define WGLEW_ARB_pixel_format_float WGLEW_GET_VAR(__WGLEW_ARB_pixel_format_float)

#endif /* WGL_ARB_pixel_format_float */

/* ------------------------- WGL_ARB_render_texture ------------------------ */

#ifndef WGL_ARB_render_texture
#define WGL_ARB_render_texture 1

#define WGL_BIND_TO_TEXTURE_RGB_ARB 0x2070
#define WGL_BIND_TO_TEXTURE_RGBA_ARB 0x2071
#define WGL_TEXTURE_FORMAT_ARB 0x2072
#define WGL_TEXTURE_TARGET_ARB 0x2073
#define WGL_MIPMAP_TEXTURE_ARB 0x2074
#define WGL_TEXTURE_RGB_ARB 0x2075
#define WGL_TEXTURE_RGBA_ARB 0x2076
#define WGL_NO_TEXTURE_ARB 0x2077
#define WGL_TEXTURE_CUBE_MAP_ARB 0x2078
#define WGL_TEXTURE_1D_ARB 0x2079
#define WGL_TEXTURE_2D_ARB 0x207A
#define WGL_MIPMAP_LEVEL_ARB 0x207B
#define WGL_CUBE_MAP_FACE_ARB 0x207C
#define WGL_TEXTURE_CUBE_MAP_POSITIVE_X_ARB 0x207D
#define WGL_TEXTURE_CUBE_MAP_NEGATIVE_X_ARB 0x207E
#define WGL_TEXTURE_CUBE_MAP_POSITIVE_Y_ARB 0x207F
#define WGL_TEXTURE_CUBE_MAP_NEGATIVE_Y_ARB 0x2080
#define WGL_TEXTURE_CUBE_MAP_POSITIVE_Z_ARB 0x2081
#define WGL_TEXTURE_CUBE_MAP_NEGATIVE_Z_ARB 0x2082
#define WGL_FRONT_LEFT_ARB 0x2083
#define WGL_FRONT_RIGHT_ARB 0x2084
#define WGL_BACK_LEFT_ARB 0x2085
#define WGL_BACK_RIGHT_ARB 0x2086
#define WGL_AUX0_ARB 0x2087
#define WGL_AUX1_ARB 0x2088
#define WGL_AUX2_ARB 0x2089
#define WGL_AUX3_ARB 0x208A
#define WGL_AUX4_ARB 0x208B
#define WGL_AUX5_ARB 0x208C
#define WGL_AUX6_ARB 0x208D
#define WGL_AUX7_ARB 0x208E
#define WGL_AUX8_ARB 0x208F
#define WGL_AUX9_ARB 0x2090

typedef BOOL (WINAPI * PFNWGLBINDTEXIMAGEARBPROC) (HPBUFFERARB hPbuffer, int iBuffer);
typedef BOOL (WINAPI * PFNWGLRELEASETEXIMAGEARBPROC) (HPBUFFERARB hPbuffer, int iBuffer);
typedef BOOL (WINAPI * PFNWGLSETPBUFFERATTRIBARBPROC) (HPBUFFERARB hPbuffer, const int* piAttribList);

#define wglBindTexImageARB WGLEW_GET_FUN(__wglewBindTexImageARB)
#define wglReleaseTexImageARB WGLEW_GET_FUN(__wglewReleaseTexImageARB)
#define wglSetPbufferAttribARB WGLEW_GET_FUN(__wglewSetPbufferAttribARB)

#define WGLEW_ARB_render_texture WGLEW_GET_VAR(__WGLEW_ARB_render_texture)

#endif /* WGL_ARB_render_texture */

/* ----------------------- WGL_ATI_pixel_format_float ---------------------- */

#ifndef WGL_ATI_pixel_format_float
#define WGL_ATI_pixel_format_float 1

#define WGL_TYPE_RGBA_FLOAT_ATI 0x21A0
#define GL_RGBA_FLOAT_MODE_ATI 0x8820
#define GL_COLOR_CLEAR_UNCLAMPED_VALUE_ATI 0x8835

#define WGLEW_ATI_pixel_format_float WGLEW_GET_VAR(__WGLEW_ATI_pixel_format_float)

#endif /* WGL_ATI_pixel_format_float */

/* -------------------- WGL_ATI_render_texture_rectangle ------------------- */

#ifndef WGL_ATI_render_texture_rectangle
#define WGL_ATI_render_texture_rectangle 1

#define WGL_TEXTURE_RECTANGLE_ATI 0x21A5

#define WGLEW_ATI_render_texture_rectangle WGLEW_GET_VAR(__WGLEW_ATI_render_texture_rectangle)

#endif /* WGL_ATI_render_texture_rectangle */

/* -------------------------- WGL_EXT_depth_float -------------------------- */

#ifndef WGL_EXT_depth_float
#define WGL_EXT_depth_float 1

#define WGL_DEPTH_FLOAT_EXT 0x2040

#define WGLEW_EXT_depth_float WGLEW_GET_VAR(__WGLEW_EXT_depth_float)

#endif /* WGL_EXT_depth_float */

/* ---------------------- WGL_EXT_display_color_table ---------------------- */

#ifndef WGL_EXT_display_color_table
#define WGL_EXT_display_color_table 1

typedef GLboolean (WINAPI * PFNWGLBINDDISPLAYCOLORTABLEEXTPROC) (GLushort id);
typedef GLboolean (WINAPI * PFNWGLCREATEDISPLAYCOLORTABLEEXTPROC) (GLushort id);
typedef void (WINAPI * PFNWGLDESTROYDISPLAYCOLORTABLEEXTPROC) (GLushort id);
typedef GLboolean (WINAPI * PFNWGLLOADDISPLAYCOLORTABLEEXTPROC) (GLushort* table, GLuint length);

#define wglBindDisplayColorTableEXT WGLEW_GET_FUN(__wglewBindDisplayColorTableEXT)
#define wglCreateDisplayColorTableEXT WGLEW_GET_FUN(__wglewCreateDisplayColorTableEXT)
#define wglDestroyDisplayColorTableEXT WGLEW_GET_FUN(__wglewDestroyDisplayColorTableEXT)
#define wglLoadDisplayColorTableEXT WGLEW_GET_FUN(__wglewLoadDisplayColorTableEXT)

#define WGLEW_EXT_display_color_table WGLEW_GET_VAR(__WGLEW_EXT_display_color_table)

#endif /* WGL_EXT_display_color_table */

/* ----------------------- WGL_EXT_extensions_string ----------------------- */

#ifndef WGL_EXT_extensions_string
#define WGL_EXT_extensions_string 1

typedef const char* (WINAPI * PFNWGLGETEXTENSIONSSTRINGEXTPROC) (void);

#define wglGetExtensionsStringEXT WGLEW_GET_FUN(__wglewGetExtensionsStringEXT)

#define WGLEW_EXT_extensions_string WGLEW_GET_VAR(__WGLEW_EXT_extensions_string)

#endif /* WGL_EXT_extensions_string */

/* ------------------------ WGL_EXT_framebuffer_sRGB ----------------------- */

#ifndef WGL_EXT_framebuffer_sRGB
#define WGL_EXT_framebuffer_sRGB 1

#define WGL_FRAMEBUFFER_SRGB_CAPABLE_EXT 0x20A9

#define WGLEW_EXT_framebuffer_sRGB WGLEW_GET_VAR(__WGLEW_EXT_framebuffer_sRGB)

#endif /* WGL_EXT_framebuffer_sRGB */

/* ----------------------- WGL_EXT_make_current_read ----------------------- */

#ifndef WGL_EXT_make_current_read
#define WGL_EXT_make_current_read 1

#define ERROR_INVALID_PIXEL_TYPE_EXT 0x2043

typedef HDC (WINAPI * PFNWGLGETCURRENTREADDCEXTPROC) (VOID);
typedef BOOL (WINAPI * PFNWGLMAKECONTEXTCURRENTEXTPROC) (HDC hDrawDC, HDC hReadDC, HGLRC hglrc);

#define wglGetCurrentReadDCEXT WGLEW_GET_FUN(__wglewGetCurrentReadDCEXT)
#define wglMakeContextCurrentEXT WGLEW_GET_FUN(__wglewMakeContextCurrentEXT)

#define WGLEW_EXT_make_current_read WGLEW_GET_VAR(__WGLEW_EXT_make_current_read)

#endif /* WGL_EXT_make_current_read */

/* -------------------------- WGL_EXT_multisample -------------------------- */

#ifndef WGL_EXT_multisample
#define WGL_EXT_multisample 1

#define WGL_SAMPLE_BUFFERS_EXT 0x2041
#define WGL_SAMPLES_EXT 0x2042

#define WGLEW_EXT_multisample WGLEW_GET_VAR(__WGLEW_EXT_multisample)

#endif /* WGL_EXT_multisample */

/* ---------------------------- WGL_EXT_pbuffer ---------------------------- */

#ifndef WGL_EXT_pbuffer
#define WGL_EXT_pbuffer 1

#define WGL_DRAW_TO_PBUFFER_EXT 0x202D
#define WGL_MAX_PBUFFER_PIXELS_EXT 0x202E
#define WGL_MAX_PBUFFER_WIDTH_EXT 0x202F
#define WGL_MAX_PBUFFER_HEIGHT_EXT 0x2030
#define WGL_OPTIMAL_PBUFFER_WIDTH_EXT 0x2031
#define WGL_OPTIMAL_PBUFFER_HEIGHT_EXT 0x2032
#define WGL_PBUFFER_LARGEST_EXT 0x2033
#define WGL_PBUFFER_WIDTH_EXT 0x2034
#define WGL_PBUFFER_HEIGHT_EXT 0x2035

DECLARE_HANDLE(HPBUFFEREXT);

typedef HPBUFFEREXT (WINAPI * PFNWGLCREATEPBUFFEREXTPROC) (HDC hDC, int iPixelFormat, int iWidth, int iHeight, const int* piAttribList);
typedef BOOL (WINAPI * PFNWGLDESTROYPBUFFEREXTPROC) (HPBUFFEREXT hPbuffer);
typedef HDC (WINAPI * PFNWGLGETPBUFFERDCEXTPROC) (HPBUFFEREXT hPbuffer);
typedef BOOL (WINAPI * PFNWGLQUERYPBUFFEREXTPROC) (HPBUFFEREXT hPbuffer, int iAttribute, int* piValue);
typedef int (WINAPI * PFNWGLRELEASEPBUFFERDCEXTPROC) (HPBUFFEREXT hPbuffer, HDC hDC);

#define wglCreatePbufferEXT WGLEW_GET_FUN(__wglewCreatePbufferEXT)
#define wglDestroyPbufferEXT WGLEW_GET_FUN(__wglewDestroyPbufferEXT)
#define wglGetPbufferDCEXT WGLEW_GET_FUN(__wglewGetPbufferDCEXT)
#define wglQueryPbufferEXT WGLEW_GET_FUN(__wglewQueryPbufferEXT)
#define wglReleasePbufferDCEXT WGLEW_GET_FUN(__wglewReleasePbufferDCEXT)

#define WGLEW_EXT_pbuffer WGLEW_GET_VAR(__WGLEW_EXT_pbuffer)

#endif /* WGL_EXT_pbuffer */

/* -------------------------- WGL_EXT_pixel_format ------------------------- */

#ifndef WGL_EXT_pixel_format
#define WGL_EXT_pixel_format 1

#define WGL_NUMBER_PIXEL_FORMATS_EXT 0x2000
#define WGL_DRAW_TO_WINDOW_EXT 0x2001
#define WGL_DRAW_TO_BITMAP_EXT 0x2002
#define WGL_ACCELERATION_EXT 0x2003
#define WGL_NEED_PALETTE_EXT 0x2004
#define WGL_NEED_SYSTEM_PALETTE_EXT 0x2005
#define WGL_SWAP_LAYER_BUFFERS_EXT 0x2006
#define WGL_SWAP_METHOD_EXT 0x2007
#define WGL_NUMBER_OVERLAYS_EXT 0x2008
#define WGL_NUMBER_UNDERLAYS_EXT 0x2009
#define WGL_TRANSPARENT_EXT 0x200A
#define WGL_TRANSPARENT_VALUE_EXT 0x200B
#define WGL_SHARE_DEPTH_EXT 0x200C
#define WGL_SHARE_STENCIL_EXT 0x200D
#define WGL_SHARE_ACCUM_EXT 0x200E
#define WGL_SUPPORT_GDI_EXT 0x200F
#define WGL_SUPPORT_OPENGL_EXT 0x2010
#define WGL_DOUBLE_BUFFER_EXT 0x2011
#define WGL_STEREO_EXT 0x2012
#define WGL_PIXEL_TYPE_EXT 0x2013
#define WGL_COLOR_BITS_EXT 0x2014
#define WGL_RED_BITS_EXT 0x2015
#define WGL_RED_SHIFT_EXT 0x2016
#define WGL_GREEN_BITS_EXT 0x2017
#define WGL_GREEN_SHIFT_EXT 0x2018
#define WGL_BLUE_BITS_EXT 0x2019
#define WGL_BLUE_SHIFT_EXT 0x201A
#define WGL_ALPHA_BITS_EXT 0x201B
#define WGL_ALPHA_SHIFT_EXT 0x201C
#define WGL_ACCUM_BITS_EXT 0x201D
#define WGL_ACCUM_RED_BITS_EXT 0x201E
#define WGL_ACCUM_GREEN_BITS_EXT 0x201F
#define WGL_ACCUM_BLUE_BITS_EXT 0x2020
#define WGL_ACCUM_ALPHA_BITS_EXT 0x2021
#define WGL_DEPTH_BITS_EXT 0x2022
#define WGL_STENCIL_BITS_EXT 0x2023
#define WGL_AUX_BUFFERS_EXT 0x2024
#define WGL_NO_ACCELERATION_EXT 0x2025
#define WGL_GENERIC_ACCELERATION_EXT 0x2026
#define WGL_FULL_ACCELERATION_EXT 0x2027
#define WGL_SWAP_EXCHANGE_EXT 0x2028
#define WGL_SWAP_COPY_EXT 0x2029
#define WGL_SWAP_UNDEFINED_EXT 0x202A
#define WGL_TYPE_RGBA_EXT 0x202B
#define WGL_TYPE_COLORINDEX_EXT 0x202C

typedef BOOL (WINAPI * PFNWGLCHOOSEPIXELFORMATEXTPROC) (HDC hdc, const int* piAttribIList, const FLOAT *pfAttribFList, UINT nMaxFormats, int *piFormats, UINT *nNumFormats);
typedef BOOL (WINAPI * PFNWGLGETPIXELFORMATATTRIBFVEXTPROC) (HDC hdc, int iPixelFormat, int iLayerPlane, UINT nAttributes, int* piAttributes, FLOAT *pfValues);
typedef BOOL (WINAPI * PFNWGLGETPIXELFORMATATTRIBIVEXTPROC) (HDC hdc, int iPixelFormat, int iLayerPlane, UINT nAttributes, int* piAttributes, int *piValues);

#define wglChoosePixelFormatEXT WGLEW_GET_FUN(__wglewChoosePixelFormatEXT)
#define wglGetPixelFormatAttribfvEXT WGLEW_GET_FUN(__wglewGetPixelFormatAttribfvEXT)
#define wglGetPixelFormatAttribivEXT WGLEW_GET_FUN(__wglewGetPixelFormatAttribivEXT)

#define WGLEW_EXT_pixel_format WGLEW_GET_VAR(__WGLEW_EXT_pixel_format)

#endif /* WGL_EXT_pixel_format */

/* ------------------- WGL_EXT_pixel_format_packed_float ------------------- */

#ifndef WGL_EXT_pixel_format_packed_float
#define WGL_EXT_pixel_format_packed_float 1

#define WGL_TYPE_RGBA_UNSIGNED_FLOAT_EXT 0x20A8

#define WGLEW_EXT_pixel_format_packed_float WGLEW_GET_VAR(__WGLEW_EXT_pixel_format_packed_float)

#endif /* WGL_EXT_pixel_format_packed_float */

/* -------------------------- WGL_EXT_swap_control ------------------------- */

#ifndef WGL_EXT_swap_control
#define WGL_EXT_swap_control 1

typedef int (WINAPI * PFNWGLGETSWAPINTERVALEXTPROC) (void);
typedef BOOL (WINAPI * PFNWGLSWAPINTERVALEXTPROC) (int interval);

#define wglGetSwapIntervalEXT WGLEW_GET_FUN(__wglewGetSwapIntervalEXT)
#define wglSwapIntervalEXT WGLEW_GET_FUN(__wglewSwapIntervalEXT)

#define WGLEW_EXT_swap_control WGLEW_GET_VAR(__WGLEW_EXT_swap_control)

#endif /* WGL_EXT_swap_control */

/* --------------------- WGL_I3D_digital_video_control --------------------- */

#ifndef WGL_I3D_digital_video_control
#define WGL_I3D_digital_video_control 1

#define WGL_DIGITAL_VIDEO_CURSOR_ALPHA_FRAMEBUFFER_I3D 0x2050
#define WGL_DIGITAL_VIDEO_CURSOR_ALPHA_VALUE_I3D 0x2051
#define WGL_DIGITAL_VIDEO_CURSOR_INCLUDED_I3D 0x2052
#define WGL_DIGITAL_VIDEO_GAMMA_CORRECTED_I3D 0x2053

typedef BOOL (WINAPI * PFNWGLGETDIGITALVIDEOPARAMETERSI3DPROC) (HDC hDC, int iAttribute, int* piValue);
typedef BOOL (WINAPI * PFNWGLSETDIGITALVIDEOPARAMETERSI3DPROC) (HDC hDC, int iAttribute, const int* piValue);

#define wglGetDigitalVideoParametersI3D WGLEW_GET_FUN(__wglewGetDigitalVideoParametersI3D)
#define wglSetDigitalVideoParametersI3D WGLEW_GET_FUN(__wglewSetDigitalVideoParametersI3D)

#define WGLEW_I3D_digital_video_control WGLEW_GET_VAR(__WGLEW_I3D_digital_video_control)

#endif /* WGL_I3D_digital_video_control */

/* ----------------------------- WGL_I3D_gamma ----------------------------- */

#ifndef WGL_I3D_gamma
#define WGL_I3D_gamma 1

#define WGL_GAMMA_TABLE_SIZE_I3D 0x204E
#define WGL_GAMMA_EXCLUDE_DESKTOP_I3D 0x204F

typedef BOOL (WINAPI * PFNWGLGETGAMMATABLEI3DPROC) (HDC hDC, int iEntries, USHORT* puRed, USHORT *puGreen, USHORT *puBlue);
typedef BOOL (WINAPI * PFNWGLGETGAMMATABLEPARAMETERSI3DPROC) (HDC hDC, int iAttribute, int* piValue);
typedef BOOL (WINAPI * PFNWGLSETGAMMATABLEI3DPROC) (HDC hDC, int iEntries, const USHORT* puRed, const USHORT *puGreen, const USHORT *puBlue);
typedef BOOL (WINAPI * PFNWGLSETGAMMATABLEPARAMETERSI3DPROC) (HDC hDC, int iAttribute, const int* piValue);

#define wglGetGammaTableI3D WGLEW_GET_FUN(__wglewGetGammaTableI3D)
#define wglGetGammaTableParametersI3D WGLEW_GET_FUN(__wglewGetGammaTableParametersI3D)
#define wglSetGammaTableI3D WGLEW_GET_FUN(__wglewSetGammaTableI3D)
#define wglSetGammaTableParametersI3D WGLEW_GET_FUN(__wglewSetGammaTableParametersI3D)

#define WGLEW_I3D_gamma WGLEW_GET_VAR(__WGLEW_I3D_gamma)

#endif /* WGL_I3D_gamma */

/* ---------------------------- WGL_I3D_genlock ---------------------------- */

#ifndef WGL_I3D_genlock
#define WGL_I3D_genlock 1

#define WGL_GENLOCK_SOURCE_MULTIVIEW_I3D 0x2044
#define WGL_GENLOCK_SOURCE_EXTERNAL_SYNC_I3D 0x2045
#define WGL_GENLOCK_SOURCE_EXTERNAL_FIELD_I3D 0x2046
#define WGL_GENLOCK_SOURCE_EXTERNAL_TTL_I3D 0x2047
#define WGL_GENLOCK_SOURCE_DIGITAL_SYNC_I3D 0x2048
#define WGL_GENLOCK_SOURCE_DIGITAL_FIELD_I3D 0x2049
#define WGL_GENLOCK_SOURCE_EDGE_FALLING_I3D 0x204A
#define WGL_GENLOCK_SOURCE_EDGE_RISING_I3D 0x204B
#define WGL_GENLOCK_SOURCE_EDGE_BOTH_I3D 0x204C

typedef BOOL (WINAPI * PFNWGLDISABLEGENLOCKI3DPROC) (HDC hDC);
typedef BOOL (WINAPI * PFNWGLENABLEGENLOCKI3DPROC) (HDC hDC);
typedef BOOL (WINAPI * PFNWGLGENLOCKSAMPLERATEI3DPROC) (HDC hDC, UINT uRate);
typedef BOOL (WINAPI * PFNWGLGENLOCKSOURCEDELAYI3DPROC) (HDC hDC, UINT uDelay);
typedef BOOL (WINAPI * PFNWGLGENLOCKSOURCEEDGEI3DPROC) (HDC hDC, UINT uEdge);
typedef BOOL (WINAPI * PFNWGLGENLOCKSOURCEI3DPROC) (HDC hDC, UINT uSource);
typedef BOOL (WINAPI * PFNWGLGETGENLOCKSAMPLERATEI3DPROC) (HDC hDC, UINT* uRate);
typedef BOOL (WINAPI * PFNWGLGETGENLOCKSOURCEDELAYI3DPROC) (HDC hDC, UINT* uDelay);
typedef BOOL (WINAPI * PFNWGLGETGENLOCKSOURCEEDGEI3DPROC) (HDC hDC, UINT* uEdge);
typedef BOOL (WINAPI * PFNWGLGETGENLOCKSOURCEI3DPROC) (HDC hDC, UINT* uSource);
typedef BOOL (WINAPI * PFNWGLISENABLEDGENLOCKI3DPROC) (HDC hDC, BOOL* pFlag);
typedef BOOL (WINAPI * PFNWGLQUERYGENLOCKMAXSOURCEDELAYI3DPROC) (HDC hDC, UINT* uMaxLineDelay, UINT *uMaxPixelDelay);

#define wglDisableGenlockI3D WGLEW_GET_FUN(__wglewDisableGenlockI3D)
#define wglEnableGenlockI3D WGLEW_GET_FUN(__wglewEnableGenlockI3D)
#define wglGenlockSampleRateI3D WGLEW_GET_FUN(__wglewGenlockSampleRateI3D)
#define wglGenlockSourceDelayI3D WGLEW_GET_FUN(__wglewGenlockSourceDelayI3D)
#define wglGenlockSourceEdgeI3D WGLEW_GET_FUN(__wglewGenlockSourceEdgeI3D)
#define wglGenlockSourceI3D WGLEW_GET_FUN(__wglewGenlockSourceI3D)
#define wglGetGenlockSampleRateI3D WGLEW_GET_FUN(__wglewGetGenlockSampleRateI3D)
#define wglGetGenlockSourceDelayI3D WGLEW_GET_FUN(__wglewGetGenlockSourceDelayI3D)
#define wglGetGenlockSourceEdgeI3D WGLEW_GET_FUN(__wglewGetGenlockSourceEdgeI3D)
#define wglGetGenlockSourceI3D WGLEW_GET_FUN(__wglewGetGenlockSourceI3D)
#define wglIsEnabledGenlockI3D WGLEW_GET_FUN(__wglewIsEnabledGenlockI3D)
#define wglQueryGenlockMaxSourceDelayI3D WGLEW_GET_FUN(__wglewQueryGenlockMaxSourceDelayI3D)

#define WGLEW_I3D_genlock WGLEW_GET_VAR(__WGLEW_I3D_genlock)

#endif /* WGL_I3D_genlock */

/* -------------------------- WGL_I3D_image_buffer ------------------------- */

#ifndef WGL_I3D_image_buffer
#define WGL_I3D_image_buffer 1

#define WGL_IMAGE_BUFFER_MIN_ACCESS_I3D 0x00000001
#define WGL_IMAGE_BUFFER_LOCK_I3D 0x00000002

typedef BOOL (WINAPI * PFNWGLASSOCIATEIMAGEBUFFEREVENTSI3DPROC) (HDC hdc, HANDLE* pEvent, LPVOID *pAddress, DWORD *pSize, UINT count);
typedef LPVOID (WINAPI * PFNWGLCREATEIMAGEBUFFERI3DPROC) (HDC hDC, DWORD dwSize, UINT uFlags);
typedef BOOL (WINAPI * PFNWGLDESTROYIMAGEBUFFERI3DPROC) (HDC hDC, LPVOID pAddress);
typedef BOOL (WINAPI * PFNWGLRELEASEIMAGEBUFFEREVENTSI3DPROC) (HDC hdc, LPVOID* pAddress, UINT count);

#define wglAssociateImageBufferEventsI3D WGLEW_GET_FUN(__wglewAssociateImageBufferEventsI3D)
#define wglCreateImageBufferI3D WGLEW_GET_FUN(__wglewCreateImageBufferI3D)
#define wglDestroyImageBufferI3D WGLEW_GET_FUN(__wglewDestroyImageBufferI3D)
#define wglReleaseImageBufferEventsI3D WGLEW_GET_FUN(__wglewReleaseImageBufferEventsI3D)

#define WGLEW_I3D_image_buffer WGLEW_GET_VAR(__WGLEW_I3D_image_buffer)

#endif /* WGL_I3D_image_buffer */

/* ------------------------ WGL_I3D_swap_frame_lock ------------------------ */

#ifndef WGL_I3D_swap_frame_lock
#define WGL_I3D_swap_frame_lock 1

typedef BOOL (WINAPI * PFNWGLDISABLEFRAMELOCKI3DPROC) (VOID);
typedef BOOL (WINAPI * PFNWGLENABLEFRAMELOCKI3DPROC) (VOID);
typedef BOOL (WINAPI * PFNWGLISENABLEDFRAMELOCKI3DPROC) (BOOL* pFlag);
typedef BOOL (WINAPI * PFNWGLQUERYFRAMELOCKMASTERI3DPROC) (BOOL* pFlag);

#define wglDisableFrameLockI3D WGLEW_GET_FUN(__wglewDisableFrameLockI3D)
#define wglEnableFrameLockI3D WGLEW_GET_FUN(__wglewEnableFrameLockI3D)
#define wglIsEnabledFrameLockI3D WGLEW_GET_FUN(__wglewIsEnabledFrameLockI3D)
#define wglQueryFrameLockMasterI3D WGLEW_GET_FUN(__wglewQueryFrameLockMasterI3D)

#define WGLEW_I3D_swap_frame_lock WGLEW_GET_VAR(__WGLEW_I3D_swap_frame_lock)

#endif /* WGL_I3D_swap_frame_lock */

/* ------------------------ WGL_I3D_swap_frame_usage ----------------------- */

#ifndef WGL_I3D_swap_frame_usage
#define WGL_I3D_swap_frame_usage 1

typedef BOOL (WINAPI * PFNWGLBEGINFRAMETRACKINGI3DPROC) (void);
typedef BOOL (WINAPI * PFNWGLENDFRAMETRACKINGI3DPROC) (void);
typedef BOOL (WINAPI * PFNWGLGETFRAMEUSAGEI3DPROC) (float* pUsage);
typedef BOOL (WINAPI * PFNWGLQUERYFRAMETRACKINGI3DPROC) (DWORD* pFrameCount, DWORD *pMissedFrames, float *pLastMissedUsage);

#define wglBeginFrameTrackingI3D WGLEW_GET_FUN(__wglewBeginFrameTrackingI3D)
#define wglEndFrameTrackingI3D WGLEW_GET_FUN(__wglewEndFrameTrackingI3D)
#define wglGetFrameUsageI3D WGLEW_GET_FUN(__wglewGetFrameUsageI3D)
#define wglQueryFrameTrackingI3D WGLEW_GET_FUN(__wglewQueryFrameTrackingI3D)

#define WGLEW_I3D_swap_frame_usage WGLEW_GET_VAR(__WGLEW_I3D_swap_frame_usage)

#endif /* WGL_I3D_swap_frame_usage */

/* --------------------------- WGL_NV_copy_image --------------------------- */

#ifndef WGL_NV_copy_image
#define WGL_NV_copy_image 1

typedef BOOL (WINAPI * PFNWGLCOPYIMAGESUBDATANVPROC) (HGLRC hSrcRC, GLuint srcName, GLenum srcTarget, GLint srcLevel, GLint srcX, GLint srcY, GLint srcZ, HGLRC hDstRC, GLuint dstName, GLenum dstTarget, GLint dstLevel, GLint dstX, GLint dstY, GLint dstZ, GLsizei width, GLsizei height, GLsizei depth);

#define wglCopyImageSubDataNV WGLEW_GET_FUN(__wglewCopyImageSubDataNV)

#define WGLEW_NV_copy_image WGLEW_GET_VAR(__WGLEW_NV_copy_image)

#endif /* WGL_NV_copy_image */

/* -------------------------- WGL_NV_float_buffer -------------------------- */

#ifndef WGL_NV_float_buffer
#define WGL_NV_float_buffer 1

#define WGL_FLOAT_COMPONENTS_NV 0x20B0
#define WGL_BIND_TO_TEXTURE_RECTANGLE_FLOAT_R_NV 0x20B1
#define WGL_BIND_TO_TEXTURE_RECTANGLE_FLOAT_RG_NV 0x20B2
#define WGL_BIND_TO_TEXTURE_RECTANGLE_FLOAT_RGB_NV 0x20B3
#define WGL_BIND_TO_TEXTURE_RECTANGLE_FLOAT_RGBA_NV 0x20B4
#define WGL_TEXTURE_FLOAT_R_NV 0x20B5
#define WGL_TEXTURE_FLOAT_RG_NV 0x20B6
#define WGL_TEXTURE_FLOAT_RGB_NV 0x20B7
#define WGL_TEXTURE_FLOAT_RGBA_NV 0x20B8

#define WGLEW_NV_float_buffer WGLEW_GET_VAR(__WGLEW_NV_float_buffer)

#endif /* WGL_NV_float_buffer */

/* -------------------------- WGL_NV_gpu_affinity -------------------------- */

#ifndef WGL_NV_gpu_affinity
#define WGL_NV_gpu_affinity 1

#define WGL_ERROR_INCOMPATIBLE_AFFINITY_MASKS_NV 0x20D0
#define WGL_ERROR_MISSING_AFFINITY_MASK_NV 0x20D1

DECLARE_HANDLE(HGPUNV);
typedef struct _GPU_DEVICE {
  DWORD cb; 
  CHAR DeviceName[32]; 
  CHAR DeviceString[128]; 
  DWORD Flags; 
  RECT rcVirtualScreen; 
} GPU_DEVICE, *PGPU_DEVICE;

typedef HDC (WINAPI * PFNWGLCREATEAFFINITYDCNVPROC) (const HGPUNV *phGpuList);
typedef BOOL (WINAPI * PFNWGLDELETEDCNVPROC) (HDC hdc);
typedef BOOL (WINAPI * PFNWGLENUMGPUDEVICESNVPROC) (HGPUNV hGpu, UINT iDeviceIndex, PGPU_DEVICE lpGpuDevice);
typedef BOOL (WINAPI * PFNWGLENUMGPUSFROMAFFINITYDCNVPROC) (HDC hAffinityDC, UINT iGpuIndex, HGPUNV *hGpu);
typedef BOOL (WINAPI * PFNWGLENUMGPUSNVPROC) (UINT iGpuIndex, HGPUNV *phGpu);

#define wglCreateAffinityDCNV WGLEW_GET_FUN(__wglewCreateAffinityDCNV)
#define wglDeleteDCNV WGLEW_GET_FUN(__wglewDeleteDCNV)
#define wglEnumGpuDevicesNV WGLEW_GET_FUN(__wglewEnumGpuDevicesNV)
#define wglEnumGpusFromAffinityDCNV WGLEW_GET_FUN(__wglewEnumGpusFromAffinityDCNV)
#define wglEnumGpusNV WGLEW_GET_FUN(__wglewEnumGpusNV)

#define WGLEW_NV_gpu_affinity WGLEW_GET_VAR(__WGLEW_NV_gpu_affinity)

#endif /* WGL_NV_gpu_affinity */

/* ---------------------- WGL_NV_multisample_coverage ---------------------- */

#ifndef WGL_NV_multisample_coverage
#define WGL_NV_multisample_coverage 1

#define WGL_COVERAGE_SAMPLES_NV 0x2042
#define WGL_COLOR_SAMPLES_NV 0x20B9

#define WGLEW_NV_multisample_coverage WGLEW_GET_VAR(__WGLEW_NV_multisample_coverage)

#endif /* WGL_NV_multisample_coverage */

/* -------------------------- WGL_NV_present_video ------------------------- */

#ifndef WGL_NV_present_video
#define WGL_NV_present_video 1

#define WGL_NUM_VIDEO_SLOTS_NV 0x20F0

DECLARE_HANDLE(HVIDEOOUTPUTDEVICENV);

typedef BOOL (WINAPI * PFNWGLBINDVIDEODEVICENVPROC) (HDC hDc, unsigned int uVideoSlot, HVIDEOOUTPUTDEVICENV hVideoDevice, const int* piAttribList);
typedef int (WINAPI * PFNWGLENUMERATEVIDEODEVICESNVPROC) (HDC hDc, HVIDEOOUTPUTDEVICENV* phDeviceList);
typedef BOOL (WINAPI * PFNWGLQUERYCURRENTCONTEXTNVPROC) (int iAttribute, int* piValue);

#define wglBindVideoDeviceNV WGLEW_GET_FUN(__wglewBindVideoDeviceNV)
#define wglEnumerateVideoDevicesNV WGLEW_GET_FUN(__wglewEnumerateVideoDevicesNV)
#define wglQueryCurrentContextNV WGLEW_GET_FUN(__wglewQueryCurrentContextNV)

#define WGLEW_NV_present_video WGLEW_GET_VAR(__WGLEW_NV_present_video)

#endif /* WGL_NV_present_video */

/* ---------------------- WGL_NV_render_depth_texture ---------------------- */

#ifndef WGL_NV_render_depth_texture
#define WGL_NV_render_depth_texture 1

#define WGL_NO_TEXTURE_ARB 0x2077
#define WGL_BIND_TO_TEXTURE_DEPTH_NV 0x20A3
#define WGL_BIND_TO_TEXTURE_RECTANGLE_DEPTH_NV 0x20A4
#define WGL_DEPTH_TEXTURE_FORMAT_NV 0x20A5
#define WGL_TEXTURE_DEPTH_COMPONENT_NV 0x20A6
#define WGL_DEPTH_COMPONENT_NV 0x20A7

#define WGLEW_NV_render_depth_texture WGLEW_GET_VAR(__WGLEW_NV_render_depth_texture)

#endif /* WGL_NV_render_depth_texture */

/* -------------------- WGL_NV_render_texture_rectangle -------------------- */

#ifndef WGL_NV_render_texture_rectangle
#define WGL_NV_render_texture_rectangle 1

#define WGL_BIND_TO_TEXTURE_RECTANGLE_RGB_NV 0x20A0
#define WGL_BIND_TO_TEXTURE_RECTANGLE_RGBA_NV 0x20A1
#define WGL_TEXTURE_RECTANGLE_NV 0x20A2

#define WGLEW_NV_render_texture_rectangle WGLEW_GET_VAR(__WGLEW_NV_render_texture_rectangle)

#endif /* WGL_NV_render_texture_rectangle */

/* --------------------------- WGL_NV_swap_group --------------------------- */

#ifndef WGL_NV_swap_group
#define WGL_NV_swap_group 1

typedef BOOL (WINAPI * PFNWGLBINDSWAPBARRIERNVPROC) (GLuint group, GLuint barrier);
typedef BOOL (WINAPI * PFNWGLJOINSWAPGROUPNVPROC) (HDC hDC, GLuint group);
typedef BOOL (WINAPI * PFNWGLQUERYFRAMECOUNTNVPROC) (HDC hDC, GLuint* count);
typedef BOOL (WINAPI * PFNWGLQUERYMAXSWAPGROUPSNVPROC) (HDC hDC, GLuint* maxGroups, GLuint *maxBarriers);
typedef BOOL (WINAPI * PFNWGLQUERYSWAPGROUPNVPROC) (HDC hDC, GLuint* group, GLuint *barrier);
typedef BOOL (WINAPI * PFNWGLRESETFRAMECOUNTNVPROC) (HDC hDC);

#define wglBindSwapBarrierNV WGLEW_GET_FUN(__wglewBindSwapBarrierNV)
#define wglJoinSwapGroupNV WGLEW_GET_FUN(__wglewJoinSwapGroupNV)
#define wglQueryFrameCountNV WGLEW_GET_FUN(__wglewQueryFrameCountNV)
#define wglQueryMaxSwapGroupsNV WGLEW_GET_FUN(__wglewQueryMaxSwapGroupsNV)
#define wglQuerySwapGroupNV WGLEW_GET_FUN(__wglewQuerySwapGroupNV)
#define wglResetFrameCountNV WGLEW_GET_FUN(__wglewResetFrameCountNV)

#define WGLEW_NV_swap_group WGLEW_GET_VAR(__WGLEW_NV_swap_group)

#endif /* WGL_NV_swap_group */

/* ----------------------- WGL_NV_vertex_array_range ----------------------- */

#ifndef WGL_NV_vertex_array_range
#define WGL_NV_vertex_array_range 1

typedef void * (WINAPI * PFNWGLALLOCATEMEMORYNVPROC) (GLsizei size, GLfloat readFrequency, GLfloat writeFrequency, GLfloat priority);
typedef void (WINAPI * PFNWGLFREEMEMORYNVPROC) (void *pointer);

#define wglAllocateMemoryNV WGLEW_GET_FUN(__wglewAllocateMemoryNV)
#define wglFreeMemoryNV WGLEW_GET_FUN(__wglewFreeMemoryNV)

#define WGLEW_NV_vertex_array_range WGLEW_GET_VAR(__WGLEW_NV_vertex_array_range)

#endif /* WGL_NV_vertex_array_range */

/* -------------------------- WGL_NV_video_output -------------------------- */

#ifndef WGL_NV_video_output
#define WGL_NV_video_output 1

#define WGL_BIND_TO_VIDEO_RGB_NV 0x20C0
#define WGL_BIND_TO_VIDEO_RGBA_NV 0x20C1
#define WGL_BIND_TO_VIDEO_RGB_AND_DEPTH_NV 0x20C2
#define WGL_VIDEO_OUT_COLOR_NV 0x20C3
#define WGL_VIDEO_OUT_ALPHA_NV 0x20C4
#define WGL_VIDEO_OUT_DEPTH_NV 0x20C5
#define WGL_VIDEO_OUT_COLOR_AND_ALPHA_NV 0x20C6
#define WGL_VIDEO_OUT_COLOR_AND_DEPTH_NV 0x20C7
#define WGL_VIDEO_OUT_FRAME 0x20C8
#define WGL_VIDEO_OUT_FIELD_1 0x20C9
#define WGL_VIDEO_OUT_FIELD_2 0x20CA
#define WGL_VIDEO_OUT_STACKED_FIELDS_1_2 0x20CB
#define WGL_VIDEO_OUT_STACKED_FIELDS_2_1 0x20CC

DECLARE_HANDLE(HPVIDEODEV);

typedef BOOL (WINAPI * PFNWGLBINDVIDEOIMAGENVPROC) (HPVIDEODEV hVideoDevice, HPBUFFERARB hPbuffer, int iVideoBuffer);
typedef BOOL (WINAPI * PFNWGLGETVIDEODEVICENVPROC) (HDC hDC, int numDevices, HPVIDEODEV* hVideoDevice);
typedef BOOL (WINAPI * PFNWGLGETVIDEOINFONVPROC) (HPVIDEODEV hpVideoDevice, unsigned long* pulCounterOutputPbuffer, unsigned long *pulCounterOutputVideo);
typedef BOOL (WINAPI * PFNWGLRELEASEVIDEODEVICENVPROC) (HPVIDEODEV hVideoDevice);
typedef BOOL (WINAPI * PFNWGLRELEASEVIDEOIMAGENVPROC) (HPBUFFERARB hPbuffer, int iVideoBuffer);
typedef BOOL (WINAPI * PFNWGLSENDPBUFFERTOVIDEONVPROC) (HPBUFFERARB hPbuffer, int iBufferType, unsigned long* pulCounterPbuffer, BOOL bBlock);

#define wglBindVideoImageNV WGLEW_GET_FUN(__wglewBindVideoImageNV)
#define wglGetVideoDeviceNV WGLEW_GET_FUN(__wglewGetVideoDeviceNV)
#define wglGetVideoInfoNV WGLEW_GET_FUN(__wglewGetVideoInfoNV)
#define wglReleaseVideoDeviceNV WGLEW_GET_FUN(__wglewReleaseVideoDeviceNV)
#define wglReleaseVideoImageNV WGLEW_GET_FUN(__wglewReleaseVideoImageNV)
#define wglSendPbufferToVideoNV WGLEW_GET_FUN(__wglewSendPbufferToVideoNV)

#define WGLEW_NV_video_output WGLEW_GET_VAR(__WGLEW_NV_video_output)

#endif /* WGL_NV_video_output */

/* -------------------------- WGL_OML_sync_control ------------------------- */

#ifndef WGL_OML_sync_control
#define WGL_OML_sync_control 1

typedef BOOL (WINAPI * PFNWGLGETMSCRATEOMLPROC) (HDC hdc, INT32* numerator, INT32 *denominator);
typedef BOOL (WINAPI * PFNWGLGETSYNCVALUESOMLPROC) (HDC hdc, INT64* ust, INT64 *msc, INT64 *sbc);
typedef INT64 (WINAPI * PFNWGLSWAPBUFFERSMSCOMLPROC) (HDC hdc, INT64 target_msc, INT64 divisor, INT64 remainder);
typedef INT64 (WINAPI * PFNWGLSWAPLAYERBUFFERSMSCOMLPROC) (HDC hdc, INT fuPlanes, INT64 target_msc, INT64 divisor, INT64 remainder);
typedef BOOL (WINAPI * PFNWGLWAITFORMSCOMLPROC) (HDC hdc, INT64 target_msc, INT64 divisor, INT64 remainder, INT64* ust, INT64 *msc, INT64 *sbc);
typedef BOOL (WINAPI * PFNWGLWAITFORSBCOMLPROC) (HDC hdc, INT64 target_sbc, INT64* ust, INT64 *msc, INT64 *sbc);

#define wglGetMscRateOML WGLEW_GET_FUN(__wglewGetMscRateOML)
#define wglGetSyncValuesOML WGLEW_GET_FUN(__wglewGetSyncValuesOML)
#define wglSwapBuffersMscOML WGLEW_GET_FUN(__wglewSwapBuffersMscOML)
#define wglSwapLayerBuffersMscOML WGLEW_GET_FUN(__wglewSwapLayerBuffersMscOML)
#define wglWaitForMscOML WGLEW_GET_FUN(__wglewWaitForMscOML)
#define wglWaitForSbcOML WGLEW_GET_FUN(__wglewWaitForSbcOML)

#define WGLEW_OML_sync_control WGLEW_GET_VAR(__WGLEW_OML_sync_control)

#endif /* WGL_OML_sync_control */

/* ------------------------------------------------------------------------- */

#ifdef GLEW_MX
#define WGLEW_EXPORT
#else
#define WGLEW_EXPORT GLEWAPI
#endif /* GLEW_MX */

#ifdef GLEW_MX
struct WGLEWContextStruct
{
#endif /* GLEW_MX */

WGLEW_EXPORT PFNWGLSETSTEREOEMITTERSTATE3DLPROC __wglewSetStereoEmitterState3DL;

WGLEW_EXPORT PFNWGLBLITCONTEXTFRAMEBUFFERAMDPROC __wglewBlitContextFramebufferAMD;
WGLEW_EXPORT PFNWGLCREATEASSOCIATEDCONTEXTAMDPROC __wglewCreateAssociatedContextAMD;
WGLEW_EXPORT PFNWGLCREATEASSOCIATEDCONTEXTATTRIBSAMDPROC __wglewCreateAssociatedContextAttribsAMD;
WGLEW_EXPORT PFNWGLDELETEASSOCIATEDCONTEXTAMDPROC __wglewDeleteAssociatedContextAMD;
WGLEW_EXPORT PFNWGLGETCONTEXTGPUIDAMDPROC __wglewGetContextGPUIDAMD;
WGLEW_EXPORT PFNWGLGETCURRENTASSOCIATEDCONTEXTAMDPROC __wglewGetCurrentAssociatedContextAMD;
WGLEW_EXPORT PFNWGLGETGPUIDSAMDPROC __wglewGetGPUIDsAMD;
WGLEW_EXPORT PFNWGLGETGPUINFOAMDPROC __wglewGetGPUInfoAMD;
WGLEW_EXPORT PFNWGLMAKEASSOCIATEDCONTEXTCURRENTAMDPROC __wglewMakeAssociatedContextCurrentAMD;

WGLEW_EXPORT PFNWGLCREATEBUFFERREGIONARBPROC __wglewCreateBufferRegionARB;
WGLEW_EXPORT PFNWGLDELETEBUFFERREGIONARBPROC __wglewDeleteBufferRegionARB;
WGLEW_EXPORT PFNWGLRESTOREBUFFERREGIONARBPROC __wglewRestoreBufferRegionARB;
WGLEW_EXPORT PFNWGLSAVEBUFFERREGIONARBPROC __wglewSaveBufferRegionARB;

WGLEW_EXPORT PFNWGLCREATECONTEXTATTRIBSARBPROC __wglewCreateContextAttribsARB;

WGLEW_EXPORT PFNWGLGETEXTENSIONSSTRINGARBPROC __wglewGetExtensionsStringARB;

WGLEW_EXPORT PFNWGLGETCURRENTREADDCARBPROC __wglewGetCurrentReadDCARB;
WGLEW_EXPORT PFNWGLMAKECONTEXTCURRENTARBPROC __wglewMakeContextCurrentARB;

WGLEW_EXPORT PFNWGLCREATEPBUFFERARBPROC __wglewCreatePbufferARB;
WGLEW_EXPORT PFNWGLDESTROYPBUFFERARBPROC __wglewDestroyPbufferARB;
WGLEW_EXPORT PFNWGLGETPBUFFERDCARBPROC __wglewGetPbufferDCARB;
WGLEW_EXPORT PFNWGLQUERYPBUFFERARBPROC __wglewQueryPbufferARB;
WGLEW_EXPORT PFNWGLRELEASEPBUFFERDCARBPROC __wglewReleasePbufferDCARB;

WGLEW_EXPORT PFNWGLCHOOSEPIXELFORMATARBPROC __wglewChoosePixelFormatARB;
WGLEW_EXPORT PFNWGLGETPIXELFORMATATTRIBFVARBPROC __wglewGetPixelFormatAttribfvARB;
WGLEW_EXPORT PFNWGLGETPIXELFORMATATTRIBIVARBPROC __wglewGetPixelFormatAttribivARB;

WGLEW_EXPORT PFNWGLBINDTEXIMAGEARBPROC __wglewBindTexImageARB;
WGLEW_EXPORT PFNWGLRELEASETEXIMAGEARBPROC __wglewReleaseTexImageARB;
WGLEW_EXPORT PFNWGLSETPBUFFERATTRIBARBPROC __wglewSetPbufferAttribARB;

WGLEW_EXPORT PFNWGLBINDDISPLAYCOLORTABLEEXTPROC __wglewBindDisplayColorTableEXT;
WGLEW_EXPORT PFNWGLCREATEDISPLAYCOLORTABLEEXTPROC __wglewCreateDisplayColorTableEXT;
WGLEW_EXPORT PFNWGLDESTROYDISPLAYCOLORTABLEEXTPROC __wglewDestroyDisplayColorTableEXT;
WGLEW_EXPORT PFNWGLLOADDISPLAYCOLORTABLEEXTPROC __wglewLoadDisplayColorTableEXT;

WGLEW_EXPORT PFNWGLGETEXTENSIONSSTRINGEXTPROC __wglewGetExtensionsStringEXT;

WGLEW_EXPORT PFNWGLGETCURRENTREADDCEXTPROC __wglewGetCurrentReadDCEXT;
WGLEW_EXPORT PFNWGLMAKECONTEXTCURRENTEXTPROC __wglewMakeContextCurrentEXT;

WGLEW_EXPORT PFNWGLCREATEPBUFFEREXTPROC __wglewCreatePbufferEXT;
WGLEW_EXPORT PFNWGLDESTROYPBUFFEREXTPROC __wglewDestroyPbufferEXT;
WGLEW_EXPORT PFNWGLGETPBUFFERDCEXTPROC __wglewGetPbufferDCEXT;
WGLEW_EXPORT PFNWGLQUERYPBUFFEREXTPROC __wglewQueryPbufferEXT;
WGLEW_EXPORT PFNWGLRELEASEPBUFFERDCEXTPROC __wglewReleasePbufferDCEXT;

WGLEW_EXPORT PFNWGLCHOOSEPIXELFORMATEXTPROC __wglewChoosePixelFormatEXT;
WGLEW_EXPORT PFNWGLGETPIXELFORMATATTRIBFVEXTPROC __wglewGetPixelFormatAttribfvEXT;
WGLEW_EXPORT PFNWGLGETPIXELFORMATATTRIBIVEXTPROC __wglewGetPixelFormatAttribivEXT;

WGLEW_EXPORT PFNWGLGETSWAPINTERVALEXTPROC __wglewGetSwapIntervalEXT;
WGLEW_EXPORT PFNWGLSWAPINTERVALEXTPROC __wglewSwapIntervalEXT;

WGLEW_EXPORT PFNWGLGETDIGITALVIDEOPARAMETERSI3DPROC __wglewGetDigitalVideoParametersI3D;
WGLEW_EXPORT PFNWGLSETDIGITALVIDEOPARAMETERSI3DPROC __wglewSetDigitalVideoParametersI3D;

WGLEW_EXPORT PFNWGLGETGAMMATABLEI3DPROC __wglewGetGammaTableI3D;
WGLEW_EXPORT PFNWGLGETGAMMATABLEPARAMETERSI3DPROC __wglewGetGammaTableParametersI3D;
WGLEW_EXPORT PFNWGLSETGAMMATABLEI3DPROC __wglewSetGammaTableI3D;
WGLEW_EXPORT PFNWGLSETGAMMATABLEPARAMETERSI3DPROC __wglewSetGammaTableParametersI3D;

WGLEW_EXPORT PFNWGLDISABLEGENLOCKI3DPROC __wglewDisableGenlockI3D;
WGLEW_EXPORT PFNWGLENABLEGENLOCKI3DPROC __wglewEnableGenlockI3D;
WGLEW_EXPORT PFNWGLGENLOCKSAMPLERATEI3DPROC __wglewGenlockSampleRateI3D;
WGLEW_EXPORT PFNWGLGENLOCKSOURCEDELAYI3DPROC __wglewGenlockSourceDelayI3D;
WGLEW_EXPORT PFNWGLGENLOCKSOURCEEDGEI3DPROC __wglewGenlockSourceEdgeI3D;
WGLEW_EXPORT PFNWGLGENLOCKSOURCEI3DPROC __wglewGenlockSourceI3D;
WGLEW_EXPORT PFNWGLGETGENLOCKSAMPLERATEI3DPROC __wglewGetGenlockSampleRateI3D;
WGLEW_EXPORT PFNWGLGETGENLOCKSOURCEDELAYI3DPROC __wglewGetGenlockSourceDelayI3D;
WGLEW_EXPORT PFNWGLGETGENLOCKSOURCEEDGEI3DPROC __wglewGetGenlockSourceEdgeI3D;
WGLEW_EXPORT PFNWGLGETGENLOCKSOURCEI3DPROC __wglewGetGenlockSourceI3D;
WGLEW_EXPORT PFNWGLISENABLEDGENLOCKI3DPROC __wglewIsEnabledGenlockI3D;
WGLEW_EXPORT PFNWGLQUERYGENLOCKMAXSOURCEDELAYI3DPROC __wglewQueryGenlockMaxSourceDelayI3D;

WGLEW_EXPORT PFNWGLASSOCIATEIMAGEBUFFEREVENTSI3DPROC __wglewAssociateImageBufferEventsI3D;
WGLEW_EXPORT PFNWGLCREATEIMAGEBUFFERI3DPROC __wglewCreateImageBufferI3D;
WGLEW_EXPORT PFNWGLDESTROYIMAGEBUFFERI3DPROC __wglewDestroyImageBufferI3D;
WGLEW_EXPORT PFNWGLRELEASEIMAGEBUFFEREVENTSI3DPROC __wglewReleaseImageBufferEventsI3D;

WGLEW_EXPORT PFNWGLDISABLEFRAMELOCKI3DPROC __wglewDisableFrameLockI3D;
WGLEW_EXPORT PFNWGLENABLEFRAMELOCKI3DPROC __wglewEnableFrameLockI3D;
WGLEW_EXPORT PFNWGLISENABLEDFRAMELOCKI3DPROC __wglewIsEnabledFrameLockI3D;
WGLEW_EXPORT PFNWGLQUERYFRAMELOCKMASTERI3DPROC __wglewQueryFrameLockMasterI3D;

WGLEW_EXPORT PFNWGLBEGINFRAMETRACKINGI3DPROC __wglewBeginFrameTrackingI3D;
WGLEW_EXPORT PFNWGLENDFRAMETRACKINGI3DPROC __wglewEndFrameTrackingI3D;
WGLEW_EXPORT PFNWGLGETFRAMEUSAGEI3DPROC __wglewGetFrameUsageI3D;
WGLEW_EXPORT PFNWGLQUERYFRAMETRACKINGI3DPROC __wglewQueryFrameTrackingI3D;

WGLEW_EXPORT PFNWGLCOPYIMAGESUBDATANVPROC __wglewCopyImageSubDataNV;

WGLEW_EXPORT PFNWGLCREATEAFFINITYDCNVPROC __wglewCreateAffinityDCNV;
WGLEW_EXPORT PFNWGLDELETEDCNVPROC __wglewDeleteDCNV;
WGLEW_EXPORT PFNWGLENUMGPUDEVICESNVPROC __wglewEnumGpuDevicesNV;
WGLEW_EXPORT PFNWGLENUMGPUSFROMAFFINITYDCNVPROC __wglewEnumGpusFromAffinityDCNV;
WGLEW_EXPORT PFNWGLENUMGPUSNVPROC __wglewEnumGpusNV;

WGLEW_EXPORT PFNWGLBINDVIDEODEVICENVPROC __wglewBindVideoDeviceNV;
WGLEW_EXPORT PFNWGLENUMERATEVIDEODEVICESNVPROC __wglewEnumerateVideoDevicesNV;
WGLEW_EXPORT PFNWGLQUERYCURRENTCONTEXTNVPROC __wglewQueryCurrentContextNV;

WGLEW_EXPORT PFNWGLBINDSWAPBARRIERNVPROC __wglewBindSwapBarrierNV;
WGLEW_EXPORT PFNWGLJOINSWAPGROUPNVPROC __wglewJoinSwapGroupNV;
WGLEW_EXPORT PFNWGLQUERYFRAMECOUNTNVPROC __wglewQueryFrameCountNV;
WGLEW_EXPORT PFNWGLQUERYMAXSWAPGROUPSNVPROC __wglewQueryMaxSwapGroupsNV;
WGLEW_EXPORT PFNWGLQUERYSWAPGROUPNVPROC __wglewQuerySwapGroupNV;
WGLEW_EXPORT PFNWGLRESETFRAMECOUNTNVPROC __wglewResetFrameCountNV;

WGLEW_EXPORT PFNWGLALLOCATEMEMORYNVPROC __wglewAllocateMemoryNV;
WGLEW_EXPORT PFNWGLFREEMEMORYNVPROC __wglewFreeMemoryNV;

WGLEW_EXPORT PFNWGLBINDVIDEOIMAGENVPROC __wglewBindVideoImageNV;
WGLEW_EXPORT PFNWGLGETVIDEODEVICENVPROC __wglewGetVideoDeviceNV;
WGLEW_EXPORT PFNWGLGETVIDEOINFONVPROC __wglewGetVideoInfoNV;
WGLEW_EXPORT PFNWGLRELEASEVIDEODEVICENVPROC __wglewReleaseVideoDeviceNV;
WGLEW_EXPORT PFNWGLRELEASEVIDEOIMAGENVPROC __wglewReleaseVideoImageNV;
WGLEW_EXPORT PFNWGLSENDPBUFFERTOVIDEONVPROC __wglewSendPbufferToVideoNV;

WGLEW_EXPORT PFNWGLGETMSCRATEOMLPROC __wglewGetMscRateOML;
WGLEW_EXPORT PFNWGLGETSYNCVALUESOMLPROC __wglewGetSyncValuesOML;
WGLEW_EXPORT PFNWGLSWAPBUFFERSMSCOMLPROC __wglewSwapBuffersMscOML;
WGLEW_EXPORT PFNWGLSWAPLAYERBUFFERSMSCOMLPROC __wglewSwapLayerBuffersMscOML;
WGLEW_EXPORT PFNWGLWAITFORMSCOMLPROC __wglewWaitForMscOML;
WGLEW_EXPORT PFNWGLWAITFORSBCOMLPROC __wglewWaitForSbcOML;
WGLEW_EXPORT GLboolean __WGLEW_3DFX_multisample;
WGLEW_EXPORT GLboolean __WGLEW_3DL_stereo_control;
WGLEW_EXPORT GLboolean __WGLEW_AMD_gpu_association;
WGLEW_EXPORT GLboolean __WGLEW_ARB_buffer_region;
WGLEW_EXPORT GLboolean __WGLEW_ARB_create_context;
WGLEW_EXPORT GLboolean __WGLEW_ARB_create_context_profile;
WGLEW_EXPORT GLboolean __WGLEW_ARB_extensions_string;
WGLEW_EXPORT GLboolean __WGLEW_ARB_framebuffer_sRGB;
WGLEW_EXPORT GLboolean __WGLEW_ARB_make_current_read;
WGLEW_EXPORT GLboolean __WGLEW_ARB_multisample;
WGLEW_EXPORT GLboolean __WGLEW_ARB_pbuffer;
WGLEW_EXPORT GLboolean __WGLEW_ARB_pixel_format;
WGLEW_EXPORT GLboolean __WGLEW_ARB_pixel_format_float;
WGLEW_EXPORT GLboolean __WGLEW_ARB_render_texture;
WGLEW_EXPORT GLboolean __WGLEW_ATI_pixel_format_float;
WGLEW_EXPORT GLboolean __WGLEW_ATI_render_texture_rectangle;
WGLEW_EXPORT GLboolean __WGLEW_EXT_depth_float;
WGLEW_EXPORT GLboolean __WGLEW_EXT_display_color_table;
WGLEW_EXPORT GLboolean __WGLEW_EXT_extensions_string;
WGLEW_EXPORT GLboolean __WGLEW_EXT_framebuffer_sRGB;
WGLEW_EXPORT GLboolean __WGLEW_EXT_make_current_read;
WGLEW_EXPORT GLboolean __WGLEW_EXT_multisample;
WGLEW_EXPORT GLboolean __WGLEW_EXT_pbuffer;
WGLEW_EXPORT GLboolean __WGLEW_EXT_pixel_format;
WGLEW_EXPORT GLboolean __WGLEW_EXT_pixel_format_packed_float;
WGLEW_EXPORT GLboolean __WGLEW_EXT_swap_control;
WGLEW_EXPORT GLboolean __WGLEW_I3D_digital_video_control;
WGLEW_EXPORT GLboolean __WGLEW_I3D_gamma;
WGLEW_EXPORT GLboolean __WGLEW_I3D_genlock;
WGLEW_EXPORT GLboolean __WGLEW_I3D_image_buffer;
WGLEW_EXPORT GLboolean __WGLEW_I3D_swap_frame_lock;
WGLEW_EXPORT GLboolean __WGLEW_I3D_swap_frame_usage;
WGLEW_EXPORT GLboolean __WGLEW_NV_copy_image;
WGLEW_EXPORT GLboolean __WGLEW_NV_float_buffer;
WGLEW_EXPORT GLboolean __WGLEW_NV_gpu_affinity;
WGLEW_EXPORT GLboolean __WGLEW_NV_multisample_coverage;
WGLEW_EXPORT GLboolean __WGLEW_NV_present_video;
WGLEW_EXPORT GLboolean __WGLEW_NV_render_depth_texture;
WGLEW_EXPORT GLboolean __WGLEW_NV_render_texture_rectangle;
WGLEW_EXPORT GLboolean __WGLEW_NV_swap_group;
WGLEW_EXPORT GLboolean __WGLEW_NV_vertex_array_range;
WGLEW_EXPORT GLboolean __WGLEW_NV_video_output;
WGLEW_EXPORT GLboolean __WGLEW_OML_sync_control;

#ifdef GLEW_MX
}; /* WGLEWContextStruct */
#endif /* GLEW_MX */

/* ------------------------------------------------------------------------- */

#ifdef GLEW_MX

typedef struct WGLEWContextStruct WGLEWContext;
GLEWAPI GLenum wglewContextInit (WGLEWContext* ctx);
GLEWAPI GLboolean wglewContextIsSupported (WGLEWContext* ctx, const char* name);

#define wglewInit() wglewContextInit(wglewGetContext())
#define wglewIsSupported(x) wglewContextIsSupported(wglewGetContext(), x)

#define WGLEW_GET_VAR(x) (*(const GLboolean*)&(wglewGetContext()->x))
#define WGLEW_GET_FUN(x) wglewGetContext()->x

#else /* GLEW_MX */

#define WGLEW_GET_VAR(x) (*(const GLboolean*)&x)
#define WGLEW_GET_FUN(x) x

GLEWAPI GLboolean wglewIsSupported (const char* name);

#endif /* GLEW_MX */

GLEWAPI GLboolean wglewGetExtension (const char* name);

#ifdef __cplusplus
}
#endif

#undef GLEWAPI

#endif /* __wglew_h__ */


```

## Source file: include/IL/ilu.h

{
  "path": "include/IL/ilu.h",
  "blob_sha": "49871d79fdf98cb6b4e99423d16caf7d82669044",
  "sha256": "c8c732d54a17a2f1d6a253a0162f251d493b7f44a26d9d413eaa595474ef56a4",
  "lines": 195,
  "bytes": 6861,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 4,
    "lip": 1
  },
  "url": "https://github.com/sp4cerat/Skinned-Skeletal-Animation-in-OpenGL/blob/2436debd125b6ef946c99b8ca9f7f5b846aa8e60/include/IL/ilu.h",
  "function_signature_candidates": []
}

```text

//-----------------------------------------------------------------------------
//
// ImageLib Utility Sources
// Copyright (C) 2000-2009 by Denton Woods
// Last modified: 03/07/2009
//
// Filename: IL/ilu.h
//
// Description: The main include file for ILU
//
//-----------------------------------------------------------------------------

// Doxygen comment
/*! \file ilu.h
    The main include file for ILU
*/

#ifndef __ilu_h_
#ifndef __ILU_H__

#define __ilu_h_
#define __ILU_H__

#include <IL/il.h>

#ifdef __cplusplus
extern "C" {
#endif


#ifdef _WIN32
	#if (defined(IL_USE_PRAGMA_LIBS)) && (!defined(_IL_BUILD_LIBRARY))
		#if defined(_MSC_VER) || defined(__BORLANDC__)
			#pragma comment(lib, "ILU.lib")
		#endif
	#endif
#endif


#define ILU_VERSION_1_7_8 1
#define ILU_VERSION       178


#define ILU_FILTER         0x2600
#define ILU_NEAREST        0x2601
#define ILU_LINEAR         0x2602
#define ILU_BILINEAR       0x2603
#define ILU_SCALE_BOX      0x2604
#define ILU_SCALE_TRIANGLE 0x2605
#define ILU_SCALE_BELL     0x2606
#define ILU_SCALE_BSPLINE  0x2607
#define ILU_SCALE_LANCZOS3 0x2608
#define ILU_SCALE_MITCHELL 0x2609


// Error types
#define ILU_INVALID_ENUM      0x0501
#define ILU_OUT_OF_MEMORY     0x0502
#define ILU_INTERNAL_ERROR    0x0504
#define ILU_INVALID_VALUE     0x0505
#define ILU_ILLEGAL_OPERATION 0x0506
#define ILU_INVALID_PARAM     0x0509


// Values
#define ILU_PLACEMENT          0x0700
#define ILU_LOWER_LEFT         0x0701
#define ILU_LOWER_RIGHT        0x0702
#define ILU_UPPER_LEFT         0x0703
#define ILU_UPPER_RIGHT        0x0704
#define ILU_CENTER             0x0705
#define ILU_CONVOLUTION_MATRIX 0x0710
  
#define ILU_VERSION_NUM IL_VERSION_NUM
#define ILU_VENDOR      IL_VENDOR


// Languages
#define ILU_ENGLISH            0x0800
#define ILU_ARABIC             0x0801
#define ILU_DUTCH              0x0802
#define ILU_JAPANESE           0x0803
#define ILU_SPANISH            0x0804
#define ILU_GERMAN             0x0805
#define ILU_FRENCH             0x0806


// Filters
/*
#define ILU_FILTER_BLUR         0x0803
#define ILU_FILTER_GAUSSIAN_3x3 0x0804
#define ILU_FILTER_GAUSSIAN_5X5 0x0805
#define ILU_FILTER_EMBOSS1      0x0807
#define ILU_FILTER_EMBOSS2      0x0808
#define ILU_FILTER_LAPLACIAN1   0x080A
#define ILU_FILTER_LAPLACIAN2   0x080B
#define ILU_FILTER_LAPLACIAN3   0x080C
#define ILU_FILTER_LAPLACIAN4   0x080D
#define ILU_FILTER_SHARPEN1     0x080E
#define ILU_FILTER_SHARPEN2     0x080F
#define ILU_FILTER_SHARPEN3     0x0810
*/


typedef struct ILinfo
{
	ILuint  Id;         // the image's id
	ILubyte *Data;      // the image's data
	ILuint  Width;      // the image's width
	ILuint  Height;     // the image's height
	ILuint  Depth;      // the image's depth
	ILubyte Bpp;        // bytes per pixel (not bits) of the image
	ILuint  SizeOfData; // the total size of the data (in bytes)
	ILenum  Format;     // image format (in IL enum style)
	ILenum  Type;       // image type (in IL enum style)
	ILenum  Origin;     // origin of the image
	ILubyte *Palette;   // the image's palette
	ILenum  PalType;    // palette type
	ILuint  PalSize;    // palette size
	ILenum  CubeFlags;  // flags for what cube map sides are present
	ILuint  NumNext;    // number of images following
	ILuint  NumMips;    // number of mipmaps
	ILuint  NumLayers;  // number of layers
} ILinfo;


typedef struct ILpointf {
	ILfloat x;
	ILfloat y;
} ILpointf;

typedef struct ILpointi {
	ILint x;
	ILint y;
} ILpointi;

ILAPI ILboolean      ILAPIENTRY iluAlienify(void);
ILAPI ILboolean      ILAPIENTRY iluBlurAvg(ILuint Iter);
ILAPI ILboolean      ILAPIENTRY iluBlurGaussian(ILuint Iter);
ILAPI ILboolean      ILAPIENTRY iluBuildMipmaps(void);
ILAPI ILuint         ILAPIENTRY iluColoursUsed(void);
ILAPI ILboolean      ILAPIENTRY iluCompareImage(ILuint Comp);
ILAPI ILboolean      ILAPIENTRY iluContrast(ILfloat Contrast);
ILAPI ILboolean      ILAPIENTRY iluCrop(ILuint XOff, ILuint YOff, ILuint ZOff, ILuint Width, ILuint Height, ILuint Depth);
ILAPI void           ILAPIENTRY iluDeleteImage(ILuint Id); // Deprecated
ILAPI ILboolean      ILAPIENTRY iluEdgeDetectE(void);
ILAPI ILboolean      ILAPIENTRY iluEdgeDetectP(void);
ILAPI ILboolean      ILAPIENTRY iluEdgeDetectS(void);
ILAPI ILboolean      ILAPIENTRY iluEmboss(void);
ILAPI ILboolean      ILAPIENTRY iluEnlargeCanvas(ILuint Width, ILuint Height, ILuint Depth);
ILAPI ILboolean      ILAPIENTRY iluEnlargeImage(ILfloat XDim, ILfloat YDim, ILfloat ZDim);
ILAPI ILboolean      ILAPIENTRY iluEqualize(void);
ILAPI ILconst_string 		 ILAPIENTRY iluErrorString(ILenum Error);
ILAPI ILboolean      ILAPIENTRY iluConvolution(ILint *matrix, ILint scale, ILint bias);
ILAPI ILboolean      ILAPIENTRY iluFlipImage(void);
ILAPI ILboolean      ILAPIENTRY iluGammaCorrect(ILfloat Gamma);
ILAPI ILuint         ILAPIENTRY iluGenImage(void); // Deprecated
ILAPI void           ILAPIENTRY iluGetImageInfo(ILinfo *Info);
ILAPI ILint          ILAPIENTRY iluGetInteger(ILenum Mode);
ILAPI void           ILAPIENTRY iluGetIntegerv(ILenum Mode, ILint *Param);
ILAPI ILstring 		 ILAPIENTRY iluGetString(ILenum StringName);
ILAPI void           ILAPIENTRY iluImageParameter(ILenum PName, ILenum Param);
ILAPI void           ILAPIENTRY iluInit(void);
ILAPI ILboolean      ILAPIENTRY iluInvertAlpha(void);
ILAPI ILuint         ILAPIENTRY iluLoadImage(ILconst_string FileName);
ILAPI ILboolean      ILAPIENTRY iluMirror(void);
ILAPI ILboolean      ILAPIENTRY iluNegative(void);
ILAPI ILboolean      ILAPIENTRY iluNoisify(ILclampf Tolerance);
ILAPI ILboolean      ILAPIENTRY iluPixelize(ILuint PixSize);
ILAPI void           ILAPIENTRY iluRegionfv(ILpointf *Points, ILuint n);
ILAPI void           ILAPIENTRY iluRegioniv(ILpointi *Points, ILuint n);
ILAPI ILboolean      ILAPIENTRY iluReplaceColour(ILubyte Red, ILubyte Green, ILubyte Blue, ILfloat Tolerance);
ILAPI ILboolean      ILAPIENTRY iluRotate(ILfloat Angle);
ILAPI ILboolean      ILAPIENTRY iluRotate3D(ILfloat x, ILfloat y, ILfloat z, ILfloat Angle);
ILAPI ILboolean      ILAPIENTRY iluSaturate1f(ILfloat Saturation);
ILAPI ILboolean      ILAPIENTRY iluSaturate4f(ILfloat r, ILfloat g, ILfloat b, ILfloat Saturation);
ILAPI ILboolean      ILAPIENTRY iluScale(ILuint Width, ILuint Height, ILuint Depth);
ILAPI ILboolean      ILAPIENTRY iluScaleAlpha(ILfloat scale);
ILAPI ILboolean      ILAPIENTRY iluScaleColours(ILfloat r, ILfloat g, ILfloat b);
ILAPI ILboolean      ILAPIENTRY iluSetLanguage(ILenum Language);
ILAPI ILboolean      ILAPIENTRY iluSharpen(ILfloat Factor, ILuint Iter);
ILAPI ILboolean      ILAPIENTRY iluSwapColours(void);
ILAPI ILboolean      ILAPIENTRY iluWave(ILfloat Angle);

#define iluColorsUsed   iluColoursUsed
#define iluSwapColors   iluSwapColours
#define iluReplaceColor iluReplaceColour
#define iluScaleColor   iluScaleColour

#ifdef __cplusplus
}
#endif

#endif // __ILU_H__
#endif // __ilu_h_


```
