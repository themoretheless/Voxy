// Executes the actual CUDA source as host arithmetic; no NVIDIA runtime claim.
#include <cmath>
#include <cstdint>
#include <fstream>
#include <vector>
#define __global__
struct Index { unsigned int x; };
static Index blockIdx{},blockDim{64},threadIdx{};
#include "../../crates/voxy_cuda/src/tissue_search.cu"
int main(int argc,char** argv) {
    if(argc!=3) return 2;
    std::ifstream input(argv[1],std::ios::binary|std::ios::ate);
    const auto size=input.tellg();
    if(size<24 || size>16*1024*1024 || size%8!=0) return 3;
    std::vector<double> data(static_cast<std::size_t>(size)/8);
    input.seekg(0); input.read(reinterpret_cast<char*>(data.data()),size);
    if(!input) return 3;
    for(double x:data) if(!std::isfinite(x)) return 3;
    auto integer=[](double x,unsigned int limit) {return x>=0 && x<=limit && x==std::floor(x);};
    if(!integer(data[0],65536) || !integer(data[1],65536) || data[0]==0 || data[1]==0) return 3;
    const auto n=(unsigned int)data[0],e=(unsigned int)data[1];
    if(data.size()!=3u+6u*n+23u*e) return 3;
    const auto offsets=2u+5u*n+19u*e, refs=offsets+n+1u;
    for(unsigned int i=0;i<n;++i) if(data[2u+i]!=0 && data[2u+i]!=1) return 3;
    for(unsigned int i=0;i<e;++i) for(unsigned int c=0;c<4;++c)
        if(!integer(data[2u+5u*n+19u*i+c],n-1u)) return 3;
    if(data[offsets]!=0 || data[offsets+n]!=4u*e) return 3;
    for(unsigned int i=0;i<n;++i)
        if(!integer(data[offsets+i],4u*e) || data[offsets+i]>data[offsets+i+1u]) return 3;
    for(unsigned int i=0;i<4u*e;++i) if(!integer(data[refs+i],4u*e-1u)) return 3;
    std::vector<double> force(12u*e),result(3u*n);
    for(unsigned int b=0;b<(e+63u)/64u;++b) {
        blockIdx.x=b;
        for(unsigned int t=0;t<64;++t) {threadIdx.x=t;tissue_search_elements(data.data(),force.data());}
    }
    for(unsigned int b=0;b<(n+63u)/64u;++b) {
        blockIdx.x=b;
        for(unsigned int t=0;t<64;++t) {threadIdx.x=t;tissue_search_nodes(data.data(),force.data(),result.data());}
    }
    std::ofstream output(argv[2],std::ios::binary);
    output.write(reinterpret_cast<const char*>(result.data()),result.size()*sizeof(double));
    return output ? 0 : 4;
}
