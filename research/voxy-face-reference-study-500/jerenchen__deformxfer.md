# Facial rendering source scan: jerenchen/deformxfer

Repository: https://github.com/jerenchen/deformxfer

Pinned source revision: 0477d631e282cd751abccdd224b79342f1bfe07b

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"deform": ["dx/deformxfer.h"]}

## Source file: dx/deformxfer.h

{
  "path": "dx/deformxfer.h",
  "blob_sha": "3091fd03f691f620ff01d5a62f2c5138cf3f89db",
  "sha256": "a0fd4da8e00683866b64c32a91f017c8ea992c09d7b5d592431aff1a31b3f91c",
  "lines": 718,
  "bytes": 18565,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1,
    "deform": 20,
    "face": 17
  },
  "url": "https://github.com/jerenchen/deformxfer/blob/0477d631e282cd751abccdd224b79342f1bfe07b/dx/deformxfer.h",
  "function_signature_candidates": [
    {
      "line": 46,
      "text": "void compute_Es_(const Eigen::MatrixXi&);"
    },
    {
      "line": 47,
      "text": "void compute_Ei_(const Eigen::MatrixXi&);"
    },
    {
      "line": 48,
      "text": "void compute_Ed_("
    },
    {
      "line": 53,
      "text": "bool fit_into_("
    },
    {
      "line": 78,
      "text": "bool initialize("
    },
    {
      "line": 82,
      "text": "bool map_correspondences("
    },
    {
      "line": 91,
      "text": "bool precompute(const Eigen::VectorXi& CI);"
    },
    {
      "line": 92,
      "text": "bool transfer("
    },
    {
      "line": 105,
      "text": "void compute_inverse_transforms("
    },
    {
      "line": 131,
      "text": "void compute_deform_rotations("
    },
    {
      "line": 174,
      "text": "inline void add_frobenius_row_("
    },
    {
      "line": 226,
      "text": "bool compute_AtA("
    },
    {
      "line": 259,
      "text": "bool compute_AtC("
    },
    {
      "line": 291,
      "text": "void find_correspondences("
    },
    {
      "line": 351,
      "text": "void closest_valid_points("
    }
  ]
}

```text

/*
DeformXfer (https://github.com/jerenchen/deformxfer)

An Eigen/libIGL (C++) implementation of "Deformation transfer for triangle meshes"
  (https://people.csail.mit.edu/sumner/research/deftransfer/), largely based on
  the Python implementation of "Landmark-guided deformation transfer of template
  facial expressions for automatic generation of avatar blendshapes"
  (https://github.com/diegothomas/Avatar-generation-3DRW2019-)

This source code is licensed under the BSD 3-Clause "New" or "Revised" License
*/
#ifndef _DEFORMXFER_H_
#define _DEFORMXFER_H_

#include <Eigen/Dense>
#include <Eigen/Sparse>
#include <Eigen/SparseLU>

#include <igl/triangle_triangle_adjacency.h>
#include <igl/per_vertex_normals.h>
#include <igl/barycenter.h>

#include <iostream>
#include <vector>


namespace dx
{
class DeformXfer
{
  unsigned int n_verts_, n_faces_, n_dim_;

  std::vector<Eigen::Matrix<double, 3 ,3> > T_inv_;

  Eigen::SparseMatrix<double> Ed_At_;  // displacement terms
  Eigen::SparseMatrix<double> Ed_AtA_;
  Eigen::SparseMatrix<double> Ed_AtC_;
  Eigen::SparseMatrix<double> Es_AtA_; // smoothness, Equation (11)
  Eigen::SparseMatrix<double> Es_AtC_;
  Eigen::SparseMatrix<double> Ei_AtA_; // deformation identity, Equation (12)
  Eigen::SparseMatrix<double> Ei_AtC_;
  Eigen::SparseMatrix<double> AtA_;
  Eigen::SparseMatrix<double> AtC_;
  Eigen::SparseMatrix<double> At_;

  void compute_Es_(const Eigen::MatrixXi&);
  void compute_Ei_(const Eigen::MatrixXi&);
  void compute_Ed_(
    const Eigen::MatrixXi&,
    const std::vector<std::pair<int, int> >&
  );

  bool fit_into_(
    const Eigen::MatrixXd&,
    const Eigen::MatrixXi&,
    const Eigen::MatrixXd&,
    const Eigen::MatrixXi&,
    const std::vector<std::pair<int, int> >&,
    std::vector<Eigen::MatrixXd>&
  );

  Eigen::SparseLU<Eigen::SparseMatrix<double> > solver_;

public:

  bool keep_progress = false;
  double max_distance = 100;
  double threshold = 10;
  double Wd = 1;
  double Ws = 1;
  double Wi = 0.001;
  std::vector<double> Wc = {1, 50, 1000, 5000};

  DeformXfer(
    const Eigen::MatrixXd& = Eigen::MatrixXd(),
    const Eigen::MatrixXi& = Eigen::MatrixXi()
  );
  bool initialize(
    const Eigen::MatrixXd&,
    const Eigen::MatrixXi&
  );
  bool map_correspondences(
    const Eigen::MatrixXd& SV,
    const Eigen::MatrixXi& SF,
    const Eigen::MatrixXd& TV,
    const Eigen::MatrixXi& TF,
    const std::vector<std::pair<int, int> >& markers,
    std::vector<Eigen::MatrixXd>& X, // progress of source morphing into target
    std::vector<std::pair<int, int> >& M // correspondences, Equation (5)
  );
  bool precompute(const Eigen::VectorXi& CI);
  bool transfer(
    const Eigen::MatrixXd&,
    const Eigen::MatrixXi&,
    const Eigen::MatrixXd&,
    const Eigen::MatrixXd&,
    const std::vector<std::pair<int, int> >&,
    Eigen::MatrixXd&
  );
};

// utility funcs impl.

// compute inversed affine transformation matrices for triangles in V/F
void compute_inverse_transforms(
  const Eigen::MatrixXd& V,
  const Eigen::MatrixXi& F,
  std::vector<Eigen::Matrix<double, 3, 3> >& T
)
{
  Eigen::Matrix<double, -1, 3> v1 = V(F.col(0), Eigen::all);
  Eigen::Matrix<double, -1, 3> v21 = V(F.col(1), Eigen::all) - v1;
  Eigen::Matrix<double, -1, 3> v31 = V(F.col(2), Eigen::all) - v1;

  T.resize(F.rows());
  for (unsigned int ti = 0; ti < T.size(); ++ti)
  {
    const auto& v2_ = v21.row(ti);
    const auto& v3_ = v31.row(ti);
    Eigen::Vector3d v41 = v2_.cross(v3_);

    auto m = Eigen::Matrix<double, 3, 3>(3, 3);
    m.col(0) = v2_;
    m.col(1) = v3_;
    m.col(2) = v41 / std::sqrt(v41.norm());
    T[ti] = m.inverse();
  }
}

// compute 3 x 3 affine transformation matrices Q between V/F & U/F
void compute_deform_rotations(
  const Eigen::MatrixXd& V,
  const Eigen::MatrixXi& F,
  const Eigen::MatrixXd& U,
  std::vector<Eigen::Vector<double, 9> >& Q
)
{
  Q.resize(F.rows());

  Eigen::Matrix<double, -1, 3> v1 = V(F.col(0), Eigen::all);
  Eigen::Matrix<double, -1, 3> v21 = V(F.col(1), Eigen::all) - v1;
  Eigen::Matrix<double, -1, 3> v31 = V(F.col(2), Eigen::all) - v1;

  Eigen::Matrix<double, -1, 3> u1 = U(F.col(0), Eigen::all);
  Eigen::Matrix<double, -1, 3> u21 = U(F.col(1), Eigen::all) - u1;
  Eigen::Matrix<double, -1, 3> u31 = U(F.col(2), Eigen::all) - u1;

  for (unsigned int ti = 0; ti < Q.size(); ++ti)
  {
    const auto& v2_ = v21.row(ti);
    const auto& v3_ = v31.row(ti);
    Eigen::Vector3d v41 = v2_.cross(v3_);

    auto r_ = Eigen::Matrix<double, 3, 3>(3, 3);
    r_.col(0) = v2_;
    r_.col(1) = v3_;
    r_.col(2) = v41 / std::sqrt(v41.norm());

    const auto& u2_ = u21.row(ti);
    const auto& u3_ = u31.row(ti);
    Eigen::Vector3d u41 = u2_.cross(u3_);
    
    auto q_ = Eigen::Matrix<double, 3, 3>(3, 3);
    q_.col(0) = u2_;
    q_.col(1) = u3_;
    q_.col(2) = u41 / std::sqrt(u41.norm());

    q_ *= r_.inverse();
    Q[ti] = Eigen::Map<Eigen::Vector<double, 9> >(q_.data(), 9);
  }
}

// inline func to compute the Frobenius norm for a given triangle
inline void add_frobenius_row_(
  std::vector<Eigen::Triplet<double> >& coeffs,
  unsigned int row_off,
  unsigned int col_off,
  const Eigen::ArrayXi& verts,
  const Eigen::Matrix<double, 3, 3>& inv_xf,
  double mult_coeff = 1.0
)
{
  auto es = -1 * inv_xf.colwise().sum();

  for (int ii = 0; ii < 3; ++ii)
  {
    unsigned int row_ii = 9 * row_off + 3 * ii;
    for (int jj = 0; jj < 3; ++jj)
    {
      unsigned int row_ii_jj = row_ii + jj;
      coeffs.push_back( 
        Eigen::Triplet<double>(
          row_ii_jj, 
          3 * verts(0) + jj, 
          mult_coeff * es(ii)
        )
      );

      coeffs.push_back( 
        Eigen::Triplet<double>(
          row_ii_jj, 
          3 * verts(1) + jj, 
          mult_coeff * inv_xf(0, ii)
        )
      );

      coeffs.push_back( 
        Eigen::Triplet<double>(
          row_ii_jj, 
          3 * verts(2) + jj, 
          mult_coeff * inv_xf(1, ii)
        )
      );

      coeffs.push_back( 
        Eigen::Triplet<double>(
          row_ii_jj, 
          3 * col_off + jj, 
          mult_coeff * inv_xf(2, ii)
        )
      );
    }
  }
}

bool compute_AtA(
  const Eigen::VectorXi& CI,
  const int n_dim,
  Eigen::SparseMatrix<double>& At,
  Eigen::SparseMatrix<double>& AtA
)
{
  std::vector<Eigen::Triplet<double> > coeffs;

  unsigned int n_elems = 0;
  for (int ci = 0; ci < CI.size(); ++ci)
  {
    for (int ii = 0; ii < 3; ++ii)
    {
      coeffs.push_back(Eigen::Triplet<double>(n_elems, 3 * CI[ci] + ii, 1));
      ++n_elems;
    }
  }

  if (coeffs.size() == 0)
  {
    return false;
  }

  auto A = Eigen::SparseMatrix<double>(coeffs.size(), n_dim);
  A.setFromTriplets(coeffs.begin(), coeffs.end());

  At = A.transpose();
  AtA = At * A;

  return true;
}

bool compute_AtC(
  const Eigen::MatrixXd& CC,
  const Eigen::SparseMatrix<double>& At,
  Eigen::SparseMatrix<double>& AtC
)
{
  std::vector<Eigen::Triplet<double> > coeffs;

  unsigned int n_elems = 0;
  for (int ci = 0; ci < CC.rows(); ++ci)
  {
    for (int ii = 0; ii < 3; ++ii)
    {
      coeffs.push_back(Eigen::Triplet<double>(n_elems, 0, CC(ci, ii)));
      ++n_elems;
    }
  }

  if (coeffs.size() == 0)
  {
    return false;
  }

  auto C = Eigen::SparseMatrix<double>(coeffs.size(), 1);
  C.setFromTriplets(coeffs.begin(), coeffs.end());

  AtC = At * C;

  return true;
}

// find mapping between source & target triangles as M (Equation 5)
void find_correspondences(
  const Eigen::MatrixXd& SV,
  const Eigen::MatrixXi& SF,
  const Eigen::MatrixXd& TV,
  const Eigen::MatrixXi& TF,
  const double threshold,
  std::vector<std::pair<int, int> >& M
)
{
  M.clear();
  double t2 = threshold * threshold;

  auto map_triangles = [&M, t2](
    const Eigen::MatrixXd& CX,
    const Eigen::MatrixXd& NX,
    const Eigen::MatrixXd& CY,
    const Eigen::MatrixXd& NY,
    bool swap = false
  )
  {
    for (unsigned int ii = 0; ii < CX.rows(); ++ii)
    {
      Eigen::VectorXd D = (CY.rowwise() - CX.row(ii)).cwiseAbs2().rowwise().sum();
      int index;
      const auto& nx = NX.row(ii);
      while (D.minCoeff(&index) < t2)
      {
        if (NY.row(index).dot(nx) > 0)
        {
          M.push_back(swap?
            std::pair<int, int>(ii, index) : std::pair<int, int>(index, ii)
          );
          break;
        }
        else
        {
          // facing opposite dir, invalidate the vert
          D(index) = t2;
        }
      }
    }
  };

  // compute face centroids/barycenters
  Eigen::MatrixXd SBC, TBC;
  igl::barycenter(SV, SF, SBC);
  igl::barycenter(TV, TF, TBC);
  // compute per-face normals
  Eigen::MatrixXd SFN, TFN;
  igl::per_face_normals(SV, SF, SFN);
  igl::per_face_normals(TV, TF, TFN);

  // map the source triangles to the target's triangles
  map_triangles(TBC, TFN, SBC, SFN);
  // map the target triangles to the source's triangles
  map_triangles(SBC, SFN, TBC, TFN, true);
}

// find closest points on mesh Y for each vert on mesh X, valid if
//   they "face" the same direction, i.e. 2 vert normals dot product > 0
void closest_valid_points(
  const Eigen::MatrixXd& VX,
  const Eigen::MatrixXd& NX,
  const Eigen::MatrixXd& VY,
  const Eigen::MatrixXd& NY,
  const double max_distance,
  std::map<int, Eigen::Vector3d>& C
)
{
  double md2 = max_distance * max_distance;

  C.clear();
  for (unsigned int vi = 0; vi < VX.rows(); ++vi)
  {
    Eigen::VectorXd D = (VY.rowwise() - VX.row(vi)).cwiseAbs2().rowwise().sum();
    int index;
    while (D.minCoeff(&index) < md2)
    {
      if (NY.row(index).dot(NX.row(vi)) > 0)
      {
        C[vi] = VY.row(index);
        break;
      }
      else
      {   // invalidate the vertex for facing opposite dir
        D(index) = md2;
      }
    }
  }
}

// class funcs impl.

DeformXfer::DeformXfer(
  const Eigen::MatrixXd& TV,
  const Eigen::MatrixXi& TF
)
{
  initialize(TV, TF);
}

bool DeformXfer::initialize(
  const Eigen::MatrixXd& TV,
  const Eigen::MatrixXi& TF
)
{
  n_verts_ = TV.rows();
  n_faces_ = TF.rows();
  if (n_verts_ == 0 || n_faces_ == 0)
  {
    return false;
  }

  compute_inverse_transforms(TV, TF, T_inv_);

  n_dim_ = 3 * (n_verts_ + n_faces_);

  compute_Es_(TF);
  compute_Ei_(TF);
  return true;
}

void DeformXfer::compute_Es_(const Eigen::MatrixXi& F)
{
  // find adjacent triangles
  std::vector<std::vector<std::vector<int> > > adjacent_triangles;
  igl::triangle_triangle_adjacency(F, adjacent_triangles);

  std::vector<Eigen::Triplet<double> > coeffs;
  unsigned int n_adj = 0;
  for (unsigned int ti = 0; ti < adjacent_triangles.size(); ++ti)
  {
    for (unsigned int tc = 0; tc < adjacent_triangles[ti].size(); ++tc)
    {
      for (unsigned int tj : adjacent_triangles[ti][tc])
      {
        add_frobenius_row_(coeffs, n_adj, ti + n_verts_, F.row(ti), T_inv_[ti]);
        add_frobenius_row_(coeffs, n_adj, tj + n_verts_, F.row(tj), T_inv_[tj], -1);
        ++n_adj;
      }
    }
  }
  auto Es_A = Eigen::SparseMatrix<double>(9 * n_adj, n_dim_);
  Es_A.setFromTriplets(coeffs.begin(), coeffs.end());

  auto Es_At = Es_A.transpose();
  Es_AtA_ = Es_At * Es_A;
  Es_AtC_ = Es_At * Eigen::SparseMatrix<double>(Es_A.rows(), 1);
}

void DeformXfer::compute_Ei_(const Eigen::MatrixXi& F)
{
  std::vector<Eigen::Triplet<double> > coeffs;
  for (unsigned int ti = 0; ti < n_faces_; ++ti)
  {
    add_frobenius_row_(coeffs, ti, ti + n_verts_, F.row(ti), T_inv_[ti]);
  }
  auto Ei_A = Eigen::SparseMatrix<double>(9 * n_faces_, n_dim_);
  Ei_A.setFromTriplets(coeffs.begin(), coeffs.end());

  std::vector<Eigen::Triplet<double> > identities;
  for (unsigned int ti = 0; ti < n_faces_; ++ti)
  {
    for (int ii = 0; ii < 3; ++ii)
    {
      identities.push_back(Eigen::Triplet<double>(9 * ti + 3 * ii + ii, 0, 1.0));
    }
  }
  auto Ei_C = Eigen::SparseMatrix<double>(Ei_A.rows(), 1);
  Ei_C.setFromTriplets(identities.begin(), identities.end());

  auto Ei_At = Ei_A.transpose();
  Ei_AtA_ = Ei_At * Ei_A;
  Ei_AtC_ = Ei_At * Ei_C;
}

void DeformXfer::compute_Ed_(
  const Eigen::MatrixXi& F,
  const std::vector<std::pair<int, int> >& M
)
{
  std::vector<Eigen::Triplet<double> > coeffs;
  unsigned int offset = 0;
  for (const auto& it : M)
  {
    auto ti = it.second;
    add_frobenius_row_(coeffs, offset++, ti + n_verts_, F.row(ti), T_inv_[ti]);
  }
  auto Ed_A = Eigen::SparseMatrix<double>(9 * M.size(), n_dim_);
  Ed_A.setFromTriplets(coeffs.begin(), coeffs.end());

  Ed_At_ = Ed_A.transpose();
  Ed_AtA_ = Ed_At_ * Ed_A;
}

// implementing Equation (14) for the iterative optimization to 
//   fit the target mesh into the shape of the source
bool DeformXfer::fit_into_(
  const Eigen::MatrixXd& VX,
  const Eigen::MatrixXi& FX,
  const Eigen::MatrixXd& VY,
  const Eigen::MatrixXi& FY,
  const std::vector<std::pair<int, int> >& markers,
  std::vector<Eigen::MatrixXd>& X
)
{
  Eigen::VectorXi CI(markers.size());
  Eigen::MatrixXd CC(markers.size(), 3);
  int cc = 0;
  for (const auto& it : markers)
  {
    if (it.first < VY.rows() && it.second < n_verts_)
    {
      CI(cc) = it.second;
      CC.row(cc) = VY.row(it.first);
      ++cc;
    }
    else
    {
      std::cout << "Skipping invalid marker pair: (" <<
        it.first << ", " << it.second << ")..." << std::endl;
    }
  }
  CI.conservativeResize(cc);
  CC.conservativeResize(cc, 3);

  Eigen::SparseMatrix<double> At;
  Eigen::SparseMatrix<double> AtA;
  Eigen::SparseMatrix<double> AtC;
  if (!compute_AtA(CI, n_dim_, At, AtA))
  {
    return false;
  }
  if (!compute_AtC(CC, At, AtC))
  {
    return false;
  }

  std::map<int, Eigen::Vector3d> C;

  auto fit_iter = [&](double W_s, double W_i, double W_c, Eigen::MatrixXd& X_)->bool
  {
    std::cout << "Fitting X [" << W_s << "][" << W_i << "][" << W_c << "]..." << std::endl;
    Eigen::SparseMatrix<double> AtA_m = AtA + W_s * Es_AtA_ + W_i * Ei_AtA_;
    Eigen::SparseMatrix<double> AtC_m = AtC + W_s * Es_AtC_ + W_i * Ei_AtC_;

    if (C.size())
    {
      // compute closest valid point, Equation (14),
      //   using compute AtA with constraints being closest valid points
      Eigen::SparseMatrix<double> Ec_At;
      Eigen::SparseMatrix<double> Ec_AtA;
      Eigen::SparseMatrix<double> Ec_AtC;
      Eigen::VectorXi Ec_CI(C.size());
      Eigen::MatrixXd Ec_CC(C.size(), 3);
      int ci = 0;
      for (auto& c : C)
      {
        Ec_CI(ci) = c.first;
        Ec_CC.row(ci) = c.second;
        ++ci;
      }
      if (compute_AtA(Ec_CI, n_dim_, Ec_At, Ec_AtA))
      {
        AtA_m += W_c * Ec_AtA;
      }
      if (compute_AtC(Ec_CC, Ec_At, Ec_AtC))
      {
        AtC_m += W_c * Ec_AtC;
      }
    }

    Eigen::SparseLU<Eigen::SparseMatrix<double> > solver;
    solver.compute( AtA_m );
    if(solver.info() != Eigen::Success)
    {
      std::cout << solver.lastErrorMessage() << std::endl;
      X_ = Eigen::MatrixXd::Zero(n_verts_, 3);
      return false;
    }

    Eigen::VectorXd x = solver.solve( AtC_m );
    if(solver.info() != Eigen::Success)
    {
      std::cout << solver.lastErrorMessage() << std::endl;
      X_ = Eigen::MatrixXd::Zero(n_verts_, 3);
      return false;
    }

    X_ = (Eigen::Map<Eigen::MatrixXd>(x.data(), 3, n_verts_)).transpose();
    return true;
  };

  Eigen::MatrixXd NX, NY;
  igl::per_vertex_normals(VY, FY, NY);
  X.clear();
  X.push_back(Eigen::MatrixXd());

  // phase-1: ignore closest valid points
  if (!fit_iter(1.0, 0.1, 0, X.back()))
  {
    return false;
  }

  // phase-2: iteratively increase closest valid points term
  for (auto wc : Wc)
  {
    igl::per_vertex_normals(X.back(), FX, NX);
    closest_valid_points(X.back(), NX, VY, NY, max_distance, C);
    if (keep_progress)
    {
      X.push_back(Eigen::MatrixXd());
    }
    if (!fit_iter(1.0, 0.001, wc, X.back()))
    {
      return false;
    }
  }

  return true;
}

bool DeformXfer::map_correspondences(
  const Eigen::MatrixXd& SV,
  const Eigen::MatrixXi& SF,
  const Eigen::MatrixXd& TV,
  const Eigen::MatrixXi& TF,
  const std::vector<std::pair<int, int> >& markers,
  std::vector<Eigen::MatrixXd>& X, // progress of source morphing into target
  std::vector<std::pair<int, int> >& M // correspondences, Equation (5)
)
{
  if (TV.rows() != n_verts_ || TF.rows() != n_faces_)
  {
    std::cerr << "Mismatched target dimensions!" << std::endl;
    return false;
  }

  M.clear();

  if (markers.size())
  {
    // morph target into source before mapping faces between the two
    if (!fit_into_(TV, TF, SV, SF, markers, X))
    {
      std::cerr << "Unable to morph target into source!" << std::endl;
      return false;
    }
    find_correspondences(SV, SF, X.back(), TF, threshold, M);
  }

  if (!M.size())
  {
    // assume 1-to-1 mapping
    unsigned int min_faces = std::min<int>(SF.rows(), TF.rows());
    for (unsigned int ii = 0; ii < min_faces; ++ii)
    {
      M.push_back(std::pair<int, int>(ii, ii));
    }
  }

  // re-compute the displacement terms
  compute_Ed_(TF, M);
  return true;
}

bool DeformXfer::precompute(
  const Eigen::VectorXi& CI // constaint indices into target verts
)
{
  if (compute_AtA(CI, n_dim_, At_, AtA_))
  {
    solver_.compute(AtA_ + Wd * Ed_AtA_ + Ws * Es_AtA_ + Wi * Ei_AtA_);
    if(solver_.info() == Eigen::Success)
    {
      return true;
    }
    std::cout << solver_.lastErrorMessage() << std::endl;
  }
  return false;
}

bool DeformXfer::transfer(
  const Eigen::MatrixXd& SV, // source verts
  const Eigen::MatrixXi& SF, // source faces
  const Eigen::MatrixXd& DV, // deformed source verts
  const Eigen::MatrixXd& CC, // constraint conditions
  const std::vector<std::pair<int, int> >& M,
  Eigen::MatrixXd& RV // output result target verts
)
{
  if (!compute_AtC(CC, At_, AtC_))
  {
    RV = Eigen::MatrixXd::Zero(n_verts_, 3);
    return false;
  }

  std::vector<Eigen::Vector<double, 9> > Q;
  compute_deform_rotations(SV, SF, DV, Q);

  std::vector<Eigen::Triplet<double> > coeffs;
  unsigned int offset = 0;
  for (const auto& it : M)
  {
    const auto& r = Q[it.first];
    for (unsigned int ii = 0; ii < 9; ++ii)
    {
      coeffs.push_back(Eigen::Triplet<double>(offset++, 0, r[ii]));
    }
  }
  auto Ed_C = Eigen::SparseMatrix<double>(Ed_At_.cols(), 1);
  Ed_C.setFromTriplets(coeffs.begin(), coeffs.end());

  Eigen::SparseMatrix<double> Ed_AtC_ = Ed_At_ * Ed_C;
  Eigen::VectorXd x = solver_.solve(AtC_ + Wd * Ed_AtC_ + Ws * Es_AtC_ + Wi * Ei_AtC_);
  if(solver_.info() != Eigen::Success)
  {
    std::cout << solver_.lastErrorMessage() << std::endl;
    RV = Eigen::MatrixXd::Zero(n_verts_, 3);
    return false;
  }

  RV = (Eigen::Map<Eigen::MatrixXd>(x.data(), 3, n_verts_)).transpose();
  return true;
}
};

#endif

```

## Source file: demo/main.cpp

{
  "path": "demo/main.cpp",
  "blob_sha": "bb8b5636a6088c3388a1f108605f51b10fe1b365",
  "sha256": "5ceba77877cb949d404bf3b5a37781b5bc9930ec6fd0987a7b4bb4b6573d75db",
  "lines": 215,
  "bytes": 6626,
  "selection_score": -2,
  "keyword_line_counts": {
    "blendshape": 3,
    "deform": 6
  },
  "url": "https://github.com/jerenchen/deformxfer/blob/0477d631e282cd751abccdd224b79342f1bfe07b/demo/main.cpp",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "int main(int argc, char *argv[])"
    }
  ]
}

```text

/*
DeformXfer (https://github.com/jerenchen/deformxfer) - demo program

An Eigen/libIGL (C++) implementation of "Deformation transfer for triangle meshes"
  (https://people.csail.mit.edu/sumner/research/deftransfer/), largely based on
  the Python implementation of "Landmark-guided deformation transfer of template
  facial expressions for automatic generation of avatar blendshapes"
  (https://github.com/diegothomas/Avatar-generation-3DRW2019-)

This source code is licensed under the BSD 3-Clause "New" or "Revised" License
*/
#include <deformxfer.h>

#include <iostream>
#include <chrono>

#include <igl/combine.h>
#include <igl/per_vertex_normals.h>
#include <igl/read_triangle_mesh.h>
#include <igl/opengl/glfw/Viewer.h>


const double pi = std::acos(-1);
double anim_t = 0.0;
int shape_curr = 24;
int shape_prev = 0;
bool show_morph = false;

// util func to read example landmark data
std::vector<int> read_landmarks(const std::string& filename)
{
  std::cout << "Reading landmarks: " << filename << "..." << std::endl;
  std::vector<int> markers;
  std::ifstream fin;
  fin.open(filename, std::ifstream::in);
  int s_;
  while(fin >> s_)
  {
      markers.push_back(s_);
  }
  fin.close();
  return markers;
}

int main(int argc, char *argv[])
{
  Eigen::MatrixXd V, SV, TV, DV, RV;
  Eigen::MatrixXi F, SF, TF;

  // variables to store morphed targets & correspondences
  std::vector<Eigen::MatrixXd> X;
  std::vector<std::pair<int, int> > M;
  // variables to store constaint indices & conditions
  Eigen::VectorXi CI;
  Eigen::MatrixXd CC;

  // begin to read inputs

  std::vector<Eigen::MatrixXd> BS(48);
  for (unsigned int ii = 0; ii < BS.size(); ++ii)
  {
    std::string filename(BLENDSHAPE_DIR);
    std::string no("0" + std::to_string(ii));
    filename += no.substr(no.size() - 2, 2) + "m.obj";
    std::cout << "Reading " << filename << " ..." << std::endl;
    igl::read_triangle_mesh(filename, BS[ii], SF);
  }
  igl::read_triangle_mesh(std::string(BLENDSHAPE_DIR) + "Neutralm.obj", SV, SF);
  igl::read_triangle_mesh(std::string(INPUTSHAPE_DIR) + "Horse.ply", TV, TF);

  DV = SV;
  RV = TV;

  std::string landmark_dir(LANDMARKS_DIR);
  auto landmarks = read_landmarks(landmark_dir + "SOURCE/FACIAL_LANDMARKS.txt");
  auto horse_markers = read_landmarks(landmark_dir + "landmarks_Horse.txt");

  // rescale target to match source
  auto SB = SV.colwise().maxCoeff() - SV.colwise().minCoeff();
  auto TB = TV.colwise().maxCoeff() - TV.colwise().minCoeff();
  auto rescale = SB.sum() / TB.sum();
  auto center = (TV.colwise().maxCoeff() + TV.colwise().minCoeff()) / 2.0;
  TV = rescale * (TV.rowwise() - center);

  // offest from target to source
  auto offset = SV.row(landmarks[13]) - TV.row(horse_markers[13]);
  TV = TV.rowwise() + offset;

  // adding marker pairs for mapping mesh correspondences
  std::vector<std::pair<int, int> > markers;
  for (unsigned int ii = 0; ii < landmarks.size(); ++ii)
  {
    markers.push_back(std::pair<int, int>(landmarks[ii], horse_markers[ii]));
  }

  // adding backward landmarks as positional constraints
  auto backward_markers = read_landmarks(landmark_dir + "SOURCE/BACKWARD_LANDMARKS.txt");
  CI.resize(backward_markers.size());
  CC.resize(backward_markers.size(), 3);
  int ci = 0;
  for (auto idx : backward_markers)
  {
    int index;
    auto D = (TV.rowwise() - SV.row(idx)).cwiseAbs2().rowwise().sum();
    D.minCoeff(&index);
    CI(ci) = index;
    CC.row(ci) = TV.row(index);
    ++ci;
  }

  // create dx object
  dx::DeformXfer dxo;
  dxo.keep_progress = true;
  dxo.Wc = {0.1, 1.0, 10.0};
  dxo.Ws = 0.01;
  dxo.Wi = 0;

  auto start = std::chrono::high_resolution_clock::now();
  dxo.initialize(TV, TF);
  auto elapsed = std::chrono::duration<double>(std::chrono::high_resolution_clock::now() - start);
  std::cout << "Init time: " << elapsed.count() << "s\n";

  // map correspondences
  start = std::chrono::high_resolution_clock::now();
  dxo.map_correspondences(SV, SF, TV, TF, markers, X, M);
  elapsed = std::chrono::duration<double>(std::chrono::high_resolution_clock::now() - start);
  std::cout << "Map correspondences time: " << elapsed.count() << "s\n";

  // precompute large AtA matrices with the indices of contraint conditions
  start = std::chrono::high_resolution_clock::now();
  dxo.precompute(CI);
  elapsed = std::chrono::duration<double>(std::chrono::high_resolution_clock::now() - start);
  std::cout << "Pre-compute time: " << elapsed.count() << "s\n";

  // set up viewer & callbacks
  igl::opengl::glfw::Viewer viewer;

  const auto& key_down =
    [&](igl::opengl::glfw::Viewer& view, unsigned char key, int mod)->bool
  {
    switch(key)
    {
      case ' ':
      {
        view.core().is_animating ^= 1;
        anim_t = 0;
        break;
      }
      case 'M':
      case 'm':
      {
        show_morph ^= 1;
        DV = SV;
        anim_t = 0;
        break;
      }
      default:
        return false;
    }
    return true;
  };

  const auto& pre_draw = [&](igl::opengl::glfw::Viewer& view)->bool
  {
    if(viewer.core().is_animating)
    {
      if (anim_t >= 1.0)
      {
        shape_prev = shape_curr;
        shape_curr = std::rand() % BS.size();
        anim_t = 0.0;
      }

      if (show_morph)
      {
        int idx = std::floor(anim_t * (X.size() + 1) - 1);
        RV = idx < 0? TV : X[idx];
        anim_t += 0.01;
      }
      else
      {
        // smoothstep interpolation
        double w = anim_t * anim_t * (3 - 2 * anim_t);
        DV = (1 - w) * BS[shape_prev] + w * BS[shape_curr];
        dxo.transfer(SV, SF, DV, CC, M, RV);
        anim_t += 0.05;
      }
    }

    std::vector<Eigen::MatrixXd> VV;
    std::vector<Eigen::MatrixXi> FF;
    VV.push_back(DV.rowwise() + Eigen::RowVector3d(-100, 0, 0));
    FF.push_back(SF);
    VV.push_back(RV.rowwise() + Eigen::RowVector3d(100, 0, 0));
    FF.push_back(TF);
    igl::combine(VV, FF, V, F);
    view.data().set_mesh(V, F);
    // draw the constraint markers
    Eigen::MatrixXd N;
    igl::per_vertex_normals(VV[0], SF, N);
    viewer.data().set_points((VV[0] + N)(landmarks, Eigen::all), Eigen::RowVector3d(0,1,0));
    igl::per_vertex_normals(VV[1], TF, N);
    viewer.data().add_points((VV[1] + N)(horse_markers, Eigen::all), Eigen::RowVector3d(1,0,0));
    return false;
  };
  pre_draw(viewer); // this will frame meshes in view
  
  viewer.callback_pre_draw = pre_draw;
  viewer.callback_key_down = key_down;
  viewer.core().is_animating = true;
  viewer.data().point_size = 6;

  std::cout << "Press [m] to toggle the 'morph topology' mode." << std::endl;
  return viewer.launch(true, false, "DeformXfer");
}


```
