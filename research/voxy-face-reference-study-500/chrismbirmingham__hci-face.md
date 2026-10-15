# Facial rendering source scan: chrismbirmingham/hci-face

Repository: https://github.com/chrismbirmingham/hci-face

Pinned source revision: 42df92fb959f19d05cb8346bcccbed67cedd1445

Discovery topic: facial animation

Description: Human-Computer Interaction Facial Animation and Conversation Engine

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["frontend/face/src/components/cordial/eyes.js", "frontend/face/src/components/default/eyes.js", "frontend/face/src/components/qt/eyes.js"], "mouth": ["frontend/face/src/components/cordial/mouth.js", "frontend/face/src/components/default/mouth.js", "frontend/face/src/components/qt/mouth.js"], "face": ["docs/tutorials/advanced/customizing_face.md", "frontend/WoZ/public/face.png", "frontend/face/.gitignore", "frontend/face/README.md", "frontend/face/npm-README.md", "frontend/face/package-lock.json", "frontend/face/package.json", "frontend/face/public/demo.png", "frontend/face/public/face.png", "frontend/face/public/favicon.ico", "frontend/face/public/index.html", "frontend/face/public/logo192.png", "frontend/face/public/logo512.png", "frontend/face/public/manifest.json", "frontend/face/public/robots.txt", "frontend/face/src/App.css", "frontend/face/src/App.js", "frontend/face/src/App.test.js", "frontend/face/src/components/Form.js", "frontend/face/src/components/Head.js", "frontend/face/src/components/cordial/brow.js", "frontend/face/src/components/cordial/eyes.js", "frontend/face/src/components/cordial/mouth.js", "frontend/face/src/components/default/brow.js", "frontend/face/src/components/default/eyes.js", "frontend/face/src/components/default/mouth.js", "frontend/face/src/components/qt/brow.js", "frontend/face/src/components/qt/eyes.js", "frontend/face/src/components/qt/mouth.js", "frontend/face/src/helpers/Behaviors.js"], "lip": ["backend/app/utils/tts/backends/resources/aws_example_code/polly_lipsync.py", "docs/reference/utils/tts/backends/resources/aws_example_code/polly_lipsync.md"]}

## Source file: frontend/face/src/components/qt/eyes.js

{
  "path": "frontend/face/src/components/qt/eyes.js",
  "blob_sha": "f8d4d05628309473fb0653981902bfcd412416c4",
  "sha256": "f171c9bb7682aa1bd6b6008978edc4c8c55f531f2a53bfb62ea6838d09d56eef",
  "lines": 150,
  "bytes": 5015,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 21,
    "rig": 22,
    "face": 2
  },
  "url": "https://github.com/chrismbirmingham/hci-face/blob/42df92fb959f19d05cb8346bcccbed67cedd1445/frontend/face/src/components/qt/eyes.js",
  "function_signature_candidates": []
}

```text

import { animated, useSpring } from "react-spring";

var ey, ex, eright_x, eright_y;
var px, py, pr, sr;
var ulc, llc;
var lr, ud;




function moveEye(new_px, new_py, r, eright_x, eright_y){
    if (new_px < 0){
        px = Math.min(Math.abs(new_px), eright_x-r) * -1
    }
    else {px = Math.min(new_px, eright_x-r)}

    if (new_py < 0){
        py = Math.min(Math.abs(new_py), eright_y-r) * -1
    }
    else {py = Math.min(new_py, eright_y-r)}

    return [px, py]


}

function circlePath(center_x, center_y, r){
    var dis = [
        "M", center_x-2, center_y+r*.8,
        "A", r*.7,r, "0","1","0",center_x-2, center_y-r*.8,
        "A", r*.7,r, "0","0","0",center_x-2, center_y+r*.8,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}

function bezelCirclePath(center_x, center_y, r){
    var dis = [
        "M", center_x-r, center_y,
        "C", center_x-r, center_y+r, center_x+r, center_y+r, center_x+r, center_y,// bottom half
        "C", center_x+r, center_y-r, center_x-r, center_y-r, center_x-r, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}

function lowerLidPath(center_x, center_y, eright_x, eright_y, c){
    // v: 1 is bottom half, with higher c = more closed. 0 is upper half with lower c more closed
    var dis = [
        "M", center_x-eright_x*1.1, center_y,
        "C", center_x-eright_x*1.1, center_y+eright_y, center_x+eright_x*1.1, center_y+eright_y, center_x+eright_x*1.1, center_y,
        "Q", center_x, center_y+eright_y*(1-c)*2, center_x-eright_x*1.1, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}
function upperLidPath(center_x, center_y, eright_x, eright_y, c){
    // desired behavior is 0 is open, 1 is closed and 2 is all the way down
    var dis = [
        "M", center_x-eright_x*1.1, center_y,
        "C", center_x-eright_x*1.1, center_y-eright_y, center_x+eright_x*1.1, center_y-eright_y, center_x+eright_x*1.1, center_y,
        "Q", center_x, center_y-eright_y*(1-c)*2, center_x-eright_x*1.1, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}


function Eye({position, eyeAU}) {
    [ey, ex] = [position.y+3, position.x];
    // Eye Params
    [eright_x, eright_y, pr, sr] = [10, 10, 6, 1];
    [llc, ulc] = [0,0];
    [lr, ud] = [0,0];

    var au5_upper_lid_raiser = eyeAU.au5_upper_lid_raiser;
    var au6_cheek_raiser = eyeAU.au6_cheek_raiser;
    var au7_lid_tightener = eyeAU.au7_lid_tightener;
    var au41_lid_droop = eyeAU.au41_lid_droop;
    var au42_slit = eyeAU.au42_slit;
    var au43_eyes_closed = eyeAU.au43_eyes_closed;
    var au44_squint = eyeAU.au44_squint;
    var au45_blink = eyeAU.au45_blink;
    var au61_left = eyeAU.au61_left;
    var au62_right = eyeAU.au62_right;
    var au63_up = eyeAU.au63_up;
    var au64_down = eyeAU.au64_down;


    lr = lr + au62_right - au61_left;
    ud = ud + au63_up - au64_down;

    ulc = ulc - au5_upper_lid_raiser*.5;
    llc = llc + au6_cheek_raiser*.5;

    ulc=ulc + au7_lid_tightener*.3;
    llc=llc + au7_lid_tightener*.3;

    ulc = ulc + au41_lid_droop*.7;

    ulc = ulc + au42_slit*.6;
    llc = llc + au42_slit*.6;

    ulc = ulc + au43_eyes_closed;
    llc = llc + au43_eyes_closed;

    ulc = ulc + au44_squint*.3;
    llc = llc + au44_squint*.2;

    ulc = ulc + au45_blink*2;

    [px, py] = moveEye(lr * eright_x, ud * -1 * eright_y, pr, eright_x, eright_y)

    let ed = bezelCirclePath(ex,ey, eright_x)
    let bd = bezelCirclePath(ex,ey, eright_x+3)

    let pd = circlePath(ex+px, ey+py, pr)// Pupil
    let sd = circlePath(ex+px-pr*.1, ey+py-pr*.3, sr*1.2) // Pupil shine


    let lld = lowerLidPath(ex+px, ey+py, eright_x, eright_y, llc)
    let uld = upperLidPath(ex+px, ey+py, eright_x, eright_y, ulc)

    const animationProps = useSpring({
        border: bd,
        eyeball: ed,
        pupil: pd,
        sparkle: sd,
        lowerLid: lld,
        upperLid: uld
    }
        )

    return (
        <g>
            {/* <animated.path className="eyeball" d={animationProps.eyeball} stroke="#000" fill="#FFF" fillOpacity="0.95" strokeWidth=".15" strokeLinecap="round"/> */}
            <animated.path className="pupil" d={animationProps.pupil} stroke="#777" fill="#111" fillOpacity="0.95" strokeWidth="1" strokeLinecap="round"/>
            <animated.path className="sparkle" d={animationProps.sparkle} stroke="#000" fill="#CCC" fillOpacity="1" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="lowerLid" d={animationProps.lowerLid} stroke="#000" fill={position.faceColor} fillOpacity=".995" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="upperLid" d={animationProps.upperLid} stroke="#000" fill={position.faceColor} fillOpacity=".995" strokeWidth="0" strokeLinecap="round"/>
            {/* <animated.path className="border" d={animationProps.border} stroke="#D7E4F5" fill="#FFF" fillOpacity="0" strokeWidth="5" strokeLinecap="round"/> */}
        </g>
    )
}

export default Eye;

```

## Source file: frontend/face/src/components/qt/mouth.js

{
  "path": "frontend/face/src/components/qt/mouth.js",
  "blob_sha": "c2cb028f60a5da3f5272898c3c2e73092695346b",
  "sha256": "1062f89533a67bc4ea6c2d885db1e697fd372f494029e455329c1bb10dcc111b",
  "lines": 286,
  "bytes": 9286,
  "selection_score": 18,
  "keyword_line_counts": {
    "jaw": 5,
    "mouth": 34,
    "rig": 37,
    "lip": 50
  },
  "url": "https://github.com/chrismbirmingham/hci-face/blob/42df92fb959f19d05cb8346bcccbed67cedd1445/frontend/face/src/components/qt/mouth.js",
  "function_signature_candidates": []
}

```text

import { animated, useSpring } from "react-spring";


let center_x, center_y
let right_x, right_y, left_x, left_y, top_x, top_y, bottom_x, bottom_y
let lbottom_x, lbottom_y, rbottom_x, rbottom_y, ltop_x, ltop_y, rtop_x, rtop_y;
let chin_y, nose_y;

function AdjustMouth (position, mouthAU) 
{
    var scale = position.auScaler;
    var au10_raise_upper = mouthAU.au10_raise_upper*scale;
    var au12_lip_corners_out = mouthAU.au12_lip_corners_out*scale;
    var au13_cheek_puffer = mouthAU.au13_cheek_puffer*scale;
    var au14_dimpler = mouthAU.au14_dimpler*scale;
    var au15_lip_corner_depr = mouthAU.au15_lip_corner_depr*scale;
    var au16_lower_lip_depr = mouthAU.au16_lower_lip_depr*scale;
    var au17_chin_raiser = mouthAU.au17_chin_raiser*scale;
    var au18_lip_pucker = mouthAU.au18_lip_pucker*scale;
    var au20_lip_stretcher = mouthAU.au20_lip_stretcher*scale;
    var au22_lip_funneler = mouthAU.au22_lip_funneler*scale;
    var au23_lip_tightener = mouthAU.au23_lip_tightener*scale;
    var au24_lip_pressor = mouthAU.au24_lip_pressor*scale;
    var au25_lips_part = mouthAU.au25_lips_part*scale;
    var au26_jaw_drop = mouthAU.au26_jaw_drop*scale;
    var au27_mouth_stretch = mouthAU.au27_mouth_stretch*scale;
    var au28_lip_suck = mouthAU.au28_lip_suck*scale;

    // SET BASELINES
    [center_x, center_y] = [position.x, position.y]; // location of the mouth - all coordinates relative to center
    const width_factor=.4;

    [right_x, right_y] = [25*width_factor, 1]; // lip right corner x and y
    [left_x, left_y] = [-25*width_factor, 1]; // lip left corner x and y

    [top_x, top_y] = [0, 1.5]; // lip top x and y
    [rtop_x, rtop_y] = [12.5*width_factor, 1.8]; // right side top line control point
    [ltop_x, ltop_y] = [-12.5*width_factor, 1.8]; // left side top line control point

    [bottom_x, bottom_y] = [0, -2];  // lip bottom x and y
    [rbottom_x, rbottom_y] = [12.5*width_factor, -1.4]; // right side bottom line control point
    [lbottom_x, lbottom_y] = [-12.5*width_factor, -1.4]; // left side bottom line control point
    chin_y = -20
    nose_y = 12


    
    // Adjust baselines
    top_y = top_y + au10_raise_upper
    rtop_y = rtop_y + au10_raise_upper
    ltop_y = ltop_y + au10_raise_upper
    
    right_x = right_x + au12_lip_corners_out
    right_y = right_y + au12_lip_corners_out*1.2
    left_x = left_x - au12_lip_corners_out
    left_y = left_y + au12_lip_corners_out*1.2
    
    center_y = center_y - au13_cheek_puffer*.7
    right_x = right_x + au13_cheek_puffer*.5
    right_y = right_y + au13_cheek_puffer*.6
    right_x = right_x - au13_cheek_puffer*.5
    left_y = left_y + au13_cheek_puffer*.6
    
    right_x = right_x + au14_dimpler
    left_x = left_x - au14_dimpler
    
    right_y = right_y - au15_lip_corner_depr
    left_y = left_y - au15_lip_corner_depr
    
    bottom_y = bottom_y - au16_lower_lip_depr
    rbottom_y = rbottom_y - au16_lower_lip_depr
    lbottom_y = lbottom_y - au16_lower_lip_depr
    
    bottom_y = bottom_y + au17_chin_raiser
    rbottom_y = rbottom_y + au17_chin_raiser
    lbottom_y = lbottom_y + au17_chin_raiser
    chin_y = chin_y + au17_chin_raiser
    
    left_x = left_x + au18_lip_pucker*1.5
    right_x = right_x - au18_lip_pucker*1.5
    bottom_y = bottom_y - au18_lip_pucker*.333
    
    right_x = right_x + au20_lip_stretcher
    right_y = right_y - au20_lip_stretcher
    left_x = left_x - au20_lip_stretcher
    left_y = left_y - au20_lip_stretcher
    
    left_x = left_x + au22_lip_funneler
    right_x = right_x - au22_lip_funneler
    bottom_y = bottom_y - au22_lip_funneler*.333
    top_y = top_y + au22_lip_funneler
    
    left_x = left_x + au23_lip_tightener*2
    right_x = right_x - au23_lip_tightener*2
    
    top_y = top_y - au24_lip_pressor*.5
    bottom_y = bottom_y + au24_lip_pressor*.25
    
    top_y = top_y + au25_lips_part*.5
    bottom_y = bottom_y - au25_lips_part*.25
    
    bottom_y = bottom_y - au26_jaw_drop
    rbottom_y = rbottom_y - au26_jaw_drop
    lbottom_y = lbottom_y - au26_jaw_drop
    chin_y = chin_y - au26_jaw_drop
    
    bottom_y = bottom_y - au27_mouth_stretch*2.5
    rbottom_y = rbottom_y - au27_mouth_stretch*2.2
    lbottom_y = lbottom_y - au27_mouth_stretch*2.2
    top_y = top_y + au27_mouth_stretch*1.2
    rtop_y = rtop_y + au27_mouth_stretch
    ltop_y = ltop_y + au27_mouth_stretch
    
    top_y = top_y - au28_lip_suck*.333
    bottom_y = bottom_y + au28_lip_suck*.333
    rtop_y = rtop_y - au28_lip_suck*.333
    rbottom_y = rbottom_y + au28_lip_suck*.333
    ltop_y = ltop_y - au28_lip_suck*.333
    lbottom_y = lbottom_y + au28_lip_suck*.333

    let chinPoints = [center_x, center_y, right_x, chin_y]
    let nosePoints = [center_x, center_y, right_x, nose_y]

    let mouthPoints = [center_x, center_y, right_x, right_y, left_x, left_y, top_x, top_y, bottom_x, bottom_y, lbottom_x, lbottom_y, rbottom_x, rbottom_y, ltop_x, ltop_y, rtop_x, rtop_y]

    return [mouthPoints, chinPoints, nosePoints]
}

function LinifyMouth(mouthPoints)
{
    [center_x, center_y, right_x, right_y, left_x, left_y, top_x, top_y, bottom_x, bottom_y, lbottom_x, lbottom_y, rbottom_x, rbottom_y, ltop_x, ltop_y, rtop_x, rtop_y] = mouthPoints

    var udis = [
    //left to top
    "M", center_x+left_x, center_y-left_y,
    "Q", 
    center_x+ltop_x, center_y-ltop_y-5,
    center_x+top_x, center_y-top_y-4,
    //top to right
    "Q", 
    center_x+rtop_x, center_y-rtop_y-5,
    center_x+right_x, center_y-right_y,
    //right to top
    "Q", 
    center_x+rtop_x, center_y-rtop_y,
    center_x+top_x, center_y-top_y,
    //top to left
    "Q", 
    center_x+ltop_x, center_y-ltop_y,
    center_x+left_x, center_y-left_y,
    ];

    var ldis = [
    //right to bottom
    "M", center_x+right_x, center_y-right_y,
    "Q", 
    center_x+rbottom_x, center_y-rbottom_y,
    center_x+bottom_x, center_y-bottom_y,

    //bottom to left
    "Q", 
    center_x+lbottom_x, center_y-lbottom_y,
    center_x+left_x, center_y-left_y,

    //left to bottom
    "Q", 
    center_x+lbottom_x, center_y-lbottom_y+5,
    center_x+bottom_x, center_y-bottom_y+4,
    
    //bottom to right
    "Q", 
    center_x+rbottom_x, center_y-rbottom_y+5,
    center_x+right_x, center_y-right_y,
    "Z"
    ]

    var mdis = [
    //right to top
    "M", center_x+right_x, center_y-right_y,
    "Q", 
    center_x+rtop_x, center_y-rtop_y,
    center_x+top_x, center_y-top_y,
    //top to left
    "Q", 
    center_x+ltop_x, center_y-ltop_y,
    center_x+left_x, center_y-left_y,
    //left to bottom
    "Q", 
    center_x+lbottom_x, center_y-lbottom_y,
    center_x+bottom_x, center_y-bottom_y,
    
    //bottom to right
    "Q", 
    center_x+rbottom_x, center_y-rbottom_y,
    center_x+right_x, center_y-right_y,
    "Z"
    ]


    const ud = udis.join(' ');
    const ld = ldis.join(' ');
    const md = mdis.join(' ');
    // console.log(d)
    return [ud, ld, md]
}

function LinifyChin(chinPoints)
{
    [center_x, center_y, right_x, chin_y] = chinPoints

    var dis = [
    //chin
    "M", center_x-right_x*.3, center_y-chin_y,
    "C", 
    center_x-right_x*.3, center_y-(chin_y-chin_y*.3),
    center_x+right_x*.3, center_y-(chin_y-chin_y*.3),
    center_x+right_x*.3, center_y-chin_y,
    ];


    const d = dis.join(' ');
    // console.log(d)
    return d
}

function LinifyNose(nosePoints)
{
    [center_x, center_y, right_x, nose_y] = nosePoints

    let i = 3

    var dis = [
    //nose
    "M", center_x-i*1.5, center_y-(nose_y+i*1.8),
    "C", 
    center_x-i*2, center_y-(nose_y-i*.25),
    center_x-i*1.8, center_y-(nose_y-i*.1),
    center_x-i, center_y-nose_y,

    "M", center_x-i, center_y-nose_y,
    "C", 
    center_x+i*.3, center_y-(nose_y-nose_y*.1),
    center_x-i*.3, center_y-(nose_y-nose_y*.1),
    center_x+i, center_y-nose_y,

    "M", center_x+i*1.5, center_y-(nose_y+i*1.8),
    "C", 
    center_x+i*2, center_y-(nose_y-i*.25),
    center_x+i*1.8, center_y-(nose_y-i*.1),
    center_x+i, center_y-nose_y,
    ];


    const d = dis.join(' ');
    // console.log(d)
    return d
}


function Mouth({position, mouthAU}) {
   
    const [mouth_positions, chinPoints, nosePoints] = AdjustMouth(position, mouthAU)
    const [ud, ld, md] = LinifyMouth(mouth_positions)
    // const md = ud + " " + ld
    const cd = LinifyChin(chinPoints)
    const nd = LinifyNose(nosePoints)
    const animationProps = useSpring({
        mouth: md,
        upperlip: ud,
        lowerlip: ld,
        chin: cd,
        nose: nd,
    })
  
    return (
        <g>
          <animated.path d={animationProps.mouth} stroke="#000" fill="#000" fillOpacity=".9" strokeWidth="1.2" strokeLinecap="round"/>
          {/* <animated.path d={animationProps.upperlip} stroke="#000" fill="#000" fillOpacity="0.1" strokeWidth=".01" strokeLinecap="round"/> */}
          {/* <animated.path d={animationProps.lowerlip} stroke="#000" fill="#000" fillOpacity="0.1" strokeWidth=".01" strokeLinecap="round"/> */}
          {/* <animated.path d={animationProps.chin} stroke="#000" fill="#002" fillOpacity=".1" strokeWidth="0" strokeLinecap="round"/> */}
          {/* <animated.path d={animationProps.nose} stroke="#000" fill="#00F" fillOpacity=".0" strokeWidth=".5" strokeLinecap="round"/> */}
        </g>
    );
}

export default Mouth;

```

## Source file: frontend/face/src/components/cordial/eyes.js

{
  "path": "frontend/face/src/components/cordial/eyes.js",
  "blob_sha": "999c95a9eb61ccd2569d1159e0162fbbf23959d0",
  "sha256": "f1f7d8a79e42a847bcead40295e5fd3ab3119ba52bbdb8be0b7c69a9b190a531",
  "lines": 150,
  "bytes": 4966,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 21,
    "rig": 22,
    "face": 3
  },
  "url": "https://github.com/chrismbirmingham/hci-face/blob/42df92fb959f19d05cb8346bcccbed67cedd1445/frontend/face/src/components/cordial/eyes.js",
  "function_signature_candidates": []
}

```text

import { animated, useSpring } from "react-spring";

var ey, ex, eright_x, eright_y;
var px, py, pr, sr;
var ulc, llc;
var lr, ud;




function moveEye(new_px, new_py, r, eright_x, eright_y){
    if (new_px < 0){
        px = Math.min(Math.abs(new_px), eright_x-r) * -1
    }
    else {px = Math.min(new_px, eright_x-r)}

    if (new_py < 0){
        py = Math.min(Math.abs(new_py), eright_y-r) * -1
    }
    else {py = Math.min(new_py, eright_y-r)}

    return [px, py]


}

function circlePath(center_x, center_y, r){
    var dis = [
        "M", center_x, center_y+r,
        "A", r,r, "0","1","0",center_x, center_y-r,
        "A", r,r, "0","0","0",center_x, center_y+r,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}

function bezelCirclePath(center_x, center_y, r){
    var dis = [
        "M", center_x-r, center_y,
        "C", center_x-r, center_y+r, center_x+r, center_y+r, center_x+r, center_y,// bottom half
        "C", center_x+r, center_y-r, center_x-r, center_y-r, center_x-r, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}

function lowerLidPath(center_x, center_y, eright_x, eright_y, c){
    // v: 1 is bottom half, with higher c = more closed. 0 is upper half with lower c more closed
    var dis = [
        "M", center_x-eright_x*1.1, center_y,
        "C", center_x-eright_x*1.1, center_y+eright_y, center_x+eright_x*1.1, center_y+eright_y, center_x+eright_x*1.1, center_y,
        "Q", center_x, center_y+eright_y*(1-c)*2, center_x-eright_x*1.1, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}
function upperLidPath(center_x, center_y, eright_x, eright_y, c){
    // desired behavior is 0 is open, 1 is closed and 2 is all the way down
    var dis = [
        "M", center_x-eright_x*1.1, center_y,
        "C", center_x-eright_x*1.1, center_y-eright_y, center_x+eright_x*1.1, center_y-eright_y, center_x+eright_x*1.1, center_y,
        "Q", center_x, center_y-eright_y*(1-c)*2, center_x-eright_x*1.1, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}


function Eye({position, eyeAU}) {
    [ey, ex] = [position.y, position.x];
    // Eye Params
    [eright_x, eright_y, pr, sr] = [10, 10, 6, 1];
    [llc, ulc] = [0,0];
    [lr, ud] = [0,0];

    var au5_upper_lid_raiser = eyeAU.au5_upper_lid_raiser;
    var au6_cheek_raiser = eyeAU.au6_cheek_raiser;
    var au7_lid_tightener = eyeAU.au7_lid_tightener;
    var au41_lid_droop = eyeAU.au41_lid_droop;
    var au42_slit = eyeAU.au42_slit;
    var au43_eyes_closed = eyeAU.au43_eyes_closed;
    var au44_squint = eyeAU.au44_squint;
    var au45_blink = eyeAU.au45_blink;
    var au61_left = eyeAU.au61_left;
    var au62_right = eyeAU.au62_right;
    var au63_up = eyeAU.au63_up;
    var au64_down = eyeAU.au64_down;


    lr = lr + au62_right - au61_left;
    ud = ud + au63_up - au64_down;

    ulc = ulc - au5_upper_lid_raiser*.5;
    llc = llc + au6_cheek_raiser*.5;

    ulc=ulc + au7_lid_tightener*.3;
    llc=llc + au7_lid_tightener*.3;

    ulc = ulc + au41_lid_droop*.7;

    ulc = ulc + au42_slit*.6;
    llc = llc + au42_slit*.6;

    ulc = ulc + au43_eyes_closed;
    llc = llc + au43_eyes_closed;

    ulc = ulc + au44_squint*.7;
    llc = llc + au44_squint*.6;

    ulc = ulc + au45_blink*2;

    [px, py] = moveEye(lr*eright_x, ud* -1 * eright_y, pr, eright_x, eright_y)

    let ed = bezelCirclePath(ex,ey, eright_x)
    let bd = bezelCirclePath(ex,ey, eright_x+3)

    let pd = circlePath(ex+px, ey+py, pr)// Pupil
    let sd = circlePath(ex+px-pr/2, ey+py-pr/2, sr) // Pupil shine


    let lld = lowerLidPath(ex, ey, eright_x, eright_y, llc)
    let uld = upperLidPath(ex, ey, eright_x, eright_y, ulc)

    const animationProps = useSpring({
        border: bd,
        eyeball: ed,
        pupil: pd,
        sparkle: sd,
        lowerLid: lld,
        upperLid: uld
    }
        )

    return (
        <g>
            <animated.path className="eyeball" d={animationProps.eyeball} stroke="#000" fill="#FFF" fillOpacity="0.95" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="pupil" d={animationProps.pupil} stroke="purple" fill="#000" fillOpacity="0.95" strokeWidth="3" strokeLinecap="round"/>
            <animated.path className="sparkle" d={animationProps.sparkle} stroke="#000" fill="#FFF" fillOpacity="1" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="lowerLid" d={animationProps.lowerLid} stroke="#000" fill={position.faceColor} fillOpacity=".995" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="upperLid" d={animationProps.upperLid} stroke="#000" fill={position.faceColor} fillOpacity=".995" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="border" d={animationProps.border} stroke={position.faceColor} fill="#FFF" fillOpacity="0" strokeWidth="5" strokeLinecap="round"/>
        </g>
    )
}

export default Eye;

```

## Source file: frontend/face/src/components/default/eyes.js

{
  "path": "frontend/face/src/components/default/eyes.js",
  "blob_sha": "a53c35bebee5b9166f71a4e3bf06aa4184f88f5e",
  "sha256": "d7ed4e12681cd9f7e6b3026169d5f5549d7d19599d1d2fafce97b68fb042dced",
  "lines": 150,
  "bytes": 4971,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 21,
    "rig": 22,
    "face": 3
  },
  "url": "https://github.com/chrismbirmingham/hci-face/blob/42df92fb959f19d05cb8346bcccbed67cedd1445/frontend/face/src/components/default/eyes.js",
  "function_signature_candidates": []
}

```text

import { animated, useSpring } from "react-spring";

var ey, ex, eright_x, eright_y;
var px, py, pr, sr;
var ulc, llc;
var lr, ud;




function moveEye(new_px, new_py, r, eright_x, eright_y){
    if (new_px < 0){
        px = Math.min(Math.abs(new_px), eright_x-r) * -1
    }
    else {px = Math.min(new_px, eright_x-r)}

    if (new_py < 0){
        py = Math.min(Math.abs(new_py), eright_y-r) * -1
    }
    else {py = Math.min(new_py, eright_y-r)}

    return [px, py]


}

function circlePath(center_x, center_y, r){
    var dis = [
        "M", center_x, center_y+r,
        "A", r,r, "0","1","0",center_x, center_y-r,
        "A", r,r, "0","0","0",center_x, center_y+r,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}

function bezelCirclePath(center_x, center_y, r){
    var dis = [
        "M", center_x-r, center_y,
        "C", center_x-r, center_y+r, center_x+r, center_y+r, center_x+r, center_y,// bottom half
        "C", center_x+r, center_y-r, center_x-r, center_y-r, center_x-r, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}

function lowerLidPath(center_x, center_y, eright_x, eright_y, c){
    // v: 1 is bottom half, with higher c = more closed. 0 is upper half with lower c more closed
    var dis = [
        "M", center_x-eright_x*1.1, center_y,
        "C", center_x-eright_x*1.1, center_y+eright_y, center_x+eright_x*1.1, center_y+eright_y, center_x+eright_x*1.1, center_y,
        "Q", center_x, center_y+eright_y*(1-c)*2, center_x-eright_x*1.1, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}
function upperLidPath(center_x, center_y, eright_x, eright_y, c){
    // desired behavior is 0 is open, 1 is closed and 2 is all the way down
    var dis = [
        "M", center_x-eright_x*1.1, center_y,
        "C", center_x-eright_x*1.1, center_y-eright_y, center_x+eright_x*1.1, center_y-eright_y, center_x+eright_x*1.1, center_y,
        "Q", center_x, center_y-eright_y*(1-c)*2, center_x-eright_x*1.1, center_y,
    ];
    const d = dis.join(' ');
    // console.log(d)
    return d
}


function Eye({position, eyeAU}) {
    [ey, ex] = [position.y, position.x];
    // Eye Params
    [eright_x, eright_y, pr, sr] = [10, 10, 6, 1];
    [llc, ulc] = [0,0];
    [lr, ud] = [0,0];

    var au5_upper_lid_raiser = eyeAU.au5_upper_lid_raiser;
    var au6_cheek_raiser = eyeAU.au6_cheek_raiser;
    var au7_lid_tightener = eyeAU.au7_lid_tightener;
    var au41_lid_droop = eyeAU.au41_lid_droop;
    var au42_slit = eyeAU.au42_slit;
    var au43_eyes_closed = eyeAU.au43_eyes_closed;
    var au44_squint = eyeAU.au44_squint;
    var au45_blink = eyeAU.au45_blink;
    var au61_left = eyeAU.au61_left;
    var au62_right = eyeAU.au62_right;
    var au63_up = eyeAU.au63_up;
    var au64_down = eyeAU.au64_down;


    lr = lr + au62_right - au61_left;
    ud = ud + au63_up - au64_down;

    ulc = ulc - au5_upper_lid_raiser*.5;
    llc = llc + au6_cheek_raiser*.5;

    ulc=ulc + au7_lid_tightener*.3;
    llc=llc + au7_lid_tightener*.3;

    ulc = ulc + au41_lid_droop*.7;

    ulc = ulc + au42_slit*.6;
    llc = llc + au42_slit*.6;

    ulc = ulc + au43_eyes_closed;
    llc = llc + au43_eyes_closed;

    ulc = ulc + au44_squint*.7;
    llc = llc + au44_squint*.6;

    ulc = ulc + au45_blink*2;

    [px, py] = moveEye(lr*eright_x, ud* -1 * eright_y, pr, eright_x, eright_y)

    let ed = bezelCirclePath(ex,ey, eright_x)
    let bd = bezelCirclePath(ex,ey, eright_x+3)

    let pd = circlePath(ex+px, ey+py, pr)// Pupil
    let sd = circlePath(ex+px-pr/2, ey+py-pr/2, sr) // Pupil shine


    let lld = lowerLidPath(ex, ey, eright_x, eright_y, llc)
    let uld = upperLidPath(ex, ey, eright_x, eright_y, ulc)

    const animationProps = useSpring({
        border: bd,
        eyeball: ed,
        pupil: pd,
        sparkle: sd,
        lowerLid: lld,
        upperLid: uld
    }
        )

    return (
        <g>
            <animated.path className="eyeball" d={animationProps.eyeball} stroke="#000" fill="#FFF" fillOpacity="0.95" strokeWidth=".15" strokeLinecap="round"/>
            <animated.path className="pupil" d={animationProps.pupil} stroke="green" fill="#000" fillOpacity="0.95" strokeWidth="3" strokeLinecap="round"/>
            <animated.path className="sparkle" d={animationProps.sparkle} stroke="#000" fill="#FFF" fillOpacity="1" strokeWidth="0" strokeLinecap="round"/>
            <animated.path className="lowerLid" d={animationProps.lowerLid} stroke="#000" fill={position.faceColor} fillOpacity=".995" strokeWidth="0.3" strokeLinecap="round"/>
            <animated.path className="upperLid" d={animationProps.upperLid} stroke="#000" fill={position.faceColor} fillOpacity=".995" strokeWidth="0.3" strokeLinecap="round"/>
            <animated.path className="border" d={animationProps.border} stroke={position.faceColor} fill="#FFF" fillOpacity="0" strokeWidth="5" strokeLinecap="round"/>
        </g>
    )
}

export default Eye;

```
