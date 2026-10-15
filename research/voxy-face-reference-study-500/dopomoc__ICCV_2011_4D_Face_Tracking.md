# Facial rendering source scan: dopomoc/ICCV_2011_4D_Face_Tracking

Repository: https://github.com/dopomoc/ICCV_2011_4D_Face_Tracking

Pinned source revision: 85e0b09b9533a31af9fce6b69b33dee6f4095634

Discovery topic: FACS face

Description: Demo code (Matlab) for ICCV 2011 paper A FACS Valid 3D Dynamic Action Unit Database With Applications to 3D Dynamic Morphable Facial Modeling. D Cosker, E Krumhuber, A Hilton  IEEE International Conference on Computer Vision (ICCV), 2296-2303, 2011.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: scripts/tsai.m

{
  "path": "scripts/tsai.m",
  "blob_sha": "61547252d67ef4c519160f34bcbe15a81c41cce3",
  "sha256": "356db8f617e69665cd1bc1bab84165d54f66137d0d6af337852a900311b0fed6",
  "lines": 52,
  "bytes": 1196,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/dopomoc/ICCV_2011_4D_Face_Tracking/blob/85e0b09b9533a31af9fce6b69b33dee6f4095634/scripts/tsai.m",
  "function_signature_candidates": []
}

```text

function p1 = tsai(Cal_1C,v1)

%
% function p1 = tsai(Cal_1C,v1)
%
% Given a 3D point v1, find its 2D projection p1.
% Requires camera paraneter matrix Cal_1C .
%
% (c) Darren Cosker, 2008.
%
% Based on an earlier C++ implementation by 3DMD Ltd
%

v1.x = v1.x - Cal_1C.X;
v1.y = v1.y - Cal_1C.Y;
v1.z = v1.z - Cal_1C.Z;

p1.x = (Cal_1C.M(1,1)*v1.x) + (Cal_1C.M(2,1)*v1.y) + (Cal_1C.M(3,1)*v1.z);
p1.y = (Cal_1C.M(1,2)*v1.x) + (Cal_1C.M(2,2)*v1.y) + (Cal_1C.M(3,2)*v1.z);
p1.z = (Cal_1C.M(1,3)*v1.x) + (Cal_1C.M(2,3)*v1.y) + (Cal_1C.M(3,3)*v1.z);

p1.x = (Cal_1C.f * p1.x)/p1.z;
p1.y = (Cal_1C.f * p1.y)/p1.z;

R = sqrt((p1.x.*p1.x)+(p1.y.*p1.y));

r=R;
r2 = r*r;
epsilon = 0.01/1000.0;

fprime = 1+r2*(3*Cal_1C.K + r2*5*Cal_1C.K2);

if(fprime<1.0e-4)
    disp('fprime is low');
end
error=10000000;

while((abs(R)>0.001) & (abs(error/R)>epsilon))
    distortion = r*(1 + r2*(Cal_1C.K+r2*Cal_1C.K2));
    r = r - (distortion - R)/fprime;
    error = (R - distortion)/fprime;
end

p1.x = p1.x * (r/R);
p1.y = p1.y * (r/R);

p1.x = p1.x/Cal_1C.x_;
p1.y = p1.y/Cal_1C.y_;

p1.x = p1.x + Cal_1C.a;
p1.y = p1.y + Cal_1C.b;
p1.y = Cal_1C.is(1,2) - p1.y;

```

## Source file: scripts/interp2d.m

{
  "path": "scripts/interp2d.m",
  "blob_sha": "8c3f735f79776b02163c2c469c25e8a70dddfa74",
  "sha256": "7bc130747b1444fb7993a09c82b09905de5f2c974e0fba8178e6d68bc0e00fd3",
  "lines": 55,
  "bytes": 1455,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/dopomoc/ICCV_2011_4D_Face_Tracking/blob/85e0b09b9533a31af9fce6b69b33dee6f4095634/scripts/interp2d.m",
  "function_signature_candidates": []
}

```text

function [imgw, imgwr, map] = interp2d(X, Y, img, Xwr, Ywr, outH, outW, interp)


%% initialization
color = size(img,3);
imgwr=zeros(outH,outW,color); % output image
% window dimension for filling
maxhw = fix((interp.radius-1)/2);
% input image params
color = size(img,3);
imgH  = size(img,1);
imgW  = size(img,2);

%% Round warping coordinates
% rounding warped coordinates
Xwi = round(Xwr);
Ywi = round(Ywr);

% Bound warped coordinates to image frame
Xwi = max(min(Xwi,outH),1);
Ywi = max(min(Ywi,outW),1);

% Convert 2D coordinates into 1D indices
fiw = sub2ind([outH,outW],Xwi,Ywi); % warped coordinates
fip = sub2ind([imgH,imgW],X,Y); % input

%% Warped image construction with holes
% Note: This mapping doesn't treat 2 or more points (from source image)
%     mapping to same point (on warped image) differently than 1-1 mapping.
o_r=zeros(outH,outW);
for colIx = 1:color,
    img_r=img(:,:,colIx);
    o_r(fiw)=img_r(fip);
    imgwr(:,:,colIx)=o_r;
end


%% Filling of holes

% Create a binary image with 0s for holes and 1s for mappings
map=zeros(outH,outW);
map(fiw)=1;

% Fill holes
if strcmp(interp.method,'nearest') % Median of nearest neigbor filling
    imgw = nearestInterp(imgwr, map, maxhw); % fill

elseif strcmp(interp.method,'invdist')% inverse distance weighted interpolation
    imgw = idwMvInterp(imgwr, map, maxhw, interp.power); % fill
else
    imgw = imgwr;
end

return;



```

## Source file: scripts/tpswarp_pre.m

{
  "path": "scripts/tpswarp_pre.m",
  "blob_sha": "9bf062f861229d3fa931fa1af3b58563ce8f330a",
  "sha256": "88b06f03458bf398ce8bd3dd8f6de2302b6fb3314ad837f5b6f6ed9339edc919",
  "lines": 96,
  "bytes": 2695,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/dopomoc/ICCV_2011_4D_Face_Tracking/blob/85e0b09b9533a31af9fce6b69b33dee6f4095634/scripts/tpswarp_pre.m",
  "function_signature_candidates": []
}

```text

function [X, Y, Xw, Yw, outH, outW, interp] = tpswarp_pre(img, outDim, Zp, Zs, interp)
%

% Thin plate spline warp

%% Initialization
NPs = size(Zp,1); % number of landmark points

imgH = size(img,1); % height
imgW = size(img,2); % width

outH = outDim(2);
outW = outDim(1);

% landmark in input
Xp = Zp(:,1)';
Yp = Zp(:,2)';

% landmark in output (homologous)
Xs = Zs(:,1)';
Ys = Zs(:,2)';

%% Algebra of Thin-plate splines

% Compute thin-plate spline mapping [W|a1 ax ay] using landmarks
[wL]=computeWl(Xp, Yp, NPs);
wY = [Xs(:) Ys(:); zeros(3,2)]; % Y = ( V| 0 0 0)'   where V = [G] where G is landmark homologous (nx2) ; Y is col vector of length (n+3)
wW = inv(wL)*wY; % (W|a1 ax ay)' = inv(L)*Y

% Thin-plate spline mapping (Map all points in the plane)
% f(x,y) = a1 + ax * x + ay * y + SUM(wi * U(|Pi-(x,y)|)) for i = 1 to n
[Xw, Yw]=tpsMap(wW, imgH, imgW, Xp, Yp, NPs);

%% Warping

% input grid for warping
[X Y] = meshgrid(1:imgH,1:imgW); % HxW


return

%% [L] = [[K P];[P' 0]]
% np - number of landmark points
% (xp, yp) - coordinate of landmark points
function [wL]=computeWl(xp, yp, np)

rXp = repmat(xp(:),1,np); % 1xNp to NpxNp
rYp = repmat(yp(:),1,np); % 1xNp to NpxNp

wR = sqrt((rXp-rXp').^2 + (rYp-rYp').^2); % compute r(i,j)

wK = radialBasis(wR); % compute [K] with elements U(r)=r^2 * log (r^2)
wP = [ones(np,1) xp(:) yp(:)]; % [P] = [1 xp' yp'] where (xp',yp') are n landmark points (nx2)
wL = [wK wP;wP' zeros(3,3)]; % [L] = [[K P];[P' 0]]

return


%% Mapping: f(x,y) = a1 + ax * x + ay * y + SUM(wi * U(|Pi-(x,y)|)) for i = 1 to n
% np - number of landmark points
% (xp, yp) - coordinate of landmark points
function [Xw, Yw]=tpsMap(wW, imgH, imgW, xp, yp, np)

[X Y] = meshgrid(1:imgH,1:imgW); % HxW
X=X(:)'; % convert to 1D array by reading columnwise (NWs=H*W)
Y=Y(:)'; % convert to 1D array (NWs)
NWs = length(X); % total number of points in the plane

% all points in plane
rX = repmat(X,np,1); % Np x NWs
rY = repmat(Y,np,1); % Np x NWs

% landmark points
rxp = repmat(xp(:),1,NWs); % 1xNp to Np x NWs
ryp = repmat(yp(:),1,NWs); % 1xNp to Np x NWs

% Mapping Algebra
wR = sqrt((rxp-rX).^2 + (ryp-rY).^2); % distance measure r(i,j)=|Pi-(x,y)|

wK = radialBasis(wR); % compute [K] with elements U(r)=r^2 * log (r^2)
wP = [ones(NWs,1) X(:) Y(:)]'; % [P] = [1 x' y'] where (x',y') are n landmark points (nx2)
wL = [wK;wP]'; % [L] = [[K P];[P' 0]]

Xw  = wL*wW(:,1); % [Pw] = [L]*[W]
Yw  = wL*wW(:,2); % [Pw] = [L]*[W]

return

%% k=(r^2) * log(r^2)
function [ko]=radialBasis(ri)

r1i = ri;
r1i(find(ri==0))=realmin; % Avoid log(0)=inf
ko = 2*(ri.^2).*log(r1i);

return


```

## Source file: scripts/nearestInterp.m

{
  "path": "scripts/nearestInterp.m",
  "blob_sha": "3beaba10603c6ee92ba65f956c54df3f7bf0e9cc",
  "sha256": "139d896343607e98c2ed955a5983e43b67f4315098aa1d7c02f6cc7bb98ca0cd",
  "lines": 44,
  "bytes": 1123,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/dopomoc/ICCV_2011_4D_Face_Tracking/blob/85e0b09b9533a31af9fce6b69b33dee6f4095634/scripts/nearestInterp.m",
  "function_signature_candidates": []
}

```text

function out = nearestInterp(imgw, map, maxhw )

% Fill holes using nearest neighbor (median filter of neighbors) interpolation


outH  = size(imgw,1);
outW  = size(imgw,2);
out = imgw;

[yi_arr, xi_arr] = find(map==0); % Find locations needing fill
if isempty(yi_arr) == false
    color = size(imgw,3);
    for ix = 1:length(yi_arr),

        xi = xi_arr(ix);
        yi = yi_arr(ix);

        % Find min window which has non-hole neighbors
        nz = false;
        for h = 1:maxhw,
            yixL=max(yi-h,1);
            yixU=min(yi+h,outH);
            xixL=max(xi-h,1);
            xixU=min(xi+h,outW);

            if isempty(find(map(yixL:yixU,xixL:xixU), 1)) == false
                nz = true;
                break;
            end
        end

        % use the median of non-hole neighbors in the window for filling
        if nz == true,
            for colIx = 1:color,
                win=imgw(yixL:yixU, xixL:xixU, colIx);
                out(yi,xi,colIx) = median(win(find(map(yixL:yixU, xixL:xixU)~=0)));
            end
        end

    end
end

return;



```
