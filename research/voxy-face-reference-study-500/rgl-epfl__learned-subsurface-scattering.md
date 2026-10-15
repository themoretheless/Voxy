# Facial rendering source scan: rgl-epfl/learned-subsurface-scattering

Repository: https://github.com/rgl-epfl/learned-subsurface-scattering

Pinned source revision: f33e86271db0e771e9753cd8281a901ce1fc3d44

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "doc/license.txt"]

Topic paths: {"subsurface": ["doc/images/subsurface_dipole.jpg", "doc/images/subsurface_dipole_2.jpg", "doc/images/subsurface_dipole_bad1.jpg", "doc/images/subsurface_dipole_bad2.jpg", "doc/images/subsurface_dipole_bad3.jpg", "doc/images/subsurface_dipole_dragon.jpg", "doc/images/subsurface_dipole_dragon2.jpg", "doc/section_subsurface.tex", "include/mitsuba/render/subsurface.h", "src/librender/subsurface.cpp", "src/subsurface/SConscript", "src/subsurface/bluenoise.cpp", "src/subsurface/bluenoise.h", "src/subsurface/dipole.cpp", "src/subsurface/dipole_rt.cpp", "src/subsurface/irrproc.cpp", "src/subsurface/irrproc.h", "src/subsurface/irrtree.cpp", "src/subsurface/irrtree.h", "src/subsurface/json.hpp", "src/subsurface/scattereigen.h", "src/subsurface/singlescatter.cpp", "src/subsurface/vaeconfig.cpp", "src/subsurface/vaeconfig.h", "src/subsurface/vaehelper.cpp", "src/subsurface/vaehelper.h", "src/subsurface/vaehelpereigen.cpp", "src/subsurface/vaehelpereigen.h", "src/subsurface/vaehelperpt.cpp", "src/subsurface/vaehelperpt.h"], "sss": ["include/mitsuba/render/sss_particle_tracer.h", "src/librender/sss_particle_tracer.cpp"], "deform": ["src/shapes/deformable.cpp"], "shader": ["include/mitsuba/hw/basicshader.h", "include/mitsuba/render/shader.h", "pysrc/viewer/shaders/alpha_surface.frag", "pysrc/viewer/shaders/alpha_surface.vert", "pysrc/viewer/shaders/basic_surface.frag", "pysrc/viewer/shaders/basic_surface.vert", "pysrc/viewer/shaders/colored_surface.frag", "pysrc/viewer/shaders/colored_surface.vert", "pysrc/viewer/shaders/dashed.frag", "pysrc/viewer/shaders/dashed.vert", "pysrc/viewer/shaders/facetted.frag", "pysrc/viewer/shaders/facetted.geom", "pysrc/viewer/shaders/facetted.vert", "pysrc/viewer/shaders/point.frag", "pysrc/viewer/shaders/point.vert", "pysrc/viewer/shaders/quad.frag", "pysrc/viewer/shaders/quad.vert", "pysrc/viewer/shaders/raymarch.frag", "pysrc/viewer/shaders/raymarch.vert", "pysrc/viewer/shaders/scatter.frag", "pysrc/viewer/shaders/scatter.vert", "pysrc/viewer/shaders/simple.frag", "pysrc/viewer/shaders/simple.vert", "src/libhw/basicshader.cpp", "src/libhw/data/shaders.h", "src/librender/shader.cpp", "src/mtsgui/shaders/downsampling.frag", "src/mtsgui/shaders/downsampling.vert", "src/mtsgui/shaders/gamma.frag", "src/mtsgui/shaders/gamma.vert"], "rig": ["data/linux/debian/copyright"], "face": ["data/microfacet/beckmann.dat", "data/microfacet/ggx.dat", "data/microfacet/phong.dat", "doc/images/subsurface_dipole.jpg", "doc/images/subsurface_dipole_2.jpg", "doc/images/subsurface_dipole_bad1.jpg", "doc/images/subsurface_dipole_bad2.jpg", "doc/images/subsurface_dipole_bad3.jpg", "doc/images/subsurface_dipole_dragon.jpg", "doc/images/subsurface_dipole_dragon2.jpg", "doc/section_subsurface.tex", "include/mitsuba/render/subsurface.h", "pysrc/viewer/shaders/alpha_surface.frag", "pysrc/viewer/shaders/alpha_surface.vert", "pysrc/viewer/shaders/basic_surface.frag", "pysrc/viewer/shaders/basic_surface.vert", "pysrc/viewer/shaders/colored_surface.frag", "pysrc/viewer/shaders/colored_surface.vert", "pysrc/viewer/shaders/facetted.frag", "pysrc/viewer/shaders/facetted.geom", "pysrc/viewer/shaders/facetted.vert", "src/bsdfs/microfacet.h", "src/librender/subsurface.cpp", "src/subsurface/SConscript", "src/subsurface/bluenoise.cpp", "src/subsurface/bluenoise.h", "src/subsurface/dipole.cpp", "src/subsurface/dipole_rt.cpp", "src/subsurface/irrproc.cpp", "src/subsurface/irrproc.h"], "lip": ["data/darwin/lipo.sh", "pysrc/resources/test/ellipsoid.obj", "src/utils/cylclip.cpp"]}

## Source file: src/librender/subsurface.cpp

{
  "path": "src/librender/subsurface.cpp",
  "blob_sha": "2b75b96651db86927f916ac02b3dace46de18421",
  "sha256": "2b4d14adbd2d0a683843eaf8f3a67420224a382c30f3dd77d4f9604bec489340",
  "lines": 62,
  "bytes": 2042,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 8,
    "rig": 1,
    "face": 8
  },
  "url": "https://github.com/rgl-epfl/learned-subsurface-scattering/blob/f33e86271db0e771e9753cd8281a901ce1fc3d44/src/librender/subsurface.cpp",
  "function_signature_candidates": []
}

```text

/*
    This file is part of Mitsuba, a physically based rendering system.

    Copyright (c) 2007-2014 by Wenzel Jakob and others.

    Mitsuba is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License Version 3
    as published by the Free Software Foundation.

    Mitsuba is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program. If not, see <http://www.gnu.org/licenses/>.
*/

#include <mitsuba/core/properties.h>
#include <mitsuba/render/subsurface.h>
#include <mitsuba/render/shape.h>

MTS_NAMESPACE_BEGIN

Subsurface::Subsurface(const Properties &props)
 : NetworkedObject(props), m_active(false) { }

Subsurface::Subsurface(Stream *stream, InstanceManager *manager) :
    NetworkedObject(stream, manager) {
    size_t shapeCount = stream->readSize();
    for (size_t i=0; i<shapeCount; ++i) {
        Shape *shape = static_cast<Shape *>(manager->getInstance(stream));
        m_shapes.push_back(shape);
    }
    m_active = false;
}

Subsurface::~Subsurface() { }

void Subsurface::cancel() { }

void Subsurface::setParent(ConfigurableObject *parent) {
    if (parent->getClass()->derivesFrom(MTS_CLASS(Shape))) {
        Shape *shape = static_cast<Shape *>(parent);
        if (shape->isCompound())
            return;
        if (std::find(m_shapes.begin(), m_shapes.end(), shape) == m_shapes.end()) {
            m_shapes.push_back(shape);
        }
    }
}

void Subsurface::serialize(Stream *stream, InstanceManager *manager) const {
    NetworkedObject::serialize(stream, manager);

    stream->writeSize(m_shapes.size());
    for (unsigned int i=0; i<m_shapes.size(); ++i)
        manager->serialize(stream, m_shapes[i]);
}

MTS_IMPLEMENT_CLASS(Subsurface, true, NetworkedObject)
MTS_NAMESPACE_END


```

## Source file: include/mitsuba/render/subsurface.h

{
  "path": "include/mitsuba/render/subsurface.h",
  "blob_sha": "fefdcb8bbfab046c305fb6cf8e63ed4a1daa6184",
  "sha256": "b4e5aa6f5f3ffbc0d6b232bed414eefbefc011db7eb3a16ae0d80a98c280f1bc",
  "lines": 89,
  "bytes": 3089,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 15,
    "rig": 1,
    "face": 16
  },
  "url": "https://github.com/rgl-epfl/learned-subsurface-scattering/blob/f33e86271db0e771e9753cd8281a901ce1fc3d44/include/mitsuba/render/subsurface.h",
  "function_signature_candidates": [
    {
      "line": 47,
      "text": "virtual bool preprocess(const Scene *scene, RenderQueue *queue, const RenderJob *job,"
    },
    {
      "line": 51,
      "text": "inline void setActive(bool active) { m_active = active; }"
    },
    {
      "line": 54,
      "text": "inline bool isActive() const { return m_active; }"
    },
    {
      "line": 57,
      "text": "virtual void cancel();"
    },
    {
      "line": 67,
      "text": "void serialize(Stream *stream, InstanceManager *manager) const;"
    },
    {
      "line": 70,
      "text": "void setParent(ConfigurableObject *parent);"
    }
  ]
}

```text

/*
    This file is part of Mitsuba, a physically based rendering system.

    Copyright (c) 2007-2014 by Wenzel Jakob and others.

    Mitsuba is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License Version 3
    as published by the Free Software Foundation.

    Mitsuba is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program. If not, see <http://www.gnu.org/licenses/>.
*/

#pragma once
#if !defined(__MITSUBA_RENDER_SUBSURFACE_H_)
#define __MITSUBA_RENDER_SUBSURFACE_H_

#include <mitsuba/core/netobject.h>

MTS_NAMESPACE_BEGIN

/**
 * \brief Abstract subsurface scattering models
 *
 * Can be attached to an arbitrary shape to compute exitant
 * radiance due to internal scattering. How that is done is
 * completely up to the implementation. It might for instance
 * recursively trace rays or perform lookups into a precomputed
 * point cloud radiance representation.
 *
 * \ingroup librender
 */
class MTS_EXPORT_RENDER Subsurface : public NetworkedObject {
public:
    /**
     * \brief Possibly perform a pre-process task.
     *
     * The last three parameters are resource IDs of the associated scene,
     * camera and sample generator, which have been made available to all
     * local and remote workers.
     */
    virtual bool preprocess(const Scene *scene, RenderQueue *queue, const RenderJob *job,
        int sceneResID, int cameraResID, int samplerResID) = 0;

    /// Selectively activate/deactivate the subsurface integrator
    inline void setActive(bool active) { m_active = active; }

    /// Return whether or not the subsurface integrator is currently active
    inline bool isActive() const { return m_active; }

    /// Cancel any running pre-process tasks
    virtual void cancel();

    /// Return the list of shapes associated with this subsurface integrator
    inline const std::vector<Shape *> getShapes() const { return m_shapes; }

    /// Get the exitant radiance for a point on the surface
    virtual Spectrum Lo(const Scene *scene, Sampler *sampler,
        const Intersection &its, const Vector &d, int depth = 0) const = 0;

    /// Serialize this subsurface integrator to a binary data stream
    void serialize(Stream *stream, InstanceManager *manager) const;

    /// Set the parent node of the subsurface integrator
    void setParent(ConfigurableObject *parent);

    MTS_DECLARE_CLASS()
protected:
    /// Create a new subsurface scattering class
    Subsurface(const Properties &props);

    /// Unserialize a subsurface integrator from a binary data stream
    Subsurface(Stream *stream, InstanceManager *manager);

    /// Virtual destructor
    virtual ~Subsurface();
protected:
    std::vector<Shape *> m_shapes;
    bool m_active;
};

MTS_NAMESPACE_END

#endif /* __MITSUBA_RENDER_SUBSURFACE_H_ */


```

## Source file: src/utils/cylclip.cpp

{
  "path": "src/utils/cylclip.cpp",
  "blob_sha": "3e697fedf3b2d23ab2e0c7da7ca7b4175c4f0f69",
  "sha256": "65882a84cdfcf953c366f6eec310567b8a0b589c0ac5c994c83fefd4e546d459",
  "lines": 310,
  "bytes": 10818,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 12,
    "lip": 46
  },
  "url": "https://github.com/rgl-epfl/learned-subsurface-scattering/blob/f33e86271db0e771e9753cd8281a901ce1fc3d44/src/utils/cylclip.cpp",
  "function_signature_candidates": [
    {
      "line": 39,
      "text": "void windowResized(const DeviceEvent &event) {"
    },
    {
      "line": 45,
      "text": "void mouseDragged(const DeviceEvent &event) {"
    },
    {
      "line": 71,
      "text": "bool intersectCylPlane(Point planePt, Normal planeNrml,"
    },
    {
      "line": 208,
      "text": "void keyPressed(const DeviceEvent &event) {"
    },
    {
      "line": 229,
      "text": "bool init(int argc, char **argv) {"
    },
    {
      "line": 234,
      "text": "void draw() {"
    }
  ]
}

```text

/*
    This file is part of Mitsuba, a physically based rendering system.

    Copyright (c) 2007-2014 by Wenzel Jakob and others.

    Mitsuba is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License Version 3
    as published by the Free Software Foundation.

    Mitsuba is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program. If not, see <http://www.gnu.org/licenses/>.
*/

#include <mitsuba/hw/viewer.h>

MTS_NAMESPACE_BEGIN

class CylClip : public Viewer {
public:
    CylClip() : m_red(0.0f), m_blue(0.0f), m_gray(.5f), m_angle(0) {
        m_viewTransform = Transform::lookAt(Point(10*std::sin(m_angle), 0, std::cos(m_angle)*10),
                Point(0, 0, 0), Vector(0, 1, 0));
        m_lineParams = Point2(M_PI/2, 0.28f);
        m_cylPos = Point(0.0f);
        m_device->setFSAA(4);
        m_red[0] = 1.0f;
        m_blue[2] = 1.0f;
        m_showEllipses = false;
        m_showRectangles = false;
        m_showClippedAABB = false;
        m_radius = .2f;
    }

    void windowResized(const DeviceEvent &event) {
        Float aspect = m_device->getAspect();
        m_projTransform = Transform::glPerspective(45, 1e-4, 1e4)
            * Transform::scale(Vector(1.0f, aspect, 1.0f));
    }

    void mouseDragged(const DeviceEvent &event) {
        if (event.getMouseButton() == Device::ELeftButton) {
            m_angle += event.getMouseRelative().x / 100.0f;
            m_viewTransform = Transform::lookAt(Point(10*std::sin(m_angle), 0, std::cos(m_angle)*10),
                    Point(0, 0, 0), Vector(0, 1, 0));
        } else if (event.getMouseButton() == Device::ERightButton) {
            m_lineParams += Vector2(
                event.getMouseRelative().x / 500.0f,
                event.getMouseRelative().y / 500.0f
            );
        } else if (event.getMouseButton() == Device::EMiddleButton) {
            m_cylPos += Vector3(
                event.getMouseRelative().x / 300.0f,
                0,
                event.getMouseRelative().y / 300.0f
            );
        }
        redraw();
    }

    /**
     * Compute the ellipse created by the intersection of an infinite
     * cylinder and a plane. Returns false in the degenerate case.
     * Based on:
     * www.geometrictools.com/Documentation/IntersectionCylinderPlane.pdf
     */
    bool intersectCylPlane(Point planePt, Normal planeNrml,
            Point cylPt, Vector cylD, Float radius, Point &center,
            Vector *axes, Float *lengths) const {
        if (absDot(planeNrml, cylD) < Epsilon)
            return false;

        Vector B, A = cylD - dot(cylD, planeNrml)*planeNrml;

        Float length = A.length();
        if (length != 0) {
            A /= length;
            B = cross(planeNrml, A);
        } else {
            coordinateSystem(planeNrml, A, B);
        }

        Vector delta = planePt - cylPt,
               deltaProj = delta - cylD*dot(delta, cylD);

        Float aDotD = dot(A, cylD);
        Float bDotD = dot(B, cylD);
        Float c0 = 1-aDotD*aDotD;
        Float c1 = 1-bDotD*bDotD;
        Float c2 = 2*dot(A, deltaProj);
        Float c3 = 2*dot(B, deltaProj);
        Float c4 = dot(delta, deltaProj) - radius*radius;

        Float lambda = (c2*c2/(4*c0) + c3*c3/(4*c1) - c4)/(c0*c1);

        Float alpha0 = -c2/(2*c0),
              beta0 = -c3/(2*c1);

        lengths[0] = std::sqrt(c1*lambda),
        lengths[1] = std::sqrt(c0*lambda);

        center = planePt + alpha0 * A + beta0 * B;
        axes[0] = A;
        axes[1] = B;
        return true;
    }

    AABB intersectCylFace(int axis,
            const Point &min, const Point &max,
            const Point &cylPt, const Vector &cylD) {
        int axis1 = (axis + 1) % 3;
        int axis2 = (axis + 2) % 3;

        Normal planeNrml(0.0f);
        planeNrml[axis] = 1;

        Point ellipseCenter;
        Vector ellipseAxes[2];
        Float ellipseLengths[2];

        AABB aabb;
        if (!intersectCylPlane(min, planeNrml, cylPt, cylD, m_radius,
            ellipseCenter, ellipseAxes, ellipseLengths)) {
            /* Degenerate case -- return an invalid AABB. This is
               not a problem, since one of the other faces will provide
               enough information to arrive at a correct clipped AABB */
            return aabb;
        }

        if (m_showEllipses) {
            m_renderer->setColor(m_gray);
            m_renderer->drawEllipse(ellipseCenter,
                    ellipseAxes[0]*ellipseLengths[0],
                    ellipseAxes[1]*ellipseLengths[1]);
        }

        /* Intersect the ellipse against the sides of the AABB face */
        for (int i=0; i<4; ++i) {
            Point p1, p2;
            p1[axis] = p2[axis] = min[axis];
            p1[axis1] = ((i+1) & 2) ? min[axis1] : max[axis1];
            p1[axis2] = ((i+0) & 2) ? min[axis2] : max[axis2];
            p2[axis1] = ((i+2) & 2) ? min[axis1] : max[axis1];
            p2[axis2] = ((i+1) & 2) ? min[axis2] : max[axis2];

            Point2 p1l(
                dot(p1 - ellipseCenter, ellipseAxes[0]) / ellipseLengths[0],
                dot(p1 - ellipseCenter, ellipseAxes[1]) / ellipseLengths[1]);
            Point2 p2l(
                dot(p2 - ellipseCenter, ellipseAxes[0]) / ellipseLengths[0],
                dot(p2 - ellipseCenter, ellipseAxes[1]) / ellipseLengths[1]);

            Vector2 rel = p2l-p1l;
            Float A = dot(rel, rel);
            Float B = 2*dot(Vector2(p1l), rel);
            Float C = dot(Vector2(p1l), Vector2(p1l))-1;

            Float x0, x1;
            if (solveQuadratic(A, B, C, x0, x1)) {
                m_renderer->setColor(m_red);
                if (x0 >= 0 && x0 <= 1) {
                    m_renderer->drawPoint(p1 + (p2-p1) * x0);
                    aabb.expandBy(p1+(p2-p1)*x0);
                }
                if (x1 >= 0 && x1 <= 1) {
                    m_renderer->drawPoint(p1 + (p2-p1) * x1);
                    aabb.expandBy(p1+(p2-p1)*x1);
                }
            }
        }

        ellipseAxes[0] *= ellipseLengths[0];
        ellipseAxes[1] *= ellipseLengths[1];
        m_renderer->setColor(m_blue);

        AABB faceBounds(min, max);
        /* Find the componentwise maxima of the ellipse */
        for (int i=0; i<2; ++i) {
            int j = (i==0) ? axis1 : axis2;
            Float alpha = ellipseAxes[0][j];
            Float beta = ellipseAxes[1][j];
            Float ratio = beta/alpha, tmp = std::sqrt(1+ratio*ratio);
            Float cosTheta = 1/tmp, sinTheta = ratio/tmp;
            Point p1 = ellipseCenter + cosTheta*ellipseAxes[0] + sinTheta*ellipseAxes[1];
            Point p2 = ellipseCenter - cosTheta*ellipseAxes[0] - sinTheta*ellipseAxes[1];

            if (faceBounds.contains(p1)) {
                aabb.expandBy(p1);
                m_renderer->drawPoint(p1);
            }
            if (faceBounds.contains(p2)) {
                aabb.expandBy(p2);
                m_renderer->drawPoint(p2);
            }
        }

        m_renderer->setColor(m_gray);
        if (aabb.isValid() && m_showRectangles)
            m_renderer->drawAABB(aabb);

        return aabb;
    }

    void keyPressed(const DeviceEvent &event) {
        switch (event.getKeyboardKey()) {
            case 'e':
                m_showEllipses = !m_showEllipses;
                break;
            case 'r':
                m_showRectangles = !m_showRectangles;
                break;
            case 'a':
                m_showClippedAABB = !m_showClippedAABB;
                break;
            case '[':
                m_radius *= 1.1;
                break;
            case ']':
                m_radius /= 1.1;
                break;
        }
        redraw();
    }

    bool init(int argc, char **argv) {
        m_renderer->setPointSize(4.0f);
        return true;
    }

    void draw() {
        m_renderer->setDepthTest(true);
        m_renderer->setCamera(m_projTransform.getMatrix(),
            m_viewTransform.inverse().getMatrix());

        AABB aabb(Point(-3, -1, -1), Point(3, 1, 1));
        m_renderer->setColor(Spectrum(0.3f));
        m_renderer->drawAABB(aabb);

        m_renderer->setColor(m_gray);
        Vector cylD(sphericalDirection(m_lineParams.x, m_lineParams.y));

        m_renderer->drawLine(m_cylPos-cylD*1e4, m_cylPos+cylD*1e4);
        AABB clippedAABB;

        clippedAABB.expandBy(intersectCylFace(0,
                Point(aabb.min.x, aabb.min.y, aabb.min.z),
                Point(aabb.min.x, aabb.max.y, aabb.max.z),
                m_cylPos, cylD));

        clippedAABB.expandBy(intersectCylFace(0,
                Point(aabb.max.x, aabb.min.y, aabb.min.z),
                Point(aabb.max.x, aabb.max.y, aabb.max.z),
                m_cylPos, cylD));

        clippedAABB.expandBy(intersectCylFace(1,
                Point(aabb.min.x, aabb.min.y, aabb.min.z),
                Point(aabb.max.x, aabb.min.y, aabb.max.z),
                m_cylPos, cylD));

        clippedAABB.expandBy(intersectCylFace(1,
                Point(aabb.min.x, aabb.max.y, aabb.min.z),
                Point(aabb.max.x, aabb.max.y, aabb.max.z),
                m_cylPos, cylD));

        clippedAABB.expandBy(intersectCylFace(2,
                Point(aabb.min.x, aabb.min.y, aabb.min.z),
                Point(aabb.max.x, aabb.max.y, aabb.min.z),
                m_cylPos, cylD));

        clippedAABB.expandBy(intersectCylFace(2,
                Point(aabb.min.x, aabb.min.y, aabb.max.z),
                Point(aabb.max.x, aabb.max.y, aabb.max.z),
                m_cylPos, cylD));

        m_renderer->setColor(m_gray);

        if (m_showClippedAABB) {
            if (clippedAABB.isValid())
                m_renderer->drawAABB(clippedAABB);
        }

        m_renderer->setDepthTest(false);
        drawHUD(formatString("Cylinder clipping test. LMB-dragging moves the camera, RMB-dragging rotates the cylinder\n"
                "[e] Ellipses: %s\n"
                "[r] Bounding rectangles : %s\n"
                "[a] Clipped AABB : %s",
            m_showEllipses ? "On": "Off",
            m_showRectangles ? "On": "Off",
            m_showClippedAABB ? "On": "Off"
        ));
    }

    MTS_DECLARE_UTILITY()
private:
    Transform m_projTransform, m_viewTransform;
    Spectrum m_red, m_blue, m_gray;
    Point2 m_lineParams;
    Point m_cylPos;
    Float m_angle, m_radius;
    bool m_showEllipses;
    bool m_showRectangles;
    bool m_showClippedAABB;
};

MTS_EXPORT_UTILITY(CylClip, "Cylinder clipping test")
MTS_NAMESPACE_END


```

## Source file: src/bsdfs/microfacet.h

{
  "path": "src/bsdfs/microfacet.h",
  "blob_sha": "0bb1a0fd0a2af1c08c48d0540ca7612a8e61121b",
  "sha256": "16ba8bcab099fea6333a5efce393b20ae9657a2c4a56c2be6855979aa0146430",
  "lines": 725,
  "bytes": 27836,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 5,
    "rig": 2,
    "face": 39
  },
  "url": "https://github.com/rgl-epfl/learned-subsurface-scattering/blob/f33e86271db0e771e9753cd8281a901ce1fc3d44/src/bsdfs/microfacet.h",
  "function_signature_candidates": [
    {
      "line": 169,
      "text": "inline bool getSampleVisible() const { return m_sampleVisible; }"
    },
    {
      "line": 172,
      "text": "inline bool isAnisotropic() const { return m_alphaU != m_alphaV; }"
    },
    {
      "line": 175,
      "text": "inline bool isIsotropic() const { return m_alphaU == m_alphaV; }"
    },
    {
      "line": 178,
      "text": "inline void scaleAlpha(Float value) {"
    },
    {
      "line": 701,
      "text": "void computePhongExponent() {"
    },
    {
      "line": 707,
      "text": "void sampleFirstQuadrant(Float u1, Float &phi, Float &exponent) const {"
    }
  ]
}

```text

/*
    This file is part of Mitsuba, a physically based rendering system.

    Copyright (c) 2007-2014 by Wenzel Jakob and others.

    Mitsuba is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License Version 3
    as published by the Free Software Foundation.

    Mitsuba is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program. If not, see <http://www.gnu.org/licenses/>.
*/

#if !defined(__MICROFACET_H)
#define __MICROFACET_H

#include <mitsuba/mitsuba.h>
#include <mitsuba/core/frame.h>
#include <mitsuba/core/properties.h>
#include <boost/algorithm/string.hpp>

MTS_NAMESPACE_BEGIN

/**
 * \brief Implementation of the Beckman and GGX / Trowbridge-Reitz microfacet
 * distributions and various useful sampling routines
 *
 * Based on the papers
 *
 *   "Microfacet Models for Refraction through Rough Surfaces"
 *    by Bruce Walter, Stephen R. Marschner, Hongsong Li, and Kenneth E. Torrance
 *
 * and
 *
 *   "Importance Sampling Microfacet-Based BSDFs using the Distribution of Visible Normals"
 *    by Eric Heitz and Eugene D'Eon
 *
 *  The visible normal sampling code was provided by Eric Heitz and Eugene D'Eon.
 */
class MicrofacetDistribution {
public:
    /// Supported distribution types
    enum EType {
        /// Beckmann distribution derived from Gaussian random surfaces
        EBeckmann         = 0,

        /// GGX: Long-tailed distribution for very rough surfaces (aka. Trowbridge-Reitz distr.)
        EGGX              = 1,

        /// Phong distribution (with the anisotropic extension by Ashikhmin and Shirley)
        EPhong            = 2
    };

    /**
     * Create an isotropic microfacet distribution of the specified type
     *
     * \param type
     *     The desired type of microfacet distribution
     * \param alpha
     *     The surface roughness
     */
    inline MicrofacetDistribution(EType type, Float alpha, bool sampleVisible = true)
        : m_type(type), m_alphaU(alpha), m_alphaV(alpha), m_sampleVisible(sampleVisible),
          m_exponentU(0.0f), m_exponentV(0.0f) {
        m_alphaU = std::max(m_alphaU, (Float) 1e-4f);
        m_alphaV = std::max(m_alphaV, (Float) 1e-4f);
        if (m_type == EPhong)
            computePhongExponent();
    }

    /**
     * Create an anisotropic microfacet distribution of the specified type
     *
     * \param type
     *     The desired type of microfacet distribution
     * \param alphaU
     *     The surface roughness in the tangent direction
     * \param alphaV
     *     The surface roughness in the bitangent direction
     */
    inline MicrofacetDistribution(EType type, Float alphaU, Float alphaV, bool sampleVisible = true)
        : m_type(type), m_alphaU(alphaU), m_alphaV(alphaV), m_sampleVisible(sampleVisible),
          m_exponentU(0.0f), m_exponentV(0.0f) {
        m_alphaU = std::max(m_alphaU, (Float) 1e-4f);
        m_alphaV = std::max(m_alphaV, (Float) 1e-4f);
        if (m_type == EPhong)
            computePhongExponent();
    }

    /**
     * \brief Create a microfacet distribution from a Property data
     * structure
     */
    MicrofacetDistribution(const Properties &props, EType type = EBeckmann,
        Float alphaU = 0.1f, Float alphaV = 0.1f, bool sampleVisible = true)
        : m_type(type), m_alphaU(alphaU), m_alphaV(alphaV), m_exponentU(0.0f),
          m_exponentV(0.0f) {

        if (props.hasProperty("distribution")) {
            std::string distr = boost::to_lower_copy(props.getString("distribution"));
            if (distr == "beckmann")
                m_type = EBeckmann;
            else if (distr == "ggx")
                m_type = EGGX;
            else if (distr == "phong" || distr == "as")
                m_type = EPhong;
            else
                SLog(EError, "Specified an invalid distribution \"%s\", must be "
                    "\"beckmann\", \"ggx\", or \"phong\"/\"as\"!", distr.c_str());
        }

        if (props.hasProperty("alpha")) {
            m_alphaU = m_alphaV = props.getFloat("alpha");
            if (props.hasProperty("alphaU") || props.hasProperty("alphaV"))
                SLog(EError, "Microfacet model: please specify either 'alpha' or 'alphaU'/'alphaV'.");
        } else if (props.hasProperty("alphaU") || props.hasProperty("alphaV")) {
            if (!props.hasProperty("alphaU") || !props.hasProperty("alphaV"))
                SLog(EError, "Microfacet model: both 'alphaU' and 'alphaV' must be specified.");
            if (props.hasProperty("alpha"))
                SLog(EError, "Microfacet model: please specify either 'alpha' or 'alphaU'/'alphaV'.");
            m_alphaU = props.getFloat("alphaU");
            m_alphaV = props.getFloat("alphaV");
        }

        if (m_alphaU == 0 || m_alphaV == 0) {
            SLog(EWarn, "Cannot create a microfacet distribution with alphaU/alphaV=0 (clamped to 0.0001)."
                    "Please use the corresponding smooth reflectance model to get zero roughness.");
        }

        m_alphaU = std::max(m_alphaU, (Float) 1e-4f);
        m_alphaV = std::max(m_alphaV, (Float) 1e-4f);

        m_sampleVisible = props.getBoolean("sampleVisible", sampleVisible);

        /* Visible normal sampling is not supported for the Phong / Ashikhmin-Shirley distribution */
        if (m_type == EPhong) {
            m_sampleVisible = false;
            computePhongExponent();
        }
    }

    /// Return the distribution type
    inline EType getType() const { return m_type; }

    /// Return the roughness (isotropic case)
    inline Float getAlpha() const { return m_alphaU; }

    /// Return the roughness along the tangent direction
    inline Float getAlphaU() const { return m_alphaU; }

    /// Return the roughness along the bitangent direction
    inline Float getAlphaV() const { return m_alphaV; }

    /// Return the Phong exponent (isotropic case)
    inline Float getExponent() const { return m_exponentU; }

    /// Return the Phong exponent along the tangent direction
    inline Float getExponentU() const { return m_exponentU; }

    /// Return the Phong exponent along the bitangent direction
    inline Float getExponentV() const { return m_exponentV; }

    /// Return whether or not only visible normals are sampled?
    inline bool getSampleVisible() const { return m_sampleVisible; }

    /// Is this an anisotropic microfacet distribution?
    inline bool isAnisotropic() const { return m_alphaU != m_alphaV; }

    /// Is this an anisotropic microfacet distribution?
    inline bool isIsotropic() const { return m_alphaU == m_alphaV; }

    /// Scale the roughness values by some constant
    inline void scaleAlpha(Float value) {
        m_alphaU *= value;
        m_alphaV *= value;
        if (m_type == EPhong)
            computePhongExponent();
    }

    /**
     * \brief Evaluate the microfacet distribution function
     *
     * \param m
     *     The microfacet normal
     */
    inline Float eval(const Vector &m) const {
        if (Frame::cosTheta(m) <= 0)
            return 0.0f;

        Float cosTheta2 = Frame::cosTheta2(m);
        Float beckmannExponent = ((m.x*m.x) / (m_alphaU * m_alphaU)
                + (m.y*m.y) / (m_alphaV * m_alphaV)) / cosTheta2;

        Float result;
        switch (m_type) {
            case EBeckmann: {
                    /* Beckmann distribution function for Gaussian random surfaces - [Walter 2005] evaluation */
                    result = math::fastexp(-beckmannExponent) /
                        (M_PI * m_alphaU * m_alphaV * cosTheta2 * cosTheta2);
                }
                break;

            case EGGX: {
                    /* GGX / Trowbridge-Reitz distribution function for rough surfaces */
                    Float root = ((Float) 1 + beckmannExponent) * cosTheta2;
                    result = (Float) 1 / (M_PI * m_alphaU * m_alphaV * root * root);
                }
                break;

            case EPhong: {
                    /* Isotropic case: Phong distribution. Anisotropic case: Ashikhmin-Shirley distribution */
                    Float exponent = interpolatePhongExponent(m);
                    result = std::sqrt((m_exponentU + 2) * (m_exponentV + 2))
                        * INV_TWOPI * std::pow(Frame::cosTheta(m), exponent);
                }
                break;


            default:
                SLog(EError, "Invalid distribution type!");
                return -1;
        }

        /* Prevent potential numerical issues in other stages of the model */
        if (result * Frame::cosTheta(m) < 1e-20f)
            result = 0;

        return result;
    }

    /**
     * \brief Wrapper function which calls \ref sampleAll() or \ref sampleVisible()
     * depending on the parameters of this class
     */
    inline Normal sample(const Vector &wi, const Point2 &sample, Float &pdf) const {
        Normal m;
        if (m_sampleVisible) {
            m = sampleVisible(wi, sample);
            pdf = pdfVisible(wi, m);
        } else {
            m = sampleAll(sample, pdf);
        }
        return m;
    }

    /**
     * \brief Wrapper function which calls \ref sampleAll() or \ref sampleVisible()
     * depending on the parameters of this class
     */
    inline Normal sample(const Vector &wi, const Point2 &sample) const {
        Normal m;
        if (m_sampleVisible) {
            m = sampleVisible(wi, sample);
        } else {
            Float pdf;
            m = sampleAll(sample, pdf);
        }
        return m;
    }

    /**
     * \brief Wrapper function which calls \ref pdfAll() or \ref pdfVisible()
     * depending on the parameters of this class
     */
    inline Float pdf(const Vector &wi, const Vector &m) const {
        if (m_sampleVisible)
            return pdfVisible(wi, m);
        else
            return pdfAll(m);
    }

    /**
     * \brief Draw a sample from the microfacet normal distribution
     * (including *all* normals) and return the associated
     * probability density
     *
     * \param sample
     *    A uniformly distributed 2D sample
     * \param pdf
     *    The probability density wrt. solid angles
     */
    inline Normal sampleAll(const Point2 &sample, Float &pdf) const {
        /* The azimuthal component is always selected
           uniformly regardless of the distribution */
        Float cosThetaM = 0.0f;
        Float sinPhiM, cosPhiM;
        Float alphaSqr;

        switch (m_type) {
            case EBeckmann: {
                    /* Beckmann distribution function for Gaussian random surfaces */
                    if (isIsotropic()) {
                        /* Sample phi component (isotropic case) */
                        math::sincos((2.0f * M_PI) * sample.y, &sinPhiM, &cosPhiM);

                        alphaSqr = m_alphaU * m_alphaU;
                    } else {
                        /* Sample phi component (anisotropic case) */
                        Float phiM = std::atan(m_alphaV / m_alphaU *
                            std::tan(M_PI + 2*M_PI*sample.y)) + M_PI * std::floor(2*sample.y + 0.5f);
                        math::sincos(phiM, &sinPhiM, &cosPhiM);

                        Float cosSc = cosPhiM / m_alphaU, sinSc = sinPhiM / m_alphaV;
                        alphaSqr = 1.0f / (cosSc*cosSc + sinSc*sinSc);
                    }

                    /* Sample theta component */
                    Float tanThetaMSqr = alphaSqr * -math::fastlog(1.0f - sample.x);
                    cosThetaM = 1.0f / std::sqrt(1.0f + tanThetaMSqr);

                    /* Compute probability density of the sampled position */
                    pdf = (1.0f - sample.x) / (M_PI*m_alphaU*m_alphaV*cosThetaM*cosThetaM*cosThetaM);
                }
                break;

            case EGGX: {
                    /* GGX / Trowbridge-Reitz distribution function for rough surfaces */
                    if (isIsotropic()) {
                        /* Sample phi component (isotropic case) */
                        math::sincos((2.0f * M_PI) * sample.y, &sinPhiM, &cosPhiM);

                        /* Sample theta component */
                        alphaSqr = m_alphaU*m_alphaU;
                    } else {
                        /* Sample phi component (anisotropic case) */
                        Float phiM = std::atan(m_alphaV / m_alphaU *
                            std::tan(M_PI + 2*M_PI*sample.y)) + M_PI * std::floor(2*sample.y + 0.5f);
                        math::sincos(phiM, &sinPhiM, &cosPhiM);

                        Float cosSc = cosPhiM / m_alphaU, sinSc = sinPhiM / m_alphaV;
                        alphaSqr = 1.0f / (cosSc*cosSc + sinSc*sinSc);
                    }

                    /* Sample theta component */
                    Float tanThetaMSqr = alphaSqr * sample.x / (1.0f - sample.x);
                    cosThetaM = 1.0f / std::sqrt(1.0f + tanThetaMSqr);

                    /* Compute probability density of the sampled position */
                    Float temp = 1+tanThetaMSqr/alphaSqr;
                    pdf = INV_PI / (m_alphaU*m_alphaV*cosThetaM*cosThetaM*cosThetaM*temp*temp);
                }
                break;

            case EPhong: {
                    Float phiM;
                    Float exponent;
                    if (isIsotropic()) {
                        phiM = (2.0f * M_PI) * sample.y;
                        exponent = m_exponentU;
                    } else {
                        /* Sampling method based on code from PBRT */
                        if (sample.y < 0.25f) {
                            sampleFirstQuadrant(4 * sample.y, phiM, exponent);
                        } else if (sample.y < 0.5f) {
                            sampleFirstQuadrant(4 * (0.5f - sample.y), phiM, exponent);
                            phiM = M_PI - phiM;
                        } else if (sample.y < 0.75f) {
                            sampleFirstQuadrant(4 * (sample.y - 0.5f), phiM, exponent);
                            phiM += M_PI;
                        } else {
                            sampleFirstQuadrant(4 * (1 - sample.y), phiM, exponent);
                            phiM = 2 * M_PI - phiM;
                        }
                    }
                    math::sincos(phiM, &sinPhiM, &cosPhiM);
                    cosThetaM = std::pow(sample.x, 1.0f / (exponent + 2.0f));
                    pdf = std::sqrt((m_exponentU + 2.0f) * (m_exponentV + 2.0f))
                        * INV_TWOPI * std::pow(cosThetaM, exponent + 1.0f);
                }
                break;

            default:
                SLog(EError, "Invalid distribution type!");
                pdf = -1;
                return Vector(-1);
        }

        /* Prevent potential numerical issues in other stages of the model */
        if (pdf < 1e-20f)
            pdf = 0;

        Float sinThetaM = std::sqrt(
            std::max((Float) 0, 1 - cosThetaM*cosThetaM));

        return Vector(
            sinThetaM * cosPhiM,
            sinThetaM * sinPhiM,
            cosThetaM
        );
    }

    /**
     * \brief Returns the density function associated with
     * the \ref sampleAll() function.
     *
     * \param m
     *     The microfacet normal
     */
    inline Float pdfAll(const Vector &m) const {
        /* PDF is just D(m) * cos(theta_M) */
        return eval(m) * Frame::cosTheta(m);
    }


    /**
     * \brief Draw a sample from the distribution of visible normals
     * and return the associated probability density
     *
     * \param _wi
     *    A reference direction that defines the set of visible normals
     * \param sample
     *    A uniformly distributed 2D sample
     * \param pdf
     *    The probability density wrt. solid angles
     */
    inline Normal sampleVisible(const Vector &_wi, const Point2 &sample) const {
        /* Step 1: stretch wi */
        Vector wi = normalize(Vector(
            m_alphaU * _wi.x,
            m_alphaV * _wi.y,
            _wi.z
        ));

        /* Get polar coordinates */
        Float theta = 0, phi = 0;
        if (wi.z < (Float) 0.99999) {
            theta = std::acos(wi.z);
            phi = std::atan2(wi.y, wi.x);
        }
        Float sinPhi, cosPhi;
        math::sincos(phi, &sinPhi, &cosPhi);

        /* Step 2: simulate P22_{wi}(slope.x, slope.y, 1, 1) */
        Vector2 slope = sampleVisible11(theta, sample);

        /* Step 3: rotate */
        slope = Vector2(
            cosPhi * slope.x - sinPhi * slope.y,
            sinPhi * slope.x + cosPhi * slope.y);

        /* Step 4: unstretch */
        slope.x *= m_alphaU;
        slope.y *= m_alphaV;

        /* Step 5: compute normal */
        Float normalization = (Float) 1 / std::sqrt(slope.x*slope.x
                + slope.y*slope.y + (Float) 1.0);

        return Normal(
            -slope.x * normalization,
            -slope.y * normalization,
            normalization
        );
    }

    /// Implements the probability density of the function \ref sampleVisible()
    Float pdfVisible(const Vector &wi, const Vector &m) const {
        if (Frame::cosTheta(wi) == 0)
            return 0.0f;
        return smithG1(wi, m) * absDot(wi, m) * eval(m) / std::abs(Frame::cosTheta(wi));
    }

    /**
     * \brief Smith's shadowing-masking function G1 for each
     * of the supported microfacet distributions
     *
     * \param v
     *     An arbitrary direction
     * \param m
     *     The microfacet normal
     */
    Float smithG1(const Vector &v, const Vector &m) const {
        /* Ensure consistent orientation (can't see the back
           of the microfacet from the front and vice versa) */
        if (dot(v, m) * Frame::cosTheta(v) <= 0)
            return 0.0f;

        /* Perpendicular incidence -- no shadowing/masking */
        Float tanTheta = std::abs(Frame::tanTheta(v));
        if (tanTheta == 0.0f)
            return 1.0f;

        Float alpha = projectRoughness(v);
        switch (m_type) {
            case EPhong:
            case EBeckmann: {
                    Float a = 1.0f / (alpha * tanTheta);
                    if (a >= 1.6f)
                        return 1.0f;

                    /* Use a fast and accurate (<0.35% rel. error) rational
                       approximation to the shadowing-masking function */
                    Float aSqr = a*a;
                    return (3.535f * a + 2.181f * aSqr)
                         / (1.0f + 2.276f * a + 2.577f * aSqr);
                }
                break;

            case EGGX: {
                    Float root = alpha * tanTheta;
                    return 2.0f / (1.0f + math::hypot2((Float) 1.0f, root));
                }
                break;

            default:
                SLog(EError, "Invalid distribution type!");
                return -1.0f;
        }
    }

    /**
     * \brief Separable shadow-masking function based on Smith's
     * one-dimensional masking model
     */
    inline Float G(const Vector &wi, const Vector &wo, const Vector &m) const {
        return smithG1(wi, m) * smithG1(wo, m);
    }

    /// Return a string representation of the name of a distribution
    inline static std::string distributionName(EType type) {
        switch (type) {
            case EBeckmann: return "beckmann"; break;
            case EGGX: return "ggx"; break;
            case EPhong: return "phong"; break;
            default: return "invalid"; break;
        }
    }

    /// Return a string representation of the contents of this instance
    std::string toString() const {
        return formatString("MicrofacetDistribution[type=\"%s\", alphaU=%f, alphaV=%f]",
            distributionName(m_type).c_str(), m_alphaU, m_alphaV);
    }
protected:
    /// Compute the effective roughness projected on direction \c v
    inline Float projectRoughness(const Vector &v) const {
        Float invSinTheta2 = 1 / Frame::sinTheta2(v);

        if (isIsotropic() || invSinTheta2 <= 0)
            return m_alphaU;

        Float cosPhi2 = v.x * v.x * invSinTheta2;
        Float sinPhi2 = v.y * v.y * invSinTheta2;

        return std::sqrt(cosPhi2 * m_alphaU * m_alphaU + sinPhi2 * m_alphaV * m_alphaV);
    }

    /// Compute the interpolated roughness for the Phong model
    inline Float interpolatePhongExponent(const Vector &v) const {
        const Float sinTheta2 = Frame::sinTheta2(v);

        if (isIsotropic() || sinTheta2 <= RCPOVERFLOW)
            return m_exponentU;

        Float invSinTheta2 = 1 / sinTheta2;
        Float cosPhi2 = v.x * v.x * invSinTheta2;
        Float sinPhi2 = v.y * v.y * invSinTheta2;

        return m_exponentU * cosPhi2 + m_exponentV * sinPhi2;
    }

    /**
     * \brief Visible normal sampling code for the alpha=1 case
     *
     * Source: supplemental material of "Importance Sampling
     * Microfacet-Based BSDFs using the Distribution of Visible Normals"
     */
    Vector2 sampleVisible11(Float thetaI, Point2 sample) const {
        const Float SQRT_PI_INV = 1 / std::sqrt(M_PI);
        Vector2 slope;

        switch (m_type) {
            case EBeckmann: {
                    /* Special case (normal incidence) */
                    if (thetaI < 1e-4f) {
                        Float sinPhi, cosPhi;
                        Float r = std::sqrt(-math::fastlog(1.0f-sample.x));
                        math::sincos(2 * M_PI * sample.y, &sinPhi, &cosPhi);
                        return Vector2(r * cosPhi, r * sinPhi);
                    }

                    /* The original inversion routine from the paper contained
                       discontinuities, which causes issues for QMC integration
                       and techniques like Kelemen-style MLT. The following code
                       performs a numerical inversion with better behavior */
                    Float tanThetaI = std::tan(thetaI);
                    Float cotThetaI = 1 / tanThetaI;

                    /* Search interval -- everything is parameterized
                       in the erf() domain */
                    Float a = -1, c = math::erf(cotThetaI);
                    Float sample_x = std::max(sample.x, (Float) 1e-6f);

                    /* Start with a good initial guess */
                    //Float b = (1-sample_x) * a + sample_x * c;

                    /* We can do better (inverse of an approximation computed in Mathematica) */
                    Float fit = 1 + thetaI*(-0.876f + thetaI * (0.4265f - 0.0594f*thetaI));
                    Float b = c - (1+c) * std::pow(1-sample_x, fit);

                    /* Normalization factor for the CDF */
                    Float normalization = 1 / (1 + c + SQRT_PI_INV*
                        tanThetaI*std::exp(-cotThetaI*cotThetaI));

                    int it = 0;
                    while (++it < 10) {
                        /* Bisection criterion -- the oddly-looking
                           boolean expression are intentional to check
                           for NaNs at little additional cost */
                        if (!(b >= a && b <= c))
                            b = 0.5f * (a + c);

                        /* Evaluate the CDF and its derivative
                           (i.e. the density function) */
                        Float invErf = math::erfinv(b);
                        Float value = normalization*(1 + b + SQRT_PI_INV*
                            tanThetaI*std::exp(-invErf*invErf)) - sample_x;
                        Float derivative = normalization * (1
                            - invErf*tanThetaI);

                        if (std::abs(value) < 1e-5f)
                            break;

                        /* Update bisection intervals */
                        if (value > 0)
                            c = b;
                        else
                            a = b;

                        b -= value / derivative;
                    }

                    /* Now convert back into a slope value */
                    slope.x = math::erfinv(b);

                    /* Simulate Y component */
                    slope.y = math::erfinv(2.0f*std::max(sample.y, (Float) 1e-6f) - 1.0f);
                };
                break;

            case EGGX: {
                    /* Special case (normal incidence) */
                    if (thetaI < 1e-4f) {
                        Float sinPhi, cosPhi;
                        Float r = math::safe_sqrt(sample.x / (1 - sample.x));
                        math::sincos(2 * M_PI * sample.y, &sinPhi, &cosPhi);
                        return Vector2(r * cosPhi, r * sinPhi);
                    }

                    /* Precomputations */
                    Float tanThetaI = std::tan(thetaI);
                    Float a = 1 / tanThetaI;
                    Float G1 = 2.0f / (1.0f + math::safe_sqrt(1.0f + 1.0f / (a*a)));

                    /* Simulate X component */
                    Float A = 2.0f * sample.x / G1 - 1.0f;
                    if (std::abs(A) == 1)
                        A -= math::signum(A)*Epsilon;
                    Float tmp = 1.0f / (A*A - 1.0f);
                    Float B = tanThetaI;
                    Float D = math::safe_sqrt(B*B*tmp*tmp - (A*A - B*B) * tmp);
                    Float slope_x_1 = B * tmp - D;
                    Float slope_x_2 = B * tmp + D;
                    slope.x = (A < 0.0f || slope_x_2 > 1.0f / tanThetaI) ? slope_x_1 : slope_x_2;

                    /* Simulate Y component */
                    Float S;
                    if (sample.y > 0.5f) {
                        S = 1.0f;
                        sample.y = 2.0f * (sample.y - 0.5f);
                    } else {
                        S = -1.0f;
                        sample.y = 2.0f * (0.5f - sample.y);
                    }

                    /* Improved fit */
                    Float z =
                        (sample.y * (sample.y * (sample.y * (-(Float) 0.365728915865723) + (Float) 0.790235037209296) -
                            (Float) 0.424965825137544) + (Float) 0.000152998850436920) /
                        (sample.y * (sample.y * (sample.y * (sample.y * (Float) 0.169507819808272 - (Float) 0.397203533833404) -
                            (Float) 0.232500544458471) + (Float) 1) - (Float) 0.539825872510702);

                    slope.y = S * z * std::sqrt(1.0f + slope.x*slope.x);
                };
                break;

            default:
                SLog(EError, "Invalid distribution type!");
                return Vector2(-1);
        };
        return slope;
    }


    /// Helper routine: convert from Beckmann-style roughness values to Phong exponents (Walter et al.)
    void computePhongExponent() {
        m_exponentU = std::max(2.0f / (m_alphaU * m_alphaU) - 2.0f, (Float) 0.0f);
        m_exponentV = std::max(2.0f / (m_alphaV * m_alphaV) - 2.0f, (Float) 0.0f);
    }

    /// Helper routine: sample the azimuthal part of the first quadrant of the A&S distribution
    void sampleFirstQuadrant(Float u1, Float &phi, Float &exponent) const {
        Float cosPhi, sinPhi;
        phi = std::atan(
                std::sqrt((m_exponentU + 2.0f) / (m_exponentV + 2.0f)) *
                std::tan(M_PI * u1 * 0.5f));
        math::sincos(phi, &sinPhi, &cosPhi);
        /* Return the interpolated roughness */
        exponent = m_exponentU * cosPhi * cosPhi + m_exponentV * sinPhi * sinPhi;
    }
protected:
    EType m_type;
    Float m_alphaU, m_alphaV;
    bool m_sampleVisible;
    Float m_exponentU, m_exponentV;
};

MTS_NAMESPACE_END

#endif /* __MICROFACET_H */


```
