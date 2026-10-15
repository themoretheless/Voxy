# Facial rendering source scan: tokoik/imsss

Repository: https://github.com/tokoik/imsss

Pinned source revision: b93ac19a736a51caf995ccf355ea0ff863c1bdba

Discovery topic: subsurface scattering

Description: Image Based Screen Space Subsurface Scattering Sample

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"sss": ["imsss.sln", "imsss.vcxproj", "imsss.vcxproj.filters", "imsss.xcodeproj/project.pbxproj"], "shader": ["GgObjShader.cpp", "GgObjShader.h", "GgPointShader.cpp", "GgPointShader.h"]}

## Source file: GgObjShader.h

{
  "path": "GgObjShader.h",
  "blob_sha": "311a1e8600cde5b15b5dabc66febaff2b30ce9b7",
  "sha256": "c75bd403a7cdaf70aab93d57fe2c3adcd1a8a543fe55fa1c49d452ded9c3f0cd",
  "lines": 137,
  "bytes": 3903,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 12
  },
  "url": "https://github.com/tokoik/imsss/blob/b93ac19a736a51caf995ccf355ea0ff863c1bdba/GgObjShader.h",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "void loadNormalMatrix(const GgMatrix &m) { m.get(g); }"
    },
    {
      "line": 70,
      "text": "virtual void use(void) const;"
    },
    {
      "line": 73,
      "text": "virtual void unuse(void) const;"
    },
    {
      "line": 76,
      "text": "void setLightPosition(GLfloat x, GLfloat y, GLfloat z, GLfloat w = 1.0f)"
    },
    {
      "line": 83,
      "text": "void setLightPosition(const GLfloat *pos)"
    },
    {
      "line": 89,
      "text": "void setLightAmbient(GLfloat r, GLfloat g, GLfloat b, GLfloat a = 1.0f)"
    },
    {
      "line": 96,
      "text": "void setLightAmbient(const GLfloat *amb)"
    },
    {
      "line": 102,
      "text": "void setLightDiffuse(GLfloat r, GLfloat g, GLfloat b, GLfloat a = 1.0f)"
    },
    {
      "line": 109,
      "text": "void setLightDiffuse(const GLfloat *diff)"
    },
    {
      "line": 115,
      "text": "void setLightSpecular(GLfloat r, GLfloat g, GLfloat b, GLfloat a = 1.0f)"
    },
    {
      "line": 122,
      "text": "void setLightSpecular(const GLfloat *spec)"
    },
    {
      "line": 128,
      "text": "void setMaterial(const GLfloat *amb, const GLfloat *diff, const GLfloat *spec, GLfloat shi);"
    },
    {
      "line": 131,
      "text": "virtual void loadMatrix(const GgMatrix &mp, const GgMatrix &mw);"
    },
    {
      "line": 132,
      "text": "virtual void loadMatrix(const GLfloat *mp, const GLfloat *mw);"
    }
  ]
}

```text

﻿#ifndef __GG_SIMPLESHADER_H__
#define __GG_SIMPLESHADER_H__

/*
** Alias OBJ 形式の形状データの陰影付け
*/
#include "GgPointShader.h"

namespace gg
{
  class GgObjShader
    : public GgPointShader
  {
    // 光源
    struct
    {
      GLfloat pos[4];   // 光源位置
      GLfloat amb[4];   // 光源強度の環境光成分
      GLfloat diff[4];  // 光源強度の拡散反射光成分
      GLfloat spec[4];  // 光源強度の鏡面反射光成分
    } l;

    // 変換
    struct
    {
      GLfloat g[16];    // モデルビュー変換の法線変換行列
      void loadNormalMatrix(const GgMatrix &m) { m.get(g); }
    } m;

    // 場所
    struct
    {
      GLint pl;         // 光源位置の uniform 変数の場所
      GLint lamb;       // 光源強度の環境光成分の uniform 変数の場所
      GLint ldiff;      // 光源強度の拡散反射光成分の uniform 変数の場所
      GLint lspec;      // 光源強度の鏡面反射光成分の uniform 変数の場所
      GLint kamb;       // 環境光の反射係数の uniform 変数の場所
      GLint kdiff;      // 拡散反射係数の uniform 変数の場所
      GLint kspec;      // 鏡面反射係数の uniform 変数の場所
      GLint kshi;       // 輝き係数の uniform 変数の場所
      GLint mg;         // モデルビュー変換の法線変換行列の uniform 変数の場所
    } loc;

  public:

    // デストラクタ
    virtual ~GgObjShader(void) {}

    // コンストラクタ
    GgObjShader(void) {}
    GgObjShader(const char *vert, const char *frag = 0,
      const char *geom = 0, GLint nvarying = 0, const char **varyings = 0);
    GgObjShader(const GgObjShader &o)
      : GgPointShader(o), l(o.l), m(o.m), loc(o.loc) {}

    // 代入
    GgObjShader &operator=(const GgObjShader &o)
    {
      if (&o != this)
      {
        GgPointShader::operator=(o);
        l = o.l;
        m = o.m;
        loc = o.loc;
      }
      return *this;
    }

    // シェーダの使用開始
    virtual void use(void) const;

    // シェーダの使用終了
    virtual void unuse(void) const;

    // 光源の位置
    void setLightPosition(GLfloat x, GLfloat y, GLfloat z, GLfloat w = 1.0f)
    {
      l.pos[0] = x;
      l.pos[1] = y;
      l.pos[2] = z;
      l.pos[3] = w;
    }
    void setLightPosition(const GLfloat *pos)
    {
      setLightPosition(pos[0], pos[1], pos[2], pos[3]);
    }

    // 光源強度の環境光成分
    void setLightAmbient(GLfloat r, GLfloat g, GLfloat b, GLfloat a = 1.0f)
    {
      l.amb[0] = r;
      l.amb[1] = g;
      l.amb[2] = b;
      l.amb[3] = a;
    }
    void setLightAmbient(const GLfloat *amb)
    {
      setLightAmbient(amb[0], amb[1], amb[2], amb[3]);
    }

    // 光源強度の拡散反射光成分
    void setLightDiffuse(GLfloat r, GLfloat g, GLfloat b, GLfloat a = 1.0f)
    {
      l.diff[0] = r;
      l.diff[1] = g;
      l.diff[2] = b;
      l.diff[3] = a;
    }
    void setLightDiffuse(const GLfloat *diff)
    {
      setLightDiffuse(diff[0], diff[1], diff[2], diff[3]);
    }

    // 光源強度の鏡面反射光成分
    void setLightSpecular(GLfloat r, GLfloat g, GLfloat b, GLfloat a = 1.0f)
    {
      l.spec[0] = r;
      l.spec[1] = g;
      l.spec[2] = b;
      l.spec[3] = a;
    }
    void setLightSpecular(const GLfloat *spec)
    {
      setLightSpecular(spec[0], spec[1], spec[2], spec[3]);
    }

    // 環境光に対する反射係数
    void setMaterial(const GLfloat *amb, const GLfloat *diff, const GLfloat *spec, GLfloat shi);

    // 変換行列の設定
    virtual void loadMatrix(const GgMatrix &mp, const GgMatrix &mw);
    virtual void loadMatrix(const GLfloat *mp, const GLfloat *mw);
  };

}

#endif


```

## Source file: GgObjShader.cpp

{
  "path": "GgObjShader.cpp",
  "blob_sha": "505949040a7d76ad57128c9387708ab2c0739968",
  "sha256": "7e5886465a9e1bb9d3beb15f01b0f8a9bc6b403b083e43887ca5305debeefce7",
  "lines": 76,
  "bytes": 2308,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 11
  },
  "url": "https://github.com/tokoik/imsss/blob/b93ac19a736a51caf995ccf355ea0ff863c1bdba/GgObjShader.cpp",
  "function_signature_candidates": []
}

```text

﻿/*
** Alias OBJ 形式の形状データの陰影付け
*/
#include "GgObjShader.h"

// コンストラクタ
gg::GgObjShader::GgObjShader(const char *vert, const char *frag,
  const char *geom, GLint nvarying, const char **varyings)
  : GgPointShader(vert, frag, geom, nvarying, varyings)
{
  // プログラム名
  GLuint program = get();

  // 光源のパラメータの uniform 変数の場所
  loc.pl = glGetUniformLocation(program, "pl");
  loc.lamb = glGetUniformLocation(program, "lamb");
  loc.ldiff = glGetUniformLocation(program, "ldiff");
  loc.lspec = glGetUniformLocation(program, "lspec");

  // 材質のパラメータの uniform 変数の場所
  loc.kamb = glGetUniformLocation(program, "kamb");
  loc.kdiff = glGetUniformLocation(program, "kdiff");
  loc.kspec = glGetUniformLocation(program, "kspec");
  loc.kshi = glGetUniformLocation(program, "kshi");

  // 変換行列の uniform 変数の場所
  loc.mg = glGetUniformLocation(program, "mg");
}

// シェーダの使用開始
void gg::GgObjShader::use(void) const
{
  // 基底クラスのシェーダの設定を呼び出す
  GgPointShader::use();

  // 光源
  glUniform4fv(loc.pl, 1, l.pos);
  glUniform4fv(loc.lamb, 1, l.amb);
  glUniform4fv(loc.ldiff, 1, l.diff);
  glUniform4fv(loc.lspec, 1, l.spec);

  // 変換
  glUniformMatrix4fv(loc.mg, 1, GL_FALSE, m.g);
}

// シェーダの使用終了
void gg::GgObjShader::unuse(void) const
{
  // attribute 変数 pv をバッファオブジェクトから得ることを無効にする
  glDisableVertexAttribArray(1);

  // 基底クラスのシェーダの設定を呼び出す
  GgShader::unuse();
}

// 変換行列の設定
void gg::GgObjShader::loadMatrix(const GgMatrix &mp, const GgMatrix &mw)
{
  GgPointShader::loadMatrix(mp, mw);
  m.loadNormalMatrix(mw.normal());
}
void gg::GgObjShader::loadMatrix(const GLfloat *mp, const GLfloat *mw)
{
  GgMatrix tmp(mp), tmw(mw);
  loadMatrix(tmp, tmw);
}

// 環境光に対する反射係数
void gg::GgObjShader::setMaterial(const GLfloat *amb, const GLfloat *diff, const GLfloat *spec, GLfloat shi)
{
  glUniform4fv(loc.kamb, 1, amb);
  glUniform4fv(loc.kdiff, 1, diff);
  glUniform4fv(loc.kspec, 1, spec);
  glUniform1f(loc.kshi, shi);
}



```

## Source file: GgPointShader.h

{
  "path": "GgPointShader.h",
  "blob_sha": "38991997fa84b5460640609ca9cfec441ede6d8e",
  "sha256": "606f904c33d1608483dc3c236e1d49b3fc77c6931a0fd15f257a34fe1d1376d9",
  "lines": 68,
  "bytes": 1640,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 11
  },
  "url": "https://github.com/tokoik/imsss/blob/b93ac19a736a51caf995ccf355ea0ff863c1bdba/GgPointShader.h",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "void loadModelViewProjectionMatrix(const GgMatrix &m) { m.get(c); }"
    },
    {
      "line": 21,
      "text": "void loadModelViewMatrix(const GgMatrix &m) { m.get(w); }"
    },
    {
      "line": 56,
      "text": "virtual void use(void) const;"
    },
    {
      "line": 59,
      "text": "virtual void unuse(void) const;"
    },
    {
      "line": 62,
      "text": "virtual void loadMatrix(const GgMatrix &mp, const GgMatrix &mw);"
    },
    {
      "line": 63,
      "text": "virtual void loadMatrix(const GLfloat *mp, const GLfloat *mw);"
    }
  ]
}

```text

﻿#ifndef __GG_POINTSHADER_H__
#define __GG_POINTSHADER_H__

/*
** ポイント
*/
#include "gg.h"

namespace gg
{

  class GgPointShader
    : public GgShader
  {
    // 変換
    struct
    {
      GLfloat c[16];    // モデルビュー・投影変換行列
      void loadModelViewProjectionMatrix(const GgMatrix &m) { m.get(c); }
      GLfloat w[16];    // モデルビュー変換行列
      void loadModelViewMatrix(const GgMatrix &m) { m.get(w); }
    } m;

    // 場所
    struct
    {
      GLint mc;         // モデルビュー・投影変換行列の uniform 変数の場所
      GLint mw;         // モデルビュー変換行列の uniform 変数の場所
    } loc;

  public:

    // デストラクタ
    virtual ~GgPointShader(void) {}

    // コンストラクタ
    GgPointShader(void) {}
    GgPointShader(const char *vert, const char *frag = 0,
      const char *geom = 0, GLint nvarying = 0, const char **varyings = 0);
    GgPointShader(const GgPointShader &o)
      : GgShader(o), m(o.m), loc(o.loc) {}

    // 代入
    GgPointShader &operator=(const GgPointShader &o)
    {
      if (&o != this)
      {
        GgShader::operator=(o);
        m = o.m;
        loc = o.loc;
      }
      return *this;
    }

    // シェーダの使用開始
    virtual void use(void) const;

    // シェーダの使用終了
    virtual void unuse(void) const;

    // 変換行列の設定
    virtual void loadMatrix(const GgMatrix &mp, const GgMatrix &mw);
    virtual void loadMatrix(const GLfloat *mp, const GLfloat *mw);
  };

}

#endif


```

## Source file: GgPointShader.cpp

{
  "path": "GgPointShader.cpp",
  "blob_sha": "dfbe4e09c647c44bd4c55d5302b2725befd03166",
  "sha256": "c24129be944a8a8350c95574ac4f58302811a30d744f9705445769102fdced7a",
  "lines": 47,
  "bytes": 1249,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 9
  },
  "url": "https://github.com/tokoik/imsss/blob/b93ac19a736a51caf995ccf355ea0ff863c1bdba/GgPointShader.cpp",
  "function_signature_candidates": []
}

```text

﻿/*
** 点を球として表示する
*/
#include "GgPointShader.h"

// コンストラクタ
gg::GgPointShader::GgPointShader(const char *vert, const char *frag,
  const char *geom, GLint nvarying, const char **varyings)
  : GgShader(vert, frag, geom, nvarying, varyings)
{
  // プログラム名
  GLuint program = get();

  // 変換行列の uniform 変数の場所
  loc.mc = glGetUniformLocation(program, "mc");
  loc.mw = glGetUniformLocation(program, "mw");
}

// シェーダの使用開始
void gg::GgPointShader::use(void) const
{
  // 基底クラスのシェーダの設定を呼び出す
  GgShader::use();

  // 変換
  glUniformMatrix4fv(loc.mc, 1, GL_FALSE, m.c);
  glUniformMatrix4fv(loc.mw, 1, GL_FALSE, m.w);
}

// シェーダの使用終了
void gg::GgPointShader::unuse(void) const
{
  // 基底クラスのシェーダの設定を呼び出す
  GgShader::unuse();
}

// 変換行列の設定
void gg::GgPointShader::loadMatrix(const GgMatrix &mp, const GgMatrix &mw)
{
  m.loadModelViewProjectionMatrix(mp * mw);
  m.loadModelViewMatrix(mw);
}
void gg::GgPointShader::loadMatrix(const GLfloat *mp, const GLfloat *mw)
{
  GgMatrix tmp(mp), tmw(mw);
  loadMatrix(tmp, tmw);
}


```
