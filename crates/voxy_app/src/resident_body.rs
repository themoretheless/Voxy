//! Resident base-body deformation; CPU remains owner of physics and face details.
use super::FemaleDemo;
use glam::{Mat4,Vec3};
use voxy_render::{SceneError,SceneGeometry,SceneMesh,SceneRenderer,SurfaceDeformation};
#[derive(Debug)]
pub(crate) struct ResidentBody { surface:SurfaceDeformation, dirty:bool, base_vertices:usize }
impl ResidentBody {
    fn inputs(demo:&FemaleDemo,full:&SceneMesh)->Result<(SceneMesh,Vec<Mat4>,Vec3),SceneError> {
        let stance=demo.physical_stance.as_ref().ok_or(SceneError::InvalidTransform)?;
        if demo.film.is_some() || demo.face_parameters!=Default::default()
            || demo.render_body_parameters()!=Default::default() {
            return Err(SceneError::InvalidGeometry);
        }
        let mut vertices=demo.pre_skin_surface_vertices();
        if full.vertices().len()<vertices.len() {return Err(SceneError::InvalidGeometry);}
        for(source,posed) in vertices.iter_mut().zip(full.vertices()) {source.uv=posed.uv;source.color=posed.color;}
        let source=SceneMesh::new(vertices,demo.indices.clone())?
            .with_normals(demo.normals.iter().map(|n|n.to_array()).collect())?;
        Ok((source,stance.palette.clone(),stance.offset.as_vec3()))
    }
    pub fn new(device:&wgpu::Device,queue:&wgpu::Queue,renderer:&SceneRenderer,demo:&FemaleDemo,full:&SceneMesh,target:&SceneGeometry)->Result<Self,SceneError> {
        let(source,palette,_)=Self::inputs(demo,full)?;
        let(offsets,weights,controls)=demo.gpu_shell_binding(source.vertices().len())?;
        let mut surface=renderer.prepare_surface_deformation_in_geometry(device,&source,target,&offsets,&weights,controls)?;
        let bind=SceneMesh::new(demo.vertices.clone(),demo.indices.clone())?
            .with_normals(demo.normals.iter().map(|n|n.to_array()).collect())?;
        surface.set_normal_reference(device,&bind)?;
        surface.enable_rigid_skinning(device,source.authored_normals().ok_or(SceneError::InvalidGeometry)?,&demo.gpu_jump_weights(),palette.len() as u32,None)?;
        let mut result=Self{surface,dirty:false,base_vertices:source.vertices().len()};result.refresh(queue,demo,full)?;Ok(result)
    }
    pub fn base_vertices(&self)->usize {self.base_vertices}
    pub fn refresh(&mut self,queue:&wgpu::Queue,demo:&FemaleDemo,full:&SceneMesh)->Result<(),SceneError> {
        let(source,palette,offset)=Self::inputs(demo,full)?;
        let controls=demo.gpu_shell_controls()?;
        self.surface.upload_source_mesh(queue,&source)?;
        self.surface.upload_rigid_pose_with_offset(queue,&palette,offset)?;
        self.surface.update_controls(queue,&controls)?;
        self.dirty=true;Ok(())
    }
    pub fn encode(&mut self,encoder:&mut wgpu::CommandEncoder)->Result<(),SceneError> {
        if self.dirty {self.surface.encode(encoder)?;self.dirty=false;}
        Ok(())
    }
    #[cfg(test)]
    pub fn surface(&self)->&SurfaceDeformation {&self.surface}
}
