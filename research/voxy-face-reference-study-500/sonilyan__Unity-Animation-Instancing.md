# Facial rendering source scan: sonilyan/Unity-Animation-Instancing

Repository: https://github.com/sonilyan/Unity-Animation-Instancing

Pinned source revision: d6583ad8352ce52d0d12a63d990017f1203a9952

Discovery topic: skin shader

Description: bake animation info into a Texture2d and use shader skinning

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["Assets/anim_baker/anim.shader", "Assets/anim_baker/anim.shader.meta", "Assets/anim_baker/anim_bones.shader", "Assets/anim_baker/anim_bones.shader.meta", "Assets/flag.shader", "Assets/flag.shader.meta", "Assets/test.shader", "Assets/test.shader.meta", "Assets/world_pos.shader", "Assets/world_pos.shader.meta"]}

## Source file: Assets/flag.shader

{
  "path": "Assets/flag.shader",
  "blob_sha": "f471367978ecd0f5d39ee5ed94e3dbd68b4607b0",
  "sha256": "7c430fc601bff47304de1dd5075851f6ab82a0510ff62862d307c936b3d171c9",
  "lines": 60,
  "bytes": 986,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/sonilyan/Unity-Animation-Instancing/blob/d6583ad8352ce52d0d12a63d990017f1203a9952/Assets/flag.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Unlit/flag"
{
	Properties
	{
		_MainTex ("Texture", 2D) = "white" {}
	}
	SubShader
	{
		Tags { "RenderType"="Opaque" }
		LOD 100
		Cull off

		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			// make fog work
			#pragma multi_compile_fog
			
			#include "UnityCG.cginc"

			struct appdata
			{
				float4 vertex : POSITION;
				float2 uv : TEXCOORD0;
			};

			struct v2f
			{
				float2 uv : TEXCOORD0;
				UNITY_FOG_COORDS(1)
				float4 vertex : SV_POSITION;
			};

			sampler2D _MainTex;
			float4 _MainTex_ST;
			
			v2f vert (appdata v)
			{
				v2f o;
				o.vertex = UnityObjectToClipPos(v.vertex);
				o.vertex.x += sin(_Time.w + v.vertex.y);
				o.uv = TRANSFORM_TEX(v.uv, _MainTex);
				UNITY_TRANSFER_FOG(o,o.vertex);
				return o;
			}
			
			fixed4 frag (v2f i) : SV_Target
			{
				// sample the texture
				fixed4 col = tex2D(_MainTex, i.uv);
				// apply fog
				UNITY_APPLY_FOG(i.fogCoord, col);
				return col;
			}
			ENDCG
		}
	}
}


```

## Source file: Assets/world_pos.shader

{
  "path": "Assets/world_pos.shader",
  "blob_sha": "31b527cc3a7a82e4c5f21bdfd1e0973cbed99395",
  "sha256": "d3201a401e84383a6fcf662f1a760194c882a415c46396ed779dfebd8719a343",
  "lines": 70,
  "bytes": 1228,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/sonilyan/Unity-Animation-Instancing/blob/d6583ad8352ce52d0d12a63d990017f1203a9952/Assets/world_pos.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Unlit/world_pos"
{
	Properties
	{
		_MainTex ("Texture", 2D) = "white" {}
	}
	SubShader
	{
		Tags { "RenderType"="Opaque" }
		LOD 100

		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			// make fog work
			#pragma multi_compile_fog
			
			#include "UnityCG.cginc"

			struct appdata
			{
				float4 vertex : POSITION;
				float2 uv : TEXCOORD0;
			};

			struct v2f
			{
				float4 uv : TEXCOORD0;
				UNITY_FOG_COORDS(1)
				float4 vertex : SV_POSITION;
			};

			sampler2D _MainTex;
			float4 _MainTex_ST;
			
			v2f vert (appdata v)
			{
				v2f o;
				o.vertex = UnityObjectToClipPos(v.vertex);
				o.uv = o.vertex; 
				UNITY_TRANSFER_FOG(o,o.vertex);
				return o;
			}
			
			fixed4 frag (v2f i) : SV_Target
			{
				// apply fog
				UNITY_APPLY_FOG(i.fogCoord, col);
				
				float4 tmp = mul(unity_CameraInvProjection,i.uv / i.uv.w);
				tmp.z = -tmp.z;
				float4 tmp2 = mul(unity_CameraToWorld,tmp/tmp.w);
				if(tmp2.y > 0) {
				    if(tmp2.x > 0) {
				        if(tmp2.z > 0){
				            return float4(1,0,0,0);
				        }
				        return float4(0,1,0,0);
				    }
				    return float4(1,1,1,1);
				}
				        
				return float4(0,0,0,0);
			}
			ENDCG
		}
	}
}


```

## Source file: Assets/anim_baker/anim.shader

{
  "path": "Assets/anim_baker/anim.shader",
  "blob_sha": "592a631ecf3a191bd4b6d155b900084dc7c4110c",
  "sha256": "1a0baa234d998810ffe12df61b79ff5306c8d6c9f22b2b43bd6e5d18352b6cd9",
  "lines": 80,
  "bytes": 1668,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/sonilyan/Unity-Animation-Instancing/blob/d6583ad8352ce52d0d12a63d990017f1203a9952/Assets/anim_baker/anim.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Unlit/anim"
{
	Properties
	{
		_MainTex ("Texture", 2D) = "white" {}
		_AnimationTex ("Animation", 2D) = "white" {}
		_AnimHeight("Anim Height", Float) = 0
		_AnimWidth("Anim Width", Float) = 0
		_Fps("Fps",Float) = 30
	}
	SubShader
	{
		Tags { "RenderType"="Opaque" }
		LOD 100

		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			#pragma multi_compile_instancing
			
			#include "UnityCG.cginc"

			struct appdata
			{
				float4 vertex : POSITION;
				float2 uv : TEXCOORD0;
				UNITY_VERTEX_INPUT_INSTANCE_ID
			};

			struct v2f
			{
				float2 uv : TEXCOORD0;
				float4 vertex : SV_POSITION;
				UNITY_VERTEX_INPUT_INSTANCE_ID
			};
			
			UNITY_INSTANCING_BUFFER_START(Props)
                UNITY_DEFINE_INSTANCED_PROP(float, _TimeOffset)
            UNITY_INSTANCING_BUFFER_END(Props)

            float _Fps;
            float _AnimHeight;
            float _AnimWidth;
            
            sampler2D _AnimationTex;
			sampler2D _MainTex;
			float4 _MainTex_ST;
			
			v2f vert (appdata v,uint vid:SV_VertexID)
			{
				v2f o;
				UNITY_SETUP_INSTANCE_ID(v);
				
				float f = (_Time.y+UNITY_ACCESS_INSTANCED_PROP(Props, _TimeOffset)) * _Fps / _AnimHeight;
				fmod(f, 1.0);
				
				float4 off = tex2Dlod(_AnimationTex,float4(vid / _AnimWidth,f,0,0));
				
				off.xyz = off.xyz * 2 - 1;
				off *= off.w;
				
				v.vertex += float4(off.xyz,0);
				
				o.vertex = UnityObjectToClipPos(v.vertex);
				o.uv = TRANSFORM_TEX(v.uv, _MainTex);
				return o;
			}
			
			fixed4 frag (v2f i) : SV_Target
			{
			    UNITY_SETUP_INSTANCE_ID(i);
				fixed4 col = tex2D(_MainTex, i.uv);
				return col;
			}
			ENDCG
		}
	}
}


```

## Source file: Assets/anim_baker/anim_bones.shader

{
  "path": "Assets/anim_baker/anim_bones.shader",
  "blob_sha": "e52bca66210b0435c136ab021f5c8487bd2ad51c",
  "sha256": "f2c29b7197b44baeb6b6bc4ef5b17783646d0219a679c2e30b3af9069f91565b",
  "lines": 99,
  "bytes": 2454,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/sonilyan/Unity-Animation-Instancing/blob/d6583ad8352ce52d0d12a63d990017f1203a9952/Assets/anim_baker/anim_bones.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Unlit/anim_bones"
{
	Properties
	{
		_MainTex ("Texture", 2D) = "white" {}
		_AnimationTex ("Animation", 2D) = "black" {}
		_AnimHeight("Anim Height", Float) = 0
		_AnimWidth("Anim Width", Float) = 0
		_Fps("Fps",Float) = 30
	}
	SubShader
	{
		Tags { "RenderType"="Opaque" }
		LOD 100

		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			#pragma multi_compile_instancing
			
			#include "UnityCG.cginc"

			struct appdata
			{
				float4 vertex : POSITION;
				float2 uv : TEXCOORD0;
				float4 uv2 : TEXCOORD2;
				float4 color : COLOR;
				UNITY_VERTEX_INPUT_INSTANCE_ID
			};

			struct v2f
			{
				float2 uv : TEXCOORD0;
				float4 vertex : SV_POSITION;
				UNITY_VERTEX_INPUT_INSTANCE_ID
			};
			
			UNITY_INSTANCING_BUFFER_START(Props)
                UNITY_DEFINE_INSTANCED_PROP(float, _TimeOffset)
            UNITY_INSTANCING_BUFFER_END(Props)

            float _Fps;
            float _AnimHeight;
            float _AnimWidth;
            
            sampler2D _AnimationTex;
			sampler2D _MainTex;
			float4 _MainTex_ST;
			
			float4x4 GetMatrix(float boneIndex,float frame)
			{
			    float4 r1 = tex2Dlod(_AnimationTex,float4(boneIndex*4 / _AnimWidth,frame,0,0));
			    float4 r2 = tex2Dlod(_AnimationTex,float4((boneIndex*4+1) / _AnimWidth,frame,0,0));
			    float4 r3 = tex2Dlod(_AnimationTex,float4((boneIndex*4+2) / _AnimWidth,frame,0,0));
			    float4 r4 = tex2Dlod(_AnimationTex,float4((boneIndex*4+3) / _AnimWidth,frame,0,0));
			    
			    return float4x4(r1,r2,r3,r4);
			}
			
			v2f vert (appdata v)
			{
			    float4 boneIndex = v.color;
			    float4 boneWeight = v.uv2;
			    
				v2f o;
				UNITY_SETUP_INSTANCE_ID(v);
				
				float f = (_Time.y+UNITY_ACCESS_INSTANCED_PROP(Props, _TimeOffset)) * _Fps / _AnimHeight;
				fmod(f, 1.0);
				
				float4x4 Matrix = GetMatrix(boneIndex.x,f) * boneWeight.x; 
				
				if(boneWeight.y > 0)
				    Matrix += GetMatrix(boneIndex.y,f) * boneWeight.y;
				if(boneWeight.z > 0)
				    Matrix += GetMatrix(boneIndex.z,f) * boneWeight.z;
				if(boneWeight.w > 0)
				    Matrix += GetMatrix(boneIndex.w,f) * boneWeight.w;
				    
				o.vertex = mul(Matrix,v.vertex);
				
				o.vertex = UnityObjectToClipPos(o.vertex);
				o.uv = TRANSFORM_TEX(v.uv, _MainTex);
				return o;
			}
			
			fixed4 frag (v2f i) : SV_Target
			{
			    UNITY_SETUP_INSTANCE_ID(i);
				fixed4 col = tex2D(_MainTex, i.uv);
				return col;
			}
			ENDCG
		}
	}
}


```
