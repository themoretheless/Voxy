# Facial rendering source scan: confSmiPaper4/FacialRetargetingFrUnPC

Repository: https://github.com/confSmiPaper4/FacialRetargetingFrUnPC

Pinned source revision: 948467afdbd3c98cb377789964948f6420eb4573

Discovery topic: facial retargeting

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"deform": ["Deformation-Transfer-Matlab/1. BasicModules/build_correspondence.m", "Deformation-Transfer-Matlab/1. BasicModules/build_elementary_cell.m", "Deformation-Transfer-Matlab/1. BasicModules/deformation_transfer.m", "Deformation-Transfer-Matlab/1. BasicModules/dispMesh.m", "Deformation-Transfer-Matlab/1. BasicModules/init_marker.m", "Deformation-Transfer-Matlab/1. BasicModules/kd_tree/demo.m", "Deformation-Transfer-Matlab/1. BasicModules/kd_tree/kd_delete.m", "Deformation-Transfer-Matlab/1. BasicModules/kd_tree/kd_insert.m", "Deformation-Transfer-Matlab/1. BasicModules/kd_tree/kd_query.m", "Deformation-Transfer-Matlab/1. BasicModules/kd_tree/kd_tree.m", "Deformation-Transfer-Matlab/1. BasicModules/kd_tree/test.m", "Deformation-Transfer-Matlab/1. BasicModules/normPts.m", "Deformation-Transfer-Matlab/1. BasicModules/read_obj_file.m", "Deformation-Transfer-Matlab/1. BasicModules/similarity_fitting.m", "Deformation-Transfer-Matlab/1. BasicModules/v4_normal.m", "Deformation-Transfer-Matlab/1. BasicModules/write_obj_file.m", "Deformation-Transfer-Matlab/demo.m"], "rig": ["nicp-software_package/QtQuick/Controls/Styles/Base/images/arrow-right.png", "nicp-software_package/QtQuick/Controls/Styles/Base/images/arrow-right@2x.png", "nicp-software_package/QtQuick/Controls/Styles/Base/images/rightanglearrow.png"]}

## Source file: Deformation-Transfer-Matlab/1. BasicModules/deformation_transfer.m

{
  "path": "Deformation-Transfer-Matlab/1. BasicModules/deformation_transfer.m",
  "blob_sha": "7704206791cb12ddd8406f10fb0ff99392c2029b",
  "sha256": "5175c6b8e01c74e4a73ae197902332d047a2f5a9f8f6d01dd419e5effa029714",
  "lines": 186,
  "bytes": 5342,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 5,
    "deform": 4,
    "face": 3
  },
  "url": "https://github.com/confSmiPaper4/FacialRetargetingFrUnPC/blob/948467afdbd3c98cb377789964948f6420eb4573/Deformation-Transfer-Matlab/1.%20BasicModules/deformation_transfer.m",
  "function_signature_candidates": []
}

```text

function [ x nx ] = deformation_transfer( VS, FS, VT, FT, VS2, FS2, corres )
%Implementation of Deformation transfer
%
%   ????? ???? ???

lenFS = length(FS);
lenFT = length(FT);
SD = cell(lenFS, 1);

%%  Visualize
fprintf('Visualize input meshes \n');
set(gcf, 'Renderer', 'opengl');
figure; title('Source mesh');
trimesh(FS, VS(:, 1), VS(:, 2), VS(:, 3), ...
    'EdgeColor', 'none', 'FaceColor', [1 1 1], 'FaceLighting', 'phong');
light('Position',[0 0 1],'Style','infinite');
figure; title('Source deformed');
trimesh(FS2, VS2(:, 1), VS2(:, 2), VS2(:, 3), ...
    'EdgeColor', 'none', 'FaceColor', [1 1 1], 'FaceLighting', 'phong');
light('Position',[0 0 1],'Style','infinite');
figure; title('Target mesh');
trimesh(FT, VT(:, 1), VT(:, 2), VT(:, 3), ...
    'EdgeColor', 'none', 'FaceColor', [0.5 1 0.5], 'FaceLighting', 'phong');
light('Position',[0 0 1],'Style','infinite');


%%
[TS NS VS4 FS4]= v4_normal(VS, FS);
[TS2 NS2 VS42 FS42] = v4_normal(VS2, FS2);
[TT NT VT4 FT4] = v4_normal(VT, FT);
clear NS N2 NT;
cnt =0;
cnt_ok=0;
for i=1:lenFS
    if(sum(sum((inv(TS{i}) - pinv(TS{i})) > 1e-7))<1)    
        SD{i} = TS2{i} / TS{i};
    else
        SD{i} = TS2{i} / TS{i};
        cnt=cnt+1;
    end
end

displaceNorm=zeros(length(VS),1);
for i=1:length(VS)
    displaceNorm(i)=norm(VS(i,:)-VS2(i,:));
end




% %%
Nye=cell(length(VT),1);
Ninx=cell(length(VT),1);
for  i=1:lenFS
    Nye{FS(i,1)}=[Nye{FS(i,1)}   norm(SD{i}*VS(FS(i,1),:)'-VS(FS(i,1),:)') ];
    Ninx{FS(i,1)}=[Ninx{FS(i,1)} i];
    Nye{FS(i,2)}=[Nye{FS(i,2)}   norm(SD{i}*VS(FS(i,2),:)'-VS(FS(i,2),:)') ];
    Ninx{FS(i,2)}=[Ninx{FS(i,2)} i];
    Nye{FS(i,3)}=[Nye{FS(i,3)}   norm(SD{i}*VS(FS(i,3),:)'-VS(FS(i,3),:)') ];
    Ninx{FS(i,3)}=[Ninx{FS(i,3)} i];
end
threshold=max(displaceNorm);
for i=1:length(VT)
    for j=1:length(Nye{i})
        if  abs(Nye{i}(j)-min(Nye{i}))>10*threshold || abs(Nye{i}(j)-max(Nye{i}))>10*threshold
           % min(abs(distanceNorm{i}(j)-min(distanceNorm{i})),abs(distanceNorm{i}(j)-max(distanceNorm{i})))
            SD{Ninx{i}(j)}=eye(3);
        end

    end
end


% 
% for i=1:length(VT)
%     for j=1:length(distanceIdx{i})
%         if  min(abs(distanceNorm{i}(j)-min(distanceNorm{i})),abs(distanceNorm{i}(j)-max(distanceNorm{i})))>100
%             SD{distanceIdx{i}(j)}=eye(3);
%         end
%     end
% end




fprintf('cnt==%d,cnt_ok=%d\n',cnt,cnt_ok);

E = build_elementary_cell(TT, length(FT));

n_corres = sum(cellfun('length', corres));
n_non_corres = sum(cellfun('isempty',corres));
I = zeros(9*(n_corres+n_non_corres)*4, 3);
C = zeros(9*(n_corres+n_non_corres), 1);

% tic
fprintf('Transfer deformation..\n');
offset = 0;
offset2 = 0;
% reverseStr=[];
for i=1:lenFT
    lenCor = length(corres{i});
    Cor = corres{i};
    U = FT4(i, :);
    cnt=0;
    if lenCor
        for j=1:lenCor
            for k = 1:3 %x, y, z
                row = repmat((1:3)+ offset + (j-1)*3*3 + (k-1)*3, [4 1]);
                col1 = repmat( (U-1)*3 + k, [3 1] )';
                val1 = E{i}';
                % if(sum(sum((inv(TS{i}) - pinv(TS{i})) > 1e-15))<1)
                I((1:12)+ offset2 + (j-1)*3*3*4 + (k-1)*3*4,:) = [row(:) col1(:) val1(:)];
                
            end
            
            
            C((1:9)+ offset + (j-1)*9,1) = reshape(SD{Cor(j)}', [9 1]);
            
            
            
        end
        offset = offset + 3*3*lenCor;
        offset2 = offset2 + 3*3*lenCor*4;
    else
        for k=1:3
            row = repmat((1:3)+ offset + (k-1)*3, [4 1]);
            col1 = repmat( (U-1)*3 + k, [3 1] )';
            val1 = E{i}';
            I((1:12)+ offset2 + (k-1)*3*4,:) = [row(:) col1(:) val1(:)];
        end
        C((1:9)+ offset,1) = reshape(eye(3)', [9 1]);
        offset = offset + 3*3;
        offset2 = offset2 + 3*3*4;
    end
    % 	if ~mod(i, 10000)
    %         msg = sprintf('Processed %d/%d', i, size(FT,1));
    %         fprintf([reverseStr, msg]);
    %         reverseStr = repmat(sprintf('\b'), 1, length(msg));
    %     end
end

% toc
% I = I(I(:,1)>0,:);

M = sparse(I(:,1), I(:,2), I(:,3), 9*(n_corres + n_non_corres), 3*length(VT4));
Msize=size(M);
 yeInx=[1:1:Msize(2)];
 eeye=1e-7*sparse(yeInx,yeInx,ones(Msize(2),1));
 x = (M'*M+eeye)\(M'*C);
% x = (M'*M)\(M'*C);
x = reshape(x, [3 length(x)/3])';
x = x(1:length(VT), :);
fprintf('Finsiehd\n');
[temp nx] = v4_normal(x, FT);
% Aye2=zeros(26553,1);
% for i=1:26553
%     Aye2(i) = norm(x(i,:)-VT(i,:));
% end
% max(Aye2)


end
%%  W.r.t. source triangle

% for i=1:lenFS
%     lenCor = length(corres{i});
%     Cor = corres{i};
%     for j=1:lenCor
%         U = FT4(Cor(j), :);
%         for k = 1:3 %x, y, z
% %             row = repmat((1:3)+(i-1)*3*3*prelenCor + (j-1)*3*3 + (k-1)*3, [4 1]);
%             row = repmat((1:3)+ offset + (j-1)*3*3 + (k-1)*3, [4 1]);
%             col1 = repmat( (U-1)*3 + k, [3 1] )';
%             val1 = E{Cor(j)}';
%             I((1:12)+ offset2 + (j-1)*3*3*4 + (k-1)*3*4,:) = [row(:) col1(:) val1(:)];
%         end
%         C((1:9)+ offset + (j-1)*9,1) = reshape(SD{i}', [9 1]);
%     end
%     offset = offset + 3*3*lenCor;
%     offset2 = offset2 + 3*3*lenCor*4;
% 	if ~mod(i, 10000)
%         msg = sprintf('Processed %d/%d', i, size(FS,1));
%         fprintf([reverseStr, msg]);
%         reverseStr = repmat(sprintf('\b'), 1, length(msg));
%     end
% end


```

## Source file: Deformation-Transfer-Matlab/1. BasicModules/normPts.m

{
  "path": "Deformation-Transfer-Matlab/1. BasicModules/normPts.m",
  "blob_sha": "bd1824ad268c5842f7e752aed4452d42b992c05d",
  "sha256": "4bcabfc803695687c6b288f01ced792b0aec6f1508552145b3356d2d54ad4dea",
  "lines": 26,
  "bytes": 906,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/confSmiPaper4/FacialRetargetingFrUnPC/blob/948467afdbd3c98cb377789964948f6420eb4573/Deformation-Transfer-Matlab/1.%20BasicModules/normPts.m",
  "function_signature_candidates": []
}

```text

function [ x, T] = normPts( x, m, s )
%Normalize coordinate values of x.
%   Note. Transformation is done in homogeneous coordinate
%   Note. k should be less than 3
%   x : n x k vector (n is number of elements, k is dimension in inhomogeneous coordinate)
%   mu : scalar which represents target center of normalization
%   s : scalar which represents average scale of normalization
n = size(x,1);
k = size(x,2);
T = eye(k+1);
mu = mean(x);
% mu = [mean(x(:,1)) mean(x(:,2)) mean(x(:,3))];
T(1:k, k+1) = (m-mu)';
% Average distance to mean location
mean_distance = mean(sum(sqrt((x - repmat(mu, [n 1])).^2),2));
% mean_distance = mean(sqrt((x(:,1)-mu(1)).^2+(x(:,2)-mu(2)).^2)+(x(:,3)-mu(3)).^2);
scale = s/mean_distance;
     
T = scale*T; T(k+1,k+1) = 1;
% T(1:3, 1:4) = sx*T(1:3, 1:4);
% Convert to homogeneous coordinate (Assuming w = 1)
x = [x ones(length(x),1)];
x = (T*x')';
x = x(:,1:k);    
end



```

## Source file: Deformation-Transfer-Matlab/1. BasicModules/dispMesh.m

{
  "path": "Deformation-Transfer-Matlab/1. BasicModules/dispMesh.m",
  "blob_sha": "227f8bf527d67de12f72bf48b9e5ce0759d65c0e",
  "sha256": "1bccc9c4ed1ad6716aa06db2e4ffdb352884e8382f7548bae9dbd6d1f1c22e42",
  "lines": 77,
  "bytes": 2118,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 7
  },
  "url": "https://github.com/confSmiPaper4/FacialRetargetingFrUnPC/blob/948467afdbd3c98cb377789964948f6420eb4573/Deformation-Transfer-Matlab/1.%20BasicModules/dispMesh.m",
  "function_signature_candidates": []
}

```text

function dispMesh (shp, tl, color, alpha, normal, name, saveMesh)
% Note, shp has nx3 elements and tl has mx3 elements
% Note, If saveMesh activated, mesh is saved as obj file format
if nargin < 4
    alpha = 1;
end
if nargin < 5
    saveMesh = false;    
end

FV.vertices = shp;
FV.faces = tl;
if size(color,1) < 2
    patch(FV, 'facecolor', color, 'edgecolor', 'none', 'vertexnormalsmode', 'auto', 'FaceAlpha', alpha);
else
    patch(FV, 'FaceVertexCData',color,'facecolor', 'interp', 'edgecolor', 'none', 'vertexnormalsmode', 'auto', 'FaceAlpha', alpha);
end
camlight('headlight');
lighting phong;
material dull;
axis vis3d
axis equal;
% title(name);

if saveMesh
    write_obj_file(shp, tl, normal, sprintf('%s%s',name,'.obj'));
    frpintf('Mesh file %s is saved\n', name);
end



end

%% Deprecated (From Basel Face Model)
% function dispFace (shp, tl, rp)
% % 	if size(shp,1) > size(shp,2)
% %        shp = shp'; 
% %     end
%     if size(shp,2) == 1
%         shp = reshape(shp, [ 3 prod(size(shp))/3 ])'; 
%     end
% 
% 	set(gcf, 'Renderer', 'opengl');
% % 	fig_pos = get(gcf, 'Position');
% % 	fig_pos(3) = rp.width;
% % 	fig_pos(4) = rp.height;
% % 	set(gcf, 'Position', fig_pos);
% 	set(gcf, 'ResizeFcn', @resizeCallback);
% 
% 	mesh_h = trimesh(tl, shp(:, 1), shp(:, 3), shp(:, 2),...
%         'Edgecolor', 'none', 'FaceColor', 'interp', 'FaceVertexCData', ...
%         repmat(rp.color, [size(shp,1),1]),'FaceLighting', 'phong');
% 
% % 	set(gca, ...
% % 		'DataAspectRatio', [ 1 1 1 ], ...
% % 		'PlotBoxAspectRatio', [ 1 1 1 ], ...
% % 		'Units', 'pixels', ...
% % 		'GridLineStyle', 'none', ...
% % 		'Position', [ 0 0 fig_pos(3) fig_pos(4) ], ...
% % 		'Visible', 'off', 'box', 'off', ...
% % 		'Projection', 'perspective' ...
% % 		); 
% 	
% % 	set(gcf, 'Color', [ 0 0 0 ]); 
% 	view(180 + rp.phi * 180 / pi, 0);
% 
% 	material([.5, .5, .1 1  ])
% 	camlight('headlight');
% 	
% function resizeCallback (obj, eventdata)
% 	
% 	fig = gcbf;
% 	fig_pos = get(fig, 'Position');
% 
% 	axis = findobj(get(fig, 'Children'), 'Tag', 'Axis.Head');
% 	set(axis, 'Position', [ 0 0 fig_pos(3) fig_pos(4) ]);
	


```

## Source file: Deformation-Transfer-Matlab/1. BasicModules/v4_normal.m

{
  "path": "Deformation-Transfer-Matlab/1. BasicModules/v4_normal.m",
  "blob_sha": "6981eb5508a1abc009cab47dbbca6d9b97ed9cdd",
  "sha256": "b8bb172d03be98c59ba369f772cf0fbe3b291cd6d9be0356acae8f32681d2f9f",
  "lines": 34,
  "bytes": 928,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/confSmiPaper4/FacialRetargetingFrUnPC/blob/948467afdbd3c98cb377789964948f6420eb4573/Deformation-Transfer-Matlab/1.%20BasicModules/v4_normal.m",
  "function_signature_candidates": []
}

```text

function [T N V F] = v4_normal(Vert, Face)
%Convert 3 vertices representation to 4 vertices with representation of
%normal, 
%   input
%   Vert : n x 3 matrix
%   Face : m x 3 matrix
%   output
%   V : (m+n) x 3 matrix (vertices with added approximated normals)
%   T : n x 1 cell containing 3 x 3 matrices (representation of triangles)

f1 = Face(:,1); f2 = Face(:,2); f3 = Face(:,3);

e1 = Vert(f2,:) - Vert(f1,:);
e2 = Vert(f3,:) - Vert(f1,:);

c = 1.*cross(e1, e2, 2);
c_norm = sqrt(c(:,1).^2+c(:,2).^2+c(:,3).^2);
c_norm(c_norm==0)=1;

N = c./repmat(c_norm, [1 3]);
v4 = Vert(f1,:) + c;%yyyeeee./repmat(c_norm, [1 3]);
size(v4)
V = [Vert ;v4];
F = Face;
F(:, 4) = size(Vert,1)+find(F(:,3));
T = cell(size(F,1), 1);

for i = 1:size(F,1)
    T{i,1} = [(V(F(i,2), :) - V(F(i,1),:))' (V(F(i,3),:) - V(F(i,1),:))' (V(F(i,4),:) - V(F(i,1),:))'];
end

% quiver3(V(F(:,1),1), V(F(:,2),2), V(F(:,3),3), N(:,1), N(:,2), N(:,3));

end

```
