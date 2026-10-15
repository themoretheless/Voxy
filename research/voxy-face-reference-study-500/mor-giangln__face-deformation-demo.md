# Facial rendering source scan: mor-giangln/face-deformation-demo

Repository: https://github.com/mor-giangln/face-deformation-demo

Pinned source revision: ec78af9297902e8e7ec59b53a1cf3ff22792572b

Discovery topic: face deformation

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: src/Constant.js

{
  "path": "src/Constant.js",
  "blob_sha": "d7b0baf13a9c0a95fb571dcd0818eb8bb1a7bc7b",
  "sha256": "f64953f416cc26029784d75c99e00fded8f8c8f4f5bab7157a1224aaf632597c",
  "lines": 20,
  "bytes": 335,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 2
  },
  "url": "https://github.com/mor-giangln/face-deformation-demo/blob/ec78af9297902e8e7ec59b53a1cf3ff22792572b/src/Constant.js",
  "function_signature_candidates": []
}

```text

export const meshOptions = {
    color: 0xffffff,
    wireframe: false,
    intensity: 500
}

export const sphereOptions = {
    showCtrlPoint: true,
    width: 0.5,
    height: 32,
    depth: 32,
    widthSegments: 1,
    heightSegments: 1,
    depthSegments: 1,
};

export const morphOptions = {
    upperLips: 0,
    lowerLips: 0
};

```

## Source file: src/react-app-env.d.ts

{
  "path": "src/react-app-env.d.ts",
  "blob_sha": "6431bc5fc6b2c932dfe5d0418fc667b86c18b9fc",
  "sha256": "57eda4c4c04a1dca45c62857326882ce9cc948c4b52973c0e3c3b7e4c3fa3990",
  "lines": 1,
  "bytes": 40,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mor-giangln/face-deformation-demo/blob/ec78af9297902e8e7ec59b53a1cf3ff22792572b/src/react-app-env.d.ts",
  "function_signature_candidates": []
}

```text

/// <reference types="react-scripts" />


```

## Source file: src/reportWebVitals.ts

{
  "path": "src/reportWebVitals.ts",
  "blob_sha": "49a2a16e0fbc7636ee16bf907257a5282b856493",
  "sha256": "fa208d5a5ab6d20b64122998b074707dea08d12ed619652955f77df958d85786",
  "lines": 15,
  "bytes": 425,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mor-giangln/face-deformation-demo/blob/ec78af9297902e8e7ec59b53a1cf3ff22792572b/src/reportWebVitals.ts",
  "function_signature_candidates": []
}

```text

import { ReportHandler } from 'web-vitals';

const reportWebVitals = (onPerfEntry?: ReportHandler) => {
  if (onPerfEntry && onPerfEntry instanceof Function) {
    import('web-vitals').then(({ getCLS, getFID, getFCP, getLCP, getTTFB }) => {
      getCLS(onPerfEntry);
      getFID(onPerfEntry);
      getFCP(onPerfEntry);
      getLCP(onPerfEntry);
      getTTFB(onPerfEntry);
    });
  }
};

export default reportWebVitals;


```

## Source file: src/setupTests.ts

{
  "path": "src/setupTests.ts",
  "blob_sha": "8f2609b7b3e0e3897ab3bcaad13caf6876e48699",
  "sha256": "22583759d0045fdf8d62c9db0aacba9fd8bddde79c671aa08c97dcfd4e930cc6",
  "lines": 5,
  "bytes": 241,
  "selection_score": -2,
  "keyword_line_counts": {},
  "url": "https://github.com/mor-giangln/face-deformation-demo/blob/ec78af9297902e8e7ec59b53a1cf3ff22792572b/src/setupTests.ts",
  "function_signature_candidates": []
}

```text

// jest-dom adds custom jest matchers for asserting on DOM nodes.
// allows you to do things like:
// expect(element).toHaveTextContent(/react/i)
// learn more: https://github.com/testing-library/jest-dom
import '@testing-library/jest-dom';


```
