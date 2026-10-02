/* Compile real CUDA source to PTX without a GPU. Driver execution is separate. */
#include <nvrtc.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static void check(nvrtcResult result) {
    if (result != NVRTC_SUCCESS) { fprintf(stderr,"NVRTC: %s\n",nvrtcGetErrorString(result)); exit(1); }
}
static char* source(const char* path) {
    FILE* input = fopen(path,"rb");
    if (!input || fseek(input,0,SEEK_END)) exit(2);
    long size=ftell(input); if (size<0 || fseek(input,0,SEEK_SET)) exit(2);
    char* text=calloc((size_t)size+1,1); if (!text) exit(2);
    if (fread(text,1,(size_t)size,input)!=(size_t)size) exit(2);
    fclose(input); return text;
}
int main(int argc,char** argv) {
    if (argc == 2 && strcmp(argv[1], "--recovery-check") == 0) {
        nvrtcProgram program;
        const char* kernel = "#ifdef VOXY_FAIL\n#error voxy intentional compilation error\n#endif\nextern \"C\" __global__ void recovery(unsigned* out) { out[0] = 42u; }";
        check(nvrtcCreateProgram(&program, kernel, "recovery.cu", 0, NULL, NULL));
        const char* invalid[] = {"--voxy-intentionally-invalid-option"};
        nvrtcResult rejected = nvrtcCompileProgram(program, 1, invalid);
        if (rejected != NVRTC_ERROR_INVALID_OPTION) {
            fprintf(stderr, "Expected invalid compiler option, received %s\n", nvrtcGetErrorString(rejected));
            check(nvrtcDestroyProgram(&program));
            return 1;
        }
        size_t log_size = 0;
        check(nvrtcGetProgramLogSize(program, &log_size));
        if (log_size <= 1) { check(nvrtcDestroyProgram(&program)); return 1; }
        char* log = calloc(log_size, 1);
        if (!log) return 2;
        check(nvrtcGetProgramLog(program, log));
        if (!strstr(log, "voxy-intentionally-invalid-option")) {
            fprintf(stderr, "Missing compiler option diagnostic: %s\n", log);
            free(log); check(nvrtcDestroyProgram(&program)); return 1;
        }
        free(log);
        const char* fail_source[] = {"--define-macro=VOXY_FAIL"};
        rejected = nvrtcCompileProgram(program, 1, fail_source);
        if (rejected != NVRTC_ERROR_COMPILATION) {
            fprintf(stderr, "Expected shader compilation failure, received %s\n", nvrtcGetErrorString(rejected));
            check(nvrtcDestroyProgram(&program)); return 1;
        }
        check(nvrtcGetProgramLogSize(program, &log_size));
        log = calloc(log_size, 1);
        if (!log) return 2;
        check(nvrtcGetProgramLog(program, log));
        if (!strstr(log, "voxy intentional compilation error")) {
            fprintf(stderr, "Missing source diagnostic: %s\n", log);
            free(log); check(nvrtcDestroyProgram(&program)); return 1;
        }
        free(log);
        check(nvrtcCompileProgram(program, 0, NULL));
        size_t size = 0;
        check(nvrtcGetPTXSize(program, &size));
        char* ptx = calloc(size, 1);
        if (!ptx) return 2;
        check(nvrtcGetPTX(program, ptx));
        int restored = strstr(ptx, ".entry recovery(") != NULL;
        free(ptx); check(nvrtcDestroyProgram(&program));
        if (!restored) return 1;
        puts("PASS: NVRTC invalid-option/source diagnostics and same-program recompilation recovery; no GPU execution");
        return 0;
    }
    if (argc == 2 && strcmp(argv[1], "--list-architectures") == 0) {
        int count = 0;
        check(nvrtcGetNumSupportedArchs(&count));
        if (count < 1 || count > 256) return 2;
        int supported[256];
        check(nvrtcGetSupportedArchs(supported));
        for (int i = 0; i < count; i++) {
            if (supported[i] <= 0) return 2;
            printf("%d%s", supported[i], i + 1 == count ? "\n" : " ");
        }
        return 0;
    }
    if (argc<4) { fprintf(stderr,"nvrtc-probe input.cu output.ptx entry [entry...]\n"); return 2; }
    int major,minor; check(nvrtcVersion(&major,&minor));
    char* text=source(argv[1]); nvrtcProgram program;
    check(nvrtcCreateProgram(&program,text,argv[1],0,NULL,NULL)); free(text);
    int count; check(nvrtcGetNumSupportedArchs(&count));
    if (count<1 || count>256) return 2;
    int supported[256]; check(nvrtcGetSupportedArchs(supported));
    const char* requested=getenv("VOXY_NVRTC_ARCH");
    char* end; long architecture=requested ? strtol(requested,&end,10) : 52;
    if (requested && (!*requested || *end)) return 2;
    int found=0;
    for (int i=0;i<count;i++) if (supported[i]==architecture) found=1;
    if (!found) { fprintf(stderr,"Unsupported compiler architecture: %ld\n",architecture); return 2; }
    char target[64]; snprintf(target,sizeof(target),"--gpu-architecture=compute_%ld",architecture);
    const char* options[]={target,"--fmad=false","--ftz=false","--prec-div=true","--prec-sqrt=true"};
    nvrtcResult result=nvrtcCompileProgram(program,5,options);
    size_t log_size; check(nvrtcGetProgramLogSize(program,&log_size));
    char* log=calloc(log_size,1); if (!log) return 2;
    check(nvrtcGetProgramLog(program,log)); if (log_size>1) fprintf(stderr,"%s",log); free(log);
    check(result);
    size_t size; check(nvrtcGetPTXSize(program,&size));
    char* ptx=calloc(size,1); if (!ptx) return 2;
    check(nvrtcGetPTX(program,ptx));
    for (int i=3;i<argc;i++) { char symbol[256]; snprintf(symbol,sizeof(symbol),".entry %s(",argv[i]);
        if (!strstr(ptx,symbol)) { fprintf(stderr,"Missing PTX entry: %s\n",argv[i]); return 1; }
    }
    FILE* output=fopen(argv[2],"wb"); if (!output) return 2;
    if (fwrite(ptx,1,size-1,output)!=size-1) return 2;
    fclose(output); free(ptx); check(nvrtcDestroyProgram(&program));
    printf("NVRTC %d.%d compute_%ld PASS: %s, %zu PTX bytes, %d entrypoints; no GPU execution\n",major,minor,architecture,argv[1],size-1,argc-3);
    return 0;
}
