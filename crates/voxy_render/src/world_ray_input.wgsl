// Read-only interpretation of the original 80-byte source ray record.
@group(0) @binding(0) var<storage,read> exact_rays:array<u32>;
fn exact_ray_word(index:u32)->u32 {return exact_rays[index];}
fn exact_ray_word_count()->u32 {return arrayLength(&exact_rays);}
