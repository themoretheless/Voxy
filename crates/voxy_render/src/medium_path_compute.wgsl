// === OPTIMIZATION #16-20: Workgroup size batch processing ===
// All dielectric branches in reduced radiance L/n^2. Bounded private frontier.
struct OpticalBranch {origin:vec3f,direction:vec3f,weight:vec3f,medium:u32}
fn finish_transport(pixel:u32,radiance:vec3f,tail:vec3f,rays:u32,status:u32,count:u32) {
    let color=words[3]+4u*pixel;let diag_base=words[3]+4u*words[1]+8u*pixel;
    // Failed candidates contain no partial published RGB. Host decode rejects
    // the entire batch; native whole-frame acceptance is a separate integration.
    let accepted=status==0u;let rgb=select(vec3f(0.0),radiance,accepted);
    for(var k=0u;k<3u;k+=1u) {words[color+k]=bitcast<u32>(rgb[k]);words[diag_base+k]=bitcast<u32>(tail[k]);}
    words[color+3u]=bitcast<u32>(select(0.0,1.0,accepted));
    words[diag_base+3u]=rays;words[diag_base+4u]=status;words[diag_base+5u]=count;
    words[diag_base+6u]=0u;words[diag_base+7u]=0u;
}
@compute @workgroup_size(8, 8, 1) fn cs_main(@builtin(global_invocation_id) id:vec3u,@builtin(num_workgroups) groups:vec3u) {
    // === OPTIMIZATION #17: Parallel workgroup execution (64x threads per frame) ===
    let pixel=id.x+id.y*groups.x;
    
    if id.z!=0u || pixel>=words[1] {return;}
    
    let config=4u+24u*words[0];let medium_count=words[config];let max_rays=words[config+1u];let max_pending=words[config+2u];let media=words[config+3u];
    let lower=vector(config+4u);let upper=vector(config+8u);
    let environment=vector(config+12u);let maximum=vector(config+16u);let tolerance=vector(config+20u);
    let ray=words[2]+8u*pixel;let camera_medium=words[ray+3u];
    let camera_scale=scalar(media+12u*camera_medium+1u);
    
    var pending:array<OpticalBranch,64>;
    pending[0]=OpticalBranch(vector(ray),normalize(vector(ray+4u)),vec3f(1.0),camera_medium);
    var count=1u;var traced=0u;var radiance=vec3f(0.0);var tail=vec3f(0.0);
    
    loop {
        tail=vec3f(0.0);
        for(var i=0u;i<count;i+=1u) {tail+=pending[i].weight*maximum*camera_scale;}
        
        if !finite3(tail) || !finite3(radiance) {finish_transport(pixel,radiance,tail,traced,2u,count);return;}
        if all(tail<=tolerance) {finish_transport(pixel,radiance,tail,traced,0u,count);return;}
        if traced>=max_rays {finish_transport(pixel,radiance,tail,traced,3u,count);return;}
        
        var selected=0u;var priority=0.0;
        for(var i=0u;i<count;i+=1u) {
            let weight=pending[i].weight;let score=max(weight.x,max(weight.y,weight.z));
            if score>=priority {priority=score;selected=i;}
        }
        
        let branch=pending[selected];count-=1u;pending[selected]=pending[count];traced+=1u;
        var terminal=3.4e38;
        
        for(var axis=0u;axis<3u;axis+=1u) {
            let d=branch.direction[axis];
            if d!=0.0 {let plane=select(lower[axis],upper[axis],d>0.0);terminal=min(terminal,(plane-branch.origin[axis])/d);}
        }
        
        if !finite(terminal) || terminal<0.0 {finish_transport(pixel,radiance,tail,traced,2u,count);return;}
        
        let hit=optical_geometry_hit(branch.origin,branch.direction,0.0,terminal);
        if hit.error!=0u {finish_transport(pixel,radiance,tail,traced,hit.error,count);return;}
        
        let m=media+12u*branch.medium;let ior=scalar(m);let scale=scalar(m+1u);
        let segment=medium_homogeneous(vector(m+4u),vector(m+8u),hit.distance);
        radiance+=branch.weight*(segment.source/scale)*camera_scale;
        
        let weight=branch.weight*segment.transmission;
        if !hit.found {radiance+=weight*environment*camera_scale;continue;}
        
        let triangle=4u+24u*hit.index;
        if words[triangle+20u]==1u {
            radiance+=weight*(vector(triangle+16u)/scale)*camera_scale;continue;
        }
        
        let incident=select(words[triangle+21u],words[triangle+22u],hit.entering);
        let transmitted=select(words[triangle+22u],words[triangle+21u],hit.entering);
        if incident!=branch.medium || transmitted>=medium_count {finish_transport(pixel,radiance,tail,traced,5u,count);return;}
        
        let boundary=dielectric_boundary(branch.direction,hit.normal,ior,scalar(media+12u*transmitted));
        let reflected_weight=weight*boundary.reflectance;
        if any(reflected_weight>vec3f(0.0)) {
            if count>=max_pending {finish_transport(pixel,radiance,tail,traced,4u,count);return;}
            pending[count]=OpticalBranch(hit.position,normalize(boundary.reflected),reflected_weight,branch.medium);count+=1u;
        }
        
        let transmitted_weight=weight*boundary.transmittance;
        if boundary.has_transmission && any(transmitted_weight>vec3f(0.0)) {
            if count>=max_pending {finish_transport(pixel,radiance,tail,traced,4u,count);return;}
            pending[count]=OpticalBranch(hit.position,normalize(boundary.transmitted),transmitted_weight,transmitted);count+=1u;
        }
    }
}
