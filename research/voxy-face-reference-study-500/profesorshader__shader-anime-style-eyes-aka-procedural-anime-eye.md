# Facial rendering source scan: profesorshader/shader-anime-style-eyes-aka-procedural-anime-eye

Repository: https://github.com/profesorshader/shader-anime-style-eyes-aka-procedural-anime-eye

Pinned source revision: 52724f1ad234ccef8413ce2f21d09953f3bd3583

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["materials/eye1.tres", "materials/eye2.tres", "scripts/eye.gd", "scripts/eye.gd.uid", "shaders/anime_eyes_v1.gdshader", "shaders/anime_eyes_v1.gdshader.uid", "shaders/anime_eyes_v2.gdshader", "shaders/anime_eyes_v2.gdshader.uid", "shaders/anime_eyes_v3.gdshader", "shaders/anime_eyes_v3.gdshader.uid", "shaders/anime_eyes_v4.gdshader", "shaders/anime_eyes_v4.gdshader.uid", "shaders/anime_eyes_v5.gdshader", "shaders/anime_eyes_v5.gdshader.uid", "shaders/anime_eyes_v6.gdshader", "shaders/anime_eyes_v6.gdshader.uid", "shaders/anime_eyes_v7.gdshader", "shaders/anime_eyes_v7.gdshader.uid"], "shader": ["shaders/anime_eyes_v1.gdshader", "shaders/anime_eyes_v1.gdshader.uid", "shaders/anime_eyes_v2.gdshader", "shaders/anime_eyes_v2.gdshader.uid", "shaders/anime_eyes_v3.gdshader", "shaders/anime_eyes_v3.gdshader.uid", "shaders/anime_eyes_v4.gdshader", "shaders/anime_eyes_v4.gdshader.uid", "shaders/anime_eyes_v5.gdshader", "shaders/anime_eyes_v5.gdshader.uid", "shaders/anime_eyes_v6.gdshader", "shaders/anime_eyes_v6.gdshader.uid", "shaders/anime_eyes_v7.gdshader", "shaders/anime_eyes_v7.gdshader.uid"]}

## Source file: shaders/anime_eyes_v1.gdshader

{
  "path": "shaders/anime_eyes_v1.gdshader",
  "blob_sha": "aab5d23cd3038a28670d8eee91e3c65c21f96c7c",
  "sha256": "42d3b9b5a75ad87eb5d8242b8f6f41ca43ef5bd7fc9b78252bc3f2cdae9e9923",
  "lines": 24,
  "bytes": 675,
  "selection_score": 30,
  "keyword_line_counts": {
    "iris": 3,
    "shader": 1
  },
  "url": "https://github.com/profesorshader/shader-anime-style-eyes-aka-procedural-anime-eye/blob/52724f1ad234ccef8413ce2f21d09953f3bd3583/shaders/anime_eyes_v1.gdshader",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "float circle(vec2 position, float radius)"
    },
    {
      "line": 15,
      "text": "vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)"
    },
    {
      "line": 21,
      "text": "void fragment() {"
    }
  ]
}

```text

shader_type spatial;
render_mode unshaded;

uniform float g_radius = 0.2;
uniform vec3 g_iris_color_a : source_color = vec3(0.3, 1.0, 0.3);
uniform vec3 g_background_color : source_color = vec3(1.0, 1.0, 1.0);

// returns 0 if position is inside the circle, otherwise returns 1
float circle(vec2 position, float radius)
{
	return step(radius, length(position - vec2(0.5)));
}

// fills a circle
vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)
{
	float c = circle(position, radius);
	return mix(inside_color, outside_color, c);
}

void fragment() {
	// draw iris
	ALBEDO = fill_circle(UV, g_background_color, g_iris_color_a, g_radius); 
}


```

## Source file: shaders/anime_eyes_v2.gdshader

{
  "path": "shaders/anime_eyes_v2.gdshader",
  "blob_sha": "3323689943e094afbbb0db1eb6e732628edffdd1",
  "sha256": "844276bb12c5738232af68498ffd86e825af035bf9f7038d4584673e66e88daf",
  "lines": 33,
  "bytes": 962,
  "selection_score": 30,
  "keyword_line_counts": {
    "iris": 3,
    "shader": 1
  },
  "url": "https://github.com/profesorshader/shader-anime-style-eyes-aka-procedural-anime-eye/blob/52724f1ad234ccef8413ce2f21d09953f3bd3583/shaders/anime_eyes_v2.gdshader",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "float circle(vec2 position, float radius)"
    },
    {
      "line": 18,
      "text": "vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)"
    },
    {
      "line": 24,
      "text": "void fragment() {"
    }
  ]
}

```text

shader_type spatial;
render_mode unshaded;

uniform float g_radius = 0.2;
uniform float g_pupil_radius = 0.5; // relative size
uniform vec3 g_pupil_color : source_color = vec3(0.0, 0.0, 0.0);

uniform vec3 g_iris_color_a : source_color = vec3(0.3, 1.0, 0.3);
uniform vec3 g_background_color : source_color = vec3(1.0, 1.0, 1.0);

// returns 0 if position is inside the circle, otherwise returns 1
float circle(vec2 position, float radius)
{
	return step(radius, length(position - vec2(0.5)));
}

// fills a circle
vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)
{
	float c = circle(position, radius);
	return mix(inside_color, outside_color, c);
}

void fragment() {	
	// draw iris
	vec3 current_color = fill_circle(UV, g_background_color, g_iris_color_a, g_radius); 
	
	// draw pupil
	current_color = fill_circle(UV, current_color, g_pupil_color, g_radius * g_pupil_radius); 

	// set final color
	ALBEDO = current_color;
}


```

## Source file: shaders/anime_eyes_v3.gdshader

{
  "path": "shaders/anime_eyes_v3.gdshader",
  "blob_sha": "cee4eff04041e5ea30f0cccaa8aba4e01def0ea5",
  "sha256": "8863d0997421af460b964407b5486ca82d53a682f496ef489def5754e8925435",
  "lines": 46,
  "bytes": 1523,
  "selection_score": 30,
  "keyword_line_counts": {
    "iris": 3,
    "shader": 1
  },
  "url": "https://github.com/profesorshader/shader-anime-style-eyes-aka-procedural-anime-eye/blob/52724f1ad234ccef8413ce2f21d09953f3bd3583/shaders/anime_eyes_v3.gdshader",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "float circle(vec2 position, float radius)"
    },
    {
      "line": 20,
      "text": "vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)"
    },
    {
      "line": 27,
      "text": "vec3 stroke_circle(vec2 position, vec3 outside_color, vec3 inside_color, float circle_radius, float stroke_size)"
    },
    {
      "line": 34,
      "text": "void fragment() {"
    }
  ]
}

```text

shader_type spatial;
render_mode unshaded;

uniform float g_outline_radius : hint_range(0.0, 1.0, 0.001) = 0.01;
uniform float g_radius = 0.2;
uniform float g_pupil_radius = 0.5; // relative size
uniform vec3 g_outline_color : source_color = vec3(0.0, 0.0, 0.0);
uniform vec3 g_pupil_color : source_color = vec3(0.0, 0.0, 0.0);

uniform vec3 g_iris_color_a : source_color = vec3(0.3, 1.0, 0.3);
uniform vec3 g_background_color : source_color = vec3(1.0, 1.0, 1.0);

// returns 0 if position is inside the circle, otherwise returns 1
float circle(vec2 position, float radius)
{
	return step(radius, length(position - vec2(0.5)));
}

// fills a circle
vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)
{
	float c = circle(position, radius);
	return mix(inside_color, outside_color, c);
}

// strokes a circle
vec3 stroke_circle(vec2 position, vec3 outside_color, vec3 inside_color, float circle_radius, float stroke_size)
{
	float outside = circle(position, circle_radius);
	float inside = circle(position, circle_radius - stroke_size);
	return mix(outside_color, inside_color, inside - outside);
}

void fragment() {
	// draw iris
	vec3 current_color = fill_circle(UV, g_background_color, g_iris_color_a, g_radius); 
	
	// draw pupil
	current_color = fill_circle(UV, current_color, g_pupil_color, g_radius * g_pupil_radius); 
	
	// draw outline
	current_color = stroke_circle(UV, current_color, g_outline_color, g_radius, g_outline_radius);

	// set final color
	ALBEDO = current_color;
}


```

## Source file: shaders/anime_eyes_v4.gdshader

{
  "path": "shaders/anime_eyes_v4.gdshader",
  "blob_sha": "cc1f66d54b311823b57b37d4f77711582ec5fe8f",
  "sha256": "32bd9068c47d53ce05ed57a39b39602ccd1bea7f10b2b8afffac5af9fbee8c8f",
  "lines": 61,
  "bytes": 2295,
  "selection_score": 30,
  "keyword_line_counts": {
    "iris": 6,
    "shader": 1
  },
  "url": "https://github.com/profesorshader/shader-anime-style-eyes-aka-procedural-anime-eye/blob/52724f1ad234ccef8413ce2f21d09953f3bd3583/shaders/anime_eyes_v4.gdshader",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "float circle(vec2 position, float radius)"
    },
    {
      "line": 21,
      "text": "vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)"
    },
    {
      "line": 28,
      "text": "vec3 stroke_circle(vec2 position, vec3 outside_color, vec3 inside_color, float circle_radius, float stroke_size)"
    },
    {
      "line": 36,
      "text": "vec3 fill_masked_circle("
    },
    {
      "line": 44,
      "text": "void fragment() {"
    }
  ]
}

```text

shader_type spatial;
render_mode unshaded;

uniform vec3 g_iris_shadow = vec3(0.0, 0.25, 1.5); // xy = position offset, z = scale
uniform float g_outline_radius : hint_range(0.0, 1.0, 0.001) = 0.01;
uniform float g_radius = 0.2;
uniform float g_pupil_radius = 0.5; // relative size
uniform vec3 g_outline_color : source_color = vec3(0.0, 0.0, 0.0);
uniform vec3 g_pupil_color : source_color = vec3(0.0, 0.0, 0.0);
uniform vec3 g_iris_shadow_color : source_color = vec3(0.7, 0.7, 0.7); // shadow
uniform vec3 g_iris_color_a : source_color = vec3(0.3, 1.0, 0.3);
uniform vec3 g_background_color : source_color = vec3(1.0, 1.0, 1.0);

// returns 0 if position is inside the circle, otherwise returns 1
float circle(vec2 position, float radius)
{
	return step(radius, length(position - vec2(0.5)));
}

// fills a circle
vec3 fill_circle(vec2 position, vec3 outside_color, vec3 inside_color, float radius)
{
	float c = circle(position, radius);
	return mix(inside_color, outside_color, c);
}

// strokes a circle
vec3 stroke_circle(vec2 position, vec3 outside_color, vec3 inside_color, float circle_radius, float stroke_size)
{
	float outside = circle(position, circle_radius);
	float inside = circle(position, circle_radius - stroke_size);
	return mix(outside_color, inside_color, inside - outside);
}

// fills a circle but also cuts it with anothed circle mask
vec3 fill_masked_circle(
	vec2 mask_position, float mask_radius, vec3 mask_color, 
	vec2 position, float radius, vec3 outside_color, vec3 inside_color)
{
	vec3 in_out_color = fill_circle(position, outside_color, inside_color, radius);
	return fill_circle(mask_position, mask_color, in_out_color, mask_radius);
}

void fragment() {
	vec3 current_color = g_background_color;
	
	// draw shadow
	current_color = fill_masked_circle(UV, g_radius, current_color, UV - g_iris_shadow.xy, g_radius * g_iris_shadow.z, g_iris_color_a * g_iris_shadow_color, g_iris_color_a);
		
	//// draw iris
	//current_color = fill_circle(UV, g_background_color, g_iris_color_a, g_radius); 
	
	// draw pupil
	current_color = fill_circle(UV, current_color, g_pupil_color, g_radius * g_pupil_radius); 
	
	// draw outline
	current_color = stroke_circle(UV, current_color, g_outline_color, g_radius, g_outline_radius);

	// set final color
	ALBEDO = current_color;
}


```
