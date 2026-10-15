# Facial rendering source scan: dafei-qin/NFR_pytorch

Repository: https://github.com/dafei-qin/NFR_pytorch

Pinned source revision: 664488e224333deb46f85279b54b6dd1d02e81b1

Discovery topic: facial retargeting

Description: Offical repo of "Neural Face Rigging for Animating and Retargeting Facial Meshes in the Wild"

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "third_party/diffusion-net/LICENSE", "third_party/mesh_signatures/LICENSE"]

Topic paths: {"deform": ["deformation_transfer.py"], "shader": ["third_party/mesh_signatures/pygl/shader.py", "third_party/mesh_signatures/shaders/pbr.fs", "third_party/mesh_signatures/shaders/pbr.vs"]}

## Source file: deformation_transfer.py

{
  "path": "deformation_transfer.py",
  "blob_sha": "d83b5d4acce63eaf875c30cefb9e3d6ae0f3cb74",
  "sha256": "fbd148c93e92bfb3fdf37d6c5ffa3fcccfc591cbaee2602fe3e53ef1f7fc6f43",
  "lines": 534,
  "bytes": 21695,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 26,
    "rig": 7,
    "face": 55
  },
  "url": "https://github.com/dafei-qin/NFR_pytorch/blob/664488e224333deb46f85279b54b6dd1d02e81b1/deformation_transfer.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def calculate_jacobians(neutral_mesh, vertices):"
    },
    {
      "line": 42,
      "text": "def __init__(self, source: Mesh, target: Mesh, project: bool=False, area: bool=True, device='cuda:0', use_chol=False):"
    },
    {
      "line": 101,
      "text": "def __call__(self, pose: Mesh, project=False):"
    },
    {
      "line": 102,
      "text": "def _get_mass_matrix(vertices,faces,is_sparse):"
    },
    {
      "line": 138,
      "text": "def target_coef(self):"
    },
    {
      "line": 147,
      "text": "def target_A(self):"
    },
    {
      "line": 167,
      "text": "def target_area(self):"
    },
    {
      "line": 183,
      "text": "def get_fv(cls, mesh: Mesh):"
    },
    {
      "line": 187,
      "text": "def span_matrix(cls, mesh: Mesh):"
    },
    {
      "line": 198,
      "text": "def span_to_fv(cls, span):"
    },
    {
      "line": 206,
      "text": "def deformation_gradient(self, pose: Mesh) -> np.ndarray:"
    },
    {
      "line": 216,
      "text": "def forward(ctx, input, solver, idxs, vals, shape):"
    },
    {
      "line": 249,
      "text": "def backward(ctx, grad_output):"
    },
    {
      "line": 285,
      "text": "def forward(ctx, input, solver, idxs, vals, shape):"
    },
    {
      "line": 318,
      "text": "def backward(ctx, grad_output):"
    },
    {
      "line": 335,
      "text": "def calc_transform(mesh_x, mesh_y):"
    },
    {
      "line": 353,
      "text": "def form_V(mesh):"
    },
    {
      "line": 379,
      "text": "def calc_qr(face_vertices):"
    },
    {
      "line": 392,
      "text": "def calc_S(face_vertices, Q, q, r):"
    },
    {
      "line": 403,
      "text": "def calc_A(face_vertices, faces_idx, num_of_vertices):"
    },
    {
      "line": 422,
      "text": "def eliminate_constrains(A, S, vertices_idx, vertices_gt):"
    },
    {
      "line": 440,
      "text": "def calc_deform(A, S, tol, use_lu=True, area=None):"
    },
    {
      "line": 465,
      "text": "def calc_area(face_vertices):"
    },
    {
      "line": 471,
      "text": "def deformation_transfer(source_orig, source_deformed, target_orig, tol=1e-8, use_lu=True):"
    }
  ]
}

```text

import warnings
warnings.filterwarnings("ignore", category=PendingDeprecationWarning)
from copy import deepcopy
import igl
import trimesh
import numpy as np
np.seterr(all='ignore')
from scipy.sparse.linalg import lsqr as sparse_lsqr
from scipy.sparse.linalg import splu as sparse_lu
import scipy.sparse as sparse
import scipy
from matplotlib import pyplot as plt
from cupyx.scipy.sparse.linalg import SuperLU as cupy_SuperLU
from cupyx.scipy import sparse as cupy_sparse
import cupy
import torch
from torch.utils.dlpack import to_dlpack
from torch.utils.dlpack import from_dlpack
from torch_sparse import spmm, transpose
# from cholespy import CholeskySolverF, MatrixType
import time
import logging
from myutils.myutils import Mesh
# from Mesh import Mesh


vecQR = np.vectorize(np.linalg.qr, signature='(m,n)->(m,p),(p,n)')


def calculate_jacobians(neutral_mesh, vertices):
    transfer = Transfer(neutral_mesh, deepcopy(neutral_mesh))
    jacobians = []
    for v in vertices:
        current_mesh = Mesh(v, neutral_mesh.faces)
        j = transfer.deformation_gradient(current_mesh)
        jacobians.append(deepcopy(j[np.newaxis]))

    return np.concatenate(jacobians, axis=0)


class Transfer:
    def __init__(self, source: Mesh, target: Mesh, project: bool=False, area: bool=True, device='cuda:0', use_chol=False):
        self.source = source
        self.target = target
        self.do_project = project
        # self.coeffs = self.target_coef.astype(np.single)
        self.coeffs = self.target_coef # IMPORTANT: change to single precision will lose ~11 order of reconstruct accuracy (1e-14 -> 1e-3)
        self.device = int(device[-1])
        # self.coeffs = cupy.array(self.coeffs)
        self.A, self.cupy_A, self.idxs, self.vals = self.target_A
        # self.A.tocsc()
        if area:
            self.area, self.cupy_area = self.target_area
        else:
            self.area = None
        # self.coeffs = cupy.array(self.coeffs)
        # self.A = cupy.array(self.A)
        # self.A = cupy_sparse.csc_matrix(self.A)
        # self.area = cupy.array(self.area)
        if area:
            if use_chol:
                self.A = self.A.astype(np.single)
                self.area = self.area.astype(np.single)
                self.ATareaA = (self.A.T @ self.area @ self.A).tocoo()
                rows = torch.from_numpy(self.ATareaA.row).to(self.device)
                cols = torch.from_numpy(self.ATareaA.col).to(self.device)
                data = torch.from_numpy(self.ATareaA.data).to(self.device)
                print(np.linalg.matrix_rank(self.ATareaA.todense()))
                try:
                    self.solver = CholeskySolverF(self.ATareaA.shape[0], rows, cols, data, MatrixType.COO)
                except ValueError:
                    logging.warning('WARNING! Add correction matrix for Cholesky Decomposition.')
                    self.ATareaA += (sparse.identity(self.ATareaA.shape[0]) * 1e-6)
                    self.ATareaA = self.ATareaA.tocoo()
                    rows = torch.from_numpy(self.ATareaA.row).to(self.device)
                    cols = torch.from_numpy(self.ATareaA.col).to(self.device)
                    data = torch.from_numpy(self.ATareaA.data).to(self.device)
                    self.solver = CholeskySolverF(self.ATareaA.shape[0], rows, cols, data, MatrixType.COO)
            else:
                self.ATareaA = (self.A.T @ self.area @ self.A).tocsc()
                self.lu = sparse_lu(self.ATareaA)
                self.solver = cupy_SuperLU(self.lu)

            # self.ATarea = (self.A.T.astype(np.single) @ self.area.astype(np.single)).T # Later transpose back
            self.ATarea = (self.A.T @ self.area).T # Later transpose back
            self.ATarea.sort_indices()
            self.ATarea = self.ATarea.tolil()
            self.lil_rows = self.ATarea.rows
            rows = [np.repeat(row_idx, len(self.lil_rows[row_idx])) for row_idx in range(len(self.lil_rows))]
            rows = [x for y in rows for x in y]
            cols = [x for y in self.lil_rows for x in y]
            with cupy.cuda.Device(self.device):
                self.idxs = cupy.concatenate((cupy.array(rows)[:, cupy.newaxis], cupy.array(cols)[:, cupy.newaxis]), axis=1).T
                # self.vals = (self.cupy_A.T @ self.cupy_area).T.data
                # self.vals = cupy.array([x for y in self.ATarea.data for x in y], dtype=np.single)
                self.vals = cupy.array([x for y in self.ATarea.data for x in y])
        else:
            raise NotImplementedError
            self.lu = sparse_lu((self.A.T @ self.A).tocsc())

    def __call__(self, pose: Mesh, project=False):
        def _get_mass_matrix(vertices,faces,is_sparse):

            d_area = igl.doublearea(vertices,faces)
            d_area = np.hstack((d_area, d_area, d_area))
            if is_sparse:
                return sparse.csc_matrix(sparse.diags(d_area))
            return sparse.diags(d_area)
        S = self.deformation_gradient(pose)
        if project: # Project into the tangent space of the target 
            target_fv = self.get_fv(self.target)
            target_W = (target_fv[:, :, 1:] - target_fv[:, :, 0][:, :, np.newaxis]) # The basis are not formed by perpendicular unit vectors. TODO: Use igl.local_basis
            b0, b1, b_o = igl.local_basis(pose.vertices, pose.faces.astype(int))
            igl_W = np.concatenate([b0[:, :, np.newaxis], b1[:, :, np.newaxis]], axis=-1)
            # S = S @ target_W @ self.coeffs # TODO: Why multiply self.coeffs here?
            # S = S @ igl_W # TODO: Why multiply self.coeffs here?
            # S = S.reshape(-1, 2)
            # vertices = self.lu.solve(self.A.T @ self.area @ S)
            igl_cot = igl.cotmatrix(pose.vertices, pose.faces)
            igl_grad = igl.grad(pose.vertices, pose.faces)
            igl_lap = igl_grad.T @ self.area @ igl_grad
            lu_igl = sparse_lu(igl_lap)
            S = S.reshape(-1, 3)
            v1 = self.lu.solve(self.A.T @ self.area @ S)
            v2 = lu_igl.solve(igl_grad.T @ self.area @ S)
            print()
        else:
            S = S.reshape(-1, 3)
        if self.area is not None:     
            vertices = self.lu.solve(self.A.T @ self.area @ S)
        else:
            vertices = self.lu.solve(self.A.T @ S)

        return Mesh(vertices, self.target.faces)
    

    @property
    def target_coef(self):
        target_fv = self.get_fv(self.target)
        target_W = target_fv[:, :, 1:] - target_fv[:, :, 0][:, :, np.newaxis]

        q_, r_ = vecQR(target_W)
        coeffs = np.linalg.inv(r_) @ q_.transpose(0, 2, 1)
        return coeffs

    @property
    def target_A(self):
        
        target_fv = self.get_fv(self.target).astype(np.single)

        # A = sparse.lil_matrix((3 * target_fv.shape[0], len(self.target.vertices)))
        # A = cupy_sparse.csc_matrix((3 * target_fv.shape[0], len(self.target.vertices)))
        coeffs = np.concatenate((-(self.coeffs[:, 0] + self.coeffs[:, 1])[:, np.newaxis], self.coeffs), axis=1).reshape(-1)
        row_idxs = np.arange(3 * target_fv.shape[0]).reshape(-1, 3).repeat(3, axis=0).reshape(-1)
        col_idxs = np.array(self.target.faces).repeat(3)
        A = sparse.csc_matrix((coeffs, (row_idxs, col_idxs)))
        with cupy.cuda.Device(self.device):
            cupy_coeffs = cupy.array(coeffs)
            cupy_row_idxs = cupy.array(row_idxs)
            cupy_col_idxs = cupy.array(col_idxs)
            cupy_A = cupy_sparse.csc_matrix((cupy_coeffs, (cupy_row_idxs, cupy_col_idxs)))


            return A, cupy_A, cupy.concatenate((cupy_row_idxs[:, cupy.newaxis], cupy_col_idxs[:, cupy.newaxis]), axis=1).T, cupy_coeffs

    @property
    def target_area(self):
        target_fv = self.get_fv(self.target)
        AB = target_fv[:, :, 1] - target_fv[:, :, 0]
        AC = target_fv[:, :, 2] - target_fv[:, :, 0]
        area = 0.5 * np.linalg.norm(np.cross(AB, AC), axis=-1)
        area_diag = np.repeat(area[:, np.newaxis], 3, axis=-1).reshape(-1)
        area = sparse.diags(area_diag)
        with cupy.cuda.Device(self.device):
            cupy_area = cupy_sparse.diags(area_diag)
        return area, cupy_area


        

    
    @classmethod
    def get_fv(cls, mesh: Mesh):
        return mesh.vertices[mesh.faces].transpose(0, 2, 1)

    @classmethod
    def span_matrix(cls, mesh: Mesh):
        fv = cls.get_fv(mesh)

        v1 = fv[:, :, 0]
        v2 = fv[:, :, 1]
        v3 = fv[:, :, 2]
        cross = np.cross(v2 - v1, v3 - v1)
        v4 = v1 +  cross / np.sqrt(np.linalg.norm(cross, axis=-1))[:, np.newaxis]

        return np.stack((v2 - v1, v3 - v1, v4 - v1), axis=-1)
    @classmethod
    def span_to_fv(cls, span):
        cross = np.cross(span[:, :, 0], span[:, :, 1])
        v1 = span[:, :, 2] - cross / np.sqrt(np.linalg.norm(cross, axis=-1))[:, np.newaxis]
        v2 = span[:, :, 0] + v1
        v3 = span[:, :, 1] + v1
        return np.stack((v1, v2, v3), axis=-1)
    
    
    def deformation_gradient(self, pose: Mesh) -> np.ndarray:
        source_span = self.span_matrix(self.source)
        pose_span = self.span_matrix(pose)
        Q = pose_span @ np.linalg.inv(source_span)

        return Q.transpose(0, 2, 1)


class deformation_gradient(torch.autograd.Function):
    @staticmethod
    def forward(ctx, input, solver, idxs, vals, shape):
        # input: (B, F, ?) The jacobian information, shape depends on the expression format:
        # repr == 'matrix': (B, F, 3, 3)
        # repr == '6dof':   (B, F, 12)
        # repr == 'quat':   (B, F, 10)
        # repr == 'expmap': (B, F, 9)

        # input = jacobians
        # lu_solver: The solver of the deformation transfer problem
        # idxs, vals: the sparse representation of the rhs matrix
        # shape: the shape of the rhs matrix
        batch_size = input.shape[0]
        input = input.reshape(input.shape[0], -1, 3)
        input = input.transpose(1, 0)
        input = input.reshape(input.shape[0], -1) # since lu.solve only support dim (3F, D), we concate all additional dimentions to the last dim

        ctx.solver = solver
        ctx.idxs = idxs
        ctx.vals = vals
        ctx.shape = shape
        ctx.set_materialize_grads(False)
        # ctx.grad = from_dlpack(ctx.solver.solve(ctx.rhs.toarray()).toDlpack())
        # cupy_input = cupy.from_dlpack(to_dlpack(input))
        b = spmm(idxs, vals, m=shape[0], n=shape[1], matrix=input)
        b = cupy.from_dlpack(to_dlpack(b))
        cupy_output = ctx.solver.solve(b)
        output = from_dlpack(cupy_output.toDlpack())
        output = output.reshape(-1, batch_size, 3)
        output = output.transpose(0, 1)
        # print(f'forward time: {time.time() - t:.4f}s')
        return output

    @staticmethod
    def backward(ctx, grad_output):
        t = time.time()
        if grad_output is None:
            return None, None, None, None, None

    
        grad_output = grad_output.permute(1, 0, 2).reshape(grad_output.shape[1], -1)
        grad = from_dlpack(ctx.solver.solve(cupy.from_dlpack(to_dlpack(grad_output))).toDlpack())
        if grad.isnan().any():
            print(grad)
            raise ValueError('Nan found after solving for gradient!')
        # raise ValueError
        ctx.idxs, ctx.vals = transpose(ctx.idxs, ctx.vals, m=ctx.shape[0], n=ctx.shape[1])
        grad = spmm(ctx.idxs, ctx.vals, m=ctx.shape[1], n=ctx.shape[0], matrix=grad)
        if grad.isnan().any():
            print(grad)
            raise ValueError('Nan found after spmm with rhs!')
        grad = grad.reshape(grad.shape[0], -1, 3)
        grad = grad.transpose(0, 1)
        grad = grad.reshape(grad.shape[0], -1, 3, 3)
        # grad = torch.matmul(grad_output, grad)
        # grad = grad.permute(0, 2, 1).reshape(grad_output.shape[0],-1 , 3, 3)
        # print(f'Backward time: {time.time() - t:.4f}s')
        
        # Some clean up
        mempool = cupy.get_default_memory_pool()
        pinned_mempool = cupy.get_default_pinned_memory_pool()
        mempool.free_all_blocks()
        pinned_mempool.free_all_blocks()
        del ctx.solver

        return grad, None, None, None, None


class deformation_gradient_cho(torch.autograd.Function):
    @staticmethod
    def forward(ctx, input, solver, idxs, vals, shape):
        raise NotImplementedError
        # input: (B, F, ?) The jacobian information, shape depends on the expression format:
        # repr == 'matrix': (B, F, 3, 3)
        # repr == '6dof':   (B, F, 12)
        # repr == 'quat':   (B, F, 10)
        # repr == 'expmap': (B, F, 9)

        # input = jacobians
        # lu_solver: The solver of the deformation transfer problem
        # idxs, vals: the sparse representation of the rhs matrix
        # shape: the shape of the rhs matrix
        batch_size = input.shape[0]
        input = input.reshape(input.shape[0], -1, 3)
        input = input.transpose(1, 0)
        input = input.reshape(input.shape[0], -1) # since lu.solve only support dim (3F, D), we concate all additional dimentions to the last dim

        ctx.solver = solver
        ctx.idxs = idxs
        ctx.vals = vals
        ctx.shape = shape
        ctx.set_materialize_grads(False)
        # ctx.grad = from_dlpack(ctx.solver.solve(ctx.rhs.toarray()).toDlpack())
        # cupy_input = cupy.from_dlpack(to_dlpack(input))
        b = spmm(idxs, vals, m=shape[0], n=shape[1], matrix=input)
        output = torch.zeros_like(b)
        ctx.solver.solve(b, output)
        output = output.reshape(-1, batch_size, 3)
        output = output.transpose(0, 1)

        return output

    @staticmethod
    def backward(ctx, grad_output):
        t = time.time()
        if grad_output is None:
            return None, None, None, None, None

        grad_output = grad_output.permute(1, 0, 2).reshape(grad_output.shape[1], -1)
        grad = torch.zeros_like(grad_output)
        ctx.solver.solve(grad_output, grad)
        # raise ValueError
        ctx.idxs, ctx.vals = transpose(ctx.idxs, ctx.vals, m=ctx.shape[0], n=ctx.shape[1])
        grad = spmm(ctx.idxs, ctx.vals, m=ctx.shape[1], n=ctx.shape[0], matrix=grad)
        grad = grad.reshape(grad.shape[0], -1, 3)
        grad = grad.transpose(0, 1)
        grad = grad.reshape(grad.shape[0], -1, 3, 3)

        return grad, None, None, None, None

def calc_transform(mesh_x, mesh_y):
    """
    Calculate the Rotation R, Scaling S and translation t from triangle meshs mesh_x to mesh_y.
    I.e. mesh_y = R @ mesh_x + t
    The vertices of a mesh should be separated by column.
    I.e. mesh_x[i, :, j] = the jth vertex of the ith mesh.
    --------------------
    Inputs:
    ---------------------
    mesh_x (N, 3, 3):   N meshs of x
    mesh_y (N, 3, 3):   N meshs of y

    Returns:
    ----------------------
    Q   (N, 3, 3):      Rotations and scaling
    t   (N, 3, 1):      Translations

    """
    def form_V(mesh):
        if len(mesh.shape) == 2:
            mesh = mesh[np.newaxis, :, :]
        v1 = mesh[:, :, 0]
        v2 = mesh[:, :, 1]
        v3 = mesh[:, :, 2]
        cross = np.cross(v2 - v1, v3 - v1)
        v4 = v1 +  cross / np.sqrt(np.linalg.norm(cross, axis=-1))[:, np.newaxis]

        return np.stack((v2 - v1, v3 - v1, v4 - v1), axis=-1)

    
    V_x = form_V(mesh_x)
    V_y = form_V(mesh_y)

    Q = V_y @ np.linalg.inv(V_x)

    t = mesh_y - Q @ mesh_x
    assert np.allclose(t, t[:, :, 0][:, :, np.newaxis]) , "The transformation t should be the same for all three vetrices, otherwise the data order is wrong. The scripts only support trianglated mesh."

    return Q, t[:, :, 0][:, :, np.newaxis]



# def vectorized_qr(M):

def calc_qr(face_vertices):
    '''
    Return:
    --------
    q (F, 3, 2)
    r (F, 2, 2)
    '''
    W_3x2 = (face_vertices[:, :, 1:] - face_vertices[:, :, 0][:, :, np.newaxis])
    # W_3x2 -= 
    vecQR = np.vectorize(np.linalg.qr, signature='(m,n)->(m,p),(p,n)')
    q, r = vecQR(W_3x2)
    return q, r
    
def calc_S(face_vertices, Q, q, r):
    '''
    Return:
    -------
    S (F, 3, 3)
    '''
    W_3x2 = face_vertices[:, :, 1:] - face_vertices[:, :, 0][:, :, np.newaxis]
    r_inv = np.linalg.inv(r)

    return Q @ W_3x2 @ r_inv @ q.transpose(0, 2, 1)

def calc_A(face_vertices, faces_idx, num_of_vertices):
    '''
    Return:
    --------
    A (3*F, V)
    '''
    q, r = calc_qr(face_vertices) # Target
    coeffs = np.linalg.inv(r) @ q.transpose(0, 2, 1)

    # Multiply by area.
    # A = np.zeros((3 * face_vertices.shape[0], num_of_vertices))
    A = scipy.sparse.lil_matrix((3 * face_vertices.shape[0], num_of_vertices))
    for f in range(face_vertices.shape[0]):
        A[3*f:3*f+3, faces_idx[f, 1]] = coeffs[f, 0]
        A[3*f:3*f+3, faces_idx[f, 2]] = coeffs[f, 1]
        A[3*f:3*f+3, faces_idx[f, 0]] = -(coeffs[f, 0] + coeffs[f, 1])

    return A

def eliminate_constrains(A, S, vertices_idx, vertices_gt):
    '''
    A (3*F, V)
    S (F, 3, 3)
    vertices_idx
    vertices_gt
    '''
    col_list = np.arange(A.shape[1])
    col_list = np.delete(col_list, vertices_idx)

    A = A.tocsc()
    S = S.reshape((-1, 3))
    S -= A[:, vertices_idx] @ vertices_gt
    A = scipy.sparse.lil_matrix(A[:, col_list])

    # A[:, vertices_idx] = 0
    return A, S

def calc_deform(A, S, tol, use_lu=True, area=None):
    if use_lu:
        A.tocsc()
        if area is not None:
            area = np.repeat(area[:, np.newaxis], 3, axis=-1).reshape(-1)
            area = sparse.diags(area)
            lu = sparse_lu((A.T @ area @ A).tocsc())
            return lu.solve(A.T @ area @ S)
        else:
            lu = sparse_lu(A.T @ A)
            return lu.solve(A.T @ S)

    # A = np.float32(A)
    # S = np.float32(A)
    A = A.tocsr()
    # AA = A.T @ A
    # AA = AA.tocsc()
    # plt.matshow(AA.todense())
    # plt.show()
    # inv = sparse_inv(AA)
    result = np.zeros((A.shape[1], 3))
    for i in range(3):
        result[:, i] = sparse_lsqr(A, S[:, i], atol=tol, btol=tol)[0]
    return result

def calc_area(face_vertices):
    AB = face_vertices[:, :, 1] - face_vertices[:, :, 0]
    AC = face_vertices[:, :, 2] - face_vertices[:, :, 0]
    area = 0.5 * np.linalg.norm(np.cross(AB, AC), axis=-1)
    return area

def deformation_transfer(source_orig, source_deformed, target_orig, tol=1e-8, use_lu=True):
    vertices, faces = source_orig.vertices, source_orig.faces
    deform_vertices, _ = source_deformed.vertices, source_deformed.faces
    vertices_target, faces_target  = target_orig.vertices, target_orig.faces

    if np.max(faces) == vertices.shape[0]:
        faces -= 1
        faces_target -= 1
    
    orig_face_vertices = vertices[faces].transpose(0, 2, 1)
    deform_face_vertices = deform_vertices[faces].transpose(0, 2, 1)
    target_face_vertices = vertices_target[faces_target].transpose(0, 2, 1)

    Q, t = calc_transform(orig_face_vertices, deform_face_vertices)

    assert np.allclose(Q @ orig_face_vertices + t, deform_face_vertices) # Assert the transformation is correct

    area = calc_area(target_face_vertices)
    q, r = calc_qr(target_face_vertices) # Calculate the QR factorization of the W matrix
    # S = calc_S(orig_face_vertices, Q, q, r)
    S = calc_S(target_face_vertices, Q, q, r)
    # S = Q
    S = S.transpose(0, 2, 1)
    A = calc_A(target_face_vertices, faces, vertices.shape[0])
    S = S.reshape(-1, 3)
    # assert np.allclose(A @ deform_vertices, S)

    deformed_vertices = calc_deform(A, S, tol=tol, use_lu=use_lu, area=area)
    # if np.max(faces_target) == vertices_target.shape[0] - 1:
    #     faces_target += 1
    return (deformed_vertices, faces_target)
    
if __name__ == "__main__":
    
    pass
    # pass
    # vertices, faces = load_obj_file('./mh/emory_default_tri.obj')
    # vertices_target, faces_target = load_obj_file('./mh/hadley_default_tri.obj')
    # deform_vertices, _ = load_obj_file('./mh/emory_02_tri.obj')
    # face_vertices = vertices[faces].transpose(0, 2, 1)
    # deform_face_vertices = deform_vertices[faces].transpose(0, 2, 1)
    # face_vertices_target = vertices_target[faces_target].transpose(0, 2, 1)
    # Q, t = calc_transform(face_vertices, deform_face_vertices)
    
    # assert np.allclose(Q @ face_vertices + t, deform_face_vertices)
    # q, r = calc_qr(face_vertices)
    # S = calc_S(face_vertices, Q, q, r)
    # target_area = calc_area(face_vertices_target)[:, np.newaxis, np.newaxis]
    # S = S.transpose(0, 2, 1) * target_area
    # S = S.reshape(-1, 3)
    # A = calc_A(face_vertices_target, faces, vertices.shape[0]) # The face_vertices of the target mesh
    # # assert np.allclose(A @ deform_vertices, S)
    # vertices_constraint_idx = np.array([0, 1])
    # vertices_constraint_gt = deform_vertices[:2]
    # # A, S = eliminate_constrains(A, S, vertices_constraint_idx, vertices_constraint_gt)
    # deformed_vertices = calc_deform(A, S, tol=1e-8, area=target_area)
    # # for i, idx in enumerate(vertices_constraint_idx):
    # #     deformed_vertices = np.insert(deformed_vertices, idx, vertices_constraint_gt[i], axis=0)
    # # deform_vertices[vertices_constraint_idx] = vertices_constraint_gt
    # writeObj('./mh/deformed_02_area.obj', deformed_vertices, faces)


    # print()



```

## Source file: CNN.py

{
  "path": "CNN.py",
  "blob_sha": "a1b5d64d8a668a474dd609d963e054775255f4b7",
  "sha256": "f2a1a15b902390a483f52b278bd7765f0c2db05c3329d8005883eab89f48509e",
  "lines": 693,
  "bytes": 22056,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "deform": 1,
    "rig": 2
  },
  "url": "https://github.com/dafei-qin/NFR_pytorch/blob/664488e224333deb46f85279b54b6dd1d02e81b1/CNN.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__("
    },
    {
      "line": 28,
      "text": "def forward(self, avgtex, mesh, view, cams=None):"
    },
    {
      "line": 50,
      "text": "def get_mesh_branch_params(self):"
    },
    {
      "line": 54,
      "text": "def get_tex_branch_params(self):"
    },
    {
      "line": 58,
      "text": "def get_model_params(self):"
    },
    {
      "line": 65,
      "text": "def get_cc_params(self):"
    },
    {
      "line": 70,
      "text": "def __init__("
    },
    {
      "line": 92,
      "text": "def forward(self, avgtex, mesh, view, cams=None):"
    },
    {
      "line": 113,
      "text": "def get_mesh_branch_params(self):"
    },
    {
      "line": 117,
      "text": "def get_tex_branch_params(self):"
    },
    {
      "line": 121,
      "text": "def get_model_params(self):"
    },
    {
      "line": 127,
      "text": "def get_cc_params(self):"
    },
    {
      "line": 133,
      "text": "def __init__(self, grid_size=64, z_dim=128, var=0.032):"
    },
    {
      "line": 168,
      "text": "def forward(self, z, img):"
    },
    {
      "line": 204,
      "text": "def __init__("
    },
    {
      "line": 222,
      "text": "def forward(self, z, v):"
    },
    {
      "line": 231,
      "text": "def get_mesh_branch_params(self):"
    },
    {
      "line": 234,
      "text": "def get_tex_branch_params(self):"
    },
    {
      "line": 244,
      "text": "def __init__(self, inp_size=1024, mesh_inp_size=21918, n_latent=128, res=False):"
    },
    {
      "line": 257,
      "text": "def forward(self, tex, mesh):"
    },
    {
      "line": 264,
      "text": "def get_mesh_branch_params(self):"
    },
    {
      "line": 267,
      "text": "def get_tex_branch_params(self):"
    },
    {
      "line": 276,
      "text": "def __init__(self, tex_size, z_dim, res=False, non=False, bilinear=False):"
    },
    {
      "line": 303,
      "text": "def forward(self, x):"
    },
    {
      "line": 312,
      "text": "def __init__(self, res=False):"
    },
    {
      "line": 321,
      "text": "def forward(self, x):"
    },
    {
      "line": 329,
      "text": "def __init__(self, nin, nhidden, nout):"
    },
    {
      "line": 334,
      "text": "def forward(self, x):"
    },
    {
      "line": 341,
      "text": "def __init__(self, cin, chidden, cout, res=False):"
    },
    {
      "line": 351,
      "text": "def forward(self, x):"
    },
    {
      "line": 362,
      "text": "def __init__("
    },
    {
      "line": 387,
      "text": "def forward(self, x):"
    },
    {
      "line": 403,
      "text": "def __init__("
    },
    {
      "line": 431,
      "text": "def forward(self, x):"
    },
    {
      "line": 440,
      "text": "def __init__(self, n_cameras, nc=3):"
    },
    {
      "line": 448,
      "text": "def forward(self, texture, cam):"
    },
    {
      "line": 464,
      "text": "def __init__(self, n_cameras, nc=3):"
    },
    {
      "line": 472,
      "text": "def forward(self, texture, cam):"
    },
    {
      "line": 481,
      "text": "def glorot(m, alpha):"
    },
    {
      "line": 527,
      "text": "def __init__(self, in_features, out_features, bias=True):"
    },
    {
      "line": 531,
      "text": "def forward(self, input):"
    },
    {
      "line": 537,
      "text": "def __init__("
    },
    {
      "line": 563,
      "text": "def forward(self, x):"
    },
    {
      "line": 580,
      "text": "def __init__("
    },
    {
      "line": 603,
      "text": "def forward(self, x):"
    },
    {
      "line": 617,
      "text": "def __init__("
    },
    {
      "line": 643,
      "text": "def forward(self, x):"
    },
    {
      "line": 660,
      "text": "def __init__("
    },
    {
      "line": 683,
      "text": "def forward(self, x):"
    }
  ]
}

```text

# Copyright (c) Meta Platforms, Inc. and affiliates.
# All rights reserved.
# 
# This source code is licensed under the license found in the
# LICENSE file in the root directory of this source tree.

import math

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F


class WarpFieldVAE(nn.Module):
    def __init__(
        self, tex_size=1024, mesh_inp_size=21918, mode="vae", n_cameras=38, z_dim=128
    ):
        super(WarpFieldVAE, self).__init__()
        z_dim = z_dim if mode == "vae" else z_dim * 2
        grid = 64 if tex_size == 512 else 128
        self.mode = mode
        self.cc = ColorCorrection(n_cameras)
        self.enc = DeepApperanceEncoder(tex_size, mesh_inp_size, n_latent=z_dim)
        self.dec = DeepAppearanceDecoder(tex_size, mesh_inp_size, z_dim=z_dim)
        self.warp = WarpFieldDecoder(grid_size=grid, z_dim=z_dim)

    def forward(self, avgtex, mesh, view, cams=None):
        b, n, _ = mesh.shape
        mesh = mesh.view((b, -1))
        mean, logstd = self.enc(avgtex, mesh)
        mean = mean * 0.1
        logstd = logstd * 0.01
        if self.mode == "vae":
            kl = 0.5 * torch.mean(torch.exp(2 * logstd) + mean**2 - 1.0 - 2 * logstd)
            std = torch.exp(logstd)
            eps = torch.randn_like(mean)
            z = mean + std * eps
        else:
            z = torch.cat((mean, logstd), -1)
            kl = torch.tensor(0).to(z.device)

        pred_tex, pred_mesh = self.dec(z, view)
        warped_tex, warp_field = self.warp(z, pred_tex)
        pred_mesh = pred_mesh.view((b, n, 3))
        if cams is not None:
            warped_tex = self.cc(warped_tex, cams)
        return warped_tex, pred_mesh, pred_tex, warp_field, kl

    def get_mesh_branch_params(self):
        p = self.enc.get_mesh_branch_params() + self.dec.get_mesh_branch_params()
        return p

    def get_tex_branch_params(self):
        p = self.enc.get_tex_branch_params() + self.dec.get_tex_branch_params()
        return p

    def get_model_params(self):
        params = []
        params += list(self.enc.parameters())
        params += list(self.dec.parameters())
        params += list(self.warp.parameters())
        return params

    def get_cc_params(self):
        return self.cc.parameters()


class DeepAppearanceVAE(nn.Module):
    def __init__(
        self,
        tex_size=1024,
        mesh_inp_size=21918,
        mode="vae",
        n_latent=128,
        n_cams=38,
        res=False,
        non=False,
        bilinear=False,
    ):
        super(DeepAppearanceVAE, self).__init__()
        z_dim = n_latent if mode == "vae" else n_latent * 2
        self.mode = mode
        self.enc = DeepApperanceEncoder(
            tex_size, mesh_inp_size, n_latent=z_dim, res=res
        )
        self.dec = DeepAppearanceDecoder(
            tex_size, mesh_inp_size, z_dim=z_dim, res=res, non=non, bilinear=bilinear
        )
        self.cc = ColorCorrection(n_cams)

    def forward(self, avgtex, mesh, view, cams=None):
        b, n, _ = mesh.shape
        mesh = mesh.view((b, -1))
        mean, logstd = self.enc(avgtex, mesh)
        mean = mean * 0.1
        logstd = logstd * 0.01
        if self.mode == "vae":
            kl = 0.5 * torch.mean(torch.exp(2 * logstd) + mean**2 - 1.0 - 2 * logstd)
            std = torch.exp(logstd)
            eps = torch.randn_like(mean)
            z = mean + std * eps
        else:
            z = torch.cat((mean, logstd), -1)
            kl = torch.tensor(0).to(z.device)

        pred_tex, pred_mesh = self.dec(z, view)
        pred_mesh = pred_mesh.view((b, n, 3))
        if cams is not None:
            pred_tex = self.cc(pred_tex, cams)
        return pred_tex, pred_mesh, kl

    def get_mesh_branch_params(self):
        p = self.enc.get_mesh_branch_params() + self.dec.get_mesh_branch_params()
        return p

    def get_tex_branch_params(self):
        p = self.enc.get_tex_branch_params() + self.dec.get_tex_branch_params()
        return p

    def get_model_params(self):
        params = []
        params += list(self.enc.parameters())
        params += list(self.dec.parameters())
        return params

    def get_cc_params(self):
        return self.cc.parameters()


# adapted from: https://github.com/zhixinshu/DeformingAutoencoders-pytorch
class WarpFieldDecoder(nn.Module):
    def __init__(self, grid_size=64, z_dim=128, var=0.032):
        super(WarpFieldDecoder, self).__init__()
        self.mlp = nn.Sequential(
            LinearWN(z_dim, 256),
            nn.LeakyReLU(0.2, inplace=True),
            LinearWN(256, 1024),
            nn.LeakyReLU(0.2, inplace=True),
        )
        if grid_size == 64:
            self.upsample = nn.Sequential(
                ConvUpsample(256, 128, 64, 2),
                ConvUpsample(64, 32, 32, 2 * (2**2)),
                ConvTranspose2dWN(32, 2, 4, 2, 1),
            )
            self.apply(lambda x: glorot(x, 0.2))
            glorot(self.upsample[-1], 1.0)
        elif grid_size == 128:
            self.upsample = nn.Sequential(
                ConvUpsample(256, 128, 64, 2),
                ConvUpsample(64, 64, 32, 2 * (2**2)),
                ConvUpsample(32, 32, 16, 2 * (2**4), no_activ=True),
            )
            self.apply(lambda x: glorot(x, 0.2))
            glorot(self.upsample[-1].conv2, 1.0)
        self.grid_size = grid_size
        self.var = var
        self.sigmoid = nn.Sigmoid()
        self.hardtanh = nn.Hardtanh()
        self.filterx = torch.ones(
            (1, 1, 1, grid_size), dtype=torch.float32, requires_grad=False
        )
        self.filtery = torch.ones(
            (1, 1, grid_size, 1), dtype=torch.float32, requires_grad=False
        )

    def forward(self, z, img):
        g = self.grid_size
        b, c, h, w = img.shape

        # a = torch.arange(1 - g, g, 2, dtype=torch.float32) / (g - 1)
        # x = a.repeat(g, 1)
        # y = x.t()
        # id_grid = torch.cat((x.unsqueeze(0), y.unsqueeze(0)), 0)
        # id_grid = id_grid.unsqueeze(0).repeat(b, 1, 1, 1).to(z.device)

        feat = self.mlp(z).view((-1, 256, 2, 2))
        grid = self.sigmoid(self.upsample(feat))
        fullx = F.conv_transpose2d(
            grid[:, 0, :, :].unsqueeze(1),
            self.filterx.to(grid.device),
            stride=1,
            padding=0,
        )
        fully = F.conv_transpose2d(
            grid[:, 1, :, :].unsqueeze(1),
            self.filtery.to(grid.device),
            stride=1,
            padding=0,
        )
        int_grid = torch.cat((fullx[:, :, 0:g, 0:g], fully[:, :, 0:g, 0:g]), 1)
        int_grid = int_grid - torch.mean(int_grid, (2, 3), keepdim=True)
        # max_x = torch.max(int_grid[:, 0, :, :].view(b, -1), -1).values.view(b, 1, 1, 1)
        # max_y = torch.max(int_grid[:, 1, :, :].view(b, -1), -1).values.view(b, 1, 1, 1)
        # int_grid = torch.cat((int_grid[:, 0:1, :, :] / max_x, int_grid[:, 1:, :, :] / max_y), 1)
        warp = self.hardtanh(int_grid * self.var)
        warp_img = F.interpolate(warp, h, mode="bilinear").permute(0, 2, 3, 1)
        out = F.grid_sample(img, warp_img)
        return out, warp_img


class DeepAppearanceDecoder(nn.Module):
    def __init__(
        self, tex_size, mesh_size, z_dim=128, res=False, non=False, bilinear=False
    ):
        super(DeepAppearanceDecoder, self).__init__()
        nhidden = z_dim * 4 * 4 if tex_size == 1024 else z_dim * 2 * 2
        self.texture_decoder = TextureDecoder(
            tex_size, z_dim, res=res, non=non, bilinear=bilinear
        )
        self.view_fc = LinearWN(3, 8)
        self.z_fc = LinearWN(z_dim, 256)
        self.mesh_fc = LinearWN(256, mesh_size)
        self.texture_fc = LinearWN(256 + 8, nhidden)
        self.relu = nn.LeakyReLU(0.2, inplace=True)

        self.apply(lambda x: glorot(x, 0.2))
        glorot(self.mesh_fc, 1.0)
        glorot(self.texture_decoder.upsample[-1].conv2, 1.0)

    def forward(self, z, v):
        view_code = self.relu(self.view_fc(v))
        z_code = self.relu(self.z_fc(z))
        feat = torch.cat((view_code, z_code), 1)
        texture_code = self.relu(self.texture_fc(feat))
        texture = self.texture_decoder(texture_code)
        mesh = self.mesh_fc(z_code)
        return texture, mesh

    def get_mesh_branch_params(self):
        return list(self.mesh_fc.parameters())

    def get_tex_branch_params(self):
        p = []
        p += list(self.texture_decoder.parameters())
        p += list(self.view_fc.parameters())
        p += list(self.z_fc.parameters())
        p += list(self.texture_fc.parameters())
        return p


class DeepApperanceEncoder(nn.Module):
    def __init__(self, inp_size=1024, mesh_inp_size=21918, n_latent=128, res=False):
        super(DeepApperanceEncoder, self).__init__()
        self.n_latent = n_latent
        ntexture_feat = 2048 if inp_size == 1024 else 512
        self.texture_encoder = TextureEncoder(res=res) # CONV
        self.texture_fc = LinearWN(ntexture_feat, 256)
        self.mesh_fc = LinearWN(mesh_inp_size, 256)
        self.fc = LinearWN(512, n_latent * 2)
        self.relu = nn.LeakyReLU(0.2, inplace=True)

        self.apply(lambda x: glorot(x, 0.2))
        glorot(self.fc, 1.0)

    def forward(self, tex, mesh):
        tex_feat = self.relu(self.texture_fc(self.texture_encoder(tex)))
        mesh_feat = self.relu(self.mesh_fc(mesh))
        feat = torch.cat((tex_feat, mesh_feat), -1)
        latent = self.fc(feat)
        return latent[:, : self.n_latent], latent[:, self.n_latent :]

    def get_mesh_branch_params(self):
        return list(self.mesh_fc.parameters())

    def get_tex_branch_params(self):
        p = []
        p += list(self.texture_encoder.parameters())
        p += list(self.texture_fc.parameters())
        p += list(self.fc.parameters())
        return p


class TextureDecoder(nn.Module):
    def __init__(self, tex_size, z_dim, res=False, non=False, bilinear=False):
        super(TextureDecoder, self).__init__()
        base = 2 if tex_size == 512 else 4
        self.z_dim = z_dim

        self.upsample = nn.Sequential(
            ConvUpsample(
                z_dim, z_dim, 64, base, res=res, use_bilinear=bilinear, non=non
            ),
            ConvUpsample(
                64, 64, 32, base * (2**2), res=res, use_bilinear=bilinear, non=non
            ),
            ConvUpsample(
                32, 32, 16, base * (2**4), res=res, use_bilinear=bilinear, non=non
            ),
            ConvUpsample(
                16,
                16,
                3,
                base * (2**6),
                no_activ=True,
                res=res,
                use_bilinear=bilinear,
                non=non,
            ),
        )

    def forward(self, x):
        b, n = x.shape
        h = int(np.sqrt(n / self.z_dim))
        x = x.view((-1, self.z_dim, h, h))
        out = self.upsample(x)
        return out


class TextureEncoder(nn.Module):
    def __init__(self, res=False):
        super(TextureEncoder, self).__init__()
        self.downsample = nn.Sequential(
            ConvDownsample(3, 16, 16, res=res),
            ConvDownsample(16, 32, 32, res=res),
            ConvDownsample(32, 64, 64, res=res),
            ConvDownsample(64, 128, 128, res=res),
        )

    def forward(self, x):
        b, c, h, w = x.shape
        feat = self.downsample(x)
        out = feat.view((b, -1))
        return out


class MLP(nn.Module):
    def __init__(self, nin, nhidden, nout):
        self.fc1 = LinearWN(nin, nhidden)
        self.fc2 = LinearWN(nhidden, nout)
        self.relu = nn.LeakyReLU(0.2, inplace=True)

    def forward(self, x):
        h = self.relu(self.fc1(x))
        out = self.fc2(h)
        return out


class ConvDownsample(nn.Module):
    def __init__(self, cin, chidden, cout, res=False):
        super(ConvDownsample, self).__init__()
        self.conv1 = Conv2dWN(cin, chidden, 4, 2, padding=1)
        self.conv2 = Conv2dWN(chidden, cout, 4, 2, padding=1)
        self.relu = nn.LeakyReLU(0.2, inplace=True)
        self.res = res
        if res:
            self.res1 = Conv2dWN(chidden, chidden, 3, 1, 1)
            self.res2 = Conv2dWN(cout, cout, 3, 1, 1)

    def forward(self, x):
        h = self.relu(self.conv1(x))
        if self.res:
            h = self.relu(self.res1(h) + h)
        h = self.relu(self.conv2(h))
        if self.res:
            h = self.relu(self.res2(h) + h)
        return h


class ConvUpsample(nn.Module):
    def __init__(
        self,
        cin,
        chidden,
        cout,
        feature_size,
        no_activ=False,
        res=False,
        use_bilinear=False,
        non=False,
    ):
        super(ConvUpsample, self).__init__()
        self.conv1 = DeconvTexelBias(
            cin, chidden, feature_size * 2, use_bilinear=use_bilinear, non=non
        )
        self.conv2 = DeconvTexelBias(
            chidden, cout, feature_size * 4, use_bilinear=use_bilinear, non=non
        )
        self.relu = nn.LeakyReLU(0.2, inplace=True)
        self.no_activ = no_activ
        self.res = res
        if self.res:
            self.res1 = Conv2dWN(chidden, chidden, 3, 1, 1)
            self.res2 = Conv2dWN(cout, cout, 3, 1, 1)

    def forward(self, x):
        h = self.relu(self.conv1(x))
        if self.res:
            h = self.relu(self.res1(h) + h)
        if self.no_activ:
            h = self.conv2(h)
            if self.res:
                h = self.res2(h) + h
        else:
            h = self.relu(self.conv2(h))
            if self.res:
                h = self.relu(self.res2(h) + h)
        return h


class DeconvTexelBias(nn.Module):
    def __init__(
        self,
        cin,
        cout,
        feature_size,
        ksize=4,
        stride=2,
        padding=1,
        use_bilinear=False,
        non=False,
    ):
        super(DeconvTexelBias, self).__init__()
        if isinstance(feature_size, int):
            feature_size = (feature_size, feature_size)
        self.use_bilinear = use_bilinear
        if use_bilinear:
            self.deconv = Conv2dWN(cin, cout, 3, 1, 1, bias=False)
        else:
            self.deconv = ConvTranspose2dWN(
                cin, cout, ksize, stride, padding, bias=False
            )
        if non:
            self.bias = nn.Parameter(torch.zeros(1, cout, 1, 1))
        else:
            self.bias = nn.Parameter(
                torch.zeros(1, cout, feature_size[0], feature_size[1])
            )

    def forward(self, x):
        if self.use_bilinear:
            x = F.interpolate(x, scale_factor=2)
        out = self.deconv(x) + self.bias
        return out


"""
class ColorCorrection(nn.Module):
    def __init__(self, n_cameras, nc=3):
        super(ColorCorrection, self).__init__()
        weights = torch.zeros(n_cameras, nc, nc, 1, 1)
        for i in range(nc):
            weights[:, i, i] = 1
        self.weights = nn.Parameter(weights)
        self.bias = nn.Parameter(torch.zeros(n_cameras, nc))
        self.anchor = 0
    def forward(self, texture, cam):
        b, c, h, w = texture.shape
        texture = texture.view(1, b*c, h, w)
        weight = self.weights[cam]
        bias = self.bias[cam]
        if self.training:
            weight[cam == self.anchor] = torch.eye(3).view(3, 3, 1, 1).to(texture.device)
            bias[cam == self.anchor] = 0
        weight = weight.view(b*c, 3, 1, 1)
        bias = bias.view(b*c)
        out = F.conv2d(texture, weight, bias, groups=b).view(b, c, h, w)
        return out
"""


class ColorCorrection(nn.Module):
    def __init__(self, n_cameras, nc=3):
        super(ColorCorrection, self).__init__()
        # anchors the 0th camera
        self.weight_anchor = nn.Parameter(torch.ones(1, nc, 1, 1), requires_grad=False)
        self.bias_anchor = nn.Parameter(torch.zeros(1, 3, 1, 1), requires_grad=False)
        self.weight = nn.Parameter(torch.ones(n_cameras - 1, 3, 1, 1))
        self.bias = nn.Parameter(torch.zeros(n_cameras - 1, 3, 1, 1))

    def forward(self, texture, cam):
        weights = torch.cat([self.weight_anchor, self.weight], dim=0)
        biases = torch.cat([self.bias_anchor, self.bias], dim=0)
        w = weights[cam]
        b = biases[cam]
        output = texture * w + b
        return output


def glorot(m, alpha):
    gain = math.sqrt(2.0 / (1.0 + alpha**2))

    if isinstance(m, nn.Conv2d):
        ksize = m.kernel_size[0] * m.kernel_size[1]
        n1 = m.in_channels
        n2 = m.out_channels

        std = gain * math.sqrt(2.0 / ((n1 + n2) * ksize))
    elif isinstance(m, nn.ConvTranspose2d):
        ksize = m.kernel_size[0] * m.kernel_size[1] // 4
        n1 = m.in_channels
        n2 = m.out_channels

        std = gain * math.sqrt(2.0 / ((n1 + n2) * ksize))
    elif isinstance(m, nn.Linear):
        n1 = m.in_features
        n2 = m.out_features

        std = gain * math.sqrt(2.0 / (n1 + n2))
    else:
        return

    # m.weight.data.normal_(0, std)
    m.weight.data.uniform_(-std * math.sqrt(3.0), std * math.sqrt(3.0))
    m.bias.data.zero_()

    if isinstance(m, nn.ConvTranspose2d):
        # hardcoded for stride=2 for now
        m.weight.data[:, :, 0::2, 1::2] = m.weight.data[:, :, 0::2, 0::2]
        m.weight.data[:, :, 1::2, 0::2] = m.weight.data[:, :, 0::2, 0::2]
        m.weight.data[:, :, 1::2, 1::2] = m.weight.data[:, :, 0::2, 0::2]

    # if isinstance(m, Conv2dWNUB) or isinstance(m, ConvTranspose2dWNUB) or isinstance(m, LinearWN):
    if (
        isinstance(m, Conv2dWNUB)
        or isinstance(m, Conv2dWN)
        or isinstance(m, ConvTranspose2dWN)
        or isinstance(m, ConvTranspose2dWNUB)
        or isinstance(m, LinearWN)
    ):
        norm = np.sqrt(torch.sum(m.weight.data[:] ** 2))
        m.g.data[:] = norm


class LinearWN(nn.Linear):
    def __init__(self, in_features, out_features, bias=True):
        super(LinearWN, self).__init__(in_features, out_features, bias)
        self.g = nn.Parameter(torch.ones(out_features))

    def forward(self, input):
        wnorm = torch.sqrt(torch.sum(self.weight**2))
        return F.linear(input, self.weight * self.g[:, None] / wnorm, self.bias)


class Conv2dWNUB(nn.Conv2d):
    def __init__(
        self,
        in_channels,
        out_channels,
        height,
        width,
        kernel_size,
        stride=1,
        padding=0,
        dilation=1,
        groups=1,
        bias=False,
    ):
        super(Conv2dWNUB, self).__init__(
            in_channels,
            out_channels,
            kernel_size,
            stride,
            padding,
            dilation,
            groups,
            False,
        )
        self.g = nn.Parameter(torch.ones(out_channels))
        self.bias = nn.Parameter(torch.zeros(out_channels, height, width))

    def forward(self, x):
        wnorm = torch.sqrt(torch.sum(self.weight**2))
        return (
            F.conv2d(
                x,
                self.weight * self.g[:, None, None, None] / wnorm,
                bias=None,
                stride=self.stride,
                padding=self.padding,
                dilation=self.dilation,
                groups=self.groups,
            )
            + self.bias[None, ...]
        )


class Conv2dWN(nn.Conv2d):
    def __init__(
        self,
        in_channels,
        out_channels,
        kernel_size,
        stride=1,
        padding=0,
        dilation=1,
        groups=1,
        bias=True,
    ):
        super(Conv2dWN, self).__init__(
            in_channels,
            out_channels,
            kernel_size,
            stride,
            padding,
            dilation,
            groups,
            True,
        )
        self.g = nn.Parameter(torch.ones(out_channels))

    def forward(self, x):
        wnorm = torch.sqrt(torch.sum(self.weight**2))
        return F.conv2d(
            x,
            self.weight * self.g[:, None, None, None] / wnorm,
            bias=self.bias,
            stride=self.stride,
            padding=self.padding,
            dilation=self.dilation,
            groups=self.groups,
        )


class ConvTranspose2dWNUB(nn.ConvTranspose2d):
    def __init__(
        self,
        in_channels,
        out_channels,
        height,
        width,
        kernel_size,
        stride=1,
        padding=0,
        dilation=1,
        groups=1,
        bias=False,
    ):
        super(ConvTranspose2dWNUB, self).__init__(
            in_channels,
            out_channels,
            kernel_size,
            stride,
            padding,
            dilation,
            groups,
            False,
        )
        self.g = nn.Parameter(torch.ones(out_channels))
        self.bias = nn.Parameter(torch.zeros(out_channels, height, width))

    def forward(self, x):
        wnorm = torch.sqrt(torch.sum(self.weight**2))
        return (
            F.conv_transpose2d(
                x,
                self.weight * self.g[None, :, None, None] / wnorm,
                bias=None,
                stride=self.stride,
                padding=self.padding,
                dilation=self.dilation,
                groups=self.groups,
            )
            + self.bias[None, ...]
        )


class ConvTranspose2dWN(nn.ConvTranspose2d):
    def __init__(
        self,
        in_channels,
        out_channels,
        kernel_size,
        stride=1,
        padding=0,
        dilation=1,
        groups=1,
        bias=True,
    ):
        super(ConvTranspose2dWN, self).__init__(
            in_channels,
            out_channels,
            kernel_size,
            stride,
            padding,
            dilation,
            groups,
            True,
        )
        self.g = nn.Parameter(torch.ones(out_channels))

    def forward(self, x):
        wnorm = torch.sqrt(torch.sum(self.weight**2))
        return F.conv_transpose2d(
            x,
            self.weight * self.g[None, :, None, None] / wnorm,
            bias=self.bias,
            stride=self.stride,
            padding=self.padding,
            dilation=self.dilation,
            groups=self.groups,
        )

```

## Source file: UNET.py

{
  "path": "UNET.py",
  "blob_sha": "a77f110b55c0ba9d541c2a5bb19e3dc3d9fdcf24",
  "sha256": "6c3f261d9cf0c1c98758b9f6724fcbd8ac0e8fa308c68f95b7db3b411a0da476",
  "lines": 143,
  "bytes": 5540,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/dafei-qin/NFR_pytorch/blob/664488e224333deb46f85279b54b6dd1d02e81b1/UNET.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, n_channels, n_classes, bilinear=False, norm='bn'):"
    },
    {
      "line": 38,
      "text": "def forward(self, x):"
    },
    {
      "line": 67,
      "text": "def __init__(self, in_channels, out_channels, mid_channels=None, norm='bn'):"
    },
    {
      "line": 90,
      "text": "def forward(self, x):"
    },
    {
      "line": 97,
      "text": "def __init__(self, in_channels, out_channels, norm='bn'):"
    },
    {
      "line": 104,
      "text": "def forward(self, x):"
    },
    {
      "line": 111,
      "text": "def __init__(self, in_channels, out_channels, bilinear=True, norm='bn'):"
    },
    {
      "line": 122,
      "text": "def forward(self, x1, x2):"
    },
    {
      "line": 138,
      "text": "def __init__(self, in_channels, out_channels):"
    },
    {
      "line": 142,
      "text": "def forward(self, x):"
    }
  ]
}

```text

""" Parts of the U-Net model """

import torch
import torch.nn as nn
import torch.nn.functional as F


class UNet(nn.Module):
    def __init__(self, n_channels, n_classes, bilinear=False, norm='bn'):
        super(UNet, self).__init__()
        self.n_channels = n_channels
        self.n_classes = n_classes
        self.bilinear = bilinear

        self.inc = (DoubleConv(n_channels, 16, norm=norm))
        self.down1 = (Down(16, 32, norm=norm))
        self.down2 = (Down(32, 64, norm=norm))
        self.down3 = (Down(64, 128, norm=norm))
        factor = 2 if bilinear else 1
        self.down4 = (Down(512, 1024 // factor, norm=norm))
        self.up1 = (Up(1024, 512 // factor, bilinear, norm=norm))
        self.up2 = (Up(512, 256 // factor, bilinear, norm=norm))
        self.up3 = (Up(256, 128 // factor, bilinear, norm=norm))
        self.up4 = (Up(128, 64, bilinear, norm=norm))
        self.outc = (OutConv(64, n_classes))
        # self.inc = (DoubleConv(n_channels, 16, norm=norm))
        # self.down1 = (Down(16, 32, norm=norm))
        # self.down2 = (Down(32, 64, norm=norm))
        # self.down3 = (Down(64, 128, norm=norm))
        # factor = 2 if bilinear else 1
        # self.down4 = (Down(128, 256 // factor, norm=norm))
        # self.up1 = (Up(256, 128 // factor, bilinear, norm=norm))
        # self.up2 = (Up(128, 64 // factor, bilinear, norm=norm))
        # self.up3 = (Up(64, 32 // factor, bilinear, norm=norm))
        # self.up4 = (Up(32, 16, bilinear, norm=norm))
        # self.outc = (OutConv(16, n_classes))

    def forward(self, x):
        x1 = self.inc(x)
        x2 = self.down1(x1)
        x3 = self.down2(x2)
        x4 = self.down3(x3)
        x5 = self.down4(x4)
        x = self.up1(x5, x4)
        x = self.up2(x, x3)
        x = self.up3(x, x2)
        x = self.up4(x, x1)
        logits = self.outc(x)
        return logits, x5

    # def use_checkpointing(self):
    #     self.inc = torch.utils.checkpoint(self.inc)
    #     self.down1 = torch.utils.checkpoint(self.down1)
    #     self.down2 = torch.utils.checkpoint(self.down2)
    #     self.down3 = torch.utils.checkpoint(self.down3)
    #     self.down4 = torch.utils.checkpoint(self.down4)
    #     self.up1 = torch.utils.checkpoint(self.up1)
    #     self.up2 = torch.utils.checkpoint(self.up2)
    #     self.up3 = torch.utils.checkpoint(self.up3)
    #     self.up4 = torch.utils.checkpoint(self.up4)
    #     self.outc = torch.utils.checkpoint(self.outc)


class DoubleConv(nn.Module):
    """(convolution => [BN] => ReLU) * 2"""

    def __init__(self, in_channels, out_channels, mid_channels=None, norm='bn'):
        super().__init__()
        if not mid_channels:
            mid_channels = out_channels
        if norm == 'bn':
            self.double_conv = nn.Sequential(
                nn.Conv2d(in_channels, mid_channels, kernel_size=3, padding=1, bias=False),
                nn.BatchNorm2d(mid_channels),
                nn.ReLU(inplace=True),
                nn.Conv2d(mid_channels, out_channels, kernel_size=3, padding=1, bias=False),
                nn.BatchNorm2d(out_channels),
                nn.ReLU(inplace=True)
            )
        elif norm == 'gn':
            self.double_conv = nn.Sequential(
                nn.Conv2d(in_channels, mid_channels, kernel_size=3, padding=1, bias=False),
                nn.GroupNorm(4, mid_channels),
                nn.ReLU(inplace=True),
                nn.Conv2d(mid_channels, out_channels, kernel_size=3, padding=1, bias=False),
                nn.GroupNorm(4, out_channels),
                nn.ReLU(inplace=True)
            )

    def forward(self, x):
        return self.double_conv(x)


class Down(nn.Module):
    """Downscaling with maxpool then double conv"""

    def __init__(self, in_channels, out_channels, norm='bn'):
        super().__init__()
        self.maxpool_conv = nn.Sequential(
            nn.MaxPool2d(2),
            DoubleConv(in_channels, out_channels, norm=norm)
        )

    def forward(self, x):
        return self.maxpool_conv(x)


class Up(nn.Module):
    """Upscaling then double conv"""

    def __init__(self, in_channels, out_channels, bilinear=True, norm='bn'):
        super().__init__()

        # if bilinear, use the normal convolutions to reduce the number of channels
        if bilinear:
            self.up = nn.Upsample(scale_factor=2, mode='bilinear', align_corners=True)
            self.conv = DoubleConv(in_channels, out_channels, in_channels // 2, norm=norm)
        else:
            self.up = nn.ConvTranspose2d(in_channels, in_channels // 2, kernel_size=2, stride=2)
            self.conv = DoubleConv(in_channels, out_channels, norm=norm)

    def forward(self, x1, x2):
        x1 = self.up(x1)
        # input is CHW
        diffY = x2.size()[2] - x1.size()[2]
        diffX = x2.size()[3] - x1.size()[3]

        x1 = F.pad(x1, [diffX // 2, diffX - diffX // 2,
                        diffY // 2, diffY - diffY // 2])
        # if you have padding issues, see
        # https://github.com/HaiyongJiang/U-Net-Pytorch-Unstructured-Buggy/commit/0e854509c2cea854e247a9c615f175f76fbb2e3a
        # https://github.com/xiaopeng-liao/Pytorch-UNet/commit/8ebac70e633bac59fc22bb5195e513d5832fb3bd
        x = torch.cat([x2, x1], dim=1)
        return self.conv(x)


class OutConv(nn.Module):
    def __init__(self, in_channels, out_channels):
        super(OutConv, self).__init__()
        self.conv = nn.Conv2d(in_channels, out_channels, kernel_size=1)

    def forward(self, x):
        return self.conv(x)

```

## Source file: FWH_run.py

{
  "path": "FWH_run.py",
  "blob_sha": "a6414249c3f7d603eb2ac905e70614bbb6952b93",
  "sha256": "619c6d674d43b2eeb4d3c821113950d4d67260a0306e04c2b63581526369c5ed",
  "lines": 1163,
  "bytes": 82573,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 5,
    "deform": 4,
    "facs": 15,
    "face": 137
  },
  "url": "https://github.com/dafei-qin/NFR_pytorch/blob/664488e224333deb46f85279b54b6dd1d02e81b1/FWH_run.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def calc_loss(idx, v=None, v_gt=None, j=None, j_gt=None, mu=None, log_var=None, latent=None, latent_shape=128, latent_type='facs', reduce=True, v_factor=100, j_factor=1, kld_factor=5e-4, mask=None, latent_const=None, iden_vec=None, latent_f"
    },
    {
      "line": 141,
      "text": "def eval(model, e, face, val_loaders, dfn_info, transfs, args, normalizer, write=True, shift=None, device='cuda:0', head='val', fraction=20, mask=None, latent_const=None, iden_vec=None, exp_weights=None):"
    },
    {
      "line": 421,
      "text": "def train(model, train_loaders, val_loaders, dfn_info, transfs, face, args, normalizer, transf_shift, latent_const=None, latent_const_val=None, iden_vec=None, exp_weights = None, epoch=None, head='train', test_loaders=None, fix_mlp=False):"
    }
  ]
}

```text


import logging
import os
import sys
from copy import deepcopy
from collections import defaultdict
import warnings
import shutil

import numpy as np

import torch
from torch import nn
from torch.utils.dlpack import from_dlpack
from cupyx.scipy.sparse.linalg import SuperLU
from torch_sparse import coalesce, transpose
from tensorboardX import SummaryWriter
import cupy

sys.path.append('./third_party/diffusion-net/src')
import diffusion_net
from deformation_transfer import Mesh, Transfer, deformation_gradient, deformation_gradient_cho
from mymodel import df_net, latent_space

import time
from FWH_args import train_parser
from data_loader import load_data_ict_all, load_data_ict_random, load_data_ict_live, load_data_mf_v3,load_data_ict_argument
from myutils.myutils import reconstruct_jacobians, positional_encoder, Normalizer, load_state_dict, Normalizer_landmark, Normalizer_img, calc_norm_torch
from datasets import fake_dataset

def calc_loss(idx, v=None, v_gt=None, j=None, j_gt=None, mu=None, log_var=None, latent=None, latent_shape=128, latent_type='facs', reduce=True, v_factor=100, j_factor=1, kld_factor=5e-4, mask=None, latent_const=None, iden_vec=None, latent_factor=2e-2, zero_loss=None, zero_activation=None, global_weights=None, norm=None, norm_gt=None, norm_factor=1, latent_reg_factor=0.01):
    if type(idx) == int:
        idx = [idx]
    if global_weights is None:
        global_weights = torch.ones(len(idx)).unsqueeze(-1).unsqueeze(-1).to(args.device)

    else:
        global_weights = global_weights[idx].unsqueeze(-1).unsqueeze(-1).to(args.device)


    Loss = nn.MSELoss(reduction='none')
    Loss_l1 = nn.L1Loss(reduction='none')
    loss = {'total_loss': torch.tensor(0).to(args.device)}

    if v is not None:
        if mask is not None:
            mask = mask.unsqueeze(0).unsqueeze(-1)
            v = v * mask
            v_gt = v_gt * mask
        vloss = Loss(v, v_gt) * global_weights
        vloss = torch.sqrt(vloss.sum(dim=-1) + 1e-6).mean(dim=1)
        loss['vloss'] = vloss * 100 # for display 
        loss['total_loss'] = loss['total_loss'] + v_factor * vloss

    if j is not None and j_factor != 0:
        jloss = Loss(j, j_gt).view(j.shape[0], j.shape[1],-1)* global_weights
        jloss = jloss.mean(dim=(1, 2))
        loss['jloss'] = jloss
        loss['total_loss'] = loss['total_loss'] + jloss * j_factor

    if norm is not None and norm_factor != 0:
        nloss = Loss(norm, norm_gt) * global_weights
        nloss = nloss.mean(dim=(1, 2))
        loss['nloss'] = nloss
        loss['total_loss'] = loss['total_loss'] + nloss * norm_factor
    if mu is not None:
        kld_loss = torch.mean(-0.5 * torch.sum(1 + log_var - mu ** 2 - log_var.exp(), dim=1))
        loss['kld'] = kld_loss
        loss['total_loss'] += kld_factor * kld_loss

    assert latent_type in ['facs', 'extend']

    if latent_type == 'facs': # Only 53 dimensional facs expression vector
        if type(latent_const) == str or latent_const is None: # Unsupervised
            latent_exp = latent[:, -53:].clone()
            mask_below_zero = latent_exp < 0
            mask_greater_one = latent_exp > 1
            mask_good = ~mask_below_zero * ~mask_greater_one
            latent_loss = latent_exp
            latent_loss[mask_good] = 0
            latent_loss[mask_greater_one] -= 1
            latent_loss[mask_below_zero] *= -1
            latent_loss = latent_loss.mean(dim=-1)
        else:
            # Supervised
            latent_const_ = latent_const[idx].to(latent.device)
            latent_exp = latent[:, -53:]
            latent_loss = Loss(latent_exp, latent_const_).sum(dim=1)

    elif latent_type == 'extend': # Add to 128 dim to allow details
        if latent_const == 'restricted': # Only allow FACS units
            latent_facs = latent[:, -latent_shape: -latent_shape + 53].clone()
            latent_extended = latent[:, -latent_shape+53:]
            mask_below_zero = latent_facs < 0
            mask_greater_one = latent_facs > 1
            mask_good = ~mask_below_zero * ~mask_greater_one
            latent_loss = latent_facs
            latent_loss[mask_good] = 0
            latent_loss[mask_greater_one] -= 1
            latent_loss[mask_below_zero] *= -1
            latent_loss = latent_loss.mean(dim=-1) + Loss(latent_extended, torch.zeros_like(latent_extended).to(latent_extended.device)).mean(dim=-1) # Keep the FACS units in [0, 1] and let the extended terms be 0

        elif latent_const == 'free': # Fully unsupervised
            latent_facs = latent[:, -latent_shape:-latent_shape + 53].clone()
            latent_extended = latent[:, -latent_shape + 53:]
            mask_below_zero = latent_facs < 0
            mask_greater_one = latent_facs > 1
            mask_good = ~mask_below_zero * ~mask_greater_one
            latent_loss = latent_facs
            latent_loss[mask_good] = 0
            latent_loss[mask_greater_one] -= 1
            latent_loss[mask_below_zero] *= -1
            latent_loss = latent_loss.mean(dim=-1) + latent_reg_factor * torch.norm(latent_extended, p=1, dim=-1) # Keep the FACS units in [0, 1] and add l1 regularization to the extended terms

        else: # Supervised
            latent_const_ = latent_const[idx].to(latent.device)
            latent_exp = latent[:, -latent_shape: -latent_shape + 53] # The FACS unit
            latent_extended = latent[:, -latent_shape + 53:] # The extended unit
            latent_loss = Loss(latent_exp, latent_const_).sum(dim=1) + Loss(latent_extended, torch.zeros_like(latent_extended).to(latent_extended.device)).sum(dim=1)

    loss['latent'] = latent_loss
    loss['total_loss'] += latent_factor * latent_loss

    if zero_loss is not None:

        zero_loss = Loss(zero_activation, torch.empty_like(zero_activation)).mean(dim=1)
        loss['zero'] = zero_loss
        loss['total_loss'] += zero_loss

    with torch.no_grad():
        l1_loss = Loss_l1(v, v_gt)  * global_weights 
        l1_loss = l1_loss.mean(dim=(1, 2))
        loss['l1'] =  l1_loss

    if reduce:
        for key in loss.keys():
            loss[key] = loss[key].mean()

    return loss

def eval(model, e, face, val_loaders, dfn_info, transfs, args, normalizer, write=True, shift=None, device='cuda:0', head='val', fraction=20, mask=None, latent_const=None, iden_vec=None, exp_weights=None):
    global writer
    t__ = time.time() # Overall time

    dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]
    if isinstance(model, nn.DataParallel):
        model.module.update_precomputes(dfn_info)
    else:
        model.update_precomputes(dfn_info)

    if exp_weights is not None and type(exp_weights) == np.ndarray:
        exp_weights = torch.from_numpy(exp_weights)
    if exp_weights is not None:
        exp_weights = exp_weights.to(args.device)
    # if latent_const is not None and type(latent_const) == np.ndarray:
    #     latent_const = torch.from_numpy(latent_const).float()


    # model.eval()
    Loss = nn.MSELoss(reduction='none')
    e = int(e)
    if write:
        if os.path.exists(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}')):
            logging.warning(f"Store directory {os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}')} exists, deleting...")
            shutil.rmtree(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}'))
        
        os.makedirs(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}'), exist_ok=False)

    total_loss = defaultdict(list)
    counter = 0
    t = time.time()
    myfunc = deformation_gradient.apply
    if 'test' in head:
        l2_distance = []
    with torch.no_grad():
        # t_all = np.array([0, 0, 0, 0])
        for out_idx, train_loader in enumerate(val_loaders):
            input = train_loader.dataset.inputs_source_f.to(args.device).unsqueeze(0)
            input_source_v = train_loader.dataset.inputs_source_v.to(args.device).unsqueeze(0)
            face = train_loader.dataset.face
            
            # Randomize dfn operators for DN_e
            if train_loader.dataset.dfn_info_list is not None and args.global_encoder == 'dfn':
                dfn_info = train_loader.dataset.dfn_info_list[torch.randint(len(train_loader.dataset.dfn_info_list), (1,))]
                dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]

            #     if len(args.parallel_ids) > 0:
            #         model.module.encoder.update_precomputes(dfn_info)
            #     else:
            #         model.encoder.update_precomputes(dfn_info)

            # Update dfn operatores for DN
            if train_loader.dataset.dfn_info is not None and args.iden_encoder == 'dfn':
                dfn_info = train_loader.dataset.dfn_info
                # face = train_loader.dataset.face
                dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]
                if len(args.parallel_ids) > 0:
                    model.module.global_pn.update_precomputes(dfn_info)
                else:
                    model.global_pn.update_precomputes(dfn_info)
                if not train_loader.dataset.dfn_info_list:
                    if len(args.parallel_ids) > 0:
                        model.module.encoder.update_precomputes(dfn_info)
                    else:
                        model.encoder.update_precomputes(dfn_info)
            
            if train_loader.dataset.dfn_info is not None:
                dfn_info = train_loader.dataset.dfn_info
                face = train_loader.dataset.face
                dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]
                if len(args.parallel_ids) > 0:
                    model.module.update_precomputes(dfn_info)
                else:
                    model.update_precomputes(dfn_info)
                    
            if args.img:
                neutral_img = train_loader.dataset.neutral_img.to(args.device).unsqueeze(0)
                if args.pix2face:
                    neutral_pix2face_indices = train_loader.dataset.neutral_pix2face_indices.to(args.device).unsqueeze(0)
                    neutral_pix2face_values = train_loader.dataset.neutral_pix2face_values.to(args.device).unsqueeze(0)
                    face2vertex_indices = train_loader.dataset.face2vertex_indices.to(args.device).unsqueeze(0)
                    face2vertex_values = train_loader.dataset.face2vertex_values.to(args.device).unsqueeze(0)
                else:
                    neutral_pix2face_indices = None
                    neutral_pix2face_values = None
                    face2vertex_indices = None
                    face2vertex_values = None
            else:
                neutral_img, neutral_pix2face_indices, neutral_pix2face_values, face2vertex_indices, face2vertex_values = None, None, None, None, None
            if len(args.parallel_ids) > 0:
                input_source_v = input_source_v.expand(len(args.parallel_ids), -1, -1)
                input = input.expand(len(args.parallel_ids), -1, -1)
                if args.img:
                    neutral_img = neutral_img.expand(len(args.parallel_ids), -1, -1, -1)
                    if args.pix2face:
                        neutral_pix2face_indices = neutral_pix2face_indices.expand(len(args.parallel_ids), -1, -1)
                        neutral_pix2face_values = neutral_pix2face_values.expand(len(args.parallel_ids), -1)
                        
                        face2vertex_indices = face2vertex_indices.expand(len(args.parallel_ids), -1, -1)
                        face2vertex_values = face2vertex_values.expand(len(args.parallel_ids), -1)
                    else:
                        neutral_pix2face_indices, neutral_pix2face_values, face2vertex_indices, face2vertex_values = None, None, None, None
            loader_fake = torch.utils.data.DataLoader(fake_dataset(len(train_loader.dataset)), batch_size=args.batch_size, shuffle=True, num_workers=0, drop_last=False)

            for idx in loader_fake:
                input_v = train_loader.dataset.inputs_target_v[idx].to(args.device)
                g = train_loader.dataset.gradients[idx].to(args.device)
                out_gt = train_loader.dataset.verts[idx].to(args.device)
                norm_gt = train_loader.dataset.norms[idx].to(args.device)
                if args.img:
                    imgs = train_loader.dataset.img[idx].to(args.device)

                    if args.pix2face:
                        pix2face_idxs = train_loader.dataset.pix2face_indices[idx]
                        pix2face_vals = train_loader.dataset.pix2face_values[idx]
                    else:
                        pix2face_idxs = None
                        pix2face_vals = None

                else:
                    imgs = None
                    pix2face_idxs = None
                    pix2face_vals = None
                    
                if train_loader.dataset.gradX is not None:
                    gradX_batch = train_loader.dataset.gradX[idx].to(args.device)
                    gradX_batch.requires_grad = False
                    gradY_batch = train_loader.dataset.gradY[idx].to(args.device)
                    gradY_batch.requires_grad = False
                else:
                    gradX_batch = None
                    gradY_batch = None
                counter +=1
                if counter % fraction:
                    continue
                if type(input) == list:
                    N_FACE = input[0].shape[1]
                else:
                    N_FACE = input.shape[1]
                N_VERTEX = out_gt.shape[1]

                # if len(input.shape) == 4:
                #     input = input[0].to(device)
                #     g = g.squeeze(0).to(device)
                #     out_gt = out_gt[0].to(device)


                # if len(input_v.shape) == 4:
                #         input_v = input_v[0].to(device)
                # else:
                #         input_v = input_v.to(device)

                if not args.ab_vertex:
                    g = normalizer.inv_normalize(g)

                    out_gt = out_gt - out_gt.mean(axis=1, keepdim=True)
                    if 'test' in head:
                        write_idx = shift['test'][out_idx]
                    elif 'val' in head:
                        write_idx = shift['val'][out_idx]
                    elif 'train' in head:
                        write_idx = shift['train'][out_idx]
                    else:
                        logging.error(f'Evaluation must provide a head to contain [train, val, test] to identify the transfs_shift, but f{head} is provided.')
                        raise ValueError

                    transf = transfs[write_idx]

                    solver = transf.solver
                    idxs, vals = coalesce(from_dlpack(transf.idxs.toDlpack()).long(), from_dlpack(transf.vals.toDlpack()), m=N_FACE *3, n=N_VERTEX)
                    idxs, vals = transpose(idxs, vals, m=N_FACE *3, n=N_VERTEX)

                    g = reconstruct_jacobians(g)

                    out_gt_g = myfunc(g, solver, idxs, vals, (N_VERTEX, N_FACE * 3))
                    out_gt_g = out_gt_g - out_gt_g.mean(axis=1, keepdim=True)
                    out_gt = out_gt.to(device)

                    try:
                        assert Loss(out_gt, out_gt_g).mean() < 1e-8
                    except AssertionError:
                        logging.error('Assertion Error! Reconstruction of ground truth jacobians have big errors, please check transf')
                        if not os.path.exists(os.path.join(args.save_dir, args.save_head+f'{e:04d}_error')):
                            os.makedirs(os.path.join(args.save_dir, args.save_head+f'{e:04d}_error'))
                        for exp_idx in range(out_gt.shape[0]):
                            mesh_out = Mesh(out_gt[exp_idx].detach().cpu().numpy(), face)
                            mesh_out.write(os.path.join(args.save_dir, args.save_head+f'{e:04d}_error', f"{write_idx:03d}_{exp_idx:02d}_gt.obj"))
                            mesh_out = Mesh(out_gt_g[exp_idx].detach().cpu().numpy(), face)
                            mesh_out.write(os.path.join(args.save_dir, args.save_head+f'{e:04d}_error', f"{write_idx:03d}_{exp_idx:02d}.obj"))
                        exit(-1)
                
                # Do some argumentation & add noises

                if args.shift_noise:
                    with torch.no_grad():
                        input[:, :, :3] = input[:, :, :3] + torch.randn(3).to(input.device) * 0.1
                if args.teacher_forcing:
                    if latent_const is None or type(latent_const) == str:
                        logging.error('Teacher Forcing requires gt expression vectors')
                        raise ValueError
                    latent_t_f = latent_const[idx].to(input_v.device)
                    if args.teacher_forcing_noise:
                        latent_t_f += (torch.randn(latent_t_f.shape, device=latent_t_f.device) * 1e-3)
                    if args.latent_type == 'extend':
                        latent_t_f = torch.cat([latent_t_f, torch.zeros((latent_t_f.shape[0], args.feat_dim - 53)).to(latent_t_f.device)], dim=-1)

                    # g_pred, mu, log_var, latent, t = model(input_v, input, latent=latent_t_f)
                    g_pred, mu, log_var, latent, latent_img = model(input_v, input, x_source_v=input_source_v, latent=latent_t_f, source_img=neutral_img, target_img=imgs, pix2face_idxs=pix2face_idxs, pix2face_vals=pix2face_vals, neutral_pix2face_idxs=neutral_pix2face_indices, neutral_pix2face_vals=neutral_pix2face_values, face2vertex_idxs=face2vertex_indices, face2vertex_vals=face2vertex_values, batch_gradX=gradX_batch, batch_gradY=gradY_batch)
                    # t_all = t_all +  t
                else:
                    g_pred, mu, log_var, latent, latent_img = model(input_v, input, x_source_v=input_source_v, source_img=neutral_img, target_img=imgs, pix2face_idxs=pix2face_idxs, pix2face_vals=pix2face_vals, neutral_pix2face_idxs=neutral_pix2face_indices, neutral_pix2face_vals=neutral_pix2face_values, face2vertex_idxs=face2vertex_indices, face2vertex_vals=face2vertex_values, batch_gradX=gradX_batch, batch_gradY=gradY_batch)

                if torch.isnan(g_pred).any():
                    print()

                if not args.ab_vertex:
                    g_pred = normalizer.inv_normalize(g_pred)

                    g_pred = reconstruct_jacobians(g_pred)
                    if torch.isnan(g_pred).any():
                        print()
                    out_pred = myfunc(g_pred, solver, idxs, vals, (N_VERTEX, N_FACE * 3))
                else:
                    out_pred = normalizer.inv_normalize(g_pred)

                out_pred = out_pred -  out_pred.mean(axis=1, keepdim=True)


                if iden_vec is not None:
                    iden_vec_ = iden_vec[write_idx]
                else:
                    iden_vec_ = None
                if args.norm_factor != 0:
                    norm = calc_norm_torch(out_pred, face)
                else:
                    norm = None
                loss = calc_loss(idx, out_pred, out_gt, g_pred, g, mu, log_var, latent, reduce=False, v_factor=args.v_factor, j_factor=args.j_factor, kld_factor=args.kld_factor, mask=mask, latent_const=latent_const, latent_factor=args.latent_factor, iden_vec=iden_vec_, global_weights=exp_weights, norm=norm, norm_gt=norm_gt, norm_factor=args.norm_factor, latent_type=args.latent_type, latent_shape=args.feat_dim, latent_reg_factor=args.latent_reg_factor)

                if 'test' in head:
                    l2_distance.append(torch.sqrt(torch.sum((out_pred - out_gt)**2, dim=2)))
                for loss_key in loss.keys():
                    total_loss[loss_key].append(loss[loss_key].mean().item())

                if write:
                    if latent_img is not None:
                        latent_img_source=  latent_img[0].detach().cpu().numpy()
                        np.save(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_neutral_img.npy"), (((latent_img_source - latent_img_source.min()) / (latent_img_source.max()- latent_img_source.min())) * 255).astype(np.uint8))

                    for _, exp_idx in enumerate(idx):
                        mesh_out = Mesh(out_pred[_].detach().cpu().numpy(), face)
                        mesh_out.write(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_{exp_idx:04d}_" + ''.join([f'{key}_{value[_]:.4f}_' for key, value in loss.items()]) + '.obj'))
                        mesh_gt = Mesh(out_gt[_].detach().cpu().numpy(), face)
                        mesh_gt.write(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_{exp_idx:04d}_gt.obj"))

                        if args.save_exps:
                            latent_ = latent[_].detach().cpu().numpy()
                            np.save(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_{exp_idx:04d}_vec.npy"), latent_)
                            if latent_img is not None:
                                if latent_img.shape[0] > 1:
                                    latent_img_=  latent_img[_ + 1].detach().cpu().numpy()
                                else:
                                    latent_img_ = latent_img[0].cpu().numpy()
                                if len(latent_img_.shape) == 3: # Full img
                                    np.save(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_{exp_idx:04d}_img.npy"), (((latent_img_ - latent_img_.min()) / (latent_img_.max()- latent_img_.min())) * 255).astype(np.uint8))
                                else:
                                    np.save(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_{exp_idx:04d}_img.npy"), latent_img_)
                            if latent_const is not None and type(latent_const) != str:
                                latent_gt = latent_const[exp_idx].detach().cpu().numpy()
                                np.save(os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}', f"{out_idx:04d}_{exp_idx:04d}_expvec_gt.npy"), latent_gt)


    logging.info(f" [{args.save_head}] Epoch {e} done. {head}: " + ''.join([f'{key}: {np.mean(value):.6f}-{np.std(value):.6f}-{np.median(value):.6f} ' for key, value in total_loss.items()]) + f't: {time.time()-t__:.2f}s')
    if 'test' in head:
        l2_distance = torch.tensor([_.mean() for _ in l2_distance])
        logging.info(f" [{args.save_head}] Epoch {e} done. {head} l2 distance: {torch.mean(l2_distance.view(-1, )):.6f}-{torch.std(l2_distance.view(-1, )):.6f}-{torch.median(l2_distance.view(-1, )):.6f}")
    for key, value in total_loss.items():
        writer.add_scalar(f'{head}-loss/{key}', np.mean(value), e)

    model.train()

def train(model, train_loaders, val_loaders, dfn_info, transfs, face, args, normalizer, transf_shift, latent_const=None, latent_const_val=None, iden_vec=None, exp_weights = None, epoch=None, head='train', test_loaders=None, fix_mlp=False):
    global writer, optim, scheduler, myfunc, Loss


    dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]
    if isinstance(model, nn.DataParallel):
        model.module.update_precomputes(dfn_info)
        for name, param in model.module.named_parameters():
            if any(_ in name for _ in ['mass', 'L_ind', 'L_val', 'evals', 'evecs', 'grad_X_ind', 'grad_X_val', 'grad_Y_ind', 'grad_Y_val', 'faces']):
                param.requires_grad = False
            else:
                param.requires_grad = True
        if fix_mlp:
            for param in model.module.linears.parameters():
                param.requires_grad = False
            for param in model.module.linear_out.parameters():
                param.requires_grad= False
            for param in model.module.gns.parameters():
                param.requires_grad = False
    else:
        model.update_precomputes(dfn_info)
        for name, param in model.named_parameters():
            if any(_ in name for _ in ['mass', 'L_ind', 'L_val', 'evals', 'evecs', 'grad_X_ind', 'grad_X_val', 'grad_Y_ind', 'grad_Y_val', 'faces']):
                param.requires_grad = False
            else:
                param.requires_grad = True
        if fix_mlp:
            for param in model.linears.parameters():
                param.requires_grad = False
            for param in model.linear_out.parameters():
                param.requires_grad= False
            for param in model.gns.parameters():
                param.requires_grad = False
    
    # print(head)
    # for name, param in model.named_parameters():
    #     print(name, param.requires_grad)
    # return 
    if exp_weights is not None and type(exp_weights) == np.ndarray:
        exp_weights = torch.from_numpy(exp_weights).float()
    if exp_weights is not None:
        exp_weights = exp_weights.to(args.device)

    if epoch == None:
        resume_epoch = args.resume_id
        epoch_to_run = args.EPOCH
    else:
        epoch_to_run = 0
        resume_epoch = epoch
    
    for e in range(resume_epoch, resume_epoch + epoch_to_run + 1):
        # print(e)
        total_loss = defaultdict(list)
        tt = time.time()
        if args.debug:
            t_ = time.time()
            t_data_prepare = 0
            t_data_load = 0
            t_model_detail = np.array([0, 0, 0])
            t_model = 0
            t_df = 0
            t_bk = 0
            t_grad = 0
            t_render = 0
        if args.write:
            save_dir = os.path.join(args.save_dir, args.save_head+f'{e:04d}_{head}_train')
                                    
            if os.path.exists(save_dir):
                logging.warning(f"Store directory {save_dir} exists, deleting...")
                shutil.rmtree(save_dir)
        
            os.makedirs(save_dir)
        for out_idx, train_loader in enumerate(train_loaders):
            face = train_loader.dataset.face
            input = train_loader.dataset.inputs_source_f.to(args.device).unsqueeze(0)
            input_source_v = train_loader.dataset.inputs_source_v.to(args.device).unsqueeze(0)
            # Randomize dfn operators for DN_e, not using
            if train_loader.dataset.dfn_info_list is not None and args.global_encoder == 'dfn':
                dfn_info = train_loader.dataset.dfn_info_list[torch.randint(len(train_loader.dataset.dfn_info_list), (1,))]
                dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]
                # face = train_loader.dataset.face

                if len(args.parallel_ids) > 0:
                    model.module.encoder.update_precomputes(dfn_info)
                else:
                    model.encoder.update_precomputes(dfn_info)

            # Update dfn operatores for DN
            if train_loader.dataset.dfn_info is not None and args.iden_encoder == 'dfn':
                dfn_info = train_loader.dataset.dfn_info
                
                dfn_info = [_.to(args.device).float() if type(_) is not torch.Size else _  for _ in dfn_info]
                if len(args.parallel_ids) > 0:
                    model.module.global_pn.update_precomputes(dfn_info)
                else:
                    model.global_pn.update_precomputes(dfn_info)
                if not train_loader.dataset.dfn_info_list:
                    if len(args.parallel_ids) > 0:
                        model.module.encoder.update_precomputes(dfn_info)
                    else:
                        model.encoder.update_precomputes(dfn_info)

            if args.img:
                neutral_img = train_loader.dataset.neutral_img.to(args.device).unsqueeze(0)
                if args.pix2face:
                    neutral_pix2face_indices = train_loader.dataset.neutral_pix2face_indices.to(args.device).unsqueeze(0)
                    neutral_pix2face_values = train_loader.dataset.neutral_pix2face_values.to(args.device).unsqueeze(0)
                    face2vertex_indices = train_loader.dataset.face2vertex_indices.to(args.device).unsqueeze(0)
                    face2vertex_values = train_loader.dataset.face2vertex_values.to(args.device).unsqueeze(0)
                else:
                    # neutral_img = None
                    neutral_pix2face_indices = None
                    neutral_pix2face_values = None
                    face2vertex_indices = None
                    face2vertex_values = None
            else:
                neutral_img, neutral_pix2face_indices, neutral_pix2face_values, face2vertex_indices, face2vertex_values = None, None, None, None, None
            if len(args.parallel_ids) > 0:
                input_source_v = input_source_v.expand(len(args.parallel_ids), -1, -1)
                input = input.expand(len(args.parallel_ids), -1, -1)
                if args.img:
                    neutral_img = neutral_img.expand(len(args.parallel_ids), -1, -1, -1)
                    if args.pix2face:
                        neutral_pix2face_indices = neutral_pix2face_indices.expand(len(args.parallel_ids), -1, -1)
                        neutral_pix2face_values = neutral_pix2face_values.expand(len(args.parallel_ids), -1)
                        
                        face2vertex_indices = face2vertex_indices.expand(len(args.parallel_ids), -1, -1)
                        face2vertex_values = face2vertex_values.expand(len(args.parallel_ids), -1)

            loader_fake = torch.utils.data.DataLoader(fake_dataset(len(train_loader.dataset)), batch_size=args.batch_size, shuffle=True, num_workers=0, drop_last=False)

            for idx in loader_fake:
                input_v = train_loader.dataset.inputs_target_v[idx].to(args.device)
                g = train_loader.dataset.gradients[idx].to(args.device)
                out_gt = train_loader.dataset.verts[idx].to(args.device)
                norm_gt = train_loader.dataset.norms[idx].to(args.device)
                if args.img:
                    imgs = train_loader.dataset.img[idx].to(args.device)

                    if args.pix2face:
                        pix2face_idxs = train_loader.dataset.pix2face_indices[idx]
                        pix2face_vals = train_loader.dataset.pix2face_values[idx]

                    else:
                        pix2face_idxs = None
                        pix2face_vals = None
                else:
                    imgs, pix2faces = None, None
                    pix2face_idxs = None
                    pix2face_vals = None
                if train_loader.dataset.gradX is not None:
                    gradX_batch = train_loader.dataset.gradX[idx].to(args.device)
                    # gradX_batch.requires_grad = False
                    gradY_batch = train_loader.dataset.gradY[idx].to(args.device)
                    # gradY_batch.requires_grad = False
                else:
                    gradX_batch = None
                    gradY_batch = None
                    
                N_VERTEX = out_gt.shape[1]
                N_FACE = input.shape[1]

                if args.debug:
                    t_data_prepare += (time.time() - t_)
                    # logging.debug(f'Data preparing: {time.time() - t:.2f}s')
                    t_ = time.time()


                g = g.squeeze().to(args.device)
                g = normalizer.inv_normalize(g)
                if len(out_gt.shape) == 4:
                    out_gt = out_gt.squeeze()
                out_gt = out_gt.to(args.device)
                out_gt = out_gt - out_gt.mean(axis=1, keepdim=True)
                if 'test' in head:
                    transf = transfs[transf_shift['test'][out_idx]]
                elif 'val' in head:
                    transf = transfs[transf_shift['val'][out_idx]]
                else:
                    transf = transfs[transf_shift['train'][out_idx]]

                if args.debug:
                    t_data_load += (time.time() - t_)
                    t_ = time.time()

                solver = transf.solver
                idxs, vals = coalesce(from_dlpack(transf.idxs.toDlpack()).long(), from_dlpack(transf.vals.toDlpack()), m=N_FACE *3, n=N_VERTEX)
                idxs, vals = transpose(idxs, vals, m=N_FACE *3, n=N_VERTEX)
                g = reconstruct_jacobians(g)
                
                if e == 0:
                    Loss = torch.nn.MSELoss()
                    with torch.no_grad():

                        out_gt_g = myfunc(g, solver, idxs, vals, (N_VERTEX, N_FACE * 3))
                        out_gt_g = out_gt_g - out_gt_g.mean(axis=1, keepdim=True)
                    try:
                        assert Loss(out_gt, out_gt_g) < 1e-7
                    except AssertionError:
                        logging.info(f'The l2 difference between ground truth and transfered: {Loss(out_gt, out_gt_g)} is bigger than theroshold 1e-7!')   
                        if len(idx) == 1:
                            write_idx = idx
                        else:
                            write_idx = out_idx

                        for exp_idx in range(out_gt.shape[0]):
                            with torch.no_grad():
                                mesh = Mesh(out_gt_g[exp_idx].detach().cpu().numpy(), face)
                                mesh_gt = Mesh(out_gt[exp_idx].detach().cpu().numpy(), face)
                                logging.info(exp_idx, Loss(out_gt_g[exp_idx], out_gt[exp_idx]).detach().item())
                                mesh.write(f'./{write_idx}_{exp_idx}_transf_{Loss(out_gt[exp_idx], out_gt_g[exp_idx]).item()}.obj')
                                mesh_gt.write(f'./{write_idx}_{exp_idx}_gt.obj')
                                dg_temp = g[exp_idx].detach().cpu().numpy()
                                np.save(f'./{write_idx}_{exp_idx}_g.npy', dg_temp)
                        logging.info(f'idx: {write_idx} All saves to ./[idx]_[exp_idx]_[gt/transf].obj')
                        logging.info(f'Abort.')
                        exit(1)

                for param in model.parameters():
                    param.grad = None

                if args.teacher_forcing:
                    if latent_const is None or type(latent_const) == str:
                        logging.error('Teacher Forcing requires gt expression vectors')
                        raise ValueError
                    latent_t_f = latent_const[idx].to(args.device)
                    if args.teacher_forcing_noise:
                        latent_t_f += (torch.randn(latent_t_f.shape, device=latent_t_f.device) * 1e-3)
                    if args.latent_type == 'extend':
                        latent_t_f = torch.cat([latent_t_f, torch.zeros((latent_t_f.shape[0], args.feat_dim - 53)).to(latent_t_f.device)], dim=-1)

                    g_pred, mu, log_var, latent, latent_img= model(input_v, input, latent=latent_t_f, x_source_v=input_source_v, source_img=neutral_img, target_img=imgs, pix2face_idxs=pix2face_idxs, pix2face_vals=pix2face_vals, neutral_pix2face_idxs=neutral_pix2face_indices, neutral_pix2face_vals=neutral_pix2face_values, face2vertex_idxs=face2vertex_indices, face2vertex_vals=face2vertex_values,return_time=False, batch_gradX=gradX_batch, batch_gradY=gradY_batch)

                    # t_all = t_all +  t_model_np
                else:

                    g_pred, mu, log_var, latent, latent_img = model(input_v, input, x_source_v=input_source_v, source_img=neutral_img, target_img=imgs,pix2face_idxs=pix2face_idxs, pix2face_vals=pix2face_vals, neutral_pix2face_idxs=neutral_pix2face_indices, neutral_pix2face_vals=neutral_pix2face_values, face2vertex_idxs=face2vertex_indices, face2vertex_vals=face2vertex_values,return_time=False, batch_gradX=gradX_batch, batch_gradY=gradY_batch)


                if args.debug:

                    t_model += (time.time() - t_)
                    t_ = time.time()

                    if torch.isnan(g_pred).any():
                        print()
                    if torch.isnan(g_pred).any():
                        logging.error('Found Nan in g_pred directly after model output!')
                        ckpt = {
                            'epoch': e,
                            'model': model.state_dict(),
                            'optim': optim.state_dict(),
                            'lr_sched': scheduler.state_dict(),
                            'args': args
                        }
                        torch.save(ckpt, os.path.join(args.save_dir, args.save_head + f'{e}.pth'))
                        logging.info(f"Save checkpoints to {os.path.join(args.save_dir, args.save_head + f'{e}.pth')}")
                        raise ValueError


                g_pred = normalizer.inv_normalize(g_pred)
                g_pred = reconstruct_jacobians(g_pred)

                if args.debug:
                    if torch.isnan(g_pred).any():
                        logging.error('Found Nan in g_pred directly after reconstruction!')
                        ckpt = {
                            'epoch': e,
                            'model': model.state_dict(),
                            'optim': optim.state_dict(),
                            'lr_sched': scheduler.state_dict(),
                            'args': args
                        }
                        torch.save(ckpt, os.path.join(args.save_dir, args.save_head + f'{e}.pth'))
                        logging.info(f"Save checkpoints to {os.path.join(args.save_dir, args.save_head + f'{e}.pth')}")
                        raise ValueError
                out_pred = myfunc(g_pred, solver, idxs, vals, (N_VERTEX, N_FACE * 3))
                out_pred = out_pred -out_pred.mean(axis=1, keepdim=True)

                if args.debug:
                    # logging.debug(f'DF running: {time.time() - t:.4f}')
                    t_df += (time.time() - t_)
                    t_ = time.time()

                if iden_vec is not None:
                    iden_vec_ = iden_vec[out_idx] # The current identity
                else:
                    iden_vec_ = None
                if args.norm_factor != 0:
                    norm = calc_norm_torch(out_pred, face)
                else:
                    norm = None

                loss = calc_loss(idx, out_pred, out_gt, g_pred, g, None, log_var, latent, v_factor=args.v_factor, j_factor=args.j_factor, kld_factor=args.kld_factor, mask=None, latent_const=latent_const, latent_factor=args.latent_factor, iden_vec=iden_vec_, zero_loss=None, zero_activation=None, global_weights=exp_weights, norm=norm, norm_gt=norm_gt, norm_factor=args.norm_factor, latent_type=args.latent_type, latent_shape=args.feat_dim, latent_reg_factor=args.latent_reg_factor)

                loss['total_loss'].backward()

                if args.debug:
                    t_bk += (time.time()-  t_)
                    t_ = time.time()
                    for name, param in model.named_parameters():
                        if param.grad is not None and param.grad.isnan().any():
                            logging.error(f'Nan found in gradients of parameter: {name}')
                            ckpt = {
                                'epoch': e,
                                'model': model.state_dict(),
                                'optim': optim.state_dict(),
                                'lr_sched': scheduler.state_dict(),
                                'args': args
                            }
                            torch.save(ckpt, os.path.join(args.save_dir, args.save_head + f'{e}.pth'))
                            logging.info(f"Save checkpoints to {os.path.join(args.save_dir, args.save_head + f'{e}.pth')}")
                            raise ValueError
                optim.step()
                for key, value in loss.items():
                    total_loss[key].append(value.mean().item())
                # logging.debug(f'Gradient updating {time.time() - t:.4f}')
                if args.debug:
                    t_grad += (time.time() - t_)
                

                max_grads = [x.grad.abs().max().item() for x in list(model.parameters()) if x.grad is not None]
                avg_grads = [x.grad.abs().mean().item() for x in list(model.parameters()) if x.grad is not None]
                if torch.isnan(torch.mean(torch.tensor(avg_grads))):
                    print()

                t_ = time.time()
                if args.write:
                    for _, exp_idx in enumerate(idx):
                        mesh_out = Mesh(out_pred[_].detach().cpu().numpy(), face)
                        mesh_out.write(os.path.join(save_dir, f"{out_idx:04d}_{exp_idx:05d}_" + '.obj'))
                        mesh_gt = Mesh(out_gt[_].detach().cpu().numpy(), face)
                        mesh_gt.write(os.path.join(save_dir, f"{out_idx:04d}_{exp_idx:05d}_gt.obj"))
                        if args.save_exps:
                            latent_ = latent[_].detach().cpu().numpy()
                            np.save(os.path.join(save_dir, f"{out_idx:04d}_{exp_idx:04d}_vec.npy"), latent_)
                            # if latent_img is not None:
                            #     if latent_img.shape[0] > 1:
                            #         latent_img_=  latent_img[_ + 1].detach().cpu().numpy()
                            #     else:
                            #         latent_img_ = latent_img[0].cpu().numpy()
                            #     if len(latent_img_.shape) == 3: # Full img
                            #         np.save(os.path.join(save_dir, f"{out_idx:04d}_{exp_idx:04d}_img.npy"), (((latent_img_ - latent_img_.min()) / (latent_img_.max()- latent_img_.min())) * 255).astype(np.uint8))
                            #     else:
                            #         np.save(os.path.join(save_dir, f"{out_idx:04d}_{exp_idx:04d}_img.npy"), latent_img_)
                            if latent_const is not None and type(latent_const) != str:
                                latent_gt = latent_const[exp_idx].detach().cpu().numpy()
                                np.save(os.path.join(save_dir, f"{out_idx:04d}_{exp_idx:04d}_expvec_gt.npy"), latent_gt)
        # Finished an epoch
        scheduler.step()
        if 'test' not in head:
            logging.info(f" [{args.save_head}] Epoch {e} done. {head} " + ''.join([f'{key}: {np.mean(value):.4f} ' for key, value in total_loss.items()])  + f"max_grad: {np.max(max_grads):.4f}, avg_grad: {np.mean(avg_grads):.4f},  lr: {scheduler.get_last_lr()}, t: {time.time() - tt:.4f}s")
        else:
             logging.info(f" [{args.save_head}] Epoch {e} done. {head} " + ''.join([f'{key}: {np.mean(value):.4f} ' for key, value in total_loss.items()])  + f"t: {time.time() - tt:.4f}s")
        if args.debug:
            print(f't_data_prep: {t_data_prepare:.3f}s | t_data_load: {t_data_load:.3f}s | t_model: {t_model:.3f}s | t_df: {t_df:.3f}s | t_bk: {t_bk:.3f}s | t_step: {t_grad:.3f}s |t_render: {t_render:.3f}s| total: {t_data_load + t_data_prepare + t_model + t_df + t_grad + t_render:.3f}s')
            print(f''.join([f'{name}:{ttt:.2f}s ' for name, ttt in zip(['exp', 'iden','linear'], t_model_detail)]))

        for key, value in total_loss.items():
            writer.add_scalar(f"{head}-loss/{key}", np.mean(value), e)
        if 'test' not in head:
            writer.add_scalar(f'{head}/max_grad', np.max(max_grads), e)
            writer.add_scalar(f'{head}/avg_grad', np.mean(avg_grads), e)
            writer.add_scalar('lr/lr', scheduler.get_last_lr(), e)
            writer.add_scalar(f'{head}/t', time.time() - tt, e)

        if not e % args.eval_freq:
            ckpt = {
                'epoch': e,
                'model': model.state_dict(),
                'optim': optim.state_dict(),
                'lr_sched': scheduler.state_dict(),
                'args': args
            }
            torch.save(ckpt, os.path.join(args.save_dir, args.save_head + f'{e}.pth'))
            eval(model, e, face, val_loaders, dfn_info, transfs, args,  normalizer, write=False, shift=transf_shift, device=args.device, fraction=1, latent_const=latent_const_val, iden_vec=iden_vec,  exp_weights=exp_weights, head=head+'_val')
            eval(model, e, face, train_loaders, dfn_info, transfs, args,  normalizer, write=False, shift=transf_shift, device=args.device, head=head + '_train', fraction=10, exp_weights=exp_weights, latent_const=latent_const, iden_vec=iden_vec)
            if test_loaders is not None:
                eval(model, e, face, test_loaders, dfn_info, transfs, args,  normalizer, write=False, shift=transf_shift, device=args.device, fraction=1, latent_const=latent_const_val, iden_vec=iden_vec,  exp_weights=exp_weights, head=head+'_test')
        if not e % 10:
            ckpt = {
                'epoch': e,
                'model': model.state_dict(),
                'optim': optim.state_dict(),
                'lr_sched': scheduler.state_dict(),
                'args': args
            }
            torch.save(ckpt, os.path.join(args.save_dir, args.save_head + f'{e}.pth'))



if __name__ == '__main__':
    use_clearml = False
    os.environ['OMP_NUM_THREADS'] = '8'
    torch.set_num_threads(8)
    torch.manual_seed(0)
    np.random.seed(0)

    torch.multiprocessing.set_start_method('spawn')
    warnings.filterwarnings("ignore", category=UserWarning)
    warnings.filterwarnings("ignore", category=DeprecationWarning)
    
    parser = train_parser()
    args = parser.parse_args()
    if use_clearml:
        from clearml import Task
        task = Task.init(project_name='NFR_biwi', task_name=args.save_head)
    if args.iden_encoder == 'dfn':
        args.use_source_v = True
    else:
        args.use_source_v = False
    args.save_dir = os.path.join(args.save_dir, args.save_head)
    log_file = os.path.join(args.save_dir, 'log.txt')
    args.save_head = args.save_head + '_'

    if not os.path.exists(args.save_dir):
        os.makedirs(args.save_dir)

    file_handler = logging.FileHandler(filename=log_file)
    stdout_handler = logging.StreamHandler(sys.stdout)
    handlers = [file_handler, stdout_handler]
    logging.basicConfig(handlers=handlers, format="[%(levelname)s] %(funcName)s: %(message)s", level=logging.DEBUG, force=True)
    writer = SummaryWriter(log_dir=log_file.replace('.txt', '.board'))

    logging.info(f'logging to {log_file}')
    logging.info(f'Dataset: {args.dataset}')
    if args.ab_vertex:
        logging.info(f'WARNING, ablation on jacobians activated.')
        if args.feature_type != 'cents&norms':
            logging.error(f'When ablate jacobians, inputs_v can only be cents&norms! Abort')
            raise ValueError

    t = time.time()
    latent_const = None
    exp_weights = None
    pre_computes = None

    normalizer = Normalizer(args.std_file, args.device)
    if args.wks:
        wks_normalizer = Normalizer(args.wks_std_file, args.device, zero_mean=False)
    else:
        wks_normalizer = None
    if args.landmark:
        landmark_normalizer = Normalizer_landmark(args.landmark_file, args.device)
    else:
        landmark_normalizer = None
    if args.img:
        img_normalizer = Normalizer_img(args.img_file, args.device)
    else:
        img_normalizer = None
    # renderer = renderer()
    with cupy.cuda.Device(int(args.device[-1])):
        if args.dataset == 'MF':
            if args.data_head == 'MF_all_v5' or args.data_head == 'MF_all_v4':
                n_mf = 11
                shuffle = False
                per_loader_dfn_info = False
                transfs_shift_mf = {'train': range(n_mf), 'val': range(n_mf), 'test': range(n_mf)}

            elif args.data_head == 'MF_shift_scale':
                n_mf = 100
                shuffle = True
                per_loader_dfn_info = True
                transfs_shift_mf = {'train': range(n_mf), 'val': range(n_mf, n_mf+10), 'test': range(n_mf+10, n_mf+20)}
            elif args.data_head == 'MF_holes':
                n_mf = 100
                shuffle = True
                per_loader_dfn_info = True
                transfs_shift_mf = {'train': range(n_mf), 'val': range(n_mf, n_mf+10), 'test': range(n_mf+10, n_mf+20)}
            elif args.data_head == 'COMA_no_inner':
                n_mf = 12
                shuffle = False
                per_loader_dfn_info = False
            face_mf, transfs_mf, train_loaders_mf, val_loaders_mf, test_loaders_mf, dfn_info_mf = load_data_mf_v3(args.data_dir, args.data_head, args.batch_size, n_mf, normalizer,   use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, shuffle=shuffle, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf = {'train': range(len(train_loaders_mf)), 'val': range(len(val_loaders_mf)), 'test': range(len(test_loaders_mf))}
            dfn_info = dfn_info_mf
        elif args.dataset == 'COMA':
            n_coma = 1
            face_mf, transfs_mf, train_loaders_mf, val_loaders_mf, test_loaders_mf, dfn_info_mf = load_data_mf_v3(args.data_dir, args.data_head, args.batch_size, n_coma, normalizer,   use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, shuffle=False, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            dfn_info = dfn_info_mf
            transfs_shift_mf = {'train': range(n_coma), 'val': range(n_coma), 'test': range(n_coma)}

        elif args.dataset == 'ICT_single':
            exp_weights, latent_const, iden_vec, face_mf, transfs_mf, train_loaders_mf, val_loaders_mf, test_loaders_mf,  dfn_info_mf = load_data_ict_all(args.data_dir, args.data_head, args.batch_size, 10, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf = {'train': range(len(train_loaders_mf)), 'val': range(len(train_loaders_mf), len(train_loaders_mf) + len(val_loaders_mf)), 'test': range(len(train_loaders_mf) + len(val_loaders_mf), len(train_loaders_mf) + len(val_loaders_mf) + len(test_loaders_mf))}
            latent_const = torch.from_numpy(latent_const).float()
            dfn_info = dfn_info_mf
            # latent_const = train_loaders[0].dataset.exp_vec        
        elif args.dataset == 'ICT_random':
            latent_const, iden_vec, face, transfs, train_loaders, val_loaders, test_loaders,  dfn_info,  inputs_shape, inputs_v_shape = load_data_ict_random(args.data_dir, args.data_head, args.batch_size, args.eval_range[1], normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, use_hks=args.hks, zero_mean=args.zero_mean, only_test=args.test, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)

            # latent_const = train_loaders[0].dataset.exp_vec
        elif args.dataset == 'ICT_live':
            iden_vec_live, latent_const_live, latent_const_val_live, latent_const_test_live, face_mf, transfs_mf, train_loaders_mf, val_loaders_mf, test_loaders_mf,  dfn_info_mf = load_data_ict_live(args.data_dir, args.data_head, args.batch_size, 10, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf = {'train': range(len(train_loaders_mf)), 'val': range(len(train_loaders_mf), len(train_loaders_mf) + len(val_loaders_mf)), 'test': range(len(train_loaders_mf) + len(val_loaders_mf), len(train_loaders_mf) + len(val_loaders_mf) + len(test_loaders_mf))}
            latent_const = torch.from_numpy(latent_const_live).float()
            dfn_info = dfn_info_mf
            latent_const_live = torch.from_numpy(latent_const_live).float()
            latent_const_val_live = torch.from_numpy(latent_const_val_live).float()
            latent_const_test_live = torch.from_numpy(latent_const_test_live).float()
            
        elif args.dataset == 'ICT_all':
            if args.debug:
                n_single = 1
                n_random = 1
                n_live = 4
                n_holes = 1
                n_shift_scale = 1
            else:
                n_single = 10
                n_random = 6
                n_live = 10
                n_holes = 100
                n_shift_scale = 100
            args.data_head = 'ICT_single_coarse' # Single expression activations
            exp_weights, latent_const, iden_vec, face, transfs, train_loaders, val_loaders, test_loaders,  dfn_info = load_data_ict_all(args.data_dir, args.data_head, args.batch_size, n_single, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift = {'train': range(len(train_loaders)), 'val': range(len(train_loaders), len(train_loaders) + len(val_loaders)), 'test': range(len(train_loaders) + len(val_loaders), len(train_loaders) + len(val_loaders) + len(test_loaders))}
            latent_const = torch.from_numpy(latent_const).float()
            args.data_head = 'ICT_random_50' # Random expression activations
            latent_const_, iden_vec_, face, transfs_, train_loaders_, val_loaders_, test_loaders_,  dfn_info_ = load_data_ict_random(args.data_dir, args.data_head, args.batch_size, n_random, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_ = {'train': range(len(train_loaders_)), 'val': [len(train_loaders_)], 'test': [len(train_loaders_) + 1]}
            latent_const_ = torch.from_numpy(latent_const_).float()
            args.data_head = 'ICT_live_100' # Live captured expression activations
            iden_vec_live, latent_const_live, latent_const_val_live, latent_const_test_live, face, transfs_live, train_loaders_live, val_loaders_live, test_loaders_live,  dfn_info_live = load_data_ict_live(args.data_dir, args.data_head, args.batch_size, n_live, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_live = {'train': range(len(train_loaders_live)), 'val': range(len(train_loaders_live), len(train_loaders_live) + len(val_loaders_live)), 'test': range(len(val_loaders_live), len(val_loaders_live) + len(test_loaders_live))}
            latent_const_live = torch.from_numpy(latent_const_live).float()
            latent_const_val_live = torch.from_numpy(latent_const_val_live).float()
            latent_const_test_live = torch.from_numpy(latent_const_test_live).float()
            
            args.data_head = 'ICT_shift_scale' # Shift and scale augmented expression activations
            latent_const_ss, face_ss, transfs_ss, train_loaders_ss, val_loaders_ss, test_loaders_ss,  dfn_info_ss = load_data_ict_argument(args.data_dir, args.data_head, args.batch_size, n_shift_scale, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_ss = {'train': range(len(train_loaders_ss)), 'val': range(len(train_loaders_ss), len(train_loaders_ss) + 10), 'test': range(len(train_loaders_ss)+10, len(train_loaders_ss)+20)}
            latent_const_ss = torch.from_numpy(latent_const_ss).float()
            args.data_head = 'ICT_holes' # Holes augmented expression activations
            latent_const_holes, face_holes, transfs_holes, train_loaders_holes, val_loaders_holes, test_loaders_holes,  dfn_info_holes = load_data_ict_argument(args.data_dir, args.data_head, args.batch_size, n_holes, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_holes = {'train': range(len(train_loaders_holes)), 'val': range(len(train_loaders_holes), len(train_loaders_holes) + 10), 'test': range(len(train_loaders_holes)+10, len(train_loaders_holes)+20)}
            # dfn_info = dfn_info_holes
            latent_const_holes = torch.from_numpy(latent_const_holes).float()
        elif args.dataset == 'ALL' or args.dataset == 'MF_all':
            if args.debug:
                n_live = 4
                n_mf = 1
                n_coma = 1
                n_ict_ss = 1
                n_mf_ss = 1
                n_mf_holes=  1
            else:
                n_live = 20
                n_mf = 11
                n_coma = 12
                n_ict_ss = 100
                n_mf_ss = 100
                n_mf_holes = 100
                if args.test:
                    n_live = 1
                    n_mf = 11
                    n_coma = 1
                    n_ict_ss = 1
                    n_mf_ss = 1
                    n_mf_holes = 1
            args.data_head = 'ICT_live_100'
            iden_vec_live, latent_const_live, latent_const_val_live, latent_const_test_live, face, transfs_live, train_loaders_live, val_loaders_live, test_loaders_live,  dfn_info_live = load_data_ict_live(args.data_dir, args.data_head, args.batch_size, n_live, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_live = {'train': range(len(train_loaders_live)), 'val': range(len(train_loaders_live), len(train_loaders_live) + len(val_loaders_live)), 'test': range(len(val_loaders_live), len(val_loaders_live) + len(test_loaders_live))}
            latent_const_live = torch.from_numpy(latent_const_live).float()
            latent_const_val_live = torch.from_numpy(latent_const_val_live).float()
            latent_const_test_live = torch.from_numpy(latent_const_test_live).float()
            dfn_info = dfn_info_live
            
            
            args.data_head = 'MF_all_v5' # Multiface activations
            # args.test = False
            face_mf, transfs_mf, train_loaders_mf, val_loaders_mf, test_loaders_mf, dfn_info_mf = load_data_mf_v3(args.data_dir, args.data_head, args.batch_size, n_mf, normalizer,   use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf = {'train': range(n_mf), 'val': range(n_mf), 'test': range(n_mf)}
            args.data_head = 'ICT_shift_scale'
            latent_const_ss, face_ss, transfs_ss, train_loaders_ss, val_loaders_ss, test_loaders_ss,  dfn_info_ss = load_data_ict_argument(args.data_dir, args.data_head, args.batch_size, n_ict_ss, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type, global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_ss = {'train': range(len(train_loaders_ss)), 'val': range(len(train_loaders_ss), len(train_loaders_ss) + 10), 'test': range(len(train_loaders_ss)+10, len(train_loaders_ss)+20)}
            latent_const_ss = torch.from_numpy(latent_const_ss).float()
            args.data_head = 'MF_shift_scale'
            face_mf_ss, transfs_mf_ss, train_loaders_mf_ss, val_loaders_mf_ss, test_loaders_mf_ss, dfn_info_mf_ss = load_data_mf_v3(args.data_dir, args.data_head, args.batch_size, n_mf_ss, normalizer,   use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info,  grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf_ss = {'train': range(len(train_loaders_mf_ss)), 'val': range(len(train_loaders_mf_ss), len(train_loaders_mf_ss) + 10), 'test': range(len(train_loaders_mf_ss) + 10, len(train_loaders_mf_ss) + 20)}
            args.data_head = 'MF_holes'
            face_mf_holes, transfs_mf_holes, train_loaders_mf_holes, val_loaders_mf_holes, test_loaders_mf_holes, dfn_info_mf_holes = load_data_mf_v3(args.data_dir, args.data_head, args.batch_size, n_mf_holes, normalizer,   use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info,  grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf_holes = {'train': range(len(train_loaders_mf_holes)), 'val': range(len(train_loaders_mf_holes), len(train_loaders_mf_holes) + 10), 'test': range(len(train_loaders_mf_holes) + 10, len(train_loaders_mf_holes) + 20)}


        elif args.dataset == 'biwi':
            if args.debug:
                n_mf = 1
                n_live = 4
            else:
                n_live = 20
                n_mf = 14
            # args.test = False
            args.data_head = 'ICT_live_100'
            iden_vec_live, latent_const_live, latent_const_val_live, latent_const_test_live, face, transfs_live, train_loaders_live, val_loaders_live, test_loaders_live,  dfn_info_live = load_data_ict_live(args.data_dir, args.data_head, args.batch_size, n_live, normalizer,  use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_live = {'train': range(len(train_loaders_live)), 'val': range(len(train_loaders_live), len(train_loaders_live) + len(val_loaders_live)), 'test': range(len(val_loaders_live), len(val_loaders_live) + len(test_loaders_live))}
            latent_const_live = torch.from_numpy(latent_const_live).float()
            latent_const_val_live = torch.from_numpy(latent_const_val_live).float()
            latent_const_test_live = torch.from_numpy(latent_const_test_live).float()
            dfn_info = dfn_info_live
            args.data_head = 'biwi_ssp_8000' # biwi with 8k vertices
            face_mf, transfs_mf, train_loaders_mf, val_loaders_mf, test_loaders_mf, dfn_info_mf = load_data_mf_v3(args.data_dir, args.data_head, args.batch_size, n_mf, normalizer,   use_wks=args.wks, wks_normalizer=wks_normalizer, feature_type=args.feature_type,  global_encoder=args.global_encoder, cache_dir=args.cache_dir, device=args.device, only_test=args.test, use_f=True, use_source_v=args.use_source_v, use_landmarks=args.landmark, landmark_normalizer=landmark_normalizer, use_img=args.img, img_normalizer=img_normalizer, use_pix2face=args.pix2face, random_dfn_info=args.random_dfn_info, grad=args.grad, dfn_info_per_loader=args.dfn_info_per_loader)
            transfs_shift_mf = {'train': range(n_mf), 'val': range(n_mf), 'test': range(n_mf)}

        else:
            logging.error(f'Not implemented datset: {args.dataset}')
            raise NotImplementedError

        logging.info(f'Data loaded, takes up {time.time() - t:.2f}s')


        # DFN_e
        global_encoder_in_shape = 6 if args.feature_type == 'cents&norms' else 12
        if args.wks:
            global_encoder_in_shape += 10
        if args.landmark:
            global_encoder_in_shape += 68 * 2
        # MLP & DFN_i
        in_shape = 6 # cents&norms
        if args.wks:
            in_shape += 10
        if args.landmark:
            in_shape += 68 * 2
        out_shapes = {'matrix': 9, '6dof': 12, 'quat': 10, 'expmap': 9, 'verts':3}
        if args.ab_vertex:
            args.repr = 'verts'

        if dfn_info is not None:
            dfn_info = [_.to(args.device) if type(_) is not torch.Size else _  for _ in dfn_info ]


        logging.info(f'Exp enc: {args.global_encoder}\nExp enc inshape: {global_encoder_in_shape}\nIden enc: {args.iden_encoder}\nIden enc inshape: {in_shape}\n')
        model = latent_space(global_encoder_in_shape, in_shape=in_shape,  out_shape=9, pre_computes=dfn_info, latent_shape=args.feat_dim, encoder=args.global_encoder, iden_encoder=args.iden_encoder, iden_blocks=args.iden_blocks, hid_shape=args.mlp_hid_channel,  non_linear=args.non_linear, residual=False, global_pn=args.global_pn, sampling=args.sampling, pn_norm=args.pn_norm, number_gn=args.num_gn, dfn_blocks=args.dfn_blocks, global_pn_shape=args.global_pn_shape, code_concat=args.code_concat, img_encoder=args.img_enc, img_norm=args.img_norm, use_siren=args.use_siren, img_feat=args.img_feat, img_only_mlp=args.img_only_mlp, img_warp=args.img_warp, global_encoder_grad= not args.no_global_encoder_grad, iden_encoder_grad= not args.no_iden_encoder_grad)

        if len(args.parallel_ids) > 0:
            logging.info(f"Let's use {len(args.parallel_ids)} GPUs!")
            # dim = 0 [30, xxx] -> [10, ...], [10, ...], [10, ...] on 3 GPUs
            model = nn.DataParallel(model, device_ids=args.parallel_ids)
                    
        if not args.test:
            model.to(args.device)
            model.float()
            optim = torch.optim.Adam([{'params':model.parameters(), 'initial_lr': args.initial_lr}], lr=args.initial_lr)
            scheduler = torch.optim.lr_scheduler.StepLR(optim, step_size=100, gamma=0.75, last_epoch=args.resume_id)
            myfunc = deformation_gradient.apply

            if args.resume_id > 0:
                ckpt = torch.load(os.path.join(args.save_dir, args.save_head + f'{args.resume_id}.pth'), map_location=args.device)
                model_state_dict = ckpt['model']
                model = load_state_dict(model, model_state_dict)

                if args.load_lr:
                    optim.load_state_dict(ckpt['optim'])
                    scheduler.load_state_dict(ckpt['lr_sched'])

            if args.pretrained_global_pn:
                ckpt_global_pn = torch.load(args.pretrained_global_pn_dir)
                model.global_pn.load_state_dict(ckpt_global_pn['model'])

            iden_vec = None # just dont use this now 
            if args.dataset == 'ALL' or args.dataset == 'ICT_all' or args.dataset == 'biwi':
                for e in range(args.resume_id, args.EPOCH+args.resume_id):
                    if args.dataset == 'ICT_all':
                        train(model, train_loaders, val_loaders, dfn_info, transfs, face, args, normalizer, transfs_shift, latent_const=latent_const, latent_const_val=latent_const, iden_vec=iden_vec, epoch=e, head='ict_single', exp_weights=exp_weights)

                        train(model, train_loaders_, val_loaders_,  dfn_info_, transfs_, face, args, normalizer, transfs_shift_, latent_const=latent_const_, latent_const_val=latent_const_, iden_vec=None, epoch=e, head='ict_random', exp_weights=None)

                        train(model, train_loaders_live, val_loaders_live, dfn_info_live, transfs_live, face, args, normalizer, transfs_shift_live, latent_const=latent_const_live, latent_const_val=latent_const_val_live, iden_vec=None, epoch=e, head='ict_live', exp_weights=None)

                        train(model, train_loaders_ss, val_loaders_ss, dfn_info_ss, transfs_ss, face_ss, args, normalizer, transfs_shift_ss, latent_const=latent_const_ss, latent_const_val=latent_const_ss, iden_vec=None, epoch=e, head='ict_ss', exp_weights=None)
                        train(model, train_loaders_holes, val_loaders_holes, dfn_info_holes, transfs_holes, face_holes, args, normalizer, transfs_shift_holes, latent_const=latent_const_holes, latent_const_val=latent_const_holes, iden_vec=None, epoch=e, head='ict_holes', exp_weights=None)
                    elif args.dataset == 'ALL':
                        
                        train(model, train_loaders_live, val_loaders_live, dfn_info_live, transfs_live, face, args, normalizer, transfs_shift_live, latent_const=latent_const_live, latent_const_val=latent_const_val_live, iden_vec=None, epoch=e, head='ict_live', exp_weights=None)

                        train(model, train_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=e, head='mf', exp_weights=None, fix_mlp=args.fix_mlp)

                        train(model, train_loaders_ss, val_loaders_ss, dfn_info_ss, transfs_ss, face_ss, args, normalizer, transfs_shift_ss, latent_const=latent_const_ss, latent_const_val=latent_const_ss, iden_vec=None, epoch=e, head='ict_ss', exp_weights=None)

                        train(model, train_loaders_mf_ss, val_loaders_mf_ss, dfn_info_mf_ss, transfs_mf_ss, face_mf_ss, args, normalizer, transfs_shift_mf_ss, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, iden_vec=None, epoch=e, head='mf_ss', exp_weights=None, fix_mlp=args.fix_mlp)

                        train(model, train_loaders_mf_holes, val_loaders_mf_holes, dfn_info_mf_holes, transfs_mf_holes, face_mf_holes, args, normalizer, transfs_shift_mf_holes, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, iden_vec=None, epoch=e, head='mf_holes', exp_weights=None, fix_mlp=args.fix_mlp)

                    elif args.dataset == 'MF_all':
                        train(model, train_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=e, head='mf', exp_weights=None, fix_mlp=args.fix_mlp)

                        train(model, train_loaders_mf_ss, val_loaders_mf_ss, dfn_info_mf_ss, transfs_mf_ss, face_mf_ss, args, normalizer, transfs_shift_mf_ss, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, iden_vec=None, epoch=e, head='mf_ss', exp_weights=None, fix_mlp=args.fix_mlp)

                        train(model, train_loaders_mf_holes, val_loaders_mf_holes, dfn_info_mf_holes, transfs_mf_holes, face_mf_holes, args, normalizer, transfs_shift_mf_holes, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, iden_vec=None, epoch=e, head='mf_holes', exp_weights=None, fix_mlp=args.fix_mlp)

                    elif args.dataset == 'biwi':
                        train(model, train_loaders_live, val_loaders_live, dfn_info_live, transfs_live, face, args, normalizer, transfs_shift_live, latent_const=latent_const_live, latent_const_val=latent_const_val_live, iden_vec=None, epoch=e, head='ict_live', exp_weights=None)
                        
                        train(model, train_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=e, head='biwi', exp_weights=None, fix_mlp=args.fix_mlp)

            else:
                for e in range(args.resume_id, args.EPOCH+args.resume_id):
                    train(model, train_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=e, head='mf', exp_weights=None, test_loaders=test_loaders_mf, fix_mlp = args.fix_mlp)
                    
        else:
            logging.info('Do testing')
            ckpt = torch.load(os.path.join(args.save_dir, args.save_head + f'{args.resume_id}.pth'), map_location=args.device)
            state_dict = ckpt['model']
            model.float()
            myfunc = deformation_gradient.apply
            model = load_state_dict(model, state_dict)
            model.to(args.device)
            optim = torch.optim.Adam([{'params':model.parameters(), 'initial_lr': args.initial_lr}], lr=args.initial_lr)
            scheduler = torch.optim.lr_scheduler.StepLR(optim, step_size=100, gamma=0.75, last_epoch=args.resume_id)
            
            if args.dataset == 'ICT_all':
                eval(model, args.resume_id, face, test_loaders, dfn_info, transfs, args, normalizer, write=args.write, shift=transfs_shift, device=args.device, head='test_ict_single', fraction=1, mask=None, latent_const=latent_const, iden_vec=None)
                eval(model, args.resume_id, face, test_loaders_, dfn_info_, transfs_, args, normalizer, write=args.write, shift=transfs_shift_, device=args.device, head='test_ict_random', fraction=1, mask=None, latent_const=latent_const_, iden_vec=None)
                eval(model, args.resume_id, face, test_loaders_live, dfn_info_live, transfs_live, args, normalizer, write=args.write, shift=transfs_shift_live, device=args.device, head='test_ict_live', fraction=1, mask=None, latent_const=latent_const_test_live, iden_vec=None)
                eval(model, args.resume_id, face, test_loaders_ss, dfn_info_ss, transfs_ss, args, normalizer, write=args.write, shift=transfs_shift_ss, device=args.device, head='test_ict_ss', fraction=1, mask=None, latent_const=latent_const_ss, iden_vec=None)
                eval(model, args.resume_id, face, test_loaders_holes, dfn_info_holes, transfs_holes, args, normalizer, write=args.write, shift=transfs_shift_holes, device=args.device, head='test_ict_holes', fraction=1, mask=None, latent_const=latent_const_holes, iden_vec=None)

            elif args.dataset == 'ALL':

                # eval(model, args.resume_id, face, test_loaders_live, dfn_info_live, transfs_live, args, normalizer, write=args.write, shift=transfs_shift_live, device=args.device, head='test_ict_live', fraction=1, mask=None, latent_const=latent_const_test_live, iden_vec=None)

                # eval(model, args.resume_id, face_mf, test_loaders_mf, dfn_info_mf, transfs_mf, args, normalizer, write=args.write, shift=transfs_shift_mf,  device=args.device, head='test_mf', fraction=1, mask=None, latent_const=args.latent_loss_type, iden_vec=None)
                # eval(model, args.resume_id, face_mf, val_loaders_mf, dfn_info_mf, transfs_mf, args, normalizer, write=args.write, shift=transfs_shift_mf,  device=args.device, head='mf_val', fraction=1, mask=None, latent_const=args.latent_loss_type, iden_vec=None)
                # eval(model, args.resume_id, face_mf, train_loaders_mf, dfn_info_mf, transfs_mf, args, normalizer, write=args.write, shift=transfs_shift_mf,  device=args.device, head='mf_train', fraction=1, mask=None, latent_const=args.latent_loss_type, iden_vec=None)
                # train(model, train_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=int(args.resume_id) + 1, head='mf', exp_weights=None, fix_mlp=args.fix_mlp)
                # train(model, val_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=int(args.resume_id) + 1, head='mf', exp_weights=None, fix_mlp=args.fix_mlp)
                train(model, test_loaders_mf, test_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=int(args.resume_id) + 1, head='mf_test', exp_weights=None, fix_mlp=args.fix_mlp)
                train(model, test_loaders_live, test_loaders_live, dfn_info_live, transfs_live, face, args, normalizer, transfs_shift_live, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=int(args.resume_id) + 1, head='ict_live_test', exp_weights=None, fix_mlp=args.fix_mlp)
                
                # eval(model, e, face, val_loaders, dfn_info, transfs, args,  normalizer, write=False, shift=transf_shift, device=args.device, fraction=1, latent_const=latent_const_val, iden_vec=iden_vec,  exp_weights=exp_weights, head=head+'_val')
                # eval(model, args.resume_id, face_mf, val_loaders_mf, dfn_info_mf, transfs_mf, args, normalizer, write=args.write, shift=transfs_shift_mf,  device=args.device, head='val_mf', fraction=1, mask=None, latent_const=None, iden_vec=None)

                # eval(model, args.resume_id, face_coma, test_loaders_coma, dfn_info_coma, transfs_coma, args, normalizer, write=args.write, shift=transfs_shift_coma, device=args.device, head='test_coma', fraction=1, mask=None, latent_const=None, iden_vec=None)

            elif args.dataset == 'ICT_live':
                train(model, test_loaders_mf, test_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=latent_const_test_live, latent_const_val=latent_const_test_live, epoch=int(args.resume_id) + 1, head=args.data_head+'_test', exp_weights=None, fix_mlp=args.fix_mlp)   
            elif args.dataset == 'biwi':
                train(model, test_loaders_mf, test_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=latent_const_test_live, latent_const_val=latent_const_test_live, epoch=int(args.resume_id) + 1, head=args.data_head+'_test', exp_weights=None, fix_mlp=args.fix_mlp)   
            else:      
                train(model, test_loaders_mf, test_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=int(args.resume_id) + 1, head=args.data_head+'_test', exp_weights=None, fix_mlp=args.fix_mlp)   
                # train(model, test_loaders_mf, val_loaders_mf, dfn_info_mf, transfs_mf, face_mf, args, normalizer, transfs_shift_mf, latent_const=args.latent_loss_type, latent_const_val=args.latent_loss_type, epoch=int(args.resume_id) + 1, head=args.data_head+'_test', exp_weights=None, fix_mlp=args.fix_mlp)   

   

```
