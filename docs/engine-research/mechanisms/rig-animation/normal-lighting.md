# Rig normal transport and editor lighting

The scene skin compute pass transports authored normals by the inverse transpose
of the weighted affine skin matrix. CPU baked meshes and skeletal LOD retain the
same authored normal stream. Normal cache invalidation includes authored normals,
so an unchanged vertex position does not conceal a shading change.

Preflight rejects singular and numerically uncertain normal transforms before
palette writes, retaining the last accepted GPU pose. Scaled cofactors avoid
unstable direct inversion for anisotropic transforms and preserve reflection sign.

The editor material previously derived a flat normal from position derivatives,
ignoring the normal buffer entirely. It now uses transported vertex normals with
world inverse-transpose transport; zero streams fall back to position derivatives.
Lighting remains double-sided Lambert. This is not a PBR/shadow implementation.

The focused real-GPU regression renders three identical meshes: GPU skin normals,
CPU inverse-transpose normals, and the former forward-matrix normal formula.
The first two images have zero channel difference; all 2317 covered pixels differ
with the old formula. The test also checks tangent orthogonality, reflections,
extreme anisotropy, and rejection without overwriting the accepted pose.

Limitations: locally constant blended skin matrices do not include gradients of
skin weights. Morph-target normals and continuous-time coverage are not proven.
World-transform shading is implemented but this pixel fixture uses identity world.
Full-frame performance and NVIDIA/CUDA acceptance remain separate requirements.
