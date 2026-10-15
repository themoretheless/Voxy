# Facial rendering source scan: makehuman-js/makehuman-js

Repository: https://github.com/makehuman-js/makehuman-js

Pinned source revision: 26e99dceb402e97fe75242d0383d4c26d63d76b4

Discovery topic: MakeHuman

Description: A library to build 3D human characters in the browser

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["src/js/human/ethnic-skin-blender.js", "test/mocha/ethnic-skin-blender_spec.js"]}

## Source file: src/js/human/ethnic-skin-blender.js

{
  "path": "src/js/human/ethnic-skin-blender.js",
  "blob_sha": "6396f848e5407bd8acaca2f0c49668819aa32af0",
  "sha256": "7066639e23cc3e6ae5c6c241615d772dcd8193c78c022f6c3abcd693036120f3",
  "lines": 41,
  "bytes": 1318,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 5,
    "rig": 2
  },
  "url": "https://github.com/makehuman-js/makehuman-js/blob/26e99dceb402e97fe75242d0383d4c26d63d76b4/src/js/human/ethnic-skin-blender.js",
  "function_signature_candidates": []
}

```text

/**
 * @name            MakeHuman
 * @copyright       MakeHuman Team 2001-2016
 * @license         [AGPL3]{@link http://www.makehuman.org/license.php}
 * @description     blends three ethnic skin tones
 */
import * as THREE from 'three'
import _ from 'lodash'

// these are set to look right when added to the caucasian skin
const asianColor = new THREE.Color().setHSL(0.078, 0.34, 0.576)
const africanColor = new THREE.Color().setHSL(0.09, 0.83, 0.21)
const caucasianColor = new THREE.Color().setHSL(0.062, 0.51, 0.68)


export class EthnicSkinBlender {
    /**
     * return a blend of the three ethnic skin tones based on the human macro settings.
    **/
    constructor(human) {
        this.human = human
    }

    valueOf() {
        const blends = [
            this.human.factors.getCaucasian(),
            this.human.factors.getAfrican(),
            this.human.factors.getAsian()
        ]

        // Set diffuse color
        const color = new THREE.Color(0, 0, 0)
            .add(caucasianColor.clone().multiplyScalar(blends[0]))
            .add(africanColor.clone().multiplyScalar(blends[1]))
            .add(asianColor.clone().multiplyScalar(blends[2]))
        // clamp to [0,1]
        return color.fromArray(color.toArray().map(v => _.clamp(v, 0, 1)))
    }
}

export default EthnicSkinBlender


```

## Source file: test/mocha/ethnic-skin-blender_spec.js

{
  "path": "test/mocha/ethnic-skin-blender_spec.js",
  "blob_sha": "28c8a30ddf7ef0a7eb970a93051bc5068ba4bcb0",
  "sha256": "777908a9a2b20ef8deceefa21b80538168d210eadb353391412bc6342aab5370",
  "lines": 23,
  "bytes": 641,
  "selection_score": 13,
  "keyword_line_counts": {
    "skin": 3
  },
  "url": "https://github.com/makehuman-js/makehuman-js/blob/26e99dceb402e97fe75242d0383d4c26d63d76b4/test/mocha/ethnic-skin-blender_spec.js",
  "function_signature_candidates": []
}

```text


import _ from 'lodash'
import * as THREE from 'three'

import {
    Human
} from '../../src/js/human/human.js'
import { config, expect } from './fixtures.js'

describe('ethnic-skin-blender.js', () => {
    describe('ethnicSkinBlender', () => {
        it('description', (done) => {
            const human = new Human(config)
            human.loadModel(config)
                .then(() => {
                    const color = self.human.ethnicSkinBlender.valueOf()
                    expect(color.isColor).to.be(true)
                })
                .then(() => done())
                .catch(error => done(error))
        })
    })
})


```

## Source file: .eslintrc.js

{
  "path": ".eslintrc.js",
  "blob_sha": "1ebef7b390a6fcb7e4686cc6e887fb11bfb3bc1d",
  "sha256": "f4dfdfbe623d22587302adfded00487f1d2aebd1f66b7388426b0f262efe2d95",
  "lines": 81,
  "bytes": 2545,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/makehuman-js/makehuman-js/blob/26e99dceb402e97fe75242d0383d4c26d63d76b4/.eslintrc.js",
  "function_signature_candidates": []
}

```text

module.exports = {
    "parser": 'babel-eslint',
    plugins: [
    ],
    extends: [
        'airbnb'
    ],
    "env": {
        "es6": true,
        "node": true,
        "browser": true,
        // "amd": true,
        "mocha": true
    },
    settings:{
         'import/resolver': 'webpack'
    },
    parserOptions: {
        ecmaVersion: 6,
        ecmaFeatures: {
            experimentalObjectRestSpread: true
        },
        sourceType: 'module'
    },
    "rules": {
        "max-len": [1, 180, 2, {"ignoreComments": true}],
        "quote-props": [1, "consistent-as-needed"],
        "comma-dangle": [0, "never"],
        // "no-unused-vars": [1, {"vars": "local", "args": "none"}],
        "indent": ["error", 4],
        "no-underscore-dangle": [0],
        "linebreak-style": ["error", "unix"],
        "semi": ["warn", "never"],
        "no-console": 0,
        "no-use-before-define": 1,
        "no-else-return": 0,
        "quotes": 0,
        'no-mixed-operators': ["error", {"allowSamePrecedence": true}],
        "import/no-extraneous-dependencies": 'off',
        "import/extensions": 'off',
        "import/no-unresolved": 'warn',
        "no-unused-vars": 1,
        "no-param-reassign": 0,
        "curly": 1,
        "class-methods-use-this": 0,
        "no-return-assign": 0,
        "object-curly-spacing": 1,
        "no-mixed-operators": 0,
        "no-plusplus": 0,
        "arrow-body-style": 0
        // "arrow-parens": 0,
        // 'arrow-spacing': 'error',
        // 'block-spacing': 'error',
        // 'comma-dangle': 'off',
        // 'comma-spacing': 'error',
        // 'comma-style': 'error',
        // 'curly': 'error',
        // 'dot-notation': 'error',
        // 'eqeqeq': 'error',
        // 'eol-last': 'error',
        // 'indent': ['error', 4, {SwitchCase: 1}],
        // 'key-spacing': 'error',
        // 'keyword-spacing': 'error',
        // 'linebreak-style': ['error', 'unix'],
        // 'no-console': 'off',
        // 'no-param-reassign': 'error',
        // 'no-tabs': 'error',
        // 'no-trailing-spaces': 'error',
        // 'no-underscore-dangle': 'error',
        // 'no-whitespace-before-property': 'error',
        // 'quotes': ['error', 'single'],
        // 'semi': ['error', 'always'],
        // 'semi-spacing': 'error',
        // 'space-before-blocks': 'error',
        // 'space-before-function-paren': ['error', 'never'],
        // 'space-in-parens': 'error',
        //
        // 'react/display-name': 'off',
        // 'react/prop-types': 'off'
    }
}


```

## Source file: src/bundle.js

{
  "path": "src/bundle.js",
  "blob_sha": "f4fb4254835c9a5326dac7c1432b5b905d303f3a",
  "sha256": "4a7b917f7492f7018064417dbe3555118f87ca9314de302f144ab91cb93a0ee0",
  "lines": 7,
  "bytes": 302,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/makehuman-js/makehuman-js/blob/26e99dceb402e97fe75242d0383d4c26d63d76b4/src/bundle.js",
  "function_signature_candidates": []
}

```text

// export * as humanModifier from './human/human-modifier.js'
// export * as human from './human/human.js'
// export * as targets from './human/targets.js'
// export * from './human/human-modifier.js'
// export * from './human/proxy.js'
// export * from './human/factors.js'
export * from './js/human'


```
