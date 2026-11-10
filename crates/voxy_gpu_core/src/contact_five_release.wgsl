@group(0) @binding(0) var<storage,read_write> release_words:array<u32>;
fn release_zero()->FiveRounded{var value:FiveRounded;value.valid=1u;return value;}
fn release_read(at:u32)->FiveRounded{
 var value:FiveRounded;for(var i=0u;i<5u;i++){value.terms[i]=bitcast<f32>(release_words[at+i]);}
 value.exponent=bitcast<i32>(release_words[at+5u]);value.error_bound=bitcast<f32>(release_words[at+6u]);value.valid=release_words[at+7u];return value;
}
fn release_write(at:u32,value:FiveRounded){
 for(var i=0u;i<5u;i++){release_words[at+i]=bitcast<u32>(value.terms[i]);}
 release_words[at+5u]=bitcast<u32>(value.exponent);release_words[at+6u]=bitcast<u32>(value.error_bound);release_words[at+7u]=value.valid;
}
fn release_nonnegative(value:FiveRounded)->bool{return value.valid==1u && (five_exact_zero(value)||(value.terms[0]>0.0 && five_lower_magnitude(value)>0.0));}
fn release_negative(value:FiveRounded)->bool{return value.valid==1u && value.terms[0]<0.0 && five_lower_magnitude(value)>0.0;}
fn release_same_exact(a:FiveRounded,b:FiveRounded)->bool{
 if a.valid!=1u || b.valid!=1u || a.error_bound!=0.0 || b.error_bound!=0.0 || a.exponent!=b.exponent{return false;}
 for(var i=0u;i<5u;i++){if a.terms[i]!=b.terms[i]{return false;}}
 return true;
}
// Cross multiplication compares fractions without dividing nearly equal ratios.
fn release_compare(a:u32,b:u32,count:u32)->FiveRounded{
 let old_a=release_read(8u+8u*a);let old_b=release_read(8u+8u*b);
 let next_a=release_read(8u+8u*(count+a));let next_b=release_read(8u+8u*(count+b));
 // Identical exact sources have identical ratios, without independent product
 // uncertainty. Never apply this identity to nonzero input error intervals.
 if release_same_exact(old_a,old_b)&&release_same_exact(next_a,next_b){return release_zero();}
 let den_a=five_subtract(old_a,next_a);
 let den_b=five_subtract(old_b,next_b);
 return five_subtract(five_multiply(old_a,den_b),five_multiply(old_b,den_a));
}
@compute @workgroup_size(1)
fn cs_main(){
 if release_words[0]!=0x31523556u || release_words[1]!=2u || release_words[3]!=8u || release_words[6]!=arrayLength(&release_words){return;}
 let count=release_words[2];let output=release_words[5];release_words[output]=3u;release_words[output+1u]=0xffffffffu;
 var best=0xffffffffu;
 for(var row=0u;row<count;row++){
  let old=release_read(8u+8u*row);let next=release_read(8u+8u*(count+row));
  if !release_nonnegative(old){return;}
  if release_negative(next){
   if best==0xffffffffu {best=row;}else{if release_negative(release_compare(row,best,count)){best=row;}}
  }else if !release_nonnegative(next){return;}
 }
 if best==0xffffffffu {release_words[output]=2u;return;}
 for(var row=0u;row<count;row++){
  if row==best || !release_negative(release_read(8u+8u*(count+row))){continue;}
  if !release_nonnegative(release_compare(row,best,count)){return;}
 }
 let old=release_read(8u+8u*best);let next=release_read(8u+8u*(count+best));
 let fraction=five_divide(old,five_subtract(old,next));
 var one=release_zero();one.terms[0]=1.0;
 if !release_nonnegative(fraction)||!release_nonnegative(five_subtract(one,fraction)){return;}
 release_write(output+2u,fraction);
 for(var row=0u;row<count;row++){
  let current=release_read(8u+8u*row);let candidate=release_read(8u+8u*(count+row));
  var updated=five_add(current,five_multiply(fraction,five_subtract(candidate,current)));
  // At the exact selected ratio this reaction is identically zero. Equal ratios
  // are also exact zeros only when the cross-product certificate is exact zero.
  if row==best || (release_negative(candidate)&&five_exact_zero(release_compare(row,best,count))){updated=release_zero();}
  if !release_nonnegative(updated){return;}
  release_write(output+10u+8u*row,updated);
 }
 let width=release_words[7];let state=8u+16u*count;
 for(var coordinate=0u;coordinate<width;coordinate++){
  let current=release_read(state+8u*coordinate);let candidate=release_read(state+8u*(width+coordinate));
  let updated=five_add(current,five_multiply(fraction,five_subtract(candidate,current)));
  if updated.valid!=1u{return;}
  release_write(output+10u+8u*(count+coordinate),updated);
 }
 release_words[output]=1u;release_words[output+1u]=best;
}
