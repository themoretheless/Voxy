struct Transform { mvp: mat4x4<f32>, previous_mvp: mat4x4<f32>, camera: vec4<f32> }
@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) point: vec3<f32>,
    @location(3) to_camera: vec3<f32>,
    @location(4) smooth_normal: vec3<f32>,
    @location(6) material_point:vec3<f32>,
    @location(5) @interpolate(flat) film_optics: vec4<f32>,
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>, @location(3) normal: vec3<f32>, @location(4) film_optics: vec4<f32>, @location(5) material_point:vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = transform.mvp * vec4<f32>(position, 1.0);
    out.uv = uv;
    out.color = color;
    out.point = position;
    out.smooth_normal = normal;
    out.film_optics = film_optics;
    out.material_point=material_point;
    out.to_camera = transform.camera.xyz-position;
    return out;
}
fn ggx(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32) -> f32 {
    return ggx_f0(n,v,l,rough,0.028);
}
fn ggx_f0(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32, f0:f32) -> f32 {
    let h=normalize(v+l);
    let nl=max(dot(n,l),0.0);
    let nv=max(dot(n,v),0.0001);
    let nh=max(dot(n,h),0.0);
    let vh=max(dot(v,h),0.0);
    let a=rough*rough;
    let a2=a*a;
    let den=nh*nh*(a2-1.0)+1.0;
    let distribution=a2/(3.14159265*den*den);
    let k=(rough+1.0)*(rough+1.0)/8.0;
    let masking=nv/(nv*(1.0-k)+k)*nl/(nl*(1.0-k)+k);
    let fresnel=f0+(1.0-f0)*pow(1.0-vh,5.0);
    return fresnel*distribution*masking/(4.0*nv);
}
// Match the finite key-light disk used by the visibility probe. Averaging
// both skin lobes avoids shading a softbox as a single directional point.
fn skin_area_highlight(n:vec3<f32>,v:vec3<f32>,axis:vec3<f32>,rough:f32,oil:f32)->f32 {
    let tangent=normalize(cross(axis,vec3<f32>(0.0,1.0,0.0)));
    let bitangent=cross(axis,tangent);
    var result=0.0;
    for(var sample=0;sample<16;sample+=1){
        let r=0.22*sqrt((f32(sample)+0.5)/16.0);
        let angle=f32(sample)*2.3999631;
        let light=normalize(axis+tangent*(r*cos(angle))+bitangent*(r*sin(angle)));
        // Approximate a dielectric oil coat rather than adding both lobes
        // independently at grazing angles. Coverage remains an artistic weight;
        // refraction and the diffuse subsurface layer are not solved here.
        let coverage=0.35*oil;
        let reflected_in=0.035+0.965*pow(1.0-max(dot(n,light),0.0),5.0);
        let reflected_out=0.035+0.965*pow(1.0-max(dot(n,v),0.0),5.0);
        let transmission=(1.0-coverage*reflected_in)*(1.0-coverage*reflected_out);
        result+=0.8*transmission*ggx(n,v,light,rough)+coverage*ggx_f0(n,v,light,0.20,0.035);
    }
    return result/16.0;
}
// Average the oral GGX lobe over the same finite key-light disk as the skin.
fn oral_area_highlight(n:vec3<f32>,v:vec3<f32>,axis:vec3<f32>,rough:f32)->f32 {
    let tangent=normalize(cross(axis,vec3<f32>(0.0,1.0,0.0)));
    let bitangent=cross(axis,tangent);
    var result=0.0;
    for(var sample=0;sample<16;sample+=1){
        let radius=0.22*sqrt((f32(sample)+0.5)/16.0);
        let angle=f32(sample)*2.3999631;
        let light=normalize(axis+tangent*(radius*cos(angle))+bitangent*(radius*sin(angle)));
        result+=ggx(n,v,light,rough);
    }
    return result/16.0;
}
// Reflection of a finite disk emitter through a wet dielectric surface.
fn corneal_catchlight(n: vec3<f32>, v: vec3<f32>, source: vec3<f32>, angular_radius: f32, radiance: f32) -> f32 {
    let reflection=reflect(-v,n);
    let alignment=dot(reflection,source);
    let threshold=inverseSqrt(1.0+angular_radius*angular_radius);
    let coverage=smoothstep(threshold-0.004,threshold+0.004,alignment);
    let fresnel=0.025+0.975*pow(1.0-max(dot(n,v),0.0),5.0);
    return radiance*fresnel*coverage;
}
// Preserve the limbus while moving the pupil boundary. The same mapping
// feeds pigment and relief; the centre is finite and sclera is unchanged.
fn iris_rest_point(p:vec2<f32>,pupil_radius:f32)->vec2<f32> {
    let r=length(p);
    let outer=0.0057;
    let rest=0.002025;
    let pupil=clamp(pupil_radius,0.001,0.004);
    if r>=outer { return p; }
    if r<0.00000001 { return vec2<f32>(0.0); }
    let source=select(rest+(r-pupil)*(outer-rest)/(outer-pupil),r*rest/pupil,r<pupil);
    return p*(source/r);
}
// Analytic eye pigmentation evaluated at fragment resolution in bind metres.
fn eye_pigment(point: vec2<f32>,pupil_radius:f32) -> vec3<f32> {
    let p=iris_rest_point(point,pupil_radius);
    let r=length(p); let angle=atan2(p.y,p.x);
    var vessels=0.0;
    for(var i=0;i<9;i=i+1) {
        let phase=f32(i)*2.399963;
        let axis=phase+0.09*sin(r*950.0+phase);
        let d=abs(atan2(sin(angle-axis),cos(angle-axis)))*r;
        let main=1.0-smoothstep(0.000045,0.00016,d);
        let split=axis+0.12*smoothstep(0.0072,0.0095,r);
        let sd=abs(atan2(sin(angle-split),cos(angle-split)))*r;
        let fork=(1.0-smoothstep(0.000025,0.00011,sd))*smoothstep(0.0073,0.0082,r);
        vessels=max(vessels,max(main,fork)*smoothstep(0.0063,0.008,r));
    }
    let sclera=mix(vec3<f32>(0.78,0.79,0.75),vec3<f32>(0.78,0.30,0.28),vessels*0.08);
    if r>0.0059 { return sclera; }
    let warp=0.055*sin(angle*7.0+1.7)+0.028*sin(angle*13.0-0.8);
    let theta=angle+warp*smoothstep(0.002,0.0055,r);
    // Radially interrupted bundles avoid uniform spokes across the whole iris.
    let bundle_a=0.45+0.55*smoothstep(-0.65,0.75,sin(r*4100.0+sin(angle*17.0)*2.3));
    let bundle_b=0.35+0.65*smoothstep(-0.7,0.8,sin(r*5700.0+sin(angle*29.0+1.1)*1.8));
    let fibers=clamp(0.5+0.19*bundle_a*sin(theta*79.0+0.7*sin(r*1800.0))
        +0.13*bundle_b*sin(theta*137.0+2.1)+0.09*sin(theta*43.0-r*900.0+0.3)
        +0.07*sin(theta*19.0+1.9)+0.08*sin(angle*5.0+r*1300.0),0.0,1.0);
    var iris=mix(vec3<f32>(0.075,0.12,0.065),vec3<f32>(0.25,0.285,0.155),fibers);
    let collar=0.0028+0.00018*sin(angle*11.0+0.4)+0.00009*sin(angle*23.0-1.2);
    let warm=vec3<f32>(0.17,0.13,0.055)*(0.7+0.6*fibers);
    iris=mix(warm,iris,smoothstep(collar-0.00045,collar+0.00035,r));
    let crypts=smoothstep(0.84,0.98,0.5+0.5*sin(angle*31.0+1.1))
        *(1.0-smoothstep(0.00008,0.0005,abs(r-collar)));
    iris=iris*(1.0-0.45*crypts);
    iris=mix(iris,vec3<f32>(0.025,0.039,0.026),smoothstep(0.00505,0.0056,r)*0.9);
    let color=mix(iris,sclera,smoothstep(0.0055,0.0059,r));
    return mix(vec3<f32>(0.0015,0.002,0.0025),color,smoothstep(0.0019,0.00215,r));
}
// Procedural iris relief in metres; restricted to the annulus so the pupil
// remains an aperture and the sclera retains its globe normal.
fn iris_height(point:vec2<f32>,pupil_radius:f32) -> f32 {
    let p=iris_rest_point(point,pupil_radius);
    let r=length(p);
    let angle=atan2(p.y,p.x);
    let collar=0.0028+0.00018*sin(angle*11.0+0.4)+0.00009*sin(angle*23.0-1.2);
    let crypts=smoothstep(0.84,0.98,0.5+0.5*sin(angle*31.0+1.1))
        *(1.0-smoothstep(0.00008,0.0005,abs(r-collar)));
    let fibers=0.000025*sin(angle*79.0+0.7*sin(r*1800.0))
        +0.000015*sin(angle*43.0-r*900.0+0.3);
    let annulus=smoothstep(0.0021,0.0024,r)*(1.0-smoothstep(0.0052,0.0057,r));
    return (fibers-0.00009*crypts)*annulus;
}
fn diagnostic_palette(value:f32) -> vec3<f32> {
    let t=clamp(value,0.0,1.0);
    return select(mix(vec3<f32>(0.0,0.1,1.0),vec3<f32>(0.0,1.0,0.1),t*2.0),
        mix(vec3<f32>(0.0,1.0,0.1),vec3<f32>(1.0,0.0,0.0),(t-0.5)*2.0),t>0.5);
}
fn skin_hash(p:vec3<f32>) -> f32 {
    let q=fract(p*vec3<f32>(0.1031,0.1030,0.0973));
    let r=q+dot(q,q.yxz+vec3<f32>(33.33));
    return fract((r.x+r.y)*r.z);
}
fn skin_noise(p:vec3<f32>) -> f32 {
    let cell=floor(p);
    let f=fract(p);
    let w=f*f*(vec3<f32>(3.0)-2.0*f);
    let lo=mix(mix(skin_hash(cell),skin_hash(cell+vec3<f32>(1.0,0.0,0.0)),w.x),
        mix(skin_hash(cell+vec3<f32>(0.0,1.0,0.0)),skin_hash(cell+vec3<f32>(1.0,1.0,0.0)),w.x),w.y);
    let hi=mix(mix(skin_hash(cell+vec3<f32>(0.0,0.0,1.0)),skin_hash(cell+vec3<f32>(1.0,0.0,1.0)),w.x),
        mix(skin_hash(cell+vec3<f32>(0.0,1.0,1.0)),skin_hash(cell+vec3<f32>(1.0,1.0,1.0)),w.x),w.y);
    return mix(lo,hi,w.z)*2.0-1.0;
}
fn filtered_microheight(uv:vec2<f32>,dx:vec2<f32>,dy:vec2<f32>)->f32 {
    let a=(dx+dy)*0.25;
    let b=(dx-dy)*0.25;
    return (textureSampleGrad(image,image_sampler,uv+a,dx,dy).b
        +textureSampleGrad(image,image_sampler,uv-a,dx,dy).b
        +textureSampleGrad(image,image_sampler,uv+b,dx,dy).b
        +textureSampleGrad(image,image_sampler,uv-b,dx,dy).b)*0.25;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // All implicit-derivative texture operations run before material branches.
    let skin_metadata=in.uv.x>=2.0;
    let texel=textureSample(image,image_sampler,select(in.uv,vec2<f32>(0.0),skin_metadata));
    let material_uv=in.uv+vec2<f32>(0.5,0.0);
    let parameters=textureSample(image,image_sampler,material_uv);
    let uv_dx=dpdx(in.uv);
    let uv_dy=dpdy(in.uv);
    let material_dx=dpdx(in.material_point);
    let material_dy=dpdy(in.material_point);
    let point_dx=dpdx(in.point);
    let point_dy=dpdy(in.point);
    let globe_dx=dpdx(in.color.rgb*2.0-vec3<f32>(1.0));
    let globe_dy=dpdy(in.color.rgb*2.0-vec3<f32>(1.0));
    // Opaque prelit fibres have their own material. They must not run skin
    // pore/pigment integration or be mistaken for a liquid film.
    if in.uv.x == -7.0 { return texel*in.color; }
    let texel_step=max(vec2<f32>(1.0/4096.0,1.0/2048.0),(abs(uv_dx)+abs(uv_dy))*0.5);
    let height_xp=filtered_microheight(material_uv+vec2<f32>(texel_step.x,0.0),uv_dx,uv_dy);
    let height_xm=filtered_microheight(material_uv-vec2<f32>(texel_step.x,0.0),uv_dx,uv_dy);
    let height_yp=filtered_microheight(material_uv+vec2<f32>(0.0,texel_step.y),uv_dx,uv_dy);
    let height_ym=filtered_microheight(material_uv-vec2<f32>(0.0,texel_step.y),uv_dx,uv_dy);

    let view=normalize(in.to_camera);
    let smooth_length2=dot(in.smooth_normal,in.smooth_normal);
    let geometric=cross(point_dx,point_dy);
    let fallback=geometric*inverseSqrt(max(dot(geometric,geometric),0.000000000001));
    var normal=select(fallback,in.smooth_normal*inverseSqrt(max(smooth_length2,0.000000000001)),smooth_length2>0.000000000001);
    normal=select(-normal,normal,dot(normal,view)>=0.0);
    if in.uv.x < -4.5 && in.uv.x > -5.5 { return in.color; }
    if in.uv.x < -3.5 && in.uv.x > -5.5 { return vec4<f32>(diagnostic_palette(in.uv.y),1.0); }
    if in.uv.x < -1.5 {
        let thickness=max(in.uv.y,0.0);
        if thickness < 0.0000001 { discard; }
        if in.uv.x < -2.5 && in.uv.x > -5.5 {
            // Fixed SI scale: blue at zero, green at 50 um, red at 100 um.
            let t=clamp(thickness/0.0001,0.0,1.0);
            let color=select(mix(vec3<f32>(0.0,0.1,1.0),vec3<f32>(0.0,1.0,0.1),t*2.0),
                mix(vec3<f32>(0.0,1.0,0.1),vec3<f32>(1.0,0.0,0.0),(t-0.5)*2.0),t>0.5);
            return vec4<f32>(color,1.0);
        }
        // Air/water dielectric boundary, IOR 1.333. Optical path length
        // follows refracted incidence; absorption uses SI inverse metres.
        let cosine=clamp(dot(normal,view),0.0001,1.0);
        let ior=clamp(in.film_optics.x,1.0,2.5);
        let eta=1.0/ior;
        let transmitted_cosine=sqrt(max(1.0-eta*eta*(1.0-cosine*cosine),0.0001));
        let rs=(cosine-ior*transmitted_cosine)/(cosine+ior*transmitted_cosine);
        let rp=(ior*cosine-transmitted_cosine)/(ior*cosine+transmitted_cosine);
        let fresnel=0.5*(rs*rs+rp*rp);
        let transmission=exp(-max(in.film_optics.yzw,vec3<f32>(0.0))*thickness/transmitted_cosine);
        let light=normalize(vec3<f32>(-0.5,0.7,1.0));
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let reference_fresnel=0.025+0.975*pow(1.0-cosine,5.0);
        let reflection=(corneal_catchlight(normal,view,light,0.22,16.0)
            +corneal_catchlight(normal,view,fill,0.30,3.0))*fresnel/reference_fresnel;
        let coverage=smoothstep(0.0000001,0.000003,thickness);
        if in.uv.x < -5.5 {
            // Single thin interface over an opaque scene colour prepass.
            // Convert the refracted tangent offset to screen pixels using the
            // local surface Jacobian; this is not an offscreen ray tracer.
            let refracted=refract(-view,normal,eta);
            let offset=thickness*(refracted/transmitted_cosine+view/cosine);
            let xx=dot(point_dx,point_dx);
            let xy=dot(point_dx,point_dy);
            let yy=dot(point_dy,point_dy);
            let determinant=max(xx*yy-xy*xy,0.000000000000000000000001);
            let ox=dot(offset,point_dx);
            let oy=dot(offset,point_dy);
            var shift=vec2<f32>(ox*yy-oy*xy,oy*xx-ox*xy)/determinant;
            if in.uv.x < -6.5 {shift=vec2<f32>(0.0);}
            let dimensions=vec2<f32>(textureDimensions(image));
            let original_uv=in.position.xy/dimensions;
            let refracted_uv=clamp((in.position.xy+shift)/dimensions,vec2<f32>(0.0),vec2<f32>(1.0));
            let original=textureSampleLevel(image,image_sampler,original_uv,0.0).rgb;
            let substrate=textureSampleLevel(image,image_sampler,refracted_uv,0.0).rgb;
            let coated=substrate*transmission*(1.0-fresnel)+vec3<f32>(reflection);
            return vec4<f32>(mix(original,coated,coverage),1.0);
        }
        // Straight-alpha over the already shaded skin, rather than replacing
        // it with vertex albedo. One alpha cannot represent RGB transmission;
        // use its darkest channel until a scene-colour refraction pass exists.
        let remaining=transmission*(1.0-fresnel);
        let opacity=coverage*(1.0-min(remaining.x,min(remaining.y,remaining.z)));
        if opacity<0.000001 { discard; }
        let reflected=vec3<f32>(reflection)*coverage/max(opacity,0.000001);
        return vec4<f32>(reflected,opacity);
    }
    let alpha=texel.a*in.color.a;
    if alpha<=0.0 {discard;}
    if in.uv.x > -0.4 && in.uv.x < -0.1 {
        // Lash shafts have no skin pores. Surface GGX is a first approximation,
        // not a longitudinal/transmitted hair-scattering model.
        let key=normalize(vec3<f32>(-0.5,0.7,1.0));
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let visibility=clamp(2.0*(in.color.a-0.5),0.0,1.0);
        let diffuse=0.2+0.6*max(dot(normal,key),0.0)+0.2*max(dot(normal,fill),0.0);
        let roughness=clamp(in.uv.y,0.25,0.8);
        let highlight=ggx(normal,view,key,roughness)*visibility+0.2*ggx(normal,view,fill,roughness);
        return vec4<f32>(in.color.rgb*diffuse+vec3<f32>(highlight),1.0);
    }
    if in.uv.x < -1.15 && in.uv.x > -1.45 {
        let bind=vec2<f32>(((in.uv.x+1.4)/0.2-0.5)*0.032,(in.uv.y-0.5)*0.036);
        let p=bind/0.0009;
        let cell=floor(p);
        var height=0.0;
        for(var i=-1;i<=1;i+=1){for(var j=-1;j<=1;j+=1){
            let c=cell+vec2<f32>(f32(i),f32(j));
            let jitter=vec2<f32>(skin_hash(vec3<f32>(c,0.23)),skin_hash(vec3<f32>(c,0.71)));
            let d=p-c-jitter;
            height+=0.00007*exp(-dot(d,d)/0.025);
        }}
        let r1=cross(point_dy,normal);
        let r2=cross(normal,point_dx);
        let det=dot(point_dx,r1);
        normal=normalize(max(abs(det),0.000000000001)*normal-sign(det)*(dpdx(height)*r1+dpdy(height)*r2));
        let light=normalize(vec3<f32>(-0.5,0.7,1.0));
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let visibility=clamp(2.0*(in.color.a-0.5),0.0,1.0);
        let diffuse=0.2+0.6*max(dot(normal,light),0.0)+0.2*max(dot(normal,fill),0.0);
        return vec4<f32>(in.color.rgb*diffuse+vec3<f32>(oral_area_highlight(normal,view,light,0.38)*visibility),1.0);
    }
    if in.uv.x < -0.5 {
        let light=normalize(vec3<f32>(-0.5,0.7,1.0));
        let roughness=clamp(in.uv.y,0.15,0.9);
        let visibility=clamp(2.0*(in.color.a-0.5),0.0,1.0);
        let highlight=oral_area_highlight(normal,view,light,roughness)*visibility;
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let irradiance=0.2+0.6*max(dot(normal,light),0.0)+0.2*max(dot(normal,fill),0.0);
        // Constant oral U=-1 means fully visible fill in the native preview.
        // Ray probe encodes its measured fill visibility in [-1,-0.75].
        let fill_visibility=clamp((-in.uv.x-0.75)*4.0,0.0,1.0);
        let fill_highlight=oral_area_highlight(normal,view,fill,roughness)*fill_visibility/3.0;
        return vec4<f32>(in.color.rgb*irradiance+vec3<f32>(highlight+fill_highlight),1.0);
    }
    if in.uv.x>=0.1 && in.uv.x<=0.3 && in.uv.y>=0.004 && in.uv.y<=0.044 {
        let eye_normal=normalize(in.color.rgb*2.0-vec3<f32>(1.0));
        let key=normalize(vec3<f32>(-0.5,0.7,1.0));
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let visibility=clamp(2.0*(in.color.a-0.5),0.0,1.0);
        let sclera_irradiance=0.2+0.6*max(dot(eye_normal,key),0.0)*visibility+0.2*max(dot(eye_normal,fill),0.0);
        let reflection=corneal_catchlight(eye_normal,view,key,0.22,16.0)*visibility
            +corneal_catchlight(eye_normal,view,fill,0.30,3.0);
        // Recover the ocular frame from the smooth globe normal field and bind UVs.
        // This frame follows head roll and gaze instead of assuming world X/Y axes.
        let local_xy=vec2<f32>((in.uv.x-0.2)*10.0,(0.024-in.uv.y)*50.0);
        let local_z=sqrt(max(1.0-dot(local_xy,local_xy),0.0001));
        let determinant=uv_dx.x*uv_dy.y-uv_dx.y*uv_dy.x;
        let reciprocal=sign(determinant)/max(abs(determinant),0.0000000001);
        let normal_x=(globe_dx*uv_dy.y-globe_dy*uv_dx.y)*reciprocal*0.1;
        let normal_y=(globe_dy*uv_dx.x-globe_dx*uv_dy.x)*reciprocal*(-0.02);
        let axis_z_raw=eye_normal-local_xy.x*normal_x-local_xy.y*normal_y;
        let axis_z=axis_z_raw*inverseSqrt(max(dot(axis_z_raw,axis_z_raw),0.00000001));
        let axis_x_raw=normal_x-axis_z*dot(normal_x,axis_z);
        let axis_x=axis_x_raw*inverseSqrt(max(dot(axis_x_raw,axis_x_raw),0.00000001));
        let axis_y=cross(axis_z,axis_x);
        let transmitted=refract(-view,eye_normal,1.0/1.376);
        // A recessed iris plane, with the globe's surface sag subtracted.
        let depth=max(0.0003,0.003-0.012*(1.0-local_z));
        let travel=depth/max(-dot(transmitted,axis_z),0.2);
        let displacement=vec2<f32>(dot(transmitted,axis_x)*0.2/0.024,
            -dot(transmitted,axis_y)*0.04/0.024)*travel;
        let cornea=1.0-smoothstep(0.0058,0.0068,length(local_xy)*0.012);
        let refracted_uv=clamp(in.uv+displacement*cornea,
            vec2<f32>(0.100001,0.004001),vec2<f32>(0.299999,0.043999));
        let pigment_point=vec2<f32>((refracted_uv.x-0.2)*0.12,(0.024-refracted_uv.y)*0.6);
        let pixel_x=vec2<f32>(uv_dx.x*0.12,-uv_dx.y*0.6)*0.25;
        let pixel_y=vec2<f32>(uv_dy.x*0.12,-uv_dy.y*0.6)*0.25;
        let pupil_radius=select(0.002025,in.material_point.z,in.material_point.z>=0.001 && in.material_point.z<=0.004);
        let pigment=(eye_pigment(pigment_point+pixel_x+pixel_y,pupil_radius)
            +eye_pigment(pigment_point+pixel_x-pixel_y,pupil_radius)
            +eye_pigment(pigment_point-pixel_x+pixel_y,pupil_radius)
            +eye_pigment(pigment_point-pixel_x-pixel_y,pupil_radius))*0.25;
        let relief_step=max(0.00004,2.0*max(length(pixel_x),length(pixel_y)));
        let gradient=vec2<f32>(
            iris_height(pigment_point+vec2<f32>(relief_step,0.0),pupil_radius)-iris_height(pigment_point-vec2<f32>(relief_step,0.0),pupil_radius),
            iris_height(pigment_point+vec2<f32>(0.0,relief_step),pupil_radius)-iris_height(pigment_point-vec2<f32>(0.0,relief_step),pupil_radius)
        )/(2.0*relief_step);
        let iris_normal=normalize(axis_z-axis_x*gradient.x-axis_y*gradient.y);
        let iris_irradiance=0.2+0.6*max(dot(iris_normal,key),0.0)*visibility+0.2*max(dot(iris_normal,fill),0.0);
        let iris_weight=1.0-smoothstep(0.0055,0.0059,length(pigment_point));
        let irradiance=mix(sclera_irradiance,iris_irradiance,iris_weight);
        let transmission=1.0-(0.025+0.975*pow(1.0-max(dot(eye_normal,view),0.0),5.0));
        return vec4<f32>(pigment*irradiance*transmission+vec3<f32>(reflection),1.0);
    }
    if in.uv.x>=0.025 && in.uv.x<=0.475 && in.uv.y>=0.05 && in.uv.y<=0.95 {
        let determinant=uv_dx.x*uv_dy.y-uv_dx.y*uv_dy.x;
        let reciprocal=sign(determinant)/max(abs(determinant),0.0000000001);
        let tangent=normalize((point_dx*uv_dy.y-point_dy*uv_dx.y)*reciprocal);
        let bitangent=normalize((point_dy*uv_dx.x-point_dx*uv_dy.x)*reciprocal);
        let gradient_x=(height_xp-height_xm)*0.002/(2.0*texel_step.x*0.2/0.45);
        let gradient_y=(height_yp-height_ym)*0.002/(2.0*texel_step.y*0.22/0.9);
        let base_normal=normal;
        normal=normalize(normal-tangent*gradient_x-bitangent*gradient_y);
        let light=normalize(vec3<f32>(-0.5,0.7,1.0));
        let roughness=clamp(parameters.r,0.15,0.9);
        let oil=clamp(parameters.g,0.0,1.0);
        let highlight=skin_area_highlight(normal,view,light,roughness,oil);
        // Optional ray visibility uses [0.5,1] without making opaque skin transparent.
        let visibility=clamp(2.0*(in.color.a-0.5),0.0,1.0);
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let base_diffuse=0.2+0.6*max(dot(base_normal,light),0.0)+0.2*max(dot(base_normal,fill),0.0);
        // FaceWorks-inspired split: millimetre-filtered relief drives diffuse,
        // while the original pixel-filtered relief continues to drive specular.
        // Explicit gradients keep sampling valid within this material branch.
        let scatter_step=max(texel_step,vec2<f32>(0.001*0.45/0.2,0.001*0.9/0.22));
        let scatter_dx=vec2<f32>(scatter_step.x,0.0);
        let scatter_dy=vec2<f32>(0.0,scatter_step.y);
        let scatter_xp=filtered_microheight(material_uv+scatter_dx,scatter_dx,scatter_dy);
        let scatter_xm=filtered_microheight(material_uv-scatter_dx,scatter_dx,scatter_dy);
        let scatter_yp=filtered_microheight(material_uv+scatter_dy,scatter_dx,scatter_dy);
        let scatter_ym=filtered_microheight(material_uv-scatter_dy,scatter_dx,scatter_dy);
        let scatter_gradient=vec2<f32>(
            (scatter_xp-scatter_xm)*0.002/(2.0*scatter_step.x*0.2/0.45),
            (scatter_yp-scatter_ym)*0.002/(2.0*scatter_step.y*0.22/0.9));
        let diffuse_normal=normalize(base_normal-tangent*scatter_gradient.x-bitangent*scatter_gradient.y);
        let relief_diffuse=0.2+0.6*max(dot(diffuse_normal,light),0.0)+0.2*max(dot(diffuse_normal,fill),0.0);
        // Correct the baked irradiance for filtered relief. Vertex ray visibility
        // remains coarse; this ratio does not trace shadows inside a crease.
        let diffuse_ratio=relief_diffuse/base_diffuse;
        return vec4<f32>(texel.rgb*in.color.rgb*diffuse_ratio+vec3<f32>(1.0,0.96,0.92)*highlight*visibility,1.0);
    }
    if all(in.uv==vec2<f32>(0.0)) || skin_metadata {
        // Procedural fallback for untextured skin. Analytic pixel-footprint
        // filtering removes subpixel relief instead of aliasing it into noise.
        var slope=vec3<f32>(0.0);
        for(var i=0;i<12;i=i+1) {
            let seed=f32(i)+1.0;
            let direction=normalize(vec3<f32>(sin(seed*2.399963),cos(seed*1.618034),sin(seed*0.754877+0.4)));
            let frequency=4200.0+seed*593.0;
            let dx=dot(material_dx,direction)*frequency;
            let dy=dot(material_dy,direction)*frequency;
            // Below 1e-9 slope contribution at this pixel footprint. Cull
            // unresolved detail before evaluating its phase, without aliasing.
            let footprint2=dx*dx+dy*dy;
            if footprint2>32.0 { continue; }
            let attenuation=exp(-0.5*footprint2);
            let phase=dot(in.material_point,direction)*frequency+seed*1.324718;
            slope+=0.0000007*frequency*cos(phase)*attenuation*direction;
        }
        let rx=cross(point_dy,normal);
        let ry=cross(normal,point_dx);
        let determinant=dot(point_dx,rx);
        let reciprocal=sign(determinant)/max(abs(determinant),1e-12);
        let tangent_slope=(dot(slope,material_dx)*rx+dot(slope,material_dy)*ry)*reciprocal;
        let skin_normal=normalize(normal-tangent_slope);
        let variation=(sin(dot(in.material_point,vec3<f32>(63.0,41.0,57.0)))
            +0.5*sin(dot(in.material_point,vec3<f32>(-37.0,71.0,43.0))+1.7))/1.5;
        let roughness=clamp(0.48+0.10*variation,0.30,0.65);
        let light=normalize(vec3<f32>(-0.5,0.7,1.0));
        let fill=normalize(vec3<f32>(0.8,0.2,0.5));
        let original=0.2+0.6*max(dot(normal,light),0.0)+0.2*max(dot(normal,fill),0.0);
        // Normalized spectral wrapped diffuse is an inexpensive surface-only
        // approximation. It does not transport light through tissue or shadows.
        // Wider red scattering softens the terminator without creating energy.
        let wrap=vec3<f32>(0.35,0.18,0.08);
        let key_diffuse=max((vec3<f32>(dot(normal,light))+wrap)/(vec3<f32>(1.0)+wrap),vec3<f32>(0.0))/(vec3<f32>(1.0)+wrap);
        let fill_diffuse=max((vec3<f32>(dot(normal,fill))+wrap)/(vec3<f32>(1.0)+wrap),vec3<f32>(0.0))/(vec3<f32>(1.0)+wrap);
        let diffuse=vec3<f32>(0.2)+0.6*key_diffuse+0.2*fill_diffuse;
        let specular=0.8*ggx(skin_normal,view,light,roughness)+0.25*ggx(skin_normal,view,fill,roughness);
        // Illustration of pigment heterogeneity in bind-space metres, not
        // measured melanin/perfusion. Fade unresolved scales to their mean.
        var pigment=0.0;
        var redness=0.0;
        let footprint=max(length(material_dx),length(material_dy));
        for(var octave=0;octave<4;octave=octave+1) {
            let frequency=80.0*pow(3.0,f32(octave));
            let resolved=1.0-smoothstep(0.25,0.75,footprint*frequency);
            if resolved==0.0 { continue; }
            let amplitude=0.045*pow(0.55,f32(octave))*resolved;
            pigment+=amplitude*skin_noise(in.material_point*frequency+vec3<f32>(11.3,7.1,3.8));
            redness+=amplitude*skin_noise(in.material_point*frequency+vec3<f32>(-5.7,13.9,8.2));
        }
        let pigmentation=vec3<f32>(1.0)-pigment*vec3<f32>(0.65,1.0,1.25)
            +redness*vec3<f32>(0.35,-0.45,-0.35);
        let areola_radius=select(0.0185,in.uv.x-2.0,skin_metadata);
        let pigment_strength=select(1.0,in.uv.y,skin_metadata);
        let areola_distance=length(vec2<f32>(abs(in.material_point.x)-0.08,in.material_point.y-0.36));
        let coverage=(1.0-smoothstep(areola_radius-0.0025,areola_radius+0.0025,areola_distance))
            *smoothstep(0.08,0.12,in.material_point.z)*pigment_strength;
        let areola_tint=mix(vec3<f32>(1.0),vec3<f32>(0.72,0.49,0.46),coverage);
        let color=texel.rgb*in.color.rgb*pigmentation*areola_tint*(diffuse/max(original,0.2))
            +vec3<f32>(1.0,0.96,0.92)*specular;
        return vec4<f32>(color,alpha);
    }
    return texel*in.color;
}
