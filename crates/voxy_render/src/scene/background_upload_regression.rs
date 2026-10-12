use super::*;
#[test]
fn authored_normal_cache_tracks_geometry_without_duplicate_streams() {
    let mut mesh = SceneMesh::quad([1.; 4])
        .with_normals(vec![[0., 0., 1.]; 4])
        .unwrap();
    let mut cache = NormalCache::from_mesh(&mesh);
    let allocation = cache.normals.as_ptr();
    mesh.vertices[2].position[2] = 0.25;
    assert!(
        !cache.refresh(&mesh),
        "unchanged authored normals need no GPU upload"
    );
    assert_eq!(cache.normals.as_ptr(), allocation);
    assert_eq!(cache.positions[2], mesh.vertices[2].position);
    mesh.authored_normals.as_mut().unwrap()[2] = [0., 1., 0.];
    assert!(cache.refresh(&mesh));
    assert_eq!(cache.normals, *mesh.authored_normals.as_ref().unwrap());
    mesh.authored_normals = None;
    assert!(
        cache.refresh(&mesh),
        "dropping authored normals regenerates from current geometry"
    );
    assert_eq!(cache.normals, smooth_normals(&mesh));
    assert!(!cache.refresh(&mesh));
}
#[test]
fn packed_weld_keys_preserve_seams_and_signed_zero() {
    let mut mesh = SceneMesh::quad([1.; 4]);
    mesh.vertices[2].position[2] = 0.15;
    let mut duplicate = mesh.vertices.clone();
    for v in &mut duplicate {
        if v.position[2] == 0.0 {
            v.position[2] = -0.0;
        }
    }
    mesh.vertices.extend(duplicate);
    mesh.indices.extend([4, 5, 6, 4, 6, 7]);
    let mut lookup = std::collections::HashMap::new();
    let mut mapping = Vec::new();
    let mut sums = Vec::<glam::Vec3>::new();
    for vertex in &mesh.vertices {
        let key = vertex
            .position
            .map(|x| if x == 0. { 0 } else { x.to_bits() });
        let id = *lookup.entry(key).or_insert_with(|| {
            let id = sums.len();
            sums.push(glam::Vec3::ZERO);
            id
        });
        mapping.push(id);
    }
    for tri in mesh.indices.chunks_exact(3) {
        let p = |i: u32| glam::Vec3::from_array(mesh.vertices[i as usize].position);
        let n = (p(tri[1]) - p(tri[0])).cross(p(tri[2]) - p(tri[0]));
        for &i in tri {
            sums[mapping[i as usize]] += n;
        }
    }
    for n in &mut sums {
        *n = n.normalize_or_zero();
    }
    let reference: Vec<_> = mapping.iter().map(|&i| sums[i].to_array()).collect();
    assert_eq!(smooth_normals(&mesh), reference);
}
#[test]
fn worker_preparation_preserves_default_welded_shading_and_authored_streams() {
    let mut mesh = SceneMesh::quad([0.7, 0.4, 0.2, 1.]);
    mesh.vertices[2].position[2] = 0.3;
    let fallback = NormalCache::from_mesh(&mesh);
    let coordinates = mesh.material_coordinates().into_owned();
    let ready = mesh.clone().with_prepared_upload_streams();
    let prepared = NormalCache::from_mesh(&ready);
    assert_eq!(fallback.normals, prepared.normals);
    assert_eq!(coordinates, ready.material_coordinates().as_ref());
    assert_eq!(mesh.indices, ready.indices);
    let normals = vec![[0., 0., 1.]; mesh.vertices.len()];
    let authored = mesh
        .with_normals(normals.clone())
        .unwrap()
        .with_material_coordinates(coordinates.clone())
        .unwrap()
        .with_prepared_upload_streams();
    assert_eq!(authored.authored_normals.as_ref(), Some(&normals));
    assert_eq!(authored.material_coordinates.as_ref(), Some(&coordinates));
}
