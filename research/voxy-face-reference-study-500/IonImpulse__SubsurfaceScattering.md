# Facial rendering source scan: IonImpulse/SubsurfaceScattering

Repository: https://github.com/IonImpulse/SubsurfaceScattering

Pinned source revision: ff70336133025771376abdfe81cb9250846fb545

Discovery topic: subsurface scattering

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": ["output/benchy_no_sss.png", "output/benchy_sss.png"]}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "9209a888565cee9dc50371b5ef1c57c491d4d8c1",
  "sha256": "741a07621dbfe9b55fc12b026978ad177c8c2f075d91b251b5d5b8701c38e1d7",
  "lines": 46,
  "bytes": 1705,
  "selection_score": 0,
  "keyword_line_counts": {
    "subsurface": 3,
    "sss": 2,
    "gum": 10,
    "face": 3
  },
  "url": "https://github.com/IonImpulse/SubsurfaceScattering/blob/ff70336133025771376abdfe81cb9250846fb545/main.py",
  "function_signature_candidates": []
}

```text

"""
This is the main file for the subsurface scattering project.
It mainly handles argument parsing and calling the appropriate functions.
"""

import argparse

import numpy as np
from load_stl import load_stl
from scene import Scene
from render import Renderer
from save_image import save_image

if __name__ == "__main__":
    # Make ctrl-c force quit
    import signal
    signal.signal(signal.SIGINT, signal.SIG_DFL)


    # Argument parsing
    parser = argparse.ArgumentParser(description="Subsurface scattering simulation")
    parser.add_argument("input", help="The input STL file")
    parser.add_argument("output", help="The output PNG file")
    parser.add_argument("--width", type=int, default=80, help="The width of the output image")
    parser.add_argument("--height", type=int, default=45, help="The height of the output image")
    parser.add_argument("--samples", type=int, default=100, help="The number of samples to take per pixel")
    parser.add_argument("--position", type=float, nargs=3, default=[0, 1, 1], help="The position of the camera")
    parser.add_argument("--no-sss", action="store_true", help="Disable subsurface scattering")

    args = parser.parse_args()

    # Convert the position to a numpy array
    args.position = np.array(args.position, dtype=np.float32)

    # Load the STL file
    print("Loading STL file...")
    vertices, normals = load_stl(args.input)

    print("Setting up render...")
    scene = Scene(vertices, normals, args.position, None, not args.no_sss)

    print("Rendering image...")
    processed_data = Renderer.render(scene, args.width, args.height, args.samples)
    
    print("Saving image...")
    save_image(args.output, processed_data)

```

## Source file: scene.py

{
  "path": "scene.py",
  "blob_sha": "7fa9be4157319ae4effe4d7b88fe3222e32478da",
  "sha256": "0d3dbcdbf07681abd19d9df4e30d8f3d2e653d0b63297cebbfaf7864c91bf343",
  "lines": 285,
  "bytes": 11528,
  "selection_score": 0,
  "keyword_line_counts": {
    "sss": 2,
    "rig": 25
  },
  "url": "https://github.com/IonImpulse/SubsurfaceScattering/blob/ff70336133025771376abdfe81cb9250846fb545/scene.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self):"
    },
    {
      "line": 19,
      "text": "def is_leaf(self):"
    },
    {
      "line": 22,
      "text": "def ray_intersects_bbox(self, ray_origin, ray_direction):"
    },
    {
      "line": 45,
      "text": "def build_bvh(triangles: np.ndarray, depth=0):"
    },
    {
      "line": 88,
      "text": "def __init__(self, position, color, intensity):"
    },
    {
      "line": 94,
      "text": "def __init__(self, refraction_index, scattering_coefficients, color):"
    },
    {
      "line": 100,
      "text": "def __init__(self, vertices: np.ndarray, normals: np.ndarray, camera_pos: np.ndarray, camera_ori=None, use_sss=True):"
    },
    {
      "line": 158,
      "text": "def add_geometry(self, vertices, normals):"
    },
    {
      "line": 163,
      "text": "def add_material(self, material):"
    },
    {
      "line": 166,
      "text": "def add_light(self, light):"
    },
    {
      "line": 169,
      "text": "def set_camera(self, position, orientation, fov):"
    },
    {
      "line": 172,
      "text": "def print_scene(self):"
    },
    {
      "line": 217,
      "text": "def ray_intersects_triangle(ray_origin: np.array, ray_direction: np.array, triangle_vertices: Tuple[np.array]) -> Tuple[bool, float]:"
    },
    {
      "line": 241,
      "text": "def ray_march(self, ray_origin: np.array, ray_direction: np.array, max_distance: float, epsilon: float) -> Tuple[bool, np.array, np.array]:"
    },
    {
      "line": 271,
      "text": "def look_at(camera_pos, target, up):"
    }
  ]
}

```text

"""
This file contains the classes for the scene, geometry, material, and light.
"""
import numpy as np
from typing import Tuple


class BVHNode:
    """
    A node in the BVH tree. Each node has a bounding box, and either two children or a list of triangles.
    """
    def __init__(self):
        self.bbox_min = np.array([float('inf'), float('inf'), float('inf')])
        self.bbox_max = np.array([-float('inf'), -float('inf'), -float('inf')])
        self.left = None
        self.right = None
        self.triangles = []

    def is_leaf(self):
        return len(self.triangles) > 0
    
    def ray_intersects_bbox(self, ray_origin, ray_direction):
        t_min = np.empty(3)
        t_max = np.empty(3)
        
        for i in range(3):  # Iterate over x, y, z axes
            if ray_direction[i] == 0:
                t_min[i] = -float('inf')
                t_max[i] = float('inf')
                continue
            
            inv_dir = 1 / ray_direction[i]
            t_min[i] = (self.bbox_min[i] - ray_origin[i]) * inv_dir
            t_max[i] = (self.bbox_max[i] - ray_origin[i]) * inv_dir
            if inv_dir < 0:
                t_min[i], t_max[i] = t_max[i], t_min[i]
        
        # Calculate the furthest t_min and the closest t_max
        t_enter = t_min.max()
        t_exit = t_max.min()
        
        return t_enter <= t_exit and t_exit >= 0  # Intersects if t_enter <= t_exit and t_exit is positive

    @staticmethod
    def build_bvh(triangles: np.ndarray, depth=0):
        node = BVHNode()

        # Base case: if there's 1 or fewer triangles, it's a leaf node.
        if len(triangles) <= 1:
            if triangles:  # Check if there is at least one triangle
                node.bbox_min = triangles[0].min(axis=0)
                node.bbox_max = triangles[0].max(axis=0)
                node.triangles = triangles
            return node

        # Compute centroid bounding box for all triangles.
        centers = np.array([tri.mean(axis=0) for tri in triangles])
        centroid_bbox_min = centers.min(axis=0)
        centroid_bbox_max = centers.max(axis=0)

        # Find the axis along which to split: we choose the axis with the maximum extent.
        axis = np.argmax(centroid_bbox_max - centroid_bbox_min)

        # Find the median on the chosen axis.
        median = np.median(centers[:, axis])

        # Partition triangles into two sets: those less than the median and those greater or equal to the median.
        left_tris = [tri for i, tri in enumerate(triangles) if centers[i][axis] < median]
        right_tris = [tri for i, tri in enumerate(triangles) if centers[i][axis] >= median]

        # Handle the case where all centroids are at the exact same coordinate along the chosen axis
        # by splitting the list of triangles into two equal halves.
        if not left_tris or not right_tris:
            half_length = len(triangles) // 2
            left_tris, right_tris = triangles[:half_length], triangles[half_length:]

        # Recursive case: build the BVH for the two halves.
        node.left = BVHNode.build_bvh(left_tris, depth + 1)
        node.right = BVHNode.build_bvh(right_tris, depth + 1)

        # Update this node's bounding box to surround its children's bounding boxes.
        node.bbox_min = np.minimum(node.left.bbox_min if node.left else np.inf, node.right.bbox_min if node.right else np.inf)
        node.bbox_max = np.maximum(node.left.bbox_max if node.left else -np.inf, node.right.bbox_max if node.right else -np.inf)

        return node

class Light:
    def __init__(self, position, color, intensity):
        self.position = position
        self.color = color
        self.intensity = intensity

class Material:
    def __init__(self, refraction_index, scattering_coefficients, color):
        self.refraction_index = refraction_index
        self.scattering_coefficients = scattering_coefficients
        self.color = color

class Scene:
    def __init__(self, vertices: np.ndarray, normals: np.ndarray, camera_pos: np.ndarray, camera_ori=None, use_sss=True):
        """
        Setup the scene with the geometry, materials, lights, and camera.

        Args:
            vertices: Numpy array of shape (N, 3, 3) representing the geometry's vertices.
            normals: Numpy array of shape (N, 3, 3) representing the normals at each vertex.

        Returns:
            A Scene object with the geometry, materials, lights, and camera set up.
        """
        self.geometry = []
        self.materials = []
        self.lights = []
        self.camera = {}
        self.bvh = None
        self.use_sss = use_sss

        # Normalize geometry to fit within the unit cube
        min_v = vertices.min(axis=(0, 1))
        max_v = vertices.max(axis=(0, 1))
        scale = 1 / max(max_v - min_v)

        normalized_vertices = (vertices - (min_v + max_v) / 2) * scale * 1.5  # Center the geometry at the origin
        
        # Add geometry
        self.add_geometry(normalized_vertices, normals)

        # Add material properties
        # Material should mimic a blue plastic
        material = Material(
            refraction_index=1.5,
            scattering_coefficients=np.array([0.1, 0.1, 0.1]),
            color=np.array([0.5, 0.5, 1.0])
        )

        self.add_material(material)

        # Add a light source (position, color, intensity)
        light = Light(np.array([2, 2, 2]), np.array([1.0, 1.0, 1.0]), 1.0)
        self.add_light(light)

        # Setup the camera
        # The camera is above the origin and shifted slightly to the left

        if camera_ori is None:    
            # Calculate the orientation of the camera to point at the origin
            target_point = np.array([0, 0, 0])  # You can adjust this if you want to look at a different point.
            world_up = np.array([0, 0, 1])  # Assuming Y is up in the world coordinate system.
            camera_ori = Scene.look_at(camera_pos, target_point, world_up)


        fov = 60  # Field of view in degrees
        self.set_camera(camera_pos, camera_ori, fov)

        self.print_scene()
        
        
    def add_geometry(self, vertices, normals):
        self.geometry.append({'vertices': vertices, 'normals': normals})
        triangles = [vertices[i] for i in range(vertices.shape[0])]
        self.bvh = BVHNode.build_bvh(triangles)
        
    def add_material(self, material):
        self.materials.append(material)
        
    def add_light(self, light):
        self.lights.append(light)
        
    def set_camera(self, position, orientation, fov):
        self.camera = {'position': position, 'orientation': orientation, 'fov': fov}

    def print_scene(self):
        """
        Print out basic info about the scene.
        """
        print("Scene Summary:")
        print("--------------")
        
        # Print geometries information
        print("Geometries:")
        for index, geometry in enumerate(self.geometry):
            vertices = geometry['vertices']
            bbox_min = vertices.min(axis=(0, 1))
            bbox_max = vertices.max(axis=(0, 1))
            print(f"  Geometry {index}:")
            print(f"    Bounding Box Min: {bbox_min}")
            print(f"    Bounding Box Max: {bbox_max}")
            print(f"    Triangle Count: {len(vertices)}")
        
        # Print materials information
        print("\nMaterials:")
        for index, material in enumerate(self.materials):
            print(f"  Material {index}:")
            print(f"    Refraction Index: {material.refraction_index}")
            print(f"    Scattering Coefficients: {material.scattering_coefficients}")
            print(f"    Color: {material.color}")
        
        # Print lights information
        print("\nLights:")
        for index, light in enumerate(self.lights):
            print(f"  Light {index}:")
            print(f"    Position: {light.position}")
            print(f"    Color: {light.color}")
            print(f"    Intensity: {light.intensity}")
        
        # Print camera information
        print("\nCamera:")
        camera = self.camera
        print(f"  Position: {camera['position']}")
        print(f"  Orientation:\n{camera['orientation']}")
        print(f"  Field of View: {camera['fov']} degrees")

        print("\n--------------")
        print("End of Scene Summary\n")

    @staticmethod
    def ray_intersects_triangle(ray_origin: np.array, ray_direction: np.array, triangle_vertices: Tuple[np.array]) -> Tuple[bool, float]:
        """Möller-Trumbore intersection algorithm"""
        v0, v1, v2 = triangle_vertices
        edge1, edge2 = v1 - v0, v2 - v0
        h = np.cross(ray_direction, edge2)
        a = np.dot(edge1, h)
        if -1e-7 < a < 1e-7:
            return False, None  # This means the ray is parallel to the triangle.
        f = 1.0 / a
        s = ray_origin - v0
        u = f * np.dot(s, h)
        if u < 0.0 or u > 1.0:
            return False, None
        q = np.cross(s, edge1)
        v = f * np.dot(ray_direction, q)
        if v < 0.0 or u + v > 1.0:
            return False, None
        # At this stage we can compute t to find out where the intersection point is on the line.
        t = f * np.dot(edge2, q)
        if t > 1e-7:  # ray intersection
            return True, t
        else:  # This means that there is a line intersection but not a ray intersection.
            return False, None
            
    def ray_march(self, ray_origin: np.array, ray_direction: np.array, max_distance: float, epsilon: float) -> Tuple[bool, np.array, np.array]:
        closest_intersection = max_distance
        intersection_found = False
        intersection_point = None
        normal_at_intersection = None

        # Use a stack instead of the recursive function to traverse the BVH tree
        stack = [self.bvh]
        while stack:
            node = stack.pop()
            if node is None or not node.ray_intersects_bbox(ray_origin, ray_direction):
                continue

            if node.is_leaf():
                for triangle in node.triangles:
                    has_intersection, distance = self.ray_intersects_triangle(ray_origin, ray_direction, triangle)
                    if has_intersection and distance < closest_intersection:
                        intersection_found = True
                        closest_intersection = distance
                        intersection_point = ray_origin + distance * ray_direction
                        normal_at_intersection = np.cross(triangle[1] - triangle[0], triangle[2] - triangle[0])
                        normal_at_intersection /= np.linalg.norm(normal_at_intersection)
            else:
                # Add child nodes to the stack
                stack.append(node.right)
                stack.append(node.left)

        return intersection_found, intersection_point, normal_at_intersection
        
    @staticmethod
    def look_at(camera_pos, target, up):
        # Calculate forward vector (direction the camera is looking)
        forward = np.array(target) - np.array(camera_pos)
        forward /= np.linalg.norm(forward)
        
        # Calculate right vector
        right = np.cross(forward, np.array(up))
        right /= np.linalg.norm(right)

        # Calculate true up vector
        true_up = np.cross(right, forward)

        # Construct a 3x3 orientation matrix (camera to world basis transform)
        orientation = np.stack((right, true_up, -forward), axis=-1)
        return orientation


```

## Source file: render.py

{
  "path": "render.py",
  "blob_sha": "367d1a60b795f2a9dfbb5092b0c63b3977c43980",
  "sha256": "78a52b343c0f7057607b6a768dafaa514357ea28ee58977ad1ee28416f2f1663",
  "lines": 247,
  "bytes": 11275,
  "selection_score": 0,
  "keyword_line_counts": {
    "subsurface": 7,
    "sss": 14,
    "rig": 1,
    "face": 7,
    "lip": 3
  },
  "url": "https://github.com/IonImpulse/SubsurfaceScattering/blob/ff70336133025771376abdfe81cb9250846fb545/render.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self):"
    },
    {
      "line": 20,
      "text": "def calculate_color(scene: Scene, intersection_point: np.array, normal_at_intersection: np.array) -> np.array:"
    },
    {
      "line": 45,
      "text": "def calculate_phong_color(scene: Scene, intersection_point: np.array, normal_at_intersection: np.array) -> np.array:"
    },
    {
      "line": 101,
      "text": "def calculate_subsurface_color(scene: Scene, intersection_point: np.array, normal_at_intersection: np.array) -> np.array:"
    },
    {
      "line": 160,
      "text": "def raytrace(scene: Scene, pixel: np.ndarray, width: int, height: int) -> np.ndarray:"
    },
    {
      "line": 201,
      "text": "def raytrace_chunk(args):"
    },
    {
      "line": 211,
      "text": "def init_pool(shared_array_base, width, height):"
    },
    {
      "line": 218,
      "text": "def render(scene: Scene, width: int, height: int, samples: int) -> np.ndarray:"
    }
  ]
}

```text

"""
This file handles the actual subsurface scattering simulation through ray tracing.
"""

import ctypes
from multiprocessing import Pool, Array
import os
import numpy as np
from tqdm import tqdm
from numba import jit, float32, vectorize

from scene import Scene

class Renderer:
    def __init__(self):
        pass

    @staticmethod
    @jit(forceobj=True, fastmath=True)
    def calculate_color(scene: Scene, intersection_point: np.array, normal_at_intersection: np.array) -> np.array:
        """
        Extend the function to combine both the Phong lighting and SSS contributions.
        """
        # Calculate the regular Phong lighting color
        phong_color = Renderer.calculate_phong_color(scene, intersection_point, normal_at_intersection) 
        
        if scene.use_sss:
            # Calculate the subsurface scattering color
            sss_color = Renderer.calculate_subsurface_color(scene, intersection_point, normal_at_intersection)
        
            # Mix the Phong color with the SSS color based on a weighted factor
            # This weight can be part of the material properties to indicate how strong the SSS effect is for this material
            sss_weight = 0.3  # Adjust this factor to achieve the desired effect
            combined_color = (1 - sss_weight) * phong_color + sss_weight * sss_color
            
            # Make sure we still respect the 0-255 range
            combined_color = np.clip(combined_color, 0, 255).astype(np.uint8)
        
            return combined_color
        else:
            return phong_color

    @staticmethod
    @jit(forceobj=True, fastmath=True)
    def calculate_phong_color(scene: Scene, intersection_point: np.array, normal_at_intersection: np.array) -> np.array:
        """
        This function calculates the color at the given intersection point using the Phong reflection model
        with color values in the range 0 to 255.
        
        Args:
            scene: The scene object.
            intersection_point: The intersection point.
            normal_at_intersection: The normal at the intersection point.

        Returns:
            The color at the intersection point as an array of integers in the range 0 to 255.
        """ 
        # Ambient lighting - static ambient light level
        ambient_strength = 0.1
        ambient_color = np.array([255, 255, 255])  # White ambient light
        
        # Assuming there is one material and one light source in the scene
        material = scene.materials[0]
        light = scene.lights[0]

        # Convert material and light color to 0-255 scale and ensure they are np.array of integers
        material_color = np.array(material.color * 255, dtype=int)
        light_color = np.array(light.color * 255, dtype=int)

        # The resulting color starts with the ambient light
        resulting_color = material_color * ambient_strength * (ambient_color / 255)

        # Diffuse lighting
        light_dir = light.position - intersection_point
        light_dir /= np.linalg.norm(light_dir)  # Normalize light direction
        diff = max(np.dot(normal_at_intersection, light_dir), 0.0)
        diffuse = diff * material_color * (light_color / 255) * light.intensity

        # Specular lighting
        viewer_dir = scene.camera['position'] - intersection_point
        viewer_dir /= np.linalg.norm(viewer_dir)  # Normalize viewer direction
        reflect_dir = 2 * normal_at_intersection * np.dot(normal_at_intersection, light_dir) - light_dir
        reflect_dir /= np.linalg.norm(reflect_dir)  # Normalize reflection direction

        specular_strength = 0.5
        shininess = 32
        spec = pow(max(np.dot(viewer_dir, reflect_dir), 0.0), shininess)
        specular = specular_strength * spec * (light_color / 255) * light.intensity

        # Combine the ambient, diffuse, and specular components
        resulting_color += diffuse + specular
        
        # Ensure that the color does not exceed the maximum value of 255 in any channel
        resulting_color = np.clip(resulting_color, 0, 255)
        
        # Convert the result to integers as we're dealing with discrete color values
        return resulting_color.astype(np.uint8)

    @staticmethod
    @jit(forceobj=True, fastmath=True)
    def calculate_subsurface_color(scene: Scene, intersection_point: np.array, normal_at_intersection: np.array) -> np.array:
        """
        This function simulates the subsurface scattering effect at a given
        intersection point using a simple BSSRDF model.
        """
        # Assuming there's just one material in the scene for simplification
        material = scene.materials[0]
        light = scene.lights[0]  # Assuming one direct light source for simplicity

        # Constants and scale factors for the simple BSSRDF model
        scattering_radius = 0.1  # Adjust this value according to the material's scattering properties
        samples_count = 25  # The number of samples to evaluate the BSSRDF
        falloff = 1.0 / (4 * np.pi * scattering_radius**2)  # Simplistic falloff factor based on the scattering radius

        sss_color = np.zeros(3)
        
        # Retrieve the material and light color, convert to the range 0 to 1
        light_color = light.color
        material_color = material.color

        # Calculate the direction to the light source
        light_dir = light.position - intersection_point
        light_distance = np.linalg.norm(light_dir)
        light_dir /= light_distance  # Normalize
        light_intensity = light.intensity / (light_distance**2)  # Attenuate based on distance squared

        # Find the light's contribution to the subsurface scattering
        light_contribution = light_color * light_intensity

        # Iterate over sample points to simulate the BSSRDF
        for i in range(samples_count):
            # Generate a sample point within the scattering radius around the point of intersection
            offset = np.random.uniform(-1, 1, 3)
            offset /= np.linalg.norm(offset)
            offset *= scattering_radius * np.sqrt(np.random.uniform(0, 1))  # Ensure uniform distribution over the sphere

            # Sample point is the offset from the intersection point inside the material
            sample_point = intersection_point + offset
            
            # Calculate the distance between the sample point and the original intersection point
            r = np.linalg.norm(sample_point - intersection_point)
            
            # Rd(r) is the BSSRDF's diffuse reflectance, which diminishes with the square of the distance 
            Rd = falloff * np.exp(-material.scattering_coefficients * r) / r**2
            
            # Accumulate the weighted subsurface scattering color
            sss_color += Rd * material_color * light_contribution
        
        # Divide by the number of samples to normalize the SSS contribution
        sss_color /= samples_count
        
        # Finally, multiply by 255 to adapt the range from [0, 1] to [0, 255] and clamp to valid color values
        sss_color *= 255
        sss_color = np.clip(sss_color, 0, 255)

        return sss_color.astype(np.uint8)
        
    @staticmethod
    @jit(forceobj=True, fastmath=True)
    def raytrace(scene: Scene, pixel: np.ndarray, width: int, height: int) -> np.ndarray:
        """
        This function performs ray tracing for a single pixel, using the
        ray_march function to find the intersection point and the normal
        at that point.
        """
        # Extract the camera parameters
        cam_pos = scene.camera['position']
        cam_ori_matrix = scene.camera['orientation']  # Now a 3x3 matrix
        fov = scene.camera['fov']

        # Calculate the aspect ratio
        aspect_ratio = width / height

        # Calculate scales for x and y based on fov
        scale_x = np.tan(np.radians(fov) / 2) * aspect_ratio
        scale_y = np.tan(np.radians(fov) / 2)
        
        # Map pixel to [-1, 1] range (NDC space)
        x_ndc = (2 * (pixel[0] / width) - 1) * scale_x
        y_ndc = (1 - 2 * (pixel[1] / height)) * scale_y
        
        # Calculate the direction vector (in camera space) based on NDC space
        direction_camera_space = np.array([x_ndc, y_ndc, -1], dtype=np.float32)  # z-coordinate is -1 because the camera looks towards negative z.
        direction_camera_space /= np.linalg.norm(direction_camera_space)
        
        # Transform the direction from camera space to world space using the camera orientation matrix
        ray_direction_world_space = cam_ori_matrix @ direction_camera_space
        ray_direction_world_space /= np.linalg.norm(ray_direction_world_space)

        # Perform ray marching to find the intersection point and normal
        intersection_found, intersection_point, normal_at_intersection = scene.ray_march(cam_pos, ray_direction_world_space, 3, 0.00001)
        
        if intersection_found:
            # Calculate the color at the intersection point
            return Renderer.calculate_color(scene, intersection_point, normal_at_intersection)
        else:
            # No intersection found, return background color (e.g., black)
            return np.array([0, 0, 0])

    @staticmethod
    def raytrace_chunk(args):
        scene, y_row, width, height, samples, = args
        # Access the global shared memory array within this worker.
        global shared_output_array
        for x in range(width):
            # Sample and raytrace code here.
            color = Renderer.raytrace(scene, np.array([x, y_row]), width, height)
            shared_output_array[y_row, x] = color

    @staticmethod
    def init_pool(shared_array_base, width, height):
        # Initialize the shared memory buffer to be accessible to each pool worker.
        global shared_output_array
        shared_output_array = np.ctypeslib.as_array(shared_array_base.get_obj())
        shared_output_array = shared_output_array.reshape(height, width, 3)
    
    @staticmethod
    def render(scene: Scene, width: int, height: int, samples: int) -> np.ndarray:
        # Create a shared ctypes array with an appropriate lock
        shared_array_base = Array(ctypes.c_uint8, width * height * 3)
        shared_array = np.ctypeslib.as_array(shared_array_base.get_obj())
        shared_array = shared_array.reshape(height, width, 3)
        
        THREADS = os.cpu_count()
        pool = Pool(THREADS, initializer=Renderer.init_pool, initargs=(shared_array_base, width, height,))

        # Create a tqdm progress bar
        pbar = tqdm(total=height)

        # Define tasks based on rows to distribute to processes.
        tasks = [(scene, y, width, height, samples) for y in range(height)]
        
        # Keep track of the result handles
        result_handles = [pool.apply_async(Renderer.raytrace_chunk, args=(task,), callback=lambda _: pbar.update(1)) for task in tasks]

        # Prevent the main process from continuing until all results are done
        for result in result_handles:
            result.wait()

        # Close the pool and wait for all workers to finish
        pool.close()
        pool.join()

        # Ensure the progress bar is closed
        pbar.close()

        return shared_array

```

## Source file: load_stl.py

{
  "path": "load_stl.py",
  "blob_sha": "7de1e34e4841c1bc0989682fa8bc78b356627651",
  "sha256": "5fe6349d9735c03e9d7a1d9d3d63d6efd53ebecdf4930e8da3ce1bc457083fb7",
  "lines": 58,
  "bytes": 2143,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/IonImpulse/SubsurfaceScattering/blob/ff70336133025771376abdfe81cb9250846fb545/load_stl.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def load_stl(path: str) -> (np.ndarray, np.ndarray):"
    },
    {
      "line": 11,
      "text": "def read_triangle(file):"
    },
    {
      "line": 19,
      "text": "def read_binary_stl(file):"
    },
    {
      "line": 28,
      "text": "def read_ascii_stl(file):"
    }
  ]
}

```text

"""
This file handles loading of STL files into the program.
It returns a numpy array of the vertices and a numpy array of the normals.
"""

import numpy as np
import os
import struct

def load_stl(path: str) -> (np.ndarray, np.ndarray):
    def read_triangle(file):
        normal = struct.unpack('3f', file.read(12))
        vertex1 = struct.unpack('3f', file.read(12))
        vertex2 = struct.unpack('3f', file.read(12))
        vertex3 = struct.unpack('3f', file.read(12))
        file.read(2)  # skip attribute byte count
        return np.array(normal), np.array([vertex1, vertex2, vertex3])

    def read_binary_stl(file):
        file.read(80)  # skip header
        num_triangles = struct.unpack('I', file.read(4))[0]
        normals = np.zeros((num_triangles, 3), dtype=np.float32)
        vertices = np.zeros((num_triangles, 3, 3), dtype=np.float32)
        for i in range(num_triangles):
            normals[i], vertices[i] = read_triangle(file)
        return vertices, normals
    
    def read_ascii_stl(file):
        normals = []
        vertices = []
        for line in file:
            parts = line.strip().split()
            if parts[0] == 'facet' and parts[1] == 'normal':
                normals.append(np.array(parts[-3:], dtype=float))
            elif parts[0] == 'vertex':
                vertices.append(np.array(parts[-3:], dtype=float))
        
        normals = np.array(normals, dtype=np.float32)
        vertices = np.array(vertices, dtype=np.float32).reshape((-1, 3, 3))

        return vertices, normals

    if not os.path.isfile(path):
        raise FileNotFoundError(f"The file {path} does not exist.")

    with open(path, 'r') as file:  # Open as text for ASCII parsing
        try:
            first_line = file.readline()
            file.seek(0)  # reset to beginning of the file
            if 'solid' in first_line:
                # it's an ASCII STL
                return read_ascii_stl(file)
        except Exception as e:
            pass
        
        # it's a binary STL, re-open as binary
        with open(path, 'rb') as binary_file:
            return read_binary_stl(binary_file)


```
