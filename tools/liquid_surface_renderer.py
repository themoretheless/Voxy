"""Orthographic depth-tested rendering of solver surfaces with smooth normals.

Optical coefficients are presentation settings, not measured fluid parameters.
"""
import numpy as np
from PIL import Image

def render_surfaces(image, fluid, film, project):
    pixels = np.array(image, dtype=np.uint8)
    depth = np.full(pixels.shape[:2], -np.inf)
    view = np.array([.48, .30, -1.0]); view /= np.linalg.norm(view)
    light = np.array([-.3, .8, -.5]); light /= np.linalg.norm(light)
    half = view + light; half /= np.linalg.norm(half)

    def raster(triangles, heights=None):
        if not len(triangles):
            return
        vertices = np.asarray(triangles, dtype=float).reshape(-1, 3, 3)
        # Weld shared marching-tetrahedra or film vertices for area-weighted
        # normals. No geometry is blurred, inflated or translated here.
        _, inverse = np.unique(np.round(vertices.reshape(-1, 3), 11), axis=0, return_inverse=True)
        indices = inverse.reshape(-1, 3)
        face_normals = np.cross(vertices[:,1]-vertices[:,0], vertices[:,2]-vertices[:,0])
        normals = np.zeros((inverse.max()+1,3))
        for corner in range(3):
            np.add.at(normals, indices[:,corner], face_normals)
        normals /= np.maximum(np.linalg.norm(normals,axis=1)[:,None],1e-30)
        for number, tri in enumerate(vertices):
            screen = np.asarray([project(*p) for p in tri])
            lo = np.maximum(np.floor(screen.min(axis=0)).astype(int),0)
            hi = np.minimum(np.ceil(screen.max(axis=0)).astype(int),[pixels.shape[1]-1,pixels.shape[0]-1])
            if np.any(hi<lo):
                continue
            a,b,c=screen
            denominator=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
            if abs(denominator)<1e-12:
                continue
            xx,yy=np.meshgrid(np.arange(lo[0],hi[0]+1)+.5,np.arange(lo[1],hi[1]+1)+.5)
            w0=((b[1]-c[1])*(xx-c[0])+(c[0]-b[0])*(yy-c[1]))/denominator
            w1=((c[1]-a[1])*(xx-c[0])+(a[0]-c[0])*(yy-c[1]))/denominator
            w2=1-w0-w1
            weights=np.stack([w0,w1,w2],axis=-1)
            z=weights@(tri@view)
            region=depth[lo[1]:hi[1]+1,lo[0]:hi[0]+1]
            visible=(weights.min(axis=-1)>=-1e-8)&(z>region)
            if not visible.any():
                continue
            n=weights@normals[indices[number]]
            n/=np.maximum(np.linalg.norm(n,axis=-1,keepdims=True),1e-30)
            diffuse=.34+.5*np.maximum(n@light,0)
            specular=.75*np.maximum(n@half,0)**90+.14*np.maximum(n@half,0)**12
            rgb=np.clip((diffuse[...,None]*np.array([.94,.95,.94])+specular[...,None])*255,0,255)
            target=pixels[lo[1]:hi[1]+1,lo[0]:hi[0]+1]
            if heights is not None:
                h=weights@np.asarray(heights[number])
                opacity=np.clip(-np.expm1(-h/.00012),0,1)[...,None]
                rgb=target*(1-opacity)+rgb*opacity
            target[visible]=rgb[visible].astype(np.uint8)
            region[visible]=z[visible]

    # Surface-film coordinates use its actual area-weighted vertex thickness.
    film_triangles=[]; heights=[]
    for ax,az,bx,bz,cx,cz,ha,hb,hc,gel in film:
        film_triangles.append([[ax,ha,az],[bx,hb,bz],[cx,hc,cz]])
        heights.append([ha,hb,hc])
    raster(film_triangles,heights)
    raster(fluid)
    return Image.fromarray(pixels)
