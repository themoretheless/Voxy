// GGX single-scattering moments for the shared layered material candidate.
// Analytic null mass plus horizon-resolving deterministic slope quadrature.
// Returned channels: Schlick A, Schlick B, directly accumulated unit-energy loss.
// Rejected below-horizon reflections keep their mass; never renormalize them away.
struct IntegratedGgxMoments { value: vec3<f32>, valid: u32 }
fn ggx_lambda_direction(direction: vec3<f32>, alpha_squared: f32) -> f32 {
    // Tangential components retain small angles when z has rounded to one.
    let x=alpha_squared*dot(direction.xy,direction.xy)/(direction.z*direction.z);
    return x/(2.0*(sqrt(1.0+x)+1.0));
}
// Resolve the azimuth feature near pi/2 by sinh-warping its signed distance.
// Returned xy are cos(phi),sin(phi); z includes the Simpson measure weight.
fn ggx_azimuth_point(nv:f32,index:u32,steps:u32) -> vec3<f32> {
    let ratio=1.5707963267948966/nv;
    let upper=log(ratio+sqrt(1.0+ratio*ratio));
    let h=2.0*upper/f32(steps);
    let u=-upper+f32(index)*h;
    let positive=exp(u);let negative=exp(-u);
    let distance=nv*(positive-negative)*0.5;
    let jacobian=nv*(positive+negative)*0.5;
    var weight=select(2.0,4.0,index%2u==1u);
    if index==0u || index==steps {weight=1.0;}
    return vec3<f32>(-sin(distance),cos(distance),jacobian*h*weight/3.0);
}
// Integrate the below-horizon VNDF mass analytically in the radial slope
// coordinate, then use deterministic azimuth quadrature. Not a convergence
// certificate; independent directional reference remains mandatory.
fn ggx_null_mass(nv:f32, alpha:f32, lambda_view:f32, angles:u32) -> f32 {
    let a2=alpha*alpha;
    let sv=sqrt(max(0.0,(1.0-nv)*(1.0+nv)));
    var sum=0.0;var correction=0.0;
    for(var i=0u;i<=angles;i++) {
        let angular=ggx_azimuth_point(nv,i,angles);
        let k=sv*angular.x;
        let root=sqrt(nv*nv+k*k);
        var rc=(k+root)/nv;
        if k<0.0 {rc=nv/(root-k);}
        var first=1.0/(2.0*(a2+rc*rc));
        var second=atan(alpha/rc)/(2.0*alpha)+rc/(2.0*(a2+rc*rc));
        if k<0.0 {
            let maximum=-nv/k;
            first-=1.0/(2.0*(a2+maximum*maximum));
            second-=atan(alpha/maximum)/(2.0*alpha)+maximum/(2.0*(a2+maximum*maximum));
        }
        let term=a2*(nv*first+k*second)/(nv*(1.0+lambda_view))*angular.z;
        let increment=term-correction;let next=sum+increment;
        correction=(next-sum)-increment;sum=next;
    }
    return 0.6366197723675814*sum;
}
// Stable 1-exp(-x), including distances far below f32 epsilon.
fn ggx_one_minus_exp_negative(x:f32) -> f32 {
    if x<0.125 {
        return x*(1.0+x*(-0.5+x*(1.0/6.0+x*(-1.0/24.0+x*(1.0/120.0+x*(-1.0/720.0+x*(1.0/5040.0-x/40320.0)))))));
    }
    return 1.0-exp(-x);
}
// Deterministic Simpson quadrature in log(distance from the horizon).
// The existing native f64 reference uses adaptive radial integration instead.
fn ggx_scattering_quadrature(nv:f32,alpha:f32,lambda_view:f32,samples:u32) -> vec3<f32> {
    let a2=alpha*alpha;
    let sv=sqrt(max(0.0,(1.0-nv)*(1.0+nv)));
    let angles=select(select(32u,128u,samples>=16384u),256u,samples>=524288u);
    let steps=max(2u,2u*(samples/angles/2u));
    var total=vec3<f32>(0.0);var total_correction=vec3<f32>(0.0);
    for(var i=0u;i<=angles;i++) {
        let angular=ggx_azimuth_point(nv,i,angles);
        let cosine=angular.x;let sine=angular.y;let k=sv*cosine;
        let root=sqrt(nv*nv+k*k);
        var rc=(k+root)/nv;if k<0.0 {rc=nv/(root-k);}
        let upper=log(log(rc/alpha)+20.0);
        let h=(upper+24.0)/f32(steps);
        var sum=vec3<f32>(0.0);var correction=vec3<f32>(0.0);
        for(var j=0u;j<=steps;j++) {
            let distance=exp(-24.0+f32(j)*h);
            let r=rc*exp(-distance);
            let vh=nv+k*r;
            let horizon_distance=rc*ggx_one_minus_exp_negative(distance);
            let light_z=nv*horizon_distance*(r+1.0/rc)/(1.0+r*r);
            let tx=2.0*vh*r*cosine/(1.0+r*r)-sv;
            let ty=2.0*vh*r*sine/(1.0+r*r);
            let x=a2*(tx*tx+ty*ty)/(light_z*light_z);
            let lambda_light=x/(2.0*(sqrt(1.0+x)+1.0));
            let loss=lambda_light/(1.0+lambda_view+lambda_light);
            let denominator=a2+r*r;
            // Normalize the radial density before multiplying by tiny loss.
            // The unnormalized numerator can underflow although the integral
            // is representable (e.g. roughness .001, nv .25).
            let density=(a2/denominator)*(r*r/denominator)*(vh/nv)/(1.0+lambda_view);
            let norm=sqrt(1.0+r*r);
            let one_minus_vh=clamp(((1.0-nv)+r*r/(norm+1.0)-k*r)/norm,0.0,1.0);
            let square=one_minus_vh*one_minus_vh;
            let bias=square*square*one_minus_vh;
            let visibility_weight=(1.0+lambda_view)/(1.0+lambda_view+lambda_light);
            let normalized_vh=vh/norm;
            // Stable 1-(1-vh)^5 for tiny vh. Do not subtract B from one
            // after integration: that magnifies its error when A is small.
            let fresnel_a=normalized_vh*(5.0-normalized_vh*(10.0-normalized_vh*(10.0-normalized_vh*(5.0-normalized_vh))));
            let value=density*distance*vec3<f32>(fresnel_a*visibility_weight,bias*visibility_weight,loss);
            var weight=select(2.0,4.0,j%2u==1u);
            if j==0u || j==steps {weight=1.0;}
            let increment=value*weight-correction;let next=sum+increment;
            correction=(next-sum)-increment;sum=next;
        }
        let increment=sum*h/3.0*angular.z-total_correction;let next=total+increment;
        total_correction=(next-total)-increment;total=next;
    }
    return 0.6366197723675814*total;
}
// Exact N.V=0 limit of height-correlated Smith GGX. The azimuth
// integral of Schlick's polynomial is analytic; radial quadrature resolves
// the narrow layer in distance from the polar endpoint. Unit loss is zero.
fn ggx_grazing_moments(alpha:f32,samples:u32) -> IntegratedGgxMoments {
    if alpha==1.0 {return IntegratedGgxMoments(vec3<f32>(20.0/21.0,1.0/21.0,0.0),1u);}
    let steps=max(2u,2u*(min(samples,16384u)/2u));
    let lower=log(alpha)-20.0;let upper=log(1.5707963267948966);
    let h=(upper-lower)/f32(steps);
    var sum=vec2<f32>(0.0);var correction=vec2<f32>(0.0);
    for(var i=0u;i<=steps;i++) {
        let distance=exp(lower+f32(i)*h);
        var cosine=cos(distance);var sine=sin(distance);
        if i==steps {cosine=0.0;sine=1.0;}
        let q=alpha*cosine/sqrt(sine*sine+alpha*alpha*cosine*cosine);
        let a=q*(3.9269908169872414-q*(6.666666666666667-q*(5.890486225480862-q*(2.6666666666666665-q*0.4908738521234052))));
        let b=1.0-a;
        var weight=select(2.0,4.0,i%2u==1u);
        if i==0u || i==steps {weight=1.0;}
        let term=vec2<f32>(a,b)*(1.2732395447351628*cosine*cosine*distance*weight);
        let increment=term-correction;let next=sum+increment;
        correction=(next-sum)-increment;sum=next;
    }
    let integrated=sum*h/3.0;
    var value=vec3<f32>(integrated,0.0);
    // Preserve the smaller channel directly; the known total is exactly one.
    if integrated.x<=integrated.y {value.y=1.0-value.x;}else{value.x=1.0-value.y;}
    let valid=all(value>=vec3<f32>(0.0)) && all(value<=vec3<f32>(1.0));
    return IntegratedGgxMoments(value,select(0u,1u,valid));
}
fn integrate_ggx_energy(nv: f32, roughness: f32, samples: u32) -> IntegratedGgxMoments {
    if !(nv>=0.0 && nv<=1.0 && roughness>=0.0 && roughness<=1.0)
        || samples==0u || samples>1048576u {
        return IntegratedGgxMoments(vec3<f32>(0.0),0u);
    }
    if roughness==0.0 {
        let c=1.0-nv;
        let c2=c*c;
        let bias=c2*c2*c;
        // 1-c^5 = nv*(1+c+c^2+c^3+c^4). Evaluate the smaller
        // channel directly, then use the exact unit total for its complement.
        // This avoids both grazing cancellation and near-normal overshoot.
        let a=nv*(1.0+c+c2+c2*c+c2*c2);
        var value=vec3<f32>(a,bias,0.0);
        if a<=bias {value.y=1.0-a;}else{value.x=1.0-bias;}
        return IntegratedGgxMoments(value,1u);
    }
    let view=vec3<f32>(sqrt(max(0.0,fma(-nv,nv,1.0))),0.0,nv);
    let alpha=roughness*roughness;
    let alpha_squared=alpha*alpha;
    if !(alpha_squared>0.0) {
        return IntegratedGgxMoments(vec3<f32>(0.0),0u);
    }
    if nv==0.0 {return ggx_grazing_moments(alpha,samples);}
    let lambda_view=ggx_lambda_direction(view,alpha_squared);
    let integrated=ggx_scattering_quadrature(nv,alpha,lambda_view,samples);
    var unit_loss=0.0;
    if roughness==1.0 {
        // Exact boundary of height-correlated Smith GGX.
        unit_loss=nv*log(1.0+1.0/nv);
    } else {
        unit_loss=ggx_null_mass(nv,alpha,lambda_view,512u)+integrated.z;
    }
    // Two algebraically identical evaluations of A. Use the total identity
    // only when subtraction is well conditioned; retain direct quadrature
    // when A is small. This changes neither the BRDF nor its probability mass.
    var value=vec3<f32>(integrated.xy,unit_loss);
    let remaining=(1.0-unit_loss)-integrated.y;
    if remaining>0.0 {
        let condition=(1.0+unit_loss+integrated.y)/remaining;
        if condition<=8.0 {value.x=remaining;}
    }
    let valid=all(value>=vec3<f32>(0.0)) && all(value<=vec3<f32>(1.0))
        && abs(value.x+value.y+value.z-1.0)<=0.000005;
    return IntegratedGgxMoments(value,select(0u,1u,valid));
}
