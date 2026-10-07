// Canonical f64 rest-material search metric. Physical forces/history/work stay native.
// ABI: [nodes,elements,pins(n),weights(n),direction(3n),elements(19e),offsets(n+1),references(4e)].
// Element: nodes(4), gradients(12), reference volume, shear, bulk.
// CSR references encode 4*element+corner, in canonical element/corner order.
extern "C" __global__ void tissue_search_elements(const double* data, double* force) {
    const unsigned int id = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int n = (unsigned int)data[0], count = (unsigned int)data[1];
    if (id >= count) return;
    const unsigned int base = 2u + 5u*n + 19u*id;
    double h[3][3] = {};
    for (unsigned int corner=0; corner<4; ++corner) {
        const unsigned int node = (unsigned int)data[base+corner];
        if (data[2u+node] == 0.) {
            for (unsigned int a=0;a<3;++a) for(unsigned int b=0;b<3;++b)
                h[a][b] += data[2u+2u*n+3u*node+a] * data[base+4u+3u*corner+b];
        }
    }
    const double trace = h[0][0]+h[1][1]+h[2][2];
    double stress[3][3];
    for(unsigned int a=0;a<3;++a) for(unsigned int b=0;b<3;++b) {
        const double sym = 0.5*(h[a][b]+h[b][a]);
        const double dev = a==b ? sym-trace/3. : sym;
        stress[a][b] = 2.*data[base+17u]*dev + (a==b ? data[base+18u]*trace : 0.);
    }
    for(unsigned int corner=0;corner<4;++corner) for(unsigned int a=0;a<3;++a) {
        const unsigned int node = (unsigned int)data[base+corner];
        const unsigned int g = base+4u+3u*corner;
        double product=0.;
        for(unsigned int b=0;b<3;++b) product += stress[a][b]*data[g+b];
        force[12u*id+3u*corner+a] = data[2u+node] != 0. ? 0. : data[base+16u] * product;
    }
}
extern "C" __global__ void tissue_search_nodes(const double* data, const double* force, double* output) {
    const unsigned int node = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int n = (unsigned int)data[0], e = (unsigned int)data[1];
    if(node>=n) return;
    const unsigned int offsets = 2u+5u*n+19u*e, refs=offsets+n+1u;
    for(unsigned int a=0;a<3;++a) {
        double value=0.;
        if(data[2u+node]==0.) {
            value=data[2u+n+node]*data[2u+2u*n+3u*node+a];
            for(unsigned int i=(unsigned int)data[offsets+node];i<(unsigned int)data[offsets+node+1u];++i)
                value += force[3u*(unsigned int)data[refs+i]+a];
        }
        output[3u*node+a]=value;
    }
}
