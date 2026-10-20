//! Windowless orthographic clay review of the actual gameplay sculpt.
//! Geometry inspection only: no game lighting, grooming, feature meshes or gait.
use glam::Vec3;
use image::{Rgb, RgbImage};
use voxy_model::animals::{Breed, build_sculpt};
fn main() -> Result<(),Box<dyn std::error::Error>> {
    let args:Vec<_>=std::env::args().collect();
    let output=args.get(1).map(String::as_str).unwrap_or("artifacts/animal-silhouettes");
    std::fs::create_dir_all(output)?;
    let breeds=if let Some(id)=args.get(2) {vec![Breed::parse(id)?]}else{Breed::ALL.to_vec()};
    for breed in breeds {
        let mesh=build_sculpt(breed.anatomy())?;
        let views=[(-Vec3::Z,Vec3::X),(Vec3::X,Vec3::Z),
            (Vec3::new(0.8,0.,-0.6),Vec3::new(0.6,0.,0.8))];
        let (w,h)=(600_u32,600_u32);
        let bounds:Vec<_>=views.iter().map(|(right,_)| {
            mesh.positions.iter().fold((f32::INFINITY,f32::NEG_INFINITY),|(lo,hi),&p| {
                let x=Vec3::from(p).dot(*right);(lo.min(x),hi.max(x))
            })
        }).collect();
        let (bottom,top)=mesh.positions.iter().fold((f32::INFINITY,f32::NEG_INFINITY),|(lo,hi),p|
            (lo.min(p[1]),hi.max(p[1])));
        let width=bounds.iter().map(|(lo,hi)|hi-lo).fold(0.0_f32,f32::max);
        let scale=((w-60) as f32/width).min((h-60) as f32/(top-bottom));
        let mut img=RgbImage::from_pixel(w*3,h,Rgb([238,240,242]));
        let mut depth=vec![f32::NEG_INFINITY;(w*3*h) as usize];
        for (view, &(right,forward)) in views.iter().enumerate() {
            let (lo,hi)=bounds[view];
            let project=|p:Vec3| Vec3::new((p.dot(right)-(lo+hi)*0.5)*scale+w as f32*0.5,
                (h-30) as f32-(p.y-bottom)*scale,p.dot(forward));
            for tri in mesh.indices.chunks_exact(3) {
                let p:[Vec3;3]=std::array::from_fn(|k|project(Vec3::from(mesh.positions[tri[k] as usize])));
                let n:[Vec3;3]=std::array::from_fn(|k|Vec3::from(mesh.normals[tri[k] as usize]));
                let area=(p[1]-p[0]).truncate().perp_dot((p[2]-p[0]).truncate());
                if area.abs()<1e-8 {continue;}
                let low=p.iter().copied().fold(Vec3::splat(f32::INFINITY),Vec3::min);
                let high=p.iter().copied().fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max);
                for y in (low.y.floor() as i32).max(0)..=(high.y.ceil() as i32).min(h as i32-1) {
                    for x in (low.x.floor() as i32).max(0)..=(high.x.ceil() as i32).min(w as i32-1) {
                        let q=glam::Vec2::new(x as f32+0.5,y as f32+0.5);
                        let b=(q-p[0].truncate()).perp_dot((p[2]-p[0]).truncate())/area;
                        let c=(p[1]-p[0]).truncate().perp_dot(q-p[0].truncate())/area;
                        let a=1.-b-c;
                        if a < -1e-5 || b < -1e-5 || c < -1e-5 {continue;}
                        let z=p[0].z*a+p[1].z*b+p[2].z*c;
                        let px=x as u32+view as u32*w;
                        let index=(y as u32*w*3+px) as usize;
                        if z<=depth[index] {continue;}
                        depth[index]=z;
                        let normal=(n[0]*a+n[1]*b+n[2]*c).normalize_or_zero();
                        let light=(right*0.3+Vec3::Y*0.8+forward*0.5).normalize();
                        let value=(92.+128.*normal.dot(light).max(0.)) as u8;
                        img.put_pixel(px,y as u32,Rgb([value,value,value]));
                    }
                }
            }
        }
        let path=format!("{output}/{}.png",breed.id());
        img.save(&path)?;
        println!("{}: {} vertices, {} triangles -> {path}",breed.id(),mesh.positions.len(),mesh.indices.len()/3);
    }
    Ok(())
}
