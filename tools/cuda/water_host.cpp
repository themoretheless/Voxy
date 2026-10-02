// CUDA source arithmetic fixture only; never NVIDIA execution.
#include <algorithm>
#include <array>
#include <cassert>
#include <iostream>
#include <vector>
#define __device__
#define __global__
struct Index { unsigned int x=0, y=0, z=0; } blockIdx, threadIdx;
#include "../../crates/voxy_cuda/src/water.cu"
std::vector<unsigned int> fixture() {
    std::vector<unsigned int> words={3,1,8,4,8,0,0,3};
    const std::array<std::array<unsigned int,8>,3> nodes={{{8,8,0,1,2,2,2,2},{9,9,0,1,1,1,1,1},{0,0,0,1,1,1,1,1}}};
    for(const auto& node:nodes) words.insert(words.end(),node.begin(),node.end());
    words.push_back(0);return words;
}
void verify_downward_capacity() {
    unsigned int cases = 0;
    for (unsigned int source = 0; source <= 8; ++source) {
        for (unsigned int destination = 0; destination <= 8; ++destination) {
            for (unsigned int limit = 1; limit <= 8; ++limit) {
                std::vector<unsigned int> words = {3,1,limit,4,3,0,0,2};
                const std::array<std::array<unsigned int,8>,3> nodes = {{
                    {source,source,0,1,2,2,2,2},
                    {destination,destination,0,2,2,2,2,2},
                    {9,9,0,2,2,2,2,2}
                }};
                for (const auto& node : nodes) words.insert(words.end(), node.begin(), node.end());
                words.push_back(0);
                water_transfer(words.data());
                const unsigned int expected = std::min(source, std::min(8u-destination, limit));
                assert(words[5] == 0);
                assert(words[8] == source-expected && words[16] == destination+expected);
                assert(words[8]+words[16] == source+destination && words[24] == 9);
                assert(words[10] == 1 && words[18] == (source != 0));
                assert(words[26] == (source > expected));
                ++cases;
            }
        }
    }
    assert(cases == 648);
    std::cout << "PASS: CUDA water 648 exhaustive downward capacity/limit/volume cases; host arithmetic only\n";
}
void verify_ordered_cascade() {
    for (unsigned int mode = 0; mode < 3; ++mode) {
        const unsigned int active = mode == 0 ? 1 : 2;
        std::vector<unsigned int> words = {4,active,mode == 2 ? 4u : 8u,4,4,0,0,2};
        const std::array<std::array<unsigned int,8>,4> nodes = {{
            {8,8,0,1,3,3,3,3}, {0,0,0,2,3,3,3,3},
            {0,0,0,3,3,3,3,3}, {9,9,0,3,3,3,3,3}
        }};
        for (const auto& node : nodes) words.insert(words.end(), node.begin(), node.end());
        words.push_back(0);
        if (active == 2) words.push_back(1);
        water_transfer(words.data());
        assert(words[5] == 0);
        assert(words[8] == (mode == 2 ? 4u : 0u));
        assert(words[16] == (mode == 0 ? 8u : 0u));
        assert(words[24] == (mode == 0 ? 0u : mode == 1 ? 8u : 4u));
        assert(words[26] == (mode != 0) && words[34] == (mode == 2));
    }
    std::cout << "PASS: CUDA water ordered cascade and final write accounting; host arithmetic only\n";
}
int main() {
    verify_ordered_cascade();
    verify_downward_capacity();
    auto words=fixture();water_transfer(words.data());
    assert(words[5]==0 && words[6]==3);
    assert(words[8]==4 && words[24]==4 && words[16]==9);
    assert(words[10]==1 && words[18]==1 && words[26]==1);
    words=fixture();words[7]=1;water_transfer(words.data());assert(words[5]==3);
    words=fixture();words[4]=1;water_transfer(words.data());assert(words[5]==2);
    words=fixture();words[11]=4294967294u;water_transfer(words.data());assert(words[5]==4);
    words=fixture();words[12]=4294967295u;water_transfer(words.data());assert(words[5]==1);
    for(unsigned int status:{10u,11u}) {
        words=fixture();words[24]=status;words[25]=status;
        water_transfer(words.data());assert(words[5]==(status==10?6u:5u) && words[6]==2);
        assert(words[8]==8); // Failed lazy read precedes any horizontal transfer.
        words=fixture();words[8]=0;words[9]=0;words[24]=status;words[25]=status;
        water_transfer(words.data());assert(words[5]==0 && words[26]==0);
    }
    words=fixture();const auto original=words;threadIdx.x=1;water_transfer(words.data());assert(words==original);
    std::cout<<"PASS: CUDA water host arithmetic, ordered transfers, lazy provenance, budgets, faults and single-thread guard; no NVIDIA execution\n";
}
