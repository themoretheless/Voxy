// Host arithmetic harness for the fixed CUDA source. Not a CUDA runtime proof.
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>
#define __device__
#define __global__
struct Index { unsigned int x; };
static Index blockIdx{}, blockDim{64}, threadIdx{};
static long long __double_as_longlong(double value) {
    long long bits; std::memcpy(&bits,&value,sizeof(bits)); return bits;
}
static unsigned long long atomicAdd(unsigned long long* address, unsigned long long value) {
    unsigned long long before; std::memcpy(&before,address,sizeof(before));
    const auto next = before+value; std::memcpy(address,&next,sizeof(next)); return before;
}
static unsigned long long atomicMax(unsigned long long* address, unsigned long long value) {
    unsigned long long before; std::memcpy(&before,address,sizeof(before));
    const auto next = before>value ? before : value; std::memcpy(address,&next,sizeof(next)); return before;
}
static unsigned int __float_as_uint(float value) {
    unsigned int bits; std::memcpy(&bits,&value,sizeof(bits)); return bits;
}
static unsigned int atomicMax(unsigned int* address, unsigned int value) {
    const auto before=*address; *address=before>value ? before : value; return before;
}
#include "../../crates/voxy_cuda/src/gravity.cu"
static bool view_self_test() {
    const unsigned int count=257;
    std::vector<double> words(8+24*count,0.0); words[3]=count;
    for (unsigned int i=0;i<count;i++) {
        for (unsigned int k=0;k<7;k++) words[8+i*8+k]=static_cast<double>(i+k)/32.0;
        words[8+i*8+3]=1.0;
    }
    words[8]=-0.0;
    std::vector<unsigned int> output(8+8*count,0xdeadbeef);
    const auto initial=output;
    auto run=[&]() {
        unsigned int status=0;
        for (unsigned int b=0;b<(count+63)/64;b++) {
            blockIdx.x=b;
            for (unsigned int t=0;t<64;t++) { threadIdx.x=t; gravity_view_validate(words.data(),&status); }
        }
        for (unsigned int b=0;b<(count+63)/64;b++) {
            blockIdx.x=b;
            for (unsigned int t=0;t<64;t++) { threadIdx.x=t; gravity_view_commit(words.data(),&status,output.data()); }
        }
        return status;
    };
    words[8+256*8]=1e300;
    if (run()!=3 || output!=initial) return false;
    words[8+256*8]=8.0;
    words[8+3]=1e-300;
    if (run()!=3 || output!=initial) return false;
    words[8+3]=1.0;
    for (unsigned long long failure : {1ULL,2ULL}) {
        std::memcpy(&words[7],&failure,sizeof(failure));
        if (run()!=failure || output!=initial) return false;
    }
    words[7]=0.0;
    if (run()!=0) return false;
    for (unsigned int k=0;k<8;k++) if (output[k]!=(k==3 ? count : 0u)) return false;
    for (unsigned int i=0;i<count;i++) {
        for (unsigned int k=0;k<7;k++)
            if (output[8+i*8+k]!=__float_as_uint(static_cast<float>(words[8+i*8+k]))) return false;
        if (output[8+i*8+7]!=0u) return false;
    }
    return true;
}
int main(int argc, char** argv) {
    if (!view_self_test()) return 5;

    if (argc != 4) return 2;
    const auto steps = std::stoul(argv[3]);
    if (steps == 0 || steps > 4096) return 2;
    std::ifstream input(argv[1],std::ios::binary|std::ios::ate);
    const auto size = input.tellg();
    if (size < 64 || size > 1024*1024 || size % 8 != 0) return 3;
    std::vector<double> words(static_cast<std::size_t>(size)/8);
    input.seekg(0); input.read(reinterpret_cast<char*>(words.data()),size);
    if (!input || !std::isfinite(words[3]) || words[3] < 1 || words[3] > 4096) return 3;
    const auto count = static_cast<unsigned int>(words[3]);
    if (words[3] != count || words.size() != 8+24*count) return 3;
    for (unsigned int step=0; step<steps; ++step) {
        for (auto kernel : {gravity_predict,gravity_correct,gravity_commit}) {
            for (unsigned int block=0; block<(count+63)/64; ++block) {
                blockIdx.x = block;
                for (unsigned int thread=0; thread<64; ++thread) { threadIdx.x=thread; kernel(words.data()); }
            }
        }
    }
    std::ofstream output(argv[2],std::ios::binary);
    output.write(reinterpret_cast<const char*>(words.data()),size);
    return output ? 0 : 4;
}
