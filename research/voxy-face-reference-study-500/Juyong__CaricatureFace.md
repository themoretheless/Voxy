# Facial rendering source scan: Juyong/CaricatureFace

Repository: https://github.com/Juyong/CaricatureFace

Pinned source revision: c139c1ae2c0755d54a03bf9e91db4ddeae4296d7

Discovery topic: face reconstruction 3D

Description: The source code for paper "Landmark Detection and 3D Face Reconstruction for Caricature using a Nonlinear Parametric Model".

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["cariface.py", "introduction/CaricatureFace.pdf", "toy_example/mean_face.obj"]}

## Source file: cariface.py

{
  "path": "cariface.py",
  "blob_sha": "98bc46c6ec0284128bae12f43b1ea56933f89074",
  "sha256": "ba4a90bed48cc1b3b3984dd6918cab0f198b78c3995245d8831fd5dde69b1db5",
  "lines": 354,
  "bytes": 20078,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 1,
    "face": 2
  },
  "url": "https://github.com/Juyong/CaricatureFace/blob/c139c1ae2c0755d54a03bf9e91db4ddeae4296d7/cariface.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def CalculateLandmark2D(euler_angle, scale, trans, points, landmark_index, landmark_num):"
    },
    {
      "line": 59,
      "text": "def __init__(self, vertex_num, pca_pri):"
    },
    {
      "line": 71,
      "text": "def forward(self, x):"
    },
    {
      "line": 79,
      "text": "def init_numbers(self, landmark_num=68, vertex_num=6144, device_num=0):"
    },
    {
      "line": 84,
      "text": "def init_data(self, data_path=\"data/\"):"
    },
    {
      "line": 129,
      "text": "def load_train_data(self, image_path, landmark_path, vertex_path, size=32, workers=6):"
    },
    {
      "line": 133,
      "text": "def load_test_data(self, image_path, landmark_path, lrecord_path, vrecord_path, workers=6):"
    },
    {
      "line": 137,
      "text": "def load_model(self, resnet34_lr=1e-4, mynet1_lr=1e-5, mynet2_lr=1e-8,"
    },
    {
      "line": 160,
      "text": "def train(self, epoch, lambda_land=1, lambda_srt=1e-1):"
    },
    {
      "line": 266,
      "text": "def test(self):"
    },
    {
      "line": 350,
      "text": "def save_model(self, epoch, save_path=\"record/\"):"
    }
  ]
}

```text

from datagen import TrainSet, TestSet

import os
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
import torch.utils.data as data
import torchvision
import time

"""
    CalculateLandmark2D:
        'euler_angle' is a euler_angle tensor with size (batch_size, 3),
        'scale' is a scale tensor with size (batch_size, 1),
        'trans' is a translation matrix with size (batch_size, 2),
        'points' is a point tensor with size (batch_size, 3, vertex_num),
        'landmark_index' is a long tensor with size (landmark_num),
        'landmark_num' is the number of landmarks
"""
def CalculateLandmark2D(euler_angle, scale, trans, points, landmark_index, landmark_num):
    batch_size = euler_angle.shape[0]
    theta = euler_angle[:,0].reshape(-1,1,1)
    phi = euler_angle[:,1].reshape(-1,1,1)
    psi = euler_angle[:,2].reshape(-1,1,1)
    one = torch.ones(batch_size,1,1).to(euler_angle.device)
    zero = torch.zeros(batch_size, 1, 1).to(euler_angle.device)
    rot_x = torch.cat((
        torch.cat((one,zero,zero),1),
        torch.cat((zero,theta.cos(), theta.sin()),1),
        torch.cat((zero,-theta.sin(),theta.cos()),1),
        ),2)
    rot_y = torch.cat((
        torch.cat((phi.cos(),zero,-phi.sin()),1),
        torch.cat((zero,one, zero),1),
        torch.cat((phi.sin(),zero,phi.cos()),1),
        ),2)
    rot_z = torch.cat((
        torch.cat((psi.cos(),psi.sin(),zero),1),
        torch.cat((-psi.sin(),psi.cos(), zero),1),
        torch.cat((zero,zero,one),1),
        ),2)
    rot = torch.bmm(rot_z, torch.bmm(rot_y,rot_x))    
    mu = points
    idx = landmark_index
    vertex = torch.index_select(mu, 2, idx)
    xy_t = torch.bmm(scale.reshape(-1,1,1)*rot[:,0:2,:].reshape(-1,2,3), vertex)
    xy_t += trans.reshape(-1,2,1).expand_as(xy_t)
    landmarks = torch.cat((xy_t[:,0,:].reshape(-1,1), xy_t[:,1,:].reshape(-1,1)), 1).reshape(-1,landmark_num,2)

    return landmarks

"""
    MyNet:
        'vertex_num' is the number of vertices of 3D meshes,
        'pca_pri' is the PCA basis to initialize the last FC layer
"""
class MyNet(nn.Module):
    def __init__(self, vertex_num, pca_pri):
        super(MyNet, self).__init__()
        self.fc1 = nn.Linear(in_features=94, out_features=226, bias=True)
        torch.nn.init.kaiming_normal_(self.fc1.weight.data)
        torch.nn.init.zeros_(self.fc1.bias.data)
        self.fc2 = nn.Linear(in_features=226, out_features=226, bias=True)
        torch.nn.init.kaiming_normal_(self.fc2.weight.data)
        torch.nn.init.zeros_(self.fc2.bias.data)
        self.fc3 = nn.Linear(in_features=226, out_features=vertex_num*9, bias=True)
        self.fc3.weight.data = pca_pri.t()
        torch.nn.init.zeros_(self.fc3.bias.data)

    def forward(self, x):
        active_opt = nn.ReLU(True)
        x = active_opt(self.fc1(x))
        x = self.fc2(x)
        x = self.fc3(x)
        return x

class CariFace():
    def init_numbers(self, landmark_num=68, vertex_num=6144, device_num=0):
        self.landmark_num = landmark_num
        self.vertex_num = vertex_num
        self.device_num = device_num

    def init_data(self, data_path="data/"):
        """
            related document
        """
        pca_pri_path = data_path + "pca_pri.npy" # the PCA basis of latent deformation representation (DR)
        logR_S_mean_path = data_path + "logR_S_mean.npy" # the mean of DR
        A_pinv_path = data_path + "A_pinv.npy" # the matrix for solving vertices' coordinates from DR
        warehouse_vertex_path = data_path + "P_.npy" # vertices' coordinates of the mean face
        connect_path = data_path + "connect.txt" # the connected relation of vertices
        one_ring_center_ids_path = data_path + "one_ring_center_ids.txt" # the ids of 1-ring centers
        one_ring_ids_path = data_path + "one_ring_ids.txt" # the ids of vertices connected to 1-ring centers
        one_ring_lbweights_path = data_path + "one_ring_lbweights.npy" # the Laplacian weights of each connection
        landmark_index_path = data_path + "best_68.txt" # the ids of 68 3D landmarks
        # load pca_pri and logR_S_mean
        self.pca_pri = torch.from_numpy(np.load(pca_pri_path)).float().to(self.device_num)
        self.logR_S_mean = torch.from_numpy(np.load(logR_S_mean_path)).float().to(self.device_num)
        # A_pinv and warehouse_0's vertices
        self.A_pinv = torch.from_numpy(np.load(A_pinv_path)).to(self.device_num).float()
        self.P_ = torch.from_numpy(np.load(warehouse_vertex_path)).to(self.device_num).float()
        # connects and landmarks' indices
        self.one_ring_center_ids = torch.from_numpy(np.loadtxt(one_ring_center_ids_path)).to(self.device_num).long()
        self.one_ring_ids = torch.from_numpy(np.loadtxt(one_ring_ids_path)).to(self.device_num).long()
        self.one_ring_lbweights = torch.from_numpy(np.load(one_ring_lbweights_path)).to(self.device_num).float()
        file = open(connect_path, 'r')
        lines = file.readlines()
        file.close()
        connects = []
        connects_num = 0
        for line in lines:
            line = line.strip('\n')
            line = line.strip(' ')
            line = line.split(' ')
            connects.append(line)
        for i in range(self.vertex_num):
            connects_num += len(connects[i])
        conn_i = torch.zeros(2,connects_num).long()
        conn_k = 0
        for i in range(self.vertex_num):
            for j in range(len(connects[i])):
                conn_i[:,conn_k] = torch.LongTensor([i, conn_k])
                conn_k += 1
        conn_v = torch.ones(connects_num).long()
        self.connect_ = torch.sparse.FloatTensor(conn_i, conn_v, torch.Size([self.vertex_num,connects_num])).to(self.device_num).float()
        self.landmark_index = torch.from_numpy(np.loadtxt(landmark_index_path)).long().to(self.device_num)
    
    def load_train_data(self, image_path, landmark_path, vertex_path, size=32, workers=6):
        trainset = TrainSet(image_path, landmark_path, vertex_path, self.landmark_num, self.vertex_num)
        self.train_loader = torch.utils.data.DataLoader(trainset, batch_size=size, shuffle=True, num_workers=workers)

    def load_test_data(self, image_path, landmark_path, lrecord_path, vrecord_path, workers=6):
        testset = TestSet(image_path, landmark_path, lrecord_path, vrecord_path)
        self.test_loader = torch.utils.data.DataLoader(testset, batch_size=1, shuffle=False, num_workers=workers)

    def load_model(self, resnet34_lr=1e-4, mynet1_lr=1e-5, mynet2_lr=1e-8,
                    use_premodel=True, model1_path="model/resnet34_adam.pth", model2_path="model/mynet_adam.pth"):
        self.model1 = torchvision.models.resnet34(pretrained=True)
        fc_features = self.model1.fc.in_features
        self.model1.fc = nn.Linear(in_features=fc_features, out_features=100)
        self.model1 = self.model1.to(self.device_num)
        self.model2 = MyNet(self.vertex_num, self.pca_pri).to(self.device_num)
        if use_premodel == True:
            ck1 = torch.load(model1_path)
            ck2 = torch.load(model2_path)
            # ck1 = torch.load(model1_path, map_location={'cuda:0':'cuda:3'})
            # ck2 = torch.load(model2_path, map_location={'cuda:0':'cuda:3'})
            self.model1.load_state_dict(ck1['net'])
            self.model2.load_state_dict(ck2['net'])
        # optimizer
        self.optimizer1 = torch.optim.Adam(self.model1.parameters(), lr = resnet34_lr)
        self.optimizer2 = torch.optim.Adam([
            {'params':self.model2.fc1.parameters(), 'lr':mynet1_lr},
            {'params':self.model2.fc2.parameters(), 'lr':mynet1_lr}])
        self.optimizer3 = torch.optim.Adam(self.model2.fc3.parameters(), lr = mynet2_lr)
        # loss function
        self.loss_fn = nn.MSELoss().to(self.device_num)

    def train(self, epoch, lambda_land=1, lambda_srt=1e-1):
        start = time.time()
        self.model1.train()
        self.model2.train()
        total_loss = 0.0
        total_num = 0
        loss_1 = 0.0
        loss_2 = 0.0
        loss_3 = 0.0
        with torch.autograd.set_detect_anomaly(True):
            for batch_idx, (img, landmark, vertex) in enumerate(self.train_loader):
                img, landmark, vertex = img.to(self.device_num).float(), landmark.to(self.device_num).float(), vertex.to(self.device_num).float()
                output = self.model1(img)
                alpha = output[:,0:94] # alpha parameter
                scale = output[:, 94] # scale parameter
                euler_angle = output[:, 95:98] # euler_angle parameter
                trans = output[:, 98:100] # trans parameter
                
                # solve logR_S and T
                delta = self.model2(alpha)
                logR_S = delta + self.logR_S_mean
                logR_S = logR_S.reshape(-1, 9)
                rparas = logR_S[:,0:3]
                sparas = logR_S[:,3:]
                angles = rparas.norm(2,1)
                indices = angles.nonzero()
                tRs = torch.zeros_like(logR_S)
                tRs[:,0::4] = 1.0
                if indices.numel() > 0 and indices.numel() < angles.numel():
                    indices = indices[:,0]
                    crparas = rparas[indices]/angles[indices].reshape(-1,1)
                    temp = (1-torch.cos(angles[indices]).reshape(-1,1))
                    tempS = torch.sin(angles[indices]).reshape(-1,1)
                    tRs[indices, 0::4] = torch.cos(angles[indices]).reshape(-1,1) + temp * crparas * crparas
                    tRs[indices, 1] = temp.view(-1) * crparas[:,0] * crparas[:,1] - tempS.view(-1) * crparas[:,2]
                    tRs[indices, 2] = temp.view(-1) * crparas[:,0] * crparas[:,2] + tempS.view(-1) * crparas[:,1]
                    tRs[indices, 3] = temp.view(-1) * crparas[:,0] * crparas[:,1] + tempS.view(-1) * crparas[:,2]
                    tRs[indices, 5] = temp.view(-1) * crparas[:,1] * crparas[:,2] - tempS.view(-1) * crparas[:,0]
                    tRs[indices, 6] = temp.view(-1) * crparas[:,0] * crparas[:,2] - tempS.view(-1) * crparas[:,1]
                    tRs[indices, 7] = temp.view(-1) * crparas[:,1] * crparas[:,2] + tempS.view(-1) * crparas[:,0]
                elif indices.numel()==angles.numel():
                    rparas = rparas/angles.reshape(-1,1)
                    temp = (1-torch.cos(angles).reshape(-1,1))
                    tempS = torch.sin(angles).reshape(-1,1)
                    tRs[:, 0::4] = torch.cos(angles).reshape(-1,1) + temp * rparas * rparas
                    tRs[:, 1] = temp.view(-1) * rparas[:,0] * rparas[:,1] - tempS.view(-1) * rparas[:,2]
                    tRs[:, 2] = temp.view(-1) * rparas[:,0] * rparas[:,2] + tempS.view(-1) * rparas[:,1]
                    tRs[:, 3] = temp.view(-1) * rparas[:,0] * rparas[:,1] + tempS.view(-1) * rparas[:,2]
                    tRs[:, 5] = temp.view(-1) * rparas[:,1] * rparas[:,2] - tempS.view(-1) * rparas[:,0]
                    tRs[:, 6] = temp.view(-1) * rparas[:,0] * rparas[:,2] - tempS.view(-1) * rparas[:,1]
                    tRs[:, 7] = temp.view(-1) * rparas[:,1] * rparas[:,2] + tempS.view(-1) * rparas[:,0]
                tSs = torch.zeros_like(logR_S)
                tSs[:, 0:3] = sparas[:, 0:3]
                tSs[:, 3] = sparas[:, 1]
                tSs[:, 4:6] = sparas[:, 3:5]
                tSs[:, 6] = sparas[:, 2]
                tSs[:, 7] = sparas[:, 4]
                tSs[:, 8] = sparas[:, 5]
                Ts = torch.bmm(tRs.reshape(-1,3,3), tSs.reshape(-1,3,3)).reshape(-1, self.vertex_num, 9)
                
                # solve points
                Tijs = Ts.index_select(1, self.one_ring_center_ids) + Ts.index_select(1, self.one_ring_ids)
                pijs = self.P_.index_select(0, self.one_ring_center_ids) - self.P_.index_select(0, self.one_ring_ids)
                temp = torch.zeros((Tijs.size()[0],3,Tijs.size()[1]), device=Ts.device)
                temp[:,0,:] = torch.sum(Tijs[:,:,0:3]*(pijs*self.one_ring_lbweights.reshape(-1,1)), 2)
                temp[:,1,:] = torch.sum(Tijs[:,:,3:6]*(pijs*self.one_ring_lbweights.reshape(-1,1)), 2)
                temp[:,2,:] = torch.sum(Tijs[:,:,6:9]*(pijs*self.one_ring_lbweights.reshape(-1,1)), 2)
                temp = temp.reshape(-1, self.one_ring_ids.numel()).t().clone()
                RHS = torch.spmm(self.connect_, temp)
                points = (torch.matmul(self.A_pinv, RHS)).t()
                points_mean = torch.mean(points, 1).reshape(points.shape[0],-1)
                points -= points_mean.expand_as(points)
                points = points.reshape(-1,3,self.vertex_num)
                loss_geo = 10 * self.loss_fn(points, vertex)

                # solve landmarks
                lands_2d = CalculateLandmark2D(euler_angle, scale, trans, points, self.landmark_index, self.landmark_num)
                loss_land = 1e-4 * self.loss_fn(lands_2d, landmark)
                lands = CalculateLandmark2D(euler_angle, scale, trans, vertex, self.landmark_index, self.landmark_num)
                loss_srt = 1e-4 * self.loss_fn(lands, landmark)
                loss_land_srt = 0.0
                if (epoch-1) // 500 == 0:
                    loss_land_srt = lambda_srt * loss_srt
                else:
                    loss_land_srt = lambda_land * loss_land

                # back propagation
                self.optimizer1.zero_grad()
                self.optimizer2.zero_grad()
                self.optimizer3.zero_grad()
                loss_geo.backward(retain_graph=True)
                if (epoch-1) // 10000 > 0:
                    self.optimizer3.step()
                self.optimizer2.step()
                loss_land_srt.backward()
                self.optimizer1.step()

                loss_1 += loss_geo.item() * img.shape[0]
                loss_2 += loss_land.item() * img.shape[0]
                loss_3 += loss_srt.item() * img.shape[0]
                total_loss += (loss_geo.item() + loss_land.item() + loss_srt.item()) * img.shape[0]
                total_num += img.shape[0]
            end = time.time()
            print("epoch_"+str(epoch)+":\ttime: "+str(end-start)+"s")
            print("\tloss_geo: " + "{:3.6f}".format(loss_1/total_num) + "\tloss_land: " + "{:3.6f}".format(loss_2/total_num) + "\tloss_srt: " + "{:3.6f}".format(loss_3/total_num))
    
    def test(self):
        start = time.time()
        self.model1.eval()
        self.model2.eval()
        loss_test = 0.0
        total_num = 0
        with torch.no_grad():
            for img, landmark, lrecord, vrecord in self.test_loader:
                img, landmark = img.to(self.device_num).float(), landmark.to(self.device_num).float()
                output = self.model1(img)
                alpha = output[:, 0:94]
                scale = output[:, 94]
                euler_angle = output[:, 95:98]
                trans = output[:, 98:100]

                # solve logR_S and T
                delta = self.model2(alpha)
                logR_S = delta + self.logR_S_mean
                logR_S = logR_S.reshape(-1,9)
                rparas = logR_S[:,0:3]
                sparas = logR_S[:,3:]
                angles = rparas.norm(2,1)
                indices = angles.nonzero()
                tRs = torch.zeros_like(logR_S)
                tRs[:,0::4] = 1.0
                if indices.numel() > 0 and indices.numel() < angles.numel():
                    indices = indices[:,0]
                    crparas = rparas[indices]/angles[indices].reshape(-1,1)
                    temp = (1-torch.cos(angles[indices]).reshape(-1,1))
                    tempS = torch.sin(angles[indices]).reshape(-1,1)
                    tRs[indices, 0::4] = torch.cos(angles[indices]).reshape(-1,1) + temp * crparas * crparas
                    tRs[indices, 1] = temp.view(-1) * crparas[:,0] * crparas[:,1] - tempS.view(-1) * crparas[:,2]
                    tRs[indices, 2] = temp.view(-1) * crparas[:,0] * crparas[:,2] + tempS.view(-1) * crparas[:,1]
                    tRs[indices, 3] = temp.view(-1) * crparas[:,0] * crparas[:,1] + tempS.view(-1) * crparas[:,2]
                    tRs[indices, 5] = temp.view(-1) * crparas[:,1] * crparas[:,2] - tempS.view(-1) * crparas[:,0]
                    tRs[indices, 6] = temp.view(-1) * crparas[:,0] * crparas[:,2] - tempS.view(-1) * crparas[:,1]
                    tRs[indices, 7] = temp.view(-1) * crparas[:,1] * crparas[:,2] + tempS.view(-1) * crparas[:,0]
                elif indices.numel()==angles.numel():
                    rparas = rparas/angles.reshape(-1,1)
                    temp = (1 - torch.cos(angles).reshape(-1,1))
                    tempS = torch.sin(angles).reshape(-1,1)
                    tRs[:, 0::4] = torch.cos(angles).reshape(-1,1) + temp * rparas * rparas
                    tRs[:, 1] = temp.view(-1) * rparas[:,0] * rparas[:,1] - tempS.view(-1) * rparas[:,2]
                    tRs[:, 2] = temp.view(-1) * rparas[:,0] * rparas[:,2] + tempS.view(-1) * rparas[:,1]
                    tRs[:, 3] = temp.view(-1) * rparas[:,0] * rparas[:,1] + tempS.view(-1) * rparas[:,2]
                    tRs[:, 5] = temp.view(-1) * rparas[:,1] * rparas[:,2] - tempS.view(-1) * rparas[:,0]
                    tRs[:, 6] = temp.view(-1) * rparas[:,0] * rparas[:,2] - tempS.view(-1) * rparas[:,1]
                    tRs[:, 7] = temp.view(-1) * rparas[:,1] * rparas[:,2] + tempS.view(-1) * rparas[:,0]
                tSs = torch.zeros_like(logR_S)
                tSs[:, 0:3] = sparas[:, 0:3]
                tSs[:, 3] = sparas[:, 1]
                tSs[:, 4:6] = sparas[:, 3:5]
                tSs[:, 6] = sparas[:, 2]
                tSs[:, 7] = sparas[:, 4]
                tSs[:, 8] = sparas[:, 5]
                Ts = torch.bmm(tRs.reshape(-1,3,3), tSs.reshape(-1,3,3)).reshape(-1, self.vertex_num, 9)

                # solve points
                Tijs = Ts.index_select(1, self.one_ring_center_ids) + Ts.index_select(1, self.one_ring_ids)
                pijs = self.P_.index_select(0, self.one_ring_center_ids) - self.P_.index_select(0, self.one_ring_ids)
                temp = torch.zeros((Tijs.size()[0], 3, Tijs.size()[1]), device=Ts.device)
                temp[:,0,:] = torch.sum(Tijs[:,:,0:3]*(pijs*self.one_ring_lbweights.reshape(-1,1)), 2)
                temp[:,1,:] = torch.sum(Tijs[:,:,3:6]*(pijs*self.one_ring_lbweights.reshape(-1,1)), 2)
                temp[:,2,:] = torch.sum(Tijs[:,:,6:9]*(pijs*self.one_ring_lbweights.reshape(-1,1)), 2)
                temp = temp.reshape(-1, self.one_ring_ids.numel()).t().clone()
                RHS = torch.spmm(self.connect_, temp)
                points = (torch.matmul(self.A_pinv, RHS)).t()
                points_mean = torch.mean(points, 1).reshape(points.shape[0], -1)
                points -= points_mean.expand_as(points)
                points = points.reshape(-1,3,self.vertex_num)

                # solve landmarks
                lands_2d = CalculateLandmark2D(euler_angle, scale, trans, points, self.landmark_index, self.landmark_num)
                loss_land = 1e-4 * self.loss_fn(lands_2d, landmark)

                loss_test += loss_land.item() * img.shape[0]
                total_num += img.shape[0]
                np.save(str(lrecord[0]), lands_2d.reshape(self.landmark_num,2).data.cpu().numpy())
                np.save(str(vrecord[0]), points.reshape(3,self.vertex_num).data.cpu().numpy())
            end = time.time()
            print("result: "+ "{:3.6f}".format(loss_test/total_num)+"\ttime: "+str(end-start)+"s")
            print("\tloss_land: " + "{:3.6f}".format(loss_test/total_num))
            print('\n')

    def save_model(self, epoch, save_path="record/"):
        state1 = {'net':self.model1.state_dict(), 'optimizer':self.optimizer1.state_dict(), 'epoch':epoch}
        state2 = {'net':self.model2.state_dict(), 'optimizer2':self.optimizer2.state_dict(), 'optimizer3':self.optimizer3.state_dict(), 'epoch':epoch}
        torch.save(state1, save_path+"resnet34_adam_"+str(epoch)+".pth")
        torch.save(state2, save_path+"mynet_adam_"+str(epoch)+".pth")


```

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "a2d3ca448f65e2b4f8cd8992f9ebcde711800042",
  "sha256": "64a07fced44510c03ee0e7e01ed884be8edc59ab6f35eb8361ed2adebdd91222",
  "lines": 30,
  "bytes": 1507,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/Juyong/CaricatureFace/blob/c139c1ae2c0755d54a03bf9e91db4ddeae4296d7/train.py",
  "function_signature_candidates": []
}

```text

import cariface
from train_options import TrainOptions

if __name__ == '__main__':
    opt = TrainOptions().parse()

    model = cariface.CariFace()
    model.init_numbers(opt.landmark_num, opt.vertex_num, opt.device_num)
    model.init_data(opt.data_path)
    if opt.if_train == True:
        model.load_train_data(opt.train_image_path, opt.train_landmark_path, opt.train_vertex_path,
                                opt.batch_size, opt.num_workers)
        model.load_test_data(opt.test_image_path, opt.test_landmark_path,
                                opt.test_lrecord_path, opt.test_vrecord_path, opt.num_workers)
        model.load_model(opt.resnet34_lr, opt.mynet1_lr, opt.mynet2_lr, opt.use_premodel,
                            opt.model1_path, opt.model2_path)
        model.test()
        for epoch in range(1, opt.total_epoch+1):
            model.train(epoch, opt.lambda_land, opt.lambda_srt)
            if epoch % opt.test_frequency == 0:
                model.test()
            if epoch % opt.save_frequency == 0:
                model.save_model(epoch, opt.save_model_path)
        model.save_model(opt.total_epoch, opt.save_model_path)
    else:
        model.load_test_data(opt.test_image_path, opt.test_landmark_path,
                                opt.test_lrecord_path, opt.test_vrecord_path, opt.num_workers)
        model.load_model(opt.resnet34_lr, opt.mynet1_lr, opt.mynet2_lr, opt.use_premodel,
                            opt.model1_path, opt.model2_path)
        model.test()

```

## Source file: datagen.py

{
  "path": "datagen.py",
  "blob_sha": "dd0fdfbd34b5741f2a15bf7ebd42b87fe17b38b2",
  "sha256": "ec02c9a0478a4feeae822fbff9ef0b7ef8d16c87f536b4e321aba8d2b5f80bcc",
  "lines": 168,
  "bytes": 6371,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "face": 5,
    "lip": 6
  },
  "url": "https://github.com/Juyong/CaricatureFace/blob/c139c1ae2c0755d54a03bf9e91db4ddeae4296d7/datagen.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def clip_rot_flip_face(image, landmarks, scale, alpha):"
    },
    {
      "line": 70,
      "text": "def __init__(self, image_path, landmark_path, vertex_path, landmark_num=68, vertex_num=6144):"
    },
    {
      "line": 95,
      "text": "def __getitem__(self,index):"
    },
    {
      "line": 117,
      "text": "def __len__(self):"
    },
    {
      "line": 126,
      "text": "def __init__(self, image_path, landmark_path, lrecord_path, vrecord_path):"
    },
    {
      "line": 150,
      "text": "def __getitem__(self,index):"
    },
    {
      "line": 167,
      "text": "def __len__(self):"
    }
  ]
}

```text

"""
This file is for data preprocessing
"""
import os
import cv2
from PIL import Image
import numpy as np
import torch
from torch.utils import data
from torchvision import transforms

train_transform=transforms.Compose([
    transforms.ColorJitter(brightness=0.5, contrast=0.5, saturation=0.5, hue=0.5),
    transforms.ToTensor(),
    transforms.Normalize(mean=(0.5,0.5,0.5),std=(0.5,0.5,0.5))
])

test_transform=transforms.Compose([
    transforms.ToTensor(),
    transforms.Normalize(mean=(0.5,0.5,0.5),std=(0.5,0.5,0.5))
])

def clip_rot_flip_face(image, landmarks, scale, alpha):
    """
        given a face and its 68 landmarks,
        clip and rotate it with 'scale' and 'alpha',
        and return scaled and rotated face with new 68 landmarks.
    """
    # calculate x_min, x_max, y_min, y_max, center_x, center_y and box_w
    x_min = np.min(landmarks[:,0])
    x_max = np.max(landmarks[:,0])
    y_min = np.min(landmarks[:,1])
    y_max = np.max(landmarks[:,1])
    center_x = (x_min + x_max) / 2
    center_y = (y_min + y_max) / 2
    box_w = max(x_max-x_min, y_max-y_min)

    # calculate x_0, y_0, x_1 and y_1
    x_0 = np.random.uniform(center_x-(0.5+0.75*(scale-1.0))*box_w, center_x-(0.5+0.25*(scale-1.0))*box_w)
    y_0 = np.random.uniform(center_y-(0.5+0.75*(scale-1.0))*box_w, center_y-(0.5+0.25*(scale-1.0))*box_w)
    x_0 = round(max(0.0, x_0))
    y_0 = round(max(0.0, y_0))
    x_1 = round(min(image.shape[1], x_0+scale*box_w))
    y_1 = round(min(image.shape[0], y_0+scale*box_w))
    
    """
        process image and landmarks
    """
    # Random clip for image
    new_image = image[int(y_0):int(y_1), int(x_0):int(x_1)]
    new_image = cv2.resize(new_image, (224,224), interpolation=cv2.INTER_LINEAR)
    # Random rotate for image
    rot_mat = cv2.getRotationMatrix2D((112, 112), alpha, 1) # obtain RotationMatrix2D with fixed center (112, 112)
    new_image = cv2.warpAffine(new_image, rot_mat, (224, 224)) # obtain rotated image
    
    # Random clip for corresponding landmarks
    new_landmarks = landmarks
    new_landmarks[:,0] = (new_landmarks[:,0] - x_0) * 224 / (x_1 - x_0)
    new_landmarks[:,1] = (new_landmarks[:,1] - y_0) * 224 / (y_1 - y_0)
    # Random rotate for corresponding landmarks
    new_landmarks = np.asarray([(rot_mat[0][0]*x+rot_mat[0][1]*y+rot_mat[0][2],
    rot_mat[1][0]*x+rot_mat[1][1]*y+rot_mat[1][2]) for (x, y) in new_landmarks]) # adjust new_landmarks after rotating
    
    return new_image, new_landmarks

class TrainSet(data.Dataset):
    """
        construct trainset, including images, landmarks, and vertices of ground-truth meshes
    """
    def __init__(self, image_path, landmark_path, vertex_path, landmark_num=68, vertex_num=6144):
        """
            initialize TrainSet
        """
        file = open(image_path,'r')
        image = file.readlines()
        file.close()
        file = open(landmark_path,'r')
        landmark = file.readlines()
        file.close()
        file = open(vertex_path,'r')
        vertex = file.readlines()
        file.close()
        self.image = [os.path.join(k.strip('\n')) for k in image]
        self.landmark = [os.path.join(k.strip('\n')) for k in landmark]
        self.vertex = [os.path.join(k.strip('\n')) for k in vertex]
        self.transforms = train_transform
        if len(self.image) == len(self.landmark) == len(self.vertex):
            self.num_samples = len(self.image)
        self.landmark_arrays = np.zeros((self.num_samples, landmark_num, 2), np.float32)
        self.vertex_arrays = np.zeros((self.num_samples, 3, vertex_num), np.float32)
        for i in range(self.num_samples):
            self.landmark_arrays[i,...] = np.load(self.landmark[i])
            self.vertex_arrays[i,...] = np.load(self.vertex[i])

    def __getitem__(self,index):
        # get image
        image_path = self.image[index]
        image = cv2.imread(image_path)
        # get landmark
        landmark = self.landmark_arrays[index,...]
        # get vertex
        vertex = self.vertex_arrays[index,...]

        """
            preprocess image and landmark
        """
        # image, landmark = clip_rot_flip_face(image, landmark, 1.2, np.random.uniform(-10.0, 10.0))
        image, landmark = clip_rot_flip_face(image, landmark, 1.2, 5*np.random.randint(-1, 2))
        image = Image.fromarray(cv2.cvtColor(image,cv2.COLOR_BGR2RGB))
        if self.transforms:
            image = self.transforms(image)
        else:
            image = torch.from_numpy(image)

        return image, landmark, vertex

    def __len__(self):
        return self.num_samples

class TestSet(data.Dataset):
    """
        construct test set,
        including images, landmarks for calculating errors
        and 'lrecord', 'vrecord' for recording estimated landmarks and vertices of recovered meshes.
    """
    def __init__(self, image_path, landmark_path, lrecord_path, vrecord_path):
        """
            initialize Test Set
        """
        file = open(image_path,'r')
        image = file.readlines()
        file.close()
        file = open(landmark_path, 'r')
        landmark = file.readlines()
        file.close()
        file = open(lrecord_path,'r')
        lrecord = file.readlines()
        file.close()
        file = open(vrecord_path,'r')
        vrecord = file.readlines()
        file.close()
        self.image = [os.path.join(k.strip('\n')) for k in image]
        self.landmark = [os.path.join(k.strip('\n')) for k in landmark]
        self.lrecord = [os.path.join(k.strip('\n')) for k in lrecord]
        self.vrecord = [os.path.join(k.strip('\n')) for k in vrecord]
        self.transforms = test_transform
        if len(self.image) == len(self.landmark) == len(self.lrecord) == len(self.vrecord):
            self.num_samples = len(self.image)

    def __getitem__(self,index):
        # get image
        image_path = self.image[index]
        image = Image.open(image_path)
        if self.transforms:
            image = self.transforms(image)
        else:
            image = torch.from_numpy(image)
        # get landmark
        landmark_path = self.landmark[index]
        landmark = np.load(landmark_path)
        # get record
        lrecord = self.lrecord[index]
        vrecord = self.vrecord[index]

        return image, landmark, lrecord, vrecord

    def __len__(self):
        return self.num_samples

```

## Source file: train_options.py

{
  "path": "train_options.py",
  "blob_sha": "88221e4aaa6953dbbf9ed6f02a91adb803d34740",
  "sha256": "e7e89cdcdde5cfb6ea4c24ce62246a9ec1338c0487cb6597c44d72c9b4b98749",
  "lines": 66,
  "bytes": 4290,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 30
  },
  "url": "https://github.com/Juyong/CaricatureFace/blob/c139c1ae2c0755d54a03bf9e91db4ddeae4296d7/train_options.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def __init__(self):"
    },
    {
      "line": 8,
      "text": "def initialize(self, parser):"
    },
    {
      "line": 53,
      "text": "def gather_options(self):"
    },
    {
      "line": 62,
      "text": "def parse(self):"
    }
  ]
}

```text

import argparse
import os

class TrainOptions():
    def __init__(self):
        self.initialized = False

    def initialize(self, parser):
        self.initialized = True
        parser.add_argument('--landmark_num', type=int, default=68, help='landmark number')
        parser.add_argument('--vertex_num', type=int, default=6144, help='vertex number of 3D mesh')
        parser.add_argument('--device_num', type=int, default=0, help='gpu id')
        parser.add_argument('--data_path', type=str, default="data/", help='path of related data')
        if_train_parser = parser.add_mutually_exclusive_group(required=False)
        if_train_parser.add_argument('--train', dest='if_train', action='store_true') # train mode
        if_train_parser.add_argument('--no_train', dest='if_train', action='store_false') # test mode
        parser.set_defaults(if_train=True)
        parser.add_argument('--train_image_path', type=str, default="exp/train_images.txt", help='train images path')
        parser.add_argument('--train_landmark_path', type=str, default="exp/train_landmarks.txt",
                            help='train landmarks path')
        parser.add_argument('--train_vertex_path', type=str, default="exp/train_vertex.txt",
                            help='train vertex path')
        parser.add_argument('--batch_size', type=int, default=32, help='train batch size')
        parser.add_argument('--num_workers', type=int, default=6, help='threads for loading data')
        parser.add_argument('--test_image_path', type=str, default="exp/test_images.txt", help='test images path')
        parser.add_argument('--test_landmark_path', type=str, default="exp/test_landmarks.txt",
                            help='test landmarks path')
        parser.add_argument('--test_lrecord_path', type=str, default="exp/test_lrecord.txt",
                            help='path to save estimated landmarks')
        parser.add_argument('--test_vrecord_path', type=str, default="exp/test_vrecord.txt",
                            help='path to save estimated coordinates of vertices')
        parser.add_argument('--resnet34_lr', type=float, default=1e-4, help='learning rate of ResNet34')
        parser.add_argument('--mynet1_lr', type=float, default=1e-5,
                            help='learning rate of the first and second FC layers of MyNet')
        parser.add_argument('--mynet2_lr', type=float, default=1e-8,
                            help='learning rate of the last FC layer of MyNet')
        use_premodel_parser = parser.add_mutually_exclusive_group(required=False)
        use_premodel_parser.add_argument('--premodel', dest='use_premodel', action='store_true') # use pretrained model
        use_premodel_parser.add_argument('--no_premodel', dest='use_premodel', action='store_false') # no pretrained model
        parser.set_defaults(use_premodel=True)
        parser.add_argument('--model1_path', type=str, default="model/resnet34_adam.pth",
                            help='the pretrained model of ResNet34 structure')
        parser.add_argument('--model2_path', type=str, default="model/mynet_adam.pth",
                            help='the pretrained model of MyNet structure')
        parser.add_argument('--total_epoch', type=int, default=1000, help='number of total training epoch')
        parser.add_argument('--lambda_land', type=float, default=1, help='weight of landmark loss')
        parser.add_argument('--lambda_srt', type=float, default=1e-1, help='weight of srt loss')
        parser.add_argument('--test_frequency', type=int, default=100, help='frequency for testing')
        parser.add_argument('--save_frequency', type=int, default=200, help='frequency for saving models')
        parser.add_argument('--save_model_path', type=str, default="record/", help='path to save models')
        return parser

    def gather_options(self):
        # initialize parser with basic options
        if not self.initialized:
            parser = argparse.ArgumentParser(
                formatter_class=argparse.ArgumentDefaultsHelpFormatter)
            parser = self.initialize(parser)
        self.parser = parser
        return parser.parse_args()
    
    def parse(self):

        opt = self.gather_options()
        self.opt = opt
        return self.opt

```
