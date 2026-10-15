# Facial rendering source scan: jpl917/BlendshapeGeneration

Repository: https://github.com/jpl917/BlendshapeGeneration

Pinned source revision: 7b38520c3b98fe6e980d5c6b2a49735a2ad5a117

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["third_party/SuiteSparse/AMD/Doc/License.txt", "third_party/SuiteSparse/BTF/Doc/License.txt", "third_party/SuiteSparse/CAMD/Doc/License.txt", "third_party/SuiteSparse/CCOLAMD/Doc/License.txt", "third_party/SuiteSparse/CHOLMOD/Doc/License.txt", "third_party/SuiteSparse/COLAMD/Doc/License.txt", "third_party/SuiteSparse/CSparse/Doc/License.txt", "third_party/SuiteSparse/CSparse/MATLAB/UFget/Doc/License.txt", "third_party/SuiteSparse/CXSparse/Doc/License.txt", "third_party/SuiteSparse/CXSparse/MATLAB/UFget/Doc/License.txt", "third_party/SuiteSparse/CXSparse_newfiles/Doc/License.txt", "third_party/SuiteSparse/GPUQREngine/Doc/License.txt", "third_party/SuiteSparse/KLU/Doc/License.txt", "third_party/SuiteSparse/LDL/Doc/License.txt", "third_party/SuiteSparse/LICENSE.txt", "third_party/SuiteSparse/MATLAB_Tools/Doc/License.txt", "third_party/SuiteSparse/RBio/Doc/License.txt", "third_party/SuiteSparse/SPQR/Doc/License.txt", "third_party/SuiteSparse/SuiteSparse_GPURuntime/Doc/License.txt", "third_party/SuiteSparse/UFget/Doc/License.txt"]

Topic paths: {"blendshape": ["Semantically-aware Blendshape Rigs from Facial Performance Measurements.pdf", "data/Tester_1/Blendshape/shape.bs", "data/Tester_1/Blendshape/shape_0.obj", "data/Tester_1/Blendshape/shape_1.obj", "data/Tester_1/Blendshape/shape_10.obj", "data/Tester_1/Blendshape/shape_11.obj", "data/Tester_1/Blendshape/shape_12.obj", "data/Tester_1/Blendshape/shape_13.obj", "data/Tester_1/Blendshape/shape_14.obj", "data/Tester_1/Blendshape/shape_15.obj", "data/Tester_1/Blendshape/shape_16.obj", "data/Tester_1/Blendshape/shape_17.obj", "data/Tester_1/Blendshape/shape_18.obj", "data/Tester_1/Blendshape/shape_19.obj", "data/Tester_1/Blendshape/shape_2.obj", "data/Tester_1/Blendshape/shape_20.obj", "data/Tester_1/Blendshape/shape_21.obj", "data/Tester_1/Blendshape/shape_22.obj", "data/Tester_1/Blendshape/shape_23.obj", "data/Tester_1/Blendshape/shape_24.obj", "data/Tester_1/Blendshape/shape_25.obj", "data/Tester_1/Blendshape/shape_26.obj", "data/Tester_1/Blendshape/shape_27.obj", "data/Tester_1/Blendshape/shape_28.obj", "data/Tester_1/Blendshape/shape_29.obj", "data/Tester_1/Blendshape/shape_3.obj", "data/Tester_1/Blendshape/shape_30.obj", "data/Tester_1/Blendshape/shape_31.obj", "data/Tester_1/Blendshape/shape_32.obj", "data/Tester_1/Blendshape/shape_33.obj"], "deform": ["DeformationTransfer04.pdf", "include/DeformationTransfer.h", "src/DeformationTransfer.cpp"], "shader": ["third_party/trimesh2/utilsrc/shaders.inc"], "rig": ["Example-Based Facial Rigging.pdf", "Semantically-aware Blendshape Rigs from Facial Performance Measurements.pdf", "third_party/SuiteSparse/CSparse/MATLAB/CSparse/cs_qright.m", "third_party/SuiteSparse/CSparse/MATLAB/Test/chol_right.m", "third_party/SuiteSparse/CSparse/MATLAB/Test/lu_right.m", "third_party/SuiteSparse/CSparse/MATLAB/Test/lu_rightp.m", "third_party/SuiteSparse/CSparse/MATLAB/Test/lu_rightpr.m", "third_party/SuiteSparse/CSparse/MATLAB/Test/lu_rightr.m", "third_party/SuiteSparse/CSparse/MATLAB/Test/qr_right.m", "third_party/SuiteSparse/CXSparse/MATLAB/CSparse/cs_qright.m", "third_party/SuiteSparse/CXSparse/MATLAB/Test/chol_right.m", "third_party/SuiteSparse/CXSparse/MATLAB/Test/lu_right.m", "third_party/SuiteSparse/CXSparse/MATLAB/Test/lu_rightp.m", "third_party/SuiteSparse/CXSparse/MATLAB/Test/lu_rightpr.m", "third_party/SuiteSparse/CXSparse/MATLAB/Test/lu_rightr.m", "third_party/SuiteSparse/CXSparse/MATLAB/Test/qr_right.m", "third_party/trimesh2/gluit/glui_img_rightarrow.c"], "face": ["third_party/trimesh2/libsrc/OBJ.Linux64/faceflip.d", "third_party/trimesh2/libsrc/OBJ.Linux64/faceflip.o", "third_party/trimesh2/libsrc/faceflip.cc"], "lip": ["third_party/ceres-solver-1.12.0/examples/ellipse_approximation.cc", "third_party/trimesh2/libsrc/OBJ.Linux64/edgeflip.d", "third_party/trimesh2/libsrc/OBJ.Linux64/edgeflip.o", "third_party/trimesh2/libsrc/OBJ.Linux64/faceflip.d", "third_party/trimesh2/libsrc/OBJ.Linux64/faceflip.o", "third_party/trimesh2/libsrc/edgeflip.cc", "third_party/trimesh2/libsrc/faceflip.cc"]}

## Source file: src/BlendshapeGenerator.cpp

{
  "path": "src/BlendshapeGenerator.cpp",
  "blob_sha": "dbda8ace6081327a69a371bc7cab35d6bd168ee2",
  "sha256": "9dfc9f70824c6b4eb3ee8bdc0613b17b5de799f97542c8651c6100299a7163f4",
  "lines": 469,
  "bytes": 16284,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "blendshape": 18,
    "deform": 1,
    "face": 18
  },
  "url": "https://github.com/jpl917/BlendshapeGeneration/blob/7b38520c3b98fe6e980d5c6b2a49735a2ad5a117/src/BlendshapeGenerator.cpp",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "bool operator()(const T* const alpha, T* residual) const{"
    },
    {
      "line": 40,
      "text": "bool operator()(const T* const alpha, T* residual) const{"
    }
  ]
}

```text

#include "BlendshapeGenerator.h"

const int NUM_POSES  = 20;   // number of poses
const int NUM_SHAPES = 47;   // number of blendshapes


struct PointResidual{
    PointResidual(double x, double y, double z, int idx, const std::vector<trimesh::TriMesh> &dB)
        : m_x(x), m_y(y), m_z(z), m_idx(idx), m_dB(dB){}
    
    template <typename T>
    bool operator()(const T* const alpha, T* residual) const{
        T p[3];  
        p[0] = T(0); p[1] = T(0); p[2] = T(0);
        
        for(int i=0; i<NUM_SHAPES-1; i++){
            trimesh::vec3 pt = m_dB[i].vertices[m_idx];
            p[0] += T(pt[0]) * alpha[i];
            p[1] += T(pt[1]) * alpha[i];
            p[2] += T(pt[2]) * alpha[i];
        }
        
        residual[0] = T(m_x) - p[0];
        residual[1] = T(m_y) - p[1];
        residual[2] = T(m_z) - p[2];
        return true;
    }
    
private:
    const double m_x, m_y, m_z;   // S-B0
    const int    m_idx;           // vertex index
    const std::vector<trimesh::TriMesh> &m_dB;
};


struct PriorResidual{
    PriorResidual(double* prior):m_prior(prior){}
    
    template <typename T>
    bool operator()(const T* const alpha, T* residual) const{
        for(int i=0; i<NUM_SHAPES-1; i++){
            residual[i] = T(m_prior[i]) - alpha[i];
        }
        return true;
    }
private:
    const double* m_prior;
};



std::vector<double> BlendshapeGenerator::estimateWeights(const trimesh::TriMesh& S, 
                                                         const trimesh::TriMesh& B0,
                                                         const std::vector<trimesh::TriMesh>& dB,
                                                         const std::vector<double>& w0_vec,   //46 init weight
                                                         const std::vector<double>& wp_vec,   //46 prior weight
                                                         bool  prior_weight_flag)
{    
    
    ColorStream(ColorOutput::Blue)<<"estimate weight ...";
    
    double w[NUM_SHAPES-1];
    for(int i=0; i<NUM_SHAPES-1; i++) w[i] = w0_vec[i];   
    
    double wp[NUM_SHAPES-1];
    for(int i=0; i<NUM_SHAPES-1; i++) wp[i] = wp_vec[i];
    
    ceres::Problem problem;
    
    int numVerts = S.vertices.size();
    for(int i=0; i<numVerts; i++){
        trimesh::vec3 vS  = S.vertices[i];    // S
        trimesh::vec3 vB0 = B0.vertices[i];   // B0
        double dx = vS[0] - vB0[0];           // S- B0
        double dy = vS[1] - vB0[1];
        double dz = vS[2] - vB0[2];
        
        ceres::CostFunction *costFunc = new AutoDiffCostFunction<PointResidual, 3, 46>(new PointResidual(dx, dy, dz, i, dB));
        problem.AddResidualBlock(costFunc, NULL, w);
    }
    
    if(prior_weight_flag){
        ceres::CostFunction *costFunc = new AutoDiffCostFunction<PriorResidual, 46, 46>(new PriorResidual(wp));
        problem.AddResidualBlock(costFunc, NULL, w);
    }
    for(int i=0; i<NUM_SHAPES-1; i++){
        problem.SetParameterLowerBound(w, i, 0.0);
        problem.SetParameterUpperBound(w, i, 1.0);
    }
    
    // solve 
    ceres::Solver::Options options;
    options.max_num_iterations  = 10;
	options.linear_solver_type = ceres::DENSE_QR;
	options.minimizer_progress_to_stdout = false;
    options.num_threads = 8;
    options.num_linear_solver_threads = 8;
    options.initial_trust_region_radius = 1.0;
    options.min_lm_diagonal = 1.0;
    options.max_lm_diagonal = 1.0;
	ceres::Solver::Summary summary;
	ceres::Solve(options, &problem, &summary);
    //std::cout<<summary.BriefReport()<<std::endl;
    
    std::vector<double> ret;
    std::cout<<"    done";
    for(int i=0; i<NUM_SHAPES-1; i++){
        //std::cout<<w[i]<<" ";
        ret.push_back(w[i]);
    }
    //std::cout<<std::endl;
    return ret;
}



trimesh::TriMesh BlendshapeGenerator::reconstructMesh(const std::vector<double>& weight, 
                                                      const trimesh::TriMesh& B0,
                                                      const std::vector<trimesh::TriMesh>& dB)
{
    trimesh::TriMesh ret = B0;
    for(int i=0; i<ret.vertices.size(); i++)
    {
        for(int j=0; j<NUM_SHAPES-1; j++){
            ret.vertices[i].x += weight[j] * dB[j].vertices[i].x; 
            ret.vertices[i].y += weight[j] * dB[j].vertices[i].y;
            ret.vertices[i].z += weight[j] * dB[j].vertices[i].z;
        }
    }
    return ret;
}


/*
 * @input:
 * 
 */
std::vector<trimesh::TriMesh> BlendshapeGenerator::optimizeBlendshapes(
        const std::vector<trimesh::TriMesh>& S,                   // 20
        const std::vector<std::vector<Eigen::Matrix3d> >& Sgrad,  // 20 * 22800 * mat33
        const trimesh::TriMesh& B0,                               // B0
        const std::vector<std::vector<double> >& alpha,           // 20 * 46
        double beta, double gamma,
        const std::vector<std::vector<Eigen::Matrix3d> >& prior,  // 46 * 22800 * mat33
        const std::vector<std::vector<double> >& w_prior)         // 46 * 22800
{

    cout<<"Optimize blendshapes ..."<<endl;
    
    int nfaces = B0.faces.size();
    int nverts = B0.vertices.size();
    
    cout<<"nfaces: "<<nfaces<<"     nverts: "<<nverts<<endl;
    
    int nposes  = S.size();     // 20
    int nshapes = prior.size(); // 46
    
    
    // the triangle gradient of B0 
    std::vector<Eigen::Matrix3d> B0grad(nfaces);
    //std::vector<double> B0d(nfaces);
    for(int i=0; i<nfaces; i++){
//         std::pair<Eigen::Matrix3d, double> GD = triangleGradient2(B0, i);
//         B0grad[i] = GD.first;
//         B0d[i] = GD.second;
        B0grad[i] = triangleGradient(B0, i);
    }
    
    
    using Tripletd = Eigen::Triplet<double>;
    using SparseMatrixd = Eigen::SparseMatrix<double, Eigen::RowMajor>;
    std::vector<Tripletd> Adata_coeffs;

    // upper part of A
    // sigma (alpha_ij * M^B_i) = M^S_j - M^B_0 
    // [npose * 9, nshapes * 9]
    for(int i=0; i<nposes; i++){
        int rowoffset = 9 * i;
        for(int j=0; j<nshapes; j++){   // 46
            int coloffset = 9 * j;
            for(int k=0; k<9; k++){
                Adata_coeffs.push_back(Tripletd(rowoffset+k, coloffset+k, alpha[i][j]));
            }
        }
    }
    
    
    int nrows = (nposes + nshapes) * 9;
    int ncols = nshapes * 9;
    
    std::vector<std::vector<Eigen::Matrix3d> > M(nfaces);
    //#pragma omp parallel for
    for(int j=0; j<nfaces; j++){
        if(j%1000 == 0) std::cout<<j<<std::endl<<std::flush;
        //cout<<"face "<<j<<endl;    
        
        Eigen::VectorXd b(nrows);
        
        Eigen::Matrix3d B0j = B0grad[j];
        for(int i=0; i<nposes; i++){
            Eigen::Matrix3d Sgrad_ij = Sgrad[i][j]; //20 * 22800 * mat33
            for(int k=0; k<9; k++){
                b(i * 9 + k) = Sgrad_ij(k/3, k%3) - B0j(k/3, k%3);
            }
        }
        
        // lower part of A
        std::vector<Tripletd> A_coeffs = Adata_coeffs;
        for(int i=0; i<nshapes; i++){
            int rowoffset = (nposes + i) * 9;
            int coloffset = i * 9;
            for(int k=0; k<9; k++){
                A_coeffs.push_back(Tripletd(rowoffset+k, coloffset+k, beta * w_prior[i][j]));
            }
        }
        
        // lower part of B
        for(int i=0; i<nshapes; i++){
            for(int k=0; k<9; k++){
                b( (nposes + i) * 9 + k) = beta * w_prior[i][j] * prior[i][j](k/3, k%3);
            }
        }
        
        //cout<<"Constructing linear system done..."<<endl;
        
        Eigen::SparseMatrix<double> A(nrows, ncols);
        A.setFromTriplets(A_coeffs.begin(), A_coeffs.end());
        A.makeCompressed();
        Eigen::SparseMatrix<double> AtA = (A.transpose() * A).pruned();
        
        // epsilon
        const double epsilon = 1e-6;
        Eigen::SparseMatrix<double> eye(ncols, ncols);
        for(int j=0; j<ncols; j++) eye.insert(j,j) = epsilon;
        AtA += eye;
        
        // solve Ax = b
        Eigen::CholmodSupernodalLLT< Eigen::SparseMatrix<double> > solver;
        solver.compute(AtA);
        
        Eigen::VectorXd x = solver.solve(A.transpose() * b);  //vectorXd //46 * 9
        
        M[j] = std::vector<Eigen::Matrix3d>(nshapes);  
        for(int i=0; i<nshapes; i++){
            for(int k=0; k<9; k++){
                M[j][i](k/3, k%3) = x(9*i + k, 0); 
            }
        }
    }
    
    // reconstruct the blendshape
    ColorStream(ColorOutput::Blue)<<"reconstruct the blendshape from local gradient frame ...";
    MeshTransfer transfer;
    transfer.setSource(B0);
    transfer.setTarget(B0);
    
    std::vector<trimesh::TriMesh> B_new(nshapes);
    for(int i=0; i<nshapes; i++){        
        std::vector<Eigen::Matrix3d> Bgrad_i(nfaces);
        for(int j=0; j<nfaces; j++){
            Eigen::Matrix3d M0j = B0grad[j];
            Eigen::Matrix3d Mij = M[j][i];
            
            Bgrad_i[j] = ((Mij + M0j) * M0j.inverse()).transpose();       
        }
        B_new[i] = transfer.transfer(Bgrad_i, true);
        B_new[i].write("B/B_" + std::to_string(i) + ".obj");
    }
    return B_new;
}


void BlendshapeGenerator::blendshapeGeneration(){
    std::string A_path = "../data/Tester_1/Blendshape/";     // 47 template blenshape
    std::string T_path = "../data/Tester_1/TrainingPose/";   // 20 template poses
    std::string S_path = "../data/Tester_101/TrainingPose/"; // 20 training poses
    std::string B_path = "../data/Tester_101/Blendshape/";   // 47 blendshape to be solved (unknown)
    
    const int nshapes = NUM_SHAPES;  //47
    const int nposes  = NUM_POSES;   //20

    // given A0-A46, T0-T19, S0-S19
    // -> B0-B46
    std::vector<trimesh::TriMesh> A(nshapes); // 47 example blendshapes
    std::vector<trimesh::TriMesh> T(nposes);  // 20 example poses
    std::vector<trimesh::TriMesh> S(nposes);  // 20 training poses
    std::vector<trimesh::TriMesh> B(nshapes); // 47 unknown variables
    
    std::vector<trimesh::TriMesh> B_new(nshapes-1);
    
    // load the template blendshapes and groundtruth blendshapes
    for(int i=0; i<nshapes; i++){
        A[i]    = *trimesh::TriMesh::read(A_path + "shape_" + std::to_string(i) + ".obj");
    }
    
    for(int i=0; i<nshapes-1; i++){
        
        //B_new[i] = *trimesh::TriMesh::read(B_path + "shape_" + std::to_string(i+1) + ".obj");
    }
    
    // load training poses
    for(int i=0; i<nposes; i++){
        T[i] = *trimesh::TriMesh::read(T_path + "pose_" + std::to_string(i) + ".obj");
        S[i] = *trimesh::TriMesh::read(S_path + "pose_" + std::to_string(i) + ".obj");
    }
    
    trimesh::TriMesh A0 = A[0];
    trimesh::TriMesh B0 = S[0];
    
    // step 1.1: initialize B blendshape
    std::cout<<"1.1 Intialize Blendshape B ... "<<std::endl;
    MeshTransfer transfer;
    transfer.setSource(A0);
    transfer.setTarget(B0);
    std::vector<trimesh::TriMesh> B_init(nshapes);
    B_init[0] = B0;
    for(int i=1; i<nshapes; i++){
        std::cout << i <<" ";
        B_init[i] = transfer.transfer(A[i]);
        B_init[i].write("initBlendshape/"+std::to_string(i)+".obj");
    }
    std::cout<<"    Done"<<std::endl;
    
    
    // step 1.2: the delta shapes
    std::vector<trimesh::TriMesh> dB(nshapes-1);  // 46
    for(int i=0; i<nshapes-1; i++){
        dB[i] = B_init[i+1];
        for(int j=0; j<dB[i].vertices.size(); j++){
            dB[i].vertices[j] -= B0.vertices[j];
        }
    }
    
    
    // step 2.1: estimate the initial weights
    std::cout<<"2.1 Estimate initial weight ..."<<std::endl;
    std::vector<double> w0_vec(nshapes-1, 0.0), wp_vec(nshapes-1, 0.0);
    w0_vec[0] = 1.0;
    std::vector<std::vector<double> > alpha_refs(nposes, std::vector<double>(nshapes-1, 0.0));  // 20 * 46
    for(int i=0; i<nposes; i++)
    {
        std::cout << i <<" ";
//        alpha_refs[i] = estimateWeights(S[i], B0, dB, w0_vec, wp_vec, true);
//         
//         trimesh::TriMesh ret = reconstructMesh(alpha_refs[i], B0, dB);
//         ret.write("reconstruct/"+std::to_string(i)+".ply");
        
    }
    std::cout<<"    Done"<<std::endl;
    
    
    // step 2.2: triangle gradient
    std::cout<<"2.2 TriangleGradient for A0 and B0 ...";
    int nfaces = A0.faces.size();
    std::vector<Eigen::Matrix3d> MA0, MB0;  // 22800 * mat33 // local frame for each triangle
    for(int j=0; j<nfaces; j++){
        Eigen::Matrix3d MA0j = triangleGradient(A0, j);
        Eigen::Matrix3d MB0j = triangleGradient(B0, j);
        MA0.push_back(MA0j);
        MB0.push_back(MB0j);
    }
    std::cout<<"    Done"<<std::endl;
    
    
    // step 3.1: prior parameters
    std::cout<<"3.1 w_prior and prior ..."<<std::endl;
    double kappa = 0.1, theta = 2;
    std::vector<std::vector<Eigen::Matrix3d> > prior(nshapes-1, std::vector<Eigen::Matrix3d>(nfaces)); // 46 * 22800 * mat33
    std::vector<std::vector<double> > w_prior(nshapes-1, std::vector<double>(nfaces, 0.0));            // 46 * 22800
    for(int i=0; i<nshapes-1; i++){   
        std::cout<<i<<" ";
        trimesh::TriMesh Ai = A[i+1];   // for A1 - A46
        
        for(int j=0; j<nfaces; j++){
            Eigen::Matrix3d MA0j = MA0[j];
            Eigen::Matrix3d MAij = triangleGradient(Ai, j);
            Eigen::Matrix3d GA0Ai = MAij * MA0j.inverse();
            Eigen::Matrix3d MB0j = MB0[j];
            prior[i][j] = GA0Ai * MB0j - MB0j;   // localframe for B
            
            // GA0Ai * MA0j - MA0j = MAij * MA0j.inverse() * MA0j - MA0j =  MAij * MA0j - MA0j
            double MAij_norm = (MAij - MA0j).norm();   
            w_prior[i][j] = pow( (1 + MAij_norm) / (kappa + MAij_norm), theta);
        }
    }
    std::cout<<"    Done"<<std::endl;
    
    
    // triangleGradient for S
    std::vector<std::vector<Eigen::Matrix3d> > Sgrad(nposes, std::vector<Eigen::Matrix3d>(nfaces));   // 20 * 22800 * mat33
    for(int i=0; i<nposes; i++){
        for(int j=0; j<nfaces; j++){
            Eigen::Matrix3d Sij = triangleGradient(S[i], j); 
            Sgrad[i][j] = Sij;
        }
    }
    
    
    /***********************************************
     *************** main loop *********************
     * ********************************************/
    bool   converged = false;
    int    maxIters = 10;//10;
    double beta_max  = 0.5,  beta_min  = 0.1;
    double gamma_max = 1000, gamma_min = 100;
    int numVerts = B[0].vertices.size();    // 11510
    
    std::vector<std::vector<double> > alpha = alpha_refs;   // 20*46
    int iters = 0;
    while(iters < maxIters && !converged){
        cout<<"Iteration: "<<iters<<endl;
        converged = false;
        
        double beta  = beta_max  - 1.0*iters/maxIters * (beta_max  - beta_min);
        double gamma = gamma_max - 1.0*iters/maxIters * (gamma_max - gamma_min);
        
        // refine blendshapes
        B_new = optimizeBlendshapes(S, Sgrad, B0, alpha, beta, gamma, prior, w_prior);
        for(int i=0; i<nshapes-1; i++) B[i+1] = B_new[i];
        
        
        // update delta shapes
        for(int i=0; i<nshapes-1; i++){
            for(int k=0; k<numVerts; k++){
                dB[i].vertices[k] = B[i+1].vertices[k] - B[0].vertices[k];
            }
        }
        
        // update weights
        for(int i=0; i<nposes; i++){
            alpha[i] = estimateWeights(S[i], B0, dB, alpha[i], wp_vec, true);
        }
        
        // update reconstruction vertices
        for(int i=0; i<nposes; i++){
            trimesh::TriMesh tmp = B0;
            for(int j=0; j<nshapes-1; j++){
                for(int k=0; k<numVerts; k++){
                    tmp.vertices[k] += alpha[i][j] * dB[j].vertices[k];
                }
            }
            S[i] = tmp;
        }
    
        
        // compute deformation gradient for each triangle of S
        for(int i=0; i<nposes; i++){
            for(int j=0; j<nfaces; j++){
                Sgrad[i][j] = triangleGradient(S[i], j);
            }
        }
    
        iters++;
        if(converged || iters == maxIters) break;
        
    }
    
    
    // save output
    for(int i=0; i<nshapes; i++){
        B[i].write("B/B_" + std::to_string(i) + ".obj");
    }
    for(int i=0; i<nposes; i++){
        S[i].write("S/S_" + std::to_string(i) + ".obj");
    }
    
    return;
    
}


```

## Source file: src/DeformationTransfer.cpp

{
  "path": "src/DeformationTransfer.cpp",
  "blob_sha": "14963d4c296173315ba2cfd9c34bc5dff53f9062",
  "sha256": "cd7b04e44d348711e97187997dcb2635d060acebe5c9f969dbedda111eed7e38",
  "lines": 355,
  "bytes": 9453,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 12,
    "face": 13
  },
  "url": "https://github.com/jpl917/BlendshapeGeneration/blob/7b38520c3b98fe6e980d5c6b2a49735a2ad5a117/src/DeformationTransfer.cpp",
  "function_signature_candidates": [
    {
      "line": 217,
      "text": "inline void getMatrixT(const Eigen::Matrix3d& G,"
    },
    {
      "line": 260,
      "text": "inline void fillCooSys(std::vector<Eigen::Triplet<double> >& cooSys,"
    }
  ]
}

```text

#include "DeformationTransfer.h"

void DeformationTransfer::init(const trimesh::TriMesh& _A0,
                               const trimesh::TriMesh& _B0)
{
    m_A0 = _A0;
    m_B0 = _B0;      
    
    m_numVerts = m_A0.vertices.size();
    m_numFaces = m_A0.faces.size();
    m_numTotal  = m_numVerts + m_numFaces;
    
    cout<<"numVerts:"<<m_numVerts<<"  numFaces:"<<m_numFaces<<endl;
    
    // simply use point 0 as anchor points for global position
    m_anchors.clear();
    //m_anchors.push_back(0);
    
    for(int i=0; i<m_A0.vertices.size(); i++){
        if(m_A0.is_bdy(i)) m_anchors.push_back(i);
    }
    cout<<"# anchor: "<<m_anchors.size()<<endl;
    
    
    //pre-compute
    setupAnchorMat();
    setupAnchorRhs();
    
    setupRegularizationMat();
    setupRegularizationRhs();
    
    setupE1Mat();
    
    /**********************************************
     * parameters
    ***********************************************/
//  const double Transfer_Weight_Smoothness = 1e-3;
// 	const double Transfer_Weight_Identity   = 1e-6;
// 	const double Transfer_Weight_Correspond = 1.0;
// 	const double Transfer_Weight_Anchor     = 1e8;
// 	const double Transfer_Weight_Regularization = 1e-8;

//     w_anchor = Transfer_Weight_Anchor         / (1e-3f + m_anchorMat.rows());
//     w_reg    = Transfer_Weight_Regularization / (1e-3f + m_regAtA.rows());
//     w_corres = Transfer_Weight_Correspond     / (1e-3f + m_E1Mat.rows());

    w_anchor = 0; 
    w_reg    = 0; 
    w_corres = 1; 
    
    cout<<"Parameters = [anchor: " << w_anchor <<  "]  [reglar: " << w_reg<< "]  [corres: " << w_corres <<"]"<<endl;
    

    
    m_anchorMatT = m_anchorMat.transpose();  
    m_E1MatT = m_E1Mat.transpose();
    
    m_AtA = w_corres * m_E1MatT * m_E1Mat + w_anchor * m_anchorMatT * m_anchorMat  + w_reg * m_regAtA;
    m_solver.compute(m_AtA);

    
    m_anchorRegSumAtb = w_anchor * m_anchorMatT * m_anchorRhs + w_reg * m_regAtb;
    
    m_initFlag = true;
    
    return;
}


bool DeformationTransfer::transfer(const trimesh::TriMesh& _A1, 
                                   trimesh::TriMesh& _B1)
{
    
    if(!m_initFlag) return false;
    
    // computing all energy matrices
    setupE1Rhs(_A1);
    
    //// sum all the energy terms
    m_Atb = m_anchorRegSumAtb + w_corres * m_E1MatT * m_E1Rhs;
    
    //solve
    m_x = m_solver.solve(m_Atb);
    
    
    _B1 = m_B0;
    //save results
    saveResultMesh(m_x, _B1);
    
    return true;
}

void DeformationTransfer::saveResultMesh(const Eigen::Matrix<double, -1, 1>& m_x,
                                  trimesh::TriMesh& _B1)
{
    //cout<<m_x.size()<<endl;
    for(int i=0; i<m_numVerts; i++){
        for(int k=0; k<3; k++){
            _B1.vertices[i][k] = m_x[k * (m_x.size()/3) + i];
        }
    }
    _B1.write("ret_deformTransfer.ply");
}


void DeformationTransfer::setupAnchorMat(){
    m_anchorMat.resize(3*m_anchors.size(), 3*m_numTotal);
    
    for(int i=0; i<m_anchors.size(); i++){
        int idx = m_anchors[i];
        for(int k=0; k<3; k++){
            m_anchorMat.insert(i*3+k, idx + k*m_numTotal) = 1;
        }
    }
    m_anchorMat.finalize();
}


void DeformationTransfer::setupAnchorRhs(){
     m_anchorRhs.resize(3*m_anchors.size());
     m_anchorRhs.setZero();
     
     for(int i=0; i<m_anchors.size(); i++){
         int idx = m_anchors[i];
         for(int k=0; k<3; k++){
             m_anchorRhs[i*3+k] = m_B0.vertices[idx][k];
        }
    }
}

void DeformationTransfer::setupRegularizationMat(){
    m_regAtA.resize(3*m_numTotal, 3*m_numTotal);
    m_regAtA.reserve(3*m_numTotal);
    
    for(int i=0; i<3*m_numTotal; i++){
        m_regAtA.insert(i,i) = 1;
    }
    m_regAtA.finalize();
}


Eigen::Matrix<double, 3, 4> triangleLocalFrame(
    const trimesh::TriMesh& mesh, const int& fidx)
{
	trimesh::TriMesh::Face f = mesh.faces[fidx];
	
	trimesh::point v0 = mesh.vertices[f[0]];
	trimesh::point v1 = mesh.vertices[f[1]];
	trimesh::point v2 = mesh.vertices[f[2]];
	
	Eigen::Vector3d v1v0 = Eigen::Vector3d(v1.x-v0.x, v1.y-v0.y, v1.z-v0.z);
	Eigen::Vector3d v2v0 = Eigen::Vector3d(v2.x-v0.x, v2.y-v0.y, v2.z-v0.z);
	
	Eigen::Vector3d n = v1v0.cross(v2v0);
	Eigen::Vector3d nn = n.normalized();
	
	Eigen::Matrix<double, 3, 4> G;

	G << v0[0], v1[0], v2[0], v0[0] + nn[0],
		 v0[1], v1[1], v2[1], v0[1] + nn[1],
		 v0[2], v1[2], v2[2], v0[2] + nn[2];

	return G;
}


void DeformationTransfer::setupRegularizationRhs(){
    m_regAtb.resize(3*m_numTotal);
    m_regAtb.setZero();
    
    for(int i=0; i<m_numVerts; i++){
        for(int k=0; k<3; k++){ 
            m_regAtb[k * m_numTotal + i] = m_B0.vertices[i][k];
        }
    }
    
    for(int i=0; i<m_numFaces; i++){
        Eigen::Matrix<double, 3, 4> localFrame = triangleLocalFrame(m_B0, i);
        for(int k=0; k<3; k++){
            m_regAtb[k * m_numTotal + m_numVerts + i] = localFrame(k, 3);
        }
    }
}



Eigen::Matrix3d DeformationTransfer::triangleGradient(
    const trimesh::TriMesh& mesh, const int& fidx)
{
	trimesh::TriMesh::Face f = mesh.faces[fidx];
	
	trimesh::point v0 = mesh.vertices[f[0]];
	trimesh::point v1 = mesh.vertices[f[1]];
	trimesh::point v2 = mesh.vertices[f[2]];
	
	Eigen::Vector3d v1v0 = Eigen::Vector3d(v1.x-v0.x, v1.y-v0.y, v1.z-v0.z);
	Eigen::Vector3d v2v0 = Eigen::Vector3d(v2.x-v0.x, v2.y-v0.y, v2.z-v0.z);
	
	Eigen::Vector3d n = v1v0.cross(v2v0);
	Eigen::Vector3d nn = n.normalized();
	
	Eigen::Matrix3d G;

	G << v1v0[0], v2v0[0], nn[0],
		 v1v0[1], v2v0[1], nn[1],
		 v1v0[2], v2v0[2], nn[2];

	return G;
}



// G
// |v1v0[0], v2v0[0], nn[0]|
// |v1v0[1], v2v0[1], nn[1]|
// |v1v0[2], v2v0[2], nn[2]|
inline void getMatrixT(const Eigen::Matrix3d& G,
                       Eigen::Matrix<double, 3, 4>& A)
{
    // G inverse()
    Eigen::Matrix3d V = G.inverse();
    //|-V(0, 0) - V(1, 0) - V(2, 0),   V(0, 0),   V(1, 0),   V(2, 0)|
    //|-V(0, 1) - V(1, 1) - V(2, 1),   V(0, 1),   V(1, 1),   V(2, 1)|
    //|-V(0, 2) - V(1, 2) - V(2, 2),   V(0, 2),   V(1, 2),   V(2, 2)|
    
    A(0, 0) = -V(0, 0) - V(1, 0) - V(2, 0);
	A(0, 1) =  V(0, 0);
	A(0, 2) =  V(1, 0);
	A(0, 3) =  V(2, 0);
    
	A(1, 0) = -V(0, 1) - V(1, 1) - V(2, 1);
	A(1, 1) =  V(0, 1);
	A(1, 2) =  V(1, 1);
	A(1, 3) =  V(2, 1);
    
	A(2, 0) = -V(0, 2) - V(1, 2) - V(2, 2);
	A(2, 1) =  V(0, 2);
	A(2, 2) =  V(1, 2);
	A(2, 3) =  V(2, 2);
}


//  Eigen::Matrix3d TVinv = TV.inverse();
//         Eigen::Vector3d s;
//         s[0] = -TVinv(0, 0) - TVinv(1, 0);
//         s[1] = -TVinv(0, 1) - TVinv(1, 1);
//         s[2] = -TVinv(0, 2) - TVinv(1, 2);
//         
//         m_T0grad[i] << s[0], TVinv(0,0), TVinv(1,0),
//                        s[1], TVinv(0,1), TVinv(1,1),
//                        s[2], TVinv(0,2), TVinv(1,2);



// The matrix T is in block diag style:
// | A 0 0 |
// | 0 A 0 |
// | 0 0 A |
// where each A is a 3x4 matrix  
inline void fillCooSys(std::vector<Eigen::Triplet<double> >& cooSys,
                       const Eigen::Matrix<double, 3, 4>& T,
                       const int& row, const int& m_numTotal,
                       const vector<int>& id)
{
    int pos = row * 4;
    for(int k = 0; k < 3; k++) // 3
    {  
        int yb = k * 3;        
        for(int y = 0; y < 3; y++)   // 3
        {
            for(int x = 0; x<4; x++) // 4
            {
                int col = m_numTotal * k + id[x];  
                cooSys[pos++] = Eigen::Triplet<double>(row+yb+y, col, T(y, x));
            }
        }
    }
    return;
}


// gradient transfer for mesh B0
// Ax=b
// A(m_numFaces * 9, m_numTotal * 3)
void DeformationTransfer::setupE1Mat(){
    std::vector<Eigen::Triplet<double> > cooSys;
    cooSys.resize(m_numFaces * 9 * 4);   
    
    Eigen::Matrix<double, 3, 4> Ti;
    Ti.setZero();
    
    for(int fidx=0; fidx<m_numFaces; fidx++){
        Eigen::Matrix3d G = triangleGradient(m_B0, fidx);
        getMatrixT(G, Ti); //Ti (3, 4)
        
        trimesh::TriMesh::Face f = m_B0.faces[fidx];
        vector<int> pidx = {f[0], f[1], f[2], m_numVerts + fidx};
        
        const int row = fidx * 9;
        fillCooSys(cooSys, Ti, row, m_numTotal, pidx);
    }
    
    
    //fill matrix A
    m_E1Mat.resize(m_numFaces * 9, m_numTotal * 3);
    if(cooSys.size()>0){  
        m_E1Mat.setFromTriplets(cooSys.begin(), cooSys.end());  
    }
}



void DeformationTransfer::setupE1Rhs(const trimesh::TriMesh& A1){
    m_E1Rhs.resize(m_numFaces * 9);
    
    Eigen::Matrix<double, 3, 4> Si_A;
    Si_A.setZero();
    Eigen::Matrix<double, 4, 1> Si_x[3];
    Eigen::Matrix<double, 3, 1> Si_b[3];
    
    for(int fidx = 0; fidx < m_numFaces; fidx++){
        Eigen::Matrix3d G0 = triangleGradient(m_A0, fidx);        
        getMatrixT(G0, Si_A);
        
        Eigen::Matrix<double, 3, 4> localFrame = triangleLocalFrame(A1, fidx);
        
        for(int k=0; k<4; k++){
            Si_x[0][k] = localFrame(0, k);
            Si_x[1][k] = localFrame(1, k);
            Si_x[2][k] = localFrame(2, k);
        }
        
        Si_b[0] = Si_A * Si_x[0];
		Si_b[1] = Si_A * Si_x[1];
		Si_b[2] = Si_A * Si_x[2];
        
        // push matrix
		const int row = 9 * fidx;
		m_E1Rhs[row + 0] = Si_b[0][0];
		m_E1Rhs[row + 1] = Si_b[0][1];
		m_E1Rhs[row + 2] = Si_b[0][2];

		m_E1Rhs[row + 3] = Si_b[1][0];
		m_E1Rhs[row + 4] = Si_b[1][1];
		m_E1Rhs[row + 5] = Si_b[1][2];

		m_E1Rhs[row + 6] = Si_b[2][0];
		m_E1Rhs[row + 7] = Si_b[2][1];
		m_E1Rhs[row + 8] = Si_b[2][2];
    }
}






```

## Source file: include/BlendshapeGenerator.h

{
  "path": "include/BlendshapeGenerator.h",
  "blob_sha": "b84d2fa5fc0e0d90a8970bf7c06a6d158f8a01d6",
  "sha256": "e72a64737260582bbc437ea4a38470a56422a5d098bab7aa729189afe4dea903",
  "lines": 38,
  "bytes": 1610,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 5
  },
  "url": "https://github.com/jpl917/BlendshapeGeneration/blob/7b38520c3b98fe6e980d5c6b2a49735a2ad5a117/include/BlendshapeGenerator.h",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "void blendshapeGeneration();"
    }
  ]
}

```text

#ifndef BLENDSHAPE_GENERATOR_H
#define BLENDSHAPE_GENERATOR_H

#include "Common.h"
#include "MeshTransfer.h"

class BlendshapeGenerator{
public:
    void blendshapeGeneration();
    
    
protected:
    std::vector<double> estimateWeights(const trimesh::TriMesh& S, 
                                        const trimesh::TriMesh& B0,
                                        const std::vector<trimesh::TriMesh>& dB,
                                        const std::vector<double>& w0,
                                        const std::vector<double>& wp,
                                        bool  prior_weight_flag = true);
    
    trimesh::TriMesh reconstructMesh(const std::vector<double>& weight, 
                                     const trimesh::TriMesh& B0,
                                     const std::vector<trimesh::TriMesh>& dB);
    
    std::vector<trimesh::TriMesh> optimizeBlendshapes(const std::vector<trimesh::TriMesh>& S,
                                                      const std::vector<std::vector<Eigen::Matrix3d> >& Sgrad,
                                                      const trimesh::TriMesh& B0,
                                                      const std::vector<std::vector<double> >& alpha,
                                                      double beta, double gamma,
                                                      const std::vector<std::vector<Eigen::Matrix3d> >& prior,
                                                      const std::vector<std::vector<double> >& w_prior);

private:
    std::vector<trimesh::TriMesh> m_S;
    
};


#endif


```

## Source file: include/DeformationTransfer.h

{
  "path": "include/DeformationTransfer.h",
  "blob_sha": "7bc28b6077c18cfd79ec1ed97993fb935ca465ab",
  "sha256": "2c84819ecb70d6677a06a8eaa5f6d8976aac9630319556e9a18cdc1a870f9eed",
  "lines": 89,
  "bytes": 2164,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 6,
    "face": 1
  },
  "url": "https://github.com/jpl917/BlendshapeGeneration/blob/7b38520c3b98fe6e980d5c6b2a49735a2ad5a117/include/DeformationTransfer.h",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "void init(const trimesh::TriMesh& _A0,"
    },
    {
      "line": 31,
      "text": "bool transfer(const trimesh::TriMesh& _A1,"
    },
    {
      "line": 36,
      "text": "void setupAnchorMat();"
    },
    {
      "line": 37,
      "text": "void setupAnchorRhs();"
    },
    {
      "line": 39,
      "text": "void setupRegularizationMat();"
    },
    {
      "line": 40,
      "text": "void setupRegularizationRhs();"
    },
    {
      "line": 42,
      "text": "void setupE1Mat();"
    },
    {
      "line": 43,
      "text": "void setupE1Rhs(const trimesh::TriMesh& A1);"
    },
    {
      "line": 45,
      "text": "void saveResultMesh(const Eigen::Matrix<double, -1, 1>& m_x,"
    }
  ]
}

```text

// Deformation Transfer
#ifndef DEFORMATION_TRANSFER
#define DEFORMATION_TRANSFER

#include "Common.h"

typedef Eigen::Matrix<double, -1,  1> Vec;
typedef Eigen::Matrix<double, -1, -1> Mat;
typedef Eigen::SparseMatrix<double, Eigen::ColMajor> SpMat;
typedef Eigen::Vector3f Float3;
typedef Eigen::Matrix3f Mat3f;
typedef Eigen::Vector3i Int3;
typedef Eigen::Vector4i Int4;

/* 
 * Input:  
 *      A0, A1, B0
 * Output: 
 *      B1
 * Where Ai, Bi are triangle meshes with the same topology
 */
class DeformationTransfer{
public:
    DeformationTransfer(){}
    ~DeformationTransfer(){}
    
    void init(const trimesh::TriMesh& _A0,
              const trimesh::TriMesh& _B0);
    
    // given A1, to solve B1
    bool transfer(const trimesh::TriMesh& _A1,
                  trimesh::TriMesh& _B1);
    
    
private:
    void setupAnchorMat();
    void setupAnchorRhs();
    
    void setupRegularizationMat();
    void setupRegularizationRhs();
    
    void setupE1Mat();
    void setupE1Rhs(const trimesh::TriMesh& A1);
    
    void saveResultMesh(const Eigen::Matrix<double, -1, 1>& m_x,
                                  trimesh::TriMesh& _B1);
    
    Eigen::Matrix3d triangleGradient(const trimesh::TriMesh& mesh,
                                    const int& fidx);

    
private:
    bool m_initFlag;
    trimesh::TriMesh m_A0, m_B0; // A0 and B0 share the same topology
    int m_numVerts, m_numFaces, m_numTotal;
    std::vector<int> m_anchors;  //index of anchor points
    
    
    //weigths
    double w_anchor;
    double w_reg;
    double w_corres;
    
    // for anchor
    SpMat m_anchorMat;
    SpMat m_anchorMatT;
    Eigen::Matrix<double, -1, 1> m_anchorRhs;
    
    // regularization
    SpMat m_regAtA;
    Eigen::Matrix<double, -1, 1> m_regAtb;
    Eigen::Matrix<double, -1, 1> m_anchorRegSumAtb;
    
    // energy for source-target triangle correspondences
    SpMat m_E1Mat, m_E1MatT;
    Eigen::Matrix<double, -1, 1> m_E1Rhs;
    
    // total energy 
    SpMat m_AtA;
    Eigen::Matrix<double, -1, 1> m_Atb;
    Eigen::Matrix<double, -1, 1> m_x;
    
    // solver
    Eigen::SimplicialCholesky<SpMat> m_solver;
};



#endif


```
