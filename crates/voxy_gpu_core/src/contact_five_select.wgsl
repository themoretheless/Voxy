@group(0) @binding(0) var<storage,read_write> selection_words:array<u32>;
fn selected_gap(row:u32)->FiveRounded{
 let at=8u+10u*row;var result:FiveRounded;
 for(var i=0u;i<5u;i++){result.terms[i]=bitcast<f32>(selection_words[at+i]);}
 result.exponent=bitcast<i32>(selection_words[at+5u]);result.error_bound=bitcast<f32>(selection_words[at+6u]);result.valid=selection_words[at+7u];return result;
}
@compute @workgroup_size(1)
fn cs_main(){
 if selection_words[0]!=0x31533556u || selection_words[1]!=1u || selection_words[3]!=10u || selection_words[6]!=arrayLength(&selection_words){return;}
 let count=selection_words[2];let output=selection_words[5];var best=0xffffffffu;var uncertain=false;
 for(var row=0u;row<count;row++){
  if selection_words[8u+10u*row+9u]!=0u {continue;}
  let decision=selection_words[8u+10u*row+8u];
  if decision==4u {uncertain=true;}
  if decision!=1u {continue;}
  if best==0xffffffffu {best=row;continue;}
  let difference=five_subtract(selected_gap(row),selected_gap(best));
  if difference.valid==1u && difference.terms[0]<0.0 && five_lower_magnitude(difference)>0.0 {best=row;}
 }
 var status=2u;
 if best!=0xffffffffu {
  status=1u;
  // Certify the winner against EVERY inactive row, including intervals whose
  // tolerance classification was uncertain. Overlap cannot silently pick a row.
  for(var row=0u;row<count;row++){
   if row==best || selection_words[8u+10u*row+9u]!=0u {continue;}
   let difference=five_subtract(selected_gap(row),selected_gap(best));
   if difference.valid!=1u || !(five_exact_zero(difference) || (difference.terms[0]>0.0 && five_lower_magnitude(difference)>0.0)) {status=3u;}
  }
 } else if uncertain {status=3u;}
 selection_words[output]=status;selection_words[output+1u]=select(0xffffffffu,best,status==1u);
}
