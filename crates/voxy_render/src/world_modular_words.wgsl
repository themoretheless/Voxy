// Exact finite dyadic coefficient images in F3/F5. No floating arithmetic.
fn history_mod_word(raw:u32,p:u32)->u32 {
 let e=(raw>>23u)&255u;var m=raw&0x7fffffu;var power=-149;
 if e!=0u {m|=0x800000u;power=i32(e)-150;}
 let exponent=u32(power+300)%(p-1u);
 let r=((m%p)*((1u<<exponent)%p))%p;
 return select(r,(p-r)%p,(raw>>31u)!=0u);
}
fn history_mod_sub(a:vec3<u32>,b:vec3<u32>,p:u32)->vec3<u32> {return (a+vec3<u32>(p)-b)%vec3<u32>(p);}
fn history_mod_cross(a:vec3<u32>,b:vec3<u32>,p:u32)->vec3<u32> {return (a.yzx*b.zxy+vec3<u32>(p*p)-a.zxy*b.yzx)%vec3<u32>(p);}
fn history_mod_dot(a:vec3<u32>,b:vec3<u32>,p:u32)->u32 {let v=a*b;return (v.x+v.y+v.z)%p;}
